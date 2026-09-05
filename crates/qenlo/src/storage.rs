use std::{
    ffi::OsStr,
    fs::{self, File, OpenOptions},
    io::{self, BufReader, BufWriter, Cursor, Read, Write},
    path::{Path, PathBuf},
};

use crc32fast::Hasher;
use memmap2::MmapOptions;
use qenlo_core::{CoreStore, Mutation as CoreMutation, RestoredRecord};
use thiserror::Error;

const MAGIC: &[u8; 8] = b"QENLODB\0";
const FORMAT_VERSION: u32 = 1;
const HEADER_BYTES: u64 = 8 + 4 + 4 + 8 + 8 + 8;
const CHECKSUM_BYTES: u64 = 4;
const HEAD_MAGIC: &[u8; 8] = b"QENLOHD\0";
const MANIFEST_MAGIC: &[u8; 8] = b"QENLOMF\0";
const MANIFEST_VERSION: u32 = 1;
const MANIFEST_BYTES: u64 = 8 + 4 + 4 + 8 + 8 + 4;
const WAL_MAGIC: &[u8; 8] = b"QENLOWL\0";
const WAL_VERSION: u32 = 1;
const WAL_HEADER_BYTES: u64 = 8 + 4 + 4 + 8 + 8 + 8;
pub(crate) const MAX_LOAD_BYTES: u64 = 512 * 1024 * 1024;

enum WalMutation {
    Add {
        id: u64,
        user_id: u64,
        timestamp: i64,
        vector: Vec<f32>,
    },
    Delete(u64),
}

#[derive(Debug, Error)]
pub(crate) enum StorageError {
    #[error("collection already exists at {0}")]
    AlreadyExists(PathBuf),
    #[error("collection does not exist at {0}")]
    NotFound(PathBuf),
    #[error("collection has no committed snapshot at {0}")]
    NoSnapshot(PathBuf),
    #[error(
        "unsupported collection format version {found}; this build supports version {supported}"
    )]
    UnsupportedVersion { found: u32, supported: u32 },
    #[error("collection snapshot is corrupt: {0}")]
    Corrupt(String),
    #[error("collection snapshot exceeds the configured load budget or format capacity")]
    LoadLimitExceeded,
    #[error("portable collection paths must use the .qn extension: {0}")]
    InvalidPortableExtension(PathBuf),
    #[error("collection is already open by another handle or process")]
    Locked,
    #[error(
        "snapshot was published but durability confirmation failed; close and reopen to resolve commit: {0}"
    )]
    CommitUncertain(io::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Core(#[from] qenlo_core::Error),
}

#[derive(Debug)]
pub(crate) struct OpenedStore {
    pub store: CoreStore,
    pub recovered_interrupted_write: bool,
    pub lock: File,
}

pub(crate) struct OpenedPortable {
    pub store: CoreStore,
    pub recovered_interrupted_write: bool,
}

pub(crate) fn create(path: &Path, store: &CoreStore) -> Result<File, StorageError> {
    create_with_limit(path, store, MAX_LOAD_BYTES)
}

pub(crate) fn create_with_limit(
    path: &Path,
    store: &CoreStore,
    max_load_bytes: u64,
) -> Result<File, StorageError> {
    check_admission(store.dimension(), store.len() as u64, max_load_bytes)?;
    if path.exists() {
        let mut entries = fs::read_dir(path).map_err(StorageError::Io)?;
        if entries
            .any(|entry| entry.is_err() || entry.is_ok_and(|e| e.file_name() != "collection.lock"))
        {
            return Err(StorageError::AlreadyExists(path.to_path_buf()));
        }
    } else {
        create_directory(path)?;
    }
    let lock = lock_directory(path)?;
    // Recheck under the lock: another creator may have won the race.
    if fs::read_dir(path)?
        .any(|entry| entry.is_err() || entry.is_ok_and(|e| e.file_name() != "collection.lock"))
    {
        return Err(StorageError::AlreadyExists(path.to_path_buf()));
    }
    write_snapshot_with_limit(path, store, max_load_bytes)?;
    Ok(lock)
}

pub(crate) fn open(path: &Path) -> Result<OpenedStore, StorageError> {
    open_with_limit(path, MAX_LOAD_BYTES)
}

fn lock_directory(path: &Path) -> Result<File, StorageError> {
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path.join("collection.lock"))?;
    lock.try_lock().map_err(|error| match error {
        fs::TryLockError::WouldBlock => StorageError::Locked,
        fs::TryLockError::Error(error) => StorageError::Io(error),
    })?;
    Ok(lock)
}

