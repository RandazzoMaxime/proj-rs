//! The Rust API gives the directories PROJ 9.9.0 gives for the same environment:
//! `ProjBuilder::user_writable_directory` is `proj_context_get_user_writable_directory`, and
//! `lib_info().searchpath` is `proj_info().searchpath`. The environment is the process's, set
//! here, so every case takes the lock.

#[path = "common/canonical.rs"]
mod canonical;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard};

use proj_rs::ProjBuilder;

static ENVIRONMENT: Mutex<()> = Mutex::new(());

const VARIABLES: [&str; 7] = [
    "PROJ_DATA",
    "PROJ_LIB",
    "PROJ_USER_WRITABLE_DIRECTORY",
    "PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY",
    "XDG_DATA_HOME",
    "HOME",
    "LOCALAPPDATA",
];

/// Holds the lock and clears the variables the directories are read from.
fn clean_environment() -> MutexGuard<'static, ()> {
    let guard = ENVIRONMENT
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    for variable in VARIABLES {
        std::env::remove_var(variable);
    }
    guard
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("proj-rs-dirs-api-{tag}-{}", std::process::id()));
    let _gone = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch");
    canonical::canonical(&dir)
}

fn expected_default(home: &Path) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(format!("{}/proj", home.display()))
    } else if cfg!(target_vendor = "apple") {
        home.join("Library/Application Support/proj")
    } else {
        home.join(".local/share/proj")
    }
}

fn set_home(home: &Path) {
    std::env::set_var("HOME", home);
    std::env::set_var("LOCALAPPDATA", home);
}

#[test]
fn default_user_directory_follows_the_platform() {
    let _lock = clean_environment();
    let home = scratch("default");
    set_home(&home);
    let builder = ProjBuilder::new();
    assert_eq!(builder.user_writable_directory(), expected_default(&home));
    if !cfg!(windows) {
        std::env::set_var("XDG_DATA_HOME", home.join("xdg"));
        assert_eq!(builder.user_writable_directory(), home.join("xdg/proj"));
    }
    std::env::set_var("PROJ_USER_WRITABLE_DIRECTORY", home.join("chosen"));
    assert_eq!(builder.user_writable_directory(), home.join("chosen"));
    let _gone = std::fs::remove_dir_all(&home);
}

#[test]
fn user_directory_is_not_created_until_asked() {
    let _lock = clean_environment();
    let home = scratch("create");
    set_home(&home);
    let mut builder = ProjBuilder::new();
    let default = builder.user_writable_directory();
    assert!(!default.exists(), "reading the directory creates nothing");
    builder.create_user_directory().expect("create");
    assert!(default.is_dir());
    builder.clear_user_writable_directory();
    assert_eq!(builder.user_writable_directory(), default);
    let _gone = std::fs::remove_dir_all(&home);
}

#[test]
fn search_path_lists_the_user_directory_then_proj_data() {
    let _lock = clean_environment();
    let home = scratch("list");
    set_home(&home);
    let separator = if cfg!(windows) { ";" } else { ":" };
    let mut builder = ProjBuilder::new();
    let user = expected_default(&home);
    let user = user.to_str().expect("utf-8");

    std::env::set_var("PROJ_DATA", "/data/a");
    let info = builder.lib_info().expect("info");
    assert_eq!(info.searchpath, format!("{user}{separator}/data/a"));

    std::env::set_var("PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY", "YES");
    assert_eq!(builder.lib_info().expect("info").searchpath, "/data/a");

    builder.set_search_paths("/added").expect("path");
    assert_eq!(builder.lib_info().expect("info").searchpath, "/added");
    let _gone = std::fs::remove_dir_all(&home);
}

/// The same environment given to the `projinfo` of PROJ 9.9.0 on `PATH`.
#[test]
fn directories_are_those_of_proj() {
    let Some(reference) = std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join(format!("projinfo{}", std::env::consts::EXE_SUFFIX)))
            .find(|candidate| candidate.is_file())
    }) else {
        assert!(
            std::env::var_os("PROJ_CS2CS_REQUIRED").is_none_or(|value| value != "1"),
            "PROJ_CS2CS_REQUIRED=1 and no projinfo on PATH"
        );
        return;
    };
    let _lock = clean_environment();
    let home = scratch("proj");
    set_home(&home);
    let work = scratch("proj-work");

    let scenarios: Vec<Vec<(&str, PathBuf)>> = vec![
        vec![],
        vec![("PROJ_DATA", PathBuf::from("/d1"))],
        vec![("PROJ_USER_WRITABLE_DIRECTORY", home.join("w"))],
        vec![("XDG_DATA_HOME", home.join("x"))],
        vec![("HOME", PathBuf::from("/no/such/home"))],
    ];
    for scenario in scenarios {
        if cfg!(windows) && scenario.iter().any(|(key, _)| *key == "XDG_DATA_HOME") {
            continue;
        }
        for variable in VARIABLES {
            std::env::remove_var(variable);
        }
        set_home(&home);
        for (key, value) in &scenario {
            std::env::set_var(key, value);
        }
        let mut command = Command::new(&reference);
        command.arg("--searchpaths").current_dir(&work);
        let theirs: Vec<String> =
            String::from_utf8_lossy(&command.output().expect("projinfo").stdout)
                .lines()
                .map(str::to_owned)
                .collect();
        let builder = ProjBuilder::new();
        let ours = builder.lib_info().expect("info").searchpath;
        let ours: Vec<&str> = ours.split(if cfg!(windows) { ';' } else { ':' }).collect();
        assert_eq!(
            ours.first().copied(),
            theirs.first().map(String::as_str),
            "user directory, {scenario:?}"
        );
        assert_eq!(
            builder.user_writable_directory().to_str(),
            theirs.first().map(String::as_str),
            "{scenario:?}"
        );
        if scenario.iter().any(|(key, _)| *key == "PROJ_DATA") {
            assert_eq!(ours, theirs, "{scenario:?}");
        }
    }
    let _gone = std::fs::remove_dir_all(&home);
    let _gone = std::fs::remove_dir_all(&work);
}
