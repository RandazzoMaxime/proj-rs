//! What a CRS is made of: its base, datum, ensemble, coordinate system, ellipsoid and prime
//! meridian (`proj_crs_get_*`, `proj_cs_*`, `proj_get_ellipsoid`...).

use std::ffi::{c_char, c_int};

use proj_rs::iso19111::{
    CoordinateOperation, CoordinateSystem, Crs, DatumEnsemble, Ellipsoid, GeodeticCRS,
    GeodeticReferenceFrame, PrimeMeridian, UnitKind, UnitOfMeasure, UserObject,
};

use crate::context::context;
use crate::ffi::*;
use crate::iso::{derived, Obj};
use crate::pj::{pj, Pj};
use crate::util::{self, guard};

fn base_of_projected(base: &GeodeticCRS) -> Crs {
    match base {
        GeodeticCRS::Geographic(geographic) => Crs::Geographic(geographic.clone()),
        other => Crs::Geodetic(other.clone()),
    }
}

/// `proj_get_source_crs`: the base of a bound, projected or derived CRS, the source of an
/// operation.
fn source_of(object: &Pj) -> Option<Crs> {
    if let Some(proj) = object.proj.as_ref() {
        if let Some(crs) = proj.source_crs() {
            return Some(crs);
        }
    }
    match object.obj.as_ref()? {
        Obj::User(UserObject::Crs(crs)) => match crs {
            Crs::Bound(bound) => Some(bound.source_crs().clone()),
            Crs::Projected(projected) => Some(base_of_projected(projected.base_crs())),
            Crs::DerivedProjected(derived) => Some(Crs::Projected(derived.base_crs().clone())),
            _ => None,
        },
        Obj::User(UserObject::Operation(operation)) => operation_source(operation),
        Obj::User(UserObject::CoordinateMetadata(metadata)) => Some(metadata.crs().clone()),
        _ => None,
    }
}

pub(crate) fn operation_source(operation: &CoordinateOperation) -> Option<Crs> {
    match operation {
        CoordinateOperation::Conversion(value) => value.source_crs().cloned(),
        CoordinateOperation::Transformation(value) => value.source_crs().cloned(),
        CoordinateOperation::Concatenated(value) => value.source_crs().cloned(),
    }
}

pub(crate) fn operation_target(operation: &CoordinateOperation) -> Option<Crs> {
    match operation {
        CoordinateOperation::Conversion(value) => value.target_crs().cloned(),
        CoordinateOperation::Transformation(value) => value.target_crs().cloned(),
        CoordinateOperation::Concatenated(value) => value.target_crs().cloned(),
    }
}

fn target_of(object: &Pj) -> Option<Crs> {
    if let Some(proj) = object.proj.as_ref() {
        if let Some(crs) = proj.target_crs() {
            return Some(crs);
        }
    }
    match object.obj.as_ref()? {
        Obj::User(UserObject::Crs(Crs::Bound(bound))) => Some(bound.target_crs().clone()),
        Obj::User(UserObject::Operation(operation)) => operation_target(operation),
        _ => None,
    }
}

fn from_crs(object: &Pj, crs: Option<Crs>) -> *mut PJ {
    match crs {
        Some(crs) => derived(object, Obj::User(UserObject::Crs(crs))),
        None => std::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_get_source_crs(ctx: *mut PJ_CONTEXT, obj: *const PJ) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        let Some(object) = pj(obj) else {
            context.fail(
                PROJ_ERR_OTHER_API_MISUSE as c_int,
                "proj_get_source_crs: null object",
            );
            return std::ptr::null_mut();
        };
        from_crs(object, source_of(object))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_get_target_crs(ctx: *mut PJ_CONTEXT, obj: *const PJ) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        let Some(object) = pj(obj) else {
            context.fail(
                PROJ_ERR_OTHER_API_MISUSE as c_int,
                "proj_get_target_crs: null object",
            );
            return std::ptr::null_mut();
        };
        from_crs(object, target_of(object))
    })
}

// ---------------------------------------------------------------------------
// Datums.

/// The geodetic CRS a CRS is based on, itself when it is one.
fn geodetic_of(crs: &Crs) -> Option<Crs> {
    match crs {
        Crs::Geographic(_) | Crs::Geodetic(_) => Some(crs.clone()),
        Crs::Projected(projected) => Some(base_of_projected(projected.base_crs())),
        Crs::DerivedProjected(derived) => Some(base_of_projected(derived.base_crs().base_crs())),
        Crs::Bound(bound) => geodetic_of(bound.source_crs()),
        Crs::Compound(compound) => compound.components().iter().find_map(geodetic_of),
        _ => None,
    }
}

