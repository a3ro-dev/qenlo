//! Stable, panic-contained C ABI shared by Qenlo's non-Rust SDKs.

use std::{
    cell::RefCell,
    ffi::{CStr, CString, c_char},
    panic::{AssertUnwindSafe, catch_unwind},
    ptr, slice,
};

#[cfg(feature = "gpu-wgpu")]
use qenlo::{BackendSelection, GpuFilterMode};
use qenlo::{Collection, CollectionConfig, Filter, Measurement, NewRecord, TimestampRange};
use serde::Serialize;

thread_local! {
    static LAST_ERROR: RefCell<String> = const { RefCell::new(String::new()) };
}

#[repr(C)]
pub struct QenloCollection {
    collection: Collection,
    dimension: usize,
}

/// Owned, generation-bound live-row snapshot for typed SDK integrations.
#[repr(C)]
pub struct QenloSnapshot {
    generation: u64,
    rows: usize,
    dimension: usize,
    ids: Vec<u64>,
    vectors: Vec<f32>,
}

/// Owned typed search output. Result arrays and report share one completed call.
#[repr(C)]
pub struct QenloSearchResults {
    ids: Vec<u64>,
    distances: Vec<f32>,
    report: JsonReport,
}

#[derive(Serialize)]
struct JsonHit {
    id: String,
    distance: f32,
}

#[derive(Serialize)]
struct JsonSearch {
    results: Vec<JsonHit>,
    report: JsonReport,
}

#[derive(Clone, Serialize)]
struct JsonReport {
    operation_id: String,
    requested_backend: String,
    actual_backend: String,
    algorithm: String,
    filter_execution: String,
    index_generation: String,
    rebuilt: bool,
    routing_reason: Option<String>,
    fallback_reason: Option<String>,
    total_duration_ns: String,
    lock_wait_ns: String,
    eligible_rows: Option<String>,
    upload_bytes: Option<String>,
    readback_bytes: Option<String>,
    allocation_bytes: Option<String>,
    dispatch_count: Option<u32>,
    candidates: Option<String>,
    batch_size: usize,
}

#[derive(Serialize)]
struct JsonStats {
    dimension: usize,
    rows: usize,
    live_rows: usize,
    generation: String,
    prepared_generation: Option<String>,
    durable_generation: Option<String>,
    recovered_interrupted_write: bool,
    closed: bool,
}

fn measured<T: Copy>(value: &Measurement<T>) -> Option<T> {
    match value {
        Measurement::Available(value) => Some(*value),
        Measurement::Unavailable(_) => None,
    }
}

fn duration_ns(value: web_time::Duration) -> String {
    (value.as_nanos().min(u128::from(u64::MAX)) as u64).to_string()
}

fn set_error(error: impl ToString) {
    LAST_ERROR.with(|message| *message.borrow_mut() = error.to_string());
}

fn clear_error() {
    LAST_ERROR.with(|message| message.borrow_mut().clear());
}

fn ffi_status(operation: impl FnOnce() -> Result<(), qenlo::Error>) -> i32 {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(())) => {
            clear_error();
            0
        }
        Ok(Err(error)) => {
            set_error(error);
            -1
        }
        Err(_) => {
            set_error("Qenlo panicked across the native boundary");
            -2
        }
    }
}

fn ffi_pointer(
    operation: impl FnOnce() -> Result<QenloCollection, String>,
) -> *mut QenloCollection {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(collection)) => {
            clear_error();
            Box::into_raw(Box::new(collection))
        }
        Ok(Err(error)) => {
            set_error(error);
            ptr::null_mut()
        }
        Err(_) => {
            set_error("Qenlo panicked across the native boundary");
            ptr::null_mut()
        }
    }
}

fn ffi_snapshot_pointer(
    operation: impl FnOnce() -> Result<QenloSnapshot, String>,
) -> *mut QenloSnapshot {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(snapshot)) => {
            clear_error();
            Box::into_raw(Box::new(snapshot))
        }
        Ok(Err(error)) => {
            set_error(error);
            ptr::null_mut()
        }
        Err(_) => {
            set_error("Qenlo panicked across the native boundary");
            ptr::null_mut()
        }
    }
}

fn ffi_search_pointer(
    operation: impl FnOnce() -> Result<QenloSearchResults, String>,
) -> *mut QenloSearchResults {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(results)) => {
            clear_error();
            Box::into_raw(Box::new(results))
        }
        Ok(Err(error)) => {
            set_error(error);
            ptr::null_mut()
        }
        Err(_) => {
            set_error("Qenlo panicked across the native boundary");
            ptr::null_mut()
        }
    }
}

fn json_pointer<T: Serialize>(operation: impl FnOnce() -> Result<T, String>) -> *mut c_char {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(value)) => match serde_json::to_string(&value)
            .map_err(|error| error.to_string())
            .and_then(|json| CString::new(json).map_err(|error| error.to_string()))
        {
            Ok(json) => {
                clear_error();
                json.into_raw()
            }
            Err(error) => {
                set_error(error);
                ptr::null_mut()
            }
        },
        Ok(Err(error)) => {
            set_error(error);
            ptr::null_mut()
        }
        Err(_) => {
            set_error("Qenlo panicked across the native boundary");
            ptr::null_mut()
        }
    }
}

