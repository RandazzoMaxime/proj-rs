//! PROJ 9.9.0 `test/unit/test_network.cpp`, `networking.file_api` (line 2043).
//!
//! The C++ case sets a `PROJ_FILE_API` with its ten callbacks, downloads `dk_sdfe_dvr90.tif` from
//! `https://cdn.proj.org` through them, then reads the grid through them with the network off. It
//! returns at once when the CDN does not answer (`networkAccessOK`), as `download_whole_files`.
//!
//! The port's file API is [`proj_rs::FileApi`]: `open`, `exists`, `mkdir`, `unlink` and `rename`
//! on the builder, and the reads, writes and seeks on the [`proj_rs::ApiFile`] that `open` returns.
//! Each callback here is `std::fs` and records its name, which replaces the C++ `in_*` flags. The
//! close of a file is its drop, as `close_cbk` is PROJ's.
//!
//! Every value is PROJ 9.9.0's: the sequence of the calls was measured with `libproj` 9.9.0 (TIFF
//! and curl, a C probe outside the repository): the download calls `open`, `exists`, `mkdir`,
//! `write`, `close`, `unlink` and `rename` and no `read`; the grid load then seeks and reads. The
//! value of `z` is the one `download_whole_files` checks, on the same file.
//!
//! The C++ case sets `PROJ_FULL_FILE_CHUNK_SIZE` to 30000 and the user-writable directory to a
//! directory that does not exist yet, so that PROJ creates it through the API. The port does the
//! same with an absolute directory in the system temporary directory.

#[path = "net_cdn/mod.rs"]
mod net_cdn;

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use proj_rs::{ApiFile, FileAccess, FileApi, ProjBuilder, TtlSetting};

const GRID: &str = "dk_sdfe_dvr90.tif";
const PIPELINE: &str = "+proj=pipeline \
    +step +proj=unitconvert +xy_in=deg +xy_out=rad \
    +step +proj=vgridshift +grids=dk_sdfe_dvr90.tif +multiplier=1 \
    +step +proj=unitconvert +xy_in=rad +xy_out=deg";

/// `z` of PROJ 9.9.0 at lon 12, lat 56 (`EXPECT_NEAR` with 1e-10).
const EXPECTED_Z: f64 = 36.5909996032715;

/// The callbacks that were called, by name: `open`, `read`, `write`, `seek`, `close`, `exists`,
/// `mkdir`, `unlink`, `rename`. The C++ flags `in_open` and the others are membership in this set.
#[derive(Debug, Default)]
struct Calls(Mutex<BTreeSet<&'static str>>);

impl Calls {
    fn mark(&self, callback: &'static str) {
        self.0
            .lock()
            .expect("the call log is not poisoned")
            .insert(callback);
    }

    fn seen(&self, callback: &'static str) -> bool {
        self.0
            .lock()
            .expect("the call log is not poisoned")
            .contains(&callback)
    }
}

/// The file API of the case: `std::fs` under each callback, which records its name.
struct RecordingApi {
    calls: Arc<Calls>,
}

impl FileApi for RecordingApi {
    fn open(&self, path: &Path, access: FileAccess) -> io::Result<Box<dyn ApiFile>> {
        self.calls.mark("open");
        let file = match access {
            FileAccess::ReadOnly => File::open(path)?,
            FileAccess::ReadUpdate => fs::OpenOptions::new().read(true).write(true).open(path)?,
            FileAccess::Create => File::options()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)?,
        };
        Ok(Box::new(RecordedFile {
            file,
            calls: Arc::clone(&self.calls),
        }))
    }

    fn exists(&self, path: &Path) -> bool {
        self.calls.mark("exists");
        path.exists()
    }

    fn mkdir(&self, path: &Path) -> io::Result<()> {
        self.calls.mark("mkdir");
        fs::create_dir(path)
    }

    fn unlink(&self, path: &Path) -> io::Result<()> {
        self.calls.mark("unlink");
        fs::remove_file(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.calls.mark("rename");
        fs::rename(from, to)
    }
}

/// An open file of [`RecordingApi`]: each read, write and seek is recorded, and dropping it is the
/// `close` of PROJ.
struct RecordedFile {
    file: File,
    calls: Arc<Calls>,
}

impl Read for RecordedFile {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.calls.mark("read");
        self.file.read(buffer)
    }
}

impl Write for RecordedFile {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.calls.mark("write");
        self.file.write(buffer)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

impl Seek for RecordedFile {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.calls.mark("seek");
        self.file.seek(position)
    }
}

impl Drop for RecordedFile {
    fn drop(&mut self) {
        self.calls.mark("close");
    }
}

#[test]
fn file_api() {
    if !net_cdn::cdn_reachable() {
        eprintln!("file_api: the CDN does not answer, the case returns as PROJ does");
        return;
    }
    // CI sets PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY; the case clears it, as the C++ case does, so
    // that the user-writable directory is read and created.
    std::env::set_var("PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY", "");
    let dir = std::env::temp_dir().join(format!("proj-rs-file-api-{}", std::process::id()));
    let _gone = fs::remove_dir_all(&dir);
    std::env::set_var("PROJ_USER_WRITABLE_DIRECTORY", &dir);
    std::env::set_var("PROJ_FULL_FILE_CHUNK_SIZE", "30000");

    // proj_context_set_enable_network(ctx, true), then proj_context_set_fileapi.
    let calls = Arc::new(Calls::default());
    let mut ctx = ProjBuilder::new();
    ctx.enable_network(true).expect("network on");
    ctx.set_file_api(Arc::new(RecordingApi {
        calls: Arc::clone(&calls),
    }));

    // proj_is_download_needed and proj_download_file, both through the API.
    assert!(ctx
        .is_download_needed(GRID, TtlSetting::Respect)
        .expect("proj_is_download_needed"));
    ctx.download_file(GRID, TtlSetting::Respect, None)
        .expect("proj_download_file");

    assert!(calls.seen("open"), "the download opens its temporary file");
    assert!(
        !calls.seen("read"),
        "the download reads nothing through the API"
    );
    assert!(calls.seen("write"), "the download writes through the API");
    assert!(calls.seen("close"), "the download closes its file");
    assert!(
        calls.seen("exists"),
        "the directory is checked through the API"
    );
    assert!(
        calls.seen("mkdir"),
        "the directory is created through the API"
    );
    assert!(
        calls.seen("unlink"),
        "the old file is deleted through the API"
    );
    assert!(
        calls.seen("rename"),
        "the temporary file is renamed through the API"
    );

    // proj_context_set_enable_network(ctx, false), then proj_create and proj_trans_generic.
    ctx.enable_network(false).expect("network off");
    let proj = ctx
        .clone()
        .proj(PIPELINE)
        .expect("proj_create of the vgridshift pipeline");
    let (_lon, _lat, z) = proj.convert_xyz(12.0, 56.0, 0.0).expect("transform");
    assert!((z - EXPECTED_Z).abs() <= 1e-10, "z = {z}");
    drop(proj);

    assert!(calls.seen("read"), "the grid is read through the API");
    assert!(calls.seen("seek"), "the grid is sought through the API");

    let _gone = fs::remove_dir_all(&dir);
}
