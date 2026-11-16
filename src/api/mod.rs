//! The native façade: the stable surface of the crate root.
//!
//! It adds no transformation. A [`Transformer`] selects its operation with the engine of
//! `iso19111::select`, as `cs2cs` does, and runs it on [`crate::unstable::Operation`].

mod backend;
mod context;
mod crs;
mod error;
mod point;
mod transformer;

pub use backend::Backend;
pub use context::{Cache, Context, ContextBuilder, Network};
pub use crs::{BBox, Crs, CrsKind, WktVersion};
pub use error::Error;
pub use point::Point;
pub use transformer::{Ballpark, OnlyBest, OperationInfo, Transformer, TransformerBuilder};