unsafe fn collection<'a>(value: *mut QenloCollection) -> Result<&'a QenloCollection, String> {
    // SAFETY: validated for null here; ownership remains with the caller.
    unsafe { value.as_ref() }.ok_or_else(|| "collection pointer is null".to_owned())
}

unsafe fn snapshot<'a>(value: *mut QenloSnapshot) -> Result<&'a QenloSnapshot, String> {
    // SAFETY: validated for null here; ownership remains with the caller.
    unsafe { value.as_ref() }.ok_or_else(|| "snapshot pointer is null".to_owned())
}

unsafe fn search_results<'a>(
    value: *mut QenloSearchResults,
) -> Result<&'a QenloSearchResults, String> {
    // SAFETY: validated for null here; ownership remains with the caller.
    unsafe { value.as_ref() }.ok_or_else(|| "search results pointer is null".to_owned())
}

unsafe fn output_values<'a, T>(
    value: *mut T,
    len: usize,
    required: usize,
    name: &str,
) -> Result<&'a mut [T], String> {
    if len < required {
        return Err(format!("{name} output needs {required} values, got {len}"));
    }
    if required == 0 {
        return Ok(&mut []);
    }
    if value.is_null() {
        return Err(format!("{name} output pointer is null"));
    }
    // SAFETY: caller promises `len` writable values and `required <= len`.
    Ok(unsafe { slice::from_raw_parts_mut(value, required) })
}

unsafe fn floats<'a>(value: *const f32, len: usize) -> Result<&'a [f32], String> {
    if len == 0 {
        return Ok(&[]);
    }
    if value.is_null() {
        return Err("vector pointer is null".to_owned());
    }
    // SAFETY: the caller promises `len` readable f32 values.
    Ok(unsafe { slice::from_raw_parts(value, len) })
}

unsafe fn values<'a, T>(value: *const T, len: usize, name: &str) -> Result<&'a [T], String> {
    if len == 0 {
        return Ok(&[]);
    }
    if value.is_null() {
        return Err(format!("{name} pointer is null"));
    }
    // SAFETY: the caller promises `len` readable values.
    Ok(unsafe { slice::from_raw_parts(value, len) })
}

unsafe fn path(value: *const c_char) -> Result<String, String> {
    if value.is_null() {
        return Err("path pointer is null".to_owned());
    }
    // SAFETY: the caller promises a readable NUL-terminated string.
    unsafe { CStr::from_ptr(value) }
        .to_str()
        .map(str::to_owned)
        .map_err(|error| format!("path is not UTF-8: {error}"))
}

fn collection_config(
    dimension: usize,
    backend: u32,
    _gpu_filter_mode: u32,
    gpu_allocation_budget_bytes: u64,
) -> Result<CollectionConfig, String> {
    let backend = match backend {
        0 => qenlo::BackendSelection::CpuExact,
        #[cfg(feature = "gpu-wgpu")]
        1 => BackendSelection::Automatic(gpu_filter_mode_value(_gpu_filter_mode)?),
        #[cfg(feature = "gpu-wgpu")]
        2 => BackendSelection::WgpuRequired(gpu_filter_mode_value(_gpu_filter_mode)?),
        #[cfg(not(feature = "gpu-wgpu"))]
        1 | 2 => {
            return Err(
                "native artifact was built without portable GPU support; use backend 0 or install a desktop GPU artifact"
                    .into(),
            );
        }
        value => {
            return Err(format!(
                "unknown backend value {value}; expected 0, 1, or 2"
            ));
        }
    };
    Ok(CollectionConfig {
        dimension,
        backend,
        gpu_allocation_budget_bytes,
    })
}

#[cfg(feature = "gpu-wgpu")]
fn gpu_filter_mode_value(value: u32) -> Result<GpuFilterMode, String> {
    match value {
        0 => Ok(GpuFilterMode::CpuMask),
        1 => Ok(GpuFilterMode::CpuEligibleRows),
        2 => Ok(GpuFilterMode::GpuPredicate),
        value => Err(format!(
            "unknown GPU filter mode {value}; expected 0, 1, or 2"
        )),
    }
}

