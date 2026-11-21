//! ISO 19111 objects behind a `PJ *`: what they are, their identity, their equivalence, and
//! their text forms (`proj_get_type` ... `proj_as_projjson` of `iso19111/c_api.cpp`).

use std::ffi::{c_char, c_int};

use proj_rs::iso19111::{
    CoordinateOperation, CoordinateSystem, Crs, DatabaseContext, Datum, EllipsoidalHeightPolicy,
    Equivalence, GeodeticCRS, Identifier, JsonFormat, JsonLines, LinunitPolicy, ObjectDomain,
    OutputAxisRule, ProjLines, UserObject, WktConvention, WktFormatter, WktLines, WktStrictness,
};
use proj_rs::unstable::Proj;

use crate::context::{context, Ctx};
use crate::ffi::*;
use crate::pj::{hand_over, pj, Pj};
use crate::util::{self, guard};

/// An ISO 19111 object of the model: what `createFromUserInput` returns, or a part of one.
#[derive(Clone, Debug)]
#[expect(
    clippy::large_enum_variant,
    reason = "the model owns each CRS kind by value, and a coordinate system is small beside it"
)]
pub(crate) enum Obj {
    User(UserObject),
    Cs(CoordinateSystem),
}

impl Obj {
    pub(crate) fn crs(&self) -> Option<&Crs> {
        match self {
            Self::User(UserObject::Crs(crs)) => Some(crs),
            _ => None,
        }
    }

    pub(crate) fn operation(&self) -> Option<&CoordinateOperation> {
        match self {
            Self::User(UserObject::Operation(operation)) => Some(operation),
            _ => None,
        }
    }

    pub(crate) fn name(&self) -> String {
        match self {
            Self::Cs(_) => String::new(),
            Self::User(object) => match object {
                UserObject::Crs(crs) => crs.name().to_owned(),
                UserObject::GeodeticDatum(datum) => datum.name().to_owned(),
                UserObject::VerticalDatum(datum) => datum.name().to_owned(),
                UserObject::ParametricDatum(datum) => datum.name().to_owned(),
                UserObject::EngineeringDatum(datum) => datum.name().to_owned(),
                UserObject::TemporalDatum(datum) => datum.name().to_owned(),
                UserObject::Ensemble(ensemble) => ensemble.name().to_owned(),
                UserObject::PrimeMeridian(meridian) => meridian.name().to_owned(),
                UserObject::Ellipsoid(ellipsoid) => ellipsoid.name().to_owned(),
                UserObject::Operation(operation) => operation.name().to_owned(),
                UserObject::CoordinateMetadata(metadata) => metadata.crs().name().to_owned(),
            },
        }
    }

    pub(crate) fn identifiers(&self) -> &[Identifier] {
        match self {
            Self::Cs(_) => &[],
            Self::User(object) => match object {
                UserObject::Crs(crs) => crs.identifiers(),
                UserObject::GeodeticDatum(datum) => datum.identifiers(),
                UserObject::VerticalDatum(datum) => datum.identifiers(),
                UserObject::ParametricDatum(datum) => datum.identifiers(),
                UserObject::EngineeringDatum(datum) => datum.identifiers(),
                UserObject::TemporalDatum(datum) => datum.identifiers(),
                UserObject::Ensemble(ensemble) => ensemble.identifiers(),
                UserObject::PrimeMeridian(meridian) => meridian.identifiers(),
                UserObject::Ellipsoid(ellipsoid) => ellipsoid.identifiers(),
                UserObject::Operation(operation) => operation.identifiers(),
                UserObject::CoordinateMetadata(metadata) => metadata.crs().identifiers(),
            },
        }
    }

    pub(crate) fn domains(&self) -> &[ObjectDomain] {
        match self {
            Self::Cs(_) => &[],
            Self::User(object) => match object {
                UserObject::Crs(crs) => crs.domains(),
                UserObject::GeodeticDatum(datum) => datum.domains(),
                UserObject::VerticalDatum(datum) => datum.domains(),
                UserObject::Ensemble(ensemble) => ensemble.domains(),
                UserObject::Operation(operation) => operation.domains(),
                UserObject::CoordinateMetadata(metadata) => metadata.crs().domains(),
                _ => &[],
            },
        }
    }

