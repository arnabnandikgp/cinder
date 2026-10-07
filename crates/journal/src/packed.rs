//! Retained-history pack prototype. Not selected by the shipping Nitro runtime.
//! Frames/AEAD/witness semantics stay unchanged; no checkpoint, cache or pruning.
use crate::{
    Backend, Error, Frame, Head, MAX_HISTORY_BYTES, MAX_RECORDS,
    replicated::{
        Anchor, Stream, Witness, decode_frame, decode_snapshot, encode_frame, encode_snapshot,
        take_u32,
    },
    wire::MAX_RECORD,
};
use chacha20poly1305::aead::{OsRng, rand_core::RngCore};
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

const MAGIC: &[u8] = b"CINDER-PACK-1\0";
/// Fixed canonical sequence partition; changing it requires a new storage format.
pub const PACK_RECORDS: usize = 16;
/// Maximum encoded pack, including every legal full-size frame and its headers.
/// This exceeds the current shipping S3 body's limit; promotion must qualify a
/// pack-specific transport bound, not enlarge all cloud responses implicitly.
pub const MAX_PACK_BYTES: usize = MAGIC.len() + 100 + PACK_RECORDS * (MAX_RECORD + 80);

/// Trusted bounded opaque-object port. No plaintext, list-based discovery or
/// mutable latest pointer. Separate deployed failure domains remain mandatory.
pub trait Store {
    /// Distinct stable identity; not proof of infrastructure independence.
    fn identity(&self) -> [u8; 32];
    /// Durable exact bytes under the pack's ending frame hash. Repair of missing
    /// or corrupt objects must either read back exactly or prevent acceptance.
    fn put(&mut self, end: [u8; 32], bytes: &[u8]) -> Result<(), Error>;
    /// Must enforce MAX_PACK_BYTES before untrusted input allocation. Returning
    /// bytes alone does not authenticate the stream, partition or frame chain.
    fn get(&mut self, end: [u8; 32]) -> Result<Vec<u8>, Error>;
}
struct Pack(Vec<Frame>);
impl Pack {
    fn head(&self) -> Head {
        self.0.last().expect("validated nonempty pack").head
    }
    fn bytes(&self, stream: Stream) -> Result<Vec<u8>, Error> {
        let mut bytes = MAGIC.to_vec();
        bytes.extend_from_slice(&stream.domain.network.bytes());
        bytes.extend_from_slice(&stream.domain.deployment.bytes());
        bytes.extend_from_slice(&stream.id);
        bytes.extend_from_slice(&(self.0.len() as u32).to_be_bytes());
        for frame in &self.0 {
            let encoded = encode_frame(frame)?;
            bytes.extend_from_slice(&(encoded.len() as u32).to_be_bytes());
            bytes.extend_from_slice(&encoded);
        }
        if bytes.len() > MAX_PACK_BYTES {
            return Err(Error::Limit);
        }
        Ok(bytes)
    }
    fn decode(stream: Stream, expected: Head, bytes: &[u8]) -> Result<Self, Error> {
        if expected.sequence >= MAX_RECORDS as u64 || bytes.len() > MAX_PACK_BYTES {
            return Err(Error::Limit);
        }
        let prefix = [
            MAGIC,
            &stream.domain.network.bytes(),
            &stream.domain.deployment.bytes(),
            &stream.id,
        ]
        .concat();
        let mut rest = bytes.strip_prefix(prefix.as_slice()).ok_or(Error::Codec)?;
        let count = take_u32(&mut rest)? as usize;
        let start = expected.sequence / PACK_RECORDS as u64 * PACK_RECORDS as u64;
        if count != (expected.sequence - start + 1) as usize {
            return Err(Error::Codec);
        }
        let mut frames: Vec<Frame> = Vec::with_capacity(count);
        for n in 0..count {
            let size = take_u32(&mut rest)? as usize;
            if size > MAX_RECORD + 76 || size > rest.len() {
                return Err(Error::Codec);
            }
            let frame = decode_frame(&rest[..size])?;
            rest = &rest[size..];
            if frame.head.sequence != start + n as u64
                || frames
                    .last()
                    .is_some_and(|last| frame.previous != last.head.hash)
                || start == 0 && n == 0 && frame.previous != [0; 32]
            {
                return Err(Error::Codec);
            }
            frames.push(frame);
        }
        let result = Self(frames);
        if !rest.is_empty() || result.head() != expected {
            return Err(Error::Codec);
        }
        Ok(result)
    }
}

