//! `PJ_CONTEXT`: the state of a context, its error, its log, and the functions of
//! `proj.h` that create, copy and set it (`ctx.cpp`, `log.cpp`, `strerrno.cpp`).

use std::ffi::{c_char, c_int, c_void};
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;

use proj_rs::unstable::{
    Context as EngineContext, DatabaseContext, DatabaseSource, InitRules, ProjBuilder,
};

use crate::ffi::*;
use crate::pj::Pj;
use crate::util::{self, guard, Arena};

/// The state behind a `PJ_CONTEXT *`.
pub(crate) struct Ctx {
    pub(crate) errno: c_int,
    pub(crate) debug_level: c_int,
    pub(crate) logger: PJ_LOG_FUNCTION,
    pub(crate) logger_data: *mut c_void,
    pub(crate) builder: ProjBuilder,
    pub(crate) engine: EngineContext,
    pub(crate) file_finder: proj_file_finder,
    pub(crate) finder_data: *mut c_void,
    pub(crate) search_paths: Vec<PathBuf>,
    pub(crate) arena: Arena,
    pub(crate) database: Option<DatabaseContext>,
    pub(crate) autoclose_database: bool,
    pub(crate) init_rules: InitRules,
}

impl std::fmt::Debug for Ctx {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Ctx")
            .field("errno", &self.errno)
            .finish_non_exhaustive()
    }
}

/// The log of a default context: stderr, one message per line (`pj_stderr_logger`).
extern "C" fn stderr_logger(_data: *mut c_void, _level: c_int, message: *const c_char) {
    if let Some(text) = util::text(message) {
        let _ignored = writeln!(std::io::stderr(), "{text}");
    }
}

fn debug_level_from_environment() -> c_int {
    let Ok(value) = std::env::var("PROJ_DEBUG") else {
        return PJ_LOG_ERROR as c_int;
    };
    if value.eq_ignore_ascii_case("ON") {
        PJ_LOG_DEBUG as c_int
    } else if value.eq_ignore_ascii_case("OFF") {
        PJ_LOG_ERROR as c_int
    } else if let Ok(level) = value.trim().parse::<c_int>() {
        level
            .max(-(PJ_LOG_TRACE as c_int))
            .min(PJ_LOG_TRACE as c_int)
    } else {
        PJ_LOG_ERROR as c_int
    }
}

impl Ctx {
    fn fresh() -> Self {
        Self {
            errno: 0,
            debug_level: debug_level_from_environment(),
            logger: Some(stderr_logger),
            logger_data: std::ptr::null_mut(),
            builder: ProjBuilder::new(),
            engine: EngineContext::new(),
            file_finder: None,
            finder_data: std::ptr::null_mut(),
            search_paths: Vec::new(),
            arena: Arena::default(),
            database: None,
            autoclose_database: false,
            init_rules: EngineContext::default_init_rules(),
        }
    }

    /// A context that starts with the settings of `other` (`pj_ctx(const pj_ctx &)`), and its
    /// own errno.
    fn copy_of(other: &Self) -> Self {
        Self {
            errno: 0,
            debug_level: other.debug_level,
            logger: other.logger,
            logger_data: other.logger_data,
            builder: other.builder.clone(),
            engine: other.engine.clone(),
            file_finder: other.file_finder,
            finder_data: other.finder_data,
            search_paths: other.search_paths.clone(),
            arena: Arena::default(),
            database: None,
            autoclose_database: other.autoclose_database,
            init_rules: other.init_rules,
        }
    }

    /// The database of this context, opened on first use.
    pub(crate) fn database(&mut self) -> Option<DatabaseContext> {
        if self.database.is_none() {
            self.database = self.engine.database().ok();
        }
        self.database.clone()
    }

    /// The facade builder of this context, with the finder of the callback it holds.
    pub(crate) fn builder(&self) -> ProjBuilder {
        self.builder.clone()
    }

    /// Records `code` as the last error of this context, and logs `message` at error level.
    pub(crate) fn fail(&mut self, code: c_int, message: &str) {
        self.fail_bytes(code, message.as_bytes());
    }

    /// [`Self::fail`] with a message that need not be UTF-8: a WKT grammar error can cut a
    /// multi-byte character, and PROJ logs those bytes as they are.
    pub(crate) fn fail_bytes(&mut self, code: c_int, message: &[u8]) {
        self.errno = code;
        self.log_bytes(PJ_LOG_ERROR as c_int, message);
    }

    /// `proj_log_error` of `iso19111/c_api.cpp`: logs `message`, and sets `code` only when no
    /// code is left from deeper down (or from an earlier call).
    pub(crate) fn fail_unless_set(&mut self, code: c_int, message: &str) {
        if self.errno == 0 {
            self.errno = code;
        }
        self.log(PJ_LOG_ERROR as c_int, message);
    }

    /// Sends `message` to the logger when `level` is active (`pj_log_active`).
    pub(crate) fn log(&self, level: c_int, message: &str) {
        self.log_bytes(level, message.as_bytes());
    }

