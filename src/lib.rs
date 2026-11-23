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
#![allow(
    rustdoc::private_intra_doc_links,
    rustdoc::bare_urls,
    reason = "two doc comments of the hidden ISO 19111 internals (factory.rs, wkt_import.rs) predate the façade and are not edited by it"
)]
#![deny(missing_docs)]
//! PROJ-RS — a Rust port of [PROJ](https://github.com/OSGeo/PROJ) 9.9.
//!
//! The result of a transformation is the one PROJ 9.9.0 gives, to the tolerance of
//! `docs/PARITY.md`. EPSG codes resolve from the `proj.db` built into the crate. An unknown
//! method or a missing grid is an error at creation, never a wrong coordinate.
//!
//! This page is the **stable surface** of the crate: [`Context`], [`Crs`], [`Transformer`]
//! with its [`TransformerBuilder`], [`Point`], [`Backend`] and [`Error`], the catalogue of the
//! database ([`CrsCatalog`], [`CrsFilter`], [`CrsEntry`]), its [`DatabaseMetadata`], the PROJ
//! version ([`proj_version`]), and the small types they take. Everything else is public for the binaries and the tests of this repository,
//! hidden from the documentation, and outside semver. The `unstable` feature shows it under
//! [`unstable`].
//!
//! ```
//! # fn main() -> Result<(), proj_rs::Error> {
//! use proj_rs::{Context, Crs, Point, Transformer};
//!
//! let ctx = Context::default();
//! let src = Crs::from_user_input(&ctx, "EPSG:4326")?;
//! let dst = Crs::from_user_input(&ctx, "EPSG:2154")?;
//! let t = Transformer::builder(&src, &dst)
//!     .normalize_axis_order() // longitude, latitude, as a GIS reads them
//!     .build()?;
//! let p = t.forward(Point::new(2.35, 48.85))?;
//! assert!((p.x - 652_301.56).abs() < 1e-2 && (p.y - 6_861_302.73).abs() < 1e-2);
//! # Ok(())
//! # }
//! ```
//!
//! Two other crates of the workspace share this engine: `proj-rs-georust`, a drop-in for the
//! georust `proj` 0.31 crate, and `proj-rs-capi`, the C API.
//!
//! See `README.md` for the conformance status, `docs/API-v1.md` for the stability promise and
//! `ROADMAP.md` for what is left.

mod angle;
mod api;
mod compat;
mod context;
mod coord;
mod crs;
mod crs_text;
mod datadir;
#[doc(hidden)]
pub mod dms;
mod ellipsoid;
mod engine;
#[doc(hidden)]
pub mod errno;
mod error;
mod fmath;
#[cfg(feature = "geo-types")]
mod geo_impl;
#[doc(hidden)]
pub mod geod;
pub mod geodesic;
#[doc(hidden)]
pub mod gie;
mod gie_api;
mod gridpath;
mod helmert;
#[doc(hidden)]
pub mod info;
#[doc(hidden)]
pub mod iso19111;
mod lcc;
#[doc(hidden)]
pub mod listing;
#[doc(hidden)]
pub mod math;
mod merc;
#[doc(hidden)]
pub mod names;
#[doc(hidden)]
pub mod projinfo;
#[doc(hidden)]
pub mod projsync;
#[doc(hidden)]
pub mod version;

/// The deformation models (`+proj=defmodel`): the master file parser, the
/// time functions, and the evaluator over a [`GridProvider`](defmodel::GridProvider).
#[doc(hidden)]
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
#[doc(hidden)]
pub mod tinshift {
    pub use crate::projections::{
        FallbackStrategy, TinShiftEvaluator, TinShiftFile, TinShiftFileError,
    };
}
mod file_api;
mod network;
#[cfg(feature = "network")]
mod network_builtin;
mod network_cache;
mod network_chunks;
mod network_download;
#[cfg(feature = "network")]
mod network_http;
mod ported;
mod proj;
mod proj_info;
mod proj_iso;
mod proj_meta;
mod projdb;
#[cfg(target_os = "linux")]
mod projdb_exe;
mod projections;
mod projmath;
mod simd;
mod tmerc;
mod tmerc_approx;
mod transform;

