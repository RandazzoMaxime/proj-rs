//! Reorder and flip coordinate axes.
//!
//! `+order` describes the output axes. `+axis` describes the input order in
//! the classic `enu` letters. The step does not convert units, so a pipeline
//! may place it between angular or projected steps.

#![allow(
    clippy::pedantic,
    reason = "axis indexing and the duplicate check follow axisswap.cpp"
)]

use crate::engine::{Coord, CreateError, Frame, IoKind, Params, Step, TransformError};

struct AxisSwap {
    axis: [usize; 4],
    sign: [f64; 4],
    count: usize,
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "axisswap" {
        Some(&["order", "axis"])
    } else {
        None
    }
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    _frame: &Frame,
) -> Result<Option<Box<dyn Step>>, CreateError> {
    if name != "axisswap" {
        return Ok(None);
    }
    let has_order = params.flag("order");
    let has_axis = params.flag("axis");
    if has_order == has_axis {
        return Err(CreateError::IllegalValue("order"));
    }
    let mut axis = [4, 5, 6, 7];
    let mut sign = [1.0, 1.0, 1.0, 1.0];
    let count = if has_order {
        let order = params.text("order").unwrap_or("");
        if order.is_empty()
            || !order
                .bytes()
                .all(|byte| matches!(byte, b'1'..=b'4' | b',' | b'-'))
        {
            return Err(CreateError::IllegalValue("order"));
        }
        let mut count = 0usize;
        for piece in order.split(',') {
            if count == 4 || piece.is_empty() {
                return Err(CreateError::IllegalValue("order"));
            }
            let number: i32 = piece
                .parse()
                .map_err(|_| CreateError::IllegalValue("order"))?;
            let index = match number.unsigned_abs() {
                1 => 0,
                2 => 1,
                3 => 2,
                4 => 3,
                _ => return Err(CreateError::IllegalValue("order")),
            };
            axis[count] = index;
            sign[count] = if number > 0 { 1.0 } else { -1.0 };
            count += 1;
        }
        count
    } else {
        let letters = params.text("axis").unwrap_or("");
        let chars: Vec<char> = letters.chars().collect();
        if chars.len() != 3 {
            return Err(CreateError::IllegalValue("axis"));
        }
        for (slot, letter) in chars.iter().enumerate() {
            let (index, flipped) = match letter {
                'e' => (0, false),
                'w' => (0, true),
                'n' => (1, false),
                's' => (1, true),
                'u' => (2, false),
                'd' => (2, true),
                _ => return Err(CreateError::IllegalValue("axis")),
            };
            axis[slot] = index;
            sign[slot] = if flipped { -1.0 } else { 1.0 };
        }
        3
    };
    for left in 0..4 {
        for right in 0..4 {
            if left != right && axis[left] == axis[right] {
                return Err(CreateError::IllegalValue("order"));
            }
        }
    }
    let legal = match count {
        4 => true,
        3 => axis[..3].iter().all(|index| *index < 3),
        2 => axis[0] < 2 && axis[1] < 2,
        _ => false,
    };
    if !legal {
        return Err(CreateError::IllegalValue("order"));
    }
    Ok(Some(Box::new(AxisSwap { axis, sign, count })))
}

impl Step for AxisSwap {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        Ok(self.permute(coord, false))
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        Ok(self.permute(coord, true))
    }

    fn input_kind(&self) -> IoKind {
        IoKind::Any
    }

    fn output_kind(&self) -> IoKind {
        IoKind::Any
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (1.0, 0.0)
    }
}

impl AxisSwap {
    fn permute(&self, coord: Coord, inverse: bool) -> Coord {
        let input = [coord.x, coord.y, coord.z, coord.t];
        let mut output = input;
        if inverse {
            for slot in 0..self.count {
                output[self.axis[slot]] = input[slot] * self.sign[slot];
            }
        } else {
            for slot in 0..self.count {
                output[slot] = input[self.axis[slot]] * self.sign[slot];
            }
        }
        Coord {
            x: output[0],
            y: output[1],
            z: output[2],
            t: output[3],
        }
    }
}