    /// [`Self::log`] of bytes, which need not be UTF-8.
    pub(crate) fn log_bytes(&self, level: c_int, message: &[u8]) {
        let mut debug = self.debug_level;
        if debug < 0 && self.errno == 0 {
            return;
        }
        if debug < 0 {
            debug = -debug;
        }
        if level > debug {
            return;
        }
        if let Some(logger) = self.logger {
            let line = util::c_bytes(message);
            // SAFETY: the logger is the function the application registered with
            // `proj_log_func`, called with its own data pointer and a valid C string.
            unsafe { logger(self.logger_data, level, line.as_ptr()) };
        }
    }

    /// Points the engine finder at the callback of this context.
    fn install_finder(&mut self) {
        let Some(callback) = self.file_finder else {
            self.builder.clear_file_finder();
            return;
        };
        let owner = std::ptr::from_mut(self) as usize;
        let data = self.finder_data as usize;
        self.builder.set_file_finder(move |name: &str| {
            let request = util::c_string(name);
            let context = owner as *mut PJ_CONTEXT;
            // SAFETY: the callback and its data were registered together with
            // `proj_context_set_file_finder`; the context outlives the objects that use it
            // (proj.h contract).
            let found = unsafe { callback(context, request.as_ptr(), data as *mut c_void) };
            util::text(found).map(PathBuf::from)
        });
    }
}

struct DefaultContext(usize);

/// The default context, shared by every call with a `NULL` context.
pub(crate) fn default_context() -> *mut Ctx {
    static DEFAULT: OnceLock<DefaultContext> = OnceLock::new();
    DEFAULT
        .get_or_init(|| {
            let mut context = Box::new(Ctx::fresh());
            context.install_finder();
            DefaultContext(Box::into_raw(context) as usize)
        })
        .0 as *mut Ctx
}

/// The context behind a `PJ_CONTEXT *`: the default one for `NULL`.
pub(crate) fn context<'a>(pointer: *const PJ_CONTEXT) -> &'a mut Ctx {
    let raw = if pointer.is_null() {
        default_context()
    } else {
        pointer.cast_mut().cast::<Ctx>()
    };
    // The pointer is the default context, or one made by `proj_context_create`.
    util::deref_mut(raw).unwrap_or_else(|| {
        // Unreachable: `default_context` never returns NULL.
        std::process::abort()
    })
}

/// The context of an object, the default one for `NULL`.
pub(crate) fn context_of_pj<'a>(pj: *const PJ) -> &'a mut Ctx {
    match util::deref(pj.cast::<Pj>()) {
        Some(object) => context(object.ctx.cast::<PJ_CONTEXT>()),
        None => context(std::ptr::null()),
    }
}