    pub(crate) fn remarks(&self) -> &str {
        match self {
            Self::User(UserObject::Crs(crs)) => crs.remarks(),
            Self::User(UserObject::Operation(operation)) => operation.remarks(),
            _ => "",
        }
    }

    pub(crate) fn pj_type(&self) -> PJ_TYPE {
        match self {
            Self::Cs(_) => PJ_TYPE_UNKNOWN,
            Self::User(object) => match object {
                UserObject::Crs(crs) => crs_type(crs),
                UserObject::GeodeticDatum(datum) => {
                    if datum.frame_reference_epoch().is_some() {
                        PJ_TYPE_DYNAMIC_GEODETIC_REFERENCE_FRAME
                    } else {
                        PJ_TYPE_GEODETIC_REFERENCE_FRAME
                    }
                }
                UserObject::VerticalDatum(datum) => {
                    if datum.frame_reference_epoch().is_some() {
                        PJ_TYPE_DYNAMIC_VERTICAL_REFERENCE_FRAME
                    } else {
                        PJ_TYPE_VERTICAL_REFERENCE_FRAME
                    }
                }
                UserObject::ParametricDatum(_) => PJ_TYPE_PARAMETRIC_DATUM,
                UserObject::EngineeringDatum(_) => PJ_TYPE_ENGINEERING_DATUM,
                UserObject::TemporalDatum(_) => PJ_TYPE_TEMPORAL_DATUM,
                UserObject::Ensemble(_) => PJ_TYPE_DATUM_ENSEMBLE,
                UserObject::PrimeMeridian(_) => PJ_TYPE_PRIME_MERIDIAN,
                UserObject::Ellipsoid(_) => PJ_TYPE_ELLIPSOID,
                UserObject::Operation(operation) => match operation {
                    CoordinateOperation::Conversion(_) => PJ_TYPE_CONVERSION,
                    CoordinateOperation::Transformation(_) => PJ_TYPE_TRANSFORMATION,
                    CoordinateOperation::Concatenated(_) => PJ_TYPE_CONCATENATED_OPERATION,
                },
                UserObject::CoordinateMetadata(_) => PJ_TYPE_COORDINATE_METADATA,
            },
        }
    }

    /// The engine object that transforms for a CRS that `Proj::new` did not read from its
    /// text: the CRS written as WKT2 and read again.
    pub(crate) fn engine_proj(&self, ctx: &mut Ctx) -> Option<Proj> {
        if let Some(operation) = self.operation() {
            let database = ctx.database()?;
            return ctx.builder().proj_from_operation(operation, &database).ok();
        }
        if let Self::User(UserObject::CoordinateMetadata(metadata)) = self {
            let base = Self::User(UserObject::Crs(metadata.crs().clone())).engine_proj(ctx)?;
            return base
                .coordinate_metadata_create(metadata.coordinate_epoch()?)
                .ok();
        }
        let crs = self.crs()?;
        let database = ctx.database()?;
        let wkt = wkt_of_crs(crs, &database, WktConvention::Wkt2_2019)?;
        ctx.builder().proj(&wkt).ok()
    }
}

fn wkt_of_crs(crs: &Crs, database: &DatabaseContext, convention: WktConvention) -> Option<String> {
    let mut formatter = WktFormatter::create(convention).with_database(database.clone());
    crs.export_to_wkt(&mut formatter).ok()
}

