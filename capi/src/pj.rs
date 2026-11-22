//! `PJ`: the object behind a `PJ *`, and the functions that create and destroy it.
//!
//! A `PJ` holds an engine [`Proj`]: a PROJ-string operation, a CRS, or a pair of CRS with the
//! operations between them. A pair transforms in the axis order of its CRS, as
//! `proj_create_crs_to_crs` does, until `proj_normalize_for_visualization` asks for the
//! order longitude, latitude (the engine transforms in that order).

use std::ffi::{c_char, c_int};

use proj_rs::iso19111::{
    coordinate_system_from_json, create_user_object, proj_based_operation_of_text,
    CoordinateMetadata, UserObject,
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

    /// Whether the object is a coordinate operation the engine can run.
    pub(crate) fn is_transform(&self) -> bool {
        self.proj.as_ref().is_some_and(Proj::is_transform)
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
                    .map_err(|_| error),
                _ => Err(error),
            }
        }
        _ => Err(error),
    });
    let engine = context.builder().proj(definition);
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
        (Err(_), Ok(proj)) => hand_over(Pj::new(pointer, proj)),
        (Err(error), Err(create_error)) => {
            let code = create_error.code();
            context.fail(code, &format!("{who}: {error}"));
            std::ptr::null_mut()
        }
    }
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

fn looks_like_proj_string(text: &str) -> bool {
    let bare = text.trim_start().trim_start_matches('+');
    bare.starts_with("proj=") || bare.starts_with("init=") || bare.starts_with("title=")
}

/// `name@epoch`: the text before the last `@`, and the decimal year after it.
fn epoch_suffix(text: &str) -> Option<(&str, f64)> {
    let (name, epoch) = text.trim().rsplit_once('@')?;
    Some((name, epoch.trim().parse().ok()?))
}
