use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use qenlo::{Collection, CollectionConfig, CollectionStats, Filter, Mutation, StorageOptions};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordDto {
    pub id: u64,
    pub user_id: u64,
    pub timestamp: i64,
    pub vector: Vec<f32>,
    pub live: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedRecords {
    pub records: Vec<RecordDto>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHitDto {
    pub id: u64,
    pub distance: f32,
    pub similarity: f32,
    pub record: Option<RecordDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponseDto {
    pub results: Vec<SearchHitDto>,
    pub total_duration_us: u64,
    pub lock_wait_us: u64,
    pub actual_backend: String,
    pub algorithm: String,
    pub cpu_distance_path: Option<String>,
    pub evaluated_rows: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserStatus {
    pub open: bool,
    pub path: Option<String>,
    pub dimension: usize,
    pub rows: usize,
    pub live_rows: usize,
    pub tombstones: usize,
    pub generation: u64,
    pub durable_generation: Option<u64>,
    pub closed: bool,
    pub storage_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageFileDto {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub is_dir: bool,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageDetailsDto {
    pub path: Option<String>,
    pub files: Vec<StorageFileDto>,
    pub total_bytes: u64,
    pub max_load_bytes: u64,
    pub generation: u64,
    pub durable_generation: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsDto {
    pub os: String,
    pub arch: String,
    pub cpu_distance_path: String,
    pub dimension: usize,
    pub max_k: usize,
}

pub struct BrowserSession {
    pub collection: Option<Arc<Collection>>,
    pub path: Option<PathBuf>,
    pub dimension: usize,
}

impl BrowserSession {
    pub fn new() -> Self {
        Self {
            collection: None,
            path: None,
            dimension: 384,
        }
    }

    pub async fn open_collection(&mut self, path: impl AsRef<Path>, dimension: Option<usize>) -> Result<CollectionStats, String> {
        let path = path.as_ref().to_path_buf();
        
        // If dimension is not provided, check if we can infer or default
        let dim = dimension.unwrap_or(self.dimension);
        let config = CollectionConfig::cpu_exact(dim);
        
        // Close previous if open
        if let Some(c) = self.collection.take() {
            let _ = c.close();
        }

        let collection = if path.extension().map_or(false, |ext| ext == "qn") {
            Collection::import_qn(&path, config).await
                .map_err(|e| format!("Failed to import .qn file: {e}"))?
        } else {
            Collection::open(&path, config).await
                .map_err(|e| format!("Failed to open collection at {}: {e}", path.display()))?
        };

        let stats = collection.stats();
        let actual_dim = stats.dimension;
        self.dimension = actual_dim;
        self.path = Some(path);
        self.collection = Some(Arc::new(collection));

        Ok(stats)
    }

    pub async fn create_collection(&mut self, path: impl AsRef<Path>, dimension: usize) -> Result<CollectionStats, String> {
        let path = path.as_ref().to_path_buf();
        let config = CollectionConfig::cpu_exact(dimension);

        if let Some(c) = self.collection.take() {
            let _ = c.close();
        }

        let collection = Collection::create(&path, config).await
            .map_err(|e| format!("Failed to create collection at {}: {e}", path.display()))?;

        let stats = collection.stats();
        self.dimension = dimension;
        self.path = Some(path);
        self.collection = Some(Arc::new(collection));

        Ok(stats)
    }

    pub async fn create_in_memory(&mut self, dimension: usize) -> Result<CollectionStats, String> {
        let config = CollectionConfig::cpu_exact(dimension);
        if let Some(c) = self.collection.take() {
            let _ = c.close();
        }

        let collection = Collection::new(config).await
            .map_err(|e| format!("Failed to create in-memory collection: {e}"))?;

        let stats = collection.stats();
        self.dimension = dimension;
        self.path = None;
        self.collection = Some(Arc::new(collection));

        Ok(stats)
    }

    pub fn get_status(&self) -> BrowserStatus {
        if let Some(collection) = &self.collection {
            let stats = collection.stats();
            let storage_bytes = self.calc_storage_size();
            BrowserStatus {
                open: true,
                path: self.path.as_ref().map(|p| p.display().to_string()),
                dimension: stats.dimension,
                rows: stats.rows,
                live_rows: stats.live_rows,
                tombstones: stats.rows.saturating_sub(stats.live_rows),
                generation: stats.generation,
                durable_generation: stats.durable_generation,
                closed: stats.closed,
                storage_bytes,
            }
        } else {
            BrowserStatus {
                open: false,
                path: self.path.as_ref().map(|p| p.display().to_string()),
                dimension: self.dimension,
                rows: 0,
                live_rows: 0,
                tombstones: 0,
                generation: 0,
                durable_generation: None,
                closed: true,
                storage_bytes: 0,
            }
        }
    }

    pub fn scan_records(&self, offset: usize, limit: usize, filter: Option<&Filter>) -> Result<PaginatedRecords, String> {
        let collection = self.collection.as_ref().ok_or_else(|| "No collection open".to_string())?;
        let (records, total) = collection.scan_records(offset, limit, filter);
        
        let dtos = records.into_iter().map(|r| RecordDto {
            id: r.id(),
            user_id: r.user_id(),
            timestamp: r.timestamp(),
            vector: r.vector().to_vec(),
            live: r.is_live(),
        }).collect();

        Ok(PaginatedRecords {
            records: dtos,
            total,
            offset,
            limit,
        })
    }

    pub fn get_record(&self, id: u64) -> Result<Option<RecordDto>, String> {
        let collection = self.collection.as_ref().ok_or_else(|| "No collection open".to_string())?;
        Ok(collection.get_record(id).map(|r| RecordDto {
            id: r.id(),
            user_id: r.user_id(),
            timestamp: r.timestamp(),
            vector: r.vector().to_vec(),
            live: r.is_live(),
        }))
    }

    pub async fn search(&self, query: &[f32], filter: &Filter, k: usize) -> Result<SearchResponseDto, String> {
        let collection = self.collection.as_ref().ok_or_else(|| "No collection open".to_string())?;
        let response = collection.search(query, filter, k).await
            .map_err(|e| format!("Search failed: {e}"))?;

        let hits = response.results.into_iter().map(|res| {
            let record = collection.get_record(res.id).map(|r| RecordDto {
                id: r.id(),
                user_id: r.user_id(),
                timestamp: r.timestamp(),
                vector: r.vector().to_vec(),
                live: r.is_live(),
            });
            // Cosine distance is in [0, 2], similarity is 1.0 - distance
            let similarity = (1.0 - res.distance).max(-1.0).min(1.0);
            SearchHitDto {
                id: res.id,
                distance: res.distance,
                similarity,
                record,
            }
        }).collect();

        let cpu_path = response.report.cpu_distance_path.map(|p| format!("{p:?}"));
        let eligible = match response.report.eligible_rows {
            qenlo::Measurement::Available(n) => Some(n),
            qenlo::Measurement::Unavailable(_) => None,
        };

        Ok(SearchResponseDto {
            results: hits,
            total_duration_us: response.report.total_duration.as_micros() as u64,
            lock_wait_us: response.report.lock_wait.as_micros() as u64,
            actual_backend: format!("{:?}", response.report.actual_backend),
            algorithm: format!("{:?}", response.report.algorithm),
            cpu_distance_path: cpu_path,
            evaluated_rows: eligible,
        })
    }

    pub fn add_record(&self, id: u64, user_id: u64, timestamp: i64, vector: &[f32]) -> Result<(), String> {
        let collection = self.collection.as_ref().ok_or_else(|| "No collection open".to_string())?;
        collection.add(id, user_id, timestamp, vector)
            .map_err(|e| format!("Insert failed: {e}"))
    }

    pub fn delete_record(&self, id: u64) -> Result<(), String> {
        let collection = self.collection.as_ref().ok_or_else(|| "No collection open".to_string())?;
        collection.delete(id)
            .map_err(|e| format!("Delete failed: {e}"))
    }

    pub fn commit_mutations(&self, mutations: &[Mutation]) -> Result<qenlo::CommitReport, String> {
        let collection = self.collection.as_ref().ok_or_else(|| "No collection open".to_string())?;
        collection.commit(mutations)
            .map_err(|e| format!("Commit failed: {e}"))
    }

    pub fn flush(&self) -> Result<(), String> {
        let collection = self.collection.as_ref().ok_or_else(|| "No collection open".to_string())?;
        collection.flush()
            .map_err(|e| format!("Flush failed: {e}"))
    }

    pub fn export_qn(&self, path: impl AsRef<Path>) -> Result<(), String> {
        let collection = self.collection.as_ref().ok_or_else(|| "No collection open".to_string())?;
        collection.export_qn(path)
            .map_err(|e| format!("Export failed: {e}"))
    }

    pub fn get_storage_details(&self) -> StorageDetailsDto {
        let mut files = Vec::new();
        let mut total_bytes = 0;

        if let Some(path) = &self.path {
            if path.is_dir() {
                if let Ok(entries) = std::fs::read_dir(path) {
                    for entry in entries.flatten() {
                        let meta = entry.metadata().ok();
                        let size = meta.as_ref().map_or(0, |m| m.len());
                        total_bytes += size;
                        let filename = entry.file_name().to_string_lossy().to_string();
                        let kind = if filename == "HEAD" {
                            "Watermark".to_string()
                        } else if filename.ends_with(".qdb") {
                            "Snapshot".to_string()
                        } else if filename.ends_with(".wal") {
                            "WAL Segment".to_string()
                        } else if filename.ends_with(".pending") {
                            "Staging".to_string()
                        } else if filename.ends_with(".lock") {
                            "Lockfile".to_string()
                        } else if filename.ends_with(".qidx") {
                            "Index Metadata".to_string()
                        } else {
                            "Auxiliary".to_string()
                        };

                        files.push(StorageFileDto {
                            name: filename,
                            path: entry.path().display().to_string(),
                            size_bytes: size,
                            is_dir: meta.map_or(false, |m| m.is_dir()),
                            kind,
                        });
                    }
                }
            } else if path.is_file() {
                let size = std::fs::metadata(path).map_or(0, |m| m.len());
                total_bytes = size;
                files.push(StorageFileDto {
                    name: path.file_name().map_or("file".to_string(), |n| n.to_string_lossy().to_string()),
                    path: path.display().to_string(),
                    size_bytes: size,
                    is_dir: false,
                    kind: "Portable .qn Archive".to_string(),
                });
            }
        }

        let (generation_num, d_gen) = if let Some(c) = &self.collection {
            let s = c.stats();
            (s.generation, s.durable_generation)
        } else {
            (0, None)
        };

        files.sort_by(|a, b| a.name.cmp(&b.name));

        StorageDetailsDto {
            path: self.path.as_ref().map(|p| p.display().to_string()),
            files,
            total_bytes,
            max_load_bytes: StorageOptions::default().max_load_bytes,
            generation: generation_num,
            durable_generation: d_gen,
        }
    }

    pub fn get_diagnostics(&self) -> DiagnosticsDto {
        let cpu_path = format!("{:?}", qenlo_core::cpu_distance_path());
        DiagnosticsDto {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            cpu_distance_path: cpu_path,
            dimension: self.dimension,
            max_k: qenlo::MAX_K,
        }
    }

    fn calc_storage_size(&self) -> u64 {
        if let Some(path) = &self.path {
            if path.is_dir() {
                if let Ok(entries) = std::fs::read_dir(path) {
                    return entries.flatten().filter_map(|e| e.metadata().ok()).map(|m| m.len()).sum();
                }
            } else if path.is_file() {
                return std::fs::metadata(path).map_or(0, |m| m.len());
            }
        }
        0
    }
}

pub type SharedState = Arc<RwLock<BrowserSession>>;
