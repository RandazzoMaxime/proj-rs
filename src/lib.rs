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
//! PROJ-RS — Rust port of [PROJ](https://github.com/OSGeo/PROJ).
//!
//! Ported so far: exact Transverse Mercator (UTM), Lambert-93, Mercator and
//! 7-parameter Helmert. See `ROADMAP.md`.

mod angle;
mod coord;
mod crs;
mod ellipsoid;
mod engine;
mod error;
#[cfg(feature = "geo-types")]
mod geo_impl;
mod geodesic;
mod helmert;
mod lcc;
mod merc;
mod ported;
mod proj;
mod projections;
mod projmath;
#[cfg(target_arch = "x86_64")]
mod simd;
mod tmerc;
mod tmerc_approx;
mod transform;

pub use engine::{
    lpz_dist, parse_gie_coord, xyz_dist, Coord as Xyzt, CreateError, Operation,
    TransformError as StepError,
};
pub use ported::{bench_definition, PORTED};
pub use proj::{
    Area, Coord, CoordinateType, Info, Proj, ProjBuilder, ProjCreateError, ProjError, ProjInfo,
};
pub use transform::{Transform, TransformError};

pub mod wkt {
    //! WKT export types from georust `proj` 0.31.
    //!
    //! [`crate::Proj::as_wkt`] returns an error. The types exist so callers can name them.

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
pub use lcc::{Cone, Lcc};
pub use merc::Mercator;
pub use tmerc::{Hemisphere, Tmerc, UtmZone};

/// Version du crate, alignée sur `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
