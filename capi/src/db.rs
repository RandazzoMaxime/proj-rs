//! The database: opening it, reading objects and lists from it, and reading WKT
//! (`iso19111/c_api.cpp`: `proj_create_from_database` and the `*_from_database` family).

use std::ffi::{c_char, c_int};
use std::path::PathBuf;

use proj_rs::iso19111::{
    AlternativeGridNames, AuthorityFactory, CoordinateOperation, DeprecatedCodes, NameMatch,
    ObjectType, UserObject, WktDialect, WktObject, WktParser, WktStrictness,
};
use proj_rs::unstable::Datum;

use crate::context::{context, database_source, Ctx};
use crate::ffi::*;
use crate::iso::Obj;
use crate::pj::{hand_over, pj, Pj};
use crate::util::{self, guard};

/// PROJ writes a missing object as `crs not found: EPSG:1`, the model as `CRS not found (EPSG:1)`.
fn proj_wording(message: &str) -> String {
    match message
        .strip_suffix(')')
        .and_then(|head| head.split_once(" not found ("))
    {
        Some((kind, code)) => format!("{} not found: {code}", kind.to_lowercase()),
        None => message.to_owned(),
    }
}

fn fail(context: &mut Ctx, who: &str, message: &str) {
    let message = &proj_wording(message);
    context.fail(PROJ_ERR_OTHER as c_int, &format!("{who}: {message}"));
}

