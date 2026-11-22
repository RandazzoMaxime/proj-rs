//! cs2cs-style modifiers on a classic operation.
//!
//! PROJ `cs2cs_emulation_setup` in `src/create.cpp` attaches `hgridshift`,
//! `helmert`, `cart`, `vgridshift`, and `axisswap` around the projection.
//! Forward order is the one in `src/fwd.cpp`: horizontal datum, geoid, the
//! projection, then axis order. Inverse is `src/inv.cpp`.

use crate::engine::{Coord, CreateError, Frame, IoKind, Operation, Params, Step, TransformError};
use crate::Ellipsoid;

type SharedStep = std::sync::Arc<dyn Step>;

struct DatumCart {
    wgs84: SharedStep,
    local: SharedStep,
    helmert: Option<SharedStep>,
}

pub(super) struct Legacy {
    inner: SharedStep,
    horizontal: Option<SharedStep>,
    datum: Option<DatumCart>,
    vertical: Option<SharedStep>,
    axis: Option<SharedStep>,
    in_kind: IoKind,
    out_kind: IoKind,
    a: f64,
    es: f64,
}

pub(super) fn applies(name: &str, params: &Params) -> bool {
    if matches!(
        name,
        "helmert"
            | "hgridshift"
            | "vgridshift"
            | "gridshift"
            | "xyzgridshift"
            | "cart"
            | "geocent"
            | "axisswap"
            | "molodensky"
            | "molobadekas"
            | "push"
            | "pop"
            | "set"
            | "unitconvert"
            | "deformation"
            | "defmodel"
            | "tinshift"
            | "affine"
            | "noop"
    ) {
        return false;
    }
    params.flag("towgs84")
        || params.flag("nadgrids")
        || params.flag("geoidgrids")
        || swapped(params)
}

fn swapped(params: &Params) -> bool {
    params.text("axis").is_some_and(|text| text != "enu")
}

pub(super) fn wrap(
    inner: SharedStep,
    params: &Params,
    frame: &Frame,
) -> Result<SharedStep, CreateError> {
    let horizontal = if params.flag("nadgrids") {
        let grids = params.text("nadgrids").unwrap_or("");
        if grids.is_empty() {
            return Err(CreateError::IllegalValue("nadgrids"));
        }
        // A name with white space is quoted, as `pj_mkparam_ws` reads it back.
        let grids = if grids.contains(char::is_whitespace) {
            format!("\"{grids}\"")
        } else {
            grids.to_owned()
        };
        Some(open(&format!("+proj=hgridshift +grids={grids}"))?)
    } else {
        None
    };
    let datum = if horizontal.is_none() && params.flag("towgs84") {
        datum_cart(params, frame)?
    } else {
        None
    };
    let vertical = if params.flag("geoidgrids") {
        let grids = params.text("geoidgrids").unwrap_or("");
        if grids.is_empty() {
            return Err(CreateError::IllegalValue("geoidgrids"));
        }
        Some(open(&format!("+proj=vgridshift +grids={grids}"))?)
    } else {
        None
    };
    let axis = if let Some(text) = params.text("axis") {
        if text == "enu" {
            None
        } else {
            Some(open(&format!("+proj=axisswap +axis={text}"))?)
        }
    } else {
        None
    };
    let in_kind = inner.input_kind();
    let out_kind = inner.output_kind();
    let (a, es) = inner.distance_ellipsoid();
    Ok(std::sync::Arc::from(Legacy {
        inner,
        horizontal,
        datum,
        vertical,
        axis,
        in_kind,
        out_kind,
        a,
        es,
    }))
}

fn open(definition: &str) -> Result<SharedStep, CreateError> {
    Ok(Operation::create(definition)?.step)
}

fn datum_cart(params: &Params, frame: &Frame) -> Result<Option<DatumCart>, CreateError> {
    let packed = super::datum_ops::towgs84_values(params.text("towgs84").unwrap_or(""))?;
    let zero = packed.iter().all(|value| *value == 0.0);
    if zero && wgs84_earth(frame.earth) {
        return Ok(None);
    }
    let helmert = if zero {
        None
    } else {
        let text = params.text("towgs84").unwrap_or("");
        Some(open(&format!(
            "+proj=helmert +exact +towgs84={text} +convention=position_vector"
        ))?)
    };
    Ok(Some(DatumCart {
        wgs84: open("+proj=cart +ellps=WGS84")?,
        local: open(&local_cart(params, frame))?,
        helmert,
    }))
}

fn local_cart(params: &Params, frame: &Frame) -> String {
    if let Some(name) = params.text("ellps") {
        return format!("+proj=cart +ellps={name}");
    }
    let earth = frame.earth;
    if earth.flattening() == 0.0 {
        format!("+proj=cart +R={}", number(earth.a()))
    } else {
        format!(
            "+proj=cart +a={} +rf={}",
            number(earth.a()),
            number(1.0 / earth.flattening())
        )
    }
}

fn number(value: f64) -> String {
    format!("{value:.16}")
}

fn wgs84_earth(earth: Ellipsoid) -> bool {
    (earth.a() - 6_378_137.0).abs() < 1e-8 && (earth.es() - 0.006_694_379_990_141_3).abs() < 1e-15
}

impl Legacy {
    fn datum_forward(&self, mut coord: Coord) -> Result<Coord, TransformError> {
        if let Some(horizontal) = &self.horizontal {
            return horizontal.inverse(coord);
        }
        if let Some(datum) = &self.datum {
            coord = datum.wgs84.forward(coord)?;
            if let Some(helmert) = &datum.helmert {
                coord = helmert.inverse(coord)?;
            }
            coord = datum.local.inverse(coord)?;
        }
        Ok(coord)
    }

