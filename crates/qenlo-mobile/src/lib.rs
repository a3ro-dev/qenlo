use std::{
    ffi::{CStr, CString, c_char},
    panic::catch_unwind,
};

fn run(profile: &str) -> String {
    match qenlo_testkit::run_profile(profile) {
        Ok(report) => {
            serde_json::to_string(&report).unwrap_or_else(|error| error_json(&error.to_string()))
        }
        Err(error) => error_json(&error),
    }
}

fn error_json(message: &str) -> String {
    serde_json::json!({ "bridge_error": message }).to_string()
}

/// Returns a UTF-8 JSON allocation owned by Qenlo. Release with `qenlo_lab_free`.
///
/// # Safety
///
/// `profile` must be null or point to a readable NUL-terminated string for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qenlo_lab_run(profile: *const c_char) -> *mut c_char {
    let output = catch_unwind(|| {
        if profile.is_null() {
            return error_json("profile pointer is null");
        }
        // SAFETY: the caller contract requires a valid, NUL-terminated UTF-8 string.
        let profile = unsafe { CStr::from_ptr(profile) };
        match profile.to_str() {
            Ok(profile) => run(profile),
            Err(_) => error_json("profile is not UTF-8"),
        }
    })
    .unwrap_or_else(|_| error_json("native runner panicked"));
    CString::new(output)
        .expect("JSON contains no NUL")
        .into_raw()
}

#[unsafe(no_mangle)]
/// Release a non-null pointer returned by `qenlo_lab_run` exactly once.
///
/// # Safety
///
/// `value` must be null or an allocation returned by `qenlo_lab_run` that has not been freed.
pub unsafe extern "C" fn qenlo_lab_free(value: *mut c_char) {
    if !value.is_null() {
        // SAFETY: the caller must pass a pointer returned by qenlo_lab_run exactly once.
        drop(unsafe { CString::from_raw(value) });
    }
}

#[cfg(target_os = "android")]
mod android {
    use jni::{
        JNIEnv,
        objects::{JClass, JString},
        sys::jstring,
    };

    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_qenlo_lab_NativeLab_run(
        mut env: JNIEnv,
        _class: JClass,
        profile: JString,
    ) -> jstring {
        let output = env
            .get_string(&profile)
            .map(|value| super::run(&value.to_string_lossy()))
            .unwrap_or_else(|error| super::error_json(&error.to_string()));
        env.new_string(output)
            .map(|value| value.into_raw())
            .unwrap_or(std::ptr::null_mut())
    }
}
