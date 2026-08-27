use std::{
    ffi::OsStr,
    fs::{self, File, OpenOptions},
    io::{self, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
};

use crc32fast::Hasher;
use qenlo_core::{CoreStore, RestoredRecord};
use thiserror::Error;

const MAGIC: &[u8; 8] = b"QENLODB\0";
const FORMAT_VERSION: u32 = 1;
const HEADER_BYTES: u64 = 8 + 4 + 4 + 8 + 8 + 8;
const CHECKSUM_BYTES: u64 = 4;
const MAX_LOAD_BYTES: u64 = 8 * 1024 * 1024 * 1024;

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
    #[error("collection snapshot exceeds the {MAX_LOAD_BYTES}-byte load limit")]
    LoadLimitExceeded,
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Core(#[from] qenlo_core::Error),
}

#[derive(Debug)]
pub(crate) struct OpenedStore {
    pub store: CoreStore,
    pub recovered_interrupted_write: bool,
}

pub(crate) fn create(path: &Path, store: &CoreStore) -> Result<(), StorageError> {
    if path.exists() {
        let mut entries = fs::read_dir(path).map_err(StorageError::Io)?;
        if entries.next().transpose()?.is_some() {
            return Err(StorageError::AlreadyExists(path.to_path_buf()));
        }
    } else {
        fs::create_dir_all(path)?;
    }
    write_snapshot(path, store)
}

pub(crate) fn open(path: &Path) -> Result<OpenedStore, StorageError> {
    if !path.is_dir() {
        return Err(StorageError::NotFound(path.to_path_buf()));
    }

    let mut committed = Vec::new();
    let mut temporary = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_name = entry.file_name();
        if let Some(generation) = generation_from_name(&file_name, ".qdb") {
            committed.push((generation, entry.path()));
        } else if let Some(generation) = generation_from_name(&file_name, ".tmp") {
            temporary.push((generation, entry.path()));
        }
    }
    committed.sort_unstable_by_key(|(generation, _)| *generation);
    temporary.sort_unstable_by_key(|(generation, _)| *generation);

    let latest_committed = committed.last().cloned();
    let committed_generation = latest_committed
        .as_ref()
        .map_or(0, |(generation, _)| *generation);
    if let Some((generation, temp)) = temporary
        .into_iter()
        .rev()
        .find(|(generation, _)| latest_committed.is_none() || *generation > committed_generation)
        && let Ok(store) = read_snapshot(&temp, generation)
    {
        let final_path = snapshot_path(path, generation, ".qdb");
        fs::rename(&temp, &final_path)?;
        sync_directory(path)?;
        prune_snapshots(path, generation);
        return Ok(OpenedStore {
            store,
            recovered_interrupted_write: true,
        });
    }

    let Some((generation, snapshot)) = latest_committed else {
        return Err(StorageError::NoSnapshot(path.to_path_buf()));
    };
    Ok(OpenedStore {
        store: read_snapshot(&snapshot, generation)?,
        recovered_interrupted_write: false,
    })
}

pub(crate) fn write_snapshot(path: &Path, store: &CoreStore) -> Result<(), StorageError> {
    fs::create_dir_all(path)?;
    let generation = store.generation();
    let final_path = snapshot_path(path, generation, ".qdb");
    if final_path.exists() {
        return Ok(());
    }
    let temp_path = snapshot_path(path, generation, ".tmp");
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temp_path)?;
    let mut writer = CheckedWriter::new(BufWriter::new(file));
    writer.write_all(MAGIC)?;
    writer.write_all(&FORMAT_VERSION.to_le_bytes())?;
    writer.write_all(&(store.dimension() as u32).to_le_bytes())?;
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

    fs::rename(&temp_path, &final_path)?;
    sync_directory(path)?;
    prune_snapshots(path, generation);
    Ok(())
}

fn read_snapshot(path: &Path, expected_generation: u64) -> Result<CoreStore, StorageError> {
    let file = File::open(path)?;
    let file_len = file.metadata()?.len();
    if file_len > MAX_LOAD_BYTES || file_len < HEADER_BYTES + CHECKSUM_BYTES {
        return Err(if file_len > MAX_LOAD_BYTES {
            StorageError::LoadLimitExceeded
        } else {
            StorageError::Corrupt("snapshot is shorter than its header".into())
        });
    }
    let mut reader = CheckedReader::new(BufReader::new(file));
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
    if expected_len != file_len || expected_len > MAX_LOAD_BYTES {
        return Err(StorageError::Corrupt(format!(
            "declared shape requires {expected_len} bytes, file has {file_len}"
        )));
    }
    let rows = usize::try_from(rows_u64).map_err(|_| StorageError::LoadLimitExceeded)?;
    let mut records = Vec::with_capacity(rows);
    let mut actual_live = 0_u64;
    for _ in 0..rows {
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
        records.push(RestoredRecord {
            id,
            user_id,
            timestamp,
            vector,
            live,
        });
    }
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
    CoreStore::restore(dimension, generation, records).map_err(StorageError::Core)
}

fn snapshot_path(path: &Path, generation: u64, extension: &str) -> PathBuf {
    path.join(format!("canonical-{generation:020}{extension}"))
}

fn generation_from_name(name: &OsStr, extension: &str) -> Option<u64> {
    let name = name.to_str()?;
    let digits = name.strip_prefix("canonical-")?.strip_suffix(extension)?;
    (digits.len() == 20).then(|| digits.parse().ok()).flatten()
}

fn prune_snapshots(path: &Path, current_generation: u64) {
    let mut snapshots: Vec<_> = match fs::read_dir(path) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                generation_from_name(&entry.file_name(), ".qdb")
                    .map(|generation| (generation, entry.path()))
            })
            .filter(|(generation, _)| *generation < current_generation)
            .collect(),
        Err(_) => return,
    };
    snapshots.sort_unstable_by_key(|(generation, _)| *generation);
    for (_, old) in snapshots.into_iter().rev().skip(1) {
        let _ = fs::remove_file(old);
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
}
