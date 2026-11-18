//! Where the binaries look for a grid and for `proj.ini`, as PROJ 9.9.0 does
//! (`pj_open_lib_internal`, `pj_get_default_searchpaths`,
//! `proj_context_get_user_writable_directory`; `docs/PORTING-data-dirs.md`).
//!
//! Each case runs a real binary from another working directory, with a home directory of its
//! own, so that nothing of the machine it runs on or of the checkout is read. A grid is "found"
//! when the operation succeeds, and "not found" when it fails: a place that holds a file that is
//! not a grid is the one the lookup chose when the operation fails.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::Duration;

const GRID: &str = "test_hgrid.tif";
const POINT: &str = "5.5 53.5 0 0\n";
const VARIABLES: [&str; 8] = [
    "PROJ_DATA",
    "PROJ_LIB",
    "PROJ_USER_WRITABLE_DIRECTORY",
    "PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY",
    "PROJ_NETWORK",
    "XDG_DATA_HOME",
    "HOME",
    "LOCALAPPDATA",
];

/// A scratch directory with a home, a working directory and an install prefix, removed on drop.
struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!("proj-rs-dirs-{tag}-{}", std::process::id()));
        let _gone = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("sandbox");
        let root = fs::canonicalize(&root).expect("canonical sandbox");
        for dir in ["home", "work", "prefix/bin", "local"] {
            fs::create_dir_all(root.join(dir)).expect("sandbox directory");
        }
        Self { root }
    }

    fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    fn work(&self) -> PathBuf {
        self.root.join("work")
    }

    fn prefix(&self) -> PathBuf {
        self.root.join("prefix")
    }

    /// The user directory PROJ gives for this home on this platform.
    fn user_dir(&self) -> PathBuf {
        if cfg!(windows) {
            PathBuf::from(format!("{}/proj", self.root.join("local").display()))
        } else if cfg!(target_vendor = "apple") {
            self.home().join("Library/Application Support/proj")
        } else {
            self.home().join(".local/share/proj")
        }
    }

    /// A command with the lookup variables cleared, the home of the sandbox and its working
    /// directory.
    fn command(&self, program: &Path) -> Command {
        let mut command = Command::new(program);
        for variable in VARIABLES {
            command.env_remove(variable);
        }
        command
            .env("HOME", self.home())
            .env("LOCALAPPDATA", self.root.join("local"))
            .current_dir(self.work());
        command
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _gone = fs::remove_dir_all(&self.root);
    }
}

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("data/tests")
        .join(GRID)
}

/// `GRID` is a valid grid in `dir`.
fn put_grid(dir: &Path) {
    fs::create_dir_all(dir).expect("directory");
    fs::copy(fixture(), dir.join(GRID)).expect("copy the grid");
}

/// `GRID` is a file that is not a grid in `dir`: a lookup that stops there fails.
fn put_broken(dir: &Path) {
    fs::create_dir_all(dir).expect("directory");
    fs::write(dir.join(GRID), b"not a grid").expect("write");
}

/// Start `command`, again when the file is "busy": another test thread has the copy of the
/// binary open for writing in a forked child that has not executed yet.
fn start(command: &mut Command) -> Child {
    for _ in 0..50 {
        match command.spawn() {
            Err(error) if error.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(Duration::from_millis(20));
            }
            other => return other.expect("the program starts"),
        }
    }
    command.spawn().expect("the program starts")
}

/// The output of `command`, started with [`start`].
fn run(command: &mut Command) -> Output {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    start(command).wait_with_output().expect("the program ends")
}

/// Whether `cct` shifts a point with `GRID`.
fn shifts(mut command: Command) -> bool {
    command
        .args(["+proj=hgridshift", &format!("+grids={GRID}")])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = start(&mut command);
    // The binary may exit on a missing grid before it reads: a broken pipe is not a failure.
    let mut stdin = child.stdin.take().expect("stdin");
    let _ignored = stdin.write_all(POINT.as_bytes());
    drop(stdin);
    let output = child.wait_with_output().expect("cct ends");
    let text = String::from_utf8_lossy(&output.stdout).to_lowercase();
    output.status.success() && !text.contains("inf") && !text.contains("nan")
}

