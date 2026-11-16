//! Reads of TIFF tags by number, through the `tiff` decoder. A required tag that is missing, or
//! of another type, is an error; an optional one is `None`.

use std::io::{Read, Seek};

use tiff::decoder::ifd::Value;
use tiff::decoder::Decoder;
use tiff::tags::Tag;

pub(super) fn required_u32<R: Read + Seek>(decoder: &mut Decoder<R>, tag: Tag) -> Result<u32, ()> {
    first_unsigned(decoder, tag)
}

pub(super) fn optional_u32<R: Read + Seek>(
    decoder: &mut Decoder<R>,
    tag: Tag,
) -> Result<Option<u32>, ()> {
    match decoder.find_tag(tag).map_err(|_| ())? {
        Some(value) => Ok(Some(unsigned_value(value)?)),
        None => Ok(None),
    }
}

pub(super) fn first_unsigned<R: Read + Seek>(
    decoder: &mut Decoder<R>,
    tag: Tag,
) -> Result<u32, ()> {
    let value = decoder.find_tag(tag).map_err(|_| ())?.ok_or(())?;
    unsigned_value(value)
}

pub(super) fn unsigned_value(value: Value) -> Result<u32, ()> {
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

pub(super) fn optional_u16s<R: Read + Seek>(
    decoder: &mut Decoder<R>,
    tag: Tag,
) -> Result<Option<Vec<u16>>, ()> {
    match decoder.find_tag(tag).map_err(|_| ())? {
        Some(value) => Ok(Some(value.into_u16_vec().map_err(|_| ())?)),
        None => Ok(None),
    }
}

pub(super) fn optional_f64s<R: Read + Seek>(
    decoder: &mut Decoder<R>,
    tag: Tag,
) -> Result<Option<Vec<f64>>, ()> {
    match decoder.find_tag(tag).map_err(|_| ())? {
        Some(value) => Ok(Some(value.into_f64_vec().map_err(|_| ())?)),
        None => Ok(None),
    }
}

pub(super) fn optional_string<R: Read + Seek>(
    decoder: &mut Decoder<R>,
    tag: Tag,
) -> Result<Option<String>, ()> {
    match decoder.find_tag(tag).map_err(|_| ())? {
        Some(value) => Ok(Some(value.into_string().map_err(|_| ())?)),
        None => Ok(None),
    }
}
