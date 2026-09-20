use serde::de::{Error as _, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::{File, OpenOptions, TryLockError};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

const LEDGER_FORMAT_VERSION: u32 = 1;
static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReservationState {
    Held,
    Settled,
    /// Outcome unknown (timeout, dropped connection, crash). Exposure is
    /// retained until an operator or reconciler resolves it.
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reservation {
    pub attempt_id: String,
    pub reserved_micro_usd: u64,
    pub settled_micro_usd: Option<u64>,
    pub state: ReservationState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    CapExceeded {
        cap: u64,
        held: u64,
        requested: u64,
    },
    SettlementExceedsReservation {
        attempt_id: String,
        reserved: u64,
        actual: u64,
    },
    DuplicateAttempt(String),
    UnknownAttempt(String),
    InvalidAttemptId,
    CapMismatch {
        persisted: u64,
        requested: u64,
    },
    LockUnavailable(String),
    CorruptState(String),
    UnsupportedVersion(u32),
    Persistence(String),
    Poisoned(String),
}

impl std::fmt::Display for LedgerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LedgerError::CapExceeded {
                cap,
                held,
                requested,
            } => write!(
                f,
                "local reservation cap {cap} micro-USD exceeded: held {held}, requested {requested}"
            ),
            LedgerError::SettlementExceedsReservation {
                attempt_id,
                reserved,
                actual,
            } => write!(
                f,
                "attempt `{attempt_id}` settlement {actual} micro-USD exceeds reservation {reserved}"
            ),
            LedgerError::DuplicateAttempt(id) => write!(f, "attempt `{id}` already reserved"),
            LedgerError::UnknownAttempt(id) => write!(f, "attempt `{id}` has no reservation"),
            LedgerError::InvalidAttemptId => {
                write!(f, "durable ledger attempt id must not be empty")
            }
            LedgerError::CapMismatch {
                persisted,
                requested,
            } => write!(
                f,
                "durable ledger cap mismatch: persisted {persisted}, requested {requested} micro-USD"
            ),
            LedgerError::LockUnavailable(path) => {
                write!(f, "durable ledger lock is already held: {path}")
            }
            LedgerError::CorruptState(message) => write!(f, "corrupt durable ledger: {message}"),
            LedgerError::UnsupportedVersion(version) => {
                write!(f, "unsupported durable ledger version {version}")
            }
            LedgerError::Persistence(message) => {
                write!(f, "durable ledger persistence failed: {message}")
            }
            LedgerError::Poisoned(message) => write!(f, "durable ledger is poisoned: {message}"),
        }
    }
}

impl std::error::Error for LedgerError {}

/// In-process accounting with optional durable state. Durable handles hold an
/// exclusive lock on a stable sidecar for their entire lifetime. Clones share
/// both the lock and the poison state.
#[derive(Debug, Clone)]
pub struct LocalLedger {
    inner: Arc<Mutex<LedgerInner>>,
    durable: Option<Arc<DurableLedger>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LedgerInner {
    cap_micro_usd: u64,
    reservations: BTreeMap<String, Reservation>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PersistedLedger {
    version: u32,
    cap_micro_usd: u64,
    #[serde(deserialize_with = "deserialize_reservations")]
    reservations: BTreeMap<String, Reservation>,
}

fn deserialize_reservations<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<String, Reservation>, D::Error>
where
    D: Deserializer<'de>,
{
    struct ReservationMapVisitor;

    impl<'de> Visitor<'de> for ReservationMapVisitor {
        type Value = BTreeMap<String, Reservation>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a reservation map with unique keys")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut reservations = BTreeMap::new();
            while let Some((key, reservation)) = map.next_entry::<String, Reservation>()? {
                if reservations.insert(key.clone(), reservation).is_some() {
                    return Err(A::Error::custom(format!(
                        "duplicate reservation key `{key}`"
                    )));
                }
            }
            Ok(reservations)
        }
    }

    deserializer.deserialize_map(ReservationMapVisitor)
}

#[derive(Debug)]
struct DurableLedger {
    path: PathBuf,
    _lock_file: File,
    poisoned: AtomicBool,
    #[cfg(test)]
    fail_after_rename_once: AtomicBool,
    #[cfg(test)]
    pause_after_rename: AtomicBool,
    #[cfg(test)]
    rename_reached: AtomicBool,
}

impl LedgerInner {
    fn checked_exposure_micro_usd(&self) -> Option<u64> {
        self.reservations
            .values()
            .try_fold(0u64, |total, reservation| {
                let exposure = match reservation.state {
                    ReservationState::Held => reservation.reserved_micro_usd,
                    ReservationState::Settled | ReservationState::Ambiguous => reservation
                        .settled_micro_usd
                        .unwrap_or(reservation.reserved_micro_usd),
                };
                total.checked_add(exposure)
            })
    }