fn cct() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_cct"))
}

fn projinfo() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_projinfo"))
}

fn lines(mut command: Command) -> Vec<String> {
    let output = run(command.arg("--searchpaths"));
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .collect()
}

/// `cct` copied under `<prefix>/bin`, as an installation lays it out.
fn install(sandbox: &Sandbox, binary: &Path) -> PathBuf {
    let name = binary.file_name().expect("file name");
    let target = sandbox.prefix().join("bin").join(name);
    fs::copy(binary, &target).expect("copy the binary");
    target
}

#[test]
fn control_without_any_grid_fails() {
    let sandbox = Sandbox::new("control");
    assert!(!shifts(sandbox.command(&cct())));
    put_grid(&sandbox.work().join("elsewhere"));
    assert!(
        !shifts(sandbox.command(&cct())),
        "an unrelated directory is not searched"
    );
}

#[test]
fn grid_in_the_user_directory_is_found_from_another_directory() {
    let sandbox = Sandbox::new("user");
    put_grid(&sandbox.user_dir());
    assert!(shifts(sandbox.command(&cct())));
    let listed = lines(sandbox.command(&projinfo()));
    assert_eq!(
        listed.first().map(String::as_str),
        Some(sandbox.user_dir().to_str().expect("utf-8"))
    );
}

#[test]
fn user_directory_variable_replaces_the_default_one() {
    let sandbox = Sandbox::new("user-var");
    let elsewhere = sandbox.root.join("chosen");
    put_grid(&elsewhere);
    put_grid(&sandbox.user_dir());
    fs::remove_file(sandbox.user_dir().join(GRID)).expect("remove");
    let mut command = sandbox.command(&cct());
    command.env("PROJ_USER_WRITABLE_DIRECTORY", &elsewhere);
    assert!(shifts(command));
}

#[test]
fn skip_variable_hides_the_user_directory() {
    let sandbox = Sandbox::new("skip");
    put_grid(&sandbox.user_dir());
    let mut command = sandbox.command(&cct());
    command.env("PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY", "YES");
    assert!(!shifts(command));
    let mut empty = sandbox.command(&cct());
    empty.env("PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY", "");
    assert!(shifts(empty), "an empty value is not a skip");
}

#[test]
fn grid_in_share_proj_next_to_the_binary_directory_is_found() {
    let sandbox = Sandbox::new("share");
    let binary = install(&sandbox, &cct());
    let info = install(&sandbox, &projinfo());
    assert!(!shifts(sandbox.command(&binary)));
    put_grid(&sandbox.prefix().join("share/proj"));
    assert!(shifts(sandbox.command(&binary)));
    let listed = lines(sandbox.command(&info));
    let slashes = |text: &str| text.replace('\\', "/");
    let share = slashes(sandbox.prefix().join("share/proj").to_str().expect("utf-8"));
    assert_eq!(listed.last().map(|line| slashes(line)), Some(share));
}

#[test]
fn proj_data_hides_share_proj_and_the_working_directory() {
    let sandbox = Sandbox::new("mask");
    let binary = install(&sandbox, &cct());
    put_grid(&sandbox.prefix().join("share/proj"));
    put_grid(&sandbox.work());
    let empty = sandbox.root.join("empty");
    fs::create_dir_all(&empty).expect("empty");
    assert!(shifts(sandbox.command(&binary)));
    let mut masked = sandbox.command(&binary);
    masked.env("PROJ_DATA", &empty);
    assert!(!shifts(masked));
}

