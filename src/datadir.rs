//! Where a resource file is looked for, and where downloaded ones are kept.
//!
//! A port of PROJ 9.9.0 `src/filemanager.cpp`, line by line in
//! `docs/PORTING-data-dirs.md`: `pj_open_lib_internal` (the order of the directories),
//! `pj_get_default_searchpaths` (what `proj_info().searchpath` lists),
//! `proj_context_get_user_writable_directory` (the per-OS default directory) and
//! `pj_get_relative_share_proj` (`../share/proj` of the module). The directory PROJ's CMake
//! compiles in (`PROJ_DATA`) has no counterpart here by default: see [`compiled_data_dir`].

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::network::{FileFinder, GridContext};

/// The family of platform whose rules `proj_context_get_user_writable_directory` follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Os {
    Windows,
    /// `__MACH__ && __APPLE__`.
    Apple,
    /// Linux and every other platform.
    Unix,
}

impl Os {
    pub(crate) const fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_vendor = "apple") {
            Self::Apple
        } else {
            Self::Unix
        }
    }
}

/// `DIR_CHAR` of `proj_internal.h`.
const DIR_CHAR: char = if cfg!(windows) { '\\' } else { '/' };

/// `dirSeparator` of `pj_open_lib_internal`: the separator of a list of directories.
pub(crate) const LIST_SEPARATOR: char = if cfg!(windows) { ';' } else { ':' };

/// The directory PROJ's CMake compiles in as `PROJ_DATA` (`CMAKE_INSTALL_FULL_DATADIR/proj`).
///
/// proj-rs has no installation, so there is none by default and nothing of the build machine
/// is baked into a binary. A packager that installs the data files sets `PROJ_RS_DATA_DIR`
/// when it builds, the counterpart of `-DCMAKE_INSTALL_PREFIX`.
pub(crate) fn compiled_data_dir() -> Option<&'static str> {
    option_env!("PROJ_RS_DATA_DIR").filter(|dir| !dir.is_empty())
}

// ---------------------------------------------------------------------------
// The user-writable directory.

/// `proj_context_get_user_writable_directory` without a directory set on the context.
///
/// The variables are read through `env` and the writability of `$HOME` is asked of `writable`
/// (`access(home, W_OK)`), so that the three platforms can be tested on one.
pub(crate) fn user_directory_from(
    os: Os,
    env: &dyn Fn(&str) -> Option<OsString>,
    writable: &dyn Fn(&Path) -> bool,
) -> PathBuf {
    if let Some(dir) = env("PROJ_USER_WRITABLE_DIRECTORY").filter(|dir| !dir.is_empty()) {
        return PathBuf::from(dir);
    }
    let mut path = match os {
        Os::Windows => env("LOCALAPPDATA")
            .or_else(|| env("TEMP"))
            .unwrap_or_else(|| OsString::from("c:/users")),
        Os::Apple | Os::Unix => match env("XDG_DATA_HOME") {
            Some(xdg) => xdg,
            None => match env("HOME").filter(|home| writable(Path::new(home))) {
                Some(mut home) => {
                    home.push(if os == Os::Apple {
                        "/Library/Application Support"
                    } else {
                        "/.local/share"
                    });
                    home
                }
                None => OsString::from("/tmp"),
            },
        },
    };
    path.push("/proj");
    PathBuf::from(path)
}

/// The user-writable directory of the process environment.
pub(crate) fn user_directory() -> PathBuf {
    user_directory_from(
        Os::current(),
        &|key| std::env::var_os(key),
        &home_is_writable,
    )
}

/// `access(path, W_OK) == 0`, as `filemanager.cpp`: the kernel decides with the real user and
/// group, root, ACLs and read-only mounts included. `rustix` makes the same system call
/// (`faccessat(AT_FDCWD, path, W_OK, 0)`) behind a safe API. A path with a NUL byte is refused.
#[cfg(unix)]
fn home_is_writable(path: &Path) -> bool {
    rustix::fs::access(path, rustix::fs::Access::WRITE_OK).is_ok()
}

#[cfg(not(unix))]
fn home_is_writable(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|meta| !meta.permissions().readonly())
}