    fn exposure_micro_usd(&self) -> u64 {
        self.checked_exposure_micro_usd().unwrap_or(u64::MAX)
    }

    fn validate(&self) -> Result<(), LedgerError> {
        for (key, reservation) in &self.reservations {
            if key.is_empty() || reservation.attempt_id.is_empty() || key != &reservation.attempt_id
            {
                return Err(LedgerError::CorruptState(format!(
                    "reservation key `{key}` does not match its attempt id"
                )));
            }
            match reservation.state {
                ReservationState::Held if reservation.settled_micro_usd.is_some() => {
                    return Err(LedgerError::CorruptState(format!(
                        "held reservation `{key}` has a settled amount"
                    )));
                }
                ReservationState::Settled if reservation.settled_micro_usd.is_none() => {
                    return Err(LedgerError::CorruptState(format!(
                        "settled reservation `{key}` has no settled amount"
                    )));
                }
                _ => {}
            }
        }
        Ok(())
    }
}

impl DurableLedger {
    fn sidecar_path(path: &Path) -> PathBuf {
        let mut name = path.as_os_str().to_os_string();
        name.push(".lock");
        PathBuf::from(name)
    }

    fn create_owned_temp(&self) -> std::io::Result<(File, PathBuf)> {
        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        let file_name = self.path.file_name().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "ledger path has no file name",
            )
        })?;
        for _ in 0..128 {
            let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
            let mut temp_name = OsString::from(file_name);
            temp_name.push(format!(".tmp.{}.{id}", std::process::id()));
            let temp_path = parent.join(temp_name);
            match OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temp_path)
            {
                Ok(file) => return Ok((file, temp_path)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "could not allocate a unique durable ledger temp file",
        ))
    }

    fn poison(&self, message: impl Into<String>) -> LedgerError {
        self.poisoned.store(true, Ordering::Release);
        LedgerError::Persistence(message.into())
    }

    fn ensure_usable(&self) -> Result<(), LedgerError> {
        if self.poisoned.load(Ordering::Acquire) {
            Err(LedgerError::Poisoned(format!(
                "state at {} may not be durably synchronized",
                self.path.display()
            )))
        } else {
            Ok(())
        }
    }

    fn persist(&self, inner: &LedgerInner) -> Result<(), LedgerError> {
        self.ensure_usable()?;
        let persisted = PersistedLedger {
            version: LEDGER_FORMAT_VERSION,
            cap_micro_usd: inner.cap_micro_usd,
            reservations: inner.reservations.clone(),
        };
        let bytes = serde_json::to_vec(&persisted)
            .map_err(|error| self.poison(format!("serialize: {error}")))?;
        let (mut temp, temp_path) = self
            .create_owned_temp()
            .map_err(|error| self.poison(format!("create temp file: {error}")))?;
        let mut renamed = false;
        let result = (|| -> std::io::Result<()> {
            temp.write_all(&bytes)?;
            temp.sync_all()?;
            drop(temp);
            std::fs::rename(&temp_path, &self.path)?;
            renamed = true;

            #[cfg(test)]
            {
                self.rename_reached.store(true, Ordering::Release);
                while self.pause_after_rename.load(Ordering::Acquire) {
                    std::thread::yield_now();
                }
                if self.fail_after_rename_once.swap(false, Ordering::AcqRel) {
                    return Err(std::io::Error::other(
                        "injected failure after rename before directory sync",
                    ));
                }
            }

            #[cfg(unix)]
            {
                let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
                File::open(parent)?.sync_all()?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            if !renamed {
                let _ = std::fs::remove_file(&temp_path);
            }
            return Err(self.poison(error.to_string()));
        }
        Ok(())
    }
}

fn stable_data_path(path: &Path) -> Result<PathBuf, LedgerError> {
    let file_name = path.file_name().ok_or_else(|| {
        LedgerError::Persistence(format!("ledger path has no file name: {}", path.display()))
    })?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let parent = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };
    let stable_parent = std::fs::canonicalize(parent).map_err(|error| {
        LedgerError::Persistence(format!(
            "resolve ledger parent {}: {error}",
            parent.display()
        ))
    })?;
    Ok(stable_parent.join(file_name))
}

