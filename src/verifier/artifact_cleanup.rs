use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::OnceLock;

pub use crate::types::ArtifactRegistry;

const SECURE_WIPE_MAX_FILE_BYTES: usize = 100 * 1024 * 1024;
const SECURE_WIPE_CHUNK_BYTES: usize = 4096;
static ARTIFACT_REGISTRY: OnceLock<Mutex<ArtifactRegistry>> = OnceLock::new();

impl ArtifactRegistry {
    fn new() -> Self {
        Self {
            temp_artifacts: Vec::new(),
        }
    }

    pub fn register(&mut self, path: PathBuf) {
        self.temp_artifacts.push(path);
    }

    pub fn unregister(&mut self, path: &Path) {
        self.temp_artifacts.retain(|registered| registered != path);
    }

    pub fn cleanup_all(&mut self) {
        for path in self.temp_artifacts.drain(..) {
            let _ = cleanup_artifact(&path);
        }
    }
}

fn get_registry() -> &'static Mutex<ArtifactRegistry> {
    ARTIFACT_REGISTRY.get_or_init(|| Mutex::new(ArtifactRegistry::new()))
}

pub fn register_artifact(path: PathBuf) {
    let registry = get_registry().lock();

    let Ok(mut reg) = registry else {
        return;
    };

    reg.register(path);
}

pub fn unregister_artifact(path: &Path) {
    let registry = get_registry().lock();

    let Ok(mut reg) = registry else {
        return;
    };

    reg.unregister(path);
}

fn secure_wipe_file(mut file: std::fs::File, file_size: usize) {
    use std::io::Write;

    let zero_fill_chunk = vec![0u8; SECURE_WIPE_CHUNK_BYTES];
    let mut remaining = file_size;

    while remaining > 0 {
        let chunk_size = remaining.min(zero_fill_chunk.len());
        let write_result = file.write_all(&zero_fill_chunk[..chunk_size]);

        if write_result.is_err() {
            break;
        }

        remaining -= chunk_size;
    }
}

fn cleanup_file(path: &Path, file_size: usize) -> std::io::Result<()> {
    if file_size > SECURE_WIPE_MAX_FILE_BYTES {
        return std::fs::remove_file(path);
    }

    let artifact_file = std::fs::OpenOptions::new().write(true).open(path);

    let Ok(file) = artifact_file else {
        return std::fs::remove_file(path);
    };

    secure_wipe_file(file, file_size);

    std::fs::remove_file(path)
}

pub fn cleanup_artifact(path: &Path) -> std::io::Result<()> {
    if !path.exists() {
        return Ok(());
    }

    if path.is_dir() {
        std::fs::remove_dir_all(path)?;

        return Ok(());
    }

    let file_metadata = std::fs::metadata(path)?;
    let file_size = usize::try_from(file_metadata.len()).unwrap_or(usize::MAX);

    cleanup_file(path, file_size)
}

pub fn cleanup_all() {
    let registry = get_registry().lock();
    let Ok(mut reg) = registry else {
        return;
    };

    reg.cleanup_all();
}

pub fn install_cleanup_handlers() {
    let default_panic = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |panic_info| {
        cleanup_all();
        default_panic(panic_info);
    }));
}

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "../../tests/internal/artifact_cleanup_tests.rs"]
mod tests;
