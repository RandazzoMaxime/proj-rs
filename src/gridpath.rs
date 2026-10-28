//! Directories a grid or model file name is resolved against.
//!
//! `data/grids` holds CDN grids and is not part of the source tree.
//! `tests/gie/tests` and `data/tests` hold the vendored fixtures. A name may
//! be a path or a bare filename. A missing file is not a coordinate. When the
//! network is on, [`read`] fetches a missing bare name into the cache first.

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

/// First existing file for `name`, including a `grid_alternatives` rename.
pub(crate) fn resolve(name: &str) -> Option<PathBuf> {
    locate(name).or_else(|| {
        let alternate = crate::network::proj_grid_filename(name)?;
        if alternate == name {
            None
        } else {
            locate(&alternate)
        }
    })
}

/// Bytes of a grid, from disk or from [`crate::network::decide_download`].
pub(crate) fn read(name: &str) -> Option<Vec<u8>> {
    if let Some(path) = resolve(name) {
        return std::fs::read(path).ok();
    }
    crate::network::fetch_missing(name)
}

/// The file is on disk, or the network would fetch it.
pub(crate) fn available(name: &str) -> bool {
    resolve(name).is_some() || crate::network::decide_download(name).is_some()
}

fn locate(name: &str) -> Option<PathBuf> {
    let name = strip_cdn(name).unwrap_or(name);
    if name.starts_with("http://") || name.starts_with("https://") {
        return None;
    }
    let given = Path::new(name);
    if !name.is_empty() && given.is_file() {
        return Some(given.to_path_buf());
    }
    let file_name = given.file_name()?;
    for directory in search_directories() {
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

fn search_directories() -> Vec<PathBuf> {
    let mut dirs = Vec::from(directories());
    dirs.extend(crate::network::search_paths());
    if let Some(cache) = crate::network::cache_dir_if_enabled() {
        dirs.push(cache);
    }
    dirs
}

/// `https://cdn.proj.org/<file>` is the file of that name on the search path.
fn strip_cdn(name: &str) -> Option<&str> {
    let rest = name.strip_prefix("https://cdn.proj.org/")?;
    if rest.is_empty() || rest.contains('/') || rest.contains('\\') || rest.contains('?') {
        None
    } else {
        Some(rest)
    }
}
