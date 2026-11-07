//! Local opaque storage adapter; no customer financial fields in SQL columns.

use crate::{
    Backend, Error, Frame, Head, MAX_HISTORY_BYTES, MAX_RECORDS, model::PrivateBytes,
    wire::MAX_RECORD,
};
use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior, params};
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::{
    fs::{self, File, OpenOptions},
    path::Path,
    time::Duration,
};

const APP_ID: i64 = 0x43494e44;
const TABLES: &str = "CREATE TABLE records(seq INTEGER PRIMARY KEY CHECK(seq>=0), previous BLOB NOT NULL CHECK(length(previous)=32), digest BLOB NOT NULL CHECK(length(digest)=32), opaque BLOB NOT NULL);";
const V2:&str="CREATE TABLE head(id INTEGER PRIMARY KEY CHECK(id=1), seq INTEGER NOT NULL, digest BLOB NOT NULL CHECK(length(digest)=32));
INSERT INTO head SELECT 1,seq,digest FROM records ORDER BY seq DESC LIMIT 1;
CREATE TRIGGER records_no_update BEFORE UPDATE ON records BEGIN SELECT RAISE(ABORT,'immutable journal'); END;
CREATE TRIGGER records_no_delete BEFORE DELETE ON records BEGIN SELECT RAISE(ABORT,'immutable journal'); END;
PRAGMA user_version=2;";

/// Explicitly authorized storage-layout migration; unknown versions always reject.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Migration {
    /// Open only the current schema.
    None,
    /// Preserve v1 record bytes and atomically add the v2 CAS head/index guards.
    V1ToV2,
}

/// Local SQLite backend. Use a trusted private local-filesystem directory, not
/// NFS/shared host authority. No raw private data or identity indexes reach SQL.
pub struct SqliteBackend {
    connection: Connection,
    #[cfg(feature = "test-hooks")]
    hook: Option<CommitHook>,
}
#[cfg(feature = "test-hooks")]
type CommitHook = Box<dyn FnMut(CommitPoint) -> Result<(), Error>>;
impl std::fmt::Debug for SqliteBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SqliteBackend([PATH REDACTED])")
    }
}
/// Precisely placed test-only interruption boundaries, absent from default builds.
#[cfg(feature = "test-hooks")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitPoint {
    /// Before starting the write transaction.
    BeforeBegin,
    /// Opaque body inserted, head unchanged, transaction uncommitted.
    AfterInsert,
    /// Body and head written but not committed.
    AfterHead,
    /// Immediately before COMMIT.
    BeforeCommit,
    /// COMMIT returned but caller has received no receipt/delivery.
    AfterCommit,
}

