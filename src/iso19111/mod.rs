//! ISO 19111 objects, authority lookup in `proj.db`, and WKT export.
//!
//! The type names follow PROJ's C++ API (`GeographicCRS`, `AuthorityFactory`,
//! `WktFormatter`). See `docs/CRS-API.md`.

mod bound;
mod bound_export;
mod bound_import;
mod bound_ops;
mod buildop;
mod compound;
mod conversion_export;
mod convmethod;
mod derived;
mod equiv;
mod esri_parammap;
mod factory;
mod geodetic_ops;
mod identify;
mod insert;
mod json;
mod json_read;
mod model;
mod niche;
mod normalize;
mod parammap;
mod pipeline;
mod proj_crs;
mod projfam;
mod projstring;
mod select;
mod transform_export;
mod wkt;
mod wkt_import;

pub use buildop::{AxisOrderReversal, OpProps};
pub use equiv::{equivalent_name, equivalent_name_bigger_differences, Equivalence};
pub use factory::{
    AlternativeGridNames, AuthorityFactory, AuthorityObject, BallparkUse, CelestialBodyInfo,
    CoordinateOperationContext, CoordinateOperationFactory, CrsExtentUse, CrsInfo, DatabaseContext,
    DeprecatedCodes, GeodeticComponents, GridAvailabilityUse, GridInfo, IntermediateCrsUse,
    NameMatch, NameSpelling, ObjectType, OfficialName, SpatialCriterion, SupersededOperations,
    UnitInfo,
};
pub use insert::insert_statements;
pub use json::{
    create_from_user_input, create_object_from_user_input, epsg_identity, JsonFormat, JsonLines,
    UserObject,
};
pub use json_read::{create_object_from_json, JsonObject};
pub use model::{
    Axis, BoundCRS, BoundingBox, CartesianCS, CompoundCRS, ConcatenatedOperation, ConcatenatedStep,
    Conversion, CoordinateOperation, CoordinateSystem, Crs, Datum, DatumEnsemble,
    DerivedConversion, DerivedProjectedCRS, Ellipsoid, EllipsoidalCS, EngineeringCRS,
    EngineeringDatum, FactoryError, GeocentricCRS, GeodeticCRS, GeodeticReferenceFrame,
    GeographicCRS, Identifier, Measure, Meridian, ObjectDomain, OperationMethod,
    OperationParameter, OperationParameterValue, OrdinalCS, Parameter, ParameterValue,
    PrimeMeridian, ProjectedCRS, SphericalCS, Transformation, UnitKind, UnitOfMeasure, VerticalCRS,
    VerticalCS, VerticalReferenceFrame,
};
pub use niche::{
    parametric_unit, DerivedCRS, DerivedKind, ParametricCRS, ParametricCS, ParametricDatum,
    TemporalCRS, TemporalCS, TemporalCsKind, TemporalDatum, TemporalKind,
    CALENDAR_PROLEPTIC_GREGORIAN,
};
pub(crate) use normalize::normalize_for_visualization;
pub use pipeline::ProjLines;
pub use projstring::{create_crs_from_proj_string, create_crs_from_proj_string_with_database};
pub(crate) use select::{best_operation_exact, lat_or_northing_first, same_datum_noop};
pub use select::{best_pipeline, best_pipeline_with, BestOperation, OnlyBest};
pub use wkt::{
    EllipsoidalHeightPolicy, LinunitPolicy, OutputAxisRule, WktConvention, WktFormatter, WktLines,
    WktStrictness,
};
pub use wkt_import::{WktDialect, WktNode, WktObject, WktParser};
