//! The callbacks of an application: its file API and its network (`proj_context_set_fileapi`,
//! `proj_context_set_network_callbacks`), adapted to the `FileApi` and `NetworkCallbacks` traits
//! of the engine.

use std::ffi::{c_char, c_int, c_void, CString};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::Arc;

use proj_rs::unstable::{
    ApiFile, FileAccess, FileApi, NetworkCallbacks, NetworkError, NetworkHandle, OpenReply,
};

use crate::context::context;
use crate::ffi::*;
use crate::util::{self, guard};

/// The room an error message of a callback gets, as `proj.h` documents.
const ERROR_SIZE: usize = 1024;

fn text_of(buffer: &[u8]) -> String {
    let end = buffer
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(buffer.len());
    String::from_utf8_lossy(&buffer[..end]).into_owned()
}

fn c_path(path: &Path) -> CString {
    util::c_string(&path.to_string_lossy())
}

// ---------------------------------------------------------------------------
// File API.

/// The file callbacks of the application, with the pointers they work on as addresses: the
/// application guarantees that they are valid from any thread that uses the context.
#[derive(Clone, Copy)]
struct CFileApi {
    api: PROJ_FILE_API,
    user_data: usize,
    ctx: usize,
}

struct CFile {
    owner: CFileApi,
    handle: usize,
}

fn io_error(message: &str) -> io::Error {
    io::Error::other(message.to_owned())
}

impl FileApi for CFileApi {
    fn open(&self, path: &Path, access: FileAccess) -> io::Result<Box<dyn ApiFile>> {
        let open = self
            .api
            .open_cbk
            .ok_or_else(|| io_error("no open callback"))?;
        let mode = match access {
            FileAccess::ReadOnly => PROJ_OPEN_ACCESS_READ_ONLY,
            FileAccess::ReadUpdate => PROJ_OPEN_ACCESS_READ_UPDATE,
            FileAccess::Create => PROJ_OPEN_ACCESS_CREATE,
        };
        let name = c_path(path);
        // SAFETY: the callback is the application's, called with the context and data it gave
        // and a valid C string.
        let handle = unsafe {
            open(
                self.ctx as *mut PJ_CONTEXT,
                name.as_ptr(),
                mode,
                self.user_data as *mut c_void,
            )
        };
        if handle.is_null() {
            return Err(io_error("the open callback failed"));
        }
        Ok(Box::new(CFile {
            owner: *self,
            handle: handle as usize,
        }))
    }

    fn exists(&self, path: &Path) -> bool {
        let Some(callback) = self.api.exists_cbk else {
            return false;
        };
        let name = c_path(path);
        // SAFETY: as in `open`.
        unsafe {
            callback(
                self.ctx as *mut PJ_CONTEXT,
                name.as_ptr(),
                self.user_data as *mut c_void,
            ) != 0
        }
    }

    fn mkdir(&self, path: &Path) -> io::Result<()> {
        self.simple(self.api.mkdir_cbk, path)
    }

    fn unlink(&self, path: &Path) -> io::Result<()> {
        self.simple(self.api.unlink_cbk, path)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        let callback = self
            .api
            .rename_cbk
            .ok_or_else(|| io_error("no rename callback"))?;
        let (old, new) = (c_path(from), c_path(to));
        // SAFETY: as in `open`.
        let done = unsafe {
            callback(
                self.ctx as *mut PJ_CONTEXT,
                old.as_ptr(),
                new.as_ptr(),
                self.user_data as *mut c_void,
            )
        };
        if done != 0 {
            Ok(())
        } else {
            Err(io_error("the rename callback failed"))
        }
    }
}

type PathCallback =
    Option<unsafe extern "C" fn(*mut PJ_CONTEXT, *const c_char, *mut c_void) -> c_int>;

impl CFileApi {
    fn simple(&self, callback: PathCallback, path: &Path) -> io::Result<()> {
        let callback = callback.ok_or_else(|| io_error("no callback"))?;
        let name = c_path(path);
        // SAFETY: as in `open`.
        let done = unsafe {
            callback(
                self.ctx as *mut PJ_CONTEXT,
                name.as_ptr(),
                self.user_data as *mut c_void,
            )
        };
        if done != 0 {
            Ok(())
        } else {
            Err(io_error("the callback failed"))
        }
    }
}