#[test]
fn working_directory_is_the_last_resort() {
    let sandbox = Sandbox::new("cwd");
    put_grid(&sandbox.work());
    assert!(shifts(sandbox.command(&cct())));
    let broken = sandbox.root.join("share-broken");
    put_broken(&broken);
    let mut data = sandbox.command(&cct());
    data.env("PROJ_DATA", &broken);
    assert!(!shifts(data), "PROJ_DATA ends the search with its own file");
    let mut absent = sandbox.command(&cct());
    absent.env("PROJ_DATA", sandbox.root.join("nowhere"));
    assert!(
        !shifts(absent),
        "a PROJ_DATA without the file does not fall back to the directory"
    );
}

#[test]
fn the_same_name_in_two_places_is_taken_from_the_first_of_proj() {
    let sandbox = Sandbox::new("order");
    let binary = install(&sandbox, &cct());
    let data = sandbox.root.join("data");
    let share = sandbox.prefix().join("share/proj");

    // User directory before PROJ_DATA.
    put_grid(&sandbox.user_dir());
    put_broken(&data);
    let mut command = sandbox.command(&binary);
    command.env("PROJ_DATA", &data);
    assert!(shifts(command), "the user directory comes first");
    fs::remove_file(sandbox.user_dir().join(GRID)).expect("remove");
    put_broken(&sandbox.user_dir());
    put_grid(&data);
    let mut command = sandbox.command(&binary);
    command.env("PROJ_DATA", &data);
    assert!(
        !shifts(command),
        "the user directory's file is the one opened"
    );
    fs::remove_file(sandbox.user_dir().join(GRID)).expect("remove");

    // PROJ_DATA before share/proj, and share/proj when PROJ_DATA is not set.
    put_broken(&share);
    let mut command = sandbox.command(&binary);
    command.env("PROJ_DATA", &data);
    assert!(shifts(command), "PROJ_DATA comes before share/proj");
    assert!(
        !shifts(sandbox.command(&binary)),
        "without PROJ_DATA, share/proj is used"
    );

    // In PROJ_DATA, the first directory that has the file is the one opened.
    let separator = if cfg!(windows) { ";" } else { ":" };
    let other = sandbox.root.join("other");
    put_broken(&other);
    let both = format!("{}{separator}{}", other.display(), data.display());
    let mut command = sandbox.command(&binary);
    command.env("PROJ_DATA", &both);
    assert!(!shifts(command), "the first directory of the list wins");
    let both = format!(
        "{}{separator}{}",
        sandbox.root.join("nowhere").display(),
        data.display()
    );
    let mut command = sandbox.command(&binary);
    command.env("PROJ_DATA", &both);
    assert!(
        shifts(command),
        "a directory without the file is passed over"
    );
}

#[test]
fn proj_lib_is_the_old_name_and_an_empty_proj_data_hides_it() {
    let sandbox = Sandbox::new("lib");
    let data = sandbox.root.join("data");
    put_grid(&data);
    let mut command = sandbox.command(&cct());
    command.env("PROJ_LIB", &data);
    assert!(shifts(command));
    let mut command = sandbox.command(&cct());
    command.env("PROJ_LIB", &data).env("PROJ_DATA", "");
    assert!(!shifts(command));
}