fn owned(collection: Collection, dimension: usize) -> QenloCollection {
    QenloCollection {
        collection,
        dimension,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn qenlo_collection_new(dimension: usize) -> *mut QenloCollection {
    ffi_pointer(|| {
        let collection =
            pollster::block_on(Collection::new(CollectionConfig::cpu_exact(dimension)))
                .map_err(|error| error.to_string())?;
        Ok(QenloCollection {
            collection,
            dimension,
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn qenlo_collection_new_configured(
    dimension: usize,
    backend: u32,
    gpu_filter_mode: u32,
    gpu_allocation_budget_bytes: u64,
) -> *mut QenloCollection {
    ffi_pointer(|| {
        let config = collection_config(
            dimension,
            backend,
            gpu_filter_mode,
            gpu_allocation_budget_bytes,
        )?;
        pollster::block_on(Collection::new(config))
            .map(|collection| owned(collection, dimension))
            .map_err(|error| error.to_string())
    })
}

#[unsafe(no_mangle)]
/// Create a durable collection at a UTF-8 filesystem path.
///
/// # Safety
/// `path_ptr` must point to a readable NUL-terminated string for this call.
pub unsafe extern "C" fn qenlo_collection_create(
    path_ptr: *const c_char,
    dimension: usize,
) -> *mut QenloCollection {
    ffi_pointer(|| {
        // SAFETY: forwarded caller contract.
        let path = unsafe { path(path_ptr) }?;
        let collection = pollster::block_on(Collection::create(
            path,
            CollectionConfig::cpu_exact(dimension),
        ))
        .map_err(|error| error.to_string())?;
        Ok(QenloCollection {
            collection,
            dimension,
        })
    })
}

#[unsafe(no_mangle)]
/// Create a durable configured collection at a UTF-8 filesystem path.
///
/// # Safety
/// `path_ptr` must point to a readable NUL-terminated string for this call.
pub unsafe extern "C" fn qenlo_collection_create_configured(
    path_ptr: *const c_char,
    dimension: usize,
    backend: u32,
    gpu_filter_mode: u32,
    gpu_allocation_budget_bytes: u64,
) -> *mut QenloCollection {
    ffi_pointer(|| {
        // SAFETY: forwarded caller contract.
        let path = unsafe { path(path_ptr) }?;
        let config = collection_config(
            dimension,
            backend,
            gpu_filter_mode,
            gpu_allocation_budget_bytes,
        )?;
        pollster::block_on(Collection::create(path, config))
            .map(|collection| owned(collection, dimension))
            .map_err(|error| error.to_string())
    })
}

#[unsafe(no_mangle)]
/// Open a durable collection at a UTF-8 filesystem path.
///
/// # Safety
/// `path_ptr` must point to a readable NUL-terminated string for this call.
pub unsafe extern "C" fn qenlo_collection_open(
    path_ptr: *const c_char,
    dimension: usize,
) -> *mut QenloCollection {
    ffi_pointer(|| {
        // SAFETY: forwarded caller contract.
        let path = unsafe { path(path_ptr) }?;
        let collection = pollster::block_on(Collection::open(
            path,
            CollectionConfig::cpu_exact(dimension),
        ))
        .map_err(|error| error.to_string())?;
        Ok(QenloCollection {
            collection,
            dimension,
        })
    })
}

#[unsafe(no_mangle)]
/// Open a durable configured collection at a UTF-8 filesystem path.
///
/// # Safety
/// `path_ptr` must point to a readable NUL-terminated string for this call.
pub unsafe extern "C" fn qenlo_collection_open_configured(
    path_ptr: *const c_char,
    dimension: usize,
    backend: u32,
    gpu_filter_mode: u32,
    gpu_allocation_budget_bytes: u64,
) -> *mut QenloCollection {
    ffi_pointer(|| {
        // SAFETY: forwarded caller contract.
        let path = unsafe { path(path_ptr) }?;
        let config = collection_config(
            dimension,
            backend,
            gpu_filter_mode,
            gpu_allocation_budget_bytes,
        )?;
        pollster::block_on(Collection::open(path, config))
            .map(|collection| owned(collection, dimension))
            .map_err(|error| error.to_string())
    })
}

#[unsafe(no_mangle)]
/// Import a portable `.qn` file into a mutable in-memory collection.
///
/// # Safety
/// `path_ptr` must point to a readable NUL-terminated string for this call.
pub unsafe extern "C" fn qenlo_collection_import_qn(
    path_ptr: *const c_char,
    dimension: usize,
) -> *mut QenloCollection {
    ffi_pointer(|| {
        // SAFETY: forwarded caller contract.
        let path = unsafe { path(path_ptr) }?;
        let collection = pollster::block_on(Collection::import_qn(
            path,
            CollectionConfig::cpu_exact(dimension),
        ))
        .map_err(|error| error.to_string())?;
        Ok(QenloCollection {
            collection,
            dimension,
        })
    })
}

#[unsafe(no_mangle)]
/// Import a portable `.qn` file into a configured mutable collection.
///
/// # Safety
/// `path_ptr` must point to a readable NUL-terminated string for this call.
pub unsafe extern "C" fn qenlo_collection_import_qn_configured(
    path_ptr: *const c_char,
    dimension: usize,
    backend: u32,
    gpu_filter_mode: u32,
    gpu_allocation_budget_bytes: u64,
) -> *mut QenloCollection {
    ffi_pointer(|| {
        // SAFETY: forwarded caller contract.
        let path = unsafe { path(path_ptr) }?;
        let config = collection_config(
            dimension,
            backend,
            gpu_filter_mode,
            gpu_allocation_budget_bytes,
        )?;
        pollster::block_on(Collection::import_qn(path, config))
            .map(|collection| owned(collection, dimension))
            .map_err(|error| error.to_string())
    })
}

#[unsafe(no_mangle)]
/// Add one vector and its metadata.
///
/// # Safety
/// `handle` must be a live Qenlo handle and `vector` must reference
/// `vector_len` readable `f32` values for this call.
pub unsafe extern "C" fn qenlo_add(
    handle: *mut QenloCollection,
    id: u64,
    user_id: u64,
    timestamp: i64,
    vector: *const f32,
    vector_len: usize,
) -> i32 {
    ffi_status(|| {
        // SAFETY: forwarded caller contracts.
        let collection = unsafe { collection(handle) }.map_err(qenlo::Error::Storage)?;
        let vector = unsafe { floats(vector, vector_len) }.map_err(qenlo::Error::Storage)?;
        collection.collection.add(id, user_id, timestamp, vector)
    })
}

#[unsafe(no_mangle)]
/// Add a batch of row-major vectors and parallel metadata arrays.
///
/// # Safety
/// `handle` must be live. `ids`, `user_ids`, and `timestamps` must each
/// reference `rows` readable values; `vectors` must reference
/// `rows * dimension` readable `f32` values for this call.
pub unsafe extern "C" fn qenlo_add_batch(
    handle: *mut QenloCollection,
    ids: *const u64,
    user_ids: *const u64,
    timestamps: *const i64,
    vectors: *const f32,
    rows: usize,
    dimension: usize,
) -> i32 {
    ffi_status(|| {
        // SAFETY: forwarded caller contracts.
        let handle = unsafe { collection(handle) }.map_err(qenlo::Error::Storage)?;
        if dimension != handle.dimension {
            return Err(qenlo::Error::Storage(format!(
                "dimension mismatch: expected {}, got {dimension}",
                handle.dimension
            )));
        }
        let vector_len = rows
            .checked_mul(dimension)
            .ok_or_else(|| qenlo::Error::Storage("batch shape overflow".to_owned()))?;
        let ids = unsafe { values(ids, rows, "ids") }.map_err(qenlo::Error::Storage)?;
        let user_ids =
            unsafe { values(user_ids, rows, "user_ids") }.map_err(qenlo::Error::Storage)?;
        let timestamps =
            unsafe { values(timestamps, rows, "timestamps") }.map_err(qenlo::Error::Storage)?;
        let vectors = unsafe { floats(vectors, vector_len) }.map_err(qenlo::Error::Storage)?;
        let records = (0..rows)
            .map(|row| NewRecord {
                id: ids[row],
                user_id: user_ids[row],
                timestamp: timestamps[row],
                vector: vectors[row * dimension..(row + 1) * dimension].to_vec(),
            })
            .collect::<Vec<_>>();
        handle.collection.add_batch(&records).map(|_| ())
    })
}

#[unsafe(no_mangle)]
/// Delete a vector by public ID.
///
/// # Safety
/// `handle` must be a live Qenlo handle for this call.
pub unsafe extern "C" fn qenlo_delete(handle: *mut QenloCollection, id: u64) -> i32 {
    ffi_status(|| {
        // SAFETY: forwarded caller contract.
        unsafe { collection(handle) }
            .map_err(qenlo::Error::Storage)?
            .collection
            .delete(id)
    })
}

#[unsafe(no_mangle)]
/// Delete a batch of vectors by public ID.
///
/// # Safety
/// `handle` must be live and `ids` must reference `rows` readable `u64`
/// values for this call.
pub unsafe extern "C" fn qenlo_delete_batch(
    handle: *mut QenloCollection,
    ids: *const u64,
    rows: usize,
) -> i32 {
    ffi_status(|| {
        // SAFETY: forwarded caller contracts.
        let handle = unsafe { collection(handle) }.map_err(qenlo::Error::Storage)?;
        let ids = unsafe { values(ids, rows, "ids") }.map_err(qenlo::Error::Storage)?;
        handle.collection.delete_batch(ids).map(|_| ())
    })
}

#[derive(Clone, Copy)]
struct SearchRequest {
    handle: *mut QenloCollection,
    query: *const f32,
    query_len: usize,
    has_user_id: bool,
    user_id: u64,
    has_lower: bool,
    lower: i64,
    has_upper: bool,
    upper: i64,
    k: usize,
}

unsafe fn execute_search(request: SearchRequest) -> Result<QenloSearchResults, String> {
    // SAFETY: forwarded caller contracts.
    let handle = unsafe { collection(request.handle) }?;
    let query = unsafe { floats(request.query, request.query_len) }?;
    let filter = Filter::new(
        request.has_user_id.then_some(request.user_id),
        TimestampRange::new(
            request.has_lower.then_some(request.lower),
            request.has_upper.then_some(request.upper),
        ),
    );
    let response = pollster::block_on(handle.collection.search(query, &filter, request.k))
        .map_err(|error| error.to_string())?;
    let mut ids = Vec::with_capacity(response.results.len());
    let mut distances = Vec::with_capacity(response.results.len());
    for hit in response.results {
        ids.push(hit.id);
        distances.push(hit.distance);
    }
    Ok(QenloSearchResults {
        ids,
        distances,
        report: JsonReport {
            operation_id: response.report.operation_id.to_string(),
            requested_backend: format!("{:?}", response.report.requested_backend),
            actual_backend: format!("{:?}", response.report.actual_backend),
            algorithm: format!("{:?}", response.report.algorithm),
            filter_execution: format!("{:?}", response.report.filter_execution),
            index_generation: response.report.index_generation.to_string(),
            rebuilt: response.report.rebuilt,
            routing_reason: response.report.routing_reason,
            fallback_reason: response.report.fallback_reason,
            total_duration_ns: duration_ns(response.report.total_duration),
            lock_wait_ns: duration_ns(response.report.lock_wait),
            eligible_rows: measured(&response.report.eligible_rows).map(|v| v.to_string()),
            upload_bytes: measured(&response.report.upload_bytes).map(|v| v.to_string()),
            readback_bytes: measured(&response.report.readback_bytes).map(|v| v.to_string()),
            allocation_bytes: measured(&response.report.qenlo_allocation_bytes)
                .map(|v| v.to_string()),
            dispatch_count: measured(&response.report.dispatch_count),
            candidates: measured(&response.report.candidates).map(|v| v.to_string()),
            batch_size: response.report.batch_size,
        },
    })
}

#[unsafe(no_mangle)]
/// Search and return an owned UTF-8 JSON response.
///
/// # Safety
/// `handle` must be live and `query` must reference `query_len` readable
/// `f32` values for this call. Free a non-null result with
/// [`qenlo_string_free`].
pub unsafe extern "C" fn qenlo_search(
    handle: *mut QenloCollection,
    query: *const f32,
    query_len: usize,
    has_user_id: bool,
    user_id: u64,
    has_lower: bool,
    lower: i64,
    has_upper: bool,
    upper: i64,
    k: usize,
) -> *mut c_char {
    json_pointer(|| {
        // SAFETY: forwarded caller contracts.
        let response = unsafe {
            execute_search(SearchRequest {
                handle,
                query,
                query_len,
                has_user_id,
                user_id,
                has_lower,
                lower,
                has_upper,
                upper,
                k,
            })
        }?;
        Ok(JsonSearch {
            results: response
                .ids
                .into_iter()
                .zip(response.distances)
                .map(|(id, distance)| JsonHit {
                    id: id.to_string(),
                    distance,
                })
                .collect(),
            report: response.report,
        })
    })
}

#[unsafe(no_mangle)]
/// Search once and return typed result buffers plus the matching report.
///
/// # Safety
/// `handle` must be live and `query` must reference `query_len` readable
/// `f32` values. Free a non-null result with [`qenlo_search_results_free`].
pub unsafe extern "C" fn qenlo_search_results_new(
    handle: *mut QenloCollection,
    query: *const f32,
    query_len: usize,
    has_user_id: bool,
    user_id: u64,
    has_lower: bool,
    lower: i64,
    has_upper: bool,
    upper: i64,
    k: usize,
) -> *mut QenloSearchResults {
    ffi_search_pointer(|| {
        // SAFETY: forwarded caller contracts.
        unsafe {
            execute_search(SearchRequest {
                handle,
                query,
                query_len,
                has_user_id,
                user_id,
                has_lower,
                lower,
                has_upper,
                upper,
                k,
            })
        }
    })
}

#[unsafe(no_mangle)]
/// Return the number of typed hits.
///
/// # Safety
/// `results` must be live and `rows` must be writable.
pub unsafe extern "C" fn qenlo_search_results_len(
    results: *mut QenloSearchResults,
    rows: *mut usize,
) -> i32 {
    ffi_status(|| {
        // SAFETY: forwarded caller contracts.
        let results = unsafe { search_results(results) }.map_err(qenlo::Error::Storage)?;
        let rows = unsafe { rows.as_mut() }
            .ok_or_else(|| qenlo::Error::Storage("rows output pointer is null".into()))?;
        *rows = results.ids.len();
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// Copy IDs and distances from one completed typed search.
///
/// # Safety
/// `results` must be live. Output pointers must each describe the supplied
/// writable lengths.
pub unsafe extern "C" fn qenlo_search_results_copy(
    results: *mut QenloSearchResults,
    ids: *mut u64,
    ids_len: usize,
    distances: *mut f32,
    distances_len: usize,
) -> i32 {
    ffi_status(|| {
        // SAFETY: forwarded caller contracts.
        let results = unsafe { search_results(results) }.map_err(qenlo::Error::Storage)?;
        let ids = unsafe { output_values(ids, ids_len, results.ids.len(), "ids") }
            .map_err(qenlo::Error::Storage)?;
        let distances = unsafe {
            output_values(
                distances,
                distances_len,
                results.distances.len(),
                "distances",
            )
        }
        .map_err(qenlo::Error::Storage)?;
        ids.copy_from_slice(&results.ids);
        distances.copy_from_slice(&results.distances);
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// Return report JSON for the same completed typed search.
///
/// # Safety
/// `results` must be live. Free a non-null result with [`qenlo_string_free`].
pub unsafe extern "C" fn qenlo_search_results_report_json(
    results: *mut QenloSearchResults,
) -> *mut c_char {
    json_pointer(|| {
        // SAFETY: forwarded caller contract.
        Ok(unsafe { search_results(results) }?.report.clone())
    })
}

#[unsafe(no_mangle)]
/// Free typed search results.
///
/// # Safety
/// `results` must be null or a live allocation returned by
/// [`qenlo_search_results_new`], transferred exactly once.
pub unsafe extern "C" fn qenlo_search_results_free(results: *mut QenloSearchResults) {
    if !results.is_null() {
        // SAFETY: caller transfers a live allocation returned by this library exactly once.
        drop(unsafe { Box::from_raw(results) });
    }
}

#[unsafe(no_mangle)]
/// Capture live rows matching a canonical filter in one generation.
///
/// # Safety
/// `handle` must be live. Free a non-null result with [`qenlo_snapshot_free`].
pub unsafe extern "C" fn qenlo_snapshot_new(
    handle: *mut QenloCollection,
    has_user_id: bool,
    user_id: u64,
    has_lower: bool,
    lower: i64,
    has_upper: bool,
    upper: i64,
) -> *mut QenloSnapshot {
    ffi_snapshot_pointer(|| {
        // SAFETY: forwarded caller contract.
        let handle = unsafe { collection(handle) }?;
        let filter = Filter::new(
            has_user_id.then_some(user_id),
            TimestampRange::new(has_lower.then_some(lower), has_upper.then_some(upper)),
        );
        let captured = handle
            .collection
            .canonical_snapshot(&filter)
            .map_err(|error| error.to_string())?;
        let rows = captured.records.len();
        let mut ids = Vec::with_capacity(rows);
        let mut vectors = Vec::with_capacity(rows.saturating_mul(captured.dimension));
        for record in captured.records {
            ids.push(record.id());
            vectors.extend_from_slice(record.vector());
        }
        Ok(QenloSnapshot {
            generation: captured.generation,
            rows,
            dimension: captured.dimension,
            ids,
            vectors,
        })
    })
}

#[unsafe(no_mangle)]
/// Read typed snapshot metadata into caller-owned scalar outputs.
///
/// # Safety
/// All pointers must be live and writable for this call.
pub unsafe extern "C" fn qenlo_snapshot_info(
    value: *mut QenloSnapshot,
    generation: *mut u64,
    rows: *mut usize,
    dimension: *mut usize,
) -> i32 {
    ffi_status(|| {
        // SAFETY: forwarded caller contracts.
        let value = unsafe { snapshot(value) }.map_err(qenlo::Error::Storage)?;
        let generation = unsafe { generation.as_mut() }
            .ok_or_else(|| qenlo::Error::Storage("generation output pointer is null".into()))?;
        let rows = unsafe { rows.as_mut() }
            .ok_or_else(|| qenlo::Error::Storage("rows output pointer is null".into()))?;
        let dimension = unsafe { dimension.as_mut() }
            .ok_or_else(|| qenlo::Error::Storage("dimension output pointer is null".into()))?;
        *generation = value.generation;
        *rows = value.rows;
        *dimension = value.dimension;
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// Copy snapshot IDs and row-major normalized vectors into caller buffers.
///
/// # Safety
/// `value` must be live. Output pointers must describe writable buffers of the
/// supplied lengths and must not overlap the snapshot allocation.
pub unsafe extern "C" fn qenlo_snapshot_copy(
    value: *mut QenloSnapshot,
    ids: *mut u64,
    ids_len: usize,
    vectors: *mut f32,
    vectors_len: usize,
) -> i32 {
    ffi_status(|| {
        // SAFETY: forwarded caller contracts.
        let value = unsafe { snapshot(value) }.map_err(qenlo::Error::Storage)?;
        let ids = unsafe { output_values(ids, ids_len, value.ids.len(), "ids") }
            .map_err(qenlo::Error::Storage)?;
        let vectors =
            unsafe { output_values(vectors, vectors_len, value.vectors.len(), "vectors") }
                .map_err(qenlo::Error::Storage)?;
        ids.copy_from_slice(&value.ids);
        vectors.copy_from_slice(&value.vectors);
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// Read the current canonical generation without JSON conversion.
///
/// # Safety
/// `handle` must be live and `generation` must be writable for this call.
pub unsafe extern "C" fn qenlo_collection_generation(
    handle: *mut QenloCollection,
    generation: *mut u64,
) -> i32 {
    ffi_status(|| {
        // SAFETY: forwarded caller contracts.
        let handle = unsafe { collection(handle) }.map_err(qenlo::Error::Storage)?;
        let generation = unsafe { generation.as_mut() }
            .ok_or_else(|| qenlo::Error::Storage("generation output pointer is null".into()))?;
        let stats = handle.collection.stats();
        if stats.closed {
            return Err(qenlo::Error::Closed);
        }
        *generation = stats.generation;
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// Free a generation-bound snapshot handle.
///
/// # Safety
/// `value` must be null or a live allocation returned by [`qenlo_snapshot_new`],
/// transferred exactly once and never used afterward.
pub unsafe extern "C" fn qenlo_snapshot_free(value: *mut QenloSnapshot) {
    if !value.is_null() {
        // SAFETY: caller transfers a live allocation returned by this library exactly once.
        drop(unsafe { Box::from_raw(value) });
    }
}

#[unsafe(no_mangle)]
/// Return collection statistics as an owned UTF-8 JSON response.
///
/// # Safety
/// `handle` must be a live Qenlo handle for this call. Free a non-null result
/// with [`qenlo_string_free`].
pub unsafe extern "C" fn qenlo_stats(handle: *mut QenloCollection) -> *mut c_char {
    json_pointer(|| {
        // SAFETY: forwarded caller contract.
        let stats = unsafe { collection(handle) }?.collection.stats();
        Ok(JsonStats {
            dimension: stats.dimension,
            rows: stats.rows,
            live_rows: stats.live_rows,
            generation: stats.generation.to_string(),
            prepared_generation: stats.prepared_generation.map(|v| v.to_string()),
            durable_generation: stats.durable_generation.map(|v| v.to_string()),
            recovered_interrupted_write: stats.recovered_interrupted_write,
            closed: stats.closed,
        })
    })
}

#[unsafe(no_mangle)]
/// Export the current canonical generation to a new portable `.qn` file.
///
/// # Safety
/// `handle` must be live and `path_ptr` must point to a readable
/// NUL-terminated string for this call.
pub unsafe extern "C" fn qenlo_export_qn(
    handle: *mut QenloCollection,
    path_ptr: *const c_char,
) -> i32 {
    ffi_status(|| {
        // SAFETY: forwarded caller contracts.
        let handle = unsafe { collection(handle) }.map_err(qenlo::Error::Storage)?;
        let path = unsafe { path(path_ptr) }.map_err(qenlo::Error::Storage)?;
        handle.collection.export_qn(path)
    })
}

#[unsafe(no_mangle)]
/// Flush a durable collection.
///
/// # Safety
/// `handle` must be a live Qenlo handle for this call.
pub unsafe extern "C" fn qenlo_flush(handle: *mut QenloCollection) -> i32 {
    ffi_status(|| {
        // SAFETY: forwarded caller contract.
        unsafe { collection(handle) }
            .map_err(qenlo::Error::Storage)?
            .collection
            .flush()
    })
}

#[unsafe(no_mangle)]
/// Close a collection without freeing its handle.
///
/// # Safety
/// `handle` must be a live Qenlo handle for this call.
pub unsafe extern "C" fn qenlo_close(handle: *mut QenloCollection) -> i32 {
    ffi_status(|| {
        // SAFETY: forwarded caller contract.
        unsafe { collection(handle) }
            .map_err(qenlo::Error::Storage)?
            .collection
            .close()
    })
}

#[unsafe(no_mangle)]
/// Close and free a collection handle.
///
/// # Safety
/// `handle` must be null or a live allocation returned by this library. A
/// non-null handle must be transferred exactly once and never used afterward.
pub unsafe extern "C" fn qenlo_collection_free(handle: *mut QenloCollection) {
    if !handle.is_null() {
        // SAFETY: caller transfers a live allocation returned by this library exactly once.
        let handle = unsafe { Box::from_raw(handle) };
        let _ = handle.collection.close();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn qenlo_last_error() -> *mut c_char {
    let message = LAST_ERROR.with(|value| value.borrow().clone());
    CString::new(message)
        .expect("stored error contains no NUL")
        .into_raw()
}

#[unsafe(no_mangle)]
/// Free a string returned by this library.
///
/// # Safety
/// `value` must be null or an allocation returned by a Qenlo string-returning
/// function. A non-null pointer must be transferred exactly once.
pub unsafe extern "C" fn qenlo_string_free(value: *mut c_char) {
    if !value.is_null() {
        // SAFETY: caller transfers an allocation returned by this library exactly once.
        drop(unsafe { CString::from_raw(value) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe fn take_string(value: *mut c_char) -> String {
        assert!(!value.is_null());
        // SAFETY: test owns the returned allocation until it is freed below.
        let output = unsafe { CStr::from_ptr(value) }
            .to_str()
            .unwrap()
            .to_owned();
        // SAFETY: value was returned by this library and is freed once.
        unsafe { qenlo_string_free(value) };
        output
    }

    #[test]
    fn configured_constructor_preserves_cpu_and_rejects_unsupported_values() {
        let cpu = qenlo_collection_new_configured(2, 0, 2, 1024);
        assert!(!cpu.is_null());
        // SAFETY: the test transfers this live handle once.
        unsafe { qenlo_collection_free(cpu) };

        let invalid = qenlo_collection_new_configured(2, 99, 2, 1024);
        assert!(invalid.is_null());
        assert!(unsafe { take_string(qenlo_last_error()) }.contains("unknown backend"));

        #[cfg(not(feature = "gpu-wgpu"))]
        {
            let gpu = qenlo_collection_new_configured(2, 2, 2, 1024);
            assert!(gpu.is_null());
            assert!(unsafe { take_string(qenlo_last_error()) }.contains("without portable GPU"));
        }
        #[cfg(feature = "gpu-wgpu")]
        {
            let automatic = qenlo_collection_new_configured(2, 1, 2, 1024);
            assert!(!automatic.is_null());
            // SAFETY: the test transfers this live handle once.
            unsafe { qenlo_collection_free(automatic) };
        }
    }

    #[test]
    fn typed_search_buffers_share_one_completed_report() {
        let handle = qenlo_collection_new(2);
        let ids = [9, 2];
        let users = [7, 7];
        let timestamps = [0, 0];
        let vectors = [1.0, 0.0, 1.0, 0.0];
        // SAFETY: arrays and handles remain live for each call and owned handles are freed once.
        unsafe {
            assert_eq!(
                qenlo_add_batch(
                    handle,
                    ids.as_ptr(),
                    users.as_ptr(),
                    timestamps.as_ptr(),
                    vectors.as_ptr(),
                    2,
                    2,
                ),
                0
            );
            let results = qenlo_search_results_new(
                handle,
                vectors.as_ptr(),
                2,
                false,
                0,
                false,
                0,
                false,
                0,
                2,
            );
            assert!(!results.is_null());
            let mut rows = 0;
            assert_eq!(qenlo_search_results_len(results, &mut rows), 0);
            assert_eq!(rows, 2);
            let mut copied_ids = [0; 2];
            let mut copied_distances = [0.0; 2];
            assert_eq!(
                qenlo_search_results_copy(
                    results,
                    copied_ids.as_mut_ptr(),
                    copied_ids.len(),
                    copied_distances.as_mut_ptr(),
                    copied_distances.len(),
                ),
                0
            );
            assert_eq!(copied_ids, [2, 9]);
            assert_eq!(copied_distances, [0.0, 0.0]);
            let report: serde_json::Value =
                serde_json::from_str(&take_string(qenlo_search_results_report_json(results)))
                    .unwrap();
            assert_eq!(report["index_generation"], "2");
            assert_eq!(report["batch_size"], 1);
            qenlo_search_results_free(results);
            qenlo_collection_free(handle);
        }
    }

    #[test]
    fn native_boundary_preserves_semantics_and_reports_execution() {
        let handle = qenlo_collection_new(2);
        assert!(!handle.is_null());
        let ids = [9, 2];
        let users = [7, 7];
        let timestamps = [-1, 4];
        let vectors = [1.0, 0.0, 1.0, 0.0];
        // SAFETY: all pointers describe the arrays above for the duration of each call.
        unsafe {
            assert_eq!(
                qenlo_add_batch(
                    handle,
                    ids.as_ptr(),
                    users.as_ptr(),
                    timestamps.as_ptr(),
                    vectors.as_ptr(),
                    2,
                    2,
                ),
                0
            );
            let output = take_string(qenlo_search(
                handle,
                vectors.as_ptr(),
                2,
                true,
                7,
                true,
                -2,
                true,
                5,
                10,
            ));
            let value: serde_json::Value = serde_json::from_str(&output).unwrap();
            assert_eq!(value["results"][0]["id"], "2");
            assert_eq!(value["results"][1]["id"], "9");
            assert_eq!(value["report"]["actual_backend"], "Cpu");
            assert_eq!(qenlo_delete_batch(handle, ids.as_ptr(), 2), 0);
            let stats: serde_json::Value =
                serde_json::from_str(&take_string(qenlo_stats(handle))).unwrap();
            assert_eq!(stats["live_rows"], 0);
            qenlo_collection_free(handle);
        }
    }

    #[test]
    fn native_boundary_round_trips_portable_qn() {
        let root = std::env::temp_dir().join(format!(
            "qenlo-ffi-qn-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("vectors.qn");
        let file = CString::new(file.to_string_lossy().as_bytes()).unwrap();
        let handle = qenlo_collection_new(2);
        assert!(!handle.is_null());
        let vector = [1.0, 0.0];
        // SAFETY: handles and pointers are live for every call and are transferred once.
        unsafe {
            assert_eq!(qenlo_add(handle, 4, 9, -2, vector.as_ptr(), 2), 0);
            assert_eq!(qenlo_export_qn(handle, file.as_ptr()), 0);
            qenlo_collection_free(handle);

            let imported = qenlo_collection_import_qn(file.as_ptr(), 2);
            assert!(!imported.is_null());
            let stats: serde_json::Value =
                serde_json::from_str(&take_string(qenlo_stats(imported))).unwrap();
            assert_eq!(stats["rows"], 1);
            assert_eq!(stats["live_rows"], 1);
            qenlo_collection_free(imported);
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn typed_snapshot_is_filtered_generation_bound_and_bulk_copied() {
        let handle = qenlo_collection_new(2);
        let ids = [u64::MAX, 7, 3];
        let users = [4, 4, 9];
        let timestamps = [-2, 8, 1];
        let vectors = [1.0, 0.0, 0.0, 1.0, -1.0, 0.0];
        // SAFETY: all pointers describe live arrays and every owned handle is freed once.
        unsafe {
            assert_eq!(
                qenlo_add_batch(
                    handle,
                    ids.as_ptr(),
                    users.as_ptr(),
                    timestamps.as_ptr(),
                    vectors.as_ptr(),
                    3,
                    2,
                ),
                0
            );
            let captured = qenlo_snapshot_new(handle, true, 4, true, -3, true, 9);
            assert!(!captured.is_null());
            let (mut generation, mut rows, mut dimension) = (0, 0, 0);
            assert_eq!(
                qenlo_snapshot_info(captured, &mut generation, &mut rows, &mut dimension),
                0
            );
            assert_eq!((generation, rows, dimension), (3, 2, 2));
            let mut copied_ids = [0; 2];
            let mut copied_vectors = [0.0; 4];
            assert_eq!(
                qenlo_snapshot_copy(
                    captured,
                    copied_ids.as_mut_ptr(),
                    copied_ids.len(),
                    copied_vectors.as_mut_ptr(),
                    copied_vectors.len(),
                ),
                0
            );
            assert_eq!(copied_ids, [u64::MAX, 7]);
            assert_eq!(copied_vectors, [1.0, 0.0, 0.0, 1.0]);

            assert_eq!(qenlo_delete(handle, u64::MAX), 0);
            let mut current = 0;
            assert_eq!(qenlo_collection_generation(handle, &mut current), 0);
            assert_eq!(current, 4);
            // The owned snapshot remains the exact generation captured before mutation.
            copied_ids.fill(0);
            assert_eq!(
                qenlo_snapshot_copy(
                    captured,
                    copied_ids.as_mut_ptr(),
                    copied_ids.len(),
                    copied_vectors.as_mut_ptr(),
                    copied_vectors.len(),
                ),
                0
            );
            assert_eq!(copied_ids, [u64::MAX, 7]);
            qenlo_snapshot_free(captured);
            qenlo_collection_free(handle);
        }
    }

    #[test]
    fn native_boundary_rejects_bad_pointers_without_unwinding() {
        // SAFETY: deliberately exercising null validation; no pointer is dereferenced.
        unsafe {
            assert_eq!(qenlo_add(ptr::null_mut(), 1, 1, 0, ptr::null(), 2), -1);
            assert!(take_string(qenlo_last_error()).contains("collection pointer is null"));
        }
    }
}
