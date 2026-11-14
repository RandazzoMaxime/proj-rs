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
mod context;
mod coord;
mod crs;
mod crs_text;
pub mod dms;
mod ellipsoid;
mod engine;
pub mod errno;
mod error;
#[cfg(feature = "geo-types")]
mod geo_impl;
pub mod geod;
mod geodesic;
pub mod gie;
mod gie_api;
mod gridpath;
mod helmert;
pub mod info;
pub mod iso19111;
mod lcc;
pub mod listing;
pub mod math;
mod merc;
pub mod names;
pub mod projinfo;
pub mod projsync;
pub mod version;

/// The deformation models (`+proj=defmodel`): the master file parser, the
/// time functions, and the evaluator over a [`GridProvider`](defmodel::GridProvider).
pub mod defmodel {
    pub use crate::projections::DefmodelGrid as Grid;
    pub use crate::projections::{
        decimal_year_from_iso8601, Authority, Bound, Component, DefmodelError, Displacement, Epoch,
        EpochScaleFactor, Evaluator, GridProvider, GridSet, Link, MasterFile, SpatialExtent,
        SpatialModel, TimeExtent, TimeFunction,
    };
}

/// The triangulation shift files (`+proj=tinshift`): the file parser with its
/// header accessors, and the evaluator.
pub mod tinshift {
    pub use crate::projections::{
        FallbackStrategy, TinShiftEvaluator, TinShiftFile, TinShiftFileError,
    };
}
mod network;
pub use network::{NetworkCallbacks, NetworkError, NetworkHandle, OpenReply};
mod network_cache;
mod ported;
mod proj;
mod proj_info;
mod proj_iso;
mod proj_meta;
mod projdb;
mod projections;
mod projmath;
#[cfg(target_arch = "x86_64")]
mod simd;
mod tmerc;
mod tmerc_approx;
mod transform;

pub use context::{Context, ContextBuilder, DatabaseSource};
pub use engine::{
    lpz_dist, parse_gie_coord, xyz_dist, Coord as Xyzt, CreateError, CrsPipeline, Operation,
    TransformError as StepError,
};
pub use gie_api::{
    list_ellps, list_operations, list_prime_meridians, xy_dist, BatchError, Factors,
};
pub use ported::{bench_definition, PORTED};
pub use proj::{
    Area, Coord, CoordinateType, Direction, ForceOver, GridCacheMode, GridCacheTtl, Info, Proj,
    ProjBuilder, ProjCreateError, ProjError, ProjInfo,
};
pub use transform::{Transform, TransformError};

/// Read access to PROJ shift grids (`grids.hpp`): horizontal, vertical and
/// generic sets, their grids and subgrids, pixels and metadata.
pub mod grids {
    pub use crate::projections::{
        ExtentAndRes, GenericShiftGridSet, Grid, GridError, HorizontalShiftGridSet, Nodata,
        NtConvention, VerticalShiftGridSet,
    };
}

pub mod wkt {
    //! WKT export types from georust `proj` 0.31.
    //!
    //! [`crate::Proj::as_wkt`] writes the CRS of a [`crate::Proj`] with the ISO 19111
    //! exporter, in the dialect asked for (WKT2:2019 by default). A transform between two
    //! CRS is not written: its WKT export is not built, and the call returns an error.

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
    create_crs_from_proj_string, create_crs_from_proj_string_with_database, create_from_user_input,
    epsg_identity, equivalent_name, equivalent_name_bigger_differences, insert_statements,
    parametric_unit, AlternativeGridNames, AuthorityFactory, AuthorityObject, Axis,
    AxisOrderReversal, BallparkUse, BoundCRS, BoundingBox, CartesianCS, CelestialBodyInfo,
    Citation, CompoundCRS, ConcatenatedOperation, ConcatenatedStep, Conversion,
    CoordinateOperation, CoordinateOperationContext, CoordinateOperationFactory, CoordinateSystem,
    Crs, CrsExtentUse, CrsInfo, DataEpoch, DatabaseContext, Datum, DatumEnsemble, DeprecatedCodes,
    DerivedConversion, DerivedKind, DerivedProjectedCRS, EllipsoidShape, EllipsoidalCS,
    EllipsoidalHeightPolicy, EngineeringCRS, EngineeringDatum, Equivalence, Extent, FactoryError,
    GeocentricCRS, GeodeticCRS, GeodeticComponents, GeodeticReferenceFrame, GeographicCRS,
    GridAvailabilityUse, GridDescription, GridInfo, Identifier, IntermediateCrsUse, LinunitPolicy,
    Measure, Meridian, NameMatch, NameSpelling, ObjectDomain, ObjectType, OfficialName, OpProps,
    OperationMethod, OperationParameter, OperationParameterValue, OrdinalCS, OutputAxisRule,
    Parameter, ParameterLinearUnit, ParameterValue, PrimeMeridian, ProjLines, ProjStringLayout,
    ProjectedCRS, RealizationMethod, SpatialCriterion, SphericalCS, SupersededOperations,
    TemporalKind, TmercExport, Transformation, UnitInfo, UnitKind, UnitOfMeasure, VerticalCRS,
    VerticalCS, VerticalReferenceFrame, Wgs84Dynamic, WktConvention, WktDialect, WktFormatter,
    WktLines, WktNode, WktObject, WktParser, WktStrictness, CALENDAR_PROLEPTIC_GREGORIAN,
};
pub use iso19111::{
    create_from_user_input_with_init_rules, suggests_code_for, CodeForm, ComponentCodes, InitRules,
    InsertSession,
};
pub use iso19111::{
    DerivedCRS, ParametricCRS, ParametricCS, ParametricDatum, TemporalCRS, TemporalCS,
    TemporalCsKind, TemporalDatum,
};
pub use lcc::{Cone, Lcc};
pub use merc::Mercator;
pub use tmerc::{Hemisphere, Tmerc, UtmZone};

/// Version du crate, alignée sur `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
