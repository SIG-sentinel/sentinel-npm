use super::*;
use std::fs;
use std::sync::{Mutex, OnceLock};

fn artifact_test_lock() -> std::sync::MutexGuard<'static, ()> {
    static TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("artifact test mutex should lock")
}

#[test]
fn test_register_artifact() {
    let _guard = artifact_test_lock();

    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let artifact_path = temp_dir.path().join("test_artifact.tmp");
    fs::write(&artifact_path, b"test content").expect("write should succeed");

    register_artifact(artifact_path.clone());
    cleanup_all();

    assert!(!artifact_path.exists());
}

#[test]
fn test_cleanup_nonexistent_artifact() {
    let _guard = artifact_test_lock();

    let nonexistent = PathBuf::from("/tmp/nonexistent_artifact_test_12345");
    let result = cleanup_artifact(&nonexistent);
    assert!(result.is_ok());
}

#[test]
fn test_cleanup_directory() {
    let _guard = artifact_test_lock();

    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let test_dir = temp_dir.path().join("test_dir");
    fs::create_dir(&test_dir).expect("mkdir should succeed");
    fs::write(test_dir.join("file.txt"), b"test").expect("write should succeed");

    let result = cleanup_artifact(&test_dir);
    assert!(result.is_ok());
    assert!(!test_dir.exists());
}
