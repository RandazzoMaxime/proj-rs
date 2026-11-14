//! ISO 19111 objects, authority lookup in `proj.db`, and WKT export.
//!
//! The type names follow PROJ's C++ API (`GeographicCRS`, `AuthorityFactory`,
//! `WktFormatter`). See `docs/CRS-API.md`.

mod alternatives;
mod bound;
mod bound_export;
mod bound_import;
mod bound_ops;
mod buildop;
mod compound;
mod conversion_export;
mod convmethod;
mod coordinate_metadata;
mod crs_ops;
mod derived;
mod equiv;
mod esri_parammap;
mod factory;
mod geodetic_ops;
mod identify;
mod insert;
mod json;
mod json_read;
mod metadata;
mod model;
mod niche;
mod normalize;
mod parammap;
mod pipeline;
mod proj_crs;
mod proj_grid_substitution;
mod projfam;
mod projstring;
pub(crate) use projstring::PRIME_MERIDIANS;
pub(crate) use select::RemoteGrids;
mod select;
mod suggest;
mod transform_export;
mod wkt;
mod wkt1_syntax;
mod wkt_import;

pub(crate) use alternatives::Item;
pub use alternatives::{
    AlternativeError, Alternatives, BestPolicy, LogLevel, LogLine, WARN_SUFFIX,
};
pub use buildop::{AxisOrderReversal, OpProps};
pub use coordinate_metadata::{coordinate_metadata_from_urn, CoordinateMetadata};
pub use crs_ops::{ParameterLinearUnit, Wgs84Dynamic};
pub use equiv::{equivalent_name, equivalent_name_bigger_differences, Equivalence};
pub use factory::{
    AlternativeGridNames, AuthorityFactory, AuthorityObject, BallparkUse, CelestialBodyInfo,
    CoordinateOperationContext, CoordinateOperationFactory, CrsExtentUse, CrsInfo, DatabaseContext,
    DeprecatedCodes, GeodeticComponents, GridAvailabilityUse, GridDescription, GridInfo,
    GridLookup, IntermediateCrsUse, NameMatch, NameSpelling, ObjectType, OfficialName,
    SpatialCriterion, SupersededOperations, UnitInfo,
};
pub use insert::{insert_statements, suggests_code_for, CodeForm, ComponentCodes, InsertSession};
pub use json::{
    create_from_user_input, create_from_user_input_with_init_rules,
    create_from_user_input_with_optional_database, create_object_from_user_input, epsg_identity,
    InitRules, JsonFormat, JsonLines, UserObject,
};
pub use json_read::{create_object_from_json, JsonObject};
pub use metadata::{Citation, DataEpoch, Extent};
pub use model::{
    Axis, BoundCRS, BoundingBox, CartesianCS, CompoundCRS, ConcatenatedOperation, ConcatenatedStep,
    Conversion, CoordinateOperation, CoordinateSystem, Crs, Datum, DatumEnsemble,
    DerivedConversion, DerivedProjectedCRS, DynamicGeodeticReferenceFrame, Ellipsoid,
    EllipsoidShape, EllipsoidalCS, EngineeringCRS, EngineeringDatum, FactoryError, GeocentricCRS,
    GeodeticCRS, GeodeticReferenceFrame, GeographicCRS, GeoidModel, Identifier, Measure, Meridian,
    ObjectDomain, OperationMethod, OperationParameter, OperationParameterValue, OrdinalCS,
    Parameter, ParameterValue, PrimeMeridian, ProjectedCRS, RealizationMethod, SphericalCS,
    Transformation, UnitKind, UnitOfMeasure, VerticalCRS, VerticalCS, VerticalReferenceFrame,
};
pub use niche::{
    parametric_unit, DerivedCRS, DerivedKind, ParametricCRS, ParametricCS, ParametricDatum,
    TemporalCRS, TemporalCS, TemporalCsKind, TemporalDatum, TemporalKind,
    CALENDAR_PROLEPTIC_GREGORIAN,
};
pub(crate) use normalize::normalize_for_visualization;
pub(crate) use pipeline::Pipeline;
pub use pipeline::{ProjLines, ProjStringLayout, TmercExport};
pub use projstring::{create_crs_from_proj_string, create_crs_from_proj_string_with_database};
pub use select::{
    area_of_interest_from_name, best_pipeline_logged, best_pipeline_logged_with_authority,
    select_alternatives, select_alternatives_in_area, select_alternatives_with_authority,
    PipelineLog, Selection,
};
pub use select::{
    best_pipeline, best_pipeline_filtered, best_pipeline_filtered_with_authority,
    best_pipeline_with, best_pipeline_with_authority, only_best_default, Ballpark, BestOperation,
    CandidateFilter, OnlyBest,
};
pub(crate) use select::{facade_candidates, same_datum_noop, FacadeCandidates};
pub use wkt::{
    EllipsoidalHeightPolicy, LinunitPolicy, OutputAxisRule, WktConvention, WktFormatter, WktLines,
    WktStrictness,
};
pub use wkt_import::{WktDialect, WktNode, WktObject, WktParser};
