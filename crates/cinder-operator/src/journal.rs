use crate::{
    derive_venue_identity, BoundedIntent, ErrorCode, FillFact, Operation, OperationId,
    OrderIdentity, OrderKind, OrderState, VenueIdentity, VenueOid,
};
use fs2::FileExt;
use rusqlite::{params, Connection, Error as SqlError, OpenFlags, OptionalExtension};
use std::{
    collections::BTreeMap,
    fmt,
    fs::{self, File, OpenOptions},
    io,
    path::{Path, PathBuf},
    time::Duration,
};

#[cfg(unix)]
use std::os::unix::{fs::MetadataExt, fs::OpenOptionsExt, fs::PermissionsExt};

/// Refuse to open a newer journal rather than silently misinterpreting it.
pub const SCHEMA_VERSION: i64 = 2;

/// SQLite journal errors are intentionally local and typed.  Remote error text
/// belongs neither in this error type nor in persistent journal records.
#[derive(Debug)]
pub enum JournalError {
    Io(io::Error),
    Sql(SqlError),
    AlreadyLocked,
    UnsafePath(&'static str),
    FutureSchema(i64),
    CorruptState(&'static str),
    InvalidIntent(&'static str),
    InvalidFill(&'static str),
    IntentConflict,
    VenueOidCollision,
    StateConflict {
        expected: OrderState,
        found: OrderState,
    },
    InvalidTransition {
        from: OrderState,
        to: OrderState,
    },
    ArithmeticOverflow,
}

impl fmt::Display for JournalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "journal I/O error: {error}"),
            Self::Sql(error) => write!(f, "journal SQLite error: {error}"),
            Self::AlreadyLocked => write!(f, "another cinder operator journal writer is active"),
            Self::UnsafePath(reason) => write!(f, "unsafe journal path: {reason}"),
            Self::FutureSchema(version) => {
                write!(f, "journal schema {version} is newer than supported")
            }
            Self::CorruptState(reason) => write!(f, "corrupt journal state: {reason}"),
            Self::InvalidIntent(reason) => write!(f, "invalid bounded intent: {reason}"),
            Self::InvalidFill(reason) => write!(f, "invalid venue fill: {reason}"),
            Self::IntentConflict => write!(
                f,
                "same operation identity has conflicting immutable intent"
            ),
            Self::VenueOidCollision => write!(f, "global Phoenix venue ID collision"),
            Self::StateConflict { expected, found } => {
                write!(
                    f,
                    "journal state conflict: expected {expected:?}, found {found:?}"
                )
            }
            Self::InvalidTransition { from, to } => {
                write!(f, "invalid journal transition {from:?} -> {to:?}")
            }
            Self::ArithmeticOverflow => write!(f, "journal arithmetic overflow"),
        }
    }
}

impl std::error::Error for JournalError {}

impl From<io::Error> for JournalError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<SqlError> for JournalError {
    fn from(value: SqlError) -> Self {
        Self::Sql(value)
    }
}

/// Safe public status for the local administration binary.  It intentionally
/// exposes counts only—not operation identifiers, users, payloads, or errors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalStatus {
    pub schema_version: i64,
    pub sqlite_version: String,
    pub operation_counts: BTreeMap<OrderState, u64>,
}

/// One SQLite connection and one non-blocking journal-file lifetime lock.
/// OperatorRuntime additionally holds the shared pool lease; neither lock
/// provides multi-host or distributed fencing.
pub struct Journal {
    connection: Connection,
    _writer_lock: File,
    path: PathBuf,
}

