//! Stable, panic-contained C ABI shared by Qenlo's non-Rust SDKs.

use std::{
    cell::RefCell,
    ffi::{CStr, CString, c_char},
    panic::{AssertUnwindSafe, catch_unwind},
    ptr, slice,
};

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

#[derive(Serialize)]
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
        let handle = unsafe { collection(handle) }?;
        let query = unsafe { floats(query, query_len) }?;
        let filter = Filter::new(
            has_user_id.then_some(user_id),
            TimestampRange::new(has_lower.then_some(lower), has_upper.then_some(upper)),
        );
        let response = pollster::block_on(handle.collection.search(query, &filter, k))
            .map_err(|error| error.to_string())?;
        Ok(JsonSearch {
            results: response
                .results
                .into_iter()
                .map(|hit| JsonHit {
                    id: hit.id.to_string(),
                    distance: hit.distance,
                })
                .collect(),
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
    })
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
    fn native_boundary_preserves_semantics_and_reports_telemetry() {
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
    fn native_boundary_rejects_bad_pointers_without_unwinding() {
        // SAFETY: deliberately exercising null validation; no pointer is dereferenced.
        unsafe {
            assert_eq!(qenlo_add(ptr::null_mut(), 1, 1, 0, ptr::null(), 2), -1);
            assert!(take_string(qenlo_last_error()).contains("collection pointer is null"));
        }
    }
}
