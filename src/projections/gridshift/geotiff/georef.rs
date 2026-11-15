//! The extent and the resolution of a GeoTIFF grid, from its GeoKeys, its affine matrix, or its
//! pixel scale and tie point, as PROJ's `GTiffDataset::nextGrid` reads them.

use std::io::{Read, Seek};

use tiff::decoder::Decoder;
use tiff::tags::Tag;

use super::super::{Domain, Extent};
use super::tags::{optional_f64s, optional_u16s};

const GEO_PIXEL_SCALE: u16 = 33_550;
const GEO_TIE_POINTS: u16 = 33_922;
const GEO_MATRIX: u16 = 34_264;
const GEO_KEYS: u16 = 34_735;

pub(super) fn read_extent<R: Read + Seek>(
    decoder: &mut Decoder<R>,
    width: u32,
    height: u32,
) -> Result<(Extent, bool), ()> {
    let mut geographic = true;
    let mut pixel_is_area = false;
    if let Some(keys) = optional_u16s(decoder, Tag::from_u16_exhaustive(GEO_KEYS))? {
        if keys.len() < 4 || keys.len() % 4 != 0 || keys[0] != 1 {
            return Err(());
        }
        let mut index = 4;
        while index + 3 < keys.len() {
            let key = keys[index];
            let value = keys[index + 3];
            if key == 1024 {
                if value == 1 {
                    geographic = false;
                } else if value != 2 {
                    return Err(());
                }
            } else if key == 1025 && value == 1 {
                pixel_is_area = true;
            }
            index += 4;
        }
    }
    let matrix = optional_f64s(decoder, Tag::from_u16_exhaustive(GEO_MATRIX))?;
    let (mut west, mut north, h_res, v_res) = if let Some(matrix) = matrix.filter(|m| m.len() == 16)
    {
        if matrix[1] != 0.0 || matrix[4] != 0.0 {
            return Err(());
        }
        (matrix[3], matrix[7], matrix[0], -matrix[5])
    } else {
        let scale = optional_f64s(decoder, Tag::from_u16_exhaustive(GEO_PIXEL_SCALE))?.ok_or(())?;
        if scale.len() != 3 {
            return Err(());
        }
        let tie = optional_f64s(decoder, Tag::from_u16_exhaustive(GEO_TIE_POINTS))?.ok_or(())?;
        if tie.len() != 6 {
            return Err(());
        }
        let h_res = scale[0];
        let v_res = scale[1];
        (
            tie[3] - tie[0] * h_res,
            tie[4] + tie[1] * v_res,
            h_res,
            v_res,
        )
    };
    if pixel_is_area {
        west += 0.5 * h_res;
        north -= 0.5 * v_res;
    }
    let span_x = f64_u32(width.saturating_sub(1));
    let span_y = f64_u32(height.saturating_sub(1));
    let mul = if geographic {
        std::f64::consts::PI / 180.0
    } else {
        1.0
    };
    let mut north_m = north * mul;
    let mut south_m = (north - v_res * span_y) * mul;
    let west_m = west * mul;
    let east_m = (west + h_res * span_x) * mul;
    let res_x = h_res * mul;
    let res_y = v_res.abs() * mul;
    if v_res < 0.0 {
        std::mem::swap(&mut north_m, &mut south_m);
    }
    let domain = if geographic {
        Domain::Geographic
    } else {
        Domain::Projected
    };
    let extent = Extent::try_new(domain, west_m, south_m, east_m, north_m, res_x, res_y)?;
    Ok((extent, v_res < 0.0))
}

fn f64_u32(value: u32) -> f64 {
    #[expect(
        clippy::as_conversions,
        reason = "a grid dimension is far below the 53-bit mantissa"
    )]
    {
        value as f64
    }
}