fn require_regular_path(path: &Path, kind: &str) -> Result<(), LedgerError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        LedgerError::Persistence(format!("inspect {kind} {}: {error}", path.display()))
    })?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(LedgerError::CorruptState(format!(
            "{kind} is not a regular file: {}",
            path.display()
        )));
    }
    Ok(())
}

/// Advisory locks belong to the open file description, which a concurrent
/// `fork` in this process duplicates until the child calls `exec`. A lock that
/// was just released can therefore look held for a few microseconds, so a
/// bounded retry separates that transient from a genuinely held sidecar.
fn try_lock_briefly(lock_file: &File, lock_path: &Path) -> Result<(), LedgerError> {
    const ATTEMPTS: u32 = 50;
    const PAUSE: std::time::Duration = std::time::Duration::from_millis(5);
    for attempt in 0..ATTEMPTS {
        match lock_file.try_lock() {
            Ok(()) => return Ok(()),
            Err(TryLockError::WouldBlock) if attempt + 1 < ATTEMPTS => {
                std::thread::sleep(PAUSE);
            }
            Err(TryLockError::WouldBlock) => {
                return Err(LedgerError::LockUnavailable(
                    lock_path.display().to_string(),
                ));
            }
            Err(TryLockError::Error(error)) => {
                return Err(LedgerError::Persistence(format!(
                    "lock sidecar {}: {error}",
                    lock_path.display()
                )));
            }
        }
    }
    Err(LedgerError::LockUnavailable(
        lock_path.display().to_string(),
    ))
}

fn open_lock_sidecar(lock_path: &Path) -> Result<File, LedgerError> {
    match std::fs::symlink_metadata(lock_path) {
        Ok(_) => {
            require_regular_path(lock_path, "ledger lock sidecar")?;
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(lock_path)
                .map_err(|error| {
                    LedgerError::Persistence(format!(
                        "open lock sidecar {}: {error}",
                        lock_path.display()
                    ))
                })
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            match OpenOptions::new()
                .create_new(true)
                .read(true)
                .write(true)
                .open(lock_path)
            {
                Ok(file) => Ok(file),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    require_regular_path(lock_path, "ledger lock sidecar")?;
                    OpenOptions::new()
                        .read(true)
                        .write(true)
                        .open(lock_path)
                        .map_err(|error| {
                            LedgerError::Persistence(format!(
                                "open lock sidecar {}: {error}",
                                lock_path.display()
                            ))
                        })
                }
                Err(error) => Err(LedgerError::Persistence(format!(
                    "create lock sidecar {}: {error}",
                    lock_path.display()
                ))),
            }
        }
        Err(error) => Err(LedgerError::Persistence(format!(
            "inspect lock sidecar {}: {error}",
            lock_path.display()
        ))),
    }
}