pub(crate) fn open_with_limit(
    path: &Path,
    max_load_bytes: u64,
) -> Result<OpenedStore, StorageError> {
    if !path.is_dir() {
        return Err(StorageError::NotFound(path.to_path_buf()));
    }

    let lock = lock_directory(path)?;
    let acknowledged = read_head(path)?;
    let manifest = read_manifest(path)?;
    if let Some((_, base_generation, durable_generation)) = manifest
        && base_generation > durable_generation
    {
        return Err(StorageError::Corrupt(
            "manifest names an invalid or missing canonical snapshot".into(),
        ));
    }
    if let Some(generation) = acknowledged
        && !snapshot_path(path, generation, ".qdb").is_file()
    {
        return Err(StorageError::Corrupt(format!(
            "acknowledged generation {generation} is missing"
        )));
    }
    let mut interrupted =
        path.join("HEAD.pending").exists() || path.join("MANIFEST.pending").exists();
    let mut latest_committed: Option<(u64, PathBuf)> = None;
    let mut latest_temporary: Option<(u64, PathBuf)> = None;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_name = entry.file_name();
        if let Some(generation) = generation_from_name(&file_name, ".qdb") {
            if latest_committed
                .as_ref()
                .is_none_or(|(latest, _)| generation > *latest)
            {
                latest_committed = Some((generation, entry.path()));
            }
        } else if let Some(generation) = generation_from_name(&file_name, ".tmp") {
            if latest_temporary
                .as_ref()
                .is_none_or(|(latest, _)| generation > *latest)
            {
                latest_temporary = Some((generation, entry.path()));
            }
            interrupted = true;
        } else if generation_from_name(&file_name, ".pending").is_some() {
            // New writers publish only by rename. A staged transaction is never
            // promoted on restart; v1 .tmp recovery below remains compatible.
            interrupted = true;
        } else if wal_generation_from_name(&file_name, ".pending").is_some() {
            interrupted = true;
        }
    }
    let committed_generation = latest_committed
        .as_ref()
        .map_or(0, |(generation, _)| *generation);
    interrupted |= acknowledged.is_some_and(|generation| generation < committed_generation);
    if let Some((generation, temp)) = latest_temporary
        .filter(|(generation, _)| latest_committed.is_none() || *generation > committed_generation)
        && let Some(store) = match read_snapshot(&temp, generation, max_load_bytes) {
            Ok(store) => Some(store),
            Err(error @ StorageError::UnsupportedVersion { .. }) => return Err(error),
            Err(error @ StorageError::LoadLimitExceeded) => return Err(error),
            Err(_) => None,
        }
    {
        let final_path = snapshot_path(path, generation, ".qdb");
        fs::rename(&temp, &final_path)?;
        sync_directory(path).map_err(StorageError::CommitUncertain)?;
        write_head(path, generation).map_err(StorageError::CommitUncertain)?;
        write_manifest(path, store.dimension(), generation, generation)
            .map_err(StorageError::CommitUncertain)?;
        prune_snapshots(path, generation);
        return Ok(OpenedStore {
            store,
            recovered_interrupted_write: true,
            lock,
        });
    }

    if let Some((_, base_generation, _)) = manifest
        && !snapshot_path(path, base_generation, ".qdb").is_file()
    {
        return Err(StorageError::Corrupt(
            "manifest names a missing canonical snapshot".into(),
        ));
    }

    let Some((generation, snapshot)) = latest_committed else {
        return Err(StorageError::NoSnapshot(path.to_path_buf()));
    };
    let mut store = read_snapshot(&snapshot, generation, max_load_bytes)?;
    if manifest.is_some_and(|(dimension, _, _)| dimension as usize != store.dimension()) {
        return Err(StorageError::Corrupt(
            "manifest dimension does not match canonical snapshot".into(),
        ));
    }
    if acknowledged != Some(generation) {
        // Reopening resolves an uncertain publication (or upgrades a legacy
        // snapshot) only after its validated generation is acknowledged too.
        write_head(path, generation).map_err(StorageError::CommitUncertain)?;
    }
    let mut wal_files = fs::read_dir(path)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            wal_generation_from_name(&entry.file_name(), ".qwal")
                .map(|generation| (generation, entry.path()))
        })
        .filter(|(end_generation, _)| *end_generation > store.generation())
        .collect::<Vec<_>>();
    wal_files.sort_unstable_by_key(|(generation, _)| *generation);
    for (end_generation, wal) in wal_files {
        replay_wal(&wal, &mut store, end_generation, max_load_bytes)?;
    }
    if manifest.is_some_and(|(_, _, durable_generation)| durable_generation > store.generation()) {
        return Err(StorageError::Corrupt(
            "manifest durable generation is missing WAL data".into(),
        ));
    }
    if manifest != Some((store.dimension() as u32, generation, store.generation())) {
        interrupted |= manifest.is_some();
        write_manifest(path, store.dimension(), generation, store.generation())
            .map_err(StorageError::CommitUncertain)?;
    }
    Ok(OpenedStore {
        store,
        recovered_interrupted_write: interrupted,
        lock,
    })
}

pub(crate) fn append_wal(
    path: &Path,
    dimension: usize,
    start_generation: u64,
    mutations: &[CoreMutation<'_>],
    max_load_bytes: u64,
) -> Result<u64, StorageError> {
    let count = u64::try_from(mutations.len()).map_err(|_| StorageError::LoadLimitExceeded)?;
    let end_generation = start_generation
        .checked_add(count)
        .ok_or(StorageError::LoadLimitExceeded)?;
    let dimension = u32::try_from(dimension).map_err(|_| StorageError::LoadLimitExceeded)?;
    let final_path = wal_path(path, end_generation, ".qwal");
    if final_path.exists() {
        return Err(StorageError::AlreadyExists(final_path));
    }
    let pending = wal_path(path, end_generation, ".pending");
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&pending)?;
    let mut writer = CheckedWriter::new(BufWriter::new(file));
    writer.write_all(WAL_MAGIC)?;
    writer.write_all(&WAL_VERSION.to_le_bytes())?;
    writer.write_all(&dimension.to_le_bytes())?;
    writer.write_all(&start_generation.to_le_bytes())?;
    writer.write_all(&end_generation.to_le_bytes())?;
    writer.write_all(&count.to_le_bytes())?;
    let mut bytes = WAL_HEADER_BYTES + CHECKSUM_BYTES;
    for mutation in mutations {
        match mutation {
            CoreMutation::Add {
                id,
                user_id,
                timestamp,
                vector,
            } => {
                writer.write_all(&[1, 0, 0, 0, 0, 0, 0, 0])?;
                writer.write_all(&id.to_le_bytes())?;
                writer.write_all(&user_id.to_le_bytes())?;
                writer.write_all(&timestamp.to_le_bytes())?;
                for value in *vector {
                    writer.write_all(&value.to_bits().to_le_bytes())?;
                }
                bytes = bytes
                    .checked_add(32 + u64::from(dimension) * 4)
                    .ok_or(StorageError::LoadLimitExceeded)?;
            }
            CoreMutation::Delete(id) => {
                writer.write_all(&[2, 0, 0, 0, 0, 0, 0, 0])?;
                writer.write_all(&id.to_le_bytes())?;
                bytes = bytes
                    .checked_add(16)
                    .ok_or(StorageError::LoadLimitExceeded)?;
            }
        }
        if bytes > max_load_bytes {
            return Err(StorageError::LoadLimitExceeded);
        }
    }
    let (mut writer, checksum) = writer.finish();
    writer.write_all(&checksum.to_le_bytes())?;
    writer.flush()?;
    writer.get_ref().sync_all()?;
    drop(writer);
    fs::rename(&pending, &final_path)?;
    sync_directory(path).map_err(StorageError::CommitUncertain)?;
    let base_generation = read_head(path)?.ok_or_else(|| {
        StorageError::Corrupt("cannot publish WAL without a canonical HEAD".into())
    })?;
    write_manifest(path, dimension as usize, base_generation, end_generation)
        .map_err(StorageError::CommitUncertain)?;
    Ok(end_generation)
}

