//! The file operations of an application, in place of `std::fs`: PROJ 9.9.0's
//! `proj_context_set_fileapi` (`PROJ_FILE_API`, `src/filemanager.cpp`).
//!
//! Once a [`FileApi`] is set on a [`ProjBuilder`](crate::ProjBuilder), the crate does these
//! operations through it: the existence checks of the grid search, the reads of grids and
//! dictionaries, the creation of the user-writable directory, and a download (create a temporary
//! file, write it, delete the old file, rename the temporary one). Without one, `std::fs` is used.
//!
//! The C API passes a `user_data` pointer with each callback. Here the state belongs to the
//! implementation of the trait.

use std::io::{self, Read, Seek, Write};
use std::path::Path;

/// How a file is opened: `PROJ_OPEN_ACCESS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FileAccess {
    /// An existing file, read only: the reads of grids and dictionaries.
    ReadOnly,
    /// An existing file, read and written in place.
    ReadUpdate,
    /// A file to write from its start, emptied when it exists: a download.
    Create,
}

/// An open file of a [`FileApi`]. It is closed when it is dropped, which is PROJ's `close_cbk`.
///
/// Every type that reads, writes and seeks is one, so a `std::fs::File` can be returned as it is.
/// A file opened [`FileAccess::ReadOnly`] may refuse `write` with an error.
pub trait ApiFile: Read + Write + Seek + Send {}

impl<T: Read + Write + Seek + Send> ApiFile for T {}

/// The file operations of a builder: PROJ's `PROJ_FILE_API`, with its ten callbacks.
///
/// The operations are called on the thread that builds, transforms or downloads. A failure is an
/// `Err` or `false`, as the application's callback reports it; PROJ logs it and goes on, and so
/// does the crate.
pub trait FileApi: Send + Sync {
    /// Open `path` with `access` (`open_cbk`).
    ///
    /// # Errors
    ///
    /// The error of the application's open. Nothing is read through the API then.
    fn open(&self, path: &Path, access: FileAccess) -> io::Result<Box<dyn ApiFile>>;

    /// Whether `path` exists, as a file or as a directory (`exists_cbk`).
    fn exists(&self, path: &Path) -> bool;

    /// Create the directory `path`, whose parent exists (`mkdir_cbk`).
    ///
    /// # Errors
    ///
    /// The error of the application's `mkdir`.
    fn mkdir(&self, path: &Path) -> io::Result<()>;

    /// Delete the file `path` (`unlink_cbk`).
    ///
    /// # Errors
    ///
    /// The error of the application's delete, such as a missing file.
    fn unlink(&self, path: &Path) -> io::Result<()>;

    /// Rename `from` to `to` (`rename_cbk`). PROJ deletes `to` before it renames onto it.
    ///
    /// # Errors
    ///
    /// The error of the application's rename.
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;
}
