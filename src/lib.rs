//! PROJ-RS — Rust port of [PROJ](https://github.com/OSGeo/PROJ).
//!
//! Ported so far: exact Transverse Mercator (UTM), Lambert-93, Mercator and
//! 7-parameter Helmert. See `ROADMAP.md`.

mod angle;
mod coord;
mod ellipsoid;
mod error;
mod helmert;
mod lcc;
mod merc;
#[cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]
mod simd;
mod tmerc;

pub use coord::{Geographic, Projected};
pub use ellipsoid::Ellipsoid;
pub use error::{EllipsoidError, ProjectionError};
pub use helmert::{Helmert, HelmertConvention, HelmertParams};
pub use lcc::{Cone, Lcc};
pub use merc::Mercator;
pub use tmerc::{Hemisphere, Tmerc, UtmZone};

/// Version du crate, alignée sur `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