pub use api::{
    proj_version, BBox, Backend, Ballpark, Cache, CatalogKind, Context, ContextBuilder, Crs,
    CrsCatalog, CrsEntry, CrsFilter, CrsKind, DatabaseMetadata, DeprecatedCrs, Error, Network,
    OnlyBest, OperationInfo, Point, ProjVersion, Transformer, TransformerBuilder, WktVersion,
};

/// Read access to PROJ shift grids (`grids.hpp`): horizontal, vertical and
/// generic sets, their grids and subgrids, pixels and metadata.
#[doc(hidden)]
pub mod grids {
    pub use crate::projections::{
        ExtentAndRes, GenericShiftGridSet, Grid, GridError, HorizontalShiftGridSet, Nodata,
        NtConvention, VerticalShiftGridSet,
    };
}

/// A read-only connection to the `proj.db` embedded in the program, for the command-line tools of
/// this repository, which query it with SQL. The path of the build machine is not used.
///
/// # Errors
///
/// The message of the SQLite failure when the embedded bytes cannot be opened.
#[doc(hidden)]
pub fn embedded_proj_db() -> Result<rusqlite::Connection, String> {
    projdb::read_only_connection().map_err(|error| error.to_string())
}

/// `projsync --system-directory`: `pj_get_relative_share_proj`, else the compiled data directory
/// (`projsync.cpp` 147-153). `None` when neither exists: `projsync` then uses the user directory.
#[doc(hidden)]
#[must_use]
pub fn system_data_directory() -> Option<std::path::PathBuf> {
    datadir::relative_share_proj()
        .map(std::path::Path::to_path_buf)
        .or_else(|| datadir::compiled_data_dir().map(std::path::PathBuf::from))
}

#[doc(hidden)]
pub mod wkt {
    //! WKT export types from georust `proj` 0.31.
    //!
    //! [`crate::unstable::Proj::as_wkt`] writes the CRS of a [`crate::unstable::Proj`] with the
    //! ISO 19111 exporter, in the dialect asked for (WKT2:2019 by default). A transform between
    //! two CRS is not written: its WKT export is not built, and the call returns an error.

    pub use crate::proj::{WktOptions, WktOutputAxis, WktVersion};
}

/// Path taken by the batch conversions (`forward_batch` and `inverse_batch` of [`Tmerc`],
/// [`Lcc`] and [`Mercator`]) on this host.
///
/// `avx2` is the vector path on x86-64 with AVX2 and FMA, and `neon` the one on aarch64, with this
/// crate's own vector math functions (no libmvec, no Accelerate, no C library calls) on every
/// platform. The two give the same bits. `portable` is the path elsewhere:
/// [`Tmerc::forward_batch`] runs a 4-lane Rust loop, bit-identical to the scalar path; the other
/// batch functions run the one-point loop (`docs/PORTING-batch-path.md`).
#[doc(hidden)]
#[must_use]
pub fn batch_path() -> &'static str {
    match simd::batch_path() {
        "avx2" => "avx2",
        "neon" => "neon",
        _ => "portable",
    }
}

/// Version du crate, alignée sur `Cargo.toml`.
#[doc(hidden)]
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The internals of the engine: the ISO 19111 model, the operation engine, the named
/// projections, the test and command-line back ends, and the georust `proj` 0.31 surface.
///
/// None of this is covered by semver. It is public because the binaries and the tests of
/// this repository use it, and the `proj-rs-georust` crate is built on it. It is shown in the
/// documentation only with the `unstable` feature.
///
/// The names of the stable surface that an internal item shares (`Context`, `ContextBuilder`,
/// `Crs`) name the internal item here.
#[cfg_attr(not(feature = "unstable"), doc(hidden))]
pub mod unstable {
    pub use crate::context::{Context, ContextBuilder, DatabaseSource};
    pub use crate::engine::{
        lpz_dist, parse_gie_coord, xyz_dist, Coord as Xyzt, CreateError, CrsPipeline, Operation,
        ParamUse, TransformError as StepError,
    };
    pub use crate::file_api::{ApiFile, FileAccess, FileApi};
    pub use crate::gie_api::{
        list_ellps, list_operations, list_prime_meridians, xy_dist, BatchError, Factors,
    };
    pub use crate::network::{NetworkCallbacks, NetworkError, NetworkHandle, OpenReply};
    pub use crate::ported::{bench_definition, PORTED};
    pub use crate::proj::{
        Area, Coord, CoordinateType, Direction, DownloadProgress, ForceOver, GridCacheMode,
        GridCacheTtl, Info, PairError, Proj, ProjBuilder, ProjCreateError, ProjError, ProjInfo,
        TtlSetting,
    };
    pub use crate::transform::{Transform, TransformError};
    pub use crate::{
        batch_path, defmodel, dms, errno, geod, gie, grids, info, iso19111, listing, math, names,
        projinfo, projsync, tinshift, version, wkt, VERSION,
    };