impl Journal {
    /// Pin the journal to one deployment, pool and operator. Changing a URL
    /// does not change this binding; changing an account identity does.
    pub fn bind_runtime(&mut self, binding: [u8; 32]) -> Result<(), JournalError> {
        let bound: bool =
            self.connection
                .query_row("SELECT EXISTS(SELECT 1 FROM runtime_binding)", [], |r| {
                    r.get(0)
                })?;
        let active: bool = self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM operations WHERE state!=1)",
            [],
            |r| r.get(0),
        )?;
        // Do not retroactively attest the pre-send discipline of an unbound
        // legacy/test journal. Unknown historical sends need explicit repair.
        if !bound && active {
            return Err(JournalError::IntentConflict);
        }
        self.connection.execute(
            "INSERT OR IGNORE INTO runtime_binding VALUES (1, ?)",
            params![binding.as_slice()],
        )?;
        let found: Vec<u8> = self.connection.query_row(
            "SELECT binding FROM runtime_binding WHERE singleton=1",
            [],
            |r| r.get(0),
        )?;
        if found != binding {
            return Err(JournalError::IntentConflict);
        }
        Ok(())
    }

    /// Monotonic local registry: a user that disappears from a later scan
    /// cannot silently disappear from reconciliation.
    pub fn remember_users(
        &mut self,
        users: &[(crate::PubkeyBytes, crate::PubkeyBytes)],
    ) -> Result<(), JournalError> {
        let tx = self.connection.transaction()?;
        for (ledger, user) in users {
            tx.execute(
                "INSERT OR IGNORE INTO users VALUES (?, ?)",
                params![ledger.as_slice(), user.as_slice()],
            )?;
            let found: Vec<u8> = tx.query_row(
                "SELECT user_pubkey FROM users WHERE user_ledger=?",
                params![ledger.as_slice()],
                |r| r.get(0),
            )?;
            if found != user {
                return Err(JournalError::IntentConflict);
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn known_users(
        &self,
    ) -> Result<Vec<(crate::PubkeyBytes, crate::PubkeyBytes)>, JournalError> {
        let mut stmt = self
            .connection
            .prepare("SELECT user_ledger,user_pubkey FROM users ORDER BY user_ledger")?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, Vec<u8>>(1)?))
        })?;
        rows.map(|row| {
            let (l, u) = row?;
            Ok((
                l.try_into()
                    .map_err(|_| JournalError::CorruptState("invalid registry ledger"))?,
                u.try_into()
                    .map_err(|_| JournalError::CorruptState("invalid registry user"))?,
            ))
        })
        .collect()
    }

    pub fn record_prepared_ack(
        &mut self,
        id: &OperationId,
        ack: &crate::PreparedAck,
    ) -> Result<(), JournalError> {
        let op = self.operation(id)?;
        if !matches!(
            op.state,
            OrderState::AckSubmissionIntent | OrderState::FailSubmissionIntent
        ) {
            return Err(JournalError::CorruptState(
                "ack preparation outside write-ahead state",
            ));
        }
        self.connection.execute(
            "INSERT INTO ack_attempts VALUES (?,?,?,?)",
            params![
                id.as_slice(),
                ack.signature.as_slice(),
                ack.last_valid_block_height.to_be_bytes().as_slice(),
                ack.observed_ledger_nonce.to_be_bytes().as_slice()
            ],
        )?;
        Ok(())
    }

    pub fn ack_attempts(&self, id: &OperationId) -> Result<Vec<crate::PreparedAck>, JournalError> {
        let mut stmt=self.connection.prepare("SELECT signature,expiry_height,observed_nonce FROM ack_attempts WHERE operation_id=? ORDER BY rowid")?;
        let rows = stmt.query_map(params![id.as_slice()], |r| {
            Ok((
                r.get::<_, Vec<u8>>(0)?,
                r.get::<_, Vec<u8>>(1)?,
                r.get::<_, Vec<u8>>(2)?,
            ))
        })?;
        rows.map(|row| {
            let (s, h, n) = row?;
            Ok(crate::PreparedAck {
                signature: s
                    .try_into()
                    .map_err(|_| JournalError::CorruptState("invalid ack signature"))?,
                last_valid_block_height: u64::from_be_bytes(
                    h.try_into()
                        .map_err(|_| JournalError::CorruptState("invalid ack expiry"))?,
                ),
                observed_ledger_nonce: u64::from_be_bytes(
                    n.try_into()
                        .map_err(|_| JournalError::CorruptState("invalid ack nonce"))?,
                ),
            })
        })
        .collect()
    }
    /// Open (or initialize) a private SQLite WAL journal.
    ///
    /// On Unix the directory is 0700 and the database, lock, WAL, and shared
    /// memory files are 0600.  Existing directories/files with group or world
    /// permissions, symlinks, or non-regular files are rejected.  The parent
    /// directory is operator-controlled and 0700, which also prevents an
    /// untrusted path-replacement race after these checks.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, JournalError> {
        let path = path.as_ref().to_path_buf();
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .ok_or(JournalError::UnsafePath(
                "journal path has no parent directory",
            ))?;
        ensure_private_dir(parent)?;
        // Resolve trusted ancestor aliases (e.g. macOS /var -> /private/var)
        // before SQLite NOFOLLOW checks, but never accept a linked leaf/parent.
        let path = parent.canonicalize()?.join(
            path.file_name()
                .ok_or(JournalError::UnsafePath("journal file name is missing"))?,
        );
        let parent = path
            .parent()
            .ok_or(JournalError::UnsafePath("missing parent"))?;
        ensure_private_regular_or_missing(&path)?;
        for sidecar in ["-wal", "-shm", "-journal"] {
            ensure_private_regular_or_missing(&sidecar_path(&path, sidecar))?;
        }

        let lock_path = lock_path(&path)?;
        ensure_private_regular_or_missing(&lock_path)?;
        let writer_lock = open_private_file(&lock_path)?;
        match writer_lock.try_lock_exclusive() {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                return Err(JournalError::AlreadyLocked)
            }
            Err(error) => return Err(JournalError::Io(error)),
        }

        // Precreate privately so SQLite also gives its sidecars private modes.
        let database_file = open_private_file(&path)?;
        database_file.sync_all()?;
        File::open(parent)?.sync_all()?;
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_NOFOLLOW;
        let connection = Connection::open_with_flags(&path, flags)?;
        connection.busy_timeout(Duration::from_secs(2))?;
        connection.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = FULL;
             PRAGMA trusted_schema = OFF;",
        )?;
        enforce_private_sqlite_sidecars(&path)?;

        let journal = Self {
            connection,
            _writer_lock: writer_lock,
            path,
        };
        journal.validate_or_initialize_schema()?;
        Ok(journal)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Store an immutable intent.  Exact replays return the existing operation;
    /// same-identity changes are rejected before any venue side effect.
    pub fn prepare_intent(&mut self, intent: BoundedIntent) -> Result<Operation, JournalError> {
        self.prepare_intent_inner(intent, None)
    }

    fn prepare_intent_inner(
        &mut self,
        intent: BoundedIntent,
        override_venue_oid: Option<VenueOid>,
    ) -> Result<Operation, JournalError> {
        intent.validate().map_err(JournalError::InvalidIntent)?;
        let venue_identity = derive_venue_identity(&intent.identity);
        let venue_oid = override_venue_oid.unwrap_or(venue_identity.venue_oid);

        if let Some(existing) = self.operation_by_identity(&intent.identity)? {
            if existing.intent == intent
                && existing.venue_identity.full_hash == venue_identity.full_hash
                && existing.venue_identity.preimage == venue_identity.preimage
                && existing.venue_identity.venue_oid == venue_oid
            {
                return Ok(existing);
            }
            return Err(JournalError::IntentConflict);
        }

        let tx = self.connection.transaction()?;
        let inserted = tx.execute(
            "INSERT INTO operations (
                operation_id, user_ledger, user_pubkey, user_nonce, client_oid, kind,
                venue_oid, derivation_preimage, asset_id, requested_lots,
                limit_price_ticks, last_valid_slot, post_fail_position_im_usdc,
                state, venue_signature, er_ack_signature, filled_lots,
                fill_vwap_quote_lots, fee_usdc, venue_observed_at_ms, last_error_code,
                retry_count, created_at_ms, updated_at_ms
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, NULL, 0, 0, ?, NULL, NULL, ?, ?, ?)",
            params![
                venue_identity.full_hash.as_slice(),
                intent.identity.user_ledger.as_slice(),
                intent.identity.user_pubkey.as_slice(),
                u64_blob(intent.identity.user_nonce).as_slice(),
                intent.identity.client_oid.as_slice(),
                intent.identity.kind as i64,
                venue_oid.as_slice(),
                venue_identity.preimage,
                i64::from(intent.asset_id),
                intent.requested_lots,
                u64_blob(intent.limit_price_ticks).as_slice(),
                u64_blob(intent.last_valid_slot).as_slice(),
                u64_blob(intent.post_fail_position_im_usdc).as_slice(),
                OrderState::Prepared as i64,
                u64_blob(0).as_slice(),
                u64_blob(0).as_slice(),
                u64_blob(intent.created_at_ms).as_slice(),
                u64_blob(intent.created_at_ms).as_slice(),
            ],
        );
        match inserted {
            Ok(_) => tx.commit()?,
            Err(SqlError::SqliteFailure(error, _))
                if matches!(
                    error.extended_code,
                    rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE
                        | rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY
                ) =>
            {
                return Err(JournalError::VenueOidCollision)
            }
            Err(error) => return Err(JournalError::Sql(error)),
        }
        self.operation(&venue_identity.full_hash)
    }

    /// Persist the pre-send boundary.  A process crash after this call is never
    /// allowed to cause an automatic resubmission.
    pub fn begin_venue_submission(
        &mut self,
        operation_id: &OperationId,
        now_ms: u64,
    ) -> Result<Operation, JournalError> {
        self.transition(
            operation_id,
            OrderState::Prepared,
            OrderState::SubmissionIntent,
            now_ms,
            None,
        )
    }

    pub fn record_venue_submission(
        &mut self,
        operation_id: &OperationId,
        signature: [u8; 64],
        now_ms: u64,
    ) -> Result<Operation, JournalError> {
        self.transition_with_signature(
            operation_id,
            OrderState::SubmissionIntent,
            OrderState::Submitted,
            now_ms,
            signature,
            false,
        )
    }

    /// Only authoritative proof that no submission reached the venue can move
    /// a pre-send ambiguity back to `Prepared`.
    pub fn record_definitely_never_submitted(
        &mut self,
        operation_id: &OperationId,
        now_ms: u64,
    ) -> Result<Operation, JournalError> {
        let operation = self.operation(operation_id)?;
        if operation.filled_lots != 0 {
            return Err(JournalError::InvalidTransition {
                from: operation.state,
                to: OrderState::Prepared,
            });
        }
        self.transition_from_any(
            operation_id,
            &[
                OrderState::Prepared,
                OrderState::SubmissionIntent,
                OrderState::Submitted,
                OrderState::Reconciling,
            ],
            OrderState::Prepared,
            now_ms,
            None,
        )
    }

    pub fn record_venue_open(
        &mut self,
        operation_id: &OperationId,
        now_ms: u64,
    ) -> Result<Operation, JournalError> {
        self.transition_from_any(
            operation_id,
            &[
                OrderState::Prepared,
                OrderState::SubmissionIntent,
                OrderState::Submitted,
                OrderState::VenueOpen,
                OrderState::Reconciling,
            ],
            OrderState::VenueOpen,
            now_ms,
            None,
        )
    }

    pub fn record_unknown_venue_outcome(
        &mut self,
        operation_id: &OperationId,
        now_ms: u64,
    ) -> Result<Operation, JournalError> {
        self.transition_from_any(
            operation_id,
            &[
                OrderState::Prepared,
                OrderState::SubmissionIntent,
                OrderState::Submitted,
                OrderState::VenueOpen,
                OrderState::Reconciling,
            ],
            OrderState::Reconciling,
            now_ms,
            Some(ErrorCode::VenueUnavailable),
        )
    }

    /// Persist fill events and the checked aggregate atomically before an ER
    /// acknowledgement may be constructed.  Duplicate event IDs are harmless
    /// only if their complete typed facts exactly match.
    pub fn record_fill_facts(
        &mut self,
        operation_id: &OperationId,
        fills: &[FillFact],
        now_ms: u64,
    ) -> Result<Operation, JournalError> {
        if fills.is_empty() {
            return self.operation(operation_id);
        }
        let operation = self.operation(operation_id)?;
        if operation.state.is_terminal()
            || matches!(
                operation.state,
                OrderState::VenueRejected
                    | OrderState::AckSubmissionIntent
                    | OrderState::AckSubmitted
                    | OrderState::FailSubmissionIntent
                    | OrderState::FailSubmitted
                    | OrderState::Failed
            )
        {
            return Err(JournalError::InvalidTransition {
                from: operation.state,
                to: OrderState::VenueFilled,
            });
        }

        let tx = self.connection.transaction()?;
        let mut filled_lots = operation.filled_lots;
        let mut quote_lots = operation.fill_vwap_quote_lots;
        let mut fee_usdc = operation.fee_usdc;
        let mut observed_at = operation.venue_observed_at_ms;
        let mut new_facts = false;
        for fill in fills {
            fill.validate().map_err(JournalError::InvalidFill)?;
            if fill.filled_lots.signum() != operation.intent.requested_lots.signum() {
                return Err(JournalError::InvalidFill(
                    "fill direction differs from requested lots",
                ));
            }
            let existing = tx
                .query_row(
                    "SELECT filled_lots, vwap_quote_lots, fee_usdc, fill_price_ticks, observed_at_ms
                     FROM fill_facts WHERE operation_id = ? AND event_id = ?",
                    params![operation_id.as_slice(), fill.event_id.as_slice()],
                    |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, Vec<u8>>(2)?,
                            row.get::<_, Vec<u8>>(3)?,
                            row.get::<_, Vec<u8>>(4)?,
                        ))
                    },
                )
                .optional()?;
            if let Some((lots, quote, fee, price, observed)) = existing {
                if lots != fill.filled_lots
                    || quote != fill.vwap_quote_lots
                    || read_u64(&fee)? != fill.fee_usdc
                    || read_u64(&price)? != fill.fill_price_ticks
                    || read_u64(&observed)? != fill.observed_at_ms
                {
                    return Err(JournalError::CorruptState(
                        "duplicate fill ID has conflicting facts",
                    ));
                }
                continue;
            }
            filled_lots = filled_lots
                .checked_add(fill.filled_lots)
                .ok_or(JournalError::ArithmeticOverflow)?;
            quote_lots = quote_lots
                .checked_add(fill.vwap_quote_lots)
                .ok_or(JournalError::ArithmeticOverflow)?;
            fee_usdc = fee_usdc
                .checked_add(fill.fee_usdc)
                .ok_or(JournalError::ArithmeticOverflow)?;
            if filled_lots.unsigned_abs() > operation.intent.requested_lots.unsigned_abs() {
                return Err(JournalError::InvalidFill(
                    "aggregate fill exceeds requested lots",
                ));
            }
            observed_at = Some(observed_at.unwrap_or(0).max(fill.observed_at_ms));
            new_facts = true;
            tx.execute(
                "INSERT INTO fill_facts (
                    operation_id, event_id, filled_lots, vwap_quote_lots, fee_usdc,
                    fill_price_ticks, observed_at_ms
                 ) VALUES (?, ?, ?, ?, ?, ?, ?)",
                params![
                    operation_id.as_slice(),
                    fill.event_id.as_slice(),
                    fill.filled_lots,
                    fill.vwap_quote_lots,
                    u64_blob(fill.fee_usdc).as_slice(),
                    u64_blob(fill.fill_price_ticks).as_slice(),
                    u64_blob(fill.observed_at_ms).as_slice(),
                ],
            )?;
        }
        if !new_facts {
            return Ok(operation);
        }
        let observed_blob = observed_at.map(|value| u64_blob(value).to_vec());
        tx.execute(
            "UPDATE operations
             SET filled_lots = ?, fill_vwap_quote_lots = ?, fee_usdc = ?,
                 venue_observed_at_ms = ?, last_error_code = NULL,
                 updated_at_ms = ?
             WHERE operation_id = ?",
            params![
                filled_lots,
                quote_lots,
                u64_blob(fee_usdc).as_slice(),
                observed_blob,
                u64_blob(now_ms).as_slice(),
                operation_id.as_slice(),
            ],
        )?;
        tx.commit()?;
        self.operation(operation_id)
    }

    /// Fill facts alone do not prove the IOC remainder is terminal.
    pub fn record_venue_filled(
        &mut self,
        operation_id: &OperationId,
        now_ms: u64,
    ) -> Result<Operation, JournalError> {
        let operation = self.operation(operation_id)?;
        if operation.filled_lots == 0 {
            return Err(JournalError::InvalidFill("terminal fill has no fill facts"));
        }
        self.transition_from_any(
            operation_id,
            &[
                OrderState::Prepared,
                OrderState::SubmissionIntent,
                OrderState::Submitted,
                OrderState::VenueOpen,
                OrderState::Reconciling,
                OrderState::VenueFilled,
            ],
            OrderState::VenueFilled,
            now_ms,
            None,
        )
    }

    /// A no-fill, authoritative venue rejection is the only path to a fail
    /// acknowledgement.  A fill is irreversible and cannot transition here.
    pub fn record_venue_rejected(
        &mut self,
        operation_id: &OperationId,
        now_ms: u64,
    ) -> Result<Operation, JournalError> {
        let operation = self.operation(operation_id)?;
        if operation.filled_lots != 0 || matches!(operation.state, OrderState::VenueFilled) {
            return Err(JournalError::InvalidTransition {
                from: operation.state,
                to: OrderState::VenueRejected,
            });
        }
        self.transition_from_any(
            operation_id,
            &[
                OrderState::Prepared,
                OrderState::SubmissionIntent,
                OrderState::Submitted,
                OrderState::VenueOpen,
                OrderState::Reconciling,
                OrderState::VenueRejected,
            ],
            OrderState::VenueRejected,
            now_ms,
            None,
        )
    }

    /// Partial fills are durable but not acknowledged until authoritative
    /// remainder handling is implemented. Never guess a full fill or a
    /// fail-ack for the remaining tentative lots.
    pub fn fill_is_complete(&self, operation: &Operation) -> bool {
        operation.filled_lots == operation.intent.requested_lots
    }

    pub fn begin_fill_ack(
        &mut self,
        operation_id: &OperationId,
        now_ms: u64,
    ) -> Result<Operation, JournalError> {
        let operation = self.operation(operation_id)?;
        if !self.fill_is_complete(&operation) {
            return Err(JournalError::InvalidTransition {
                from: operation.state,
                to: OrderState::AckSubmissionIntent,
            });
        }
        self.transition(
            operation_id,
            OrderState::VenueFilled,
            OrderState::AckSubmissionIntent,
            now_ms,
            None,
        )
    }

    pub fn record_fill_ack_submission(
        &mut self,
        operation_id: &OperationId,
        signature: [u8; 64],
        now_ms: u64,
    ) -> Result<Operation, JournalError> {
        self.transition_with_signature(
            operation_id,
            OrderState::AckSubmissionIntent,
            OrderState::AckSubmitted,
            now_ms,
            signature,
            true,
        )
    }

    pub fn begin_fail_ack(
        &mut self,
        operation_id: &OperationId,
        now_ms: u64,
    ) -> Result<Operation, JournalError> {
        self.transition(
            operation_id,
            OrderState::VenueRejected,
            OrderState::FailSubmissionIntent,
            now_ms,
            None,
        )
    }

    pub fn record_fail_ack_submission(
        &mut self,
        operation_id: &OperationId,
        signature: [u8; 64],
        now_ms: u64,
    ) -> Result<Operation, JournalError> {
        self.transition_with_signature(
            operation_id,
            OrderState::FailSubmissionIntent,
            OrderState::FailSubmitted,
            now_ms,
            signature,
            true,
        )
    }

    /// Retry only when the private port proves the earlier ack cannot land.
    pub(crate) fn retry_ack_intent(
        &mut self,
        operation_id: &OperationId,
        fill: bool,
        now_ms: u64,
    ) -> Result<Operation, JournalError> {
        if fill {
            self.transition_from_any(
                operation_id,
                &[
                    OrderState::VenueFilled,
                    OrderState::AckSubmissionIntent,
                    OrderState::AckSubmitted,
                ],
                OrderState::AckSubmissionIntent,
                now_ms,
                None,
            )
        } else {
            self.transition_from_any(
                operation_id,
                &[
                    OrderState::VenueRejected,
                    OrderState::FailSubmissionIntent,
                    OrderState::FailSubmitted,
                ],
                OrderState::FailSubmissionIntent,
                now_ms,
                None,
            )
        }
    }

    /// Marking an acknowledgement terminal is intentionally separate from its
    /// submission.  The coordinator calls this only after a matching full
    /// identity attestation and a fresh I1/I2 reconciliation snapshot.
    pub fn finalize_ack(
        &mut self,
        operation_id: &OperationId,
        fill: bool,
        now_ms: u64,
    ) -> Result<Operation, JournalError> {
        let expected = if fill {
            &[
                OrderState::VenueFilled,
                OrderState::AckSubmissionIntent,
                OrderState::AckSubmitted,
            ][..]
        } else {
            &[
                OrderState::VenueRejected,
                OrderState::FailSubmissionIntent,
                OrderState::FailSubmitted,
            ][..]
        };
        let next = if fill {
            OrderState::Acked
        } else {
            OrderState::Failed
        };
        self.transition_from_any(operation_id, expected, next, now_ms, None)
    }

    pub fn operation(&self, operation_id: &OperationId) -> Result<Operation, JournalError> {
        self.connection
            .query_row(
                &operation_select_sql("WHERE operation_id = ?"),
                params![operation_id.as_slice()],
                decode_operation,
            )
            .map_err(JournalError::Sql)
    }

    pub fn nonterminal_operations(&self) -> Result<Vec<Operation>, JournalError> {
        let mut statement = self.connection.prepare(&operation_select_sql(
            "WHERE state NOT IN (10, 13) ORDER BY created_at_ms, operation_id",
        ))?;
        let rows = statement.query_map([], decode_operation)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(JournalError::Sql)
    }

    pub fn fill_facts(&self, operation_id: &OperationId) -> Result<Vec<FillFact>, JournalError> {
        let mut statement = self.connection.prepare(
            "SELECT event_id, filled_lots, vwap_quote_lots, fee_usdc, fill_price_ticks, observed_at_ms
             FROM fill_facts WHERE operation_id = ? ORDER BY observed_at_ms, event_id",
        )?;
        let rows = statement.query_map(params![operation_id.as_slice()], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Vec<u8>>(3)?,
                row.get::<_, Vec<u8>>(4)?,
                row.get::<_, Vec<u8>>(5)?,
            ))
        })?;
        let mut facts = Vec::new();
        for row in rows {
            let (event_id, filled_lots, vwap_quote_lots, fee, price, observed) = row?;
            facts.push(FillFact {
                event_id: fixed::<32>(&event_id)?,
                filled_lots,
                vwap_quote_lots,
                fee_usdc: read_u64(&fee)?,
                fill_price_ticks: read_u64(&price)?,
                observed_at_ms: read_u64(&observed)?,
            });
        }
        Ok(facts)
    }

    pub fn status(&self) -> Result<JournalStatus, JournalError> {
        let sqlite_version = self
            .connection
            .query_row("SELECT sqlite_version()", [], |row| row.get(0))?;
        let mut statement = self
            .connection
            .prepare("SELECT state, COUNT(*) FROM operations GROUP BY state ORDER BY state")?;
        let rows =
            statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?;
        let mut operation_counts = BTreeMap::new();
        for row in rows {
            let (state, count) = row?;
            let count = u64::try_from(count)
                .map_err(|_| JournalError::CorruptState("negative operation count"))?;
            operation_counts.insert(
                OrderState::from_db(state).map_err(JournalError::CorruptState)?,
                count,
            );
        }
        Ok(JournalStatus {
            schema_version: SCHEMA_VERSION,
            sqlite_version,
            operation_counts,
        })
    }

    pub(crate) fn mark_error(
        &mut self,
        operation_id: &OperationId,
        code: ErrorCode,
        now_ms: u64,
    ) -> Result<(), JournalError> {
        self.connection.execute(
            "UPDATE operations SET last_error_code = ?, updated_at_ms = ? WHERE operation_id = ?",
            params![
                code as i64,
                u64_blob(now_ms).as_slice(),
                operation_id.as_slice()
            ],
        )?;
        Ok(())
    }

    pub fn operation_by_identity(
        &self,
        identity: &OrderIdentity,
    ) -> Result<Option<Operation>, JournalError> {
        self.connection
            .query_row(
                &operation_select_sql(
                    "WHERE user_ledger = ? AND user_pubkey = ? AND user_nonce = ?
                       AND client_oid = ? AND kind = ?",
                ),
                params![
                    identity.user_ledger.as_slice(),
                    identity.user_pubkey.as_slice(),
                    u64_blob(identity.user_nonce).as_slice(),
                    identity.client_oid.as_slice(),
                    identity.kind as i64,
                ],
                decode_operation,
            )
            .optional()
            .map_err(JournalError::Sql)
    }

    fn transition(
        &mut self,
        operation_id: &OperationId,
        expected: OrderState,
        next: OrderState,
        now_ms: u64,
        error_code: Option<ErrorCode>,
    ) -> Result<Operation, JournalError> {
        self.transition_from_any(operation_id, &[expected], next, now_ms, error_code)
    }

    fn transition_with_signature(
        &mut self,
        operation_id: &OperationId,
        expected: OrderState,
        next: OrderState,
        now_ms: u64,
        signature: [u8; 64],
        er_ack: bool,
    ) -> Result<Operation, JournalError> {
        if !transition_allowed(expected, next) {
            return Err(JournalError::InvalidTransition {
                from: expected,
                to: next,
            });
        }
        let current = self.operation(operation_id)?;
        let recorded_signature = if er_ack {
            current.er_ack_signature
        } else {
            current.venue_signature
        };
        if current.state == next && recorded_signature == Some(signature) {
            return Ok(current);
        }
        if current.state != expected {
            return Err(JournalError::StateConflict {
                expected,
                found: current.state,
            });
        }
        let retry_count = current.retry_count;
        let signature_column = if er_ack {
            "er_ack_signature"
        } else {
            "venue_signature"
        };
        let sql = format!(
            "UPDATE operations
             SET state = ?, {signature_column} = ?, last_error_code = NULL,
                 retry_count = ?, updated_at_ms = ?
             WHERE operation_id = ? AND state = ?"
        );
        let changed = self.connection.execute(
            &sql,
            params![
                next as i64,
                signature.as_slice(),
                u64_blob(retry_count).as_slice(),
                u64_blob(now_ms).as_slice(),
                operation_id.as_slice(),
                expected as i64,
            ],
        )?;
        if changed != 1 {
            return self.state_conflict(operation_id, expected);
        }
        self.operation(operation_id)
    }

    fn transition_from_any(
        &mut self,
        operation_id: &OperationId,
        expected: &[OrderState],
        next: OrderState,
        now_ms: u64,
        error_code: Option<ErrorCode>,
    ) -> Result<Operation, JournalError> {
        if expected.is_empty() {
            return Err(JournalError::CorruptState("empty state expectation"));
        }
        let current = self.operation(operation_id)?;
        if current.state == next {
            return Ok(current);
        }
        if !expected.contains(&current.state) {
            return Err(JournalError::StateConflict {
                expected: expected[0],
                found: current.state,
            });
        }
        if !transition_allowed(current.state, next) {
            return Err(JournalError::InvalidTransition {
                from: current.state,
                to: next,
            });
        }
        let retry_count = if matches!(
            next,
            OrderState::SubmissionIntent
                | OrderState::AckSubmissionIntent
                | OrderState::FailSubmissionIntent
        ) {
            current
                .retry_count
                .checked_add(1)
                .ok_or(JournalError::ArithmeticOverflow)?
        } else {
            current.retry_count
        };
        let changed = self.connection.execute(
            "UPDATE operations
             SET state = ?, last_error_code = ?, retry_count = ?, updated_at_ms = ?
             WHERE operation_id = ? AND state = ?",
            params![
                next as i64,
                error_code.map(|code| code as i64),
                u64_blob(retry_count).as_slice(),
                u64_blob(now_ms).as_slice(),
                operation_id.as_slice(),
                current.state as i64,
            ],
        )?;
        if changed != 1 {
            return self.state_conflict(operation_id, current.state);
        }
        self.operation(operation_id)
    }

    fn state_conflict<T>(
        &self,
        operation_id: &OperationId,
        expected: OrderState,
    ) -> Result<T, JournalError> {
        let found = self.operation(operation_id)?.state;
        Err(JournalError::StateConflict { expected, found })
    }

    fn validate_or_initialize_schema(&self) -> Result<(), JournalError> {
        let version: i64 = self
            .connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version > SCHEMA_VERSION {
            return Err(JournalError::FutureSchema(version));
        }
        if version == 0 {
            let existing_tables: i64 = self.connection.query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
                [],
                |row| row.get(0),
            )?;
            if existing_tables != 0 {
                return Err(JournalError::CorruptState(
                    "unversioned database already contains tables",
                ));
            }
            self.connection.execute_batch(
                "BEGIN IMMEDIATE;
                 CREATE TABLE operations (
                    operation_id BLOB PRIMARY KEY NOT NULL CHECK(length(operation_id) = 32),
                    user_ledger BLOB NOT NULL CHECK(length(user_ledger) = 32),
                    user_pubkey BLOB NOT NULL CHECK(length(user_pubkey) = 32),
                    user_nonce BLOB NOT NULL CHECK(length(user_nonce) = 8),
                    client_oid BLOB NOT NULL CHECK(length(client_oid) = 16),
                    kind INTEGER NOT NULL CHECK(kind IN (1, 2)),
                    venue_oid BLOB NOT NULL UNIQUE CHECK(length(venue_oid) = 16),
                    derivation_preimage BLOB NOT NULL,
                    asset_id INTEGER NOT NULL CHECK(asset_id BETWEEN 1 AND 65535),
                    requested_lots INTEGER NOT NULL CHECK(requested_lots != 0),
                    limit_price_ticks BLOB NOT NULL CHECK(length(limit_price_ticks) = 8),
                    last_valid_slot BLOB NOT NULL CHECK(length(last_valid_slot) = 8),
                    post_fail_position_im_usdc BLOB NOT NULL CHECK(length(post_fail_position_im_usdc) = 8),
                    state INTEGER NOT NULL CHECK(state BETWEEN 1 AND 13),
                    venue_signature BLOB CHECK(venue_signature IS NULL OR length(venue_signature) = 64),
                    er_ack_signature BLOB CHECK(er_ack_signature IS NULL OR length(er_ack_signature) = 64),
                    filled_lots INTEGER NOT NULL,
                    fill_vwap_quote_lots INTEGER NOT NULL,
                    fee_usdc BLOB NOT NULL CHECK(length(fee_usdc) = 8),
                    venue_observed_at_ms BLOB CHECK(venue_observed_at_ms IS NULL OR length(venue_observed_at_ms) = 8),
                    last_error_code INTEGER CHECK(last_error_code IS NULL OR last_error_code BETWEEN 1 AND 4),
                    retry_count BLOB NOT NULL CHECK(length(retry_count) = 8),
                    created_at_ms BLOB NOT NULL CHECK(length(created_at_ms) = 8),
                    updated_at_ms BLOB NOT NULL CHECK(length(updated_at_ms) = 8),
                    UNIQUE(user_ledger, user_pubkey, user_nonce, client_oid, kind)
                 );
                 CREATE TABLE fill_facts (
                    operation_id BLOB NOT NULL REFERENCES operations(operation_id),
                    event_id BLOB NOT NULL CHECK(length(event_id) = 32),
                    filled_lots INTEGER NOT NULL CHECK(filled_lots != 0),
                    vwap_quote_lots INTEGER NOT NULL,
                    fee_usdc BLOB NOT NULL CHECK(length(fee_usdc) = 8),
                    fill_price_ticks BLOB NOT NULL CHECK(length(fill_price_ticks) = 8),
                    observed_at_ms BLOB NOT NULL CHECK(length(observed_at_ms) = 8),
                    PRIMARY KEY(operation_id, event_id)
                 );
                 PRAGMA user_version = 1;
                 COMMIT;"
            )?;
        } else if version != 1 && version != SCHEMA_VERSION {
            return Err(JournalError::CorruptState("unsupported prior schema"));
        }
        if version < 2 {
            self.connection.execute_batch("BEGIN IMMEDIATE;
                CREATE TABLE runtime_binding (singleton INTEGER PRIMARY KEY CHECK(singleton=1), binding BLOB NOT NULL CHECK(length(binding)=32));
                CREATE TABLE users (user_ledger BLOB PRIMARY KEY CHECK(length(user_ledger)=32),user_pubkey BLOB UNIQUE NOT NULL CHECK(length(user_pubkey)=32));
                CREATE TABLE ack_attempts (operation_id BLOB NOT NULL REFERENCES operations(operation_id),signature BLOB PRIMARY KEY CHECK(length(signature)=64),expiry_height BLOB NOT NULL CHECK(length(expiry_height)=8),observed_nonce BLOB NOT NULL CHECK(length(observed_nonce)=8));
                PRAGMA user_version=2; COMMIT;")?;
        }
        let integrity: String = self
            .connection
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        if integrity != "ok" {
            return Err(JournalError::CorruptState("SQLite integrity check failed"));
        }
        let orphan: Option<String> = self
            .connection
            .query_row("PRAGMA foreign_key_check", [], |row| row.get(0))
            .optional()?;
        if orphan.is_some() {
            return Err(JournalError::CorruptState("orphaned journal record"));
        }
        for table in [
            "operations",
            "fill_facts",
            "runtime_binding",
            "users",
            "ack_attempts",
        ] {
            let exists: Option<String> = self
                .connection
                .query_row(
                    "SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?",
                    params![table],
                    |row| row.get(0),
                )
                .optional()?;
            if exists.is_none() {
                return Err(JournalError::CorruptState("required table missing"));
            }
        }
        let mut statement = self.connection.prepare(&operation_select_sql(""))?;
        for row in statement.query_map([], decode_operation)? {
            let operation = row?;
            if (operation.state == OrderState::Submitted && operation.venue_signature.is_none())
                || (matches!(
                    operation.state,
                    OrderState::AckSubmitted | OrderState::FailSubmitted
                ) && operation.er_ack_signature.is_none())
            {
                return Err(JournalError::CorruptState(
                    "submitted state lacks signature",
                ));
            }
            let facts = self.fill_facts(&operation.operation_id)?;
            let mut lots = 0i64;
            let mut quote = 0i64;
            let mut fees = 0u64;
            for fact in facts {
                fact.validate().map_err(JournalError::CorruptState)?;
                if fact.filled_lots.signum() != operation.intent.requested_lots.signum() {
                    return Err(JournalError::CorruptState("fill direction mismatch"));
                }
                lots = lots
                    .checked_add(fact.filled_lots)
                    .ok_or(JournalError::ArithmeticOverflow)?;
                quote = quote
                    .checked_add(fact.vwap_quote_lots)
                    .ok_or(JournalError::ArithmeticOverflow)?;
                fees = fees
                    .checked_add(fact.fee_usdc)
                    .ok_or(JournalError::ArithmeticOverflow)?;
            }
            if lots != operation.filled_lots
                || quote != operation.fill_vwap_quote_lots
                || fees != operation.fee_usdc
                || lots.unsigned_abs() > operation.intent.requested_lots.unsigned_abs()
            {
                return Err(JournalError::CorruptState("fill aggregate mismatch"));
            }
            if matches!(
                operation.state,
                OrderState::VenueRejected
                    | OrderState::FailSubmissionIntent
                    | OrderState::FailSubmitted
                    | OrderState::Failed
            ) && lots != 0
            {
                return Err(JournalError::CorruptState("fill on failure path"));
            }
            if matches!(
                operation.state,
                OrderState::VenueFilled
                    | OrderState::AckSubmissionIntent
                    | OrderState::AckSubmitted
                    | OrderState::Acked
            ) && lots == 0
            {
                return Err(JournalError::CorruptState("fill state without facts"));
            }
            if matches!(
                operation.state,
                OrderState::AckSubmissionIntent | OrderState::AckSubmitted | OrderState::Acked
            ) && lots != operation.intent.requested_lots
            {
                return Err(JournalError::CorruptState(
                    "partial fill acknowledgement is unsupported",
                ));
            }
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn prepare_intent_with_venue_oid_for_test(
        &mut self,
        intent: BoundedIntent,
        venue_oid: VenueOid,
    ) -> Result<Operation, JournalError> {
        self.prepare_intent_inner(intent, Some(venue_oid))
    }
}

fn operation_select_sql(where_clause: &str) -> String {
    format!(
        "SELECT operation_id, user_ledger, user_pubkey, user_nonce, client_oid, kind,
                venue_oid, derivation_preimage, asset_id, requested_lots,
                limit_price_ticks, last_valid_slot, post_fail_position_im_usdc, state,
                venue_signature, er_ack_signature, filled_lots, fill_vwap_quote_lots,
                fee_usdc, venue_observed_at_ms, last_error_code, retry_count,
                created_at_ms, updated_at_ms
         FROM operations {where_clause}"
    )
}

fn decode_operation(row: &rusqlite::Row<'_>) -> rusqlite::Result<Operation> {
    let decode = || -> Result<Operation, JournalError> {
        let operation_id = fixed::<32>(&row.get::<_, Vec<u8>>(0)?)?;
        let user_ledger = fixed::<32>(&row.get::<_, Vec<u8>>(1)?)?;
        let user_pubkey = fixed::<32>(&row.get::<_, Vec<u8>>(2)?)?;
        let user_nonce = read_u64(&row.get::<_, Vec<u8>>(3)?)?;
        let client_oid = fixed::<16>(&row.get::<_, Vec<u8>>(4)?)?;
        let kind = OrderKind::from_db(row.get(5)?).map_err(JournalError::CorruptState)?;
        let venue_oid = fixed::<16>(&row.get::<_, Vec<u8>>(6)?)?;
        let preimage = row.get::<_, Vec<u8>>(7)?;
        let asset_id: i64 = row.get(8)?;
        let asset_id =
            u16::try_from(asset_id).map_err(|_| JournalError::CorruptState("invalid asset ID"))?;
        let requested_lots = row.get(9)?;
        let limit_price_ticks = read_u64(&row.get::<_, Vec<u8>>(10)?)?;
        let last_valid_slot = read_u64(&row.get::<_, Vec<u8>>(11)?)?;
        let post_fail_position_im_usdc = read_u64(&row.get::<_, Vec<u8>>(12)?)?;
        let state = OrderState::from_db(row.get(13)?).map_err(JournalError::CorruptState)?;
        let venue_signature = row
            .get::<_, Option<Vec<u8>>>(14)?
            .map(|value| fixed::<64>(&value))
            .transpose()?;
        let er_ack_signature = row
            .get::<_, Option<Vec<u8>>>(15)?
            .map(|value| fixed::<64>(&value))
            .transpose()?;
        let filled_lots = row.get(16)?;
        let fill_vwap_quote_lots = row.get(17)?;
        let fee_usdc = read_u64(&row.get::<_, Vec<u8>>(18)?)?;
        let venue_observed_at_ms = row
            .get::<_, Option<Vec<u8>>>(19)?
            .map(|value| read_u64(&value))
            .transpose()?;
        let last_error_code = row
            .get::<_, Option<i64>>(20)?
            .map(|value| ErrorCode::from_db(value).map_err(JournalError::CorruptState))
            .transpose()?;
        let retry_count = read_u64(&row.get::<_, Vec<u8>>(21)?)?;
        let created_at_ms = read_u64(&row.get::<_, Vec<u8>>(22)?)?;
        let updated_at_ms = read_u64(&row.get::<_, Vec<u8>>(23)?)?;
        let intent = BoundedIntent {
            identity: OrderIdentity {
                user_ledger,
                user_pubkey,
                user_nonce,
                client_oid,
                kind,
            },
            asset_id,
            requested_lots,
            limit_price_ticks,
            last_valid_slot,
            post_fail_position_im_usdc,
            created_at_ms,
        };
        intent.validate().map_err(JournalError::CorruptState)?;
        let derived = derive_venue_identity(&intent.identity);
        if derived.full_hash != operation_id
            || derived.preimage != preimage
            || derived.venue_oid != venue_oid
        {
            return Err(JournalError::CorruptState(
                "operation derivation does not match identity",
            ));
        }
        Ok(Operation {
            operation_id,
            venue_identity: VenueIdentity {
                venue_oid,
                full_hash: operation_id,
                preimage,
            },
            intent,
            state,
            venue_signature,
            er_ack_signature,
            filled_lots,
            fill_vwap_quote_lots,
            fee_usdc,
            venue_observed_at_ms,
            last_error_code,
            retry_count,
            updated_at_ms,
        })
    };
    decode().map_err(|error| {
        SqlError::FromSqlConversionFailure(0, rusqlite::types::Type::Blob, Box::new(error))
    })
}

fn transition_allowed(from: OrderState, to: OrderState) -> bool {
    if from == to {
        return true;
    }
    matches!(
        (from, to),
        (OrderState::Prepared, OrderState::SubmissionIntent)
            | (OrderState::Prepared, OrderState::Reconciling)
            | (OrderState::Prepared, OrderState::VenueOpen)
            | (OrderState::Prepared, OrderState::VenueFilled)
            | (OrderState::Prepared, OrderState::VenueRejected)
            | (OrderState::SubmissionIntent, OrderState::Submitted)
            | (OrderState::SubmissionIntent, OrderState::Prepared)
            | (OrderState::SubmissionIntent, OrderState::VenueOpen)
            | (OrderState::SubmissionIntent, OrderState::VenueFilled)
            | (OrderState::SubmissionIntent, OrderState::VenueRejected)
            | (OrderState::SubmissionIntent, OrderState::Reconciling)
            | (OrderState::Submitted, OrderState::VenueOpen)
            | (OrderState::Submitted, OrderState::VenueFilled)
            | (OrderState::Submitted, OrderState::VenueRejected)
            | (OrderState::Submitted, OrderState::Reconciling)
            | (OrderState::Submitted, OrderState::Prepared)
            | (OrderState::VenueOpen, OrderState::VenueFilled)
            | (OrderState::VenueOpen, OrderState::VenueRejected)
            | (OrderState::VenueOpen, OrderState::Reconciling)
            | (OrderState::Reconciling, OrderState::Prepared)
            | (OrderState::Reconciling, OrderState::VenueOpen)
            | (OrderState::Reconciling, OrderState::VenueFilled)
            | (OrderState::Reconciling, OrderState::VenueRejected)
            | (OrderState::VenueFilled, OrderState::AckSubmissionIntent)
            | (OrderState::AckSubmissionIntent, OrderState::AckSubmitted)
            | (OrderState::AckSubmitted, OrderState::AckSubmissionIntent)
            | (OrderState::AckSubmitted, OrderState::Acked)
            | (OrderState::AckSubmissionIntent, OrderState::Acked)
            | (OrderState::VenueFilled, OrderState::Acked)
            | (OrderState::VenueRejected, OrderState::FailSubmissionIntent)
            | (OrderState::FailSubmissionIntent, OrderState::FailSubmitted)
            | (OrderState::FailSubmitted, OrderState::FailSubmissionIntent)
            | (OrderState::FailSubmitted, OrderState::Failed)
            | (OrderState::FailSubmissionIntent, OrderState::Failed)
            | (OrderState::VenueRejected, OrderState::Failed)
    )
}

fn u64_blob(value: u64) -> [u8; 8] {
    // Big endian preserves unsigned time ordering in SQLite BLOB indexes.
    value.to_be_bytes()
}

fn read_u64(value: &[u8]) -> Result<u64, JournalError> {
    let fixed = fixed::<8>(value)?;
    Ok(u64::from_be_bytes(fixed))
}

fn fixed<const N: usize>(value: &[u8]) -> Result<[u8; N], JournalError> {
    value
        .try_into()
        .map_err(|_| JournalError::CorruptState("invalid fixed-width durable field"))
}

fn lock_path(path: &Path) -> Result<PathBuf, JournalError> {
    let name = path
        .file_name()
        .ok_or(JournalError::UnsafePath("journal file name is missing"))?;
    let mut lock_name = name.to_os_string();
    lock_name.push(".lock");
    Ok(path.with_file_name(lock_name))
}

pub(crate) fn pool_lock(directory: &Path, identity: &[u8; 32]) -> Result<File, JournalError> {
    ensure_private_dir(directory)?;
    let path = directory.canonicalize()?.join(format!(
        "pool-{}.lock",
        bs58::encode(identity).into_string()
    ));
    ensure_private_regular_or_missing(&path)?;
    let file = open_private_file(&path)?;
    file.try_lock_exclusive().map_err(|e| {
        if e.kind() == io::ErrorKind::WouldBlock {
            JournalError::AlreadyLocked
        } else {
            JournalError::Io(e)
        }
    })?;
    Ok(file)
}

fn ensure_private_dir(path: &Path) -> Result<(), JournalError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(JournalError::UnsafePath(
                    "journal parent must be a real directory",
                ));
            }
            #[cfg(unix)]
            if metadata.permissions().mode() & 0o077 != 0
                || metadata.uid() != unsafe { libc::geteuid() }
            {
                return Err(JournalError::UnsafePath(
                    "journal directory grants group or world access",
                ));
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            builder.create(path)?;
        }
        Err(error) => return Err(JournalError::Io(error)),
    }
    Ok(())
}

