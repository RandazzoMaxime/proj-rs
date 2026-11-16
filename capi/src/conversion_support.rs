//! What the conversion constructors share: units given by the caller, and the result.

use std::ffi::{c_char, c_int};

use proj_rs::iso19111::{Conversion, CoordinateOperation, FactoryError, OpProps, UserObject};

use crate::context::context;
use crate::ffi::*;
use crate::iso::Obj;
use crate::pj::{hand_over, Pj};
use crate::util::guard;

/// Radians in one degree, the factor of the degree in the database (`EPSG:9122`).
const RADIANS_PER_DEGREE: f64 = 0.017_453_292_519_943_295;

/// The angular and linear units of a call. A missing name means degree and metre.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Units {
    degrees_per_angle_unit: f64,
    metres_per_length_unit: f64,
}

impl Units {
    pub(crate) fn new(
        angle_name: *const c_char,
        angle_factor: f64,
        length_name: *const c_char,
        length_factor: f64,
    ) -> Self {
        let angle = if angle_name.is_null() || angle_factor <= 0.0 {
            1.0
        } else {
            angle_factor / RADIANS_PER_DEGREE
        };
        let length = if length_name.is_null() || length_factor <= 0.0 {
            1.0
        } else {
            length_factor
        };
        Self {
            degrees_per_angle_unit: angle,
            metres_per_length_unit: length,
        }
    }

    /// An angle in degrees.
    pub(crate) fn angle(&self, value: f64) -> f64 {
        value * self.degrees_per_angle_unit
    }

    /// A length in metres.
    pub(crate) fn length(&self, value: f64) -> f64 {
        value * self.metres_per_length_unit
    }

    /// A scale factor, which no unit changes.
    pub(crate) fn scale(&self, value: f64) -> f64 {
        value
    }
}

/// Gives a new conversion to C, or reports why it could not be made.
pub(crate) fn finish(
    ctx: *mut PJ_CONTEXT,
    made: Result<Conversion, FactoryError>,
    who: &str,
) -> *mut PJ {
    let context = context(ctx);
    match made {
        Ok(conversion) => {
            let obj = Obj::User(UserObject::Operation(CoordinateOperation::Conversion(
                conversion,
            )));
            let proj = obj.engine_proj(context);
            hand_over(Pj::with_obj(std::ptr::from_mut(context), obj, proj))
        }
        Err(error) => {
            context.fail(PROJ_ERR_OTHER as c_int, &format!("{who}: {error}"));
            std::ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_create_conversion_utm(
    ctx: *mut PJ_CONTEXT,
    zone: c_int,
    north: c_int,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let props = OpProps::new();
        let made = if north != 0 {
            Conversion::create_utm_north(&props, zone)
        } else {
            Conversion::create_utm_south(&props, zone)
        };
        finish(ctx, made, "proj_create_conversion_utm")
    })
}