fn replay_wal(
    path: &Path,
    store: &mut CoreStore,
    expected_end_generation: u64,
    max_load_bytes: u64,
) -> Result<(), StorageError> {
    let file = File::open(path)?;
    let file_len = file.metadata()?.len();
    if file_len > max_load_bytes || file_len < WAL_HEADER_BYTES + CHECKSUM_BYTES {
        return Err(if file_len > max_load_bytes {
            StorageError::LoadLimitExceeded
        } else {
            StorageError::Corrupt("WAL is shorter than its header".into())
        });
    }
    let mut reader = CheckedReader::new(BufReader::new(file));
    if &reader.array::<8>()? != WAL_MAGIC {
        return Err(StorageError::Corrupt("invalid WAL magic bytes".into()));
    }
    let version = reader.u32()?;
    if version != WAL_VERSION {
        return Err(StorageError::UnsupportedVersion {
            found: version,
            supported: WAL_VERSION,
        });
    }
    let dimension = reader.u32()? as usize;
    if dimension != store.dimension() {
        return Err(StorageError::Corrupt("WAL dimension mismatch".into()));
    }
    let start_generation = reader.u64()?;
    let end_generation = reader.u64()?;
    let count = reader.u64()?;
    if start_generation != store.generation()
        || end_generation != expected_end_generation
        || start_generation.checked_add(count) != Some(end_generation)
    {
        return Err(StorageError::Corrupt(
            "non-contiguous WAL generation range".into(),
        ));
    }
    let count = usize::try_from(count).map_err(|_| StorageError::LoadLimitExceeded)?;
    let mut mutations = Vec::with_capacity(count);
    for _ in 0..count {
        let tag = reader.array::<8>()?;
        if tag[1..].iter().any(|byte| *byte != 0) {
            return Err(StorageError::Corrupt("invalid WAL mutation tag".into()));
        }
        match tag[0] {
            1 => {
                let id = reader.u64()?;
                let user_id = reader.u64()?;
                let timestamp = reader.i64()?;
                let mut vector = Vec::with_capacity(dimension);
                for _ in 0..dimension {
                    vector.push(f32::from_bits(reader.u32()?));
                }
                mutations.push(WalMutation::Add {
                    id,
                    user_id,
                    timestamp,
                    vector,
                });
            }
            2 => mutations.push(WalMutation::Delete(reader.u64()?)),
            _ => return Err(StorageError::Corrupt("unknown WAL mutation tag".into())),
        }
    }
    let (mut reader, calculated) = reader.finish();
    let stored = read_u32_unchecked(&mut reader)?;
    if calculated != stored {
        return Err(StorageError::Corrupt("WAL checksum mismatch".into()));
    }
    let mut trailing = [0_u8; 1];
    if reader.read(&mut trailing)? != 0 {
        return Err(StorageError::Corrupt(
            "trailing bytes after WAL checksum".into(),
        ));
    }
    let borrowed = mutations
        .iter()
        .map(|mutation| match mutation {
            WalMutation::Add {
                id,
                user_id,
                timestamp,
                vector,
            } => CoreMutation::Add {
                id: *id,
                user_id: *user_id,
                timestamp: *timestamp,
                vector,
            },
            WalMutation::Delete(id) => CoreMutation::Delete(*id),
        })
        .collect::<Vec<_>>();
    store.apply_batch(&borrowed)?;
    Ok(())
}

#[cfg(test)]
pub(crate) fn write_snapshot(path: &Path, store: &CoreStore) -> Result<(), StorageError> {
    write_snapshot_with_limit(path, store, MAX_LOAD_BYTES)
}

pub(crate) fn write_snapshot_with_limit(
    path: &Path,
    store: &CoreStore,
    max_load_bytes: u64,
) -> Result<(), StorageError> {
    check_admission(store.dimension(), store.len() as u64, max_load_bytes)?;
    create_directory(path)?;
    let generation = store.generation();
    let final_path = snapshot_path(path, generation, ".qdb");
    if final_path.exists() {
        return Err(StorageError::AlreadyExists(final_path));
    }
    let temp_path = snapshot_path(path, generation, ".pending");
    write_snapshot_file(&temp_path, store)?;

    fs::rename(&temp_path, &final_path)?;
    sync_directory(path).map_err(StorageError::CommitUncertain)?;
    write_head(path, generation).map_err(StorageError::CommitUncertain)?;
    write_manifest(path, store.dimension(), generation, generation)
        .map_err(StorageError::CommitUncertain)?;
    prune_snapshots(path, generation);
    prune_wals(path, generation);
    Ok(())
}

fn write_snapshot_file(path: &Path, store: &CoreStore) -> Result<(), StorageError> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)?;
    write_snapshot_to(file, store)
}

fn write_portable_pending(path: &Path, store: &CoreStore) -> Result<(), StorageError> {
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                StorageError::AlreadyExists(path.to_path_buf())
            } else {
                StorageError::Io(error)
            }
        })?;
    write_snapshot_to(file, store)
}

