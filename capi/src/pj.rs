//! `PJ`: the object behind a `PJ *`, and the functions that create and destroy it.
//!
//! A `PJ` holds an engine [`Proj`]: a PROJ-string operation, a CRS, or a pair of CRS with the
//! operations between them. A pair transforms in the axis order of its CRS, as
//! `proj_create_crs_to_crs` does, until `proj_normalize_for_visualization` asks for the
//! order longitude, latitude (the engine transforms in that order).

use std::ffi::{c_char, c_int};

use proj_rs::iso19111::{
    coordinate_system_from_json, create_user_object, proj_based_operation_of_text,
    proj_string_refusal, CoordinateMetadata, FactoryError, UserObject, UNRECOGNIZED_USER_INPUT,
};
use proj_rs::unstable::{Area, Proj, ProjCreateError};

use crate::context::{context, Ctx};
use crate::ffi::*;
use crate::iso::Obj;
use crate::util::{self, guard, Arena};

/// The object behind a `PJ *`.
pub(crate) struct Pj {
    pub(crate) ctx: *mut Ctx,
    /// The engine object that transforms: an operation, a CRS, or a pair of CRS. `None` for an
    /// object that only has an ISO 19111 form (a datum, an ellipsoid, a coordinate system).
    pub(crate) proj: Option<Proj>,
    /// The ISO 19111 object, when this one has a model. A PROJ string has none.
    pub(crate) obj: Option<Obj>,
    /// `proj_normalize_for_visualization` was applied: the pair transforms in the order
    /// longitude or easting first.
    pub(crate) normalized: bool,
    /// A point went through `proj_trans`: `proj_trans_get_last_used_operation` has an answer.
    pub(crate) transformed: bool,
    /// The object was read from a text by `proj_create`: a PROJ string is answered by the engine.
    pub(crate) from_text: bool,
    /// How PROJ reads the text `proj_create` was given.
    pub(crate) read_as: ReadAs,
    pub(crate) arena: Arena,
}

/// How PROJ reads an object made from a text.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReadAs {
    /// The object `createFromUserInput` gives.
    Object,
    /// A PROJ string without `+type=crs`: an operation for PROJ, even where this port also
    /// builds a CRS for it.
    ProjOperation,
}

impl std::fmt::Debug for Pj {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("Pj").finish_non_exhaustive()
    }
}

impl Pj {
    pub(crate) fn new(ctx: *mut Ctx, proj: Proj) -> Self {
        Self {
            ctx,
            proj: Some(proj),
            obj: None,
            normalized: false,
            transformed: false,
            from_text: false,
            read_as: ReadAs::Object,
            arena: Arena::default(),
        }
    }

    /// An object with an ISO 19111 form, and the engine object when it has one.
    pub(crate) fn with_obj(ctx: *mut Ctx, obj: Obj, proj: Option<Proj>) -> Self {
        Self {
            ctx,
            proj,
            obj: Some(obj),
            normalized: false,
            transformed: false,
            from_text: false,
            read_as: ReadAs::Object,
            arena: Arena::default(),
        }
    }

    /// Whether the object is a coordinate operation the engine can run. An ISO object that is
    /// not an operation, a CRS read from a PROJ string included, does not transform
    /// (trans.cpp:227).
    pub(crate) fn is_transform(&self) -> bool {
        let iso_non_operation = self.read_as == ReadAs::Object
            && self
                .obj
                .as_ref()
                .is_some_and(|obj| obj.operation().is_none());
        !iso_non_operation && self.proj.as_ref().is_some_and(Proj::is_transform)
    }

    /// The context of this object.
    pub(crate) fn context<'a>(&self) -> &'a mut Ctx {
        context(self.ctx.cast::<PJ_CONTEXT>())
    }

