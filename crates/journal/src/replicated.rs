//! Ciphertext replication plus a separately authenticated linearizable witness.
//! The witness is a trusted port, NOT implemented by a file on the hostile host.

use crate::{
    Backend, Error, Frame, Head, MAX_HISTORY_BYTES, MAX_RECORDS, model::PrivateBytes,
    wire::MAX_RECORD,
};
use chacha20poly1305::aead::{OsRng, rand_core::RngCore};
use cinder_kernel::identity::Domain;
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

/// Witness identity includes the journal, not just its network/deployment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stream {
    /// Independently expected network and deployment.
    pub domain: Domain,
    /// Random/nonzero stable journal identifier, not a customer identifier.
    pub id: [u8; 32],
}
/// Entire atomic witness register; an epoch change never discards an accepted head.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Anchor {
    /// Monotonic fencing token. Zero is invalid.
    pub epoch: u64,
    /// None only for explicitly provisioned, never-initialized streams.
    pub head: Option<Head>,
}
/// Authenticated, replay-resistant, durably linearizable service in a separately
/// qualified failure/admin domain. Responses must bind Stream and caller rights.
/// Provisioning and epoch advancement are operator-authorized outside this port.
/// Never reinitialize a missing register or fabricate success on a lost reply.
pub trait Witness {
    /// Fresh authenticated current register, not a cached/host-supplied snapshot.
    fn read(&mut self, stream: Stream) -> Result<Anchor, Error>;
    /// Atomically replace the exact register; this writer may change only head,
    /// not epoch. Return success only after durable commitment; stale CAS rejects.
    fn accept(&mut self, stream: Stream, expected: Anchor, next: Head) -> Result<(), Error>;
}
/// Immutable, content-addressed ciphertext storage. Successful put must survive
/// the declared failure model. Returning bytes is not proof of independence.
pub trait Replica {
    /// Stable storage identity to reject accidental aliasing. Distinct IDs do not
    /// prove independent infrastructure; that remains a deployment obligation.
    fn identity(&self) -> [u8; 32];
    /// Write the exact opaque frame durably. Same digest/body is idempotent;
    /// corrupt/unaccepted objects can be repaired without moving the witness.
    fn put(&mut self, frame: &Frame) -> Result<(), Error>;
    /// Bounded read by ciphertext digest. Absence/corruption may use another copy.
    fn get(&mut self, digest: [u8; 32]) -> Result<Frame, Error>;
}

