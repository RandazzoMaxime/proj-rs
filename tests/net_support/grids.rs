//! GeoTIFF grid fixtures for the network tests: a small, uncompressed, little-endian
//! float32 TIFF with the GeoTIFF tags PROJ reads (`ModelPixelScale`, `ModelTiepoint`,
//! `GeoKeyDirectory`). Written here so the tests need no file in the tree, and so the grid
//! served by the local server is the same bytes the reference binary reads.

/// A grid of `width` x `height` pixels, `samples_per_pixel` float32 samples per pixel, stored
/// row by row (north row first, the GeoTIFF convention), the north-west corner at
/// `(west, north)` and a square pixel of `step` degrees.
pub(crate) fn geotiff(
    width: u32,
    height: u32,
    west: f64,
    north: f64,
    step: f64,
    samples_per_pixel: u16,
    samples: &[f32],
) -> Vec<u8> {
    assert!((1..=3).contains(&samples_per_pixel), "1 to 3 samples");
    assert_eq!(
        samples.len(),
        usize::try_from(width).expect("width")
            * usize::try_from(height).expect("height")
            * usize::from(samples_per_pixel),
        "one value per sample"
    );
    let mut out = Vec::new();
    out.extend_from_slice(b"II");
    out.extend_from_slice(&42_u16.to_le_bytes());
    out.extend_from_slice(&8_u32.to_le_bytes());
    for sample in samples {
        out.extend_from_slice(&sample.to_le_bytes());
    }
    let data_len = u32::try_from(samples.len() * 4).expect("grid size");
    let scale_offset = u32::try_from(out.len()).expect("offset");
    for value in [step, step, 0.0_f64] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    let tie_offset = u32::try_from(out.len()).expect("offset");
    for value in [0.0_f64, 0.0, 0.0, west, north, 0.0] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    let keys_offset = u32::try_from(out.len()).expect("offset");
    // Version 1, revision 1.0, 3 keys: model type geographic, PixelIsArea, WGS 84.
    for key in [
        1_u16, 1, 0, 3, 1024, 0, 1, 2, 1025, 0, 1, 1, 2048, 0, 1, 4326,
    ] {
        out.extend_from_slice(&key.to_le_bytes());
    }
    if out.len() % 2 == 1 {
        out.push(0);
    }
    // Per-sample SHORT arrays fit in the 4-byte entry only for one or two samples; a third
    // sample pushes them out of line, before the IFD.
    let spp = u32::from(samples_per_pixel);
    let short_array = |value: u16, out: &mut Vec<u8>| -> (u32, u32) {
        if samples_per_pixel <= 2 {
            let mut bytes = [0_u8; 4];
            for index in 0..usize::from(samples_per_pixel) {
                bytes[index * 2..index * 2 + 2].copy_from_slice(&value.to_le_bytes());
            }
            (spp, u32::from_le_bytes(bytes))
        } else {
            let offset = u32::try_from(out.len()).expect("offset");
            for _ in 0..samples_per_pixel {
                out.extend_from_slice(&value.to_le_bytes());
            }
            (spp, offset)
        }
    };
    let (bits_count, bits_value) = short_array(32, &mut out);
    let (format_count, format_value) = short_array(3, &mut out);
    if out.len() % 2 == 1 {
        out.push(0);
    }
    // BlackIsZero for every sample count: the reference reader refuses RGB for float32 grids.
    let photometric = 1;
    let ifd_offset = u32::try_from(out.len()).expect("offset");
    let entries: [(u16, u16, u32, u32); 13] = [
        (256, 4, 1, width),
        (257, 4, 1, height),
        (258, 3, bits_count, bits_value),
        (259, 3, 1, 1),
        (262, 3, 1, photometric),
        (273, 4, 1, 8),
        (277, 3, 1, spp),
        (278, 4, 1, height),
        (279, 4, 1, data_len),
        (284, 3, 1, 1),
        (339, 3, format_count, format_value),
        (33_550, 12, 3, scale_offset),
        (33_922, 12, 6, tie_offset),
    ];
    let geokeys = (34_735_u16, 3_u16, 16_u32, keys_offset);
    let count = u16::try_from(entries.len() + 1).expect("entries");
    out.extend_from_slice(&count.to_le_bytes());
    let mut all: Vec<(u16, u16, u32, u32)> = entries.to_vec();
    all.push((geokeys.0, geokeys.1, geokeys.2, geokeys.3));
    all.sort_by_key(|entry| entry.0);
    for (tag, kind, n, value) in all {
        out.extend_from_slice(&tag.to_le_bytes());
        out.extend_from_slice(&kind.to_le_bytes());
        out.extend_from_slice(&n.to_le_bytes());
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.extend_from_slice(&0_u32.to_le_bytes());
    // The IFD offset is the header's fourth-byte field, patched once the layout is known.
    out[4..8].copy_from_slice(&ifd_offset.to_le_bytes());
    out
}

/// The samples of a grid of `width` x `height` pixels, row by row: `pixel(col, row)` gives the
/// samples of one pixel.
fn samples_of(width: u16, height: u16, pixel: impl Fn(f32, f32) -> Vec<f32>) -> Vec<f32> {
    let mut out = Vec::new();
    for row in 0..height {
        for col in 0..width {
            out.extend(pixel(f32::from(col), f32::from(row)));
        }
    }
    out
}

/// The horizontal grid served as `fr_ign_gr3df97a.tif` and as `fr_ign_ntf_r93.tif`: three
/// samples (the shifts in `xyzgridshift` order), 40 x 32 pixels of 0.5 degree over
/// `lon -10..10`, `lat 40..56`.
pub(crate) fn france_xyz_grid() -> Vec<u8> {
    let samples = samples_of(40, 32, |col, row| {
        vec![0.5 + 0.01 * col, -0.25 - 0.02 * row, 0.1 * row]
    });
    geotiff(40, 32, -10.0, 56.0, 0.5, 3, &samples)
}

/// The global geoid grid served as `us_nga_egm08_25.tif`: 72 x 36 pixels of 5 degrees.
pub(crate) fn global_geoid_grid() -> Vec<u8> {
    let samples = samples_of(72, 36, |col, row| vec![-20.0 + 0.1 * col - 0.05 * row]);
    geotiff(72, 36, -180.0, 90.0, 5.0, 1, &samples)
}

/// The NGVD29 to NAVD88 grid served as `us_noaa_vertcone.tif`: 60 x 30 pixels of 1 degree over
/// `lon -130..-70`, `lat 25..55`.
pub(crate) fn conus_vertical_grid() -> Vec<u8> {
    let samples = samples_of(60, 30, |col, row| vec![-0.15 + 0.001 * col - 0.002 * row]);
    geotiff(60, 30, -130.0, 55.0, 1.0, 1, &samples)
}

/// The grid of the cache tests, served as `fr_ign_ntf_r93.tif`: three samples, 128 x 96 pixels
/// of 0.25 degree over `lon -10..22`, `lat 32..56`. At about 144 KiB it spans nine 16 KiB
/// chunks, more than the two the saturation test keeps.
pub(crate) fn cache_grid() -> Vec<u8> {
    let samples = samples_of(128, 96, |col, row| {
        vec![0.5 + 0.01 * col, -0.25 - 0.02 * row, 0.1 * row]
    });
    geotiff(128, 96, -10.0, 56.0, 0.25, 3, &samples)
}