/// `dontReadUserWritableDirectory`: the variable is set and not empty.
pub(crate) fn skip_user_directory() -> bool {
    std::env::var_os("PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY")
        .is_some_and(|value| !value.is_empty())
}

// ---------------------------------------------------------------------------
// `../share/proj` of the module.

static MODULE: OnceLock<String> = OnceLock::new();
static RELATIVE_SHARE: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Name the file the library is loaded from, when it is not the executable: the C API of
/// `libproj_rs_capi`, as `dladdr` and `GetModuleFileName` name `libproj` for PROJ. It counts
/// only before the first lookup, which settles `../share/proj` for the life of the process.
pub(crate) fn set_module_path(path: &str) {
    let _ignored = MODULE.set(path.to_owned());
}

/// `pj_get_relative_share_proj_internal_no_check` for the module `module`.
///
/// Two directories up from the file, then `share/proj`: `<prefix>/lib/libproj.so` and
/// `<prefix>/bin/tool` both give `<prefix>/share/proj`. The relative forms that PROJ returns
/// for a path with fewer directories are kept: they are relative to the working directory.
pub(crate) fn relative_share_proj_for(module: &str, os: Os) -> String {
    let windows = os == Os::Windows;
    let separator = if windows { '\\' } else { '/' };
    let mut out = module.to_owned();
    if !windows {
        out = out.replace("/bin/../", "/");
        if let Some(rest) = out.strip_prefix("./") {
            out = rest.to_owned();
        }
    }
    let Some(position) = out.rfind(separator) else {
        return "../share/proj".to_owned();
    };
    out.truncate(position);
    let Some(position) = out.rfind(separator) else {
        return "share/proj".to_owned();
    };
    out.truncate(position);
    out.push_str("/share/proj");
    out
}

/// `pj_get_relative_share_proj`: the `../share/proj` of the module when that directory exists.
///
/// Computed once for the process, like the `static` of PROJ. The module is the executable, in
/// which the library is linked, unless [`set_module_path`] named another.
pub(crate) fn relative_share_proj() -> Option<&'static Path> {
    RELATIVE_SHARE
        .get_or_init(|| {
            let module = MODULE.get().cloned().or_else(|| {
                std::env::current_exe()
                    .ok()
                    .and_then(|path| path.to_str().map(str::to_owned))
            })?;
            let relative = relative_share_proj_for(&module, Os::current());
            Path::new(&relative)
                .exists()
                .then(|| PathBuf::from(relative))
        })
        .as_deref()
}

// ---------------------------------------------------------------------------
// The lookup.

/// The places a name is looked for, and in which order: everything `pj_open_lib_internal`
/// reads, taken out of the environment so that a test can give its own.
#[derive(Clone, Debug, Default)]
pub(crate) struct Lookup {
    /// `$HOME`, for `~/name`.
    pub(crate) home: Option<OsString>,
    /// The application's file finder.
    pub(crate) finder: Option<FileFinder>,
    /// The paths set with `proj_context_set_search_paths`. They replace all the rest.
    pub(crate) search_paths: Vec<PathBuf>,
    /// The user-writable directory, `None` when `PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY` says
    /// not to read it.
    pub(crate) user_dir: Option<PathBuf>,
    /// `PROJ_DATA`, else `PROJ_LIB`: a list of directories. When it is set the two that follow
    /// are not tried.
    pub(crate) proj_data: Option<OsString>,
    /// `../share/proj` of the module.
    pub(crate) relative_share: Option<PathBuf>,
    /// The compiled data directory.
    pub(crate) compiled: Option<PathBuf>,
    /// A hermetic lookup: the working directory is not tried after the others. It is the last
    /// place of `pj_open_lib_internal`, and the machine's.
    pub(crate) hermetic: bool,
}