/// Two-of-two durable writes, one valid-copy recovery. Orphans are never accepted
/// by existence alone. Unavailability after a successful witness CAS can leave an
/// uncertain caller result, so the journal poisons until explicit reload.
pub struct Replicated<A: Replica, B: Replica, W: Witness> {
    stream: Stream,
    epoch: u64,
    first: A,
    second: B,
    witness: W,
}
impl<A: Replica, B: Replica, W: Witness> Replicated<A, B, W> {
    /// Bind already provisioned storage and a separately authorized writer epoch.
    /// Constructor does not create/reset a witness, rotate authority, or decrypt.
    pub fn new(stream: Stream, epoch: u64, first: A, second: B, witness: W) -> Result<Self, Error> {
        if stream.id == [0; 32]
            || epoch == 0
            || first.identity() == second.identity()
            || first.identity() == [0; 32]
            || second.identity() == [0; 32]
        {
            return Err(Error::Invalid);
        }
        Ok(Self {
            stream,
            epoch,
            first,
            second,
            witness,
        })
    }
    fn anchor(&mut self) -> Result<Anchor, Error> {
        let a = self.witness.read(self.stream)?;
        if a.epoch != self.epoch {
            return Err(Error::Stale);
        }
        Ok(a)
    }
    fn fetch(&mut self, expected: Head) -> Result<Frame, Error> {
        // Recovery only needs one valid copy. Do not require repair availability
        // to serve an authenticated read; new acceptance repairs/checks both below.
        for replica in [&mut self.first as &mut dyn Replica, &mut self.second] {
            let candidate = replica.get(expected.hash);
            if let Ok(frame) = candidate
                && frame.head == expected
                && frame.validate().is_ok()
            {
                return Ok(frame);
            }
        }
        Err(Error::Storage)
    }
    fn ensure_both(&mut self, frame: &Frame) -> Result<(), Error> {
        for replica in [&mut self.first as &mut dyn Replica, &mut self.second] {
            if replica
                .get(frame.head.hash)
                .is_ok_and(|copy| copy == *frame)
            {
                continue;
            }
            replica.put(frame).map_err(|_| Error::Storage)?;
            if !replica
                .get(frame.head.hash)
                .is_ok_and(|copy| copy == *frame)
            {
                return Err(Error::Storage);
            }
        }
        Ok(())
    }
    /// Export a bounded replay snapshot of authenticated ciphertext records.
    /// It contains no decoded projection; retaining the complete history avoids
    /// losing consumption/attempt evidence. No compaction or pruning is implied.
    pub fn snapshot(&mut self) -> Result<Vec<u8>, Error> {
        let frames = self.load()?;
        let mut bytes = b"CINDER-SNAPSHOT-1\0".to_vec();
        bytes.extend_from_slice(&self.stream.domain.network.bytes());
        bytes.extend_from_slice(&self.stream.domain.deployment.bytes());
        bytes.extend_from_slice(&self.stream.id);
        bytes.extend_from_slice(&(frames.len() as u32).to_be_bytes());
        for frame in frames {
            let encoded = encode_frame(&frame)?;
            bytes.extend_from_slice(&(encoded.len() as u32).to_be_bytes());
            bytes.extend_from_slice(&encoded);
        }
        Ok(bytes)
    }
    /// Rehydrate replicas only if the complete snapshot matches the independent
    /// current accepted head and stream. Does not advance/reset any witness. The
    /// caller must subsequently Journal::open to authenticate and replay AEAD.
    pub fn restore_snapshot(&mut self, bytes: &[u8]) -> Result<(), Error> {
        if bytes.len() > MAX_HISTORY_BYTES + MAX_RECORDS * 80 + 117 {
            return Err(Error::Limit);
        }
        let prefix = [
            b"CINDER-SNAPSHOT-1\0".as_slice(),
            &self.stream.domain.network.bytes(),
            &self.stream.domain.deployment.bytes(),
            &self.stream.id,
        ]
        .concat();
        let mut rest = bytes.strip_prefix(prefix.as_slice()).ok_or(Error::Codec)?;
        let count = take_u32(&mut rest)? as usize;
        if count == 0 || count > MAX_RECORDS {
            return Err(Error::Limit);
        }
        let mut frames = Vec::with_capacity(count);
        let mut prior = [0; 32];
        let mut total = 0_usize;
        for n in 0..count {
            let len = take_u32(&mut rest)? as usize;
            if len > MAX_RECORD + 76 || len > rest.len() {
                return Err(Error::Codec);
            }
            let frame = decode_frame(&rest[..len])?;
            rest = &rest[len..];
            if frame.head.sequence != n as u64 || frame.previous != prior {
                return Err(Error::Codec);
            }
            total = total
                .checked_add(frame.opaque.as_bytes().len())
                .ok_or(Error::Limit)?;
            if total > MAX_HISTORY_BYTES {
                return Err(Error::Limit);
            }
            prior = frame.head.hash;
            frames.push(frame);
        }
        if !rest.is_empty() {
            return Err(Error::Codec);
        }
        let anchor = self.anchor()?;
        if frames.last().map(|f| f.head) != anchor.head {
            return Err(Error::Stale);
        }
        for frame in &frames {
            // Immutable stores need not accept another put of an existing
            // object. Validate both copies and write/read back only missing or
            // invalid ones, exactly as before new acceptance. Never reset CAS.
            self.ensure_both(frame)?;
        }
        if self.anchor()? != anchor {
            return Err(Error::Stale);
        }
        Ok(())
    }
    fn copy_both(&mut self, frame: &Frame) -> Result<(), Error> {
        self.first.put(frame)?;
        if self.first.get(frame.head.hash)? != *frame {
            return Err(Error::Storage);
        }
        self.second.put(frame)?;
        if self.second.get(frame.head.hash)? != *frame {
            return Err(Error::Storage);
        }
        Ok(())
    }
}
impl<A: Replica, B: Replica, W: Witness> Backend for Replicated<A, B, W> {
    fn load(&mut self) -> Result<Vec<Frame>, Error> {
        let anchor = self.anchor()?;
        let Some(mut current) = anchor.head else {
            return Ok(Vec::new());
        };
        if current.sequence >= MAX_RECORDS as u64 {
            return Err(Error::Limit);
        }
        let mut frames = Vec::with_capacity(current.sequence as usize + 1);
        let mut total = 0_usize;
        loop {
            let frame = self.fetch(current)?;
            total = total
                .checked_add(frame.opaque.as_bytes().len())
                .ok_or(Error::Limit)?;
            if total > MAX_HISTORY_BYTES {
                return Err(Error::Limit);
            }
            let previous = frame.previous;
            frames.push(frame);
            if current.sequence == 0 {
                if previous != [0; 32] {
                    return Err(Error::Codec);
                }
                break;
            }
            current = Head {
                sequence: current.sequence - 1,
                hash: previous,
            };
        }
        if self.anchor()? != anchor {
            return Err(Error::Stale);
        }
        frames.reverse();
        Ok(frames)
    }
    fn append(&mut self, expected: Option<Head>, frame: &Frame) -> Result<(), Error> {
        frame.validate()?;
        let anchor = self.anchor()?;
        if anchor.head != expected {
            return Err(Error::Stale);
        }
        let sequence = expected.map_or(Ok(0), |h| h.sequence.checked_add(1).ok_or(Error::Limit))?;
        if frame.head.sequence != sequence || frame.previous != expected.map_or([0; 32], |h| h.hash)
        {
            return Err(Error::Invalid);
        }
        if sequence >= MAX_RECORDS as u64 {
            return Err(Error::Limit);
        }
        // Check cumulative bounds before acceptance, not only at the next restart.
        let history = self.load()?;
        let total = history
            .iter()
            .try_fold(frame.opaque.as_bytes().len(), |n, f| {
                n.checked_add(f.opaque.as_bytes().len()).ok_or(Error::Limit)
            })?;
        if total > MAX_HISTORY_BYTES {
            return Err(Error::Limit);
        }
        self.copy_both(frame)?;
        // Losing an old copy must not silently turn later accepted history into
        // single-replica durability. Restore all missing/corrupt accepted objects
        // or refuse new acceptance. Orphans remain harmless if repair fails.
        for accepted in &history {
            self.ensure_both(accepted)?;
        }
        self.witness
            .accept(self.stream, anchor, frame.head)
            .map_err(|e| if e == Error::Stale { e } else { Error::Storage })?;
        // CAS already succeeded: any failure now is an uncertain caller outcome,
        // not a known rejection. Force the journal to poison until reconciliation.
        if self.anchor().map_err(|_| Error::Storage)?
            != (Anchor {
                epoch: self.epoch,
                head: Some(frame.head),
            })
        {
            return Err(Error::Storage);
        }
        Ok(())
    }
    fn check_current(&mut self, expected: Head) -> Result<(), Error> {
        if self.anchor()?.head != Some(expected) {
            return Err(Error::Stale);
        }
        Ok(())
    }
}

