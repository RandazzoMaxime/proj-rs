//! GeoTIFF shift grids.
//!
//! PROJ reads these through libtiff (`GTiffDataset::nextGrid` in `src/grids.cpp`).
//! The `tiff` crate decodes strips, tiles, endianness, BigTIFF, deflate and the
//! floating-point predictor. Georeferencing and GDAL metadata stay here.

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Seek};
use std::sync::{Arc, Mutex};

use tiff::decoder::ifd::Value;
use tiff::decoder::{Decoder, DecodingResult};
use tiff::tags::Tag;

use super::tiles::{ImageTiles, LazyIfd, Samples};
use super::{Domain, Extent};
use crate::network_chunks::{RemoteFile, RemoteReader};

/// The byte order of a TIFF file, from its first two bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ByteOrder {
    Little,
    Big,
}

#[derive(Clone, Copy)]
enum Storage {
    Chunky,
    Separate,
}

struct GdalFields {
    meta: BTreeMap<(i32, String), String>,
    scale: Vec<f64>,
    offset: Vec<f64>,
}

const GEO_PIXEL_SCALE: u16 = 33_550;
const GEO_TIE_POINTS: u16 = 33_922;
const GEO_MATRIX: u16 = 34_264;
const GEO_KEYS: u16 = 34_735;
const GDAL_METADATA: u16 = 42_112;
const GDAL_NODATA: u16 = 42_113;

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

/// Where the samples of an image go: decoded now, or left in the file for [`LazyIfd`].
#[derive(Clone, Copy)]
enum Mode<'a> {
    Eager,
    /// The tiles stay in `file`, read by [`LazyIfd::load`], in the file's byte `order`.
    Lazy {
        file: &'a Arc<Mutex<RemoteFile>>,
        order: ByteOrder,
    },
}

/// Why an image is not read: it is broken, or it is one the lazy reader does not handle.
enum IfdFail {
    Invalid,
    /// Strips, planar bands, compression or a predictor: the file is read whole.
    NotLazy,
}

impl From<()> for IfdFail {
    fn from((): ()) -> Self {
        Self::Invalid
    }
}