fn write_snapshot_to(file: File, store: &CoreStore) -> Result<(), StorageError> {
    let generation = store.generation();
    let mut writer = CheckedWriter::new(BufWriter::new(file));
    writer.write_all(MAGIC)?;
    writer.write_all(&FORMAT_VERSION.to_le_bytes())?;
    let dimension =
        u32::try_from(store.dimension()).map_err(|_| StorageError::LoadLimitExceeded)?;
    writer.write_all(&dimension.to_le_bytes())?;
    writer.write_all(&generation.to_le_bytes())?;
    writer.write_all(&(store.len() as u64).to_le_bytes())?;
    writer.write_all(&(store.live_len() as u64).to_le_bytes())?;
    for (_, record) in store.records() {
        writer.write_all(&record.id().to_le_bytes())?;
        writer.write_all(&record.user_id().to_le_bytes())?;
        writer.write_all(&record.timestamp().to_le_bytes())?;
        writer.write_all(&[u8::from(record.is_live()), 0, 0, 0, 0, 0, 0, 0])?;
        for value in record.vector() {
            writer.write_all(&value.to_bits().to_le_bytes())?;
        }
    }
    let (mut writer, checksum) = writer.finish();
    writer.write_all(&checksum.to_le_bytes())?;
    writer.flush()?;
    writer.get_ref().sync_all()?;
    drop(writer);
    Ok(())
}

pub(crate) fn write_portable_with_limit(
    path: &Path,
    store: &CoreStore,
    max_load_bytes: u64,
) -> Result<(), StorageError> {
    validate_portable_extension(path)?;
    check_admission(store.dimension(), store.len() as u64, max_load_bytes)?;
    if path.exists() {
        return Err(StorageError::AlreadyExists(path.to_path_buf()));
    }
    let parent = portable_parent(path);
    create_directory(parent)?;
    let pending = path.with_extension("qn.pending");
    if let Err(error) = write_portable_pending(&pending, store) {
        let _ = fs::remove_file(&pending);
        return Err(error);
    }
    if let Err(error) = publish_portable(&pending, path) {
        if matches!(error, StorageError::AlreadyExists(_) | StorageError::Io(_)) {
            let _ = fs::remove_file(&pending);
        }
        return Err(error);
    }
    Ok(())
}

pub(crate) fn read_portable_with_limit(
    path: &Path,
    max_load_bytes: u64,
) -> Result<OpenedPortable, StorageError> {
    validate_portable_extension(path)?;
    if path.is_file() {
        return Ok(OpenedPortable {
            store: read_snapshot_discover_generation(path, max_load_bytes)?,
            recovered_interrupted_write: false,
        });
    }
    let pending = path.with_extension("qn.pending");
    if !pending.is_file() {
        return Err(StorageError::NotFound(path.to_path_buf()));
    }
    let store = read_snapshot_discover_generation(&pending, max_load_bytes)?;
    if let Err(error) = publish_portable(&pending, path) {
        if matches!(error, StorageError::AlreadyExists(_)) {
            return Ok(OpenedPortable {
                store: read_snapshot_discover_generation(path, max_load_bytes)?,
                recovered_interrupted_write: false,
            });
        }
        return Err(error);
    }
    Ok(OpenedPortable {
        store,
        recovered_interrupted_write: true,
    })
}

fn publish_portable(pending: &Path, path: &Path) -> Result<(), StorageError> {
    fs::hard_link(pending, path).map_err(|error| {
        if error.kind() == io::ErrorKind::AlreadyExists {
            StorageError::AlreadyExists(path.to_path_buf())
        } else {
            StorageError::Io(error)
        }
    })?;
    fs::remove_file(pending).map_err(StorageError::CommitUncertain)?;
    sync_directory(portable_parent(path)).map_err(StorageError::CommitUncertain)
}

fn read_snapshot_discover_generation(
    path: &Path,
    max_load_bytes: u64,
) -> Result<CoreStore, StorageError> {
    let mut header = [0_u8; 24];
    File::open(path)?.read_exact(&mut header).map_err(|error| {
        if error.kind() == io::ErrorKind::UnexpectedEof {
            StorageError::Corrupt("portable file is shorter than its header".into())
        } else {
            StorageError::Io(error)
        }
    })?;
    let generation = u64::from_le_bytes(header[16..24].try_into().expect("fixed header slice"));
    read_snapshot(path, generation, max_load_bytes)
}

fn validate_portable_extension(path: &Path) -> Result<(), StorageError> {
    if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("qn"))
    {
        Ok(())
    } else {
        Err(StorageError::InvalidPortableExtension(path.to_path_buf()))
    }
}

