//! Disposable readiness metadata; never an authority for canonical rows.

use crate::PreparationReason;
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::Path,
};

const MAGIC: &[u8; 8] = b"QENLOIX1";
const SIZE: usize = 29;

pub(crate) fn inspect(
    path: &Path,
    dimension: usize,
    generation: u64,
    backend: u8,
) -> (Option<u64>, PreparationReason) {
    let mut file = match File::open(path.join("index.qidx")) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return (None, PreparationReason::MissingIndex);
        }
        Err(_) => return (None, PreparationReason::CorruptIndex),
    };
    let mut bytes = [0; SIZE];
    if file
        .metadata()
        .map_or(true, |metadata| metadata.len() != SIZE as u64)
        || file.read_exact(&mut bytes).is_err()
        || &bytes[..8] != MAGIC
        || crc32fast::hash(&bytes[..25]) != u32::from_le_bytes(bytes[25..].try_into().unwrap())
    {
        return (None, PreparationReason::CorruptIndex);
    }
    let stored_dimension = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
    let stored_generation = u64::from_le_bytes(bytes[16..24].try_into().unwrap());
    let reason = if stored_generation != generation
        || stored_dimension != dimension as u64
        || bytes[24] != backend
    {
        PreparationReason::StaleIndex
    } else {
        // Resident handles are not serialized. A matching marker still needs
        // rebuilding on restart; it records readiness, not graph bytes.
        PreparationReason::Restart
    };
    (Some(stored_generation), reason)
}

pub(crate) fn save(path: &Path, dimension: usize, generation: u64, backend: u8) -> io::Result<()> {
    let mut bytes = Vec::with_capacity(SIZE);
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&(dimension as u64).to_le_bytes());
    bytes.extend_from_slice(&generation.to_le_bytes());
    bytes.push(backend);
    bytes.extend_from_slice(&crc32fast::hash(&bytes).to_le_bytes());
    let staged = path.join("index.pending");
    let mut file = File::create(&staged)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(staged, path.join("index.qidx"))?;
    #[cfg(unix)]
    File::open(path)?.sync_all()?;
    Ok(())
}