    fn datum_inverse(&self, mut coord: Coord) -> Result<Coord, TransformError> {
        if let Some(horizontal) = &self.horizontal {
            return horizontal.forward(coord);
        }
        if let Some(datum) = &self.datum {
            coord = datum.local.forward(coord)?;
            if let Some(helmert) = &datum.helmert {
                coord = helmert.forward(coord)?;
            }
            coord = datum.wgs84.inverse(coord)?;
        }
        Ok(coord)
    }

    fn before_inner(&self, mut coord: Coord) -> Result<Coord, TransformError> {
        coord = self.datum_forward(coord)?;
        if let Some(vertical) = &self.vertical {
            coord = vertical.forward(coord)?;
        }
        Ok(coord)
    }

    fn after_inner(&self, mut coord: Coord) -> Result<Coord, TransformError> {
        if let Some(vertical) = &self.vertical {
            coord = vertical.inverse(coord)?;
        }
        self.datum_inverse(coord)
    }
}

/// The default of `Step::forward_batch`, for a step that holds the pipeline stack.
fn one_by_one(
    coords: &mut [Coord],
    failed: &mut [Option<TransformError>],
    step: impl Fn(Coord) -> Result<Coord, TransformError>,
) {
    for (coord, failed) in coords.iter_mut().zip(failed) {
        if failed.is_none() {
            match step(*coord) {
                Ok(out) => *coord = out,
                Err(error) => *failed = Some(error),
            }
        }
    }
}

impl Legacy {
    /// `datum_forward` and the vertical grid over a slice, step by step.
    fn before_inner_batch(&self, coords: &mut [Coord], failed: &mut [Option<TransformError>]) {
        if let Some(horizontal) = &self.horizontal {
            horizontal.inverse_batch(coords, failed);
        } else if let Some(datum) = &self.datum {
            datum.wgs84.forward_batch(coords, failed);
            if let Some(helmert) = &datum.helmert {
                helmert.inverse_batch(coords, failed);
            }
            datum.local.inverse_batch(coords, failed);
        }
        if let Some(vertical) = &self.vertical {
            vertical.forward_batch(coords, failed);
        }
    }

    /// The vertical grid and `datum_inverse` over a slice, step by step.
    fn after_inner_batch(&self, coords: &mut [Coord], failed: &mut [Option<TransformError>]) {
        if let Some(vertical) = &self.vertical {
            vertical.inverse_batch(coords, failed);
        }
        if let Some(horizontal) = &self.horizontal {
            horizontal.forward_batch(coords, failed);
        } else if let Some(datum) = &self.datum {
            datum.local.forward_batch(coords, failed);
            if let Some(helmert) = &datum.helmert {
                helmert.forward_batch(coords, failed);
            }
            datum.wgs84.inverse_batch(coords, failed);
        }
    }
}

impl Step for Legacy {
    fn forward_batch(&self, coords: &mut [Coord], failed: &mut [Option<TransformError>]) {
        assert_eq!(coords.len(), failed.len(), "one slot per point");
        if self.inner.needs_pipeline_stack() {
            return one_by_one(coords, failed, |coord| self.forward(coord));
        }
        self.before_inner_batch(coords, failed);
        self.inner.forward_batch(coords, failed);
        if let Some(axis) = &self.axis {
            axis.forward_batch(coords, failed);
        }
    }

    fn inverse_batch(&self, coords: &mut [Coord], failed: &mut [Option<TransformError>]) {
        assert_eq!(coords.len(), failed.len(), "one slot per point");
        if self.inner.needs_pipeline_stack() {
            return one_by_one(coords, failed, |coord| self.inverse(coord));
        }
        if let Some(axis) = &self.axis {
            axis.inverse_batch(coords, failed);
        }
        self.inner.inverse_batch(coords, failed);
        self.after_inner_batch(coords, failed);
    }

    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        let coord = self.inner.forward(self.before_inner(coord)?)?;
        if let Some(axis) = &self.axis {
            axis.forward(coord)
        } else {
            Ok(coord)
        }
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        let coord = if let Some(axis) = &self.axis {
            axis.inverse(coord)?
        } else {
            coord
        };
        self.after_inner(self.inner.inverse(coord)?)
    }

    fn input_kind(&self) -> IoKind {
        self.in_kind
    }

    fn output_kind(&self) -> IoKind {
        self.out_kind
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (self.a, self.es)
    }

    /// A geoid grid sets the height. A datum shift on its own is 2D and does not.
    fn computes_height(&self) -> bool {
        self.inner.computes_height() || self.vertical.is_some()
    }

    fn needs_pipeline_stack(&self) -> bool {
        self.inner.needs_pipeline_stack()
    }

    fn forward_in_pipeline(
        &self,
        coord: Coord,
        stack: &mut crate::engine::PipelineStack,
    ) -> Result<Coord, TransformError> {
        let coord = self
            .inner
            .forward_in_pipeline(self.before_inner(coord)?, stack)?;
        if let Some(axis) = &self.axis {
            axis.forward(coord)
        } else {
            Ok(coord)
        }
    }

    fn inverse_in_pipeline(
        &self,
        coord: Coord,
        stack: &mut crate::engine::PipelineStack,
    ) -> Result<Coord, TransformError> {
        let coord = if let Some(axis) = &self.axis {
            axis.inverse(coord)?
        } else {
            coord
        };
        self.after_inner(self.inner.inverse_in_pipeline(coord, stack)?)
    }
}
