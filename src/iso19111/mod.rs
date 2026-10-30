//! ISO 19111 objects, authority lookup in `proj.db`, and WKT export.
//!
//! The type names follow PROJ's C++ API (`GeographicCRS`, `AuthorityFactory`,
//! `WktFormatter`). See `docs/CRS-API.md`.

mod bound;
mod buildop;
mod equiv;
mod factory;
mod identify;
mod insert;
mod json;
mod model;
mod parammap;
mod pipeline;
mod projstring;
mod select;
mod transform_export;
mod wkt;
mod wkt_import;

pub use buildop::OpProps;
pub use equiv::{equivalent_name, equivalent_name_bigger_differences, Equivalence};
pub use factory::{
    AlternativeGridNames, AuthorityFactory, AuthorityObject, BallparkUse, CelestialBodyInfo,
    CoordinateOperationContext, CoordinateOperationFactory, CrsExtentUse, CrsInfo, DatabaseContext,
    DeprecatedCodes, IntermediateCrsUse, NameMatch, NameSpelling, ObjectType, OfficialName,
    SpatialCriterion, UnitInfo,
};
pub use insert::insert_statements;
pub use json::{create_from_user_input, epsg_identity};
pub use model::{
    Axis, BoundCRS, BoundingBox, CartesianCS, CompoundCRS, ConcatenatedOperation, ConcatenatedStep,
    Conversion, CoordinateOperation, CoordinateSystem, Crs, Datum, DatumEnsemble,
    DerivedConversion, DerivedProjectedCRS, Ellipsoid, EllipsoidalCS, EngineeringCRS,
    EngineeringDatum, FactoryError, GeocentricCRS, GeodeticCRS, GeodeticReferenceFrame,
    GeographicCRS, Identifier, Meridian, ObjectDomain, OperationMethod, OperationParameter,
    OperationParameterValue, OrdinalCS, Parameter, ParameterValue, PrimeMeridian, ProjectedCRS,
    SphericalCS, Transformation, UnitKind, UnitOfMeasure, VerticalCRS, VerticalCS,
    VerticalReferenceFrame,
};
pub use pipeline::ProjLines;
pub use projstring::create_crs_from_proj_string;
pub use select::best_pipeline;
pub use wkt::{
    EllipsoidalHeightPolicy, LinunitPolicy, OutputAxisRule, WktConvention, WktFormatter, WktLines,
    WktStrictness,
};
pub use wkt_import::{WktDialect, WktNode, WktObject, WktParser};
