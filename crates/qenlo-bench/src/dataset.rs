//! Streaming preparation and validated loading of deterministic benchmark data.
//!
//! Corpus, tuning, and evaluation occupy disjoint source-row intervals. CRC32
//! detects accidental damage, not malicious replacement; pin a publisher's
//! cryptographic checksum separately when importing an external dataset.

use std::{
    fs::File,
    io::{self, BufReader, BufWriter, Read, Write},
    path::Path,
};

use crate::{OracleRecord, SplitMix64, checked_norm, unit};

const MAGIC: &[u8; 8] = b"QNLOB001";
const HEADER_BYTES: u64 = 56;

/// Exact source-row split, persisted in the dataset header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DatasetSpec {
    pub dimension: usize,
    pub corpus: usize,
    pub tuning: usize,
    pub evaluation: usize,
    pub seed: u64,
}

impl DatasetSpec {
    fn rows(self) -> io::Result<usize> {
        if self.dimension == 0 || self.corpus == 0 || self.tuning == 0 || self.evaluation == 0 {
            return Err(invalid(
                "dimension and all three split sizes must be nonzero",
            ));
        }
        self.corpus
            .checked_add(self.tuning)
            .and_then(|v| v.checked_add(self.evaluation))
            .ok_or_else(|| invalid("dataset row count overflow"))
    }

    fn payload_bytes(self) -> io::Result<u64> {
        (self.rows()? as u64)
            .checked_mul(self.dimension as u64)
            .and_then(|v| v.checked_mul(4))
            .ok_or_else(|| invalid("dataset byte count overflow"))
    }
}

/// Validated vectors; IDs are source row numbers, never reused between splits.
pub struct Dataset {
    pub spec: DatasetSpec,
    pub checksum: u32,
    /// Raw source CRC32; absent for generated synthetic data.
    pub source_checksum: Option<u32>,
    pub corpus: Vec<OracleRecord>,
    pub tuning: Vec<Vec<f32>>,
    pub evaluation: Vec<Vec<f32>>,
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

/// CRC32 of a file using a bounded 64 KiB scratch buffer.
pub fn checksum(path: &Path) -> io::Result<u32> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut hasher = crc32fast::Hasher::new();
    let mut buffer = [0; 65_536];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hasher.finalize())
}

/// Prepare generated vectors, or import raw little-endian f32 rows after checking
/// the required expected source CRC32. Existing output files are never replaced.
/// A failed preparation may leave a partial file; `load` rejects it.
pub fn prepare(path: &Path, spec: DatasetSpec, source: Option<(&Path, u32)>) -> io::Result<u32> {
    let bytes = spec.payload_bytes()?;
    let mut input = if let Some((source, expected)) = source {
        if source.metadata()?.len() != bytes {
            return Err(invalid(
                "raw source length does not match dimensions and split sizes",
            ));
        }
        if checksum(source)? != expected {
            return Err(invalid("raw source CRC32 mismatch"));
        }
        Some(BufReader::new(File::open(source)?))
    } else {
        None
    };
    let mut output = BufWriter::new(File::create_new(path)?);
    let mut hasher = crc32fast::Hasher::new();
    let mut header = MAGIC.to_vec();
    for value in [
        spec.dimension as u64,
        spec.corpus as u64,
        spec.tuning as u64,
        spec.evaluation as u64,
        spec.seed,
        source.map_or(0, |(_, checksum)| (1_u64 << 32) | u64::from(checksum)),
    ] {
        header.extend_from_slice(&value.to_le_bytes());
    }
    output.write_all(&header)?;
    hasher.update(&header);
    let mut rng = SplitMix64(spec.seed);
    let mut row = Vec::new();
    row.try_reserve_exact(spec.dimension)
        .map_err(|_| invalid("vector allocation failed"))?;
    let mut source_hasher = crc32fast::Hasher::new();
    for _ in 0..spec.rows()? {
        row.clear();
        for _ in 0..spec.dimension {
            let value = if let Some(reader) = &mut input {
                let mut bytes = [0; 4];
                reader.read_exact(&mut bytes)?;
                f32::from_le_bytes(bytes)
            } else {
                (unit(rng.next()) * 2.0 - 1.0) as f32
            };
            row.push(value);
        }
        checked_norm(&row, None)
            .map_err(|_| invalid("source has non-finite or zero-norm vector"))?;
        for value in &row {
            let bytes = value.to_le_bytes();
            output.write_all(&bytes)?;
            hasher.update(&bytes);
            source_hasher.update(&bytes);
        }
    }
    if source.is_some_and(|(_, expected)| source_hasher.finalize() != expected) {
        return Err(invalid("raw source changed during preparation"));
    }
    let checksum = hasher.finalize();
    output.write_all(&checksum.to_le_bytes())?;
    output.flush()?;
    output.get_ref().sync_all()?;
    Ok(checksum)
}