    /// A copy of this object in `ctx`.
    pub(crate) fn copy_in(&self, ctx: *mut Ctx) -> Self {
        Self {
            ctx,
            proj: self.proj.clone(),
            obj: self.obj.clone(),
            normalized: self.normalized,
            transformed: false,
            from_text: false,
            read_as: self.read_as,
            arena: Arena::default(),
        }
    }
}

/// The object of a `PJ *`. `None` for `NULL`.
pub(crate) fn pj<'a>(pointer: *const PJ) -> Option<&'a mut Pj> {
    util::deref_mut(pointer.cast_mut().cast::<Pj>())
}

/// Gives `object` to C.
pub(crate) fn hand_over(object: Pj) -> *mut PJ {
    // `pj_obj_create` instantiates an operation with `pj_init`, which clears the errno of the
    // context (init.cpp:792); a CRS is not instantiated and keeps it.
    let instantiated = object.proj.is_some()
        && object
            .obj
            .as_ref()
            .is_none_or(|obj| obj.operation().is_some());
    if instantiated {
        object.context().errno = 0;
    }
    util::leak(object).cast::<PJ>()
}

/// The error code of a failed creation, and its text.
pub(crate) fn creation_failure(ctx: &mut Ctx, who: &str, error: &ProjCreateError) {
    ctx.fail(error.code(), &format!("{who}: {error}"));
}

/// `proj_create`: the object `createFromUserInput` reads, with the engine object that
/// transforms when the text has one.
pub(crate) fn create(ctx: *mut PJ_CONTEXT, definition: &str, who: &str) -> *mut PJ {
    let context = context(ctx);
    let pointer = std::ptr::from_mut(context);
    let database = context.database();
    let rules = context.init_rules;
    let proj_string = looks_like_proj_string(definition);
    if database.is_none() && !proj_string {
        context.fail(
            PROJ_ERR_OTHER as c_int,
            &format!("{who}: cannot open the database"),
        );
        return std::ptr::null_mut();
    }
    let init_step = proj_string && is_init_step(definition);
    let engine = context.builder().proj(definition);
    // PROJ instantiates a PROJ string before it makes the object (io.cpp:12799): an operation
    // or a CRS the engine cannot build in this context gives no object, and the code of
    // `pj_init` stays on the context. A success clears it. `init=epsg:` is read before.
    if proj_string && !init_step {
        let refusal = if names_type_crs(definition) {
            proj_string_refusal(definition)
                .or_else(|| proj_string_refusal(&crs_instantiation(definition)))
        } else {
            engine.as_ref().err().map(|error| {
                proj_string_refusal(definition).unwrap_or_else(|| (error.code(), error.to_string()))
            })
        };
        if let Some((code, text)) = refusal {
            context.fail(code, &format!("{who}: {text}"));
            return std::ptr::null_mut();
        }
        context.errno = 0;
    }
    if definition.trim_start().starts_with('{') {
        if let Ok(system) = coordinate_system_from_json(definition) {
            return hand_over(Pj::with_obj(pointer, Obj::Cs(system), None));
        }
    }
    // A PROJ string without `+type=crs` is a PROJ-based coordinate operation, not a CRS: PROJ's
    // createFromUserInput gives one (pyproj's `Proj` relies on its type).
    let user = match proj_based_operation_of_text(definition) {
        Some(Ok(operation)) => Ok(UserObject::Operation(operation)),
        _ => create_user_object(definition, database.as_ref(), rules),
    }
    .or_else(|error| match (&database, epoch_suffix(definition)) {
        // A name followed by `@epoch`, which the model reads for codes only.
        (Some(database), Some((name, epoch))) => {
            match create_user_object(name, Some(database), rules) {
                Ok(UserObject::Crs(crs)) => CoordinateMetadata::create(&crs, epoch, database)
                    .map(UserObject::CoordinateMetadata)
                    .map_err(|cause| {
                        FactoryError::Format(format!(
                            "CoordinateMetadata::create() failed with: {cause}"
                        ))
                    }),
                _ => Err(error),
            }
        }
        _ => Err(error),
    });
    match (user, engine) {
        (Ok(object), engine) => {
            let obj = Obj::User(object);
            let proj = engine.ok().or_else(|| obj.engine_proj(context));
            let mut made = Pj::with_obj(pointer, obj, proj);
            made.from_text = true;
            if proj_string && !names_type_crs(definition) {
                made.read_as = ReadAs::ProjOperation;
            }
            hand_over(made)
        }
        // A PROJ-based operation the model cannot read; a CRS refused by the model stays refused.
        (Err(_), Ok(proj)) if !init_step && !names_type_crs(definition) => {
            hand_over(Pj::new(pointer, proj))
        }
        (Err(error), _) => {
            let input = if proj_string {
                Input::ProjString
            } else {
                Input::Other
            };
            let (code, text) = user_input_failure(&error, input);
            context.fail_unless_set(code, &format!("{who}: {text}"));
            std::ptr::null_mut()
        }
    }
}

