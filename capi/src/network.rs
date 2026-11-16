//! Network access, the grid cache and file download of a context (`networkfilemanager.cpp`,
//! `filemanager.cpp`).

use std::ffi::{c_char, c_int, c_void};

use proj_rs::unstable::{GridCacheTtl, TtlSetting};

use crate::context::context;
use crate::ffi::*;
use crate::util::{self, guard};

fn ttl_of(ignore_ttl_setting: c_int) -> TtlSetting {
    if ignore_ttl_setting != 0 {
        TtlSetting::Ignore
    } else {
        TtlSetting::Respect
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_context_set_enable_network(ctx: *mut PJ_CONTEXT, enabled: c_int) -> c_int {
    guard(0, || {
        let context = context(ctx);
        context
            .builder
            .enable_network(enabled != 0)
            .map_or(0, c_int::from)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_context_is_network_enabled(ctx: *mut PJ_CONTEXT) -> c_int {
    guard(0, || c_int::from(context(ctx).builder.network_enabled()))
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_context_set_url_endpoint(ctx: *mut PJ_CONTEXT, url: *const c_char) {
    guard((), || {
        if let Some(url) = util::text(url) {
            let _ignored = context(ctx).builder.set_url_endpoint(url);
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_context_get_url_endpoint(ctx: *mut PJ_CONTEXT) -> *const c_char {
    guard(std::ptr::null(), || {
        let context = context(ctx);
        match context.builder.get_url_endpoint() {
            Ok(endpoint) => context.arena.put(&endpoint),
            Err(_) => std::ptr::null(),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_grid_cache_set_enable(ctx: *mut PJ_CONTEXT, enabled: c_int) {
    guard((), || {
        let _ignored = context(ctx).builder.grid_cache_enable(enabled != 0);
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_grid_cache_set_filename(ctx: *mut PJ_CONTEXT, fullname: *const c_char) {
    guard((), || {
        if let Some(path) = util::text(fullname) {
            let _ignored = context(ctx).builder.set_grid_cache_path(path);
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_grid_cache_set_max_size(ctx: *mut PJ_CONTEXT, max_size_MB: c_int) {
    guard((), || {
        let size = u32::try_from(max_size_MB).ok();
        let _ignored = context(ctx).builder.set_grid_cache_max_size_mb(size);
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_grid_cache_set_ttl(ctx: *mut PJ_CONTEXT, ttl_seconds: c_int) {
    guard((), || {
        let ttl = u32::try_from(ttl_seconds).map_or(GridCacheTtl::Never, GridCacheTtl::Seconds);
        let _ignored = context(ctx).builder.set_grid_cache_ttl(ttl);
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_grid_cache_clear(ctx: *mut PJ_CONTEXT) {
    guard((), || {
        let _ignored = context(ctx).builder.clear_grid_cache();
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_is_download_needed(
    ctx: *mut PJ_CONTEXT,
    url_or_filename: *const c_char,
    ignore_ttl_setting: c_int,
) -> c_int {
    guard(0, || {
        let context = context(ctx);
        let Some(name) = util::text(url_or_filename) else {
            return 0;
        };
        match context
            .builder
            .is_download_needed(name, ttl_of(ignore_ttl_setting))
        {
            Ok(needed) => c_int::from(needed),
            Err(error) => {
                context.fail(error.code(), &format!("proj_is_download_needed: {error}"));
                0
            }
        }
    })
}

/// The progress callback of `proj_download_file`.
type Progress = Option<unsafe extern "C" fn(*mut PJ_CONTEXT, f64, *mut c_void) -> c_int>;

#[unsafe(no_mangle)]
pub extern "C" fn proj_download_file(
    ctx: *mut PJ_CONTEXT,
    url_or_filename: *const c_char,
    ignore_ttl_setting: c_int,
    progress_cbk: Progress,
    user_data: *mut c_void,
) -> c_int {
    guard(0, || {
        let context = context(ctx);
        let Some(name) = util::text(url_or_filename) else {
            return 0;
        };
        let owner = std::ptr::from_mut(context).cast::<PJ_CONTEXT>();
        let mut report = |_: &proj_rs::unstable::ProjBuilder, fraction: f64| match progress_cbk {
            // SAFETY: the callback and its data were given together by the caller of
            // `proj_download_file`, which keeps them valid for the call.
            Some(callback) => unsafe { callback(owner, fraction, user_data) != 0 },
            None => true,
        };
        let progress: Option<&mut proj_rs::unstable::DownloadProgress<'_>> = Some(&mut report);
        match context
            .builder
            .download_file(name, ttl_of(ignore_ttl_setting), progress)
        {
            Ok(()) => 1,
            Err(error) => {
                context.fail(error.code(), &format!("proj_download_file: {error}"));
                0
            }
        }
    })
}
