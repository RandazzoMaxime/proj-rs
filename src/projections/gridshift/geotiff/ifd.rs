//! One image of a TIFF: its header, its georeferencing, its metadata, and its samples, decoded
//! now or left in the file for the lazy reader.

use std::collections::BTreeMap;
use std::io::{Read, Seek};
use std::sync::{Arc, Mutex};

use tiff::decoder::Decoder;
use tiff::tags::Tag;

use super::coded::{codec_tags, coded_of, decode_coded_samples, Whole};
use super::gdal::{parse_gdal, parse_nodata, GdalFields, GDAL_METADATA, GDAL_NODATA};
use super::georef::read_extent;
use super::lazy::lazy_layout;
use super::samples::decode_samples;
use super::tags::{first_unsigned, optional_string, optional_u32, required_u32};
use super::RawIfd;
use crate::network_chunks::RemoteFile;
use crate::projections::gridshift::tiles::{ImageTiles, Layout, LazyIfd, Samples, Storage};
use crate::projections::tiff_codec::ByteOrder;

/// Where the samples of an image go: decoded now, or left in the file for [`LazyIfd`].
#[derive(Clone, Copy)]
pub(super) enum Mode<'a> {
    /// The samples are decoded now, in the file's byte `order`.
    Eager { order: ByteOrder },
    /// The chunks stay in `file`, read by [`LazyIfd::load`], in the file's byte `order`. `ifd` is
    /// the index of the image in the file.
    Lazy {
        file: &'a Arc<Mutex<RemoteFile>>,
        order: ByteOrder,
        ifd: usize,
    },
}

/// Why an image is not read: it is broken, or it is one the lazy reader does not handle.
pub(super) enum IfdFail {
    Invalid,
    /// A compression or a predictor that the decoder of the lazy reader does not undo: the file is
    /// read whole.
    NotLazy,
}

impl From<()> for IfdFail {
    fn from((): ()) -> Self {
        Self::Invalid
    }
}

pub(super) fn read_ifd<R: Read + Seek>(
    decoder: &mut Decoder<R>,
    mode: Mode<'_>,
) -> Result<RawIfd, IfdFail> {
    let (width_u, height_u) = decoder.dimensions().map_err(|_| ())?;
    if width_u == 0 || height_u == 0 {
        return Err(IfdFail::Invalid);
    }
    let width = i32::try_from(width_u).map_err(|_| ())?;
    let height = i32::try_from(height_u).map_err(|_| ())?;
    let spp_u = required_u32(decoder, Tag::SamplesPerPixel)?;
    if spp_u == 0 {
        return Err(IfdFail::Invalid);
    }
    let spp = usize::try_from(spp_u).map_err(|_| ())?;
    let bits = first_unsigned(decoder, Tag::BitsPerSample)?;
    let format = first_unsigned(decoder, Tag::SampleFormat)?;
    if !supported_sample(format, bits) {
        return Err(IfdFail::Invalid);
    }
    let planar = required_u32(decoder, Tag::PlanarConfiguration)?;
    if planar != 1 && planar != 2 {
        return Err(IfdFail::Invalid);
    }
    if let Some(photo) = optional_u32(decoder, Tag::PhotometricInterpretation)? {
        if photo != 1 {
            return Err(IfdFail::Invalid);
        }
    }
    let subfile = optional_u32(decoder, Tag::NewSubfileType)?.unwrap_or(0);
    let (extent, bottom_up) = read_extent(decoder, width_u, height_u)?;
    let storage = if planar == 2 {
        Storage::Separate
    } else {
        Storage::Chunky
    };
    let samples = match mode {
        Mode::Eager { order } => {
            let (compression, predictor) = codec_tags(decoder)?;
            match coded_of(compression, predictor, bits, format) {
                Some(coded) => {
                    let whole = Whole {
                        size: (width_u, height_u),
                        spp,
                        storage,
                        bits,
                        format,
                        order,
                    };
                    Samples::Flat(decode_coded_samples(decoder, coded, &whole)?)
                }
                None => Samples::Flat(decode_samples(decoder, width_u, height_u, spp, storage)?),
            }
        }
        Mode::Lazy { file, order, ifd } => {
            let Some((layout, reading)) = lazy_layout(decoder, bits, format)? else {
                return Err(IfdFail::NotLazy);
            };
            let (offset_tag, count_tag) = match layout {
                Layout::Tiles => (Tag::TileOffsets, Tag::TileByteCounts),
                Layout::Strips => (Tag::StripOffsets, Tag::StripByteCounts),
            };
            let tile = decoder.chunk_dimensions();
            let offsets = decoder
                .get_tag_u64_vec(offset_tag)
                .map_err(|_| IfdFail::Invalid)?;
            let counts = decoder
                .get_tag_u64_vec(count_tag)
                .map_err(|_| IfdFail::Invalid)?;
            if offsets.len() != counts.len() {
                return Err(IfdFail::Invalid);
            }
            let image = ImageTiles {
                size: (width_u, height_u),
                tile,
                spp,
                bits,
                format,
                order,
                layout,
                storage,
                reading,
                ifd,
                places: offsets.into_iter().zip(counts).collect(),
            };
            Samples::Lazy(Arc::new(LazyIfd::new(Arc::clone(file), image)?))
        }
    };
    let text = optional_string(decoder, Tag::from_u16_exhaustive(GDAL_METADATA))?;
    let fields = text
        .map(|value| parse_gdal(&value, spp))
        .unwrap_or(GdalFields {
            meta: BTreeMap::new(),
            scale: Vec::new(),
            offset: Vec::new(),
        });
    let meta = fields.meta;
    let scale = fields.scale;
    let offset = fields.offset;
    let nodata = optional_string(decoder, Tag::from_u16_exhaustive(GDAL_NODATA))?
        .as_deref()
        .and_then(parse_nodata);
    Ok(RawIfd {
        width,
        height,
        extent,
        bottom_up,
        spp,
        samples,
        scale,
        offset,
        nodata,
        meta,
        subfile,
    })
}

fn supported_sample(format: u32, bits: u32) -> bool {
    matches!(
        (format, bits),
        (1, 16) | (1, 32) | (2, 16) | (2, 32) | (3, 32) | (3, 64)
    )
}