pub(super) fn load(bytes: &[u8]) -> Result<Vec<RawIfd>, ()> {
    let mut decoder = Decoder::new(Cursor::new(bytes)).map_err(|_| ())?;
    let mut out = Vec::new();
    loop {
        match read_ifd(&mut decoder, Mode::Eager) {
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
        let mode = Mode::Lazy { file, order };
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

fn read_ifd<R: Read + Seek>(decoder: &mut Decoder<R>, mode: Mode<'_>) -> Result<RawIfd, IfdFail> {
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
        Mode::Eager => Samples::Flat(decode_samples(decoder, width_u, height_u, spp, storage)?),
        Mode::Lazy { file, order } => {
            if !lazy_readable(decoder, storage)? {
                return Err(IfdFail::NotLazy);
            }
            let tile = decoder.chunk_dimensions();
            let offsets = decoder
                .get_tag_u64_vec(Tag::TileOffsets)
                .map_err(|_| IfdFail::Invalid)?;
            let counts = decoder
                .get_tag_u64_vec(Tag::TileByteCounts)
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

/// Whether the image's samples can be read tile by tile from the file: tiles, chunky samples,
/// no compression and no predictor. Anything else is read whole, as [`load`] reads it.
fn lazy_readable<R: Read + Seek>(decoder: &mut Decoder<R>, storage: Storage) -> Result<bool, ()> {
    if matches!(storage, Storage::Separate) || decoder.get_tag_u32(Tag::TileWidth).is_err() {
        return Ok(false);
    }
    let compression = optional_u32(decoder, Tag::Compression)?.unwrap_or(1);
    let predictor = optional_u32(decoder, Tag::Predictor)?.unwrap_or(1);
    Ok(compression == 1 && predictor == 1)
}

fn supported_sample(format: u32, bits: u32) -> bool {
    matches!(
        (format, bits),
        (1, 16) | (1, 32) | (2, 16) | (2, 32) | (3, 32) | (3, 64)
    )
}

fn decode_samples<R: Read + Seek>(
    decoder: &mut Decoder<R>,
    width: u32,
    height: u32,
    spp: usize,
    storage: Storage,
) -> Result<Vec<f32>, ()> {
    let (tile_w, tile_h) = decoder.chunk_dimensions();
    if tile_w == 0 || tile_h == 0 {
        return Err(());
    }
    let across = width.div_ceil(tile_w);
    let down = height.div_ceil(tile_h);
    let per_plane = across.checked_mul(down).ok_or(())?;
    let planes = if matches!(storage, Storage::Separate) {
        u32::try_from(spp).map_err(|_| ())?
    } else {
        1
    };
    let total = per_plane.checked_mul(planes).ok_or(())?;
    let width_us = usize::try_from(width).map_err(|_| ())?;
    let height_us = usize::try_from(height).map_err(|_| ())?;
    let len = width_us
        .checked_mul(height_us)
        .and_then(|pixels| pixels.checked_mul(spp))
        .ok_or(())?;
    if len > 64 * 1024 * 1024 {
        return Err(());
    }
    let mut dest = vec![0.0_f32; len];
    for index in 0..total {
        let chunk = decoder.read_chunk(index).map_err(|_| ())?;
        let values = to_f32(chunk)?;
        let (data_w, data_h) = decoder.chunk_data_dimensions(index);
        let local = index % per_plane;
        let plane = usize::try_from(index / per_plane).map_err(|_| ())?;
        let col = local % across;
        let row = local / across;
        let x0 = usize::try_from(col.saturating_mul(tile_w)).map_err(|_| ())?;
        let y0 = usize::try_from(row.saturating_mul(tile_h)).map_err(|_| ())?;
        let valid_w = width_us
            .saturating_sub(x0)
            .min(usize::try_from(tile_w).unwrap_or(0));
        let valid_h = height_us
            .saturating_sub(y0)
            .min(usize::try_from(tile_h).unwrap_or(0));
        let data_w_us = usize::try_from(data_w).map_err(|_| ())?;
        let data_h_us = usize::try_from(data_h).map_err(|_| ())?;
        let copy_w = valid_w.min(data_w_us);
        let copy_h = valid_h.min(data_h_us);
        place(
            &mut dest, width_us, spp, storage, plane, x0, y0, data_w_us, copy_w, copy_h, &values,
        )?;
    }
    Ok(dest)
}

fn place(
    dest: &mut [f32],
    width: usize,
    spp: usize,
    storage: Storage,
    plane: usize,
    x0: usize,
    y0: usize,
    data_w: usize,
    copy_w: usize,
    copy_h: usize,
    chunk: &[f32],
) -> Result<(), ()> {
    let separate = matches!(storage, Storage::Separate);
    let chunk_spp = if separate { 1 } else { spp };
    if separate && plane >= spp {
        return Err(());
    }
    for row in 0..copy_h {
        for col in 0..copy_w {
            let src = (row * data_w + col).checked_mul(chunk_spp).ok_or(())?;
            let dst = ((y0 + row) * width + (x0 + col))
                .checked_mul(spp)
                .ok_or(())?;
            if separate {
                let sample = chunk.get(src).copied().ok_or(())?;
                *dest.get_mut(dst + plane).ok_or(())? = sample;
            } else {
                for band in 0..spp {
                    let sample = chunk.get(src + band).copied().ok_or(())?;
                    *dest.get_mut(dst + band).ok_or(())? = sample;
                }
            }
        }
    }
    Ok(())
}

/// The samples of an uncompressed tile, as `f32`. `format` and `bits` are the image's
/// `SampleFormat` and `BitsPerSample`, already checked by [`supported_sample`].
pub(super) fn raw_samples(
    bytes: &[u8],
    format: u32,
    bits: u32,
    order: ByteOrder,
) -> Result<Vec<f32>, ()> {
    let size = usize::try_from(bits / 8).map_err(|_| ())?;
    if size == 0 || bytes.len() % size != 0 {
        return Err(());
    }
    let mut out = Vec::with_capacity(bytes.len() / size);
    for piece in bytes.chunks_exact(size) {
        let value = match (format, bits) {
            (1, 16) => f32::from(word::<u16>(piece, order)?),
            (2, 16) => f32::from(word::<i16>(piece, order)?),
            (1, 32) => f32_from_u32(word::<u32>(piece, order)?),
            (2, 32) => f32_from_i32(word::<i32>(piece, order)?),
            (3, 32) => word::<f32>(piece, order)?,
            (3, 64) => f32_from_f64(word::<f64>(piece, order)?),
            _ => return Err(()),
        };
        out.push(value);
    }
    Ok(out)
}

/// One sample of `piece`, in the file's byte order.
fn word<T: TiffWord>(piece: &[u8], order: ByteOrder) -> Result<T, ()> {
    T::from_piece(piece, order).ok_or(())
}

/// A sample type that a TIFF stores in a fixed number of bytes.
trait TiffWord: Sized {
    fn from_piece(piece: &[u8], order: ByteOrder) -> Option<Self>;
}

macro_rules! tiff_word {
    ($($ty:ty),*) => {
        $(
            impl TiffWord for $ty {
                fn from_piece(piece: &[u8], order: ByteOrder) -> Option<Self> {
                    let bytes: [u8; std::mem::size_of::<$ty>()] = piece.try_into().ok()?;
                    Some(if order == ByteOrder::Big {
                        <$ty>::from_be_bytes(bytes)
                    } else {
                        <$ty>::from_le_bytes(bytes)
                    })
                }
            }
        )*
    };
}

tiff_word!(u16, i16, u32, i32, f32, f64);

/// The samples of one decoded chunk, as `f32` (every grid sample is a float in PROJ).
pub(super) fn to_f32(decoded: DecodingResult) -> Result<Vec<f32>, ()> {
    match decoded {
        DecodingResult::U16(values) => Ok(values.into_iter().map(f32::from).collect()),
        DecodingResult::I16(values) => Ok(values.into_iter().map(f32::from).collect()),
        DecodingResult::U32(values) => Ok(values.into_iter().map(f32_from_u32).collect()),
        DecodingResult::I32(values) => Ok(values.into_iter().map(f32_from_i32).collect()),
        DecodingResult::F32(values) => Ok(values),
        DecodingResult::F64(values) => Ok(values.into_iter().map(f32_from_f64).collect()),
        _ => Err(()),
    }
}

fn f32_from_u32(value: u32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        clippy::as_conversions,
        reason = "PROJ stores every grid sample as float"
    )]
    {
        value as f32
    }
}

fn f32_from_i32(value: i32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        clippy::as_conversions,
        reason = "PROJ stores every grid sample as float"
    )]
    {
        value as f32
    }
}

fn f32_from_f64(value: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::as_conversions,
        reason = "PROJ stores every grid sample as float"
    )]
    {
        value as f32
    }
}