/// The datum and the datum ensemble of the horizontal part of a CRS.
fn geodetic_parts(crs: &Crs) -> Option<(Option<GeodeticReferenceFrame>, Option<DatumEnsemble>)> {
    let geodetic = geodetic_of(crs)?;
    match geodetic {
        Crs::Geographic(value) => Some((value.datum().cloned(), value.datum_ensemble().cloned())),
        Crs::Geodetic(value) => Some((value.datum().cloned(), value.datum_ensemble().cloned())),
        _ => None,
    }
}

fn datum_objects(crs: &Crs) -> Option<(Option<UserObject>, Option<DatumEnsemble>)> {
    match crs {
        Crs::Vertical(vertical) => Some((
            vertical.datum().cloned().map(UserObject::VerticalDatum),
            vertical.datum_ensemble().cloned(),
        )),
        Crs::Engineering(engineering) => Some((
            Some(UserObject::EngineeringDatum(engineering.datum().clone())),
            None,
        )),
        Crs::Compound(_) | Crs::Bound(_) => None,
        other => {
            let (datum, ensemble) = geodetic_parts(other)?;
            Some((datum.map(UserObject::GeodeticDatum), ensemble))
        }
    }
}

fn crs_arg<'a>(ctx: *mut PJ_CONTEXT, crs: *const PJ, who: &str) -> Option<(&'a mut Pj, &'a Crs)> {
    let context = context(ctx);
    let Some(object) = pj(crs) else {
        context.fail(
            PROJ_ERR_OTHER_API_MISUSE as c_int,
            &format!("{who}: null object"),
        );
        return None;
    };
    let found = object.obj.as_ref().and_then(Obj::crs);
    let Some(found) = found else {
        context.fail(
            PROJ_ERR_OTHER_API_MISUSE as c_int,
            &format!("{who}: Object is not a CRS"),
        );
        return None;
    };
    // The CRS lives as long as the object, which outlives this call.
    let crs_ref: &'a Crs = util::deref(std::ptr::from_ref(found))?;
    Some((object, crs_ref))
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_crs_get_geodetic_crs(ctx: *mut PJ_CONTEXT, crs: *const PJ) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let Some((object, found)) = crs_arg(ctx, crs, "proj_crs_get_geodetic_crs") else {
            return std::ptr::null_mut();
        };
        from_crs(object, geodetic_of(found))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_crs_get_horizontal_datum(ctx: *mut PJ_CONTEXT, crs: *const PJ) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        let Some((object, found)) = crs_arg(ctx, crs, "proj_crs_get_horizontal_datum") else {
            return std::ptr::null_mut();
        };
        let Some((datum, ensemble)) = geodetic_parts(found) else {
            return std::ptr::null_mut();
        };
        if let Some(datum) = datum {
            return derived(object, Obj::User(UserObject::GeodeticDatum(datum)));
        }
        // PROJ returns the ensemble itself (`proj_crs_get_horizontal_datum`, `c_api.cpp`).
        if let Some(ensemble) = ensemble {
            return derived(object, Obj::User(UserObject::Ensemble(ensemble)));
        }
        context.fail(
            PROJ_ERR_OTHER as c_int,
            "proj_crs_get_horizontal_datum: no datum",
        );
        std::ptr::null_mut()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_crs_get_sub_crs(
    ctx: *mut PJ_CONTEXT,
    crs: *const PJ,
    index: c_int,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let Some((object, found)) = crs_arg(ctx, crs, "proj_crs_get_sub_crs") else {
            return std::ptr::null_mut();
        };
        let Crs::Compound(compound) = found else {
            return std::ptr::null_mut();
        };
        let part = usize::try_from(index)
            .ok()
            .and_then(|at| compound.components().get(at))
            .cloned();
        from_crs(object, part)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_crs_get_datum(ctx: *mut PJ_CONTEXT, crs: *const PJ) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let Some((object, found)) = crs_arg(ctx, crs, "proj_crs_get_datum") else {
            return std::ptr::null_mut();
        };
        match datum_objects(found).and_then(|(datum, _)| datum) {
            Some(datum) => derived(object, Obj::User(datum)),
            None => std::ptr::null_mut(),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_crs_get_datum_ensemble(ctx: *mut PJ_CONTEXT, crs: *const PJ) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let Some((object, found)) = crs_arg(ctx, crs, "proj_crs_get_datum_ensemble") else {
            return std::ptr::null_mut();
        };
        match datum_objects(found).and_then(|(_, ensemble)| ensemble) {
            Some(ensemble) => derived(object, Obj::User(UserObject::Ensemble(ensemble))),
            None => std::ptr::null_mut(),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_crs_get_datum_forced(ctx: *mut PJ_CONTEXT, crs: *const PJ) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let Some((object, found)) = crs_arg(ctx, crs, "proj_crs_get_datum_forced") else {
            return std::ptr::null_mut();
        };
        let Some((datum, ensemble)) = datum_objects(found) else {
            return std::ptr::null_mut();
        };
        if let Some(datum) = datum {
            return derived(object, Obj::User(datum));
        }
        match ensemble.and_then(|ensemble| ensemble.as_datum()) {
            Some(proj_rs::unstable::Datum::Geodetic(frame)) => {
                derived(object, Obj::User(UserObject::GeodeticDatum(frame)))
            }
            Some(proj_rs::unstable::Datum::Vertical(frame)) => {
                derived(object, Obj::User(UserObject::VerticalDatum(frame)))
            }
            _ => std::ptr::null_mut(),
        }
    })
}

