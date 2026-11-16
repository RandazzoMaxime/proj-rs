//! Changes made to a CRS: its name, its identifier, its units, its dimension, its datum, its
//! bound form, and its epoch (`proj_alter_*`, `proj_crs_promote_to_3D`...).

use std::ffi::{c_char, c_int};

use proj_rs::iso19111::{
    AlternativeGridNames, BoundCRS, CoordinateMetadata, CoordinateOperation, Crs, GeodeticCRS,
    ParameterLinearUnit, UnitKind, UnitOfMeasure, UserObject,
};

use crate::context::context;
use crate::ffi::*;
use crate::iso::{derived, Obj};
use crate::pj::{pj, Pj};
use crate::util::{self, guard};

fn crs_of<'a>(ctx: *mut PJ_CONTEXT, obj: *const PJ, who: &str) -> Option<(&'a mut Pj, &'a Crs)> {
    let context = context(ctx);
    let Some(object) = pj(obj) else {
        context.fail(
            PROJ_ERR_OTHER_API_MISUSE as c_int,
            &format!("{who}: null object"),
        );
        return None;
    };
    let Some(found) = object.obj.as_ref().and_then(Obj::crs) else {
        context.fail(
            PROJ_ERR_OTHER_API_MISUSE as c_int,
            &format!("{who}: Object is not a CRS"),
        );
        return None;
    };
    let reference: &'a Crs = util::deref(std::ptr::from_ref(found))?;
    Some((object, reference))
}

fn give(object: &Pj, crs: Crs) -> *mut PJ {
    derived(object, Obj::User(UserObject::Crs(crs)))
}

