#![allow(
    clippy::approx_constant,
    clippy::assign_op_pattern,
    clippy::enum_variant_names,
    clippy::excessive_precision,
    clippy::if_same_then_else,
    clippy::let_and_return,
    clippy::manual_clamp,
    clippy::manual_range_contains,
    clippy::needless_range_loop,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::too_many_arguments,
    reason = "PROJ sources are transcribed with their coefficients and branches"
)]
//! PROJ-RS — Rust port of [PROJ](https://github.com/OSGeo/PROJ) 9.9.
//!
//! The goal is the same observable behavior as PROJ 9.9, with `Result` and
//! enums instead of error codes. Build an operation from a PROJ string with
//! [`Operation::create`], or use the georust `proj`-compatible [`Proj`] facade.
//! EPSG codes resolve from the embedded PROJ 9.9 database. An unknown method
//! or a missing grid is an error at creation, never a coordinate.
//!
//! See `README.md` for the conformance status and `ROADMAP.md` for what is
//! left.

mod angle;
mod coord;
mod crs;
mod crs_text;
mod ellipsoid;
mod engine;
mod error;
#[cfg(feature = "geo-types")]
mod geo_impl;
mod geodesic;
mod gridpath;
mod helmert;
pub mod iso19111;
mod lcc;
mod merc;
mod network;
mod ported;
mod proj;
mod projdb;
mod projections;
mod projmath;
#[cfg(target_arch = "x86_64")]
mod simd;
mod tmerc;
mod tmerc_approx;
mod transform;

pub use engine::{
    lpz_dist, parse_gie_coord, xyz_dist, Coord as Xyzt, CreateError, CrsPipeline, Operation,
    TransformError as StepError,
};
pub use ported::{bench_definition, PORTED};
pub use proj::{
    Area, Coord, CoordinateType, Direction, Info, Proj, ProjBuilder, ProjCreateError, ProjError,
    ProjInfo,
};
pub use transform::{Transform, TransformError};

pub mod wkt {
    //! WKT export types from georust `proj` 0.31.
    //!
    //! [`crate::Proj::as_wkt`] returns WKT stored for an EPSG code, or the WKT
    //! text passed to [`crate::Proj::new`]. It does not invent a dialect.

    pub use crate::proj::{WktOptions, WktOutputAxis, WktVersion};
}

/// Path taken by [`Tmerc::forward_batch`] and [`Lcc::forward_batch`] on this host.
///
/// `avx2` is the vector path. On Windows it uses the CRT math library one lane
/// at a time, not glibc libmvec. `scalar` is the one-point loop.
#[must_use]
pub fn batch_path() -> &'static str {
    #[cfg(target_arch = "x86_64")]
    if std::is_x86_feature_detected!("avx2") && std::is_x86_feature_detected!("fma") {
        return "avx2";
    }
    "scalar"
}

pub use coord::{Geographic, Projected};
pub use ellipsoid::Ellipsoid;
pub use error::{EllipsoidError, ProjectionError};
pub use helmert::{Helmert, HelmertConvention, HelmertParams};
pub use iso19111::{
    create_crs_from_proj_string, create_from_user_input, epsg_identity, equivalent_name,
    equivalent_name_bigger_differences, insert_statements, AlternativeGridNames, AuthorityFactory,
    AuthorityObject, Axis, BallparkUse, BoundCRS, BoundingBox, CartesianCS, CelestialBodyInfo,
    CompoundCRS, ConcatenatedOperation, ConcatenatedStep, Conversion, CoordinateOperation,
    CoordinateOperationContext, CoordinateOperationFactory, CoordinateSystem, Crs, CrsExtentUse,
    CrsInfo, DatabaseContext, Datum, DatumEnsemble, DeprecatedCodes, DerivedConversion,
    DerivedProjectedCRS, EllipsoidalCS, EllipsoidalHeightPolicy, EngineeringCRS, EngineeringDatum,
    Equivalence, FactoryError, GeocentricCRS, GeodeticCRS, GeodeticReferenceFrame, GeographicCRS,
    Identifier, IntermediateCrsUse, LinunitPolicy, Meridian, NameMatch, NameSpelling, ObjectDomain,
    ObjectType, OfficialName, OpProps, OperationMethod, OperationParameter,
    OperationParameterValue, OrdinalCS, OutputAxisRule, Parameter, ParameterValue, PrimeMeridian,
    ProjLines, ProjectedCRS, SpatialCriterion, SphericalCS, Transformation, UnitInfo, UnitKind,
    UnitOfMeasure, VerticalCRS, VerticalCS, VerticalReferenceFrame, WktConvention, WktDialect,
    WktFormatter, WktLines, WktNode, WktObject, WktParser, WktStrictness,
};
pub use lcc::{Cone, Lcc};
pub use merc::Mercator;
pub use tmerc::{Hemisphere, Tmerc, UtmZone};

/// Version du crate, alignée sur `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
