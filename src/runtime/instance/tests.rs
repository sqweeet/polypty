use std::ffi::OsStr;

#[cfg(unix)]
use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use super::reject_nested;
#[cfg(unix)]
use super::InstanceGuard;

#[test]
fn nested_mux_is_rejected() {
    let err = reject_nested(Some(OsStr::new("1"))).unwrap_err();
    assert!(err.to_string().contains("inside an existing mux"));
    reject_nested(None).unwrap();
}

#[cfg(unix)]
#[test]
fn lock_rejects_a_second_instance_and_releases_on_drop() {
    let path = unique_lock_path();
    let first = InstanceGuard::acquire_at(&path).unwrap();

    let err = InstanceGuard::acquire_at(&path).unwrap_err();
    assert!(err.to_string().contains("already running"));
    assert!(err.to_string().contains(&std::process::id().to_string()));

    drop(first);
    let replacement = InstanceGuard::acquire_at(&path).unwrap();
    drop(replacement);
    fs::remove_file(path).unwrap();
}

#[cfg(unix)]
fn unique_lock_path() -> std::path::PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "mux-instance-test-{}-{nanos}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}