fn unit_of(
    name: *const c_char,
    factor: f64,
    auth_name: *const c_char,
    code: *const c_char,
    kind: UnitKind,
) -> Option<UnitOfMeasure> {
    let unit = UnitOfMeasure::new(util::text(name)?, factor, kind);
    Some(match (util::text(auth_name), util::text(code)) {
        (Some(authority), Some(code)) => unit.with_identifier(authority, code),
        _ => unit,
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_alter_name(
    ctx: *mut PJ_CONTEXT,
    obj: *const PJ,
    name: *const c_char,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let Some((object, found)) = crs_of(ctx, obj, "proj_alter_name") else {
            return std::ptr::null_mut();
        };
        match util::text(name) {
            Some(name) => give(object, found.alter_name(name)),
            None => std::ptr::null_mut(),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_alter_id(
    ctx: *mut PJ_CONTEXT,
    obj: *const PJ,
    auth_name: *const c_char,
    code: *const c_char,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let Some((object, found)) = crs_of(ctx, obj, "proj_alter_id") else {
            return std::ptr::null_mut();
        };
        match (util::text(auth_name), util::text(code)) {
            (Some(authority), Some(code)) => give(object, found.alter_id(authority, code)),
            _ => std::ptr::null_mut(),
        }
    })
}

fn geodetic_crs_of(crs: &Crs) -> Option<GeodeticCRS> {
    match crs {
        Crs::Geographic(value) => Some(GeodeticCRS::Geographic(value.clone())),
        Crs::Geodetic(value) => Some(value.clone()),
        _ => None,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_crs_alter_geodetic_crs(
    ctx: *mut PJ_CONTEXT,
    obj: *const PJ,
    new_geod_crs: *const PJ,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let Some((object, found)) = crs_of(ctx, obj, "proj_crs_alter_geodetic_crs") else {
            return std::ptr::null_mut();
        };
        let Some((_, replacement)) = crs_of(ctx, new_geod_crs, "proj_crs_alter_geodetic_crs")
        else {
            return std::ptr::null_mut();
        };
        match geodetic_crs_of(replacement) {
            Some(geodetic) => give(object, found.alter_geodetic_crs(&geodetic)),
            None => std::ptr::null_mut(),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_crs_alter_cs_angular_unit(
    ctx: *mut PJ_CONTEXT,
    obj: *const PJ,
    angular_units: *const c_char,
    angular_units_conv: f64,
    unit_auth_name: *const c_char,
    unit_code: *const c_char,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let Some((object, found)) = crs_of(ctx, obj, "proj_crs_alter_cs_angular_unit") else {
            return std::ptr::null_mut();
        };
        let factor = if angular_units_conv > 0.0 {
            angular_units_conv
        } else {
            0.0
        };
        match unit_of(
            angular_units,
            factor,
            unit_auth_name,
            unit_code,
            UnitKind::Angular,
        ) {
            Some(unit) => give(object, found.alter_cs_angular_unit(&unit)),
            None => give(
                object,
                found.alter_cs_angular_unit(&UnitOfMeasure::degree()),
            ),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_crs_alter_cs_linear_unit(
    ctx: *mut PJ_CONTEXT,
    obj: *const PJ,
    linear_units: *const c_char,
    linear_units_conv: f64,
    unit_auth_name: *const c_char,
    unit_code: *const c_char,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let Some((object, found)) = crs_of(ctx, obj, "proj_crs_alter_cs_linear_unit") else {
            return std::ptr::null_mut();
        };
        match unit_of(
            linear_units,
            linear_units_conv,
            unit_auth_name,
            unit_code,
            UnitKind::Linear,
        ) {
            Some(unit) => give(object, found.alter_cs_linear_unit(&unit)),
            None => give(object, found.alter_cs_linear_unit(&UnitOfMeasure::metre())),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_crs_alter_parameters_linear_unit(
    ctx: *mut PJ_CONTEXT,
    obj: *const PJ,
    linear_units: *const c_char,
    linear_units_conv: f64,
    unit_auth_name: *const c_char,
    unit_code: *const c_char,
    convert_to_new_unit: c_int,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let Some((object, found)) = crs_of(ctx, obj, "proj_crs_alter_parameters_linear_unit")
        else {
            return std::ptr::null_mut();
        };
        let Crs::Projected(projected) = found else {
            return std::ptr::null_mut();
        };
        let unit = unit_of(
            linear_units,
            linear_units_conv,
            unit_auth_name,
            unit_code,
            UnitKind::Linear,
        )
        .unwrap_or_else(UnitOfMeasure::metre);
        let change = if convert_to_new_unit != 0 {
            ParameterLinearUnit::Convert(unit)
        } else {
            ParameterLinearUnit::Relabel(unit)
        };
        give(
            object,
            Crs::Projected(projected.alter_parameters_linear_unit(change)),
        )
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_crs_promote_to_3D(
    ctx: *mut PJ_CONTEXT,
    crs_3D_name: *const c_char,
    crs_2D: *const PJ,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        let Some((object, found)) = crs_of(ctx, crs_2D, "proj_crs_promote_to_3D") else {
            return std::ptr::null_mut();
        };
        let database = context.database();
        let promoted = found.promote_to_3d(database.as_ref());
        give(
            object,
            match util::text(crs_3D_name) {
                Some(name) => promoted.alter_name(name),
                None => promoted,
            },
        )
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_crs_demote_to_2D(
    ctx: *mut PJ_CONTEXT,
    crs_2D_name: *const c_char,
    crs_3D: *const PJ,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        let Some((object, found)) = crs_of(ctx, crs_3D, "proj_crs_demote_to_2D") else {
            return std::ptr::null_mut();
        };
        let database = context.database();
        let demoted = found.demote_to_2d(database.as_ref());
        give(
            object,
            match util::text(crs_2D_name) {
                Some(name) => demoted.alter_name(name),
                None => demoted,
            },
        )
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_crs_create_projected_3D_crs_from_2D(
    ctx: *mut PJ_CONTEXT,
    crs_name: *const c_char,
    projected_2D_crs: *const PJ,
    geog_3D_crs: *const PJ,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let Some((object, found)) = crs_of(
            ctx,
            projected_2D_crs,
            "proj_crs_create_projected_3D_crs_from_2D",
        ) else {
            return std::ptr::null_mut();
        };
        let Crs::Projected(projected) = found else {
            return std::ptr::null_mut();
        };
        let base = if geog_3D_crs.is_null() {
            None
        } else {
            match crs_of(ctx, geog_3D_crs, "proj_crs_create_projected_3D_crs_from_2D") {
                Some((_, Crs::Geographic(geographic))) => Some(geographic.clone()),
                _ => return std::ptr::null_mut(),
            }
        };
        let promoted = match base {
            Some(base) => projected.promote_to_3d_on(&base),
            None => projected.promote_to_3d(context(ctx).database().as_ref()),
        };
        let crs = Crs::Projected(promoted);
        give(
            object,
            match util::text(crs_name) {
                Some(name) => crs.alter_name(name),
                None => crs,
            },
        )
    })
}

// ---------------------------------------------------------------------------
// Bound CRS.

#[unsafe(no_mangle)]
pub extern "C" fn proj_crs_create_bound_crs(
    ctx: *mut PJ_CONTEXT,
    base_crs: *const PJ,
    hub_crs: *const PJ,
    transformation: *const PJ,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let Some((object, base)) = crs_of(ctx, base_crs, "proj_crs_create_bound_crs") else {
            return std::ptr::null_mut();
        };
        let Some((_, hub)) = crs_of(ctx, hub_crs, "proj_crs_create_bound_crs") else {
            return std::ptr::null_mut();
        };
        let Some(operation) = pj(transformation)
            .and_then(|found| found.obj.as_ref())
            .and_then(Obj::operation)
        else {
            return std::ptr::null_mut();
        };
        let _: &CoordinateOperation = operation;
        give(
            object,
            Crs::Bound(BoundCRS::create(
                base.clone(),
                hub.clone(),
                operation.clone(),
            )),
        )
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_crs_create_bound_crs_to_WGS84(
    ctx: *mut PJ_CONTEXT,
    crs: *const PJ,
    options: *const *const c_char,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        let _ = options;
        let Some((object, found)) = crs_of(ctx, crs, "proj_crs_create_bound_crs_to_WGS84") else {
            return std::ptr::null_mut();
        };
        let Some(database) = context.database() else {
            return std::ptr::null_mut();
        };
        give(
            object,
            found.create_bound_crs_to_wgs84_if_possible(&database),
        )
    })
}

// ---------------------------------------------------------------------------
// Coordinate metadata and point motion.

#[unsafe(no_mangle)]
pub extern "C" fn proj_coordinate_metadata_create(
    ctx: *mut PJ_CONTEXT,
    crs: *const PJ,
    epoch: f64,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        let Some((object, found)) = crs_of(ctx, crs, "proj_coordinate_metadata_create") else {
            return std::ptr::null_mut();
        };
        let Some(database) = context.database() else {
            return std::ptr::null_mut();
        };
        match CoordinateMetadata::create(found, epoch, &database) {
            Ok(metadata) => {
                let proj = object
                    .proj
                    .as_ref()
                    .and_then(|proj| proj.coordinate_metadata_create(epoch).ok());
                crate::pj::hand_over(Pj::with_obj(
                    object.ctx,
                    Obj::User(UserObject::CoordinateMetadata(metadata)),
                    proj,
                ))
            }
            Err(error) => {
                context.fail(
                    PROJ_ERR_OTHER as c_int,
                    &format!("proj_coordinate_metadata_create: {error}"),
                );
                std::ptr::null_mut()
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_coordinate_metadata_get_epoch(ctx: *mut PJ_CONTEXT, obj: *const PJ) -> f64 {
    guard(f64::NAN, || {
        let context = context(ctx);
        match pj(obj).and_then(|object| object.obj.as_ref()) {
            Some(Obj::User(UserObject::CoordinateMetadata(metadata))) => {
                metadata.coordinate_epoch().unwrap_or(f64::NAN)
            }
            _ => {
                context.fail(
                    PROJ_ERR_OTHER_API_MISUSE as c_int,
                    "proj_coordinate_metadata_get_epoch: Object is not a CoordinateMetadata",
                );
                f64::NAN
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_crs_has_point_motion_operation(
    ctx: *mut PJ_CONTEXT,
    crs: *const PJ,
) -> c_int {
    guard(0, || {
        let context = context(ctx);
        let Some((_, found)) = crs_of(ctx, crs, "proj_crs_has_point_motion_operation") else {
            return 0;
        };
        let (Some(geodetic), Some(database)) = (geodetic_part(found), context.database()) else {
            return 0;
        };
        let factory = proj_rs::iso19111::AuthorityFactory::create(&database, "");
        c_int::from(
            factory
                .get_point_motion_operations_for(&geodetic, AlternativeGridNames::Official)
                .is_ok_and(|operations| !operations.is_empty()),
        )
    })
}

fn geodetic_part(crs: &Crs) -> Option<GeodeticCRS> {
    match crs {
        Crs::Geographic(_) | Crs::Geodetic(_) => geodetic_crs_of(crs),
        Crs::Projected(projected) => Some(projected.base_crs().clone()),
        Crs::Compound(compound) => compound.components().iter().find_map(geodetic_part),
        _ => None,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_crs_get_coordoperation(ctx: *mut PJ_CONTEXT, crs: *const PJ) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        let Some((object, found)) = crs_of(ctx, crs, "proj_crs_get_coordoperation") else {
            return std::ptr::null_mut();
        };
        let conversion = match found {
            Crs::Projected(projected) => Some(projected.deriving_conversion().clone()),
            Crs::DerivedProjected(derived_crs) => Some(derived_crs.deriving_conversion().clone()),
            _ => None,
        };
        match conversion {
            Some(conversion) => derived(
                object,
                Obj::User(UserObject::Operation(CoordinateOperation::Conversion(
                    conversion,
                ))),
            ),
            None => {
                context.fail(
                    PROJ_ERR_OTHER_API_MISUSE as c_int,
                    "proj_crs_get_coordoperation: Object is not a DerivedCRS or BoundCRSs",
                );
                std::ptr::null_mut()
            }
        }
    })
}