impl Lookup {
    /// The lookup of a hermetic context: the search path, else the user-writable directory the
    /// builder set. Nothing else of the machine is read: no `HOME` for `~/`, no `PROJ_DATA` or
    /// `PROJ_LIB`, no directory of the module, no compiled directory and no working directory.
    /// A name that is a path (`/x`, `./x`) is still opened, as the caller wrote it.
    pub(crate) fn hermetic(ctx: &GridContext) -> Self {
        Self {
            home: None,
            finder: ctx.finder().cloned(),
            search_paths: ctx.added_search_paths().to_vec(),
            user_dir: ctx.writable_directory(),
            proj_data: None,
            relative_share: None,
            compiled: None,
            hermetic: true,
        }
    }

    /// The lookup of a context under the process environment.
    pub(crate) fn of(ctx: &GridContext) -> Self {
        if ctx.is_hermetic() {
            return Self::hermetic(ctx);
        }
        let user_dir = (ctx.has_explicit_user_directory() || !skip_user_directory())
            .then(|| ctx.cache_directory());
        let proj_data = proj_data_env();
        let (relative_share, compiled) = if proj_data.is_some() {
            (None, None)
        } else {
            (
                relative_share_proj().map(Path::to_path_buf),
                compiled_data_dir().map(PathBuf::from),
            )
        };
        Self {
            home: std::env::var_os("HOME"),
            finder: ctx.finder().cloned(),
            search_paths: ctx.added_search_paths().to_vec(),
            user_dir,
            proj_data,
            relative_share,
            compiled,
            hermetic: false,
        }
    }

    /// The file `name` is, as `pj_open_lib_internal` opens it: the first branch that applies
    /// decides, and only the user-writable directory, `../share/proj` and the compiled
    /// directory let the next one try when the file is not there. `exists` tells whether a
    /// path is a file.
    pub(crate) fn find(&self, name: &str, exists: &dyn Fn(&Path) -> bool) -> Option<PathBuf> {
        let hit = |candidate: PathBuf| exists(&candidate).then_some(candidate);
        if is_tilde_slash(name) {
            return hit(join(self.home.as_deref()?, name));
        }
        if is_rel_or_absolute_filename(name) {
            return hit(PathBuf::from(name));
        }
        if let Some(path) = self.finder.as_ref().and_then(|finder| (finder.0)(name)) {
            return hit(path);
        }
        if !self.search_paths.is_empty() {
            return self
                .search_paths
                .iter()
                .find_map(|dir| hit(join(dir.as_os_str(), name)));
        }
        if let Some(found) = self
            .user_dir
            .as_ref()
            .and_then(|dir| hit(join(dir.as_os_str(), name)))
        {
            return Some(found);
        }
        if let Some(list) = &self.proj_data {
            return split_list(list).find_map(|dir| hit(join(&strip_quotes(&dir), name)));
        }
        if let Some(found) = self.relative_share.as_ref().and_then(|dir| {
            let mut path = dir.clone().into_os_string();
            path.push("/");
            path.push(name);
            hit(PathBuf::from(path))
        }) {
            return Some(found);
        }
        if let Some(found) = self
            .compiled
            .as_ref()
            .and_then(|dir| hit(join(dir.as_os_str(), name)))
        {
            return Some(found);
        }
        if self.hermetic {
            return None;
        }
        hit(PathBuf::from(name))
    }

    /// The directories a bare name is looked in, in order, without the working directory.
    pub(crate) fn directories(&self) -> Vec<PathBuf> {
        if !self.search_paths.is_empty() {
            return self.search_paths.clone();
        }
        let mut dirs: Vec<PathBuf> = self.user_dir.iter().cloned().collect();
        if let Some(list) = &self.proj_data {
            dirs.extend(split_list(list).map(|dir| PathBuf::from(strip_quotes(&dir))));
        } else {
            dirs.extend(self.relative_share.iter().cloned());
            dirs.extend(self.compiled.iter().cloned());
        }
        dirs
    }