fn hand_over(context: Ctx) -> *mut PJ_CONTEXT {
    let pointer = util::leak(context);
    // SAFETY of the later `install_finder`: the box does not move.
    if let Some(created) = util::deref_mut(pointer) {
        created.install_finder();
    }
    pointer.cast::<PJ_CONTEXT>()
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_context_create() -> *mut PJ_CONTEXT {
    guard(std::ptr::null_mut(), || {
        hand_over(Ctx::copy_of(context(std::ptr::null())))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_context_destroy(ctx: *mut PJ_CONTEXT) -> *mut PJ_CONTEXT {
    guard(std::ptr::null_mut(), || {
        if ctx.is_null() || ctx.cast::<Ctx>() == default_context() {
            return std::ptr::null_mut();
        }
        util::release(ctx.cast::<Ctx>());
        std::ptr::null_mut()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_context_clone(ctx: *mut PJ_CONTEXT) -> *mut PJ_CONTEXT {
    guard(std::ptr::null_mut(), || {
        if ctx.is_null() {
            return proj_context_create();
        }
        hand_over(Ctx::copy_of(context(ctx)))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_context_set_file_finder(
    ctx: *mut PJ_CONTEXT,
    finder: proj_file_finder,
    user_data: *mut c_void,
) {
    guard((), || {
        let ctx = context(ctx);
        ctx.file_finder = finder;
        ctx.finder_data = user_data;
        ctx.install_finder();
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_context_set_search_paths(
    ctx: *mut PJ_CONTEXT,
    count_paths: c_int,
    paths: *const *const c_char,
) {
    guard((), || {
        let ctx = context(ctx);
        let count = usize::try_from(count_paths).unwrap_or(0);
        let list: Vec<PathBuf> = util::slice(paths, count)
            .iter()
            .filter_map(|path| util::text(*path))
            .map(PathBuf::from)
            .collect();
        ctx.builder.replace_search_paths(list.clone());
        ctx.search_paths = list;
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_context_set_ca_bundle_path(ctx: *mut PJ_CONTEXT, path: *const c_char) {
    guard((), || {
        let ctx = context(ctx);
        if let Some(path) = util::text(path) {
            let _ignored = ctx.builder.set_ca_bundle_path(path);
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_context_use_proj4_init_rules(ctx: *mut PJ_CONTEXT, enable: c_int) {
    guard((), || {
        let rules = if enable != 0 {
            InitRules::Proj4
        } else {
            InitRules::Iso19111
        };
        let ctx = context(ctx);
        ctx.init_rules = rules;
        ctx.engine.set_init_rules(rules);
        ctx.builder.set_init_rules(rules);
        if ctx_is_default(ctx) {
            EngineContext::set_default_init_rules(rules);
        }
    });
}

fn ctx_is_default(ctx: &Ctx) -> bool {
    std::ptr::eq(ctx, default_context())
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_context_get_use_proj4_init_rules(
    ctx: *mut PJ_CONTEXT,
    from_legacy_code_path: c_int,
) -> c_int {
    guard(0, || {
        let ctx = context(ctx);
        let on = ctx.init_rules == InitRules::Proj4;
        if on {
            return 1;
        }
        if from_legacy_code_path != 0 {
            return c_int::from(
                std::env::var("PROJ_USE_PROJ4_INIT_RULES").is_ok_and(|value| {
                    value.eq_ignore_ascii_case("YES")
                        || value.eq_ignore_ascii_case("ON")
                        || value.eq_ignore_ascii_case("TRUE")
                }),
            );
        }
        0
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_context_set_sqlite3_vfs_name(ctx: *mut PJ_CONTEXT, name: *const c_char) {
    guard((), || {
        let ctx = context(ctx);
        let name = util::text(name);
        ctx.engine.set_sqlite_vfs_name(name);
        if ctx_is_default(ctx) {
            EngineContext::set_default_sqlite_vfs_name(name);
        }
        ctx.database = None;
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_context_set_autoclose_database(ctx: *mut PJ_CONTEXT, autoclose: c_int) {
    guard((), || {
        context(ctx).autoclose_database = autoclose != 0;
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_context_get_user_writable_directory(
    ctx: *mut PJ_CONTEXT,
    create: c_int,
) -> *const c_char {
    guard(std::ptr::null(), || {
        let ctx = context(ctx);
        if create != 0 {
            let _ignored = ctx.builder.create_user_directory();
        }
        let directory = ctx.builder.user_writable_directory();
        ctx.arena.put(&directory.to_string_lossy())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_context_set_user_writable_directory(
    ctx: *mut PJ_CONTEXT,
    path: *const c_char,
    create: c_int,
) {
    guard((), || {
        let ctx = context(ctx);
        match util::text(path) {
            Some(path) => {
                ctx.builder.set_user_directory_path(PathBuf::from(path));
                if create != 0 {
                    let _ignored = ctx.builder.create_user_directory();
                }
            }
            None => ctx.builder.clear_user_writable_directory(),
        }
    });
}

/// `DatabaseSource` of a path given by C.
pub(crate) fn database_source(path: Option<&str>) -> DatabaseSource {
    path.map_or(DatabaseSource::Embedded, |path| {
        DatabaseSource::File(PathBuf::from(path))
    })
}

// ---------------------------------------------------------------------------
// Errors (`ctx.cpp`, `strerrno.cpp`).

#[unsafe(no_mangle)]
pub extern "C" fn proj_context_errno(ctx: *mut PJ_CONTEXT) -> c_int {
    guard(0, || context(ctx).errno)
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_errno(p: *const PJ) -> c_int {
    guard(0, || context_of_pj(p).errno)
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_errno_set(p: *const PJ, err: c_int) -> c_int {
    guard(0, || {
        if err == 0 {
            return 0;
        }
        context_of_pj(p).errno = err;
        err
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_errno_restore(p: *const PJ, err: c_int) -> c_int {
    guard(0, || {
        if err == 0 {
            return 0;
        }
        proj_errno_set(p, err);
        0
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_errno_reset(p: *const PJ) -> c_int {
    guard(0, || {
        let ctx = context_of_pj(p);
        std::mem::replace(&mut ctx.errno, 0)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_context_errno_string(ctx: *mut PJ_CONTEXT, err: c_int) -> *const c_char {
    guard(std::ptr::null(), || {
        let ctx = context(ctx);
        let message = proj_rs::unstable::errno::error_string(err);
        ctx.arena.put_opt(message.as_deref())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_errno_string(err: c_int) -> *const c_char {
    proj_context_errno_string(std::ptr::null_mut(), err)
}

// ---------------------------------------------------------------------------
// Log (`log.cpp`).

#[unsafe(no_mangle)]
pub extern "C" fn proj_log_level(ctx: *mut PJ_CONTEXT, log_level: PJ_LOG_LEVEL) -> PJ_LOG_LEVEL {
    guard(PJ_LOG_TELL, || {
        let ctx = context(ctx);
        let previous = ctx.debug_level.unsigned_abs();
        if log_level != PJ_LOG_TELL {
            ctx.debug_level = log_level as c_int;
        }
        previous
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_log_func(
    ctx: *mut PJ_CONTEXT,
    app_data: *mut c_void,
    logf: PJ_LOG_FUNCTION,
) {
    guard((), || {
        let ctx = context(ctx);
        ctx.logger_data = app_data;
        if logf.is_some() {
            ctx.logger = logf;
        }
    });
}