fn crs_type(crs: &Crs) -> PJ_TYPE {
    match crs {
        Crs::Geographic(geographic) => geographic_type(geographic.coordinate_system().axes().len()),
        Crs::Geodetic(GeodeticCRS::Geographic(geographic)) => {
            geographic_type(geographic.coordinate_system().axes().len())
        }
        Crs::Geodetic(GeodeticCRS::Geocentric(_)) => PJ_TYPE_GEOCENTRIC_CRS,
        Crs::Geodetic(_) => PJ_TYPE_GEODETIC_CRS,
        Crs::Projected(_) => PJ_TYPE_PROJECTED_CRS,
        Crs::Vertical(_) => PJ_TYPE_VERTICAL_CRS,
        Crs::Compound(_) => PJ_TYPE_COMPOUND_CRS,
        Crs::Bound(_) => PJ_TYPE_BOUND_CRS,
        Crs::Engineering(_) => PJ_TYPE_ENGINEERING_CRS,
        Crs::DerivedProjected(_) => PJ_TYPE_DERIVED_PROJECTED_CRS,
        Crs::Temporal(_) => PJ_TYPE_TEMPORAL_CRS,
        Crs::Parametric(_) | Crs::Derived(_) => PJ_TYPE_OTHER_CRS,
    }
}

fn geographic_type(axes: usize) -> PJ_TYPE {
    if axes == 3 {
        PJ_TYPE_GEOGRAPHIC_3D_CRS
    } else {
        PJ_TYPE_GEOGRAPHIC_2D_CRS
    }
}

// ---------------------------------------------------------------------------
// Identity.

