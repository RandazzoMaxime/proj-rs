//! A tiled GeoTIFF reads as its strip twin. The strip path is the oracle: it reads the grid the
//! way PROJ does (`planar_deflate_predictor3.tif` and the other fixtures). The tiled file below
//! holds the same samples in 4 by 4 tiles, so the two grids must agree at every sample.

use super::open_tiff;
use crate::projections::models::grid::Grid;

const SHORT: u16 = 3;
const LONG: u16 = 4;
const DOUBLE: u16 = 12;

#[derive(Clone, Copy)]
enum Blocks {
    /// Rows per strip.
    Strips(u32),
    /// Tile width and tile length.
    Tiles(u32, u32),
}

struct Layout {
    width: u32,
    height: u32,
    samples: u16,
    planar: u16,
    blocks: Blocks,
}

fn sample_value(x: u32, y: u32, sample: u16) -> f32 {
    (x * 100 + y) as f32 + f32::from(sample) * 0.25
}

fn le_u32s(values: &[u32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

/// One IFD entry: tag, type, count, and its value bytes (inline when they fit in four bytes).
struct Entry {
    tag: u16,
    kind: u16,
    count: u32,
    payload: Vec<u8>,
}

fn entry(tag: u16, kind: u16, count: u32, payload: Vec<u8>) -> Entry {
    Entry {
        tag,
        kind,
        count,
        payload,
    }
}

fn write_tiff(layout: &Layout) -> Vec<u8> {
    let (block_w, block_h) = match layout.blocks {
        Blocks::Strips(rows) => (layout.width, rows),
        Blocks::Tiles(tile_w, tile_h) => (tile_w, tile_h),
    };
    let across = layout.width.div_ceil(block_w);
    let down = layout.height.div_ceil(block_h);
    let planes = if layout.planar == 2 {
        u32::from(layout.samples)
    } else {
        1
    };
    let mut data = Vec::new();
    let mut offsets = Vec::new();
    let mut counts = Vec::new();
    for plane in 0..planes {
        for by in 0..down {
            for bx in 0..across {
                let mut block = Vec::new();
                for py in 0..block_h {
                    for px in 0..block_w {
                        let (x, y) = (bx * block_w + px, by * block_h + py);
                        let inside = x < layout.width && y < layout.height;
                        let samples: Vec<u16> = if layout.planar == 2 {
                            vec![plane as u16]
                        } else {
                            (0..layout.samples).collect()
                        };
                        for sample in samples {
                            let value = if inside {
                                sample_value(x, y, sample)
                            } else {
                                0.0
                            };
                            block.extend_from_slice(&value.to_le_bytes());
                        }
                    }
                }
                offsets.push(8 + data.len() as u32);
                counts.push(block.len() as u32);
                data.extend_from_slice(&block);
            }
        }
    }
    let mut tags = vec![
        entry(256, LONG, 1, layout.width.to_le_bytes().to_vec()),
        entry(257, LONG, 1, layout.height.to_le_bytes().to_vec()),
        entry(
            258,
            SHORT,
            u32::from(layout.samples),
            (0..layout.samples)
                .flat_map(|_| 32_u16.to_le_bytes())
                .collect(),
        ),
        entry(259, SHORT, 1, 1_u16.to_le_bytes().to_vec()),
        entry(262, SHORT, 1, 1_u16.to_le_bytes().to_vec()),
        entry(277, SHORT, 1, layout.samples.to_le_bytes().to_vec()),
        entry(284, SHORT, 1, layout.planar.to_le_bytes().to_vec()),
        entry(
            339,
            SHORT,
            u32::from(layout.samples),
            (0..layout.samples)
                .flat_map(|_| 3_u16.to_le_bytes())
                .collect(),
        ),
    ];
    match layout.blocks {
        Blocks::Strips(rows) => {
            tags.push(entry(278, LONG, 1, rows.to_le_bytes().to_vec()));
            tags.push(entry(273, LONG, offsets.len() as u32, le_u32s(&offsets)));
            tags.push(entry(279, LONG, counts.len() as u32, le_u32s(&counts)));
        }
        Blocks::Tiles(tile_w, tile_h) => {
            tags.push(entry(322, LONG, 1, tile_w.to_le_bytes().to_vec()));
            tags.push(entry(323, LONG, 1, tile_h.to_le_bytes().to_vec()));
            tags.push(entry(324, LONG, offsets.len() as u32, le_u32s(&offsets)));
            tags.push(entry(325, LONG, counts.len() as u32, le_u32s(&counts)));
        }
    }
    let scale: Vec<u8> = [0.5_f64, 0.5, 0.0]
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    let tie: Vec<u8> = [0.0_f64, 0.0, 0.0, 10.0, 50.0, 0.0]
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    tags.push(entry(33550, DOUBLE, 3, scale));
    tags.push(entry(33922, DOUBLE, 6, tie));
    tags.sort_by_key(|tag| tag.tag);

    let mut out = b"II".to_vec();
    out.extend_from_slice(&42_u16.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&data);
    let mut inline = Vec::new();
    for tag in &mut tags {
        if tag.payload.len() > 4 {
            let at = out.len() as u32;
            out.extend_from_slice(&tag.payload);
            tag.payload = at.to_le_bytes().to_vec();
        }
        let mut value = tag.payload.clone();
        value.resize(4, 0);
        inline.push((tag.tag, tag.kind, tag.count, value));
    }
    if out.len() % 2 == 1 {
        out.push(0);
    }
    let ifd = out.len() as u32;
    out[4..8].copy_from_slice(&ifd.to_le_bytes());
    out.extend_from_slice(&(inline.len() as u16).to_le_bytes());
    for (tag, kind, count, value) in inline {
        out.extend_from_slice(&tag.to_le_bytes());
        out.extend_from_slice(&kind.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&value);
    }
    out.extend_from_slice(&[0, 0, 0, 0]);
    out
}

fn open(bytes: &[u8]) -> Grid {
    open_tiff(bytes).expect("the TIFF grid is read")
}

fn assert_same_grid(strips: &Grid, tiles: &Grid) {
    assert_eq!(
        (strips.width, strips.height, strips.samples),
        (tiles.width, tiles.height, tiles.samples)
    );
    for y in 0..strips.height {
        for x in 0..strips.width {
            for sample in 0..strips.samples {
                let tiff_row = strips.height - 1 - y;
                assert_eq!(
                    tiles.value(x, y, sample),
                    Some(sample_value(x as u32, tiff_row as u32, sample as u16)),
                    "pixel ({x}, {y}), sample {sample}"
                );
                assert_eq!(
                    strips.value(x, y, sample),
                    tiles.value(x, y, sample),
                    "pixel ({x}, {y}), sample {sample}"
                );
            }
        }
    }
}

fn layout(planar: u16, blocks: Blocks) -> Layout {
    Layout {
        width: 10,
        height: 7,
        samples: 3,
        planar,
        blocks,
    }
}

#[test]
fn tiled_chunky_grid_reads_as_its_strips() {
    let strips = open(&write_tiff(&layout(1, Blocks::Strips(7))));
    let tiles = open(&write_tiff(&layout(1, Blocks::Tiles(4, 4))));
    assert_same_grid(&strips, &tiles);
}

#[test]
fn tiled_planar_grid_reads_as_its_strips() {
    let strips = open(&write_tiff(&layout(2, Blocks::Strips(7))));
    let tiles = open(&write_tiff(&layout(2, Blocks::Tiles(4, 4))));
    assert_same_grid(&strips, &tiles);
}
