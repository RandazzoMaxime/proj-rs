#![deny(missing_docs)]
//! A drop-in replacement for the georust [`proj`](https://crates.io/crates/proj) 0.31 crate, on
//! the [proj-rs](https://github.com/RandazzoMaxime/proj-rs) engine.
//!
//! The library is named `proj` and has the names and signatures of `proj` 0.31, so a project
//! changes one line of its `Cargo.toml` and no line of code:
//!
//! ```toml
//! proj = { package = "proj-rs-georust", version = "1" }
//! ```
//!
//! No libproj is linked and no C toolchain is needed: PROJ 9.9 is ported to Rust. The
//! examples of the `proj` 0.31 documentation run as tests of this crate (`tests/`).
//!
//! ```
//! use proj::Proj;
//!
//! let from = "EPSG:2230";
//! let to = "EPSG:26946";
//! let ft_to_m = Proj::new_known_crs(from, to, None).unwrap();
//! let (x, y) = ft_to_m.convert((4_760_096.421_921_f64, 3_744_293.729_449_f64)).unwrap();
//! assert!((x - 1_450_880.29).abs() < 1e-2 && (y - 1_141_263.01).abs() < 1e-2);
//! ```
//!
//! The `geo-types` feature, on by default, implements [`Coord`] and [`Transform`] for the
//! `geo-types` geometries. The `network`, `tiff`, `bundled_proj` and `pkg_config` features of
//! `proj` exist; `network` enables the HTTP(S) client of proj-rs, the others change nothing.

pub use proj_rs::unstable::{
    Area, Coord, Info, Proj, ProjBuilder, ProjCreateError, ProjError, ProjInfo, Transform,
    TransformError,
};

pub mod wkt {
    //! WKT export types of `proj` 0.31.

    pub use proj_rs::unstable::wkt::{WktOptions, WktOutputAxis, WktVersion};
}