fn read_extent<R: Read + Seek>(
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

fn parse_gdal(text: &str, spp: usize) -> GdalFields {
    let mut meta = BTreeMap::new();
    let mut scale = Vec::new();
    let mut offset = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("<Item ") {
        rest = &rest[start..];
        let Some(end_tag) = rest.find('>') else { break };
        let Some(end_rel) = rest[end_tag + 1..].find('<') else {
            break;
        };
        let end_value = end_tag + 1 + end_rel;
        let tag = &rest[..end_tag];
        let value = &rest[end_tag + 1..end_value];
        let Some(name) = attr(tag, "name") else { break };
        let sample = attr(tag, "sample")
            .and_then(|text| text.parse::<i32>().ok())
            .unwrap_or(-1);
        if let Some(role) = attr(tag, "role") {
            if (role == "offset" || role == "scale")
                && sample >= 0
                && usize::try_from(sample).ok().is_some_and(|slot| slot < spp)
            {
                if scale.is_empty() {
                    scale.resize(spp, 1.0);
                    offset.resize(spp, 0.0);
                }
                if let (Ok(number), Ok(slot)) = (value.parse::<f64>(), usize::try_from(sample)) {
                    if role == "offset" {
                        offset[slot] = number;
                    } else {
                        scale[slot] = number;
                    }
                }
            }
        }
        meta.insert((sample, name.to_owned()), value.to_owned());
        rest = &rest[end_value + 1..];
    }
    GdalFields {
        meta,
        scale,
        offset,
    }
}

fn attr<'a>(tag: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("{key}=\"");
    let start = tag.find(&needle)? + needle.len();
    let end = tag[start..].find('"')? + start;
    Some(&tag[start..end])
}

