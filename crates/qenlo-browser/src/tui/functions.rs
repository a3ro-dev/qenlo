//! Static reference catalog for the `?  Functions` browser tab.
//!
//! Every entry documents one public method of the first `impl Collection {`
//! block in `qenlo/src/lib.rs`. The `catalog_matches_collection_api` test
//! below re-parses that block on every test run so the catalog cannot
//! silently drift from the real SDK surface.

/// One documented `Collection` method shown in the Functions tab.
pub struct FunctionDoc {
    pub name: &'static str,
    pub group: &'static str,
    pub signature: &'static str,
    pub summary: &'static str,
    pub example: &'static str,
}

pub const FUNCTION_CATALOG: [FunctionDoc; 33] = [
    // --- Open & create ---
    FunctionDoc {
        name: "new",
        group: "Open & create",
        signature: "async fn new(config: CollectionConfig) -> Result<Self, Error>",
        summary: "Construct an in-memory collection; use `create` for durable commits.",
        example: "let db = Collection::new(CollectionConfig::cpu_exact(128)).await?;",
    },
    FunctionDoc {
        name: "new_with_options",
        group: "Open & create",
        signature: "async fn new_with_options(config: CollectionConfig, options: StorageOptions) -> Result<Self, Error>",
        summary: "Construct an in-memory collection with an explicit canonical-memory admission budget.",
        example: "let db = Collection::new_with_options(CollectionConfig::cpu_exact(128), StorageOptions { max_load_bytes: 64 << 20 }).await?;",
    },
    FunctionDoc {
        name: "create",
        group: "Open & create",
        signature: "async fn create(path: impl AsRef<Path>, config: CollectionConfig) -> Result<Self, Error>",
        summary: "Create a collection in an empty directory, held under an exclusive OS lock.",
        example: "let db = Collection::create(\"vectors.qenlo\", CollectionConfig::cpu_exact(128)).await?;",
    },
    FunctionDoc {
        name: "open",
        group: "Open & create",
        signature: "async fn open(path: impl AsRef<Path>, config: CollectionConfig) -> Result<Self, Error>",
        summary: "Recover a durable collection. A second open handle is rejected.",
        example: "let db = Collection::open(\"vectors.qenlo\", CollectionConfig::cpu_exact(128)).await?;",
    },
    FunctionDoc {
        name: "import_qn",
        group: "Open & create",
        signature: "async fn import_qn(path: impl AsRef<Path>, config: CollectionConfig) -> Result<Self, Error>",
        summary: "Import a checksummed portable `.qn` file into a mutable in-memory collection; the source file is left untouched.",
        example: "let db = Collection::import_qn(\"backup.qn\", CollectionConfig::cpu_exact(128)).await?;",
    },
    FunctionDoc {
        name: "create_with_options",
        group: "Open & create",
        signature: "async fn create_with_options(path: impl AsRef<Path>, config: CollectionConfig, options: StorageOptions) -> Result<Self, Error>",
        summary: "Create with an explicit canonical-memory admission budget.",
        example: "let db = Collection::create_with_options(\"vectors.qenlo\", CollectionConfig::cpu_exact(128), StorageOptions { max_load_bytes: 1 << 30 }).await?;",
    },
    FunctionDoc {
        name: "open_with_options",
        group: "Open & create",
        signature: "async fn open_with_options(path: impl AsRef<Path>, config: CollectionConfig, options: StorageOptions) -> Result<Self, Error>",
        summary: "Open with an explicit budget, for collections above the 512 MiB default.",
        example: "let db = Collection::open_with_options(\"vectors.qenlo\", CollectionConfig::cpu_exact(128), StorageOptions { max_load_bytes: 2 << 30 }).await?;",
    },
    FunctionDoc {
        name: "import_qn_with_options",
        group: "Open & create",
        signature: "async fn import_qn_with_options(path: impl AsRef<Path>, config: CollectionConfig, options: StorageOptions) -> Result<Self, Error>",
        summary: "Import a portable `.qn` file with an explicit canonical-memory admission budget.",
        example: "let db = Collection::import_qn_with_options(\"backup.qn\", CollectionConfig::cpu_exact(128), StorageOptions { max_load_bytes: 1 << 30 }).await?;",
    },
    // --- Write ---
    FunctionDoc {
        name: "add",
        group: "Write",
        signature: "fn add(&self, id: u64, user_id: u64, timestamp: i64, vector: &[f32]) -> Result<(), Error>",
        summary: "Validate and add one row; durable collections sync before returning.",
        example: "db.add(1, 7, 1_700_000_000, &[1.0, 0.0])?;",
    },
    FunctionDoc {
        name: "delete",
        group: "Write",
        signature: "fn delete(&self, id: u64) -> Result<(), Error>",
        summary: "Delete one row immediately and durably; IDs are never reused.",
        example: "db.delete(1)?;",
    },
    FunctionDoc {
        name: "commit",
        group: "Write",
        signature: "fn commit(&self, mutations: &[Mutation]) -> Result<CommitReport, Error>",
        summary: "Commit ordered add/delete operations atomically. Errors roll back before publication.",
        example: "let report = db.commit(&[Mutation::Add(record), Mutation::Delete(1)])?;",
    },
    FunctionDoc {
        name: "add_batch",
        group: "Write",
        signature: "fn add_batch(&self, rows: &[NewRecord]) -> Result<CommitReport, Error>",
        summary: "Add all rows atomically, rejecting duplicate IDs and invalid vectors.",
        example: "db.add_batch(&[NewRecord { id: 1, user_id: 7, timestamp: -1, vector: vec![1.0, 0.0] }])?;",
    },
    FunctionDoc {
        name: "delete_batch",
        group: "Write",
        signature: "fn delete_batch(&self, ids: &[u64]) -> Result<CommitReport, Error>",
        summary: "Delete all IDs atomically; any invalid deletion rolls back the batch.",
        example: "db.delete_batch(&[1, 2, 3])?;",
    },
    // --- Read & filter ---
    FunctionDoc {
        name: "filter",
        group: "Read & filter",
        signature: "fn filter(&self, filter: &Filter) -> Vec<u64>",
        summary: "Return live eligible IDs. Retains the original empty-on-closed behavior.",
        example: "let ids = db.filter(&Filter::new(Some(7), TimestampRange::ALL));",
    },
    FunctionDoc {
        name: "dimension",
        group: "Read & filter",
        signature: "fn dimension(&self) -> usize",
        summary: "Return the dimension configured for this collection.",
        example: "let dim = db.dimension();",
    },
    FunctionDoc {
        name: "get_record",
        group: "Read & filter",
        signature: "fn get_record(&self, id: u64) -> Option<Record>",
        summary: "Retrieve a single canonical record by ID, including tombstones.",
        example: "if let Some(rec) = db.get_record(1) { println!(\"{}\", rec.user_id()); }",
    },
    FunctionDoc {
        name: "scan_records",
        group: "Read & filter",
        signature: "fn scan_records(&self, offset: usize, limit: usize, filter: Option<&Filter>) -> (Vec<Record>, usize)",
        summary: "Return a paginated slice of records matching an optional filter, along with total matching count.",
        example: "let (page, total) = db.scan_records(0, 50, None);",
    },
    FunctionDoc {
        name: "canonical_snapshot",
        group: "Read & filter",
        signature: "fn canonical_snapshot(&self, filter: &Filter) -> Result<CanonicalSnapshot, Error>",
        summary: "Copy live rows matching `filter` while holding one canonical read generation.",
        example: "let snap = db.canonical_snapshot(&Filter::ALL)?;",
    },
    // --- Search ---
    FunctionDoc {
        name: "prepare",
        group: "Search",
        signature: "async fn prepare(&self) -> Result<bool, Error>",
        summary: "Explicitly prepare the current generation, serialized with all mutations.",
        example: "let rebuilt = db.prepare().await?;",
    },
    FunctionDoc {
        name: "search",
        group: "Search",
        signature: "async fn search(&self, query: &[f32], filter: &Filter, k: usize) -> Result<SearchResponse, Error>",
        summary: "Search one committed generation. A concurrent commit is visible entirely or not at all.",
        example: "let hits = db.search(&query, &Filter::default(), 10).await?;",
    },
    FunctionDoc {
        name: "search_batch",
        group: "Search",
        signature: "async fn search_batch(&self, queries: &[&[f32]], filter: &Filter, k: usize) -> Result<Vec<SearchResponse>, Error>",
        summary: "Search one committed generation. GPU-capable backends execute one native batch.",
        example: "let batch = db.search_batch(&[&q1, &q2], &Filter::default(), 10).await?;",
    },
    // --- Tuning & diagnostics ---
    FunctionDoc {
        name: "stats",
        group: "Tuning & diagnostics",
        signature: "fn stats(&self) -> CollectionStats",
        summary: "Inspect canonical and durable generations without scanning vectors.",
        example: "let live = db.stats().live_rows;",
    },
    FunctionDoc {
        name: "set_rebuild_policy",
        group: "Tuning & diagnostics",
        signature: "fn set_rebuild_policy(&self, policy: RebuildPolicy) -> Result<(), Error>",
        summary: "Change rebuild policy; explicit preparation is always allowed.",
        example: "db.set_rebuild_policy(RebuildPolicy::Explicit)?;",
    },
    FunctionDoc {
        name: "set_diagnostics",
        group: "Tuning & diagnostics",
        signature: "fn set_diagnostics(&self, diagnostics: Diagnostics)",
        summary: "Select tracing detail; only `Detailed` adds an eligibility-count scan.",
        example: "db.set_diagnostics(Diagnostics::Detailed);",
    },
    FunctionDoc {
        name: "set_gpu_row_preparation",
        group: "Tuning & diagnostics",
        signature: "fn set_gpu_row_preparation(&self, mode: GpuRowPreparation)",
        summary: "Select eligible-row preparation for exact WGPU row and mask filters. Requires the `gpu-wgpu` feature.",
        example: "db.set_gpu_row_preparation(GpuRowPreparation::Cached);",
    },
    FunctionDoc {
        name: "set_router_profile",
        group: "Tuning & diagnostics",
        signature: "fn set_router_profile(&self, profile: Option<RouterProfile>)",
        summary: "Install a tuning-only automatic router profile. It is ignored on mismatch. Requires the `gpu-wgpu` feature.",
        example: "db.set_router_profile(None);",
    },
    FunctionDoc {
        name: "set_gpu_ivf",
        group: "Tuning & diagnostics",
        signature: "fn set_gpu_ivf(&self, lists: usize, nprobe: usize) -> Result<(), Error>",
        summary: "Enable portable IVF candidate generation with exact FP32 GPU re-ranking. Requires the `gpu-wgpu` feature.",
        example: "db.set_gpu_ivf(32, 8)?;",
    },
    FunctionDoc {
        name: "set_gpu_ivf_sq8",
        group: "Tuning & diagnostics",
        signature: "fn set_gpu_ivf_sq8(&self, lists: usize, nprobe: usize) -> Result<(), Error>",
        summary: "Enable SQ8 coarse-centroid scoring plus exact FP32 GPU re-ranking. Requires the `gpu-wgpu` feature.",
        example: "db.set_gpu_ivf_sq8(32, 8)?;",
    },
    FunctionDoc {
        name: "gpu_capabilities",
        group: "Tuning & diagnostics",
        signature: "fn gpu_capabilities(&self) -> Option<GpuCapabilities>",
        summary: "Negotiated adapter capabilities without exposing backend-specific types. Requires the `gpu-wgpu` feature.",
        example: "if let Some(caps) = db.gpu_capabilities() { println!(\"{}\", caps.adapter_name); }",
    },
    FunctionDoc {
        name: "set_ann_search_expansion",
        group: "Tuning & diagnostics",
        signature: "fn set_ann_search_expansion(&self, expansion: usize) -> Result<(), Error>",
        summary: "Configure USearch's search expansion for this handle, including future rebuilds. Requires the `usearch` feature.",
        example: "db.set_ann_search_expansion(128)?;",
    },
    // --- Persistence & lifecycle ---
    FunctionDoc {
        name: "flush",
        group: "Persistence & lifecycle",
        signature: "fn flush(&self) -> Result<(), Error>",
        summary: "Sync pending canonical data. Normal durable mutations are already synced.",
        example: "db.flush()?;",
    },
    FunctionDoc {
        name: "export_qn",
        group: "Persistence & lifecycle",
        signature: "fn export_qn(&self, path: impl AsRef<Path>) -> Result<(), Error>",
        summary: "Atomically export the current canonical generation as one portable `.qn` file; existing targets are never overwritten.",
        example: "db.export_qn(\"backup.qn\")?;",
    },
    FunctionDoc {
        name: "close",
        group: "Persistence & lifecycle",
        signature: "fn close(&self) -> Result<(), Error>",
        summary: "Flush, mark closed, and release the exclusive filesystem lock. Idempotent.",
        example: "db.close()?;",
    },
];

