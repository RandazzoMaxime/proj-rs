//! Lists read from the database (CRS, units, celestial bodies), the information on a unit
//! and a grid, and the insert statements (`iso19111/c_api.cpp`).

use std::ffi::{c_char, c_int};

use proj_rs::iso19111::{
    AuthorityFactory, AuthorityObject, CodeForm, ComponentCodes, InsertSession, UserObject,
};
use proj_rs::unstable::Datum;

use crate::context::context;
use crate::ffi::*;
use crate::iso::Obj;
use crate::pj::pj;
use crate::util::{self, guard};

/// A C string for a field of a structure given to C: owned by the structure, freed with it.
fn field(text: &str) -> *mut c_char {
    util::give_string(text)
}

fn crs_type_of(name: &str) -> PJ_TYPE {
    match name {
        "geographic 2D" => PJ_TYPE_GEOGRAPHIC_2D_CRS,
        "geographic 3D" => PJ_TYPE_GEOGRAPHIC_3D_CRS,
        "geocentric" => PJ_TYPE_GEOCENTRIC_CRS,
        "other" => PJ_TYPE_GEODETIC_CRS,
        "projected" => PJ_TYPE_PROJECTED_CRS,
        "vertical" => PJ_TYPE_VERTICAL_CRS,
        "compound" => PJ_TYPE_COMPOUND_CRS,
        _ => PJ_TYPE_CRS,
    }
}

/// An array of pointers ended by `NULL`, given to C.
fn give_array<T>(items: Vec<*mut T>) -> *mut *mut T {
    let mut all = items;
    all.push(std::ptr::null_mut());
    Box::into_raw(all.into_boxed_slice()).cast::<*mut T>()
}