#[unsafe(no_mangle)]
pub extern "C" fn proj_get_type(obj: *const PJ) -> PJ_TYPE {
    guard(PJ_TYPE_UNKNOWN, || {
        pj(obj)
            .and_then(|object| object.obj.as_ref())
            .map_or(PJ_TYPE_UNKNOWN, Obj::pj_type)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_is_crs(obj: *const PJ) -> c_int {
    guard(0, || {
        c_int::from(
            pj(obj)
                .and_then(|object| object.obj.as_ref())
                .is_some_and(|object| object.crs().is_some()),
        )
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_get_name(obj: *const PJ) -> *const c_char {
    guard(std::ptr::null(), || {
        let Some(object) = pj(obj) else {
            return std::ptr::null();
        };
        match object.obj.as_ref() {
            // Neither is an IdentifiedObject: PROJ gives them no name.
            Some(Obj::Cs(_) | Obj::User(UserObject::CoordinateMetadata(_))) | None => {
                std::ptr::null()
            }
            Some(obj) => object.arena.put(&obj.name()),
        }
    })
}

fn identifier_of(object: &Pj, index: c_int) -> Option<&Identifier> {
    let list = object.obj.as_ref()?.identifiers();
    list.get(usize::try_from(index).ok()?)
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_get_id_auth_name(obj: *const PJ, index: c_int) -> *const c_char {
    guard(std::ptr::null(), || {
        let Some(object) = pj(obj) else {
            return std::ptr::null();
        };
        let text = identifier_of(object, index).map(|id| id.code_space().to_owned());
        object.arena.put_opt(text.as_deref())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_get_id_code(obj: *const PJ, index: c_int) -> *const c_char {
    guard(std::ptr::null(), || {
        let Some(object) = pj(obj) else {
            return std::ptr::null();
        };
        let text = identifier_of(object, index).map(|id| id.code().to_owned());
        object.arena.put_opt(text.as_deref())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_get_remarks(obj: *const PJ) -> *const c_char {
    guard(std::ptr::null(), || {
        let Some(object) = pj(obj) else {
            return std::ptr::null();
        };
        match object.obj.as_ref().map(|inner| inner.remarks().to_owned()) {
            Some(remarks) => object.arena.put(&remarks),
            None => std::ptr::null(),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_get_domain_count(obj: *const PJ) -> c_int {
    guard(0, || {
        pj(obj)
            .and_then(|object| object.obj.as_ref())
            .map_or(0, |inner| {
                c_int::try_from(inner.domains().len()).unwrap_or(0)
            })
    })
}

fn domain_of(object: &Pj, index: c_int) -> Option<&ObjectDomain> {
    let list = object.obj.as_ref()?.domains();
    list.get(usize::try_from(index).ok()?)
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_get_scope(obj: *const PJ) -> *const c_char {
    proj_get_scope_ex(obj, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_get_scope_ex(obj: *const PJ, domain_idx: c_int) -> *const c_char {
    guard(std::ptr::null(), || {
        let Some(object) = pj(obj) else {
            return std::ptr::null();
        };
        let scope =
            domain_of(object, domain_idx).and_then(|domain| domain.scope().map(str::to_owned));
        object.arena.put_opt(scope.as_deref())
    })
}

fn area_of_use(
    ctx: *mut PJ_CONTEXT,
    obj: *const PJ,
    domain_idx: c_int,
    out: [*mut f64; 4],
    area_name: *mut *const c_char,
) -> c_int {
    let context = context(ctx);
    let Some(object) = pj(obj) else {
        context.fail(
            PROJ_ERR_OTHER_API_MISUSE as c_int,
            "proj_get_area_of_use: null object",
        );
        return 0;
    };
    util::put(area_name, std::ptr::null());
    for slot in out {
        util::put(slot, -1000.0);
    }
    let Some(domain) = domain_of(object, domain_idx) else {
        return 0;
    };
    let description = domain
        .description()
        .or_else(|| domain.area())
        .map(str::to_owned);
    let bounds = (
        domain.west_bound_longitude(),
        domain.south_bound_latitude(),
        domain.east_bound_longitude(),
        domain.north_bound_latitude(),
    );
    util::put(area_name, object.arena.put_opt(description.as_deref()));
    let (Some(west), Some(south), Some(east), Some(north)) = bounds else {
        return 0;
    };
    for (slot, value) in out.into_iter().zip([west, south, east, north]) {
        util::put(slot, value);
    }
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_get_area_of_use(
    ctx: *mut PJ_CONTEXT,
    obj: *const PJ,
    out_west_lon_degree: *mut f64,
    out_south_lat_degree: *mut f64,
    out_east_lon_degree: *mut f64,
    out_north_lat_degree: *mut f64,
    out_area_name: *mut *const c_char,
) -> c_int {
    proj_get_area_of_use_ex(
        ctx,
        obj,
        0,
        out_west_lon_degree,
        out_south_lat_degree,
        out_east_lon_degree,
        out_north_lat_degree,
        out_area_name,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_get_area_of_use_ex(
    ctx: *mut PJ_CONTEXT,
    obj: *const PJ,
    domain_idx: c_int,
    out_west_lon_degree: *mut f64,
    out_south_lat_degree: *mut f64,
    out_east_lon_degree: *mut f64,
    out_north_lat_degree: *mut f64,
    out_area_name: *mut *const c_char,
) -> c_int {
    guard(0, || {
        area_of_use(
            ctx,
            obj,
            domain_idx,
            [
                out_west_lon_degree,
                out_south_lat_degree,
                out_east_lon_degree,
                out_north_lat_degree,
            ],
            out_area_name,
        )
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_is_deprecated(obj: *const PJ) -> c_int {
    guard(0, || {
        c_int::from(
            pj(obj)
                .and_then(|object| object.obj.as_ref())
                .and_then(Obj::crs)
                .is_some_and(Crs::is_deprecated),
        )
    })
}

// ---------------------------------------------------------------------------
// Equivalence.

fn criterion_of(criterion: PJ_COMPARISON_CRITERION) -> Equivalence {
    match criterion {
        PJ_COMP_STRICT => Equivalence::Strict,
        PJ_COMP_EQUIVALENT_EXCEPT_AXIS_ORDER_GEOGCRS => Equivalence::ExceptAxisOrder,
        _ => Equivalence::Equivalent,
    }
}

fn equivalent(
    context: &mut Ctx,
    left: &Obj,
    right: &Obj,
    criterion: PJ_COMPARISON_CRITERION,
) -> bool {
    let way = criterion_of(criterion);
    match (left, right) {
        (Obj::User(UserObject::Crs(a)), Obj::User(UserObject::Crs(b))) => {
            match (way, context.database()) {
                (Equivalence::Equivalent, Some(database)) => a.is_equivalent_to_db(b, &database),
                _ => a.is_equivalent_to_with(b, way),
            }
        }
        (Obj::User(UserObject::GeodeticDatum(a)), Obj::User(UserObject::GeodeticDatum(b))) => {
            a.is_equivalent_to_with(b, way)
        }
        (Obj::User(UserObject::VerticalDatum(a)), Obj::User(UserObject::VerticalDatum(b))) => {
            a.is_equivalent_to_with(b, way)
        }
        (
            Obj::User(UserObject::Operation(CoordinateOperation::Conversion(a))),
            Obj::User(UserObject::Operation(CoordinateOperation::Conversion(b))),
        ) => match a.source_crs() {
            Some(Crs::Geographic(source)) => {
                a.is_equivalent_to_with_source(b, &GeodeticCRS::Geographic(source.clone()), way)
            }
            Some(Crs::Geodetic(source)) => a.is_equivalent_to_with_source(b, source, way),
            _ => a.is_equivalent_to_with(b, way),
        },
        (Obj::User(UserObject::Operation(a)), Obj::User(UserObject::Operation(b))) => {
            a.is_equivalent_to_with(b, way)
        }
        (Obj::User(UserObject::Ellipsoid(a)), Obj::User(UserObject::Ellipsoid(b))) => {
            a.is_equivalent_to_with(b, way)
        }
        (Obj::User(UserObject::PrimeMeridian(a)), Obj::User(UserObject::PrimeMeridian(b))) => {
            a.is_equivalent_to_with(b, way)
        }
        (Obj::User(UserObject::Ensemble(a)), Obj::User(UserObject::Ensemble(b))) => {
            a.is_equivalent_to(b)
        }
        (Obj::Cs(a), Obj::Cs(b)) => a.is_equivalent_to(b),
        _ => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_is_equivalent_to(
    obj: *const PJ,
    other: *const PJ,
    criterion: PJ_COMPARISON_CRITERION,
) -> c_int {
    proj_is_equivalent_to_with_ctx(std::ptr::null_mut(), obj, other, criterion)
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_is_equivalent_to_with_ctx(
    ctx: *mut PJ_CONTEXT,
    obj: *const PJ,
    other: *const PJ,
    criterion: PJ_COMPARISON_CRITERION,
) -> c_int {
    guard(0, || {
        let (Some(left), Some(right)) = (pj(obj), pj(other)) else {
            return 0;
        };
        if let (Some(a), Some(b)) = (left.proj.as_ref(), right.proj.as_ref()) {
            if left.obj.is_none() && right.obj.is_none() {
                return c_int::from(a.is_equivalent_to(b));
            }
        }
        let context = context(ctx);
        match (left.obj.as_ref(), right.obj.as_ref()) {
            (Some(a), Some(b)) => c_int::from(equivalent(context, a, b, criterion)),
            _ => 0,
        }
    })
}

// ---------------------------------------------------------------------------
// Text forms.

/// The options of `KEY=VALUE` strings of a `proj_as_*` call, a `NULL`-terminated array.
pub(crate) fn options_of<'a>(options: *const *const c_char) -> Vec<(String, &'a str)> {
    let mut found = Vec::new();
    if options.is_null() {
        return found;
    }
    let mut index = 0;
    while let Some(item) = util::get(options.wrapping_add(index)) {
        let Some(text) = util::text(item) else {
            break;
        };
        if let Some((key, value)) = text.split_once('=') {
            found.push((key.trim().to_ascii_uppercase(), value.trim()));
        }
        index += 1;
    }
    found
}

fn yes(value: &str) -> bool {
    value.eq_ignore_ascii_case("YES")
}

fn convention_of(kind: PJ_WKT_TYPE) -> Option<WktConvention> {
    Some(match kind {
        PJ_WKT2_2015 => WktConvention::Wkt2_2015,
        PJ_WKT2_2015_SIMPLIFIED => WktConvention::Wkt2_2015Simplified,
        PJ_WKT2_2019 => WktConvention::Wkt2_2019,
        PJ_WKT2_2019_SIMPLIFIED => WktConvention::Wkt2_2019Simplified,
        PJ_WKT1_GDAL => WktConvention::Wkt1Gdal,
        PJ_WKT1_ESRI => WktConvention::Wkt1Esri,
        _ => return None,
    })
}

fn wkt_text(
    context: &mut Ctx,
    object: &Obj,
    convention: WktConvention,
    options: &[(String, &str)],
) -> Result<String, String> {
    let database = context.database().ok_or("no database")?;
    let mut formatter = WktFormatter::create(convention).with_database(database);
    for (key, value) in options {
        match key.as_str() {
            "MULTILINE" => {
                formatter.set_lines(if yes(value) {
                    WktLines::Multi
                } else {
                    WktLines::Single
                });
            }
            "INDENTATION_WIDTH" => {
                if let Ok(width) = value.parse::<usize>() {
                    formatter.set_indentation_width(width);
                }
            }
            "OUTPUT_AXIS" => {
                if value.eq_ignore_ascii_case("YES") {
                    formatter.set_output_axis(OutputAxisRule::Yes);
                } else if value.eq_ignore_ascii_case("NO") {
                    formatter.set_output_axis(OutputAxisRule::No);
                } else if !value.eq_ignore_ascii_case("AUTO") {
                    return Err(format!("Unsupported value for OUTPUT_AXIS: {value}"));
                }
            }
            "STRICT" => {
                formatter.set_strictness(if yes(value) {
                    WktStrictness::Strict
                } else {
                    WktStrictness::Lenient
                });
            }
            "ALLOW_ELLIPSOIDAL_HEIGHT_AS_VERTICAL_CRS" => {
                formatter.set_ellipsoidal_height(if yes(value) {
                    EllipsoidalHeightPolicy::AllowAsVertical
                } else {
                    EllipsoidalHeightPolicy::Reject
                });
            }
            "ALLOW_LINUNIT_NODE" => {
                formatter.set_linunit(if yes(value) {
                    LinunitPolicy::Allow
                } else {
                    LinunitPolicy::Reject
                });
            }
            other => return Err(format!("Unknown option :{other}")),
        }
    }
    let written = match object {
        Obj::User(UserObject::Crs(crs)) => crs.export_to_wkt(&mut formatter),
        Obj::User(UserObject::Operation(operation)) => operation.export_to_wkt(&mut formatter),
        Obj::User(UserObject::GeodeticDatum(datum)) => datum.export_to_wkt(&mut formatter),
        Obj::User(UserObject::VerticalDatum(datum)) => {
            Datum::Vertical(datum.clone()).export_to_wkt(&mut formatter)
        }
        Obj::User(UserObject::EngineeringDatum(datum)) => {
            Datum::Engineering(datum.clone()).export_to_wkt(&mut formatter)
        }
        Obj::User(UserObject::ParametricDatum(datum)) => datum.export_to_wkt(&mut formatter),
        Obj::User(UserObject::TemporalDatum(datum)) => datum.export_to_wkt(&mut formatter),
        Obj::User(UserObject::Ensemble(ensemble)) => ensemble.export_to_wkt(&mut formatter),
        Obj::User(UserObject::Ellipsoid(ellipsoid)) => ellipsoid.export_to_wkt(&mut formatter),
        Obj::User(UserObject::PrimeMeridian(meridian)) => {
            Ok(meridian.export_to_wkt(&mut formatter))
        }
        Obj::User(UserObject::CoordinateMetadata(metadata)) => {
            metadata.export_to_wkt(&mut formatter)
        }
        Obj::Cs(cs) => Ok(cs.export_to_wkt(&mut formatter)),
    };
    written.map_err(|error| error.to_string())
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_as_wkt(
    ctx: *mut PJ_CONTEXT,
    obj: *const PJ,
    type_: PJ_WKT_TYPE,
    options: *const *const c_char,
) -> *const c_char {
    guard(std::ptr::null(), || {
        let context = context(ctx);
        let Some(object) = pj(obj) else {
            context.fail(
                PROJ_ERR_OTHER_API_MISUSE as c_int,
                "proj_as_wkt: null object",
            );
            return std::ptr::null();
        };
        let Some(convention) = convention_of(type_) else {
            return std::ptr::null();
        };
        let Some(inner) = object.obj.as_ref() else {
            return std::ptr::null();
        };
        match wkt_text(context, inner, convention, &options_of(options)) {
            Ok(text) => object.arena.put(&text),
            Err(message) => {
                context.fail(PROJ_ERR_OTHER as c_int, &format!("proj_as_wkt: {message}"));
                std::ptr::null()
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_as_projjson(
    ctx: *mut PJ_CONTEXT,
    obj: *const PJ,
    options: *const *const c_char,
) -> *const c_char {
    guard(std::ptr::null(), || {
        let context = context(ctx);
        let Some(object) = pj(obj) else {
            context.fail(
                PROJ_ERR_OTHER_API_MISUSE as c_int,
                "proj_as_projjson: null object",
            );
            return std::ptr::null();
        };
        let (Some(obj), Some(database)) = (object.obj.as_ref(), context.database()) else {
            return std::ptr::null();
        };
        let mut format = JsonFormat::with_lines(JsonLines::Multi);
        for (key, value) in options_of(options) {
            match key.as_str() {
                "MULTILINE" => {
                    format = JsonFormat::with_lines(if yes(value) {
                        JsonLines::Multi
                    } else {
                        JsonLines::Single
                    });
                }
                "INDENTATION_WIDTH" => {
                    if let Ok(width) = value.parse::<usize>() {
                        format = format.with_indent_width(width);
                    }
                }
                "SCHEMA" => format = format.with_schema(value),
                _ => {}
            }
        }
        let exported = match obj {
            Obj::User(user) => user.export_to_json(&database, &format),
            Obj::Cs(system) => system.export_to_json_with(&database, &format),
        };
        match exported {
            Ok(text) => object.arena.put(&text),
            Err(error) => {
                context.fail(
                    PROJ_ERR_OTHER as c_int,
                    &format!("proj_as_projjson: {error}"),
                );
                std::ptr::null()
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_as_proj_string(
    ctx: *mut PJ_CONTEXT,
    obj: *const PJ,
    type_: PJ_PROJ_STRING_TYPE,
    options: *const *const c_char,
) -> *const c_char {
    guard(std::ptr::null(), || {
        let context = context(ctx);
        let Some(object) = pj(obj) else {
            context.fail(
                PROJ_ERR_OTHER_API_MISUSE as c_int,
                "proj_as_proj_string: null object",
            );
            return std::ptr::null();
        };
        let _ = type_;
        let mut lines = ProjLines::Single;
        let (mut multiline, mut indentation, mut max_line) = (false, 2_usize, 80_usize);
        let mut approx_tmerc = false;
        for (key, value) in options_of(options) {
            match key.as_str() {
                "MULTILINE" => {
                    multiline = yes(value);
                    if multiline {
                        lines = ProjLines::Multi;
                    }
                }
                "INDENTATION_WIDTH" => indentation = value.parse().unwrap_or(indentation),
                "MAX_LINE_LENGTH" => max_line = value.parse().unwrap_or(max_line),
                "USE_APPROX_TMERC" => approx_tmerc = yes(value),
                _ => {}
            }
        }
        let layout = if multiline {
            proj_rs::iso19111::ProjStringLayout::Multi {
                indentation_width: indentation,
                max_line_length: max_line,
            }
        } else {
            proj_rs::iso19111::ProjStringLayout::Single
        };
        let from_pipeline = object
            .proj
            .as_ref()
            .filter(|_| object.obj.as_ref().and_then(Obj::crs).is_none())
            .and_then(|proj| proj.as_proj_string(layout).ok());
        let text = match object.obj.as_ref() {
            _ if from_pipeline.is_some() => from_pipeline.ok_or_else(String::new),
            Some(Obj::User(UserObject::Crs(crs))) => {
                let exact = crs
                    .export_to_proj_string()
                    .map_err(|error| error.to_string());
                match (&exact, crs, approx_tmerc) {
                    (Ok(text), Crs::Projected(projected), true) => {
                        let approx = if text.contains("+proj=utm") {
                            projected.with_approx_utm()
                        } else {
                            projected.with_approx_tmerc()
                        };
                        approx
                            .export_to_proj_string()
                            .map_err(|error| error.to_string())
                    }
                    _ => exact,
                }
            }
            Some(Obj::User(UserObject::Operation(operation))) => context
                .database()
                .ok_or_else(|| "no database".to_owned())
                .and_then(|database| {
                    operation
                        .export_to_proj_string_with_database(&database, lines)
                        .map_err(|error| error.to_string())
                }),
            _ => match object.proj.as_ref() {
                Some(proj) if proj.pair_axis_swaps().is_some() => {
                    let text = if object.normalized {
                        proj.def()
                    } else {
                        proj.definition_in_authority_order()
                    };
                    text.map(|definition| with_plus(&definition))
                        .map_err(|error| error.to_string())
                }
                Some(proj) => proj
                    .as_proj_string(proj_rs::iso19111::ProjStringLayout::Single)
                    .map_err(|error| error.to_string()),
                None => Err("no PROJ string for this object".to_owned()),
            },
        };
        match text {
            Ok(text) => object.arena.put(&text),
            Err(message) => {
                context.fail(
                    PROJ_ERR_OTHER as c_int,
                    &format!("proj_as_proj_string: {message}"),
                );
                std::ptr::null()
            }
        }
    })
}

/// A copy of `object` made of another ISO object, in the same context.
pub(crate) fn derived(object: &Pj, obj: Obj) -> *mut PJ {
    let context = object.context();
    let proj = obj.engine_proj(context);
    hand_over(Pj::with_obj(object.ctx, obj, proj))
}

/// `proj_normalize_for_visualization` of a CRS or of an operation between two CRS: the object
/// with its axes in the order longitude, latitude (easting, northing).
pub(crate) fn normalized_for_visualization(object: &Pj) -> Option<*mut PJ> {
    match object.obj.as_ref()? {
        Obj::User(UserObject::Crs(crs)) => {
            let normalized = crs.normalize_for_visualization();
            Some(derived(object, Obj::User(UserObject::Crs(normalized))))
        }
        Obj::User(UserObject::Operation(operation)) => {
            crate::inspect::operation_source(operation)?;
            crate::inspect::operation_target(operation)?;
            let normalized = operation.normalize_for_visualization()?;
            Some(derived(
                object,
                Obj::User(UserObject::Operation(normalized)),
            ))
        }
        _ => None,
    }
}

/// A PROJ string with a `+` before each word, as `proj_as_proj_string` writes it.
fn with_plus(definition: &str) -> String {
    definition
        .split_whitespace()
        .map(|word| {
            if word.starts_with('+') {
                word.to_owned()
            } else {
                format!("+{word}")
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// What `proj_pj_info` says of an ISO operation: PROJ instantiates the operation from its PROJ
/// string, names it after the ISO object, and gives the string without its `+`.
pub(crate) struct OperationInfo {
    pub(crate) id: Option<String>,
    pub(crate) description: String,
    pub(crate) definition: String,
    pub(crate) has_inverse: bool,
    pub(crate) accuracy: f64,
}

pub(crate) fn operation_info(object: &Pj) -> Option<OperationInfo> {
    let operation = object.obj.as_ref().and_then(Obj::operation)?;
    let proj = object.proj.as_ref()?;
    if object.from_text {
        return None;
    }
    let database = object.context().database()?;
    let text = operation
        .export_to_proj_string_with_database(&database, ProjLines::Single)
        .ok()?;
    let mut words: Vec<String> = text
        .split_whitespace()
        .map(|word| word.trim_start_matches('+').to_owned())
        .collect();
    let bare = operation.source_crs().is_none()
        && !words.iter().any(|word| word == "proj=pipeline")
        && !words
            .iter()
            .any(|word| word.starts_with("ellps=") || word.starts_with("datum="));
    if bare {
        words.push("ellps=GRS80".to_owned());
    }
    let id = words
        .iter()
        .find_map(|word| word.strip_prefix("proj="))
        .map(str::to_owned);
    Some(OperationInfo {
        id,
        description: operation.name().to_owned(),
        definition: words.join(" "),
        has_inverse: proj.proj_info().has_inverse,
        accuracy: operation.accuracy().unwrap_or(match operation {
            CoordinateOperation::Conversion(_) => 0.0,
            _ => -1.0,
        }),
    })
}