/// Indices into [`FUNCTION_CATALOG`] whose name, group, or summary contains
/// `query` case-insensitively. An empty query matches every entry.
pub fn filter_functions(query: &str) -> Vec<usize> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return (0..FUNCTION_CATALOG.len()).collect();
    }
    FUNCTION_CATALOG
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            f.name.to_lowercase().contains(&query)
                || f.group.to_lowercase().contains(&query)
                || f.summary.to_lowercase().contains(&query)
        })
        .map(|(i, _)| i)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Extracts `NAME` from a line shaped like `    pub fn NAME(...` or
    /// `    pub async fn NAME(...`, i.e. a method one indent level (4 spaces)
    /// inside an `impl` block. Returns `None` for anything else.
    fn method_name(line: &str) -> Option<&str> {
        let rest = line
            .strip_prefix("    pub async fn ")
            .or_else(|| line.strip_prefix("    pub fn "))?;
        rest.split(|c: char| c == '(' || c == '<' || c.is_whitespace())
            .next()
            .filter(|s| !s.is_empty())
    }

    /// Drift guard: re-parses the first `impl Collection {` block in the real
    /// SDK and asserts the catalog documents exactly its public methods.
    #[test]
    fn catalog_matches_collection_api() {
        let src = include_str!("../../../qenlo/src/lib.rs");
        let start = src
            .lines()
            .position(|l| l == "impl Collection {")
            .expect("could not find `impl Collection {` block in qenlo/src/lib.rs");

        let mut real_names = BTreeSet::new();
        for line in src.lines().skip(start + 1) {
            if line == "}" {
                break; // end of the top-level impl block
            }
            if let Some(name) = method_name(line) {
                real_names.insert(name);
            }
        }

        let catalog_names: BTreeSet<&str> = FUNCTION_CATALOG.iter().map(|f| f.name).collect();

        let missing: Vec<&&str> = real_names.difference(&catalog_names).collect();
        let extra: Vec<&&str> = catalog_names.difference(&real_names).collect();

        assert!(
            missing.is_empty() && extra.is_empty(),
            "FUNCTION_CATALOG has drifted from Collection's public API — \
             missing from catalog: {missing:?}, extra entries not in the SDK: {extra:?}"
        );
    }

    #[test]
    fn filter_by_substring_matches_batch_group() {
        let idxs = filter_functions("BaTcH");
        let mut names: Vec<&str> = idxs.iter().map(|&i| FUNCTION_CATALOG[i].name).collect();
        names.sort_unstable();
        assert_eq!(names, ["add_batch", "delete_batch", "search_batch"]);
    }

    #[test]
    fn filter_matches_across_group_and_summary_too() {
        // "gpu-wgpu" only appears in summaries, not in any function name.
        let idxs = filter_functions("gpu-wgpu");
        assert!(!idxs.is_empty());
        assert!(
            idxs.iter()
                .all(|&i| FUNCTION_CATALOG[i].summary.contains("gpu-wgpu"))
        );
    }

    #[test]
    fn unmatched_query_yields_empty_set_without_panicking() {
        let idxs = filter_functions("no_such_function_zzz");
        assert!(idxs.is_empty());

        // Mirror the UI's selection clamp: a stale, nonzero index over an
        // empty result set must not panic (e.g. via `len() - 1` underflow)
        // and must produce no valid selection.
        let stale_idx = 5usize;
        let selected = if idxs.is_empty() {
            None
        } else {
            Some(stale_idx.min(idxs.len() - 1))
        };
        assert_eq!(selected, None);
    }

    #[test]
    fn empty_query_matches_everything() {
        assert_eq!(filter_functions("").len(), FUNCTION_CATALOG.len());
    }
}