/// Takes back an array made by [`give_array`] and hands each element to `free`.
fn take_array<T>(list: *mut *mut T, free: impl Fn(*mut T)) {
    if list.is_null() {
        return;
    }
    let mut count = 0_usize;
    while util::get(list.wrapping_add(count).cast_const()).is_some_and(|item| !item.is_null()) {
        count += 1;
    }
    let whole = std::ptr::slice_from_raw_parts_mut(list, count + 1);
    // SAFETY: the array was made by `give_array`: `count` elements and the NULL terminator.
    let items = unsafe { &*whole };
    for item in items.iter().take(count) {
        free(*item);
    }
    // SAFETY: `whole` is the boxed slice `give_array` leaked.
    drop(unsafe { Box::from_raw(whole) });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_get_crs_list_parameters_create() -> *mut PROJ_CRS_LIST_PARAMETERS {
    guard(std::ptr::null_mut(), || {
        util::leak(PROJ_CRS_LIST_PARAMETERS {
            types: std::ptr::null(),
            typesCount: 0,
            crs_area_of_use_contains_bbox: 1,
            bbox_valid: 0,
            west_lon_degree: 0.0,
            south_lat_degree: 0.0,
            east_lon_degree: 0.0,
            north_lat_degree: 0.0,
            allow_deprecated: 0,
            celestial_body_name: std::ptr::null(),
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_get_crs_list_parameters_destroy(params: *mut PROJ_CRS_LIST_PARAMETERS) {
    guard((), || util::release(params));
}

fn type_accepted(wanted: &[PJ_TYPE], found: PJ_TYPE) -> bool {
    wanted.is_empty()
        || wanted.iter().any(|kind| {
            *kind == found
                || (*kind == PJ_TYPE_GEOGRAPHIC_CRS
                    && (found == PJ_TYPE_GEOGRAPHIC_2D_CRS || found == PJ_TYPE_GEOGRAPHIC_3D_CRS))
                || (*kind == PJ_TYPE_GEODETIC_CRS
                    && (found == PJ_TYPE_GEOCENTRIC_CRS
                        || found == PJ_TYPE_GEOGRAPHIC_2D_CRS
                        || found == PJ_TYPE_GEOGRAPHIC_3D_CRS))
        })
}

/// Whether the area of use of a CRS passes the box filter of the parameters.
fn area_accepted(params: &PROJ_CRS_LIST_PARAMETERS, info: &proj_rs::iso19111::CrsInfo) -> bool {
    if params.bbox_valid == 0 {
        return true;
    }
    let (Some(west), Some(south), Some(east), Some(north)) =
        (info.west_lon, info.south_lat, info.east_lon, info.north_lat)
    else {
        return false;
    };
    let (qw, qs, qe, qn) = (
        params.west_lon_degree,
        params.south_lat_degree,
        params.east_lon_degree,
        params.north_lat_degree,
    );
    let contains = params.crs_area_of_use_contains_bbox != 0;
    if west <= east && qw <= qe {
        if contains {
            !(qw < west || qe > east || qs < south || qn > north)
        } else {
            !(east < qw || west > qe || north < qs || south > qn)
        }
    } else {
        // An area or a query across the antimeridian: compare the two ranges of longitudes.
        let spans = |from: f64, to: f64| -> Vec<(f64, f64)> {
            if from <= to {
                vec![(from, to)]
            } else {
                vec![(from, 180.0), (-180.0, to)]
            }
        };
        let crs_spans = spans(west, east);
        let query_spans = spans(qw, qe);
        let lat_ok = if contains {
            qs >= south && qn <= north
        } else {
            !(north < qs || south > qn)
        };
        let lon_ok = if contains {
            query_spans
                .iter()
                .all(|q| crs_spans.iter().any(|c| q.0 >= c.0 && q.1 <= c.1))
        } else {
            query_spans
                .iter()
                .any(|q| crs_spans.iter().any(|c| !(c.1 < q.0 || c.0 > q.1)))
        };
        lat_ok && lon_ok
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_get_crs_info_list_from_database(
    ctx: *mut PJ_CONTEXT,
    auth_name: *const c_char,
    params: *const PROJ_CRS_LIST_PARAMETERS,
    out_result_count: *mut c_int,
) -> *mut *mut PROJ_CRS_INFO {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        util::put(out_result_count, 0);
        let Some(database) = context.database() else {
            return std::ptr::null_mut();
        };
        let authority = util::text(auth_name).unwrap_or("");
        let found = match AuthorityFactory::create(&database, authority).get_crs_info_list() {
            Ok(list) => list,
            Err(error) => {
                context.fail(
                    PROJ_ERR_OTHER as c_int,
                    &format!("proj_get_crs_info_list_from_database: {error}"),
                );
                return std::ptr::null_mut();
            }
        };
        let wanted: Vec<PJ_TYPE> = util::deref(params)
            .map(|parameters| util::slice(parameters.types, parameters.typesCount).to_vec())
            .unwrap_or_default();
        let body =
            util::deref(params).and_then(|parameters| util::text(parameters.celestial_body_name));
        let mut items = Vec::new();
        for info in &found {
            let kind = crs_type_of(&info.type_name);
            if !type_accepted(&wanted, kind) {
                continue;
            }
            if let Some(parameters) = util::deref(params) {
                if parameters.allow_deprecated == 0 && info.deprecated {
                    continue;
                }
                if !area_accepted(parameters, info) {
                    continue;
                }
            }
            if body.is_some_and(|name| name != info.celestial_body_name) {
                continue;
            }
            let bbox = info
                .west_lon
                .zip(info.south_lat)
                .zip(info.east_lon)
                .zip(info.north_lat);
            let (west, south, east, north) =
                bbox.map_or((0.0, 0.0, 0.0, 0.0), |(((w, s), e), n)| (w, s, e, n));
            items.push(util::leak(PROJ_CRS_INFO {
                auth_name: field(&info.authority),
                code: field(&info.code),
                name: field(&info.name),
                type_: kind,
                deprecated: c_int::from(info.deprecated),
                bbox_valid: c_int::from(bbox.is_some()),
                west_lon_degree: west,
                south_lat_degree: south,
                east_lon_degree: east,
                north_lat_degree: north,
                area_name: field(&info.area_name),
                projection_method_name: if info.projection_method_name.is_empty() {
                    std::ptr::null_mut()
                } else {
                    field(&info.projection_method_name)
                },
                celestial_body_name: field(&info.celestial_body_name),
            }));
        }
        util::put(out_result_count, c_int::try_from(items.len()).unwrap_or(0));
        give_array(items)
    })
}

fn free_text(pointer: *mut c_char) {
    util::take_string(pointer);
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_crs_info_list_destroy(list: *mut *mut PROJ_CRS_INFO) {
    guard((), || {
        take_array(list, |item| {
            if let Some(info) = util::deref(item.cast_const()) {
                free_text(info.auth_name);
                free_text(info.code);
                free_text(info.name);
                free_text(info.area_name);
                free_text(info.projection_method_name);
                free_text(info.celestial_body_name);
            }
            util::release(item);
        });
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_get_units_from_database(
    ctx: *mut PJ_CONTEXT,
    auth_name: *const c_char,
    category: *const c_char,
    allow_deprecated: c_int,
    out_result_count: *mut c_int,
) -> *mut *mut PROJ_UNIT_INFO {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        util::put(out_result_count, 0);
        let Some(database) = context.database() else {
            return std::ptr::null_mut();
        };
        let authority = util::text(auth_name).unwrap_or("");
        let Ok(found) = AuthorityFactory::create(&database, authority).get_unit_list() else {
            return std::ptr::null_mut();
        };
        let wanted = util::text(category);
        let mut items = Vec::new();
        for info in found {
            if wanted.is_some_and(|name| name != info.category) {
                continue;
            }
            if allow_deprecated == 0 && info.deprecated {
                continue;
            }
            items.push(util::leak(PROJ_UNIT_INFO {
                auth_name: field(&info.authority),
                code: field(&info.code),
                name: field(&info.name),
                category: field(&info.category),
                conv_factor: info.conv_factor,
                proj_short_name: if info.proj_short_name.is_empty() {
                    std::ptr::null_mut()
                } else {
                    field(&info.proj_short_name)
                },
                deprecated: c_int::from(info.deprecated),
            }));
        }
        util::put(out_result_count, c_int::try_from(items.len()).unwrap_or(0));
        give_array(items)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_unit_list_destroy(list: *mut *mut PROJ_UNIT_INFO) {
    guard((), || {
        take_array(list, |item| {
            if let Some(info) = util::deref(item.cast_const()) {
                free_text(info.auth_name);
                free_text(info.code);
                free_text(info.name);
                free_text(info.category);
                free_text(info.proj_short_name);
            }
            util::release(item);
        });
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_get_celestial_body_list_from_database(
    ctx: *mut PJ_CONTEXT,
    auth_name: *const c_char,
    out_result_count: *mut c_int,
) -> *mut *mut PROJ_CELESTIAL_BODY_INFO {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        util::put(out_result_count, 0);
        let Some(database) = context.database() else {
            return std::ptr::null_mut();
        };
        let authority = util::text(auth_name).unwrap_or("");
        let Ok(found) = AuthorityFactory::create(&database, authority).get_celestial_body_list()
        else {
            return std::ptr::null_mut();
        };
        let items: Vec<_> = found
            .iter()
            .map(|info| {
                util::leak(PROJ_CELESTIAL_BODY_INFO {
                    auth_name: field(&info.authority),
                    name: field(&info.name),
                })
            })
            .collect();
        util::put(out_result_count, c_int::try_from(items.len()).unwrap_or(0));
        give_array(items)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_celestial_body_list_destroy(list: *mut *mut PROJ_CELESTIAL_BODY_INFO) {
    guard((), || {
        take_array(list, |item| {
            if let Some(info) = util::deref(item.cast_const()) {
                free_text(info.auth_name);
                free_text(info.name);
            }
            util::release(item);
        });
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_uom_get_info_from_database(
    ctx: *mut PJ_CONTEXT,
    auth_name: *const c_char,
    code: *const c_char,
    out_name: *mut *const c_char,
    out_conv_factor: *mut f64,
    out_category: *mut *const c_char,
) -> c_int {
    guard(0, || {
        let context = context(ctx);
        util::put(out_name, std::ptr::null());
        util::put(out_conv_factor, 0.0);
        util::put(out_category, std::ptr::null());
        let (Some(authority), Some(code), Some(database)) =
            (util::text(auth_name), util::text(code), context.database())
        else {
            return 0;
        };
        match AuthorityFactory::create(&database, authority).create_unit_of_measure(code) {
            Ok(unit) => {
                util::put(out_name, context.arena.put(unit.name()));
                util::put(out_conv_factor, unit.conversion_to_si());
                util::put(
                    out_category,
                    context
                        .arena
                        .put(crate::inspect::category_name(unit.kind())),
                );
                1
            }
            Err(error) => {
                context.fail(
                    PROJ_ERR_OTHER as c_int,
                    &format!("proj_uom_get_info_from_database: {error}"),
                );
                0
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_grid_get_info_from_database(
    ctx: *mut PJ_CONTEXT,
    grid_name: *const c_char,
    out_full_name: *mut *const c_char,
    out_package_name: *mut *const c_char,
    out_url: *mut *const c_char,
    out_direct_download: *mut c_int,
    out_open_license: *mut c_int,
    out_available: *mut c_int,
) -> c_int {
    guard(0, || {
        let context = context(ctx);
        let (Some(name), Some(database)) = (util::text(grid_name), context.database()) else {
            return 0;
        };
        match context.builder.within(|| database.grid_info(name)) {
            Ok(Some(info)) => {
                util::put(out_full_name, context.arena.put(&info.full_name));
                util::put(out_package_name, context.arena.put(&info.package_name));
                util::put(out_url, context.arena.put(&info.url));
                util::put(out_direct_download, c_int::from(info.direct_download));
                util::put(out_open_license, c_int::from(info.open_license));
                util::put(out_available, c_int::from(info.available));
                1
            }
            _ => 0,
        }
    })
}

// ---------------------------------------------------------------------------
// Insert statements.

/// The object behind a `PJ_INSERT_SESSION *`.
#[derive(Debug)]
struct Session {
    inner: InsertSession,
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_insert_object_session_create(
    ctx: *mut PJ_CONTEXT,
) -> *mut PJ_INSERT_SESSION {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        let Some(database) = context.database() else {
            return std::ptr::null_mut();
        };
        match InsertSession::start(&database) {
            Ok(inner) => util::leak(Session { inner }).cast::<PJ_INSERT_SESSION>(),
            Err(error) => {
                context.fail(
                    PROJ_ERR_OTHER as c_int,
                    &format!("proj_insert_object_session_create: {error}"),
                );
                std::ptr::null_mut()
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_insert_object_session_destroy(
    ctx: *mut PJ_CONTEXT,
    session: *mut PJ_INSERT_SESSION,
) {
    guard((), || {
        let _ = context(ctx);
        util::release(session.cast::<Session>());
    });
}

/// The authority object an ISO object stands for in the insert functions.
fn authority_object_of(object: &Obj) -> Option<AuthorityObject> {
    Some(match object {
        Obj::User(user) => match user {
            UserObject::Crs(value) => AuthorityObject::Crs(value.clone()),
            UserObject::Operation(value) => AuthorityObject::Operation(value.clone()),
            UserObject::GeodeticDatum(value) => {
                AuthorityObject::Datum(Datum::Geodetic(value.clone()))
            }
            UserObject::VerticalDatum(value) => {
                AuthorityObject::Datum(Datum::Vertical(value.clone()))
            }
            UserObject::EngineeringDatum(value) => {
                AuthorityObject::Datum(Datum::Engineering(value.clone()))
            }
            UserObject::Ensemble(value) => AuthorityObject::DatumEnsemble(value.clone()),
            UserObject::PrimeMeridian(value) => AuthorityObject::PrimeMeridian(value.clone()),
            UserObject::Ellipsoid(value) => AuthorityObject::Ellipsoid(value.clone()),
            _ => return None,
        },
        Obj::Cs(_) => return None,
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_suggests_code_for(
    ctx: *mut PJ_CONTEXT,
    object: *const PJ,
    authority: *const c_char,
    numeric_code: c_int,
    options: *const *const c_char,
) -> *mut c_char {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        let _ = options;
        let (Some(object), Some(authority), Some(database)) =
            (pj(object), util::text(authority), context.database())
        else {
            return std::ptr::null_mut();
        };
        let Some(target) = object.obj.as_ref().and_then(authority_object_of) else {
            return std::ptr::null_mut();
        };
        let form = if numeric_code != 0 {
            CodeForm::Numeric
        } else {
            CodeForm::FromName
        };
        match proj_rs::iso19111::suggests_code_for(&database, &target, authority, form) {
            Ok(code) => util::give_string(&code),
            Err(error) => {
                context.fail(
                    PROJ_ERR_OTHER as c_int,
                    &format!("proj_suggests_code_for: {error}"),
                );
                std::ptr::null_mut()
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_get_insert_statements(
    ctx: *mut PJ_CONTEXT,
    session: *mut PJ_INSERT_SESSION,
    object: *const PJ,
    authority: *const c_char,
    code: *const c_char,
    numeric_codes: c_int,
    allowed_authorities: *const *const c_char,
    options: *const *const c_char,
) -> PROJ_STRING_LIST {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        let _ = options;
        let (Some(object), Some(authority), Some(code), Some(database)) = (
            pj(object),
            util::text(authority),
            util::text(code),
            context.database(),
        ) else {
            return std::ptr::null_mut();
        };
        let Some(target) = object.obj.as_ref().and_then(authority_object_of) else {
            return std::ptr::null_mut();
        };
        let mut allowed = Vec::new();
        if !allowed_authorities.is_null() {
            let mut index = 0;
            while let Some(item) =
                util::get(allowed_authorities.wrapping_add(index)).and_then(util::text)
            {
                allowed.push(item.to_owned());
                index += 1;
            }
        } else {
            allowed.extend(["EPSG".to_owned(), "PROJ".to_owned()]);
            allowed.push(authority.to_owned());
        }
        let components = if numeric_codes != 0 {
            ComponentCodes::Numeric
        } else {
            ComponentCodes::FromParentCode
        };
        let statements = match util::deref(session.cast_const().cast::<Session>()) {
            Some(open) => open
                .inner
                .statements_for(&target, authority, code, components, &allowed),
            None => proj_rs::iso19111::insert_statements(
                &database, &target, authority, code, components, &allowed,
            ),
        };
        match statements {
            Ok(list) => util::string_list(&list),
            Err(error) => {
                context.fail(
                    PROJ_ERR_OTHER as c_int,
                    &format!("proj_get_insert_statements: {error}"),
                );
                std::ptr::null_mut()
            }
        }
    })
}