fn portable_parent(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn read_snapshot(
    path: &Path,
    expected_generation: u64,
    max_load_bytes: u64,
) -> Result<CoreStore, StorageError> {
    let file = File::open(path)?;
    let file_len = file.metadata()?.len();
    if file_len > max_load_bytes || file_len < HEADER_BYTES + CHECKSUM_BYTES {
        return Err(if file_len > max_load_bytes {
            StorageError::LoadLimitExceeded
        } else {
            StorageError::Corrupt("snapshot is shorter than its header".into())
        });
    }
    // SAFETY: canonical snapshots and portable imports are immutable while mapped.
    let mapped = unsafe { MmapOptions::new().map(&file)? };
    let mut reader = CheckedReader::new(Cursor::new(&mapped[..]));
    let magic = reader.array::<8>()?;
    if &magic != MAGIC {
        return Err(StorageError::Corrupt("invalid magic bytes".into()));
    }
    let version = reader.u32()?;
    if version != FORMAT_VERSION {
        return Err(StorageError::UnsupportedVersion {
            found: version,
            supported: FORMAT_VERSION,
        });
    }
    let dimension = reader.u32()? as usize;
    if dimension == 0 {
        return Err(StorageError::Corrupt("zero vector dimension".into()));
    }
    let generation = reader.u64()?;
    if generation != expected_generation {
        return Err(StorageError::Corrupt(format!(
            "filename generation {expected_generation} does not match header {generation}"
        )));
    }
    let rows_u64 = reader.u64()?;
    let expected_live = reader.u64()?;
    let row_bytes = 32_u64
        .checked_add(
            (dimension as u64)
                .checked_mul(4)
                .ok_or(StorageError::LoadLimitExceeded)?,
        )
        .ok_or(StorageError::LoadLimitExceeded)?;
    let expected_len = HEADER_BYTES
        .checked_add(
            rows_u64
                .checked_mul(row_bytes)
                .ok_or(StorageError::LoadLimitExceeded)?,
        )
        .and_then(|bytes| bytes.checked_add(CHECKSUM_BYTES))
        .ok_or(StorageError::LoadLimitExceeded)?;
    if expected_len != file_len {
        return Err(StorageError::Corrupt(format!(
            "declared shape requires {expected_len} bytes, file has {file_len}"
        )));
    }
    let rows = usize::try_from(rows_u64).map_err(|_| StorageError::LoadLimitExceeded)?;
    // Budget vector payload plus a conservative allowance for records and both
    // ordered indexes. This is admission accounting, not measured allocator RSS.
    check_admission(dimension, rows_u64, max_load_bytes)?;
    let mut actual_live = 0_u64;
    let records = (0..rows).map(|_| -> Result<RestoredRecord, StorageError> {
        let id = reader.u64()?;
        let user_id = reader.u64()?;
        let timestamp = reader.i64()?;
        let flags = reader.array::<8>()?;
        if flags[0] > 1 || flags[1..].iter().any(|byte| *byte != 0) {
            return Err(StorageError::Corrupt("invalid row flags".into()));
        }
        let live = flags[0] == 1;
        actual_live += u64::from(live);
        let mut vector = Vec::with_capacity(dimension);
        for _ in 0..dimension {
            vector.push(f32::from_bits(reader.u32()?));
        }
        Ok(RestoredRecord {
            id,
            user_id,
            timestamp,
            vector,
            live,
        })
    });
    let store = CoreStore::restore_iter(dimension, generation, records)?;
    if actual_live != expected_live {
        return Err(StorageError::Corrupt(format!(
            "declared {expected_live} live rows, decoded {actual_live}"
        )));
    }
    let (mut reader, calculated) = reader.finish();
    let stored = read_u32_unchecked(&mut reader)?;
    if calculated != stored {
        return Err(StorageError::Corrupt(format!(
            "checksum mismatch: expected {stored:08x}, calculated {calculated:08x}"
        )));
    }
    Ok(store)
}

fn snapshot_path(path: &Path, generation: u64, extension: &str) -> PathBuf {
    path.join(format!("canonical-{generation:020}{extension}"))
}

fn wal_path(path: &Path, generation: u64, extension: &str) -> PathBuf {
    path.join(format!("wal-{generation:020}{extension}"))
}

fn check_admission(dimension: usize, rows: u64, max_load_bytes: u64) -> Result<(), StorageError> {
    let dimension = u32::try_from(dimension).map_err(|_| StorageError::LoadLimitExceeded)?;
    let canonical_row_bytes = u64::from(dimension)
        .checked_mul(4)
        .and_then(|bytes| bytes.checked_add(32))
        .ok_or(StorageError::LoadLimitExceeded)?;
    // Exact CPU search materializes a second row-major view whose stride is rounded
    // to 16 floats for 64-byte alignment. Admit that first-query allocation up front.
    let scan_row_bytes = u64::from(dimension)
        .checked_add(15)
        .map(|values| values / 16)
        .and_then(|values| values.checked_mul(16 * 4))
        .ok_or(StorageError::LoadLimitExceeded)?;
    let file_bytes = rows
        .checked_mul(canonical_row_bytes)
        .and_then(|bytes| bytes.checked_add(HEADER_BYTES + CHECKSUM_BYTES))
        .ok_or(StorageError::LoadLimitExceeded)?;
    let admission_bytes = canonical_row_bytes
        .checked_add(scan_row_bytes)
        .and_then(|bytes| bytes.checked_add(512))
        .and_then(|bytes| rows.checked_mul(bytes))
        .ok_or(StorageError::LoadLimitExceeded)?;
    if rows > u64::from(u32::MAX) || file_bytes > max_load_bytes || admission_bytes > max_load_bytes
    {
        return Err(StorageError::LoadLimitExceeded);
    }
    Ok(())
}

pub(crate) fn check_store_admission(
    dimension: usize,
    rows: usize,
    max_load_bytes: u64,
) -> Result<(), StorageError> {
    check_admission(
        dimension,
        u64::try_from(rows).map_err(|_| StorageError::LoadLimitExceeded)?,
        max_load_bytes,
    )
}

// HEAD is a durable lower bound on acknowledged generations, not permission to
// ignore a newer canonical snapshot published immediately before a crash.
fn read_head(path: &Path) -> Result<Option<u64>, StorageError> {
    let mut file = match File::open(path.join("HEAD")) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if file.metadata()?.len() != 20 {
        return Err(StorageError::Corrupt("invalid HEAD length".into()));
    }
    let mut bytes = [0; 20];
    file.read_exact(&mut bytes)?;
    if &bytes[..8] != HEAD_MAGIC
        || crc32fast::hash(&bytes[..16]) != u32::from_le_bytes(bytes[16..].try_into().unwrap())
    {
        return Err(StorageError::Corrupt(
            "invalid HEAD checksum or magic".into(),
        ));
    }
    Ok(Some(u64::from_le_bytes(bytes[8..16].try_into().unwrap())))
}

fn write_head(path: &Path, generation: u64) -> io::Result<()> {
    let mut bytes = [0; 20];
    bytes[..8].copy_from_slice(HEAD_MAGIC);
    bytes[8..16].copy_from_slice(&generation.to_le_bytes());
    let checksum = crc32fast::hash(&bytes[..16]);
    bytes[16..].copy_from_slice(&checksum.to_le_bytes());
    let pending = path.join("HEAD.pending");
    let mut file = File::create(&pending)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(pending, path.join("HEAD"))?;
    sync_directory(path)
}

fn read_manifest(path: &Path) -> Result<Option<(u32, u64, u64)>, StorageError> {
    let bytes = match fs::read(path.join("MANIFEST")) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if bytes.len() as u64 != MANIFEST_BYTES
        || &bytes[..8] != MANIFEST_MAGIC
        || u32::from_le_bytes(bytes[8..12].try_into().unwrap()) != MANIFEST_VERSION
        || crc32fast::hash(&bytes[..32]) != u32::from_le_bytes(bytes[32..36].try_into().unwrap())
    {
        return Err(StorageError::Corrupt(
            "invalid MANIFEST length, version, magic, or checksum".into(),
        ));
    }
    Ok(Some((
        u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
        u64::from_le_bytes(bytes[16..24].try_into().unwrap()),
        u64::from_le_bytes(bytes[24..32].try_into().unwrap()),
    )))
}

fn write_manifest(
    path: &Path,
    dimension: usize,
    base_generation: u64,
    durable_generation: u64,
) -> io::Result<()> {
    let dimension = u32::try_from(dimension)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "dimension exceeds u32"))?;
    let mut bytes = [0_u8; MANIFEST_BYTES as usize];
    bytes[..8].copy_from_slice(MANIFEST_MAGIC);
    bytes[8..12].copy_from_slice(&MANIFEST_VERSION.to_le_bytes());
    bytes[12..16].copy_from_slice(&dimension.to_le_bytes());
    bytes[16..24].copy_from_slice(&base_generation.to_le_bytes());
    bytes[24..32].copy_from_slice(&durable_generation.to_le_bytes());
    let checksum = crc32fast::hash(&bytes[..32]);
    bytes[32..36].copy_from_slice(&checksum.to_le_bytes());
    let pending = path.join("MANIFEST.pending");
    let mut file = File::create(&pending)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(pending, path.join("MANIFEST"))?;
    sync_directory(path)
}