/// `proj.ini` is found like a grid: in the user directory it sets the network and the endpoint,
/// which `projinfo --remote-data` reports (`pj_load_ini`). A build without the `network` feature
/// has no client and says disabled whatever the file says, as PROJ without libcurl.
#[test]
fn proj_ini_is_read_from_the_user_directory() {
    let sandbox = Sandbox::new("ini");
    fs::create_dir_all(sandbox.user_dir()).expect("user dir");
    fs::write(
        sandbox.user_dir().join("proj.ini"),
        "network = on\ncdn_endpoint = https://example.invalid\n",
    )
    .expect("proj.ini");
    let output = run(sandbox.command(&projinfo()).arg("--remote-data"));
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    if cfg!(feature = "network") {
        assert!(text.contains("Status: enabled"), "{text}");
        assert!(text.contains("URL: https://example.invalid"), "{text}");
    } else {
        assert!(text.contains("Status: disabled"), "{text}");
    }
    let mut skipped = sandbox.command(&projinfo());
    skipped.env("PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY", "YES");
    let output = run(skipped.arg("--remote-data"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("Status: disabled"));
}

// ---------------------------------------------------------------------------
// Against PROJ 9.9.0's own projinfo.

/// The `projinfo` of PROJ 9.9.0 on `PATH`, or `None` when the case may be skipped. CI sets
/// `PROJ_CS2CS_REQUIRED=1`, as for `agree_cs2cs`, and then a missing binary fails.
fn reference_projinfo() -> Option<PathBuf> {
    let found = std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join(format!("projinfo{}", std::env::consts::EXE_SUFFIX)))
            .find(|candidate| candidate.is_file())
    });
    if found.is_none() {
        assert!(
            std::env::var_os("PROJ_CS2CS_REQUIRED").is_none_or(|value| value != "1"),
            "PROJ_CS2CS_REQUIRED=1 and no projinfo on PATH"
        );
        eprintln!("projinfo of PROJ is not on PATH: the comparison is skipped");
    }
    found
}

/// What of the output is compared.
#[derive(Clone, Copy, PartialEq)]
enum Compared {
    /// The whole list: the environment sets `PROJ_DATA` or `PROJ_LIB`, so PROJ lists nothing of
    /// its installation.
    Whole,
    /// The first line, the user directory.
    UserDirectory,
}

/// What PROJ and proj-rs print for `--searchpaths` under the same environment.
fn compare(
    sandbox: &Sandbox,
    reference: &Path,
    compared: Compared,
    configure: &dyn Fn(&mut Command),
) {
    let mut theirs = sandbox.command(reference);
    configure(&mut theirs);
    let mut ours = sandbox.command(&projinfo());
    configure(&mut ours);
    let theirs = lines(theirs);
    let ours = lines(ours);
    if compared == Compared::Whole {
        assert_eq!(ours, theirs);
    } else {
        // Without PROJ_DATA PROJ also lists the directory of its installation, which a build of
        // proj-rs has no counterpart for: only the user directory is compared.
        assert_eq!(ours.first(), theirs.first(), "{theirs:?} against {ours:?}");
    }
}

#[test]
fn search_paths_are_those_of_projinfo_of_proj() {
    let Some(reference) = reference_projinfo() else {
        return;
    };
    let sandbox = Sandbox::new("reference");
    let separator = if cfg!(windows) { ";" } else { ":" };
    let list = format!("/a/b{separator}/c");

    compare(&sandbox, &reference, Compared::UserDirectory, &|_| {});
    compare(&sandbox, &reference, Compared::Whole, &|command| {
        command.env("PROJ_DATA", "/some/data");
    });
    compare(&sandbox, &reference, Compared::Whole, &|command| {
        command.env("PROJ_DATA", &list);
    });
    compare(&sandbox, &reference, Compared::Whole, &|command| {
        command.env("PROJ_LIB", "/old/lib");
    });
    compare(&sandbox, &reference, Compared::Whole, &|command| {
        command
            .env("PROJ_DATA", "/some/data")
            .env("PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY", "YES");
    });
    compare(&sandbox, &reference, Compared::UserDirectory, &|command| {
        command.env("PROJ_USER_WRITABLE_DIRECTORY", "/chosen");
    });
    if !cfg!(windows) {
        compare(&sandbox, &reference, Compared::UserDirectory, &|command| {
            command.env("XDG_DATA_HOME", "/xdg");
        });
        compare(&sandbox, &reference, Compared::UserDirectory, &|command| {
            command.env("XDG_DATA_HOME", "");
        });
        compare(&sandbox, &reference, Compared::UserDirectory, &|command| {
            command.env("HOME", "/no/such/home");
        });
        compare(&sandbox, &reference, Compared::UserDirectory, &|command| {
            command.env_remove("HOME");
        });
        compare(&sandbox, &reference, Compared::UserDirectory, &|command| {
            command.env("HOME", "/");
        });
    }
}