fn parse_nodata(text: &str) -> Option<f32> {
    let text = text.trim_matches(|ch: char| ch == '\0' || ch.is_whitespace());
    let value = text.parse::<f64>().ok()?;
    Some(f32_from_f64(value))
}

fn required_u32<R: Read + Seek>(decoder: &mut Decoder<R>, tag: Tag) -> Result<u32, ()> {
    first_unsigned(decoder, tag)
}

fn optional_u32<R: Read + Seek>(decoder: &mut Decoder<R>, tag: Tag) -> Result<Option<u32>, ()> {
    match decoder.find_tag(tag).map_err(|_| ())? {
        Some(value) => Ok(Some(unsigned_value(value)?)),
        None => Ok(None),
    }
}

fn first_unsigned<R: Read + Seek>(decoder: &mut Decoder<R>, tag: Tag) -> Result<u32, ()> {
    let value = decoder.find_tag(tag).map_err(|_| ())?.ok_or(())?;
    unsigned_value(value)
}

fn unsigned_value(value: Value) -> Result<u32, ()> {
    match value {
        Value::List(items) => {
            let first = items.first().cloned().ok_or(())?;
            let number = first.into_u32().map_err(|_| ())?;
            for item in items.iter().skip(1) {
                if item.clone().into_u32().map_err(|_| ())? != number {
                    return Err(());
                }
            }
            Ok(number)
        }
        other => other.into_u32().map_err(|_| ()),
    }
}

fn optional_u16s<R: Read + Seek>(
    decoder: &mut Decoder<R>,
    tag: Tag,
) -> Result<Option<Vec<u16>>, ()> {
    match decoder.find_tag(tag).map_err(|_| ())? {
        Some(value) => Ok(Some(value.into_u16_vec().map_err(|_| ())?)),
        None => Ok(None),
    }
}

fn optional_f64s<R: Read + Seek>(
    decoder: &mut Decoder<R>,
    tag: Tag,
) -> Result<Option<Vec<f64>>, ()> {
    match decoder.find_tag(tag).map_err(|_| ())? {
        Some(value) => Ok(Some(value.into_f64_vec().map_err(|_| ())?)),
        None => Ok(None),
    }
}

fn optional_string<R: Read + Seek>(
    decoder: &mut Decoder<R>,
    tag: Tag,
) -> Result<Option<String>, ()> {
    match decoder.find_tag(tag).map_err(|_| ())? {
        Some(value) => Ok(Some(value.into_string().map_err(|_| ())?)),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::{raw_samples, ByteOrder};

    /// The samples of a tile are read in the file's byte order, whatever the sample type: a
    /// `u16` of `0x0102` is 258 in a big-endian file and 513 in a little-endian one.
    #[test]
    fn raw_samples_follow_the_file_byte_order() {
        assert_eq!(
            raw_samples(&[0x01, 0x02], 1, 16, ByteOrder::Big),
            Ok(vec![258.0])
        );
        assert_eq!(
            raw_samples(&[0x01, 0x02], 1, 16, ByteOrder::Little),
            Ok(vec![513.0])
        );
        assert_eq!(
            raw_samples(&[0xFF, 0xFE], 2, 16, ByteOrder::Big),
            Ok(vec![-2.0])
        );
        let float = 1.25_f32.to_be_bytes();
        assert_eq!(raw_samples(&float, 3, 32, ByteOrder::Big), Ok(vec![1.25]));
        let float = 1.25_f32.to_le_bytes();
        assert_eq!(
            raw_samples(&float, 3, 32, ByteOrder::Little),
            Ok(vec![1.25])
        );
    }

    /// A tile of a type the grid readers do not take, or a byte count that is not whole
    /// samples, is an error and not a short tile.
    #[test]
    fn raw_samples_refuse_other_types_and_partial_samples() {
        assert_eq!(raw_samples(&[0x01], 1, 8, ByteOrder::Big), Err(()));
        assert_eq!(
            raw_samples(&[0x01, 0x02, 0x03], 1, 16, ByteOrder::Big),
            Err(())
        );
    }
}
