//! Objects built from objects: conversions, transformations, projected, vertical, compound,
//! engineering and derived CRS, coordinate systems, and the queries that find a CRS from a datum
//! (`proj_create_conversion`, `proj_create_projected_crs`, ... `iso19111/c_api.cpp`).

use std::ffi::{c_char, c_int};

use proj_rs::iso19111::{
    AuthorityFactory, Axis, CartesianCS, CompoundCRS, Conversion, CoordinateOperation,
    CoordinateSystem, Crs, DerivedCRS, EllipsoidalCS, EngineeringCRS, EngineeringDatum,
    GeodeticCRS, GeographicCRS, GeoidModel, Measure, OpProps, OrdinalCS, ParameterSpec,
    ParameterValue, ProjectedCRS, Transformation, UnitKind, UnitOfMeasure, UserObject, VerticalCRS,
    VerticalCS, VerticalReferenceFrame,
};

use crate::context::context;
use crate::ffi::*;
use crate::iso::Obj;
use crate::list::object_list;
use crate::pj::{hand_over, pj, Pj};
use crate::util::{self, guard};

fn give_crs(ctx: *mut PJ_CONTEXT, crs: Crs) -> *mut PJ {
    let context = context(ctx);
    let obj = Obj::User(UserObject::Crs(crs));
    let proj = obj.engine_proj(context);
    hand_over(Pj::with_obj(std::ptr::from_mut(context), obj, proj))
}

fn give_operation(ctx: *mut PJ_CONTEXT, operation: CoordinateOperation) -> *mut PJ {
    let context = context(ctx);
    let obj = Obj::User(UserObject::Operation(operation));
    let proj = obj.engine_proj(context);
    hand_over(Pj::with_obj(std::ptr::from_mut(context), obj, proj))
}

fn misuse(ctx: *mut PJ_CONTEXT, who: &str, message: &str) {
    context(ctx).fail(
        PROJ_ERR_OTHER_API_MISUSE as c_int,
        &format!("{who}: {message}"),
    );
}

fn failed(ctx: *mut PJ_CONTEXT, who: &str, error: &dyn std::fmt::Display) {
    context(ctx).fail(PROJ_ERR_OTHER as c_int, &format!("{who}: {error}"));
}

fn crs_arg<'a>(pointer: *const PJ) -> Option<&'a Crs> {
    pj(pointer)
        .and_then(|object| object.obj.as_ref())
        .and_then(Obj::crs)
}

fn conversion_arg<'a>(pointer: *const PJ) -> Option<&'a Conversion> {
    match pj(pointer)
        .and_then(|object| object.obj.as_ref())
        .and_then(Obj::operation)
    {
        Some(CoordinateOperation::Conversion(conversion)) => Some(conversion),
        _ => None,
    }
}

fn cs_arg<'a>(pointer: *const PJ) -> Option<&'a CoordinateSystem> {
    match pj(pointer).and_then(|object| object.obj.as_ref()) {
        Some(Obj::Cs(system)) => Some(system),
        _ => None,
    }
}

fn name_of(pointer: *const c_char) -> String {
    util::text(pointer).unwrap_or("unnamed").to_owned()
}

fn geodetic_of(crs: &Crs) -> Option<GeodeticCRS> {
    match crs {
        Crs::Geographic(value) => Some(GeodeticCRS::Geographic(value.clone())),
        Crs::Geodetic(value) => Some(value.clone()),
        _ => None,
    }
}

fn unit_kind_of(kind: PJ_UNIT_TYPE) -> UnitKind {
    match kind {
        PJ_UT_ANGULAR => UnitKind::Angular,
        PJ_UT_LINEAR => UnitKind::Linear,
        PJ_UT_SCALE => UnitKind::Scale,
        PJ_UT_TIME => UnitKind::Time,
        PJ_UT_PARAMETRIC => UnitKind::Parametric,
        _ => UnitKind::Unknown,
    }
}

/// `setSingleOperationElements` (`c_api.cpp`): a missing name is "unnamed".
fn props(name: *const c_char, authority: *const c_char, code: *const c_char) -> OpProps {
    let mut built = OpProps::new().name(util::text(name).unwrap_or("unnamed"));
    if let (Some(authority), Some(code)) = (util::text(authority), util::text(code)) {
        built = built.identifier(authority, code);
    }
    built
}