/// What `proj_create` was given: `createFromPROJString` reads a PROJ string.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Input {
    ProjString,
    Other,
}

/// The code and the text `proj_create` logs for an object `createFromUserInput` refuses
/// (iso19111/c_api.cpp:proj_create): `NoSuchAuthorityCodeException` gives PROJ_ERR_OTHER and
/// `<what>: AUTH:CODE`, a `ParsingException` PROJ_ERR_INVALID_OP_WRONG_SYNTAX. Past the
/// instantiation, a PROJ string the model refuses is PROJ_ERR_OTHER.
fn user_input_failure(error: &FactoryError, input: Input) -> (c_int, String) {
    let parsing = match error {
        FactoryError::NotFound {
            object,
            authority,
            code,
        } => {
            let text = format!(
                "{} not found: {authority}:{code}",
                object.to_ascii_lowercase()
            );
            return (PROJ_ERR_OTHER as c_int, text);
        }
        FactoryError::Format(text) => {
            input == Input::Other
                || text == UNRECOGNIZED_USER_INPUT
                || text.starts_with(INIT_SYNTAX_REFUSAL)
        }
        _ => input == Input::Other,
    };
    let code = if parsing {
        PROJ_ERR_INVALID_OP_WRONG_SYNTAX
    } else {
        PROJ_ERR_OTHER
    };
    (code as c_int, error.to_string())
}

/// `createFromPROJString` (io.cpp:12554) without the PROJ 4 `init=` rules.
const INIT_SYNTAX_REFUSAL: &str = "init=epsg:/init=IGNF: syntax not supported";

