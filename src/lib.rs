//! PROJ-RS — portage Rust de [PROJ](https://github.com/OSGeo/PROJ).
//!
//! Les lots 2 et 3 portent l'UTM (Transverse Mercator exact) et Lambert-93.

mod angle;
mod coord;
mod ellipsoid;
mod error;
mod helmert;
mod lcc;
mod merc;
#[cfg(target_arch = "x86_64")]
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
