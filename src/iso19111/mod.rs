//! ISO 19111 objects, authority lookup in `proj.db`, and WKT export.
//!
//! The type names follow PROJ's C++ API (`GeographicCRS`, `AuthorityFactory`,
//! `WktFormatter`). See `docs/CRS-API.md`.

mod buildop;
mod equiv;
mod factory;
mod identify;
mod json;
mod model;
mod parammap;
mod projstring;
mod wkt;
mod wkt_import;

pub use buildop::OpProps;
pub use equiv::Equivalence;
pub use factory::{
    AlternativeGridNames, AuthorityFactory, AuthorityObject, CoordinateOperationContext,
    CoordinateOperationFactory, CrsInfo, DatabaseContext, DeprecatedCodes, IntermediateCrsUse,
    ObjectType,
};
pub use json::{create_from_user_input, epsg_identity};
pub use model::{
    Axis, BoundCRS, CartesianCS, CompoundCRS, ConcatenatedOperation, ConcatenatedStep, Conversion,
    CoordinateOperation, CoordinateSystem, Crs, Datum, DatumEnsemble, DerivedConversion,
    DerivedProjectedCRS, Ellipsoid, EllipsoidalCS, EngineeringCRS, EngineeringDatum, FactoryError,
    GeocentricCRS, GeodeticCRS, GeodeticReferenceFrame, GeographicCRS, Identifier, Meridian,
    ObjectDomain, OperationMethod, OperationParameter, OperationParameterValue, OrdinalCS,
    Parameter, ParameterValue, PrimeMeridian, ProjectedCRS, SphericalCS, Transformation, UnitKind,
    UnitOfMeasure, VerticalCRS, VerticalCS, VerticalReferenceFrame,
};
pub use projstring::create_crs_from_proj_string;
pub use wkt::{
    EllipsoidalHeightPolicy, LinunitPolicy, OutputAxisRule, WktConvention, WktFormatter, WktLines,
    WktStrictness,
};
pub use wkt_import::{WktDialect, WktNode, WktObject, WktParser};