/// A PROJ string of one `init=epsg:` or `init=IGNF:` step, which PROJ reads before it
/// instantiates anything (io.cpp:12513-12527). An init file is instantiated as usual.
fn is_init_step(text: &str) -> bool {
    let words = || {
        text.split_whitespace()
            .map(|word| word.trim_start_matches('+'))
    };
    let code_init = |word: &str| {
        word.strip_prefix("init=").is_some_and(|name| {
            let lower = name.to_ascii_lowercase();
            lower.starts_with("epsg:") || lower.starts_with("ignf:")
        })
    };
    words().any(code_init) && !words().any(|word| word.starts_with("proj=") || word == "inv")
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_create(ctx: *mut PJ_CONTEXT, definition: *const c_char) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        match util::definition_text(definition) {
            Some(text) => create(ctx, &text, "proj_create"),
            None => std::ptr::null_mut(),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_create_argv(
    ctx: *mut PJ_CONTEXT,
    argc: c_int,
    argv: *mut *mut c_char,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let count = usize::try_from(argc).unwrap_or(0);
        let words: Vec<&str> = util::slice(argv.cast_const(), count)
            .iter()
            .filter_map(|word| util::text(word.cast_const()))
            .collect();
        if words.is_empty() {
            return std::ptr::null_mut();
        }
        create(ctx, &words.join(" "), "proj_create_argv")
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_destroy(p: *mut PJ) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        util::release(p.cast::<Pj>());
        std::ptr::null_mut()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_assign_context(p: *mut PJ, ctx: *mut PJ_CONTEXT) {
    guard((), || {
        if let Some(object) = pj(p) {
            object.ctx = std::ptr::from_mut(context(ctx));
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_clone(ctx: *mut PJ_CONTEXT, obj: *const PJ) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        match pj(obj) {
            Some(object) => hand_over(object.copy_in(std::ptr::from_mut(context))),
            None => {
                context.fail(
                    PROJ_ERR_OTHER_API_MISUSE as c_int,
                    "proj_clone: null object",
                );
                std::ptr::null_mut()
            }
        }
    })
}

// ---------------------------------------------------------------------------
// PJ_AREA.

/// The area of use a CRS-to-CRS transform is created for.
#[derive(Debug, Default)]
pub(crate) struct AreaOfUse {
    pub(crate) bbox: Option<Area>,
    pub(crate) name: Option<String>,
}

/// The area of a `PJ_AREA *`.
pub(crate) fn area<'a>(pointer: *const PJ_AREA) -> Option<&'a mut AreaOfUse> {
    util::deref_mut(pointer.cast_mut().cast::<AreaOfUse>())
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_area_create() -> *mut PJ_AREA {
    guard(std::ptr::null_mut(), || {
        util::leak(AreaOfUse::default()).cast::<PJ_AREA>()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_area_set_bbox(
    area_pointer: *mut PJ_AREA,
    west_lon_degree: f64,
    south_lat_degree: f64,
    east_lon_degree: f64,
    north_lat_degree: f64,
) {
    guard((), || {
        if let Some(target) = area(area_pointer) {
            target.bbox = Some(Area::new(
                west_lon_degree,
                south_lat_degree,
                east_lon_degree,
                north_lat_degree,
            ));
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_area_set_name(area_pointer: *mut PJ_AREA, name: *const c_char) {
    guard((), || {
        if let Some(target) = area(area_pointer) {
            target.name = util::text(name).map(str::to_owned);
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_area_destroy(area_pointer: *mut PJ_AREA) {
    guard((), || util::release(area_pointer.cast::<AreaOfUse>()));
}

/// A PROJ string, which names an operation and needs no database to be read.
/// `+type=crs` among the words of a PROJ string.
fn names_type_crs(text: &str) -> bool {
    text.split_whitespace().any(|word| {
        word.trim_start_matches('+')
            .eq_ignore_ascii_case("type=crs")
    })
}

/// The PROJ string `pj_init` instantiates for a CRS: without `type=crs`, and with
/// `+disable_grid_presence_check` (io.cpp:12799), so `nadgrids` and `geoidgrids` are not opened.
fn crs_instantiation(text: &str) -> String {
    text.split_whitespace()
        .filter(|word| {
            let bare = word.trim_start_matches('+');
            !bare.eq_ignore_ascii_case("type=crs")
                && !bare.starts_with("nadgrids=")
                && !bare.starts_with("geoidgrids=")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn looks_like_proj_string(text: &str) -> bool {
    let bare = text.trim_start().trim_start_matches('+');
    // `createFromUserInput` (io.cpp): a `proj=` or `init=` word anywhere makes a PROJ string,
    // `step proj=merc` included.
    bare.starts_with("proj=")
        || bare.starts_with("init=")
        || bare.starts_with("title=")
        || [" +proj=", " proj=", " +init=", " init="]
            .iter()
            .any(|needle| text.contains(needle))
}

/// `name@epoch`: the text before the last `@`, and the decimal year after it.
fn epoch_suffix(text: &str) -> Option<(&str, f64)> {
    let (name, epoch) = text.trim().rsplit_once('@')?;
    Some((name, epoch.trim().parse().ok()?))
}
