//! Directories a grid or model file name is resolved against.
//!
//! After a file that exists relative to the working directory, the directories
//! are searched in PROJ's order (`pj_open_lib_internal`, `src/filemanager.cpp`):
//! the user-writable directory first, unless `PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY`
//! is set, then the data directories of [`crate::network::data_directories`]
//! (`PROJ_DATA`, or the default `data/grids`), then the paths added with
//! `set_search_paths`. Test fixtures are not searched unless a test puts them
//! on `PROJ_DATA`. A name may be a path or a bare filename. A missing file is
//! not a coordinate. When the network is on, [`read`] fetches a missing bare
//! name into the cache first.

use std::path::{Path, PathBuf};

/// Put the vendored fixtures on `PROJ_DATA` for this process, as PROJ's test
/// runs do (`proj_test_set_properties`, `cmake/ProjTest.cmake`). The vendored
/// `data` directory comes first, then its `tests` fixtures. Lookups read the
/// variable on each call, so the call may come after another test resolved a
/// grid. `tests/common` does the same for the integration tests.
#[cfg(test)]
pub(crate) fn use_fixture_search_path() {
    let data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
    let separator = if cfg!(windows) { ";" } else { ":" };
    let path = format!(
        "{}{separator}{}",
        data.display(),
        data.join("tests").display()
    );
    std::env::set_var("PROJ_DATA", path);
    std::env::set_var("PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY", "YES");
}

/// The directories a name is looked up in, in order.
fn search_directories() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(cache) = crate::network::cache_dir_if_enabled() {
        dirs.push(cache);
    }
    dirs.extend(crate::network::data_directories());
    dirs.extend(crate::network::search_paths());
    dirs
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

/// `https://cdn.proj.org/<file>` is the file of that name on the search path.
fn strip_cdn(name: &str) -> Option<&str> {
    let rest = name.strip_prefix("https://cdn.proj.org/")?;
    if rest.is_empty() || rest.contains('/') || rest.contains('\\') || rest.contains('?') {
        None
    } else {
        Some(rest)
    }
}