fn create_directory(path: &Path) -> io::Result<()> {
    let mut missing = Vec::new();
    let mut cursor = path;
    while !cursor.as_os_str().is_empty() && !cursor.exists() {
        missing.push(cursor);
        let Some(parent) = cursor.parent() else { break };
        cursor = parent;
    }
    fs::create_dir_all(path)?;
    // Sync the entry of every newly created directory in its parent, including
    // intermediate parents created by create_dir_all. A child fsync is not enough.
    for created in missing.into_iter().rev() {
        let parent = created
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        sync_directory(parent)?;
    }
    Ok(())
}

fn generation_from_name(name: &OsStr, extension: &str) -> Option<u64> {
    let name = name.to_str()?;
    let digits = name.strip_prefix("canonical-")?.strip_suffix(extension)?;
    (digits.len() == 20).then(|| digits.parse().ok()).flatten()
}

fn wal_generation_from_name(name: &OsStr, extension: &str) -> Option<u64> {
    let name = name.to_str()?;
    let digits = name.strip_prefix("wal-")?.strip_suffix(extension)?;
    (digits.len() == 20).then(|| digits.parse().ok()).flatten()
}

fn prune_snapshots(path: &Path, current_generation: u64) {
    let previous = match fs::read_dir(path) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .filter_map(|entry| generation_from_name(&entry.file_name(), ".qdb"))
            .filter(|generation| *generation < current_generation)
            .max(),
        Err(_) => return,
    };
    let Some(previous) = previous else { return };
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        if generation_from_name(&entry.file_name(), ".qdb")
            .is_some_and(|generation| generation < previous)
        {
            let _ = fs::remove_file(entry.path());
        }
    }
}

fn prune_wals(path: &Path, current_generation: u64) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        if wal_generation_from_name(&entry.file_name(), ".qwal")
            .is_some_and(|generation| generation <= current_generation)
        {
            let _ = fs::remove_file(entry.path());
        }
    }
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> io::Result<()> {
    // Rust has no portable directory-sync primitive on Windows. The snapshot
    // itself is synced before the same-volume atomic rename.
    Ok(())
}

struct CheckedWriter<W> {
    inner: W,
    hasher: Hasher,
}

impl<W> CheckedWriter<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            hasher: Hasher::new(),
        }
    }

    fn finish(self) -> (W, u32) {
        (self.inner, self.hasher.finalize())
    }
}

impl<W: Write> Write for CheckedWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(bytes)?;
        self.hasher.update(&bytes[..written]);
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

struct CheckedReader<R> {
    inner: R,
    hasher: Hasher,
}

impl<R: Read> CheckedReader<R> {
    fn new(inner: R) -> Self {
        Self {
            inner,
            hasher: Hasher::new(),
        }
    }

    fn array<const N: usize>(&mut self) -> io::Result<[u8; N]> {
        let mut bytes = [0; N];
        self.read_exact(&mut bytes)?;
        Ok(bytes)
    }

    fn u32(&mut self) -> io::Result<u32> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    fn u64(&mut self) -> io::Result<u64> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    fn i64(&mut self) -> io::Result<i64> {
        Ok(i64::from_le_bytes(self.array()?))
    }

    fn finish(self) -> (R, u32) {
        (self.inner, self.hasher.finalize())
    }
}

impl<R: Read> Read for CheckedReader<R> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let read = self.inner.read(bytes)?;
        self.hasher.update(&bytes[..read]);
        Ok(read)
    }
}