    /// The `searchpath` of `proj_info()` (`pj_get_default_searchpaths`): the paths set on the
    /// context, else the user-writable directory, then `PROJ_DATA` as it is written or else
    /// `../share/proj` and the compiled directory. Empty entries are left out.
    pub(crate) fn searchpath(&self) -> String {
        let entries: Vec<&OsStr> = if self.search_paths.is_empty() {
            let mut entries: Vec<&OsStr> =
                self.user_dir.iter().map(|dir| dir.as_os_str()).collect();
            match &self.proj_data {
                Some(list) => entries.push(list),
                None => {
                    entries.extend(self.relative_share.iter().map(|dir| dir.as_os_str()));
                    entries.extend(self.compiled.iter().map(|dir| dir.as_os_str()));
                }
            }
            entries
        } else {
            self.search_paths
                .iter()
                .map(|dir| dir.as_os_str())
                .collect()
        };
        entries
            .into_iter()
            .filter(|entry| !entry.is_empty())
            .map(|entry| entry.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(&LIST_SEPARATOR.to_string())
    }
}

/// `getProjDataEnvVar`: `PROJ_DATA`, else the legacy `PROJ_LIB`. An empty `PROJ_DATA` hides
/// `PROJ_LIB`, and an empty value counts as unset.
pub(crate) fn proj_data_env() -> Option<OsString> {
    std::env::var_os("PROJ_DATA")
        .or_else(|| std::env::var_os("PROJ_LIB"))
        .filter(|value| !value.is_empty())
}

/// `dir + DIR_CHAR + name`, as PROJ writes the name it opens.
fn join(dir: &OsStr, name: &str) -> PathBuf {
    let mut path = dir.to_owned();
    path.push(DIR_CHAR.to_string());
    path.push(name);
    PathBuf::from(path)
}

/// `NS_PROJ::internal::split(list, dirSeparator)`: empty entries stay.
fn split_list(list: &OsStr) -> impl Iterator<Item = OsString> + '_ {
    let text = list.to_string_lossy().into_owned();
    text.split(LIST_SEPARATOR)
        .map(OsString::from)
        .collect::<Vec<_>>()
        .into_iter()
}

/// `NS_PROJ::internal::stripQuotes`: one pair of double quotes around the whole entry.
fn strip_quotes(entry: &OsStr) -> OsString {
    let text = entry.to_string_lossy();
    match text
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
    {
        Some(inner) if text.len() >= 2 => OsString::from(inner),
        _ => entry.to_owned(),
    }
}

/// `strchr(dir_chars, c)` for the character that follows: the end of the name counts, as the
/// terminating NUL does in C.
fn is_dir_char(next: Option<u8>) -> bool {
    match next {
        None | Some(b'/') => true,
        Some(b'\\') => cfg!(windows),
        Some(_) => false,
    }
}

/// `is_tilde_slash` of `filemanager.cpp`.
pub(crate) fn is_tilde_slash(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.first() == Some(&b'~') && is_dir_char(bytes.get(1).copied())
}