fn ensemble_of<'a>(
    ctx: *mut PJ_CONTEXT,
    ensemble: *const PJ,
    who: &str,
) -> Option<(&'a mut Pj, &'a DatumEnsemble)> {
    let context = context(ctx);
    let Some(object) = pj(ensemble) else {
        context.fail(
            PROJ_ERR_OTHER_API_MISUSE as c_int,
            &format!("{who}: null object"),
        );
        return None;
    };
    let Some(Obj::User(UserObject::Ensemble(found))) = object.obj.as_ref() else {
        context.fail(
            PROJ_ERR_OTHER_API_MISUSE as c_int,
            &format!("{who}: Object is not a DatumEnsemble"),
        );
        return None;
    };
    let reference: &'a DatumEnsemble = util::deref(std::ptr::from_ref(found))?;
    Some((object, reference))
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_datum_ensemble_get_member_count(
    ctx: *mut PJ_CONTEXT,
    datum_ensemble: *const PJ,
) -> c_int {
    guard(0, || {
        ensemble_of(ctx, datum_ensemble, "proj_datum_ensemble_get_member_count")
            .map_or(0, |(_, found)| {
                c_int::try_from(found.members().len()).unwrap_or(0)
            })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_datum_ensemble_get_accuracy(
    ctx: *mut PJ_CONTEXT,
    datum_ensemble: *const PJ,
) -> f64 {
    guard(-1.0, || {
        ensemble_of(ctx, datum_ensemble, "proj_datum_ensemble_get_accuracy")
            .map_or(-1.0, |(_, found)| found.positional_accuracy())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_datum_ensemble_get_member(
    ctx: *mut PJ_CONTEXT,
    datum_ensemble: *const PJ,
    member_index: c_int,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let Some((object, found)) =
            ensemble_of(ctx, datum_ensemble, "proj_datum_ensemble_get_member")
        else {
            return std::ptr::null_mut();
        };
        let member = usize::try_from(member_index)
            .ok()
            .and_then(|at| found.members().get(at))
            .cloned();
        match member {
            Some(proj_rs::unstable::Datum::Geodetic(frame)) => {
                derived(object, Obj::User(UserObject::GeodeticDatum(frame)))
            }
            Some(proj_rs::unstable::Datum::Vertical(frame)) => {
                derived(object, Obj::User(UserObject::VerticalDatum(frame)))
            }
            Some(proj_rs::unstable::Datum::Engineering(datum)) => {
                derived(object, Obj::User(UserObject::EngineeringDatum(datum)))
            }
            None => std::ptr::null_mut(),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_dynamic_datum_get_frame_reference_epoch(
    ctx: *mut PJ_CONTEXT,
    datum: *const PJ,
) -> f64 {
    guard(-1.0, || {
        let context = context(ctx);
        let Some(object) = pj(datum) else {
            return -1.0;
        };
        match object.obj.as_ref() {
            Some(Obj::User(UserObject::GeodeticDatum(frame))) => {
                frame.frame_reference_epoch().unwrap_or(-1.0)
            }
            Some(Obj::User(UserObject::VerticalDatum(frame))) => {
                frame.frame_reference_epoch().unwrap_or(-1.0)
            }
            _ => {
                context.fail(
                    PROJ_ERR_OTHER_API_MISUSE as c_int,
                    "proj_dynamic_datum_get_frame_reference_epoch: not a datum",
                );
                -1.0
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_crs_is_dynamic(ctx: *mut PJ_CONTEXT, crs: *const PJ) -> c_int {
    guard(0, || {
        let Some((_, found)) = crs_arg(ctx, crs, "proj_crs_is_dynamic") else {
            return 0;
        };
        c_int::from(found.is_dynamic())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_crs_is_derived(ctx: *mut PJ_CONTEXT, crs: *const PJ) -> c_int {
    guard(0, || {
        let Some((_, found)) = crs_arg(ctx, crs, "proj_crs_is_derived") else {
            return 0;
        };
        c_int::from(found.is_derived())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_is_derived_crs(ctx: *mut PJ_CONTEXT, crs: *const PJ) -> c_int {
    proj_crs_is_derived(ctx, crs)
}

// ---------------------------------------------------------------------------
// Coordinate systems.

fn cs_of(crs: &Crs) -> Option<CoordinateSystem> {
    Some(match crs {
        Crs::Geographic(value) => CoordinateSystem::Ellipsoidal(value.coordinate_system().clone()),
        Crs::Geodetic(value) => value.coordinate_system(),
        Crs::Projected(value) => CoordinateSystem::Cartesian(value.coordinate_system().clone()),
        Crs::Vertical(value) => CoordinateSystem::Vertical(value.coordinate_system().clone()),
        Crs::Engineering(value) => value.coordinate_system().clone(),
        Crs::DerivedProjected(value) => value.coordinate_system().clone(),
        Crs::Bound(bound) => return cs_of(bound.source_crs()),
        _ => return None,
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_crs_get_coordinate_system(ctx: *mut PJ_CONTEXT, crs: *const PJ) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let Some((object, found)) = crs_arg(ctx, crs, "proj_crs_get_coordinate_system") else {
            return std::ptr::null_mut();
        };
        match cs_of(found) {
            Some(system) => derived(object, Obj::Cs(system)),
            None => std::ptr::null_mut(),
        }
    })
}

fn cs_arg<'a>(ctx: *mut PJ_CONTEXT, cs: *const PJ, who: &str) -> Option<&'a CoordinateSystem> {
    let context = context(ctx);
    let Some(object) = pj(cs) else {
        context.fail(
            PROJ_ERR_OTHER_API_MISUSE as c_int,
            &format!("{who}: null object"),
        );
        return None;
    };
    let Some(Obj::Cs(system)) = object.obj.as_ref() else {
        context.fail(
            PROJ_ERR_OTHER_API_MISUSE as c_int,
            &format!("{who}: Object is not a CoordinateSystem"),
        );
        return None;
    };
    util::deref(std::ptr::from_ref(system))
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_cs_get_type(
    ctx: *mut PJ_CONTEXT,
    cs: *const PJ,
) -> PJ_COORDINATE_SYSTEM_TYPE {
    guard(PJ_CS_TYPE_UNKNOWN, || {
        let Some(system) = cs_arg(ctx, cs, "proj_cs_get_type") else {
            return PJ_CS_TYPE_UNKNOWN;
        };
        match system {
            CoordinateSystem::Ellipsoidal(_) => PJ_CS_TYPE_ELLIPSOIDAL,
            CoordinateSystem::Cartesian(_) | CoordinateSystem::Affine(_) => PJ_CS_TYPE_CARTESIAN,
            CoordinateSystem::Vertical(_) => PJ_CS_TYPE_VERTICAL,
            CoordinateSystem::Spherical(_) => PJ_CS_TYPE_SPHERICAL,
            CoordinateSystem::Ordinal(_) => PJ_CS_TYPE_ORDINAL,
            _ => PJ_CS_TYPE_UNKNOWN,
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_cs_get_axis_count(ctx: *mut PJ_CONTEXT, cs: *const PJ) -> c_int {
    guard(-1, || {
        cs_arg(ctx, cs, "proj_cs_get_axis_count").map_or(-1, |system| {
            c_int::try_from(system.axes().len()).unwrap_or(-1)
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_cs_get_axis_info(
    ctx: *mut PJ_CONTEXT,
    cs: *const PJ,
    index: c_int,
    out_name: *mut *const c_char,
    out_abbrev: *mut *const c_char,
    out_direction: *mut *const c_char,
    out_unit_conv_factor: *mut f64,
    out_unit_name: *mut *const c_char,
    out_unit_auth_name: *mut *const c_char,
    out_unit_code: *mut *const c_char,
) -> c_int {
    guard(0, || {
        let context = context(ctx);
        let Some(system) = cs_arg(ctx, cs, "proj_cs_get_axis_info") else {
            return 0;
        };
        let Some(axis) = usize::try_from(index)
            .ok()
            .and_then(|at| system.axes().get(at).copied())
        else {
            context.fail(
                PROJ_ERR_OTHER_API_MISUSE as c_int,
                "proj_cs_get_axis_info: Invalid index",
            );
            return 0;
        };
        let Some(object) = pj(cs) else {
            return 0;
        };
        let unit = axis.unit();
        util::put(out_name, object.arena.put(axis.name()));
        util::put(out_abbrev, object.arena.put(axis.abbreviation()));
        util::put(out_direction, object.arena.put(axis.direction()));
        util::put(out_unit_conv_factor, unit.conversion_to_si());
        util::put(out_unit_name, object.arena.put(unit.name()));
        let code_space = (!unit.code_space().is_empty()).then(|| unit.code_space().to_owned());
        let code = (!unit.code().is_empty()).then(|| unit.code().to_owned());
        util::put(
            out_unit_auth_name,
            object.arena.put_opt(code_space.as_deref()),
        );
        util::put(out_unit_code, object.arena.put_opt(code.as_deref()));
        1
    })
}

// ---------------------------------------------------------------------------
// Ellipsoid and prime meridian.

fn ellipsoid_of(object: &Obj) -> Option<Ellipsoid> {
    match object {
        Obj::User(UserObject::Ellipsoid(value)) => Some(value.clone()),
        Obj::User(UserObject::GeodeticDatum(datum)) => Some(datum.ellipsoid().clone()),
        Obj::User(UserObject::Ensemble(ensemble)) => {
            ensemble.members().iter().find_map(|member| match member {
                proj_rs::unstable::Datum::Geodetic(frame) => Some(frame.ellipsoid().clone()),
                _ => None,
            })
        }
        Obj::User(UserObject::Crs(crs)) => {
            let (datum, ensemble) = geodetic_parts(crs)?;
            if let Some(datum) = datum {
                return Some(datum.ellipsoid().clone());
            }
            ensemble.and_then(|value| ellipsoid_of(&Obj::User(UserObject::Ensemble(value))))
        }
        _ => None,
    }
}

fn prime_meridian_of(object: &Obj) -> Option<PrimeMeridian> {
    match object {
        Obj::User(UserObject::PrimeMeridian(value)) => Some(value.clone()),
        Obj::User(UserObject::GeodeticDatum(datum)) => Some(datum.prime_meridian().clone()),
        Obj::User(UserObject::Crs(crs)) => {
            let (datum, ensemble) = geodetic_parts(crs)?;
            if let Some(datum) = datum {
                return Some(datum.prime_meridian().clone());
            }
            ensemble.and_then(|value| {
                value.members().iter().find_map(|member| match member {
                    proj_rs::unstable::Datum::Geodetic(frame) => {
                        Some(frame.prime_meridian().clone())
                    }
                    _ => None,
                })
            })
        }
        _ => None,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_get_ellipsoid(ctx: *mut PJ_CONTEXT, obj: *const PJ) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        let Some(object) = pj(obj) else {
            context.fail(
                PROJ_ERR_OTHER_API_MISUSE as c_int,
                "proj_get_ellipsoid: null object",
            );
            return std::ptr::null_mut();
        };
        // PROJ reads a CRS or a geodetic reference frame, not an ensemble (`c_api.cpp`).
        let found = match object.obj.as_ref() {
            Some(Obj::User(UserObject::Ensemble(_))) | None => None,
            Some(other) => ellipsoid_of(other),
        };
        match found {
            Some(ellipsoid) => derived(object, Obj::User(UserObject::Ellipsoid(ellipsoid))),
            None => std::ptr::null_mut(),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_ellipsoid_get_parameters(
    ctx: *mut PJ_CONTEXT,
    ellipsoid: *const PJ,
    out_semi_major_metre: *mut f64,
    out_semi_minor_metre: *mut f64,
    out_is_semi_minor_computed: *mut c_int,
    out_inv_flattening: *mut f64,
) -> c_int {
    guard(0, || {
        let context = context(ctx);
        let Some(Obj::User(UserObject::Ellipsoid(found))) =
            pj(ellipsoid).and_then(|object| object.obj.as_ref())
        else {
            context.fail(
                PROJ_ERR_OTHER_API_MISUSE as c_int,
                "proj_ellipsoid_get_parameters: Object is not a Ellipsoid",
            );
            return 0;
        };
        let metre = UnitOfMeasure::metre();
        util::put(
            out_semi_major_metre,
            found.semi_major_axis().convert_to_unit(&metre),
        );
        util::put(
            out_semi_minor_metre,
            found.computed_semi_minor_axis().convert_to_unit(&metre),
        );
        util::put(
            out_is_semi_minor_computed,
            c_int::from(found.semi_minor_axis().is_none()),
        );
        util::put(out_inv_flattening, found.inverse_flattening());
        1
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_get_celestial_body_name(
    ctx: *mut PJ_CONTEXT,
    obj: *const PJ,
) -> *const c_char {
    guard(std::ptr::null(), || {
        let context = context(ctx);
        let Some(object) = pj(obj) else {
            context.fail(
                PROJ_ERR_OTHER_API_MISUSE as c_int,
                "proj_get_celestial_body_name: null object",
            );
            return std::ptr::null();
        };
        let name = match object.obj.as_ref() {
            Some(Obj::User(UserObject::Crs(crs))) => Some(
                geodetic_parts(crs)
                    .and_then(|_| object.obj.as_ref().and_then(ellipsoid_of))
                    .map_or_else(|| "Earth".to_owned(), |e| e.celestial_body().to_owned()),
            ),
            Some(Obj::User(UserObject::VerticalDatum(_))) => Some("Earth".to_owned()),
            Some(other) => ellipsoid_of(other).map(|e| e.celestial_body().to_owned()),
            None => None,
        };
        if name.is_none() {
            context.fail(
                PROJ_ERR_OTHER_API_MISUSE as c_int,
                "proj_get_celestial_body_name: Object is not a CRS, Datum or Ellipsoid",
            );
        }
        object.arena.put_opt(name.as_deref())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_get_prime_meridian(ctx: *mut PJ_CONTEXT, obj: *const PJ) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        let Some(object) = pj(obj) else {
            context.fail(
                PROJ_ERR_OTHER_API_MISUSE as c_int,
                "proj_get_prime_meridian: null object",
            );
            return std::ptr::null_mut();
        };
        match object.obj.as_ref().and_then(prime_meridian_of) {
            Some(meridian) => derived(object, Obj::User(UserObject::PrimeMeridian(meridian))),
            None => std::ptr::null_mut(),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_prime_meridian_get_parameters(
    ctx: *mut PJ_CONTEXT,
    prime_meridian: *const PJ,
    out_longitude: *mut f64,
    out_unit_conv_factor: *mut f64,
    out_unit_name: *mut *const c_char,
) -> c_int {
    guard(0, || {
        let context = context(ctx);
        let Some(object) = pj(prime_meridian) else {
            return 0;
        };
        let Some(Obj::User(UserObject::PrimeMeridian(found))) = object.obj.as_ref() else {
            context.fail(
                PROJ_ERR_OTHER_API_MISUSE as c_int,
                "proj_prime_meridian_get_parameters: Object is not a PrimeMeridian",
            );
            return 0;
        };
        let longitude = found.longitude();
        let unit_name = longitude.unit().name().to_owned();
        util::put(out_longitude, longitude.value());
        util::put(out_unit_conv_factor, longitude.unit().conversion_to_si());
        util::put(out_unit_name, object.arena.put(&unit_name));
        1
    })
}

/// A unit category as the parameter and unit functions name it.
pub(crate) fn category_name(kind: UnitKind) -> &'static str {
    match kind {
        UnitKind::Linear => "linear",
        UnitKind::Angular => "angular",
        UnitKind::Scale => "scale",
        UnitKind::Time => "time",
        UnitKind::Parametric => "parametric",
        UnitKind::None | UnitKind::Unknown => "unknown",
    }
}
