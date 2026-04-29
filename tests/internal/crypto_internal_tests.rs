#![allow(clippy::expect_used, clippy::await_holding_lock)]

use std::sync::{Mutex, OnceLock};

use bytes::Bytes;
use futures_util::stream;

use super::hash_stream;
use crate::types::HashStreamParams;

fn artifact_test_lock() -> std::sync::MutexGuard<'static, ()> {
    static TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("artifact test mutex should lock")
}

#[tokio::test]
async fn hash_stream_captures_buffer_in_memory_mode() {
    let stream = stream::iter(vec![
        Ok::<Bytes, reqwest::Error>(Bytes::from_static(b"hello")),
        Ok::<Bytes, reqwest::Error>(Bytes::from_static(b"-world")),
    ]);

    let hash_stream_params = HashStreamParams {
        stream,
        package: "fixture",
        capture_buffer: true,
        spool_to_disk: false,
        inflight_counter: None,
    };

    let result = hash_stream(hash_stream_params)
        .await
        .expect("hash stream should succeed");

    assert_eq!(result.bytes, 11);
    assert_eq!(result.buffer.as_deref(), Some(&b"hello-world"[..]));
    assert!(result.spool_path.is_none());
}

#[tokio::test]
async fn hash_stream_uses_spool_without_capturing_buffer() {
    let _guard = artifact_test_lock();

    let stream = stream::iter(vec![
        Ok::<Bytes, reqwest::Error>(Bytes::from_static(b"chunk-1")),
        Ok::<Bytes, reqwest::Error>(Bytes::from_static(b"chunk-2")),
    ]);

    let hash_stream_params = HashStreamParams {
        stream,
        package: "fixture",
        capture_buffer: false,
        spool_to_disk: true,
        inflight_counter: None,
    };

    let result = hash_stream(hash_stream_params)
        .await
        .expect("hash stream with spool should succeed");

    assert_eq!(result.bytes, 14);
    assert!(result.buffer.is_none());

    let spool_path = result
        .spool_path
        .as_ref()
        .expect("spool mode should return persisted spool path");

    crate::verifier::artifact_cleanup::cleanup_artifact(spool_path)
        .expect("spool path should be removable");
    crate::verifier::artifact_cleanup::unregister_artifact(spool_path);
}

#[tokio::test]
async fn hash_stream_spool_mode_respects_max_tarball_size_limit() {
    let _guard = artifact_test_lock();
    let chunk_size = 1024 * 1024;
    let num_chunks = 51;
    let chunks = vec![Bytes::from(vec![0u8; chunk_size]); num_chunks];
    let stream = stream::iter(chunks.into_iter().map(Ok::<Bytes, reqwest::Error>));

    let hash_stream_params = HashStreamParams {
        stream,
        package: "oversized-pkg",
        capture_buffer: false,
        spool_to_disk: true,
        inflight_counter: None,
    };

    let result = hash_stream(hash_stream_params).await;

    assert!(
        matches!(
            result,
            Err(crate::types::SentinelError::TarballTooLarge { .. })
        ),
        "spool mode should reject tarballs exceeding MAX_TARBALL_BYTES"
    );
}

#[tokio::test]
async fn hash_stream_spool_cleans_up_on_size_exceeded_error() {
    let _guard = artifact_test_lock();

    let chunk_size = 1024 * 1024;
    let num_chunks = 51;

    let chunks = vec![Bytes::from(vec![0u8; chunk_size]); num_chunks];
    let stream = stream::iter(chunks.into_iter().map(Ok::<Bytes, reqwest::Error>));

    let hash_stream_params = HashStreamParams {
        stream,
        package: "cleanup-test-pkg",
        capture_buffer: false,
        spool_to_disk: true,
        inflight_counter: None,
    };

    let result = hash_stream(hash_stream_params).await;

    assert!(
        matches!(
            result,
            Err(crate::types::SentinelError::TarballTooLarge { .. })
        ),
        "should fail with TarballTooLarge error"
    );
}