/// `is_rel_or_absolute_filename` of `filemanager.cpp`: `/name`, `./name`, `../name`,
/// `C:/name`, and the empty name.
pub(crate) fn is_rel_or_absolute_filename(name: &str) -> bool {
    let bytes = name.as_bytes();
    let at = |index: usize| bytes.get(index).copied();
    is_dir_char(at(0))
        || (at(0) == Some(b'.') && is_dir_char(at(1)))
        || (bytes.starts_with(b"..") && is_dir_char(at(2)))
        || (!bytes.is_empty() && at(1) == Some(b':') && is_dir_char(at(2)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn env_of<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<OsString> + 'a {
        move |key| {
            pairs
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| OsString::from(*value))
        }
    }

    #[derive(Clone, Copy, PartialEq)]
    enum Home {
        Writable,
        ReadOnly,
    }

    fn user_dir(os: Os, pairs: &[(&str, &str)], home: Home) -> String {
        user_directory_from(os, &env_of(pairs), &|_| home == Home::Writable)
            .to_string_lossy()
            .into_owned()
    }

    /// `filemanager.cpp:1286-1311`.
    #[test]
    fn user_directory_on_linux_follows_xdg_then_home() {
        let home = [("HOME", "/home/u")];
        assert_eq!(
            user_dir(Os::Unix, &home, Home::Writable),
            "/home/u/.local/share/proj"
        );
        let both = [("HOME", "/home/u"), ("XDG_DATA_HOME", "/data")];
        assert_eq!(user_dir(Os::Unix, &both, Home::Writable), "/data/proj");
        assert_eq!(user_dir(Os::Unix, &both, Home::ReadOnly), "/data/proj");
    }

    #[test]
    fn user_directory_without_a_writable_home_is_tmp() {
        assert_eq!(
            user_dir(Os::Unix, &[("HOME", "/home/u")], Home::ReadOnly),
            "/tmp/proj"
        );
        assert_eq!(user_dir(Os::Unix, &[], Home::Writable), "/tmp/proj");
        assert_eq!(user_dir(Os::Apple, &[], Home::Writable), "/tmp/proj");
    }

    /// An empty `XDG_DATA_HOME` is set, so it is used: the path is `/proj`.
    #[test]
    fn empty_xdg_data_home_is_kept_as_it_is() {
        let env = [("XDG_DATA_HOME", ""), ("HOME", "/home/u")];
        assert_eq!(user_dir(Os::Unix, &env, Home::Writable), "/proj");
    }

    #[test]
    fn user_directory_on_macos_is_in_application_support() {
        let env = [("HOME", "/Users/u")];
        assert_eq!(
            user_dir(Os::Apple, &env, Home::Writable),
            "/Users/u/Library/Application Support/proj"
        );
        let xdg = [("HOME", "/Users/u"), ("XDG_DATA_HOME", "/x")];
        assert_eq!(user_dir(Os::Apple, &xdg, Home::Writable), "/x/proj");
    }

    #[test]
    fn user_directory_on_windows_is_local_app_data() {
        let env = [
            ("LOCALAPPDATA", r"C:\Users\u\AppData\Local"),
            ("TEMP", r"C:\T"),
        ];
        assert_eq!(
            user_dir(Os::Windows, &env, Home::Writable),
            r"C:\Users\u\AppData\Local/proj"
        );
        assert_eq!(
            user_dir(Os::Windows, &[("TEMP", r"C:\T")], Home::Writable),
            r"C:\T/proj"
        );
        assert_eq!(user_dir(Os::Windows, &[], Home::Writable), "c:/users/proj");
    }

    #[test]
    fn user_writable_directory_variable_wins_when_it_is_not_empty() {
        let env = [("PROJ_USER_WRITABLE_DIRECTORY", "/w"), ("HOME", "/home/u")];
        assert_eq!(user_dir(Os::Unix, &env, Home::Writable), "/w");
        let empty = [("PROJ_USER_WRITABLE_DIRECTORY", ""), ("HOME", "/home/u")];
        assert_eq!(
            user_dir(Os::Unix, &empty, Home::Writable),
            "/home/u/.local/share/proj"
        );
    }

    /// `filemanager.cpp:1401-1480`.
    #[test]
    fn relative_share_proj_goes_two_directories_up() {
        let unix = |module| relative_share_proj_for(module, Os::Unix);
        assert_eq!(
            unix("/usr/local/lib/libproj.so.25"),
            "/usr/local/share/proj"
        );
        assert_eq!(unix("/opt/app/bin/tool"), "/opt/app/share/proj");
        assert_eq!(
            unix("/opt/app/bin/../lib/libproj.so"),
            "/opt/app/share/proj"
        );
        assert_eq!(unix("bin/tool"), "share/proj");
        assert_eq!(unix("./bin/tool"), "share/proj");
        assert_eq!(unix("libproj.so"), "../share/proj");
        assert_eq!(unix("/tool"), "share/proj");
        let windows = |module| relative_share_proj_for(module, Os::Windows);
        assert_eq!(windows(r"C:\proj\bin\proj.dll"), "C:\\proj/share/proj");
        assert_eq!(windows("proj.dll"), "../share/proj");
    }

    #[test]
    fn name_kinds_follow_filemanager() {
        for name in ["/a", "./a", "../a", "C:/a", "", ".", "..", "C:"] {
            assert!(is_rel_or_absolute_filename(name), "{name:?}");
        }
        for name in ["a", "a/b", ".a", "..a", "~", "~a", "ab:/b", "http://x/y"] {
            assert!(!is_rel_or_absolute_filename(name), "{name:?}");
        }
        assert!(is_tilde_slash("~/a") && is_tilde_slash("~"));
        assert!(!is_tilde_slash("~a") && !is_tilde_slash("a~/"));
        assert_eq!(is_rel_or_absolute_filename(r"\a"), cfg!(windows));
    }

    #[test]
    fn list_keeps_empty_entries_and_strips_quotes() {
        let list = OsString::from(format!("a{LIST_SEPARATOR}{LIST_SEPARATOR}\"b\""));
        let entries: Vec<OsString> = split_list(&list).collect();
        assert_eq!(entries, ["a", "", "\"b\""].map(OsString::from));
        assert_eq!(strip_quotes(OsStr::new("\"b\"")), OsString::from("b"));
        assert_eq!(strip_quotes(OsStr::new("\"")), OsString::from("\""));
        assert_eq!(strip_quotes(OsStr::new("\"b")), OsString::from("\"b"));
    }

    fn scratch(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("proj-rs-datadir-{tag}-{}", std::process::id()));
        let _gone = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("scratch");
        dir
    }

    fn put(dir: &Path, name: &str, text: &str) -> PathBuf {
        fs::create_dir_all(dir).expect("directory");
        let path = dir.join(name);
        fs::write(&path, text).expect("file");
        path
    }

    fn content(found: Option<PathBuf>) -> Option<String> {
        found.map(|path| fs::read_to_string(path).expect("found file"))
    }

    fn is_file(path: &Path) -> bool {
        path.is_file()
    }

    /// The order of `pj_open_lib_internal` with every place holding the same name.
    #[test]
    fn the_first_place_that_has_the_file_wins() {
        let root = scratch("order");
        let user = root.join("user");
        let data = root.join("data");
        let rel = root.join("rel");
        let compiled = root.join("compiled");
        put(&user, "g.tif", "user");
        put(&data, "g.tif", "data");
        put(&rel, "g.tif", "relative");
        put(&compiled, "g.tif", "compiled");
        let added = root.join("added");
        put(&added, "g.tif", "added");
        let mut lookup = Lookup {
            user_dir: Some(user.clone()),
            proj_data: Some(data.clone().into_os_string()),
            relative_share: Some(rel.clone()),
            compiled: Some(compiled),
            ..Lookup::default()
        };
        // The paths of the context replace the others.
        lookup.search_paths = vec![added];
        assert_eq!(
            content(lookup.find("g.tif", &is_file)).as_deref(),
            Some("added")
        );
        lookup.search_paths.clear();
        // Then the user directory, then PROJ_DATA.
        assert_eq!(
            content(lookup.find("g.tif", &is_file)).as_deref(),
            Some("user")
        );
        fs::remove_file(user.join("g.tif")).expect("remove");
        assert_eq!(
            content(lookup.find("g.tif", &is_file)).as_deref(),
            Some("data")
        );
        // PROJ_DATA set: the file is not looked for further, `../share/proj` and the compiled
        // directory are out of reach, as `getProjDataEnvVar` is a branch of its own.
        fs::remove_file(data.join("g.tif")).expect("remove");
        assert_eq!(lookup.find("g.tif", &is_file), None);
        // PROJ_DATA unset: `../share/proj`, then the compiled directory.
        lookup.proj_data = None;
        assert_eq!(
            content(lookup.find("g.tif", &is_file)).as_deref(),
            Some("relative")
        );
        fs::remove_file(rel.join("g.tif")).expect("remove");
        assert_eq!(
            content(lookup.find("g.tif", &is_file)).as_deref(),
            Some("compiled")
        );
        let _gone = fs::remove_dir_all(&root);
    }

    #[test]
    fn paths_of_the_context_do_not_fall_back_to_the_defaults() {
        let root = scratch("replace");
        let user = root.join("user");
        put(&user, "g.tif", "user");
        let lookup = Lookup {
            user_dir: Some(user),
            search_paths: vec![root.join("empty")],
            ..Lookup::default()
        };
        assert_eq!(lookup.find("g.tif", &is_file), None);
        let _gone = fs::remove_dir_all(&root);
    }

    /// `filemanager.cpp:1665`: the working directory is the last resort, and only when no
    /// branch above applied. `Cargo.toml` is in the working directory of the tests.
    #[test]
    fn bare_name_is_tried_last_and_never_after_proj_data() {
        assert_eq!(
            Lookup::default().find("Cargo.toml", &is_file),
            Some(PathBuf::from("Cargo.toml"))
        );
        let with_data = Lookup {
            proj_data: Some(OsString::from("/no/such/directory")),
            ..Lookup::default()
        };
        assert_eq!(with_data.find("Cargo.toml", &is_file), None);
        let with_paths = Lookup {
            search_paths: vec![PathBuf::from("/no/such/directory")],
            ..Lookup::default()
        };
        assert_eq!(with_paths.find("Cargo.toml", &is_file), None);
    }

    #[test]
    fn search_path_string_follows_pj_get_default_searchpaths() {
        let sep = LIST_SEPARATOR;
        let mut lookup = Lookup {
            user_dir: Some(PathBuf::from("/u/proj")),
            relative_share: Some(PathBuf::from("/p/share/proj")),
            compiled: Some(PathBuf::from("/usr/share/proj")),
            ..Lookup::default()
        };
        assert_eq!(
            lookup.searchpath(),
            format!("/u/proj{sep}/p/share/proj{sep}/usr/share/proj")
        );
        lookup.proj_data = Some(OsString::from(format!("/a{sep}/b")));
        lookup.relative_share = None;
        lookup.compiled = None;
        assert_eq!(lookup.searchpath(), format!("/u/proj{sep}/a{sep}/b"));
        lookup.search_paths = vec![PathBuf::from("/x"), PathBuf::new(), PathBuf::from("/y")];
        assert_eq!(lookup.searchpath(), format!("/x{sep}/y"));
        assert_eq!(Lookup::default().searchpath(), "");
    }

    #[test]
    fn tilde_name_is_looked_up_under_home_with_the_tilde_kept() {
        let root = scratch("tilde");
        put(&root.join("~"), "g.tif", "tilde");
        let lookup = Lookup {
            home: Some(root.clone().into_os_string()),
            ..Lookup::default()
        };
        assert_eq!(
            content(lookup.find("~/g.tif", &is_file)).as_deref(),
            Some("tilde")
        );
        assert_eq!(Lookup::default().find("~/g.tif", &is_file), None);
        let _gone = fs::remove_dir_all(&root);
    }

    /// A hermetic lookup reads its search path, else the directory its builder set. The working
    /// directory is never tried (`Cargo.toml` is there), and `~/` is not read under `HOME`.
    #[test]
    fn hermetic_lookup_reads_only_the_search_path_or_the_explicit_directory() {
        let root = scratch("hermetic");
        let explicit = root.join("explicit");
        put(&explicit, "g.tif", "explicit");
        put(&root.join("home"), "g.tif", "home");
        let context = GridContext::hermetic(Vec::new(), Some(explicit.clone()));
        let lookup = Lookup::of(&context);
        assert!(lookup.hermetic);
        assert_eq!(
            content(lookup.find("g.tif", &is_file)).as_deref(),
            Some("explicit")
        );
        assert_eq!(lookup.find("Cargo.toml", &is_file), None);
        assert_eq!(lookup.find("~/g.tif", &is_file), None);
        assert_eq!(lookup.searchpath(), explicit.display().to_string());

        let bare = GridContext::hermetic(Vec::new(), None);
        assert_eq!(Lookup::of(&bare).find("g.tif", &is_file), None);
        assert_eq!(Lookup::of(&bare).searchpath(), "");

        // The search path replaces the explicit directory, as PROJ's search paths replace the
        // other places of `pj_open_lib_internal`.
        let added = root.join("added");
        put(&added, "g.tif", "added");
        let searched = GridContext::hermetic(vec![added.clone()], Some(explicit));
        assert_eq!(
            content(Lookup::of(&searched).find("g.tif", &is_file)).as_deref(),
            Some("added")
        );
        assert_eq!(
            Lookup::of(&searched).searchpath(),
            added.display().to_string()
        );
        let _gone = fs::remove_dir_all(&root);
    }
}
