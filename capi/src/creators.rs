//! Objects built from their parts: coordinate systems and geodetic CRS
//! (`proj_create_*_cs`, `proj_create_geographic_crs`, `proj_create_geocentric_crs`).

use std::ffi::{c_char, c_int};

use proj_rs::iso19111::{
    CartesianCS, CoordinateSystem, Crs, EllipsoidalCS, GeodeticComponents, UnitKind, UnitOfMeasure,
    UserObject,
};

use crate::context::context;
use crate::ffi::*;
use crate::iso::Obj;
use crate::pj::{hand_over, pj, Pj};
use crate::util::{self, guard};

/// A unit given as a name and a factor to the SI unit; the default when no name is given.
fn unit(name: *const c_char, factor: f64, kind: UnitKind, default: UnitOfMeasure) -> UnitOfMeasure {
    match util::text(name) {
        Some(name) => UnitOfMeasure::new(name, factor, kind),
        None => default,
    }
}

fn cs_pj(ctx: *mut PJ_CONTEXT, system: CoordinateSystem) -> *mut PJ {
    let context = context(ctx);
    hand_over(Pj::with_obj(
        std::ptr::from_mut(context),
        Obj::Cs(system),
        None,
    ))
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_create_cartesian_2D_cs(
    ctx: *mut PJ_CONTEXT,
    type_: PJ_CARTESIAN_CS_2D_TYPE,
    unit_name: *const c_char,
    unit_conv_factor: f64,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let linear = unit(
            unit_name,
            unit_conv_factor,
            UnitKind::Linear,
            UnitOfMeasure::metre(),
        );
        let system = match type_ {
            PJ_CART2D_EASTING_NORTHING => CartesianCS::create_easting_northing(linear),
            PJ_CART2D_NORTHING_EASTING => CartesianCS::create_northing_easting(linear),
            PJ_CART2D_NORTH_POLE_EASTING_SOUTH_NORTHING_SOUTH => {
                CartesianCS::create_north_pole_easting_south_northing_south(linear)
            }
            PJ_CART2D_SOUTH_POLE_EASTING_NORTH_NORTHING_NORTH => {
                CartesianCS::create_south_pole_easting_north_northing_north(linear)
            }
            PJ_CART2D_WESTING_SOUTHING => CartesianCS::create_westing_southing(linear),
            _ => return std::ptr::null_mut(),
        };
        cs_pj(ctx, CoordinateSystem::Cartesian(system))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_create_ellipsoidal_2D_cs(
    ctx: *mut PJ_CONTEXT,
    type_: PJ_ELLIPSOIDAL_CS_2D_TYPE,
    unit_name: *const c_char,
    unit_conv_factor: f64,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let angular = unit(
            unit_name,
            unit_conv_factor,
            UnitKind::Angular,
            UnitOfMeasure::degree(),
        );
        let system = match type_ {
            PJ_ELLPS2D_LONGITUDE_LATITUDE => EllipsoidalCS::create_longitude_latitude(angular),
            PJ_ELLPS2D_LATITUDE_LONGITUDE => EllipsoidalCS::create_latitude_longitude(angular),
            _ => return std::ptr::null_mut(),
        };
        cs_pj(ctx, CoordinateSystem::Ellipsoidal(system))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_create_ellipsoidal_3D_cs(
    ctx: *mut PJ_CONTEXT,
    type_: PJ_ELLIPSOIDAL_CS_3D_TYPE,
    horizontal_angular_unit_name: *const c_char,
    horizontal_angular_unit_conv_factor: f64,
    vertical_linear_unit_name: *const c_char,
    vertical_linear_unit_conv_factor: f64,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let angular = unit(
            horizontal_angular_unit_name,
            horizontal_angular_unit_conv_factor,
            UnitKind::Angular,
            UnitOfMeasure::degree(),
        );
        let linear = unit(
            vertical_linear_unit_name,
            vertical_linear_unit_conv_factor,
            UnitKind::Linear,
            UnitOfMeasure::metre(),
        );
        let system = match type_ {
            PJ_ELLPS3D_LONGITUDE_LATITUDE_HEIGHT => {
                EllipsoidalCS::create_longitude_latitude_height(angular, linear)
            }
            PJ_ELLPS3D_LATITUDE_LONGITUDE_HEIGHT => {
                EllipsoidalCS::create_latitude_longitude_height(angular, linear)
            }
            _ => return std::ptr::null_mut(),
        };
        cs_pj(ctx, CoordinateSystem::Ellipsoidal(system))
    })
}

fn components<'a>(
    crs_name: *const c_char,
    datum_name: *const c_char,
    ellps_name: *const c_char,
    semi_major_metre: f64,
    inv_flattening: f64,
    prime_meridian_name: *const c_char,
    prime_meridian_offset: f64,
    angular_units: *const c_char,
    angular_units_conv: f64,
) -> GeodeticComponents<'a> {
    GeodeticComponents {
        crs_name: util::text(crs_name),
        datum_name: util::text(datum_name),
        ellipsoid_name: util::text(ellps_name),
        semi_major_metre,
        inverse_flattening: inv_flattening,
        prime_meridian_name: util::text(prime_meridian_name),
        prime_meridian_offset,
        angular_unit_name: util::text(angular_units),
        angular_unit_to_radian: angular_units_conv,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_create_geographic_crs(
    ctx: *mut PJ_CONTEXT,
    crs_name: *const c_char,
    datum_name: *const c_char,
    ellps_name: *const c_char,
    semi_major_metre: f64,
    inv_flattening: f64,
    prime_meridian_name: *const c_char,
    prime_meridian_offset: f64,
    pm_angular_units: *const c_char,
    pm_units_conv: f64,
    ellipsoidal_cs: *const PJ,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        let Some(Obj::Cs(CoordinateSystem::Ellipsoidal(system))) =
            pj(ellipsoidal_cs).and_then(|object| object.obj.as_ref())
        else {
            context.fail(
                PROJ_ERR_OTHER_API_MISUSE as c_int,
                "proj_create_geographic_crs: Object is not a EllipsoidalCS",
            );
            return std::ptr::null_mut();
        };
        let Some(database) = context.database() else {
            return std::ptr::null_mut();
        };
        let parts = components(
            crs_name,
            datum_name,
            ellps_name,
            semi_major_metre,
            inv_flattening,
            prime_meridian_name,
            prime_meridian_offset,
            pm_angular_units,
            pm_units_conv,
        );
        let crs = database.create_geographic_crs(&parts, system.clone());
        let obj = Obj::User(UserObject::Crs(Crs::Geographic(crs)));
        let proj = obj.engine_proj(context);
        hand_over(Pj::with_obj(std::ptr::from_mut(context), obj, proj))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_create_geocentric_crs(
    ctx: *mut PJ_CONTEXT,
    crs_name: *const c_char,
    datum_name: *const c_char,
    ellps_name: *const c_char,
    semi_major_metre: f64,
    inv_flattening: f64,
    prime_meridian_name: *const c_char,
    prime_meridian_offset: f64,
    angular_units: *const c_char,
    angular_units_conv: f64,
    linear_units: *const c_char,
    linear_units_conv: f64,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        let Some(database) = context.database() else {
            return std::ptr::null_mut();
        };
        let parts = components(
            crs_name,
            datum_name,
            ellps_name,
            semi_major_metre,
            inv_flattening,
            prime_meridian_name,
            prime_meridian_offset,
            angular_units,
            angular_units_conv,
        );
        let crs =
            database.create_geocentric_crs(&parts, util::text(linear_units), linear_units_conv);
        let obj = Obj::User(UserObject::Crs(Crs::Geodetic(crs)));
        let proj = obj.engine_proj(context);
        hand_over(Pj::with_obj(std::ptr::from_mut(context), obj, proj))
    })
}