/// Two-copy retained pack backend. The witness still commits the SAME ending
/// frame Head, whose hash transitively binds every record and predecessor pack.
/// Canonical 16-record partitions eliminate alternate layouts for that key.
/// Each append re-loads and later checks/repairs ALL required historical packs.
pub struct Packed<A: Store, B: Store, W: Witness> {
    stream: Stream,
    epoch: u64,
    first: A,
    second: B,
    witness: W,
}
impl<A: Store, B: Store, W: Witness> Packed<A, B, W> {
    /// Explicit format selection and provisioned epoch. Never initializes the
    /// witness or falls back to legacy frame objects. Not live deployment approval.
    pub fn new(stream: Stream, epoch: u64, first: A, second: B, witness: W) -> Result<Self, Error> {
        if stream.id == [0; 32]
            || epoch == 0
            || first.identity() == [0; 32]
            || second.identity() == [0; 32]
            || first.identity() == second.identity()
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
        let anchor = self.witness.read(self.stream)?;
        if anchor.epoch != self.epoch {
            return Err(Error::Stale);
        }
        Ok(anchor)
    }
    fn fetch(&mut self, expected: Head) -> Result<Pack, Error> {
        for store in [&mut self.first as &mut dyn Store, &mut self.second] {
            if let Ok(bytes) = store.get(expected.hash)
                && let Ok(pack) = Pack::decode(self.stream, expected, &bytes)
            {
                return Ok(pack);
            }
        }
        Err(Error::Storage)
    }
    fn history(&mut self) -> Result<Vec<Pack>, Error> {
        let anchor = self.anchor()?;
        let Some(mut expected) = anchor.head else {
            return Ok(Vec::new());
        };
        if expected.sequence >= MAX_RECORDS as u64 {
            return Err(Error::Limit);
        }
        let mut packs = Vec::new();
        let mut total = 0usize;
        loop {
            let pack = self.fetch(expected)?;
            for frame in &pack.0 {
                total = total
                    .checked_add(frame.opaque.as_bytes().len())
                    .ok_or(Error::Limit)?;
                if total > MAX_HISTORY_BYTES {
                    return Err(Error::Limit);
                }
            }
            let first = &pack.0[0];
            let prior = first.head.sequence.checked_sub(1).map(|sequence| Head {
                sequence,
                hash: first.previous,
            });
            packs.push(pack);
            let Some(prior) = prior else {
                break;
            };
            expected = prior;
        }
        if self.anchor()? != anchor {
            return Err(Error::Stale);
        }
        packs.reverse();
        Ok(packs)
    }
    fn ensure_both(&mut self, pack: &Pack, fresh: bool) -> Result<(), Error> {
        let bytes = pack.bytes(self.stream)?;
        let end = pack.head().hash;
        for store in [&mut self.first as &mut dyn Store, &mut self.second] {
            if !fresh && store.get(end).is_ok_and(|copy| copy == bytes) {
                continue;
            }
            store.put(end, &bytes).map_err(|_| Error::Storage)?;
            if !store.get(end).is_ok_and(|copy| copy == bytes) {
                return Err(Error::Storage);
            }
        }
        Ok(())
    }
    /// Layout-independent ciphertext archive; same version as legacy Replicated.
    /// Includes every frame, not just a projection or a checkpoint.
    pub fn snapshot(&mut self) -> Result<Vec<u8>, Error> {
        let frames = self.load()?;
        encode_snapshot(self.stream, &frames)
    }
    /// Explicit import/repair ONLY against the independently current exact head.
    /// Never advances/resets the witness. Requires Journal::open to authenticate
    /// and replay the unchanged AEAD records; old writers must separately fence.
    pub fn restore_snapshot(&mut self, bytes: &[u8]) -> Result<(), Error> {
        let frames = decode_snapshot(self.stream, bytes)?;
        let anchor = self.anchor()?;
        if frames.last().map(|frame| frame.head) != anchor.head {
            return Err(Error::Stale);
        }
        for chunk in frames.chunks(PACK_RECORDS) {
            self.ensure_both(&Pack(chunk.to_vec()), false)?;
        }
        if self.anchor()? != anchor {
            return Err(Error::Stale);
        }
        Ok(())
    }
}
impl<A: Store, B: Store, W: Witness> Backend for Packed<A, B, W> {
    fn load(&mut self) -> Result<Vec<Frame>, Error> {
        Ok(self
            .history()?
            .into_iter()
            .flat_map(|pack| pack.0)
            .collect())
    }
    fn append(&mut self, expected: Option<Head>, frame: &Frame) -> Result<(), Error> {
        frame.validate()?;
        let anchor = self.anchor()?;
        if anchor.head != expected {
            return Err(Error::Stale);
        }
        let sequence = expected.map_or(Ok(0), |head| {
            head.sequence.checked_add(1).ok_or(Error::Limit)
        })?;
        if frame.head.sequence != sequence
            || frame.previous != expected.map_or([0; 32], |head| head.hash)
        {
            return Err(Error::Invalid);
        }
        if sequence >= MAX_RECORDS as u64 {
            return Err(Error::Limit);
        }
        let history = self.history()?;
        if history.last().map(Pack::head) != expected {
            return Err(Error::Stale);
        }
        let total = history.iter().flat_map(|pack| &pack.0).try_fold(
            frame.opaque.as_bytes().len(),
            |n, old| {
                n.checked_add(old.opaque.as_bytes().len())
                    .ok_or(Error::Limit)
            },
        )?;
        if total > MAX_HISTORY_BYTES {
            return Err(Error::Limit);
        }
        let mut next = if sequence.is_multiple_of(PACK_RECORDS as u64) {
            Vec::new()
        } else {
            history.last().ok_or(Error::Codec)?.0.clone()
        };
        next.push(frame.clone());
        // New immutable tail version first, then repair EVERY old required pack,
        // including the old partial tail. No earlier GET is reused as repair proof.
        self.ensure_both(&Pack(next), true)?;
        for pack in &history {
            self.ensure_both(pack, false)?;
        }
        self.witness
            .accept(self.stream, anchor, frame.head)
            .map_err(|error| {
                if error == Error::Stale {
                    error
                } else {
                    Error::Storage
                }
            })?;
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

/// Explicit local prototype object store. Two directories are NOT independent
/// failure domains. No plaintext/key provider or cloud fallback exists here.
pub struct FileStore {
    directory: PathBuf,
    identity: [u8; 32],
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
impl FileStore {
    /// New private directory only; never overwrites an existing replica.
    pub fn create(directory: &Path) -> Result<Self, Error> {
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        builder.mode(0o700);
        builder.create(directory).map_err(|_| Error::Storage)?;
        if let Some(parent) = directory.parent() {
            File::open(parent)
                .and_then(|file| file.sync_all())
                .map_err(|_| Error::Storage)?;
        }
        Self::open(directory)
    }
    /// Existing private directory only; object schema is verified by Packed.
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
impl Store for FileStore {
    fn identity(&self) -> [u8; 32] {
        self.identity
    }
    fn put(&mut self, end: [u8; 32], bytes: &[u8]) -> Result<(), Error> {
        if end == [0; 32] || bytes.len() > MAX_PACK_BYTES || bytes.is_empty() {
            return Err(Error::Limit);
        }
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
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| Error::Storage)?;
        fs::rename(tmp, self.directory.join(hex(&end))).map_err(|_| Error::Storage)?;
        File::open(&self.directory)
            .and_then(|file| file.sync_all())
            .map_err(|_| Error::Storage)
    }
    fn get(&mut self, end: [u8; 32]) -> Result<Vec<u8>, Error> {
        let path = self.directory.join(hex(&end));
        let meta = fs::symlink_metadata(&path).map_err(|_| Error::Storage)?;
        if !meta.is_file() || meta.file_type().is_symlink() {
            return Err(Error::Storage);
        }
        if meta.len() > MAX_PACK_BYTES as u64 {
            return Err(Error::Limit);
        }
        let mut bytes = Vec::new();
        File::open(path)
            .map_err(|_| Error::Storage)?
            .take(MAX_PACK_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Error::Storage)?;
        if bytes.len() > MAX_PACK_BYTES {
            return Err(Error::Limit);
        }
        Ok(bytes)
    }
}

#[cfg(test)]
#[path = "packed_tests.rs"]
mod tests;