fn take_u32(bytes: &mut &[u8]) -> Result<u32, Error> {
    let n = bytes
        .get(..4)
        .ok_or(Error::Codec)?
        .try_into()
        .map_err(|_| Error::Codec)?;
    *bytes = &bytes[4..];
    Ok(u32::from_be_bytes(n))
}
fn encode_frame(f: &Frame) -> Result<Vec<u8>, Error> {
    f.validate()?;
    Ok([
        &f.head.sequence.to_be_bytes()[..],
        &f.previous,
        &f.head.hash,
        &(f.opaque.as_bytes().len() as u32).to_be_bytes(),
        f.opaque.as_bytes(),
    ]
    .concat())
}
fn decode_frame(bytes: &[u8]) -> Result<Frame, Error> {
    if bytes.len() < 76 || bytes.len() > MAX_RECORD + 76 {
        return Err(Error::Codec);
    }
    let size = u32::from_be_bytes(bytes[72..76].try_into().map_err(|_| Error::Codec)?) as usize;
    if size != bytes.len() - 76 {
        return Err(Error::Codec);
    }
    let frame = Frame {
        head: Head {
            sequence: u64::from_be_bytes(bytes[..8].try_into().map_err(|_| Error::Codec)?),
            hash: bytes[40..72].try_into().map_err(|_| Error::Codec)?,
        },
        previous: bytes[8..40].try_into().map_err(|_| Error::Codec)?,
        opaque: PrivateBytes::new(bytes[76..].to_vec())?,
    };
    frame.validate()?;
    Ok(frame)
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Durable local ciphertext object adapter. Two directories on one machine are
/// NOT independent failure domains. Paths and objects carry only public metadata.
/// A hostile host can delete/deny service, but cannot forge accepted private data.
pub struct FileReplica {
    directory: PathBuf,
    identity: [u8; 32],
}
impl FileReplica {
    /// Explicit new object directory; never overwrite an existing directory.
    pub fn create(directory: &Path) -> Result<Self, Error> {
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        builder.mode(0o700);
        builder.create(directory).map_err(|_| Error::Storage)?;
        if let Some(parent) = directory.parent() {
            File::open(parent)
                .and_then(|f| f.sync_all())
                .map_err(|_| Error::Storage)?;
        }
        Self::open(directory)
    }
    /// Existing directory only. Missing replicas may be represented by a failed
    /// Replica adapter on restore; never silently create a new accepted stream.
    pub fn open(directory: &Path) -> Result<Self, Error> {
        let meta = fs::symlink_metadata(directory).map_err(|_| Error::Storage)?;
        if !meta.is_dir() || meta.file_type().is_symlink() {
            return Err(Error::Storage);
        }
        #[cfg(unix)]
        if meta.permissions().mode() & 0o077 != 0 {
            return Err(Error::Storage);
        }
        let directory = fs::canonicalize(directory).map_err(|_| Error::Storage)?;
        let identity = Sha256::digest(directory.to_str().ok_or(Error::Invalid)?.as_bytes()).into();
        Ok(Self {
            directory,
            identity,
        })
    }
}
impl Replica for FileReplica {
    fn identity(&self) -> [u8; 32] {
        self.identity
    }
    fn put(&mut self, frame: &Frame) -> Result<(), Error> {
        let bytes = encode_frame(frame)?;
        let mut random = [0; 24];
        OsRng
            .try_fill_bytes(&mut random)
            .map_err(|_| Error::Storage)?;
        let tmp = self.directory.join(format!("pending-{}", hex(&random)));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options.open(&tmp).map_err(|_| Error::Storage)?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| Error::Storage)?;
        fs::rename(&tmp, self.directory.join(hex(&frame.head.hash))).map_err(|_| Error::Storage)?;
        File::open(&self.directory)
            .and_then(|f| f.sync_all())
            .map_err(|_| Error::Storage)
    }
    fn get(&mut self, digest: [u8; 32]) -> Result<Frame, Error> {
        let path = self.directory.join(hex(&digest));
        let meta = fs::symlink_metadata(&path).map_err(|_| Error::Storage)?;
        if !meta.is_file() || meta.file_type().is_symlink() {
            return Err(Error::Storage);
        }
        if meta.len() > (MAX_RECORD + 76) as u64 {
            return Err(Error::Limit);
        }
        let mut bytes = Vec::new();
        File::open(path)
            .map_err(|_| Error::Storage)?
            .take((MAX_RECORD + 77) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| Error::Storage)?;
        let frame = decode_frame(&bytes)?;
        if frame.head.hash != digest {
            return Err(Error::Codec);
        }
        Ok(frame)
    }
}