fn specs_of(count: c_int, params: *const PJ_PARAM_DESCRIPTION) -> Vec<ParameterSpec> {
    let optional = |text: *const c_char| util::text(text).unwrap_or_default().to_owned();
    util::slice(params, usize::try_from(count).unwrap_or(0))
        .iter()
        .map(|param| {
            let unit = match param.unit_type {
                PJ_UT_ANGULAR => angular_unit(param.unit_name, param.unit_conv_factor),
                PJ_UT_LINEAR => linear_unit(param.unit_name, param.unit_conv_factor),
                other => UnitOfMeasure::new(
                    util::text(param.unit_name).unwrap_or("unnamed"),
                    param.unit_conv_factor,
                    unit_kind_of(other),
                ),
            };
            ParameterSpec {
                name: util::text(param.name).unwrap_or("unnamed").to_owned(),
                code_space: optional(param.auth_name),
                code: optional(param.code),
                value: ParameterValue::measure(param.value, unit),
            }
        })
        .collect()
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_create_conversion(
    ctx: *mut PJ_CONTEXT,
    name: *const c_char,
    auth_name: *const c_char,
    code: *const c_char,
    method_name: *const c_char,
    method_auth_name: *const c_char,
    method_code: *const c_char,
    param_count: c_int,
    params: *const PJ_PARAM_DESCRIPTION,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        // PROJ names a missing method "unnamed" (`setSingleOperationElements`); GDAL's
        // `SetProjCS` builds such a placeholder conversion.
        let method = util::text(method_name).unwrap_or("unnamed");
        let made = Conversion::create_identified(
            &props(name, auth_name, code),
            method,
            util::text(method_auth_name).unwrap_or(""),
            util::text(method_code).unwrap_or(""),
            &specs_of(param_count, params),
        );
        match made {
            Ok(conversion) => give_operation(ctx, CoordinateOperation::Conversion(conversion)),
            Err(error) => {
                failed(ctx, "proj_create_conversion", &error);
                std::ptr::null_mut()
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_create_transformation(
    ctx: *mut PJ_CONTEXT,
    name: *const c_char,
    auth_name: *const c_char,
    code: *const c_char,
    source_crs: *const PJ,
    target_crs: *const PJ,
    interpolation_crs: *const PJ,
    method_name: *const c_char,
    method_auth_name: *const c_char,
    method_code: *const c_char,
    param_count: c_int,
    params: *const PJ_PARAM_DESCRIPTION,
    accuracy: f64,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let (Some(source), Some(target)) = (crs_arg(source_crs), crs_arg(target_crs)) else {
            misuse(
                ctx,
                "proj_create_transformation",
                "missing source CRS or target CRS",
            );
            return std::ptr::null_mut();
        };
        let method = util::text(method_name).unwrap_or("unnamed");
        let made = Transformation::create_identified(
            &props(name, auth_name, code),
            source.clone(),
            target.clone(),
            crs_arg(interpolation_crs).cloned(),
            method,
            util::text(method_auth_name).unwrap_or(""),
            util::text(method_code).unwrap_or(""),
            &specs_of(param_count, params),
            (accuracy >= 0.0).then_some(accuracy),
        );
        match made {
            Ok(transformation) => {
                give_operation(ctx, CoordinateOperation::Transformation(transformation))
            }
            Err(error) => {
                failed(ctx, "proj_create_transformation", &error);
                std::ptr::null_mut()
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_create_projected_crs(
    ctx: *mut PJ_CONTEXT,
    crs_name: *const c_char,
    geodetic_crs: *const PJ,
    conversion: *const PJ,
    coordinate_system: *const PJ,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let (Some(base), Some(conversion), Some(CoordinateSystem::Cartesian(system))) = (
            crs_arg(geodetic_crs).and_then(geodetic_of),
            conversion_arg(conversion),
            cs_arg(coordinate_system),
        ) else {
            misuse(
                ctx,
                "proj_create_projected_crs",
                "wrong or missing argument",
            );
            return std::ptr::null_mut();
        };
        give_crs(
            ctx,
            Crs::Projected(ProjectedCRS::create(
                name_of(crs_name),
                base,
                conversion.clone(),
                system.clone(),
            )),
        )
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_create_derived_projected_crs(
    ctx: *mut PJ_CONTEXT,
    crs_name: *const c_char,
    base_proj_crs: *const PJ,
    deriving_conversion: *const PJ,
    coordinate_system: *const PJ,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let (Some(Crs::Projected(base)), Some(conversion), Some(system)) = (
            crs_arg(base_proj_crs),
            conversion_arg(deriving_conversion),
            cs_arg(coordinate_system),
        ) else {
            misuse(
                ctx,
                "proj_create_derived_projected_crs",
                "wrong or missing argument",
            );
            return std::ptr::null_mut();
        };
        give_crs(
            ctx,
            Crs::DerivedProjected(proj_rs::iso19111::DerivedProjectedCRS::create(
                name_of(crs_name),
                base.clone(),
                conversion.clone(),
                system.clone(),
            )),
        )
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_crs_add_horizontal_derived_conversion(
    ctx: *mut PJ_CONTEXT,
    crs_name: *const c_char,
    base_crs: *const PJ,
    deriving_conversion: *const PJ,
    coordinate_system: *const PJ,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let (Some(base), Some(conversion), Some(system)) = (
            crs_arg(base_crs),
            conversion_arg(deriving_conversion),
            cs_arg(coordinate_system),
        ) else {
            misuse(
                ctx,
                "proj_crs_add_horizontal_derived_conversion",
                "missing required input",
            );
            return std::ptr::null_mut();
        };
        match base.clone().add_horizontal_derived_conversion(
            name_of(crs_name),
            conversion.clone(),
            system.clone(),
        ) {
            Ok(crs) => give_crs(ctx, crs),
            Err(error) => {
                failed(ctx, "proj_crs_add_horizontal_derived_conversion", &error);
                std::ptr::null_mut()
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_create_derived_geographic_crs(
    ctx: *mut PJ_CONTEXT,
    crs_name: *const c_char,
    base_geographic_crs: *const PJ,
    conversion: *const PJ,
    ellipsoidal_cs: *const PJ,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let (Some(base), Some(conversion), Some(system)) = (
            crs_arg(base_geographic_crs),
            conversion_arg(conversion),
            cs_arg(ellipsoidal_cs),
        ) else {
            misuse(
                ctx,
                "proj_create_derived_geographic_crs",
                "wrong or missing argument",
            );
            return std::ptr::null_mut();
        };
        match DerivedCRS::create(
            name_of(crs_name),
            base.clone(),
            conversion.clone(),
            system.clone(),
        ) {
            Ok(crs) => give_crs(ctx, Crs::Derived(crs)),
            Err(error) => {
                failed(ctx, "proj_create_derived_geographic_crs", &error);
                std::ptr::null_mut()
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_create_engineering_crs(
    ctx: *mut PJ_CONTEXT,
    crsName: *const c_char,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        give_crs(
            ctx,
            Crs::Engineering(EngineeringCRS::create(
                name_of(crsName),
                EngineeringDatum::create("Unknown engineering datum"),
                CoordinateSystem::Cartesian(CartesianCS::create_easting_northing(
                    UnitOfMeasure::metre(),
                )),
            )),
        )
    })
}

fn linear_unit(name: *const c_char, factor: f64) -> UnitOfMeasure {
    util::text(name).map_or_else(UnitOfMeasure::metre, |name| {
        UnitOfMeasure::new(name, factor, UnitKind::Linear)
    })
}

/// `createAngularUnit` (`c_api.cpp`): degree when unnamed, and the named degree and grad.
fn angular_unit(name: *const c_char, factor: f64) -> UnitOfMeasure {
    match util::text(name) {
        None => UnitOfMeasure::degree(),
        Some(name) if name.eq_ignore_ascii_case("degree") => UnitOfMeasure::degree(),
        Some(name) if name.eq_ignore_ascii_case("grad") => UnitOfMeasure::grad(),
        Some(name) => UnitOfMeasure::new(name, factor, UnitKind::Angular),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_create_vertical_crs(
    ctx: *mut PJ_CONTEXT,
    crs_name: *const c_char,
    datum_name: *const c_char,
    linear_units: *const c_char,
    linear_units_conv: f64,
) -> *mut PJ {
    proj_create_vertical_crs_ex(
        ctx,
        crs_name,
        datum_name,
        std::ptr::null(),
        std::ptr::null(),
        linear_units,
        linear_units_conv,
        std::ptr::null(),
        std::ptr::null(),
        std::ptr::null(),
        std::ptr::null(),
        std::ptr::null(),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_create_vertical_crs_ex(
    ctx: *mut PJ_CONTEXT,
    crs_name: *const c_char,
    datum_name: *const c_char,
    datum_auth_name: *const c_char,
    datum_code: *const c_char,
    linear_units: *const c_char,
    linear_units_conv: f64,
    geoid_model_name: *const c_char,
    geoid_model_auth_name: *const c_char,
    geoid_model_code: *const c_char,
    geoid_geog_crs: *const PJ,
    options: *const *const c_char,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let mut datum = VerticalReferenceFrame::named(name_of(datum_name));
        if let (Some(authority), Some(code)) = (util::text(datum_auth_name), util::text(datum_code))
        {
            datum = datum.with_identifier(authority, code);
        }
        let system =
            VerticalCS::create_gravity_related_height(linear_unit(linear_units, linear_units_conv));
        let mut crs = VerticalCRS::create(name_of(crs_name), datum, system);
        if let Some(model_name) = util::text(geoid_model_name) {
            let mut model = GeoidModel::create(model_name);
            if let (Some(authority), Some(code)) = (
                util::text(geoid_model_auth_name),
                util::text(geoid_model_code),
            ) {
                model = model.with_identifier(authority, code);
            }
            if let Some(Crs::Geographic(geographic)) = crs_arg(geoid_geog_crs) {
                model = model.with_interpolation_crs(geographic.clone());
            }
            for (key, value) in crate::iso::options_of(options) {
                if key == "ACCURACY" {
                    if let Ok(accuracy) = value.parse::<f64>() {
                        model = model.with_accuracy(accuracy);
                    }
                }
            }
            crs = crs.with_geoid_model(model);
        }
        give_crs(ctx, Crs::Vertical(crs))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_create_compound_crs(
    ctx: *mut PJ_CONTEXT,
    crs_name: *const c_char,
    horiz_crs: *const PJ,
    vert_crs: *const PJ,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let (Some(horizontal), Some(vertical)) = (crs_arg(horiz_crs), crs_arg(vert_crs)) else {
            misuse(ctx, "proj_create_compound_crs", "missing CRS");
            return std::ptr::null_mut();
        };
        match CompoundCRS::create(
            name_of(crs_name),
            vec![horizontal.clone(), vertical.clone()],
        ) {
            Ok(compound) => give_crs(ctx, Crs::Compound(compound)),
            Err(error) => {
                failed(ctx, "proj_create_compound_crs", &error);
                std::ptr::null_mut()
            }
        }
    })
}

fn datum_parts(
    object: &Obj,
) -> Option<(
    Option<proj_rs::iso19111::GeodeticReferenceFrame>,
    Option<proj_rs::iso19111::DatumEnsemble>,
)> {
    match object {
        Obj::User(UserObject::GeodeticDatum(frame)) => Some((Some(frame.clone()), None)),
        Obj::User(UserObject::Ensemble(ensemble)) => Some((None, Some(ensemble.clone()))),
        _ => None,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_create_geographic_crs_from_datum(
    ctx: *mut PJ_CONTEXT,
    crs_name: *const c_char,
    datum_or_datum_ensemble: *const PJ,
    ellipsoidal_cs: *const PJ,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let (Some((datum, ensemble)), Some(CoordinateSystem::Ellipsoidal(system))) = (
            pj(datum_or_datum_ensemble)
                .and_then(|o| o.obj.as_ref())
                .and_then(datum_parts),
            cs_arg(ellipsoidal_cs),
        ) else {
            misuse(
                ctx,
                "proj_create_geographic_crs_from_datum",
                "wrong or missing argument",
            );
            return std::ptr::null_mut();
        };
        match GeographicCRS::create(name_of(crs_name), datum, ensemble, system.clone()) {
            Ok(crs) => give_crs(ctx, Crs::Geographic(crs)),
            Err(error) => {
                failed(ctx, "proj_create_geographic_crs_from_datum", &error);
                std::ptr::null_mut()
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_create_geocentric_crs_from_datum(
    ctx: *mut PJ_CONTEXT,
    crs_name: *const c_char,
    datum_or_datum_ensemble: *const PJ,
    linear_units: *const c_char,
    linear_units_conv: f64,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let Some((datum, ensemble)) = pj(datum_or_datum_ensemble)
            .and_then(|o| o.obj.as_ref())
            .and_then(datum_parts)
        else {
            misuse(
                ctx,
                "proj_create_geocentric_crs_from_datum",
                "wrong or missing argument",
            );
            return std::ptr::null_mut();
        };
        let system = CartesianCS::create_geocentric(linear_unit(linear_units, linear_units_conv));
        give_crs(
            ctx,
            Crs::Geodetic(GeodeticCRS::create_geocentric(
                name_of(crs_name),
                datum,
                ensemble,
                system,
            )),
        )
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_query_geodetic_crs_from_datum(
    ctx: *mut PJ_CONTEXT,
    crs_auth_name: *const c_char,
    datum_auth_name: *const c_char,
    datum_code: *const c_char,
    crs_type: *const c_char,
) -> *mut PJ_OBJ_LIST {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        let (Some(datum_auth), Some(datum_code), Some(database)) = (
            util::text(datum_auth_name),
            util::text(datum_code),
            context.database(),
        ) else {
            return std::ptr::null_mut();
        };
        let factory = AuthorityFactory::create(&database, util::text(crs_auth_name).unwrap_or(""));
        match factory.create_geodetic_crs_from_datum(
            datum_auth,
            datum_code,
            util::text(crs_type).unwrap_or(""),
        ) {
            Ok(found) => object_list(
                context,
                found
                    .into_iter()
                    .map(|crs| {
                        Obj::User(UserObject::Crs(match crs {
                            GeodeticCRS::Geographic(geographic) => Crs::Geographic(geographic),
                            other => Crs::Geodetic(other),
                        }))
                    })
                    .collect(),
            ),
            Err(error) => {
                failed(ctx, "proj_query_geodetic_crs_from_datum", &error);
                std::ptr::null_mut()
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_convert_conversion_to_other_method(
    ctx: *mut PJ_CONTEXT,
    conversion: *const PJ,
    new_method_epsg_code: c_int,
    new_method_name: *const c_char,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let Some(found) = conversion_arg(conversion) else {
            misuse(
                ctx,
                "proj_convert_conversion_to_other_method",
                "not a conversion",
            );
            return std::ptr::null_mut();
        };
        let code = if new_method_epsg_code != 0 {
            new_method_epsg_code
        } else {
            match util::text(new_method_name).and_then(proj_rs::iso19111::method_epsg_code) {
                Some(code) => code,
                None => return std::ptr::null_mut(),
            }
        };
        match found.convert_to_other_method(code) {
            Some(converted) => {
                let converted = match found.source_crs() {
                    Some(source) => converted.with_source_crs(source.clone()),
                    None => converted,
                };
                give_operation(ctx, CoordinateOperation::Conversion(converted))
            }
            None => std::ptr::null_mut(),
        }
    })
}

// ---------------------------------------------------------------------------
// Coordinate systems and affine conversions.

#[unsafe(no_mangle)]
pub extern "C" fn proj_create_cs(
    ctx: *mut PJ_CONTEXT,
    type_: PJ_COORDINATE_SYSTEM_TYPE,
    axis_count: c_int,
    axis: *const PJ_AXIS_DESCRIPTION,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let axes: Vec<Axis> = util::slice(axis, usize::try_from(axis_count).unwrap_or(0))
            .iter()
            .map(|description| {
                let unit =
                    util::text(description.unit_name).map_or_else(UnitOfMeasure::none, |name| {
                        UnitOfMeasure::new(
                            name,
                            description.unit_conv_factor,
                            unit_kind_of(description.unit_type),
                        )
                    });
                Axis::create(
                    name_of(description.name),
                    util::text(description.abbreviation).unwrap_or(""),
                    util::text(description.direction).unwrap_or("unspecified"),
                    unit,
                )
            })
            .collect();
        let made = match type_ {
            PJ_CS_TYPE_ELLIPSOIDAL => {
                EllipsoidalCS::create(axes).map(CoordinateSystem::Ellipsoidal)
            }
            PJ_CS_TYPE_CARTESIAN => CartesianCS::create(axes).map(CoordinateSystem::Cartesian),
            PJ_CS_TYPE_VERTICAL => match axes.into_iter().next() {
                Some(first) => Ok(CoordinateSystem::Vertical(VerticalCS::create(first))),
                None => Err(proj_rs::iso19111::FactoryError::Invalid(
                    "no axis".to_owned(),
                )),
            },
            PJ_CS_TYPE_ORDINAL => Ok(CoordinateSystem::Ordinal(OrdinalCS::create(axes))),
            _ => Err(proj_rs::iso19111::FactoryError::Invalid(
                "unsupported coordinate system type".to_owned(),
            )),
        };
        match made {
            Ok(system) => {
                let context = context(ctx);
                hand_over(Pj::with_obj(
                    std::ptr::from_mut(context),
                    Obj::Cs(system),
                    None,
                ))
            }
            Err(error) => {
                failed(ctx, "proj_create_cs", &error);
                std::ptr::null_mut()
            }
        }
    })
}

fn affine(
    ctx: *mut PJ_CONTEXT,
    who: &str,
    method: &str,
    name: *const c_char,
    terms: &[(f64, *const c_char, f64, UnitKind)],
) -> *mut PJ {
    let three_d = method.starts_with("3D");
    let values = terms
        .iter()
        .enumerate()
        .map(|(index, (value, unit_name, factor, kind))| {
            let unit = util::text(*unit_name).map_or_else(
                || match kind {
                    UnitKind::Linear => UnitOfMeasure::metre(),
                    // PROJ gives the registered unity to the fourth term of a row, and to the whole C row.
                    _ if three_d && (index % 4 == 3 || index >= 8) => {
                        UnitOfMeasure::unity().with_identifier("EPSG", "9201")
                    }
                    _ => UnitOfMeasure::unity(),
                },
                |text| UnitOfMeasure::new(text, *factor, *kind),
            );
            ParameterValue::Measure(Measure::new(*value, unit))
        })
        .collect();
    let built = OpProps::new().name(util::text(name).unwrap_or("unnamed"));
    match Conversion::create_by_name(&built, method, values) {
        Ok(conversion) => give_operation(ctx, CoordinateOperation::Conversion(conversion)),
        Err(error) => {
            failed(ctx, who, &error);
            std::ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_create_linear_affine_parametric_conversion(
    ctx: *mut PJ_CONTEXT,
    name: *const c_char,
    A0: f64,
    A0_unit_name: *const c_char,
    A0_unit_conv_factor: f64,
    A1: f64,
    A1_unit_name: *const c_char,
    A1_unit_conv_factor: f64,
    A2: f64,
    A2_unit_name: *const c_char,
    A2_unit_conv_factor: f64,
    B0: f64,
    B0_unit_name: *const c_char,
    B0_unit_conv_factor: f64,
    B1: f64,
    B1_unit_name: *const c_char,
    B1_unit_conv_factor: f64,
    B2: f64,
    B2_unit_name: *const c_char,
    B2_unit_conv_factor: f64,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        affine(
            ctx,
            "proj_create_linear_affine_parametric_conversion",
            "Affine parametric transformation",
            name,
            &[
                (A0, A0_unit_name, A0_unit_conv_factor, UnitKind::Linear),
                (A1, A1_unit_name, A1_unit_conv_factor, UnitKind::Scale),
                (A2, A2_unit_name, A2_unit_conv_factor, UnitKind::Scale),
                (B0, B0_unit_name, B0_unit_conv_factor, UnitKind::Linear),
                (B1, B1_unit_name, B1_unit_conv_factor, UnitKind::Scale),
                (B2, B2_unit_name, B2_unit_conv_factor, UnitKind::Scale),
            ],
        )
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_create_linear_3D_affine_parametric_conversion(
    ctx: *mut PJ_CONTEXT,
    name: *const c_char,
    A0: f64,
    A0_unit_name: *const c_char,
    A0_unit_conv_factor: f64,
    A1: f64,
    A1_unit_name: *const c_char,
    A1_unit_conv_factor: f64,
    A2: f64,
    A2_unit_name: *const c_char,
    A2_unit_conv_factor: f64,
    A3: f64,
    A3_unit_name: *const c_char,
    A3_unit_conv_factor: f64,
    B0: f64,
    B0_unit_name: *const c_char,
    B0_unit_conv_factor: f64,
    B1: f64,
    B1_unit_name: *const c_char,
    B1_unit_conv_factor: f64,
    B2: f64,
    B2_unit_name: *const c_char,
    B2_unit_conv_factor: f64,
    B3: f64,
    B3_unit_name: *const c_char,
    B3_unit_conv_factor: f64,
    C0: f64,
    C0_unit_name: *const c_char,
    C0_unit_conv_factor: f64,
    C1: f64,
    C1_unit_name: *const c_char,
    C1_unit_conv_factor: f64,
    C2: f64,
    C2_unit_name: *const c_char,
    C2_unit_conv_factor: f64,
    C3: f64,
    C3_unit_name: *const c_char,
    C3_unit_conv_factor: f64,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        affine(
            ctx,
            "proj_create_linear_3D_affine_parametric_conversion",
            "3D Affine parametric transformation",
            name,
            &[
                (A0, A0_unit_name, A0_unit_conv_factor, UnitKind::Linear),
                (A1, A1_unit_name, A1_unit_conv_factor, UnitKind::Scale),
                (A2, A2_unit_name, A2_unit_conv_factor, UnitKind::Scale),
                (A3, A3_unit_name, A3_unit_conv_factor, UnitKind::Scale),
                (B0, B0_unit_name, B0_unit_conv_factor, UnitKind::Linear),
                (B1, B1_unit_name, B1_unit_conv_factor, UnitKind::Scale),
                (B2, B2_unit_name, B2_unit_conv_factor, UnitKind::Scale),
                (B3, B3_unit_name, B3_unit_conv_factor, UnitKind::Scale),
                (C0, C0_unit_name, C0_unit_conv_factor, UnitKind::Linear),
                (C1, C1_unit_name, C1_unit_conv_factor, UnitKind::Scale),
                (C2, C2_unit_name, C2_unit_conv_factor, UnitKind::Scale),
                (C3, C3_unit_name, C3_unit_conv_factor, UnitKind::Scale),
            ],
        )
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_crs_create_bound_vertical_crs(
    ctx: *mut PJ_CONTEXT,
    vert_crs: *const PJ,
    hub_geographic_3D_crs: *const PJ,
    grid_name: *const c_char,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let (Some(vertical @ Crs::Vertical(_)), Some(hub @ Crs::Geographic(_)), Some(grid)) = (
            crs_arg(vert_crs),
            crs_arg(hub_geographic_3D_crs),
            util::text(grid_name),
        ) else {
            misuse(
                ctx,
                "proj_crs_create_bound_vertical_crs",
                "wrong or missing argument",
            );
            return std::ptr::null_mut();
        };
        let method = "Geographic3D to GravityRelatedHeight (US .gtx)";
        let code = proj_rs::iso19111::method_epsg_code(method).map(|code| code.to_string());
        let specs = [ParameterSpec {
            name: "Geoid (height correction) model file".to_owned(),
            code_space: "EPSG".to_owned(),
            code: "8666".to_owned(),
            value: ParameterValue::filename(grid),
        }];
        let made = Transformation::create_identified(
            &OpProps::new().name(format!("{} to {}", vertical.name(), hub.name())),
            vertical.clone(),
            hub.clone(),
            None,
            method,
            "EPSG",
            code.as_deref().unwrap_or(""),
            &specs,
            None,
        );
        match made {
            Ok(transformation) => give_crs(
                ctx,
                Crs::Bound(proj_rs::iso19111::BoundCRS::create(
                    vertical.clone(),
                    hub.clone(),
                    CoordinateOperation::Transformation(transformation),
                )),
            ),
            Err(error) => {
                failed(ctx, "proj_crs_create_bound_vertical_crs", &error);
                std::ptr::null_mut()
            }
        }
    })
}
