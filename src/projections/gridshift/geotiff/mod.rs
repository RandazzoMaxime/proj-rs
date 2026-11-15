//! GeoTIFF shift grids.
//!
//! PROJ reads these through libtiff (`GTiffDataset::nextGrid` in `src/grids.cpp`). The `tiff` crate
//! decodes strips, tiles, endianness, BigTIFF, deflate and the floating-point predictor; LZW is
//! decoded by `tiff_codec`. Georeferencing and GDAL metadata stay here.

mod coded;
mod gdal;
mod georef;
mod ifd;
mod lazy;
mod samples;
mod tags;

pub(super) use samples::{raw_samples, to_f32};

use std::collections::BTreeMap;
use std::io::Cursor;
use std::sync::{Arc, Mutex};

use tiff::decoder::Decoder;

use ifd::{read_ifd, IfdFail, Mode};

use super::tiles::Samples;
use super::Extent;
use crate::network_chunks::{RemoteFile, RemoteReader};
use crate::projections::tiff_codec::ByteOrder;

pub(super) struct RawIfd {
    pub width: i32,
    pub height: i32,
    pub extent: Extent,
    pub bottom_up: bool,
    pub spp: usize,
    pub samples: Samples,
    pub scale: Vec<f64>,
    pub offset: Vec<f64>,
    pub nodata: Option<f32>,
    pub meta: BTreeMap<(i32, String), String>,
    pub subfile: u32,
}

pub(super) fn is_tiff(bytes: &[u8]) -> bool {
    bytes.len() >= 4
        && ((bytes[0] == b'I' && bytes[1] == b'I') || (bytes[0] == b'M' && bytes[1] == b'M'))
        && ((bytes[2] == 0x2A && bytes[3] == 0)
            || (bytes[3] == 0x2A && bytes[2] == 0)
            || (bytes[2] == 0x2B && bytes[3] == 0)
            || (bytes[3] == 0x2B && bytes[2] == 0))
}

pub(super) fn load(bytes: &[u8]) -> Result<Vec<RawIfd>, ()> {
    let order = if bytes.starts_with(b"MM") {
        ByteOrder::Big
    } else {
        ByteOrder::Little
    };
    let mut decoder = Decoder::new(Cursor::new(bytes)).map_err(|_| ())?;
    let mut out = Vec::new();
    loop {
        match read_ifd(&mut decoder, Mode::Eager { order }) {
            Ok(ifd) => out.push(ifd),
            Err(_) => {
                if out.is_empty() {
                    return Err(());
                }
                break;
            }
        }
        if !decoder.more_images() {
            break;
        }
        if decoder.next_image().is_err() {
            break;
        }
    }
    if out.is_empty() {
        Err(())
    } else {
        Ok(out)
    }
}

/// The images of a TIFF read through `file`, with their tiles left in the file until a sample of
/// them is needed. `Ok(None)` when an image is one this does not read by tile: the caller then
/// reads the whole file and calls [`load`]. `Err` when the file is not a TIFF it can read.
pub(super) fn load_remote(file: &Arc<Mutex<RemoteFile>>) -> Result<Option<Vec<RawIfd>>, ()> {
    // The byte order is the first two bytes of the file, which the decoder reads too.
    let head = file
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .read_at(0, 2)
        .map_err(|_| ())?;
    let order = if head.as_slice() == b"MM" {
        ByteOrder::Big
    } else {
        ByteOrder::Little
    };
    let mut decoder = Decoder::new(RemoteReader::new(Arc::clone(file))).map_err(|_| ())?;
    let mut out = Vec::new();
    loop {
        let mode = Mode::Lazy {
            file,
            order,
            ifd: out.len(),
        };
        match read_ifd(&mut decoder, mode) {
            Ok(ifd) => out.push(ifd),
            Err(IfdFail::NotLazy) => return Ok(None),
            Err(IfdFail::Invalid) => {
                if out.is_empty() {
                    return Err(());
                }
                break;
            }
        }
        if !decoder.more_images() {
            break;
        }
        if decoder.next_image().is_err() {
            break;
        }
    }
    Ok(Some(out))
}