impl Read for CFile {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let callback = self
            .owner
            .api
            .read_cbk
            .ok_or_else(|| io_error("no read callback"))?;
        // SAFETY: the buffer is valid for `len` bytes; the rest is the application's callback.
        Ok(unsafe {
            callback(
                self.owner.ctx as *mut PJ_CONTEXT,
                self.handle as *mut PROJ_FILE_HANDLE,
                buffer.as_mut_ptr().cast::<c_void>(),
                buffer.len(),
                self.owner.user_data as *mut c_void,
            )
        })
    }
}

impl Write for CFile {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let callback = self
            .owner
            .api
            .write_cbk
            .ok_or_else(|| io_error("no write callback"))?;
        // SAFETY: as in `read`.
        Ok(unsafe {
            callback(
                self.owner.ctx as *mut PJ_CONTEXT,
                self.handle as *mut PROJ_FILE_HANDLE,
                buffer.as_ptr().cast::<c_void>(),
                buffer.len(),
                self.owner.user_data as *mut c_void,
            )
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Seek for CFile {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let seek = self
            .owner
            .api
            .seek_cbk
            .ok_or_else(|| io_error("no seek callback"))?;
        let tell = self
            .owner
            .api
            .tell_cbk
            .ok_or_else(|| io_error("no tell callback"))?;
        let (offset, whence) = match position {
            SeekFrom::Start(at) => (i64::try_from(at).unwrap_or(i64::MAX), 0),
            SeekFrom::Current(by) => (by, 1),
            SeekFrom::End(by) => (by, 2),
        };
        // SAFETY: the application's callbacks, called with the data it gave.
        let moved = unsafe {
            seek(
                self.owner.ctx as *mut PJ_CONTEXT,
                self.handle as *mut PROJ_FILE_HANDLE,
                offset,
                whence,
                self.owner.user_data as *mut c_void,
            )
        };
        if moved == 0 {
            return Err(io_error("the seek callback failed"));
        }
        // SAFETY: as above.
        Ok(unsafe {
            tell(
                self.owner.ctx as *mut PJ_CONTEXT,
                self.handle as *mut PROJ_FILE_HANDLE,
                self.owner.user_data as *mut c_void,
            )
        })
    }
}

impl Drop for CFile {
    fn drop(&mut self) {
        if let Some(close) = self.owner.api.close_cbk {
            // SAFETY: the handle comes from the open callback of the same application.
            unsafe {
                close(
                    self.owner.ctx as *mut PJ_CONTEXT,
                    self.handle as *mut PROJ_FILE_HANDLE,
                    self.owner.user_data as *mut c_void,
                );
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_context_set_fileapi(
    ctx: *mut PJ_CONTEXT,
    fileapi: *const PROJ_FILE_API,
    user_data: *mut c_void,
) -> c_int {
    guard(0, || {
        let context = context(ctx);
        let Some(api) = util::get(fileapi) else {
            return 0;
        };
        let complete = api.open_cbk.is_some()
            && api.read_cbk.is_some()
            && api.write_cbk.is_some()
            && api.seek_cbk.is_some()
            && api.tell_cbk.is_some()
            && api.close_cbk.is_some()
            && api.exists_cbk.is_some()
            && api.mkdir_cbk.is_some()
            && api.unlink_cbk.is_some()
            && api.rename_cbk.is_some();
        if api.version != 1 || !complete {
            context.fail(
                PROJ_ERR_OTHER_API_MISUSE as c_int,
                "proj_context_set_fileapi: version must be 1 and every callback set",
            );
            return 0;
        }
        let adapter = CFileApi {
            api,
            user_data: user_data as usize,
            ctx: ctx as usize,
        };
        context.builder.set_file_api(Arc::new(adapter));
        1
    })
}

// ---------------------------------------------------------------------------
// Network.

#[derive(Clone, Copy)]
struct CNetwork {
    open: unsafe extern "C" fn(
        *mut PJ_CONTEXT,
        *const c_char,
        u64,
        usize,
        *mut c_void,
        *mut usize,
        usize,
        *mut c_char,
        *mut c_void,
    ) -> *mut PROJ_NETWORK_HANDLE,
    close: unsafe extern "C" fn(*mut PJ_CONTEXT, *mut PROJ_NETWORK_HANDLE, *mut c_void),
    header: unsafe extern "C" fn(
        *mut PJ_CONTEXT,
        *mut PROJ_NETWORK_HANDLE,
        *const c_char,
        *mut c_void,
    ) -> *const c_char,
    read_range: unsafe extern "C" fn(
        *mut PJ_CONTEXT,
        *mut PROJ_NETWORK_HANDLE,
        u64,
        usize,
        *mut c_void,
        usize,
        *mut c_char,
        *mut c_void,
    ) -> usize,
    user_data: usize,
    ctx: usize,
}

impl NetworkCallbacks for CNetwork {
    fn open(&self, url: &str, offset: u64, size_to_read: usize) -> Result<OpenReply, NetworkError> {
        let mut data = vec![0_u8; size_to_read];
        let mut read = 0_usize;
        let mut error = vec![0_u8; ERROR_SIZE];
        let address = util::c_string(url);
        // SAFETY: the buffers are valid for the sizes given; the callback is the application's.
        let handle = unsafe {
            (self.open)(
                self.ctx as *mut PJ_CONTEXT,
                address.as_ptr(),
                offset,
                size_to_read,
                data.as_mut_ptr().cast::<c_void>(),
                &raw mut read,
                ERROR_SIZE,
                error.as_mut_ptr().cast::<c_char>(),
                self.user_data as *mut c_void,
            )
        };
        if handle.is_null() {
            return Err(NetworkError::Open(text_of(&error)));
        }
        data.truncate(read.min(size_to_read));
        Ok(OpenReply {
            handle: NetworkHandle(handle as u64),
            data,
        })
    }

    fn header(&self, handle: NetworkHandle, name: &str) -> Option<String> {
        let wanted = util::c_string(name);
        // SAFETY: the handle comes from `open`; the callback is the application's.
        let found = unsafe {
            (self.header)(
                self.ctx as *mut PJ_CONTEXT,
                handle.0 as *mut PROJ_NETWORK_HANDLE,
                wanted.as_ptr(),
                self.user_data as *mut c_void,
            )
        };
        util::text(found).map(str::to_owned)
    }

    fn read_range(
        &self,
        handle: NetworkHandle,
        offset: u64,
        size_to_read: usize,
    ) -> Result<Vec<u8>, NetworkError> {
        let mut data = vec![0_u8; size_to_read];
        let mut error = vec![0_u8; ERROR_SIZE];
        // SAFETY: as in `open`.
        let read = unsafe {
            (self.read_range)(
                self.ctx as *mut PJ_CONTEXT,
                handle.0 as *mut PROJ_NETWORK_HANDLE,
                offset,
                size_to_read,
                data.as_mut_ptr().cast::<c_void>(),
                ERROR_SIZE,
                error.as_mut_ptr().cast::<c_char>(),
                self.user_data as *mut c_void,
            )
        };
        if read < size_to_read && error[0] != 0 {
            return Err(NetworkError::ReadRange(text_of(&error)));
        }
        data.truncate(read.min(size_to_read));
        Ok(data)
    }

    fn close(&self, handle: NetworkHandle) {
        // SAFETY: the handle comes from `open`; the callback is the application's.
        unsafe {
            (self.close)(
                self.ctx as *mut PJ_CONTEXT,
                handle.0 as *mut PROJ_NETWORK_HANDLE,
                self.user_data as *mut c_void,
            );
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_context_set_network_callbacks(
    ctx: *mut PJ_CONTEXT,
    open_cbk: proj_network_open_cbk_type,
    close_cbk: proj_network_close_cbk_type,
    get_header_value_cbk: proj_network_get_header_value_cbk_type,
    read_range_cbk: proj_network_read_range_type,
    user_data: *mut c_void,
) -> c_int {
    guard(0, || {
        let context = context(ctx);
        let (Some(open), Some(close), Some(header), Some(read_range)) =
            (open_cbk, close_cbk, get_header_value_cbk, read_range_cbk)
        else {
            context.fail(
                PROJ_ERR_OTHER_API_MISUSE as c_int,
                "proj_context_set_network_callbacks: a callback is missing",
            );
            return 0;
        };
        let adapter = CNetwork {
            // The callback types of proj.h use `unsigned long long`, which is `u64` here.
            open,
            close,
            header,
            read_range,
            user_data: user_data as usize,
            ctx: ctx as usize,
        };
        context.builder.set_network_callbacks(Arc::new(adapter));
        1
    })
}