fn read_u32_unchecked(reader: &mut impl Read) -> io::Result<u32> {
    let mut bytes = [0; 4];
    reader.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("qenlo-{label}-{}-{nonce}", std::process::id()))
    }

    fn populated_store() -> CoreStore {
        let mut store = CoreStore::new(2).unwrap();
        store.add(1, 9, i64::MIN, [3.0, 4.0]).unwrap();
        store.add(2, 8, i64::MAX, [1.0, 0.0]).unwrap();
        store.delete(1).unwrap();
        store
    }

    #[test]
    fn snapshot_round_trip_preserves_rows_tombstones_and_generation() {
        let path = temp_dir("round-trip");
        let store = populated_store();
        create(&path, &store).unwrap();
        let opened = open(&path).unwrap();
        assert_eq!(opened.store.dimension(), 2);
        assert_eq!(opened.store.generation(), 3);
        assert_eq!(opened.store.len(), 2);
        assert_eq!(opened.store.live_len(), 1);
        assert!(!opened.store.record(0).unwrap().is_live());
        assert_eq!(opened.store.record(0).unwrap().vector(), &[0.6, 0.8]);
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn portable_publication_never_replaces_a_racing_target() {
        let root = temp_dir("portable-no-clobber");
        fs::create_dir_all(&root).unwrap();
        let pending = root.join("vectors.qn.pending");
        let path = root.join("vectors.qn");
        fs::write(&pending, b"candidate").unwrap();
        fs::write(&path, b"winner").unwrap();

        assert!(matches!(
            publish_portable(&pending, &path),
            Err(StorageError::AlreadyExists(existing)) if existing == path
        ));
        assert_eq!(fs::read(&path).unwrap(), b"winner");
        assert_eq!(fs::read(&pending).unwrap(), b"candidate");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn checksum_corruption_is_rejected_without_falling_back() {
        let path = temp_dir("corruption");
        let store = populated_store();
        create(&path, &store).unwrap();
        let snapshot = snapshot_path(&path, store.generation(), ".qdb");
        let file = OpenOptions::new().write(true).open(snapshot).unwrap();
        file.set_len(HEADER_BYTES + CHECKSUM_BYTES).unwrap();
        assert!(matches!(open(&path), Err(StorageError::Corrupt(_))));
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn complete_temporary_snapshot_is_recovered() {
        let path = temp_dir("recovery");
        let store = populated_store();
        create(&path, &store).unwrap();
        // Legacy snapshots predate the acknowledged-generation watermark.
        fs::remove_file(path.join("HEAD")).unwrap();
        let final_path = snapshot_path(&path, store.generation(), ".qdb");
        let temp_path = snapshot_path(&path, store.generation(), ".tmp");
        fs::rename(final_path, temp_path).unwrap();
        let opened = open(&path).unwrap();
        assert!(opened.recovered_interrupted_write);
        assert_eq!(opened.store.generation(), store.generation());
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn unknown_format_version_is_not_guessed() {
        let path = temp_dir("version");
        let store = populated_store();
        create(&path, &store).unwrap();
        let snapshot = snapshot_path(&path, store.generation(), ".qdb");
        let mut file = OpenOptions::new().write(true).open(snapshot).unwrap();
        use std::io::Seek;
        file.seek(io::SeekFrom::Start(8)).unwrap();
        file.write_all(&2_u32.to_le_bytes()).unwrap();
        file.sync_all().unwrap();
        assert!(matches!(
            open(&path),
            Err(StorageError::UnsupportedVersion {
                found: 2,
                supported: 1
            })
        ));
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn exclusive_lock_is_released_on_drop() {
        let path = temp_dir("lock");
        let lock = create(&path, &populated_store()).unwrap();
        assert!(matches!(open(&path), Err(StorageError::Locked)));
        drop(lock);
        drop(open(&path).unwrap());
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn unpublished_transaction_is_never_promoted() {
        let path = temp_dir("pending");
        let mut store = populated_store();
        drop(create(&path, &store).unwrap());
        let previous = store.generation();
        store.add(3, 9, 0, [1.0, 1.0]).unwrap();
        write_snapshot(&path, &store).unwrap();
        // Simulate staging before publication, not loss of an acknowledged file.
        write_head(&path, previous).unwrap();
        write_manifest(&path, store.dimension(), previous, previous).unwrap();
        fs::rename(
            snapshot_path(&path, store.generation(), ".qdb"),
            snapshot_path(&path, store.generation(), ".pending"),
        )
        .unwrap();
        let opened = open(&path).unwrap();
        assert_eq!(opened.store.generation(), previous);
        assert!(opened.recovered_interrupted_write);
        drop(opened);
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn load_budget_and_generation_collision_are_explicit() {
        let path = temp_dir("budget");
        let store = populated_store();
        drop(create(&path, &store).unwrap());
        assert!(matches!(
            open_with_limit(&path, 100),
            Err(StorageError::LoadLimitExceeded)
        ));
        assert!(matches!(
            write_snapshot(&path, &store),
            Err(StorageError::AlreadyExists(_))
        ));
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn checksum_bit_flip_and_truncated_pending_are_detected() {
        use std::io::{Seek, SeekFrom};
        let path = temp_dir("bit-flip");
        let store = populated_store();
        drop(create(&path, &store).unwrap());
        fs::write(
            snapshot_path(&path, store.generation() + 1, ".pending"),
            b"partial",
        )
        .unwrap();
        assert!(open(&path).unwrap().recovered_interrupted_write);
        let mut file = OpenOptions::new()
            .write(true)
            .open(snapshot_path(&path, store.generation(), ".qdb"))
            .unwrap();
        file.seek(SeekFrom::Start(HEADER_BYTES)).unwrap();
        file.write_all(&99_u64.to_le_bytes()).unwrap();
        file.sync_all().unwrap();
        assert!(matches!(open(&path), Err(StorageError::Corrupt(_))));
        drop(file);
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn write_and_open_share_admission_limits_before_publication() {
        let path = temp_dir("write-budget");
        let mut store = populated_store();
        // Two 2D rows need canonical rows plus a 64-byte aligned scan row and indexes:
        // 2 * (32 + 8 + 64 + 512) = 1,232 bytes.
        assert!(matches!(
            create_with_limit(&path, &store, 1231),
            Err(StorageError::LoadLimitExceeded)
        ));
        assert!(!path.exists());
        drop(create_with_limit(&path, &store, 1232).unwrap());
        drop(open_with_limit(&path, 1232).unwrap());
        let previous = store.generation();
        store.add(3, 1, 0, [1.0, 0.0]).unwrap();
        assert!(matches!(
            write_snapshot_with_limit(&path, &store, 1232),
            Err(StorageError::LoadLimitExceeded)
        ));
        assert!(!snapshot_path(&path, store.generation(), ".pending").exists());
        assert_eq!(
            open_with_limit(&path, 1232).unwrap().store.generation(),
            previous
        );
        // At 768 dimensions the aligned first-query matrix is another 3,072 bytes;
        // a flat 512-byte bookkeeping allowance alone is not conservative.
        assert!(matches!(
            check_admission(768, 1, 6687),
            Err(StorageError::LoadLimitExceeded)
        ));
        check_admission(768, 1, 6688).unwrap();
        assert!(matches!(
            check_admission(usize::MAX, u64::MAX, u64::MAX),
            Err(StorageError::LoadLimitExceeded)
        ));
        assert!(matches!(
            check_admission(2, 0, HEADER_BYTES),
            Err(StorageError::LoadLimitExceeded)
        ));
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn missing_acknowledged_snapshot_never_rolls_back_silently() {
        let path = temp_dir("missing-committed");
        let mut store = populated_store();
        drop(create(&path, &store).unwrap());
        store.add(3, 1, 0, [1.0, 0.0]).unwrap();
        write_snapshot(&path, &store).unwrap();
        fs::remove_file(snapshot_path(&path, store.generation(), ".qdb")).unwrap();
        assert!(
            matches!(open(&path), Err(StorageError::Corrupt(message)) if message.contains("acknowledged generation"))
        );
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn head_checksum_and_truncation_are_rejected() {
        let path = temp_dir("head-corrupt");
        drop(create(&path, &populated_store()).unwrap());
        let mut bytes = fs::read(path.join("HEAD")).unwrap();
        bytes[8] ^= 1;
        fs::write(path.join("HEAD"), &bytes).unwrap();
        assert!(
            matches!(open(&path), Err(StorageError::Corrupt(message)) if message.contains("HEAD checksum"))
        );
        fs::write(path.join("HEAD"), &bytes[..10]).unwrap();
        assert!(
            matches!(open(&path), Err(StorageError::Corrupt(message)) if message.contains("HEAD length"))
        );
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn failed_head_publication_is_uncertain_and_reopen_resolves_newer_snapshot() {
        let path = temp_dir("head-failure");
        let mut store = populated_store();
        drop(create(&path, &store).unwrap());
        fs::create_dir(path.join("HEAD.pending")).unwrap();
        store.add(3, 1, 0, [1.0, 0.0]).unwrap();
        assert!(matches!(
            write_snapshot(&path, &store),
            Err(StorageError::CommitUncertain(_))
        ));
        fs::remove_dir(path.join("HEAD.pending")).unwrap();
        let opened = open(&path).unwrap();
        assert_eq!(opened.store.generation(), store.generation());
        assert!(opened.recovered_interrupted_write);
        assert_eq!(read_head(&path).unwrap(), Some(store.generation()));
        drop(opened);
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn nested_collection_creation_is_readable() {
        let root = temp_dir("nested");
        let path = root.join("a").join("b");
        drop(create(&path, &populated_store()).unwrap());
        assert_eq!(open(&path).unwrap().store.live_len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn wal_replays_ordered_transactions_and_compacts_on_snapshot() {
        let path = temp_dir("wal-replay");
        let mut store = populated_store();
        drop(create(&path, &store).unwrap());
        let mutations = [
            CoreMutation::Add {
                id: 3,
                user_id: 7,
                timestamp: 0,
                vector: &[1.0, 1.0],
            },
            CoreMutation::Delete(2),
        ];
        store.validate_batch(&mutations).unwrap();
        append_wal(
            &path,
            store.dimension(),
            store.generation(),
            &mutations,
            MAX_LOAD_BYTES,
        )
        .unwrap();
        store.apply_batch(&mutations).unwrap();
        let opened = open(&path).unwrap();
        assert_eq!(opened.store.generation(), store.generation());
        assert_eq!(opened.store.live_len(), 1);
        assert_eq!(opened.store.record(2).unwrap().id(), 3);
        drop(opened);

        write_snapshot(&path, &store).unwrap();
        assert!(!wal_path(&path, store.generation(), ".qwal").exists());
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn manifest_corruption_and_wal_generation_gaps_fail_closed() {
        let corrupt = temp_dir("manifest-corrupt");
        drop(create(&corrupt, &populated_store()).unwrap());
        let mut manifest = fs::read(corrupt.join("MANIFEST")).unwrap();
        manifest[16] ^= 1;
        fs::write(corrupt.join("MANIFEST"), manifest).unwrap();
        assert!(matches!(open(&corrupt), Err(StorageError::Corrupt(_))));
        fs::remove_dir_all(corrupt).unwrap();

        let gap = temp_dir("wal-gap");
        let store = populated_store();
        drop(create(&gap, &store).unwrap());
        append_wal(
            &gap,
            store.dimension(),
            store.generation() + 1,
            &[CoreMutation::Add {
                id: 3,
                user_id: 1,
                timestamp: 0,
                vector: &[1.0, 0.0],
            }],
            MAX_LOAD_BYTES,
        )
        .unwrap();
        assert!(matches!(open(&gap), Err(StorageError::Corrupt(_))));
        fs::remove_dir_all(gap).unwrap();
    }
}