fn ensure_private_regular_or_missing(path: &Path) -> Result<(), JournalError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(JournalError::UnsafePath(
                    "journal file must be a regular file",
                ));
            }
            #[cfg(unix)]
            if metadata.permissions().mode() & 0o077 != 0
                || metadata.uid() != unsafe { libc::geteuid() }
                || metadata.nlink() != 1
            {
                return Err(JournalError::UnsafePath(
                    "journal file grants group or world access",
                ));
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(JournalError::Io(error)),
    }
    Ok(())
}

fn open_private_file(path: &Path) -> Result<File, JournalError> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);
    #[cfg(unix)]
    {
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    let file = options.open(path)?;
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(file)
}

fn enforce_private_sqlite_sidecars(path: &Path) -> Result<(), JournalError> {
    #[cfg(unix)]
    for candidate in [
        path.to_path_buf(),
        sidecar_path(path, "-wal"),
        sidecar_path(path, "-shm"),
    ] {
        if candidate.exists() {
            ensure_private_regular_or_missing(&candidate)?;
            fs::set_permissions(candidate, fs::Permissions::from_mode(0o600))?;
        }
    }
    Ok(())
}

fn sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_lease_fences_separate_journal_paths_and_releases_on_drop() {
        let dir = tempfile::tempdir().unwrap();
        let _first = Journal::open(dir.path().join("first/journal.sqlite")).unwrap();
        let _second = Journal::open(dir.path().join("second/journal.sqlite")).unwrap();
        let locks = dir.path().join("locks");
        let owner = pool_lock(&locks, &[7; 32]).unwrap();
        assert!(matches!(
            pool_lock(&locks, &[7; 32]),
            Err(JournalError::AlreadyLocked)
        ));
        let _other_pool = pool_lock(&locks, &[8; 32]).unwrap();
        drop(owner);
        pool_lock(&locks, &[7; 32]).unwrap();
    }
    use tempfile::TempDir;

    fn tempdir() -> TempDir {
        let mut builder = tempfile::Builder::new();
        #[cfg(unix)]
        builder.permissions(fs::Permissions::from_mode(0o700));
        builder.tempdir().unwrap()
    }

    fn intent(user: u8) -> BoundedIntent {
        BoundedIntent {
            identity: OrderIdentity {
                user_ledger: [user; 32],
                user_pubkey: [user; 32],
                user_nonce: u64::MAX,
                client_oid: [1; 16],
                kind: OrderKind::User,
            },
            asset_id: 1,
            requested_lots: 10,
            limit_price_ticks: u64::MAX,
            last_valid_slot: u64::MAX,
            post_fail_position_im_usdc: u64::MAX,
            created_at_ms: u64::MAX,
        }
    }

    fn fact(lots: i64, event: u8) -> FillFact {
        FillFact {
            event_id: [event; 32],
            filled_lots: lots,
            vwap_quote_lots: 100,
            fee_usdc: u64::MAX,
            fill_price_ticks: u64::MAX,
            observed_at_ms: u64::MAX,
        }
    }

    #[test]
    fn unsigned_fields_round_trip_without_sqlite_integer_truncation() {
        let dir = tempdir();
        let path = dir.path().join("journal.sqlite");
        let mut journal = Journal::open(&path).unwrap();
        let original = intent(1);
        let operation = journal.prepare_intent(original.clone()).unwrap();
        journal
            .begin_venue_submission(&operation.operation_id, u64::MAX)
            .unwrap();
        journal
            .record_fill_facts(&operation.operation_id, &[fact(10, 1)], u64::MAX)
            .unwrap();
        journal
            .record_venue_filled(&operation.operation_id, u64::MAX)
            .unwrap();
        drop(journal);
        let journal = Journal::open(&path).unwrap();
        let reloaded = journal.operation(&operation.operation_id).unwrap();
        assert_eq!(reloaded.intent, original);
        assert_eq!(reloaded.fee_usdc, u64::MAX);
        assert_eq!(reloaded.venue_observed_at_ms, Some(u64::MAX));
        assert_eq!(
            journal.fill_facts(&operation.operation_id).unwrap()[0].fill_price_ticks,
            u64::MAX
        );
        let version: Vec<u32> = journal
            .status()
            .unwrap()
            .sqlite_version
            .split('.')
            .map(|part| part.parse().unwrap())
            .collect();
        assert!(
            version.as_slice() >= [3, 51, 3].as_slice(),
            "patched WAL engine required"
        );
    }

    #[test]
    fn full_identity_is_idempotent_conflicts_fail_and_venue_collisions_are_guarded() {
        let dir = tempdir();
        let mut journal = Journal::open(dir.path().join("journal.sqlite")).unwrap();
        let operation = journal.prepare_intent(intent(1)).unwrap();
        assert_eq!(journal.prepare_intent(intent(1)).unwrap(), operation);
        let mut conflicting = intent(1);
        conflicting.requested_lots = 11;
        assert!(matches!(
            journal.prepare_intent(conflicting),
            Err(JournalError::IntentConflict)
        ));
        assert!(matches!(
            journal.prepare_intent_with_venue_oid_for_test(
                intent(2),
                operation.venue_identity.venue_oid
            ),
            Err(JournalError::VenueOidCollision)
        ));
        let second = journal.prepare_intent(intent(2)).unwrap();
        assert_ne!(
            second.venue_identity.venue_oid,
            operation.venue_identity.venue_oid
        );
        let mut reused = intent(1);
        reused.identity.user_nonce = 0;
        assert_ne!(
            journal
                .prepare_intent(reused)
                .unwrap()
                .venue_identity
                .venue_oid,
            operation.venue_identity.venue_oid
        );
    }

    #[test]
    fn fills_are_transactional_idempotent_and_irreversible() {
        let dir = tempdir();
        let mut journal = Journal::open(dir.path().join("journal.sqlite")).unwrap();
        let operation = journal.prepare_intent(intent(1)).unwrap();
        let id = operation.operation_id;
        journal.begin_venue_submission(&id, 0).unwrap();
        let first = journal.record_fill_facts(&id, &[fact(4, 1)], 0).unwrap();
        assert_eq!(
            journal.record_fill_facts(&id, &[fact(4, 1)], 0).unwrap(),
            first
        );
        assert!(journal.record_venue_rejected(&id, 0).is_err());
        assert!(
            journal.record_definitely_never_submitted(&id, 0).is_err(),
            "no reverse with fills"
        );
        // Overflow on the second fee rolls the entire transaction back.
        assert!(matches!(
            journal.record_fill_facts(&id, &[fact(6, 2)], 0),
            Err(JournalError::ArithmeticOverflow)
        ));
        assert_eq!(journal.operation(&id).unwrap().filled_lots, 4);
        assert_eq!(journal.fill_facts(&id).unwrap().len(), 1);
        let mut conflicting = fact(4, 1);
        conflicting.fee_usdc = 0;
        assert!(journal.record_fill_facts(&id, &[conflicting], 0).is_err());
    }

    #[test]
    fn second_writer_is_rejected_until_owner_drops() {
        let dir = tempdir();
        let path = dir.path().join("journal.sqlite");
        let owner = Journal::open(&path).unwrap();
        assert!(matches!(
            Journal::open(&path),
            Err(JournalError::AlreadyLocked)
        ));
        drop(owner);
        assert!(Journal::open(&path).is_ok());
    }

    #[test]
    fn future_schema_and_logically_corrupt_rows_fail_startup() {
        for corrupt in 0..3 {
            let dir = tempdir();
            let path = dir.path().join("journal.sqlite");
            let mut journal = Journal::open(&path).unwrap();
            journal.prepare_intent(intent(1)).unwrap();
            match corrupt {
                0 => journal
                    .connection
                    .execute_batch("PRAGMA user_version=99")
                    .unwrap(),
                1 => {
                    journal
                        .connection
                        .execute("UPDATE operations SET state=5", [])
                        .unwrap();
                }
                _ => {
                    journal
                        .connection
                        .execute(
                            "UPDATE operations SET venue_oid=?",
                            params![[0u8; 16].as_slice()],
                        )
                        .unwrap();
                }
            }
            drop(journal);
            assert!(Journal::open(&path).is_err());
        }
    }

    #[cfg(unix)]
    #[test]
    fn journal_and_sidecars_are_private_and_unsafe_paths_fail_closed() {
        use std::os::unix::fs::symlink;
        let dir = tempdir();
        let path = dir.path().join("journal.sqlite");
        let journal = Journal::open(&path).unwrap();
        for suffix in ["", "-wal", "-shm", ".lock"] {
            let candidate = sidecar_path(&path, suffix);
            if candidate.exists() {
                assert_eq!(
                    fs::metadata(candidate).unwrap().permissions().mode() & 0o777,
                    0o600
                );
            }
        }
        drop(journal);
        let alias = dir.path().join("alias.sqlite");
        symlink(&path, &alias).unwrap();
        assert!(matches!(
            Journal::open(alias),
            Err(JournalError::UnsafePath(_))
        ));
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(matches!(
            Journal::open(&path),
            Err(JournalError::UnsafePath(_))
        ));
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        let sidecar = sidecar_path(&path, "-wal");
        symlink(dir.path().join("elsewhere"), &sidecar).unwrap();
        assert!(matches!(
            Journal::open(&path),
            Err(JournalError::UnsafePath(_))
        ));
    }

    #[test]
    fn abrupt_process_exit_keeps_committed_wal_and_releases_lock() {
        let dir = tempdir();
        let path = dir.path().join("journal.sqlite");
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", "journal::tests::abrupt_exit_child"])
            .env("CINDER_JOURNAL_TEST_PATH", &path)
            .status()
            .unwrap();
        assert!(status.success());
        let journal = Journal::open(&path).unwrap();
        let operation = journal
            .operation(&derive_venue_identity(&intent(1).identity).full_hash)
            .unwrap();
        assert_eq!(operation.state, OrderState::AckSubmissionIntent);
        assert_eq!(operation.filled_lots, 10);
    }

    #[test]
    #[ignore = "subprocess fixture, invoked by abrupt_process_exit test"]
    fn abrupt_exit_child() {
        let path = std::env::var_os("CINDER_JOURNAL_TEST_PATH").expect("subprocess fixture path");
        let mut journal = Journal::open(PathBuf::from(path)).unwrap();
        let operation = journal.prepare_intent(intent(1)).unwrap();
        journal
            .begin_venue_submission(&operation.operation_id, 0)
            .unwrap();
        journal
            .record_fill_facts(&operation.operation_id, &[fact(10, 1)], 0)
            .unwrap();
        journal
            .record_venue_filled(&operation.operation_id, 0)
            .unwrap();
        journal.begin_fill_ack(&operation.operation_id, 0).unwrap();
        // Deliberately bypass Drop/checkpoint: simulate OS process termination.
        std::process::exit(0);
    }
}
