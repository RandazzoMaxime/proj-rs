//! Resolving a grid, model or dictionary name to a file, in PROJ's order.
//!
//! The order is that of `pj_open_lib_internal` (`src/filemanager.cpp`), in
//! [`crate::datadir::Lookup::find`]: the paths set on the context, or else the user-writable
//! directory (unless `PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY` is set), then `PROJ_DATA` or else
//! `../share/proj` of the module and the compiled directory, and the working directory last.
//! Test fixtures are not searched unless a test puts them on `PROJ_DATA`. A name may be a path
//! or a bare filename. A missing file is not a coordinate. When the network is on, [`read`]
//! fetches a missing bare name into the cache first.

use std::path::PathBuf;

use crate::datadir::{is_rel_or_absolute_filename, is_tilde_slash, Lookup};

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

/// First existing file for `name`, including a `grid_alternatives` rename.
///
/// As PROJ 9.9.0 `FileManager::open_resource_file` (`src/filemanager.cpp`,
/// 1786-1857): a name without `.tif` falls back to its PROJ name
/// (`getProjGridName`), a name with `.tif` to its old name
/// (`getOldProjGridName`). Paths, URLs and `proj.db` have no fallback.
pub(crate) fn resolve(name: &str) -> Option<PathBuf> {
    locate(name).or_else(|| {
        if is_tilde_slash(name)
            || is_rel_or_absolute_filename(name)
            || crate::network::is_http(name)
            || name == "proj.db"
        {
            return None;
        }
        let alternate = if name.contains(".tif") {
            crate::network::old_grid_filename(name)?
        } else {
            crate::network::proj_grid_filename(name)?
        };
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
        return crate::network::read_file(&path).ok();
    }
    crate::network::fetch_missing(name)
}

/// On disk, or, the network on, a grid the endpoint would serve ([`crate::network::decide_download`]):
/// without an open attempt. `proj_create_crs_to_crs` with the network on asks for
/// `PROJ_GRID_AVAILABILITY_KNOWN_AVAILABLE` (`crs_to_crs.cpp:572-576`), and `isPROJInstantiable`
/// (`singleoperation.cpp:339-356`) reads its grids through `gridsNeeded(db,
/// considerKnownGridsAsAvailable)` with the network state as the flag (`c_api.cpp:7918-7929`):
/// `lookForGridInfo` (`factory.cpp:3496-3510`) then answers that a grid with a package or an
/// open-license URL is available with the network temporarily off. The open attempt comes only
/// when the grid is read, at transformation time. Every building of a pair, of a pipeline and of
/// an instantiability mark goes through this one; only the lookups of `lookForGridInfo` open
/// (`DatabaseContext::look_for_grid`).
pub(crate) fn known_available(name: &str) -> bool {
    resolve(name).is_some() || crate::network::decide_download(name).is_some()
}

fn locate(name: &str) -> Option<PathBuf> {
    let name = strip_cdn(name).unwrap_or(name);
    if crate::network::is_http(name) {
        return None;
    }
    let ctx = crate::network::context();
    let api = ctx.file_api();
    let api = api.as_deref();
    Lookup::of(&ctx).find(name, &|path| crate::network::is_file_with(api, path))
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

#[cfg(test)]
mod tests {
    use super::{resolve, use_fixture_search_path};

    /// The file name `name` resolves to on the fixture path, if any.
    fn file_name_of(name: &str) -> Option<String> {
        use_fixture_search_path();
        let path = resolve(name)?;
        path.file_name()
            .map(|file| file.to_string_lossy().into_owned())
    }

    /// `de_adv_BETA2007.tif` is missing. Its `old_proj_grid_name`, `BETA2007.gsb`,
    /// is on the fixture path (`filemanager.cpp:1833-1857`, `factory.cpp:3575`).
    #[test]
    fn tif_name_falls_back_to_its_old_name() {
        assert_eq!(
            file_name_of("de_adv_BETA2007.tif").as_deref(),
            Some("BETA2007.gsb")
        );
    }

    /// `RAGTBT2016.gtx` is missing. Its PROJ name, `fr_ign_RAGTBT2016.tif`, is on
    /// the fixture path (`filemanager.cpp:1802-1831`, `factory.cpp:3563`).
    #[test]
    fn name_without_tif_falls_back_to_its_proj_name() {
        assert_eq!(
            file_name_of("RAGTBT2016.gtx").as_deref(),
            Some("fr_ign_RAGTBT2016.tif")
        );
    }

    /// A path is not renamed: the fallback is for bare names only.
    #[test]
    fn path_with_tif_is_not_renamed() {
        assert_eq!(file_name_of("sub/de_adv_BETA2007.tif"), None);
    }

    /// A name with no file and no `grid_alternatives` row stays missing.
    #[test]
    fn unknown_tif_name_stays_missing() {
        assert_eq!(file_name_of("no_such_grid.tif"), None);
    }
}