/// The object of the database `category` calls for, as an ISO object.
fn object_from_database(
    context: &mut Ctx,
    auth_name: &str,
    code: &str,
    category: PJ_CATEGORY,
    names: AlternativeGridNames,
) -> Result<UserObject, String> {
    let database = context.database().ok_or("cannot open the database")?;
    let factory = AuthorityFactory::create(&database, auth_name);
    let error = |error: proj_rs::iso19111::FactoryError| error.to_string();
    Ok(match category {
        PJ_CATEGORY_ELLIPSOID => {
            UserObject::Ellipsoid(factory.create_ellipsoid(code).map_err(error)?)
        }
        PJ_CATEGORY_PRIME_MERIDIAN => {
            UserObject::PrimeMeridian(factory.create_prime_meridian(code).map_err(error)?)
        }
        PJ_CATEGORY_DATUM => match factory.create_datum(code).map_err(error)? {
            Datum::Geodetic(frame) => UserObject::GeodeticDatum(frame),
            Datum::Vertical(frame) => UserObject::VerticalDatum(frame),
            Datum::Engineering(datum) => UserObject::EngineeringDatum(datum),
        },
        PJ_CATEGORY_DATUM_ENSEMBLE => {
            UserObject::Ensemble(factory.create_datum_ensemble(code).map_err(error)?)
        }
        PJ_CATEGORY_CRS => UserObject::Crs(
            factory
                .create_coordinate_reference_system(code)
                .map_err(error)?,
        ),
        PJ_CATEGORY_COORDINATE_OPERATION => UserObject::Operation(
            factory
                .create_coordinate_operation(code, names)
                .map_err(error)?,
        ),
        _ => return Err("unknown category".to_owned()),
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_create_from_database(
    ctx: *mut PJ_CONTEXT,
    auth_name: *const c_char,
    code: *const c_char,
    category: PJ_CATEGORY,
    usePROJAlternativeGridNames: c_int,
    options: *const *const c_char,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        let _ = options;
        let (Some(authority), Some(code)) = (util::text(auth_name), util::text(code)) else {
            context.fail(
                PROJ_ERR_OTHER_API_MISUSE as c_int,
                "proj_create_from_database: missing authority or code",
            );
            return std::ptr::null_mut();
        };
        let names = if usePROJAlternativeGridNames != 0 {
            AlternativeGridNames::Proj
        } else {
            AlternativeGridNames::Official
        };
        match object_from_database(context, authority, code, category, names) {
            Ok(object) => {
                let obj = Obj::User(object);
                let proj = obj.engine_proj(context);
                hand_over(Pj::with_obj(std::ptr::from_mut(context), obj, proj))
            }
            Err(message) => {
                fail(context, "proj_create_from_database", &message);
                std::ptr::null_mut()
            }
        }
    })
}

// ---------------------------------------------------------------------------
// The database of a context.

#[unsafe(no_mangle)]
pub extern "C" fn proj_context_set_database_path(
    ctx: *mut PJ_CONTEXT,
    dbPath: *const c_char,
    auxDbPaths: *const *const c_char,
    options: *const *const c_char,
) -> c_int {
    guard(0, || {
        let context = context(ctx);
        let _ = options;
        let mut auxiliary = Vec::new();
        if !auxDbPaths.is_null() {
            let mut index = 0;
            while let Some(item) = util::get(auxDbPaths.wrapping_add(index)) {
                let Some(path) = util::text(item) else {
                    break;
                };
                auxiliary.push(PathBuf::from(path));
                index += 1;
            }
        }
        let previous_source = context.engine.database_source().clone();
        let previous_auxiliary = context.engine.auxiliary_database_paths().to_vec();
        // `proj_context_get_database_path` names the proj.db of the search path, which stands
        // for the embedded copy: giving that path back selects the embedded database again.
        let requested = util::text(dbPath).filter(|path| {
            proj_data_database(context).is_none_or(|advertised| advertised != PathBuf::from(path))
        });
        context.engine.set_database(database_source(requested));
        context.engine.set_auxiliary_database_paths(auxiliary);
        context.database = None;
        if context.database().is_some() {
            return 1;
        }
        context.engine.set_database(previous_source);
        context
            .engine
            .set_auxiliary_database_paths(previous_auxiliary);
        context.database = None;
        fail(
            context,
            "proj_context_set_database_path",
            "cannot open the database",
        );
        0
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_context_get_database_path(ctx: *mut PJ_CONTEXT) -> *const c_char {
    guard(std::ptr::null(), || {
        let context = context(ctx);
        let path = match context.engine.database_path() {
            Some(path) => Some(path.to_path_buf()),
            None => proj_data_database(context),
        };
        match path {
            Some(path) => context.arena.put(&path.to_string_lossy()),
            None => std::ptr::null(),
        }
    })
}

/// `proj.db` as PROJ's search finds it (`pj_open_lib_internal`: the paths of the context, or the
/// user directory, `PROJ_DATA` and `../share/proj`), which names the database a PROJ context
/// opens by default. This port embeds its own copy and reads the file only when it is named.
fn proj_data_database(context: &Ctx) -> Option<PathBuf> {
    context.builder.find_local_file("proj.db")
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_context_get_database_metadata(
    ctx: *mut PJ_CONTEXT,
    key: *const c_char,
) -> *const c_char {
    guard(std::ptr::null(), || {
        let context = context(ctx);
        let (Some(key), Some(database)) = (util::text(key), context.database()) else {
            return std::ptr::null();
        };
        match database.metadata(key) {
            Ok(Some(value)) => context.arena.put(&value),
            _ => std::ptr::null(),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_context_get_database_structure(
    ctx: *mut PJ_CONTEXT,
    options: *const *const c_char,
) -> PROJ_STRING_LIST {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        let _ = options;
        match context
            .database()
            .map(|database| database.database_structure())
        {
            Some(Ok(statements)) => util::string_list(&statements),
            _ => std::ptr::null_mut(),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_context_guess_wkt_dialect(
    ctx: *mut PJ_CONTEXT,
    wkt: *const c_char,
) -> PJ_GUESSED_WKT_DIALECT {
    guard(PJ_GUESSED_NOT_WKT, || {
        let _ = context(ctx);
        let Some(text) = util::definition_text(wkt) else {
            return PJ_GUESSED_NOT_WKT;
        };
        match WktParser::guess_dialect(&text) {
            WktDialect::Wkt1Gdal => PJ_GUESSED_WKT1_GDAL,
            WktDialect::Wkt1Esri => PJ_GUESSED_WKT1_ESRI,
            WktDialect::Wkt2_2015 => PJ_GUESSED_WKT2_2015,
            WktDialect::Wkt2_2019 => PJ_GUESSED_WKT2_2019,
            WktDialect::NotWkt => PJ_GUESSED_NOT_WKT,
        }
    })
}

// ---------------------------------------------------------------------------
// WKT and names.

fn user_object_of(parsed: WktObject) -> Option<UserObject> {
    Some(match parsed {
        WktObject::Ellipsoid(value) => UserObject::Ellipsoid(value),
        WktObject::GeodeticDatum(value) => UserObject::GeodeticDatum(value),
        WktObject::VerticalDatum(value) => UserObject::VerticalDatum(value),
        WktObject::EngineeringDatum(value) => UserObject::EngineeringDatum(value),
        WktObject::TemporalDatum(value) => UserObject::TemporalDatum(value),
        WktObject::Ensemble(value) => UserObject::Ensemble(value),
        WktObject::Crs(value) => UserObject::Crs(value),
        WktObject::CoordinateMetadata(value) => UserObject::CoordinateMetadata(value),
        WktObject::Conversion(value) => {
            UserObject::Operation(CoordinateOperation::Conversion(value))
        }
        WktObject::Transformation(value) => {
            UserObject::Operation(CoordinateOperation::Transformation(value))
        }
        WktObject::Concatenated(value) => {
            UserObject::Operation(CoordinateOperation::Concatenated(value))
        }
        _ => return None,
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_create_from_wkt(
    ctx: *mut PJ_CONTEXT,
    wkt: *const c_char,
    options: *const *const c_char,
    out_warnings: *mut PROJ_STRING_LIST,
    out_grammar_errors: *mut PROJ_STRING_LIST,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        util::put(out_warnings, std::ptr::null_mut());
        util::put(out_grammar_errors, std::ptr::null_mut());
        let Some(text) = util::definition_text(wkt) else {
            return std::ptr::null_mut();
        };
        let mut parser = WktParser::new();
        if let Some(database) = context.database() {
            parser.attach_database(database);
        }
        for (key, value) in crate::iso::options_of(options) {
            match key.as_str() {
                "STRICT" => {
                    parser.set_strictness(if value.eq_ignore_ascii_case("YES") {
                        WktStrictness::Strict
                    } else {
                        WktStrictness::Lenient
                    });
                }
                "UNSET_IDENTIFIERS_IF_INCOMPATIBLE_DEF" => {
                    if value.eq_ignore_ascii_case("NO") {
                        parser.keep_incompatible_identifiers();
                    }
                }
                other => {
                    fail(
                        context,
                        "proj_create_from_wkt",
                        &format!("Unknown option :{other}"),
                    );
                    return std::ptr::null_mut();
                }
            }
        }
        let parsed = parser.create_from_wkt(&text);
        if !parser.warnings().is_empty() {
            util::put(out_warnings, util::string_list(parser.warnings()));
        }
        if !parser.grammar_errors().is_empty() {
            util::put(
                out_grammar_errors,
                util::string_list(parser.grammar_errors()),
            );
        }
        match parsed.ok().and_then(user_object_of) {
            Some(object) => {
                let obj = Obj::User(object);
                let proj = obj.engine_proj(context);
                hand_over(Pj::with_obj(std::ptr::from_mut(context), obj, proj))
            }
            None => {
                fail(context, "proj_create_from_wkt", "cannot read this WKT");
                std::ptr::null_mut()
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Lists of names and codes.

fn object_type_of(kind: PJ_TYPE) -> Option<ObjectType> {
    Some(match kind {
        PJ_TYPE_ELLIPSOID => ObjectType::Ellipsoid,
        PJ_TYPE_PRIME_MERIDIAN => ObjectType::PrimeMeridian,
        PJ_TYPE_GEODETIC_REFERENCE_FRAME => ObjectType::GeodeticReferenceFrame,
        PJ_TYPE_DYNAMIC_GEODETIC_REFERENCE_FRAME => ObjectType::DynamicGeodeticReferenceFrame,
        PJ_TYPE_VERTICAL_REFERENCE_FRAME => ObjectType::VerticalReferenceFrame,
        PJ_TYPE_DYNAMIC_VERTICAL_REFERENCE_FRAME => ObjectType::DynamicVerticalReferenceFrame,
        PJ_TYPE_DATUM_ENSEMBLE => ObjectType::DatumEnsemble,
        PJ_TYPE_CRS => ObjectType::Crs,
        PJ_TYPE_GEODETIC_CRS => ObjectType::GeodeticCrs,
        PJ_TYPE_GEOCENTRIC_CRS => ObjectType::GeocentricCrs,
        PJ_TYPE_GEOGRAPHIC_CRS => ObjectType::GeographicCrs,
        PJ_TYPE_GEOGRAPHIC_2D_CRS => ObjectType::Geographic2dCrs,
        PJ_TYPE_GEOGRAPHIC_3D_CRS => ObjectType::Geographic3dCrs,
        PJ_TYPE_VERTICAL_CRS => ObjectType::VerticalCrs,
        PJ_TYPE_PROJECTED_CRS => ObjectType::ProjectedCrs,
        PJ_TYPE_COMPOUND_CRS => ObjectType::CompoundCrs,
        PJ_TYPE_ENGINEERING_CRS => ObjectType::EngineeringCrs,
        PJ_TYPE_OTHER_CRS => ObjectType::Crs,
        PJ_TYPE_CONVERSION => ObjectType::Conversion,
        PJ_TYPE_TRANSFORMATION => ObjectType::Transformation,
        PJ_TYPE_CONCATENATED_OPERATION => ObjectType::ConcatenatedOperation,
        PJ_TYPE_OTHER_COORDINATE_OPERATION => ObjectType::CoordinateOperation,
        PJ_TYPE_ENGINEERING_DATUM => ObjectType::EngineeringDatum,
        _ => return None,
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_get_authorities_from_database(ctx: *mut PJ_CONTEXT) -> PROJ_STRING_LIST {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        match context.database().map(|database| database.authorities()) {
            Some(Ok(list)) => util::string_list(&list),
            _ => std::ptr::null_mut(),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_get_codes_from_database(
    ctx: *mut PJ_CONTEXT,
    auth_name: *const c_char,
    type_: PJ_TYPE,
    allow_deprecated: c_int,
) -> PROJ_STRING_LIST {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        let (Some(authority), Some(kind), Some(database)) = (
            util::text(auth_name),
            object_type_of(type_),
            context.database(),
        ) else {
            return std::ptr::null_mut();
        };
        let deprecated = if allow_deprecated != 0 {
            DeprecatedCodes::Included
        } else {
            DeprecatedCodes::Excluded
        };
        match AuthorityFactory::create(&database, authority).get_authority_codes(kind, deprecated) {
            Ok(codes) => util::string_list(&codes.into_iter().collect::<Vec<_>>()),
            Err(error) => {
                fail(context, "proj_get_codes_from_database", &error.to_string());
                std::ptr::null_mut()
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_get_geoid_models_from_database(
    ctx: *mut PJ_CONTEXT,
    auth_name: *const c_char,
    code: *const c_char,
    options: *const *const c_char,
) -> PROJ_STRING_LIST {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        let _ = options;
        let (Some(authority), Some(code), Some(database)) =
            (util::text(auth_name), util::text(code), context.database())
        else {
            return std::ptr::null_mut();
        };
        match AuthorityFactory::create(&database, authority).get_geoid_models(code) {
            Ok(models) => util::string_list(&models),
            Err(error) => {
                fail(
                    context,
                    "proj_get_geoid_models_from_database",
                    &error.to_string(),
                );
                std::ptr::null_mut()
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_int_list_destroy(list: *mut c_int) {
    guard((), || util::release_array(list));
}

/// An object found by name, as the list `proj_create_from_name` returns.
#[unsafe(no_mangle)]
pub extern "C" fn proj_get_non_deprecated(
    ctx: *mut PJ_CONTEXT,
    obj: *const PJ,
) -> *mut PJ_OBJ_LIST {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        let Some(Obj::User(UserObject::Crs(crs))) = pj(obj).and_then(|object| object.obj.as_ref())
        else {
            return std::ptr::null_mut();
        };
        let Some(database) = context.database() else {
            return std::ptr::null_mut();
        };
        match crs.non_deprecated(&database) {
            Ok(list) => crate::list::object_list(
                context,
                list.into_iter()
                    .map(|crs| Obj::User(UserObject::Crs(crs)))
                    .collect(),
            ),
            Err(_) => std::ptr::null_mut(),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_create_from_name(
    ctx: *mut PJ_CONTEXT,
    auth_name: *const c_char,
    searchedName: *const c_char,
    types: *const PJ_TYPE,
    typesCount: usize,
    approximateMatch: c_int,
    limitResultCount: usize,
    options: *const *const c_char,
) -> *mut PJ_OBJ_LIST {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        let _ = options;
        let (Some(name), Some(database)) = (util::text(searchedName), context.database()) else {
            return std::ptr::null_mut();
        };
        let kinds: Vec<ObjectType> = util::slice(types, typesCount)
            .iter()
            .filter_map(|kind| object_type_of(*kind))
            .collect();
        let mode = if approximateMatch != 0 {
            NameMatch::Approximate
        } else {
            NameMatch::Exact
        };
        let authority = util::text(auth_name).unwrap_or("");
        let found = AuthorityFactory::create(&database, authority).create_objects_from_name(
            name,
            &kinds,
            mode,
            limitResultCount,
        );
        match found {
            Ok(objects) => crate::list::object_list(
                context,
                objects
                    .into_iter()
                    .filter_map(crate::list::from_authority)
                    .collect(),
            ),
            Err(error) => {
                fail(context, "proj_create_from_name", &error.to_string());
                std::ptr::null_mut()
            }
        }
    })
}