/// Validate length, version, vectors, checksum and dimension before returning data.
/// `vector_budget_bytes` caps the combined source + normalized corpus vector payload
/// estimate, not process RSS or metadata/index allocations.
pub fn load(
    path: &Path,
    expected_dimension: usize,
    vector_budget_bytes: u64,
) -> io::Result<Dataset> {
    let file = File::open(path)?;
    let file_len = file.metadata()?.len();
    let mut reader = BufReader::new(file);
    let mut header = [0; HEADER_BYTES as usize];
    reader.read_exact(&mut header)?;
    if &header[..8] != MAGIC {
        return Err(invalid("unsupported dataset magic/version"));
    }
    let mut values = [0; 6];
    for (value, bytes) in values.iter_mut().zip(header[8..].as_chunks::<8>().0) {
        *value = u64::from_le_bytes(*bytes);
    }
    let size = |value| usize::try_from(value).map_err(|_| invalid("dataset size exceeds platform"));
    let spec = DatasetSpec {
        dimension: size(values[0])?,
        corpus: size(values[1])?,
        tuning: size(values[2])?,
        evaluation: size(values[3])?,
        seed: values[4],
    };
    let source_checksum = match values[5] >> 32 {
        0 if values[5] == 0 => None,
        1 => Some(values[5] as u32),
        _ => return Err(invalid("unknown dataset source kind")),
    };
    if spec.dimension != expected_dimension {
        return Err(invalid("dataset dimension mismatch"));
    }
    let payload = spec.payload_bytes()?;
    if payload.checked_add(HEADER_BYTES + 4) != Some(file_len) {
        return Err(invalid("dataset is truncated or has trailing bytes"));
    }
    let copied_corpus = (spec.corpus as u64)
        .checked_mul(spec.dimension as u64)
        .and_then(|v| v.checked_mul(4))
        .ok_or_else(|| invalid("corpus size overflow"))?;
    if payload
        .checked_add(copied_corpus)
        .is_none_or(|n| n > vector_budget_bytes)
    {
        return Err(invalid(
            "source + collection vector payload exceeds --vector-budget-mib",
        ));
    }
    let mut corpus = Vec::new();
    let mut tuning = Vec::new();
    let mut evaluation = Vec::new();
    corpus
        .try_reserve_exact(spec.corpus)
        .map_err(|_| invalid("corpus allocation failed"))?;
    tuning
        .try_reserve_exact(spec.tuning)
        .map_err(|_| invalid("tuning allocation failed"))?;
    evaluation
        .try_reserve_exact(spec.evaluation)
        .map_err(|_| invalid("evaluation allocation failed"))?;
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(&header);
    for index in 0..spec.rows()? {
        let mut vector = Vec::new();
        vector
            .try_reserve_exact(spec.dimension)
            .map_err(|_| invalid("vector allocation failed"))?;
        for _ in 0..spec.dimension {
            let mut bytes = [0; 4];
            reader.read_exact(&mut bytes)?;
            hasher.update(&bytes);
            vector.push(f32::from_le_bytes(bytes));
        }
        checked_norm(&vector, None).map_err(|_| invalid("invalid prepared vector"))?;
        if index < spec.corpus {
            corpus.push(OracleRecord {
                id: index as u64,
                user_id: 0,
                timestamp_micros: 0,
                vector,
                deleted: false,
            });
        } else if index < spec.corpus + spec.tuning {
            tuning.push(vector);
        } else {
            evaluation.push(vector);
        }
    }
    let mut expected = [0; 4];
    reader.read_exact(&mut expected)?;
    let checksum = hasher.finalize();
    if checksum != u32::from_le_bytes(expected) {
        return Err(invalid("prepared dataset CRC32 mismatch"));
    }
    Ok(Dataset {
        spec,
        checksum,
        source_checksum,
        corpus,
        tuning,
        evaluation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn deterministic_split_dimension_checksum_and_budget_validation() {
        let path = std::env::temp_dir().join(format!(
            "qenlo-dataset-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let spec = DatasetSpec {
            dimension: 4,
            corpus: 12,
            tuning: 3,
            evaluation: 5,
            seed: 7,
        };
        let crc = prepare(&path, spec, None).unwrap();
        let data = load(&path, 4, 4096).unwrap();
        assert_eq!(data.checksum, crc);
        assert_eq!(data.spec, spec);
        assert_eq!(data.corpus.len(), 12);
        assert_eq!(data.tuning.len(), 3);
        assert_eq!(data.evaluation.len(), 5);
        for query in data.tuning.iter().chain(&data.evaluation) {
            assert!(data.corpus.iter().all(|row| row.vector != *query));
        }
        assert!(
            data.tuning
                .iter()
                .all(|query| !data.evaluation.contains(query))
        );
        assert!(load(&path, 8, 4096).is_err());
        assert!(load(&path, 4, 1).is_err());
        assert!(prepare(&path, spec, None).is_err());
        let repeated = path.with_extension("repeated");
        assert_eq!(prepare(&repeated, spec, None).unwrap(), crc);
        assert_eq!(
            std::fs::read(&path).unwrap(),
            std::fs::read(&repeated).unwrap()
        );
        std::fs::remove_file(repeated).unwrap();
        let mut bytes = std::fs::read(&path).unwrap();
        let raw = path.with_extension("raw");
        let imported = path.with_extension("imported");
        std::fs::write(&raw, &bytes[HEADER_BYTES as usize..bytes.len() - 4]).unwrap();
        let source_crc = checksum(&raw).unwrap();
        assert!(prepare(&imported, spec, Some((&raw, source_crc ^ 1))).is_err());
        assert!(!imported.exists());
        prepare(&imported, spec, Some((&raw, source_crc))).unwrap();
        let imported_data = load(&imported, 4, 4096).unwrap();
        assert_eq!(imported_data.source_checksum, Some(source_crc));
        assert_eq!(data.corpus[0].vector, imported_data.corpus[0].vector);
        std::fs::remove_file(imported).unwrap();
        std::fs::remove_file(raw).unwrap();
        bytes[HEADER_BYTES as usize] ^= 1;
        std::fs::write(&path, &bytes).unwrap();
        assert!(load(&path, 4, 4096).is_err());
        std::fs::remove_file(path).unwrap();
    }
}
