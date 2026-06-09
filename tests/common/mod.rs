//! Hermetic temp-repo fixture shared by the integration tests.

use std::fs;
use std::path::{Path, PathBuf};

/// A throwaway repo on disk, removed on drop.
pub struct TempRepo {
    pub root: PathBuf,
}

impl TempRepo {
    /// Build a minimal python repo: pyproject + README + pkg/{target,other}.py
    pub fn python() -> Self {
        let root = unique_dir("ctxrt_py");
        write(&root.join("pyproject.toml"), "[project]\nname = \"fixture\"\n");
        write(
            &root.join("README.md"),
            "# Fixture Repo\n\nA hermetic fixture for context-runtime tests.\n",
        );
        write(
            &root.join("pkg/target.py"),
            "def add(a, b):\n    return a + b   \n",
        );
        write(
            &root.join("pkg/other.py"),
            "def sub(a, b):\n    return a - b\n",
        );
        TempRepo { root }
    }

    #[allow(dead_code)] // used by the assemble test crate, not the http test crate
    pub fn target_rel(&self) -> &str {
        "pkg/target.py"
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn unique_dir(prefix: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("{prefix}_{}_{nanos}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temp repo");
    dir
}

fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, content).expect("write fixture file");
}