impl LocalLedger {
    pub fn new(cap_micro_usd: u64) -> Self {
        Self {
            inner: Arc::new(Mutex::new(LedgerInner {
                cap_micro_usd,
                reservations: BTreeMap::new(),
            })),
            durable: None,
        }
    }

    /// Open or initialize a durable ledger in an operator-owned private
    /// directory. The parent is resolved to a stable absolute path, and the
    /// regular `*.lock` sidecar remains locked until the last clone drops.
    /// Ledger and lock symlinks are refused. This does not claim protection
    /// against a hostile actor concurrently replacing entries in the directory.
    pub fn open(path: impl AsRef<Path>, cap_micro_usd: u64) -> Result<Self, LedgerError> {
        let path = stable_data_path(path.as_ref())?;
        let lock_path = DurableLedger::sidecar_path(&path);
        let lock_file = open_lock_sidecar(&lock_path)?;
        try_lock_briefly(&lock_file, &lock_path)?;
        let durable = Arc::new(DurableLedger {
            path: path.clone(),
            _lock_file: lock_file,
            poisoned: AtomicBool::new(false),
            #[cfg(test)]
            fail_after_rename_once: AtomicBool::new(false),
            #[cfg(test)]
            pause_after_rename: AtomicBool::new(false),
            #[cfg(test)]
            rename_reached: AtomicBool::new(false),
        });

        let inner = match std::fs::symlink_metadata(&path) {
            Ok(_) => {
                require_regular_path(&path, "ledger data file")?;
                let mut bytes = Vec::new();
                File::open(&path)
                    .and_then(|mut file| file.read_to_end(&mut bytes))
                    .map_err(|error| {
                        LedgerError::CorruptState(format!("read {}: {error}", path.display()))
                    })?;
                let persisted: PersistedLedger =
                    serde_json::from_slice(&bytes).map_err(|error| {
                        LedgerError::CorruptState(format!("parse {}: {error}", path.display()))
                    })?;
                if persisted.version != LEDGER_FORMAT_VERSION {
                    return Err(LedgerError::UnsupportedVersion(persisted.version));
                }
                if persisted.cap_micro_usd != cap_micro_usd {
                    return Err(LedgerError::CapMismatch {
                        persisted: persisted.cap_micro_usd,
                        requested: cap_micro_usd,
                    });
                }
                let inner = LedgerInner {
                    cap_micro_usd: persisted.cap_micro_usd,
                    reservations: persisted.reservations,
                };
                inner.validate()?;
                inner
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let inner = LedgerInner {
                    cap_micro_usd,
                    reservations: BTreeMap::new(),
                };
                durable.persist(&inner)?;
                inner
            }
            Err(error) => {
                return Err(LedgerError::Persistence(format!(
                    "inspect ledger data file {}: {error}",
                    path.display()
                )));
            }
        };

        Ok(Self {
            inner: Arc::new(Mutex::new(inner)),
            durable: Some(durable),
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, LedgerInner> {
        self.inner
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    fn ensure_usable(&self) -> Result<(), LedgerError> {
        match &self.durable {
            Some(durable) => durable.ensure_usable(),
            None => Ok(()),
        }
    }

    fn ensure_durable_attempt_id(&self, attempt_id: &str) -> Result<(), LedgerError> {
        if self.durable.is_some() && attempt_id.is_empty() {
            Err(LedgerError::InvalidAttemptId)
        } else {
            Ok(())
        }
    }

    fn commit(&self, current: &mut LedgerInner, candidate: LedgerInner) -> Result<(), LedgerError> {
        if let Some(durable) = &self.durable {
            durable.persist(&candidate)?;
        }
        *current = candidate;
        Ok(())
    }

    /// Exposure currently counted against the cap. A poisoned durable handle
    /// reports maximum exposure so callers cannot infer reusable capacity.
    pub fn exposure_micro_usd(&self) -> u64 {
        let current = self.lock();
        if self.ensure_usable().is_err() {
            return u64::MAX;
        }
        current.exposure_micro_usd()
    }

    pub fn reserve(&self, attempt_id: &str, micro_usd: u64) -> Result<(), LedgerError> {
        self.ensure_durable_attempt_id(attempt_id)?;
        self.ensure_usable()?;
        let mut current = self.lock();
        if current.reservations.contains_key(attempt_id) {
            return Err(LedgerError::DuplicateAttempt(attempt_id.to_string()));
        }
        let checked_held = current.checked_exposure_micro_usd();
        let held = checked_held.unwrap_or(u64::MAX);
        if !matches!(
            checked_held.and_then(|held| held.checked_add(micro_usd)),
            Some(total) if total <= current.cap_micro_usd
        ) {
            return Err(LedgerError::CapExceeded {
                cap: current.cap_micro_usd,
                held,
                requested: micro_usd,
            });
        }
        let reservation = Reservation {
            attempt_id: attempt_id.to_string(),
            reserved_micro_usd: micro_usd,
            settled_micro_usd: None,
            state: ReservationState::Held,
        };
        if self.durable.is_none() {
            current
                .reservations
                .insert(attempt_id.to_string(), reservation);
            return Ok(());
        }
        let mut candidate = current.clone();
        candidate
            .reservations
            .insert(attempt_id.to_string(), reservation);
        self.commit(&mut current, candidate)
    }

    pub fn reserve_remaining(&self, attempt_id: &str) -> Result<u64, LedgerError> {
        self.ensure_durable_attempt_id(attempt_id)?;
        self.ensure_usable()?;
        let mut current = self.lock();
        if current.reservations.contains_key(attempt_id) {
            return Err(LedgerError::DuplicateAttempt(attempt_id.to_string()));
        }
        let held = current.exposure_micro_usd();
        let remaining = current.cap_micro_usd.saturating_sub(held);
        if remaining == 0 {
            return Err(LedgerError::CapExceeded {
                cap: current.cap_micro_usd,
                held,
                requested: 1,
            });
        }
        let reservation = Reservation {
            attempt_id: attempt_id.to_string(),
            reserved_micro_usd: remaining,
            settled_micro_usd: None,
            state: ReservationState::Held,
        };
        if self.durable.is_none() {
            current
                .reservations
                .insert(attempt_id.to_string(), reservation);
            return Ok(remaining);
        }
        let mut candidate = current.clone();
        candidate
            .reservations
            .insert(attempt_id.to_string(), reservation);
        self.commit(&mut current, candidate)?;
        Ok(remaining)
    }

    /// Settle at the actual amount. An amount above the reservation remains
    /// visible as ambiguous exposure instead of being silently clamped.
    pub fn settle(&self, attempt_id: &str, actual_micro_usd: u64) -> Result<(), LedgerError> {
        self.ensure_durable_attempt_id(attempt_id)?;
        self.ensure_usable()?;
        let mut current = self.lock();
        if self.durable.is_none() {
            let reservation = current
                .reservations
                .get_mut(attempt_id)
                .ok_or_else(|| LedgerError::UnknownAttempt(attempt_id.to_string()))?;
            if actual_micro_usd > reservation.reserved_micro_usd {
                let reserved = reservation.reserved_micro_usd;
                reservation.settled_micro_usd = Some(actual_micro_usd);
                reservation.state = ReservationState::Ambiguous;
                return Err(LedgerError::SettlementExceedsReservation {
                    attempt_id: attempt_id.to_string(),
                    reserved,
                    actual: actual_micro_usd,
                });
            }
            reservation.settled_micro_usd = Some(actual_micro_usd);
            reservation.state = ReservationState::Settled;
            return Ok(());
        }

        let mut candidate = current.clone();
        let reservation = candidate
            .reservations
            .get_mut(attempt_id)
            .ok_or_else(|| LedgerError::UnknownAttempt(attempt_id.to_string()))?;
        let overage = actual_micro_usd > reservation.reserved_micro_usd;
        let reserved = reservation.reserved_micro_usd;
        reservation.settled_micro_usd = Some(actual_micro_usd);
        reservation.state = if overage {
            ReservationState::Ambiguous
        } else {
            ReservationState::Settled
        };
        self.commit(&mut current, candidate)?;
        if overage {
            Err(LedgerError::SettlementExceedsReservation {
                attempt_id: attempt_id.to_string(),
                reserved,
                actual: actual_micro_usd,
            })
        } else {
            Ok(())
        }
    }

    pub fn mark_ambiguous(&self, attempt_id: &str) -> Result<(), LedgerError> {
        self.ensure_durable_attempt_id(attempt_id)?;
        self.ensure_usable()?;
        let mut current = self.lock();
        if self.durable.is_none() {
            current
                .reservations
                .get_mut(attempt_id)
                .ok_or_else(|| LedgerError::UnknownAttempt(attempt_id.to_string()))?
                .state = ReservationState::Ambiguous;
            return Ok(());
        }
        let mut candidate = current.clone();
        candidate
            .reservations
            .get_mut(attempt_id)
            .ok_or_else(|| LedgerError::UnknownAttempt(attempt_id.to_string()))?
            .state = ReservationState::Ambiguous;
        self.commit(&mut current, candidate)
    }

    /// Authoritatively record a known actual cost, including an overage.
    /// Reconciliation never clamps the cost to the original reservation.
    pub fn reconcile(&self, attempt_id: &str, actual_micro_usd: u64) -> Result<(), LedgerError> {
        self.ensure_durable_attempt_id(attempt_id)?;
        self.ensure_usable()?;
        let mut current = self.lock();
        if self.durable.is_none() {
            let reservation = current
                .reservations
                .get_mut(attempt_id)
                .ok_or_else(|| LedgerError::UnknownAttempt(attempt_id.to_string()))?;
            reservation.settled_micro_usd = Some(actual_micro_usd);
            reservation.state = ReservationState::Settled;
            return Ok(());
        }
        let mut candidate = current.clone();
        let reservation = candidate
            .reservations
            .get_mut(attempt_id)
            .ok_or_else(|| LedgerError::UnknownAttempt(attempt_id.to_string()))?;
        reservation.settled_micro_usd = Some(actual_micro_usd);
        reservation.state = ReservationState::Settled;
        self.commit(&mut current, candidate)
    }

    pub fn get(&self, attempt_id: &str) -> Option<Reservation> {
        self.lock().reservations.get(attempt_id).cloned()
    }

    #[cfg(test)]
    pub(crate) fn fail_after_rename_once_for_tests(&self) {
        self.durable
            .as_ref()
            .expect("failure injection requires durable ledger")
            .fail_after_rename_once
            .store(true, Ordering::Release);
    }

    #[cfg(test)]
    pub(crate) fn pause_and_fail_after_rename_for_tests(&self) {
        let durable = self
            .durable
            .as_ref()
            .expect("failure injection requires durable ledger");
        durable.rename_reached.store(false, Ordering::Release);
        durable.pause_after_rename.store(true, Ordering::Release);
        durable
            .fail_after_rename_once
            .store(true, Ordering::Release);
    }

    #[cfg(test)]
    pub(crate) fn wait_until_rename_for_tests(&self) {
        let durable = self.durable.as_ref().expect("durable ledger required");
        while !durable.rename_reached.load(Ordering::Acquire) {
            std::thread::yield_now();
        }
    }

    #[cfg(test)]
    pub(crate) fn release_after_rename_for_tests(&self) {
        self.durable
            .as_ref()
            .expect("durable ledger required")
            .pause_after_rename
            .store(false, Ordering::Release);
    }
}