fn storage(_: rusqlite::Error) -> Error {
    Error::Storage
}
fn begin_error(e: rusqlite::Error) -> Error {
    if matches!(e,rusqlite::Error::SqliteFailure(ref failure,_) if failure.code==rusqlite::ErrorCode::DatabaseBusy || failure.code==rusqlite::ErrorCode::DatabaseLocked)
    {
        Error::Busy
    } else {
        Error::Storage
    }
}
fn permissions(path: &Path, dir: bool) -> Result<(), Error> {
    let m = fs::symlink_metadata(path).map_err(|_| Error::Storage)?;
    if m.file_type().is_symlink() || (dir && !m.is_dir()) || (!dir && !m.is_file()) {
        return Err(Error::Storage);
    }
    #[cfg(unix)]
    if m.permissions().mode() & 0o077 != 0 {
        return Err(Error::Storage);
    }
    Ok(())
}
fn configure(c: &Connection) -> Result<(), Error> {
    c.busy_timeout(Duration::from_millis(250))
        .map_err(storage)?;
    c.execute_batch("PRAGMA journal_mode=DELETE; PRAGMA synchronous=EXTRA; PRAGMA fullfsync=ON; PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF; PRAGMA temp_store=MEMORY;").map_err(storage)?;
    let sync: i64 = c
        .query_row("PRAGMA synchronous", [], |r| r.get(0))
        .map_err(storage)?;
    let mode: String = c
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .map_err(storage)?;
    if sync != 3 || mode != "delete" {
        return Err(Error::Storage);
    }
    Ok(())
}
fn head(c: &Connection) -> Result<Option<Head>, Error> {
    c.query_row("SELECT seq,digest FROM head WHERE id=1", [], |r| {
        let sequence: i64 = r.get(0)?;
        let bytes: Vec<u8> = r.get(1)?;
        Ok((sequence, bytes))
    })
    .optional()
    .map_err(storage)?
    .map(|(sequence, bytes)| {
        Ok(Head {
            sequence: sequence.try_into().map_err(|_| Error::Codec)?,
            hash: bytes.try_into().map_err(|_| Error::Codec)?,
        })
    })
    .transpose()
}
fn rows(c: &Connection) -> Result<Vec<Frame>, Error> {
    let mut stmt = c
        .prepare("SELECT seq,previous,digest,length(opaque),opaque,length(previous),length(digest) FROM records ORDER BY seq")
        .map_err(storage)?;
    let mut q = stmt.query([]).map_err(storage)?;
    let mut frames = Vec::new();
    let mut total = 0_usize;
    let mut prior = [0; 32];
    while let Some(r) = q.next().map_err(storage)? {
        let len: i64 = r.get(3).map_err(storage)?;
        let len: usize = len.try_into().map_err(|_| Error::Codec)?;
        if r.get::<_, i64>(5).map_err(storage)? != 32 || r.get::<_, i64>(6).map_err(storage)? != 32
        {
            return Err(Error::Codec);
        }
        total = total.checked_add(len).ok_or(Error::Limit)?;
        if len > MAX_RECORD || total > MAX_HISTORY_BYTES || frames.len() >= MAX_RECORDS {
            return Err(Error::Limit);
        }
        let sequence: i64 = r.get(0).map_err(storage)?;
        let sequence = sequence.try_into().map_err(|_| Error::Codec)?;
        let previous: Vec<u8> = r.get(1).map_err(storage)?;
        let digest: Vec<u8> = r.get(2).map_err(storage)?;
        let frame = Frame {
            head: Head {
                sequence,
                hash: digest.try_into().map_err(|_| Error::Codec)?,
            },
            previous: previous.try_into().map_err(|_| Error::Codec)?,
            opaque: PrivateBytes::new(r.get(4).map_err(storage)?)?,
        };
        frame.validate()?;
        if frame.head.sequence != frames.len() as u64 || frame.previous != prior {
            return Err(Error::Codec);
        }
        prior = frame.head.hash;
        frames.push(frame);
    }
    Ok(frames)
}
impl SqliteBackend {
    /// Create a new private directory/database. Existing paths are never overwritten.
    pub fn create(directory: &Path) -> Result<Self, Error> {
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        builder.mode(0o700);
        builder.create(directory).map_err(|_| Error::Storage)?;
        let db = directory.join("journal.sqlite3");
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        options
            .open(&db)
            .map_err(|_| Error::Storage)?
            .sync_all()
            .map_err(|_| Error::Storage)?;
        let mut c =
            Connection::open_with_flags(&db, OpenFlags::SQLITE_OPEN_READ_WRITE).map_err(storage)?;
        configure(&c)?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(begin_error)?;
        tx.execute_batch(TABLES).map_err(storage)?;
        tx.execute_batch(V2).map_err(storage)?;
        tx.pragma_update(None, "application_id", APP_ID)
            .map_err(storage)?;
        tx.commit().map_err(storage)?;
        File::open(directory)
            .map_err(|_| Error::Storage)?
            .sync_all()
            .map_err(|_| Error::Storage)?;
        if let Some(parent) = directory.parent() {
            File::open(parent)
                .map_err(|_| Error::Storage)?
                .sync_all()
                .map_err(|_| Error::Storage)?;
        }
        Ok(Self {
            connection: c,
            #[cfg(feature = "test-hooks")]
            hook: None,
        })
    }
    /// Open existing storage only, rejecting missing data, unsafe permissions and
    /// unsupported schemas. Migration is opt-in and never rewrites event bodies.
    pub fn open(directory: &Path, migration: Migration) -> Result<Self, Error> {
        permissions(directory, true)?;
        let db = directory.join("journal.sqlite3");
        permissions(&db, false)?;
        let mut c =
            Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_WRITE).map_err(storage)?;
        configure(&c)?;
        let app: i64 = c
            .query_row("PRAGMA application_id", [], |r| r.get(0))
            .map_err(storage)?;
        let version: i64 = c
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(storage)?;
        if app != APP_ID {
            return Err(Error::Version);
        }
        if version == 1 && migration == Migration::V1ToV2 {
            let tx = c
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(begin_error)?;
            rows(&tx)?;
            tx.execute_batch(V2).map_err(storage)?;
            tx.commit().map_err(storage)?;
        } else if version != 2 {
            return Err(Error::Version);
        }
        let integrity: String = c
            .query_row("PRAGMA quick_check", [], |r| r.get(0))
            .map_err(storage)?;
        if integrity != "ok" {
            return Err(Error::Codec);
        }
        Ok(Self {
            connection: c,
            #[cfg(feature = "test-hooks")]
            hook: None,
        })
    }
    /// Install a one-process test harness callback; unavailable in the default build.
    #[cfg(feature = "test-hooks")]
    pub fn set_test_hook(&mut self, hook: impl FnMut(CommitPoint) -> Result<(), Error> + 'static) {
        self.hook = Some(Box::new(hook));
    }
    /// Constrain SQLite to its current page count to exercise real SQLITE_FULL.
    #[cfg(feature = "test-hooks")]
    pub fn cap_test_pages(&mut self) -> Result<(), Error> {
        let pages: i64 = self
            .connection
            .query_row("PRAGMA page_count", [], |r| r.get(0))
            .map_err(storage)?;
        self.connection
            .pragma_update(None, "max_page_count", pages)
            .map_err(storage)
    }
}
impl Backend for SqliteBackend {
    fn load(&mut self) -> Result<Vec<Frame>, Error> {
        let tx = self.connection.transaction().map_err(storage)?;
        let expected = head(&tx)?;
        let frames = rows(&tx)?;
        if frames.last().map(|f| f.head) != expected {
            return Err(Error::Codec);
        }
        tx.commit().map_err(storage)?;
        Ok(frames)
    }
    fn append(&mut self, expected: Option<Head>, frame: &Frame) -> Result<(), Error> {
        frame.validate()?;
        let sequence = expected.map_or(Ok(0), |h| h.sequence.checked_add(1).ok_or(Error::Limit))?;
        if sequence != frame.head.sequence || frame.previous != expected.map_or([0; 32], |h| h.hash)
        {
            return Err(Error::Invalid);
        }
        #[cfg(feature = "test-hooks")]
        if let Some(h) = &mut self.hook {
            h(CommitPoint::BeforeBegin)?;
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(begin_error)?;
        if head(&tx)? != expected {
            return Err(Error::Stale);
        }
        let (count, total): (i64, i64) = tx
            .query_row(
                "SELECT count(*),coalesce(sum(length(opaque)),0) FROM records",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(storage)?;
        let count: usize = count.try_into().map_err(|_| Error::Codec)?;
        let total: usize = total.try_into().map_err(|_| Error::Codec)?;
        if count >= MAX_RECORDS
            || total.saturating_add(frame.opaque.as_bytes().len()) > MAX_HISTORY_BYTES
        {
            return Err(Error::Limit);
        }
        tx.execute(
            "INSERT INTO records(seq,previous,digest,opaque) VALUES(?1,?2,?3,?4)",
            params![
                i64::try_from(frame.head.sequence).map_err(|_| Error::Limit)?,
                &frame.previous[..],
                &frame.head.hash[..],
                frame.opaque.as_bytes()
            ],
        )
        .map_err(storage)?;
        #[cfg(feature = "test-hooks")]
        if let Some(h) = &mut self.hook {
            h(CommitPoint::AfterInsert)?;
        }
        tx.execute("INSERT INTO head(id,seq,digest) VALUES(1,?1,?2) ON CONFLICT(id) DO UPDATE SET seq=excluded.seq,digest=excluded.digest",params![i64::try_from(frame.head.sequence).map_err(|_| Error::Limit)?,&frame.head.hash[..]]).map_err(storage)?;
        #[cfg(feature = "test-hooks")]
        if let Some(h) = &mut self.hook {
            h(CommitPoint::AfterHead)?;
            h(CommitPoint::BeforeCommit)?;
        }
        tx.commit().map_err(storage)?;
        #[cfg(feature = "test-hooks")]
        if let Some(h) = &mut self.hook {
            h(CommitPoint::AfterCommit)?;
        }
        Ok(())
    }
}