    pub use crate::coord::{Geographic, Projected};
    pub use crate::ellipsoid::Ellipsoid;
    pub use crate::error::{EllipsoidError, ProjectionError};
    pub use crate::helmert::{Helmert, HelmertConvention, HelmertParams};
    pub use crate::iso19111::Crs;
    pub use crate::iso19111::{
        create_crs_from_proj_string, create_crs_from_proj_string_with_database,
        create_from_user_input, epsg_identity, equivalent_name, equivalent_name_bigger_differences,
        insert_statements, parametric_unit, AlternativeGridNames, AuthorityFactory,
        AuthorityObject, Axis, AxisOrderReversal, BallparkUse, BoundCRS, BoundingBox, CartesianCS,
        CelestialBodyInfo, Citation, CompoundCRS, ConcatenatedOperation, ConcatenatedStep,
        Conversion, CoordinateOperation, CoordinateOperationContext, CoordinateOperationFactory,
        CoordinateSystem, CrsExtentUse, CrsInfo, DataEpoch, DatabaseContext, Datum, DatumEnsemble,
        DeprecatedCodes, DerivedConversion, DerivedKind, DerivedProjectedCRS, EllipsoidShape,
        EllipsoidalCS, EllipsoidalHeightPolicy, EngineeringCRS, EngineeringDatum, Equivalence,
        Extent, FactoryError, GeocentricCRS, GeodeticCRS, GeodeticComponents,
        GeodeticReferenceFrame, GeographicCRS, GridAvailabilityUse, GridDescription, GridInfo,
        GridLookup, Identifier, IntermediateCrsUse, LinunitPolicy, Measure, Meridian, NameMatch,
        NameSpelling, ObjectDomain, ObjectType, OfficialName, OpProps, OperationMethod,
        OperationParameter, OperationParameterValue, OrdinalCS, OutputAxisRule, Parameter,
        ParameterLinearUnit, ParameterValue, PrimeMeridian, ProjLines, ProjStringLayout,
        ProjectedCRS, RealizationMethod, SpatialCriterion, SphericalCS, SupersededOperations,
        TemporalKind, TmercExport, Transformation, UnitInfo, UnitKind, UnitOfMeasure, VerticalCRS,
        VerticalCS, VerticalReferenceFrame, Wgs84Dynamic, WktConvention, WktDialect, WktFormatter,
        WktLines, WktNode, WktObject, WktParser, WktStrictness, CALENDAR_PROLEPTIC_GREGORIAN,
    };
    pub use crate::iso19111::{
        create_from_user_input_with_init_rules, suggests_code_for, CodeForm, ComponentCodes,
        InitRules, InsertSession,
    };
    pub use crate::iso19111::{
        DerivedCRS, ParametricCRS, ParametricCS, ParametricDatum, TemporalCRS, TemporalCS,
        TemporalCsKind, TemporalDatum,
    };
    pub use crate::lcc::{Cone, Lcc};
    pub use crate::merc::Mercator;
    pub use crate::tmerc::{Hemisphere, Tmerc, UtmZone};
}

// The paths the binaries and the tests of this repository used before the façade, kept
// hidden: `proj_rs::Proj`, `proj_rs::Operation`, `proj_rs::Tmerc`, ... The names of the
// façade (`Context`, `ContextBuilder`, `Crs`) are the façade's here; the internal items
// of those names are under `unstable`.
#[doc(hidden)]
pub use unstable::*;
