//! Directories a grid or model file name is resolved against.
//!
//! `data/grids` holds CDN grids and is not part of the source tree.
//! `tests/gie/tests` and `data/tests` hold the vendored fixtures. A name may
//! be a path or a bare filename. A missing file is not a coordinate.

use std::path::{Path, PathBuf};

pub(crate) fn directories() -> [PathBuf; 4] {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    [
        root.join("data").join("grids"),
        root.join("tests").join("gie").join("tests"),
        root.join("data").join("tests"),
        root.join("tests").join("gie"),
    ]
}

/// First existing file for `name` in [`directories`], or the path itself.
pub(crate) fn resolve(name: &str) -> Option<PathBuf> {
    let given = Path::new(name);
    if !name.is_empty() && given.is_file() {
        return Some(given.to_path_buf());
    }
    let file_name = given.file_name()?;
    for directory in directories() {
        let nested = directory.join(name);
        if nested.is_file() {
            return Some(nested);
        }
        if file_name != given.as_os_str() {
            let bare = directory.join(file_name);
            if bare.is_file() {
                return Some(bare);
            }
        }
    }
    None
}
