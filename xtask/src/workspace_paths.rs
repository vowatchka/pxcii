//! Locations of the workspace directories used by the tasks.

use std::path::{Path, PathBuf};

/// Root of the `pxcii` workspace, the directory that holds the workspace `Cargo.toml`.
pub(crate) fn workspace_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir.parent().unwrap_or(manifest_dir);
    root.to_path_buf()
}

/// Directory that holds the committed fixtures.
pub(crate) fn fixtures_dir() -> PathBuf {
    workspace_root().join("fixtures")
}

/// Source directory of the `pxcii-core` crate.
pub(crate) fn core_sources() -> PathBuf {
    workspace_root().join("pxcii-core").join("src")
}

#[cfg(test)]
mod tests {
    use super::{core_sources, fixtures_dir, workspace_root};

    #[test]
    fn workspace_root_holds_the_workspace_manifest() {
        assert!(workspace_root().join("Cargo.toml").is_file());
        assert!(workspace_root().join("pxcii-dev-plan.md").is_file());
    }

    #[test]
    fn directories_hang_off_the_root() {
        let root = workspace_root();
        assert_eq!(fixtures_dir(), root.join("fixtures"));
        assert_eq!(core_sources(), root.join("pxcii-core").join("src"));
    }
}
