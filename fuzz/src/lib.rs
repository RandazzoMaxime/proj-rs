//! Fuzz entry points of proj-rs. One function per input kind; each takes raw bytes and must
//! return, never panic, hang or exhaust memory. The `fuzz_targets/` binaries (libFuzzer) call
//! them, and `tests/fuzz_regressions.rs` of the main crate replays their reproductions.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Once;

use proj_rs::iso19111::{
    create_from_user_input, create_object_from_user_input, DatabaseContext, JsonFormat,
    JsonLines, WktConvention, WktFormatter,
};
use proj_rs::{bench_definition, Operation, Proj, Xyzt, PORTED};

/// Longest text handed to a text parser: libFuzzer grows inputs, a megabyte of `+` proves nothing.
const MAX_TEXT: usize = 8192;
/// Largest grid or model file written to disk for one execution.
const MAX_FILE: usize = 1 << 20;

const EPSG_CODES: &str = include_str!("../epsg_codes.txt");

static INIT: Once = Once::new();

fn init() {
    INIT.call_once(|| {
        let data = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../data:",
            env!("CARGO_MANIFEST_DIR"),
            "/../data/tests"
        );
        // libFuzzer aborts in its panic hook, even for a panic the library catches. The grid
        // reader contains the panics of the `tiff` crate on purpose (`geotiff::contained`): let
        // those pass, and keep the hook for everything else.
        let libfuzzer_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let in_tiff_crate = info.location().is_some_and(|at| at.file().contains("/tiff-"));
            if !in_tiff_crate {
                libfuzzer_hook(info);
            }
        }));
        std::env::set_var("PROJ_DATA", data);
        std::env::set_var("PROJ_NETWORK", "OFF");
        std::env::set_var("PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY", "YES");
    });
}

fn text_of(data: &[u8]) -> Option<String> {
    (data.len() <= MAX_TEXT).then(|| String::from_utf8_lossy(data).into_owned())
}

/// Reads bytes as numbers, with the values that break arithmetic over-represented.
struct Reader<'a> {
    data: &'a [u8],
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    fn byte(&mut self) -> u8 {
        let (first, rest) = self.data.split_first().map_or((0, self.data), |(b, r)| (*b, r));
        self.data = rest;
        first
    }

    fn u16(&mut self) -> u16 {
        u16::from_le_bytes([self.byte(), self.byte()])
    }

    fn f64(&mut self) -> f64 {
        let tag = self.byte();
        let raw = f64::from_le_bytes([
            self.byte(),
            self.byte(),
            self.byte(),
            self.byte(),
            self.byte(),
            self.byte(),
            self.byte(),
            self.byte(),
        ]);
        match tag % 24 {
            0 => f64::NAN,
            1 => f64::INFINITY,
            2 => f64::NEG_INFINITY,
            3 => 1e300,
            4 => -1e300,
            5 => 5e-324,
            6 => -0.0,
            7 => f64::MAX,
            8 => f64::MIN_POSITIVE,
            9 => 180.0,
            10 => -180.0,
            11 => 90.0,
            12 => -90.0,
            13 => 1e15,
            14 => std::f64::consts::PI,
            15 => std::f64::consts::FRAC_PI_2,
            // A degree-sized value, in range for longitude and latitude.
            16..=19 => (raw % 360.0).clamp(-360.0, 360.0),
            _ => raw,
        }
    }

    fn rest(self) -> &'a [u8] {
        self.data
    }
}

const FIXED_POINTS: [(f64, f64, f64); 12] = [
    (0.0, 0.0, 0.0),
    (3.0, 49.0, 100.0),
    (-120.5, 37.2, -50.0),
    (179.999999, 89.999999, 0.0),
    (-180.0, -90.0, 0.0),
    (0.5, 0.8, 0.0),
    (f64::NAN, 0.0, 0.0),
    (0.0, f64::INFINITY, 0.0),
    (1e300, -1e300, 1e300),
    (5e-324, 5e-324, 5e-324),
    (4.0e7, 4.0e7, 0.0),
    (-1.0e7, 1.0e7, f64::NAN),
];

fn exercise_operation(op: &Operation, extra: &[(f64, f64, f64)]) {
    for &(x, y, z) in FIXED_POINTS.iter().chain(extra) {
        let coord = Xyzt { x, y, z, t: 2020.0 };
        let _ = op.forward(coord);
        let _ = op.inverse(coord);
        if let Ok(out) = op.forward(coord) {
            let _ = op.inverse(out);
        }
    }
    let _ = op.parameters();
    let _ = op.angular_input(false);
    let _ = op.angular_output(true);
    let _ = op.degree_output(false);
    let _ = op.distance_ellipsoid();
    let _ = op.computes_height();
    let _ = op.factors(0.5, 0.8);
    let _ = op.factors(f64::NAN, 2.0);
}

/// Coordinates and metadata of a facade object. The exports are left out: each one opens the
/// embedded database (10 ms), and `user_input` covers them.
fn exercise_proj(proj: &Proj, extra: &[(f64, f64, f64)]) {
    for &(x, y, z) in FIXED_POINTS.iter().chain(extra) {
        let _ = proj.convert((x, y));
        let _ = proj.project((x, y), true);
        let _ = proj.convert_xyz(x, y, z);
    }
    let _ = proj.def();
    let _ = proj.name();
    let _ = proj.proj_info();
    let _ = proj.area_of_use();
    let _ = proj.factors(0.5, 0.8);
}

/// Any text as a PROJ string: operation and facade creation, then coordinates through it.
pub fn proj_string(data: &[u8]) {
    init();
    let Some(text) = text_of(data) else { return };
    if let Ok(op) = Operation::create(&text) {
        exercise_operation(&op, &[]);
    }
    if let Ok(proj) = Proj::new(&text) {
        exercise_proj(&proj, &[]);
    }
    let _ = Operation::create_projection(&text);
}

/// Any text as `createFromUserInput`: WKT, PROJJSON, PROJ string, `AUTH:code`, URN, name,
/// then every export of the CRS it gives.
pub fn user_input(data: &[u8]) {
    init();
    let Some(text) = text_of(data) else { return };
    let Ok(db) = DatabaseContext::create() else {
        return;
    };
    if let Ok(crs) = create_from_user_input(&text, &db) {
        for lines in [JsonLines::Multi, JsonLines::Single] {
            let _ = crs.export_to_json_with(&db, &JsonFormat::with_lines(lines));
        }
        for convention in [
            WktConvention::Wkt2_2019,
            WktConvention::Wkt2_2015Simplified,
            WktConvention::Wkt1Gdal,
            WktConvention::Wkt1Esri,
        ] {
            let mut formatter = WktFormatter::create(convention).with_database(db.clone());
            let _ = crs.export_to_wkt(&mut formatter);
        }
        let _ = crs.export_to_proj_string();
    }
    let _ = create_object_from_user_input(&text, &db);
}

fn epsg_code(reader: &mut Reader<'_>) -> &'static str {
    let codes: Vec<&str> = EPSG_CODES.lines().collect();
    codes[usize::from(reader.u16()) % codes.len()]
}

/// A coordinate transform: a pair of CRS (real EPSG codes or two texts) or a projection by name,
/// then coordinates drawn from raw bytes (NaN, infinities, huge, subnormal).
pub fn transform(data: &[u8]) {
    init();
    let mut reader = Reader::new(data);
    let mode = reader.byte() % 3;
    let mut points = Vec::new();
    for _ in 0..4 {
        points.push((reader.f64(), reader.f64(), reader.f64()));
    }
    match mode {
        0 => {
            let from = format!("EPSG:{}", epsg_code(&mut reader));
            let to = format!("EPSG:{}", epsg_code(&mut reader));
            if let Ok(proj) = Proj::new_known_crs(&from, &to, None) {
                for &(x, y, z) in &points {
                    let _ = proj.convert((x, y));
                    let _ = proj.convert_xyz(x, y, z);
                }
                let mut batch: Vec<(f64, f64)> = points.iter().map(|p| (p.0, p.1)).collect();
                let _ = proj.convert_array(&mut batch);
            }
        }
        1 => {
            let id = PORTED[usize::from(reader.u16()) % PORTED.len()];
            let definition = bench_definition(id);
            let extra = reader.byte() % 8;
            let suffix = match extra {
                0 => " +over",
                1 => " +a=6378137 +b=6356752.3",
                2 => " +R=1",
                3 => " +lon_0=180",
                4 => " +x_0=1e300 +y_0=-1e300",
                5 => " +k_0=0",
                6 => " +lat_0=90",
                _ => "",
            };
            if let Ok(op) = Operation::create(&format!("{definition}{suffix}")) {
                exercise_operation(&op, &points);
            }
        }
        _ => {
            let rest = reader.rest();
            let (left, right) = match rest.iter().position(|b| *b == 0) {
                Some(at) => (&rest[..at], &rest[at + 1..]),
                None => (rest, &b""[..]),
            };
            let (Some(from), Some(to)) = (text_of(left), text_of(right)) else {
                return;
            };
            if let Ok(proj) = Proj::new_known_crs(&from, &to, None) {
                exercise_proj(&proj, &points);
            }
        }
    }
}

fn scratch_file(extension: &str, content: &[u8]) -> Option<ScratchFile> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!("proj-rs-fuzz-{}", std::process::id()));
    fs::create_dir_all(&dir).ok()?;
    let path = dir.join(format!(
        "f{}.{extension}",
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&path, content).ok()?;
    Some(ScratchFile(path))
}

struct ScratchFile(PathBuf);

impl ScratchFile {
    fn name(&self) -> String {
        self.0.display().to_string()
    }
}

impl Drop for ScratchFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

const GRID_EXTENSIONS: [&str; 6] = ["tif", "gtx", "gsb", "ct2", "dat", "tiff"];

fn exercise_grid(grid: &proj_rs::grids::Grid<'_>, depth: usize) {
    let (width, height) = (grid.width(), grid.height());
    let _ = grid.extent();
    let _ = grid.is_null_grid();
    let _ = grid.unit(0);
    let _ = grid.description(0);
    let _ = grid.metadata_item("AREA_OR_POINT");
    let samples = grid.samples_per_pixel();
    for (x, y) in [
        (0, 0),
        (width.saturating_sub(1), height.saturating_sub(1)),
        (width / 2, height / 2),
        (width, height),
        (usize::MAX, usize::MAX),
    ] {
        for sample in 0..samples.min(4) {
            let _ = grid.value_at(x, y, sample);
        }
        let _ = grid.horizontal_shift(x, y);
    }
    if depth < 4 {
        for sub in grid.sub_grids() {
            exercise_grid(&sub, depth + 1);
        }
    }
}

macro_rules! exercise_set {
    ($set:expr) => {
        for grid in $set.grids() {
            exercise_grid(&grid, 0);
        }
        for (lon, lat) in [(0.0, 0.0), (0.1, 0.8), (-2.0, 0.5), (f64::NAN, 1.0), (1e300, 0.0)] {
            if let Some(grid) = $set.grid_at(lon, lat) {
                exercise_grid(&grid, 3);
            }
        }
    };
}

/// A grid file of any format (GeoTIFF, GTX, NTv1, NTv2, CTable2), the first byte choosing the
/// extension, read as horizontal, vertical and generic sets and used by the grid steps.
pub fn grid_reader(data: &[u8]) {
    init();
    let Some((&selector, content)) = data.split_first() else {
        return;
    };
    if content.len() > MAX_FILE {
        return;
    }
    let extension = GRID_EXTENSIONS[usize::from(selector) % GRID_EXTENSIONS.len()];
    let Some(file) = scratch_file(extension, content) else {
        return;
    };
    let name = file.name();
    if let Ok(set) = proj_rs::grids::HorizontalShiftGridSet::open(&name) {
        exercise_set!(set);
    }
    if let Ok(set) = proj_rs::grids::VerticalShiftGridSet::open(&name) {
        exercise_set!(set);
    }
    if let Ok(set) = proj_rs::grids::GenericShiftGridSet::open(&name) {
        exercise_set!(set);
    }
    for step in ["hgridshift", "vgridshift", "gridshift", "deformation"] {
        let definition = match step {
            "gridshift" => format!("+proj=pipeline +step +proj=hgridshift +grids={name} +step +proj=vgridshift +grids={name}"),
            "deformation" => format!("+proj=deformation +grids={name} +t_epoch=2010"),
            _ => format!("+proj={step} +grids={name}"),
        };
        if let Ok(op) = Operation::create(&definition) {
            exercise_operation(&op, &[(0.01, 0.8, 0.0), (-0.03, 0.85, 10.0)]);
        }
    }
}

/// A deformation model master file or a TIN shift file (JSON), parsed, used as a step, and
/// evaluated.
pub fn model_files(data: &[u8]) {
    init();
    let Some((&selector, content)) = data.split_first() else {
        return;
    };
    if content.len() > MAX_FILE {
        return;
    }
    let text = String::from_utf8_lossy(content).into_owned();
    if selector % 2 == 0 {
        if let Ok(file) = proj_rs::tinshift::TinShiftFile::parse(&text) {
            let _ = (file.file_type(), file.format_version());
            let _ = (file.input_crs(), file.output_crs(), file.fallback_strategy());
            let evaluator = proj_rs::tinshift::TinShiftEvaluator::new(file);
            for &(x, y, z) in &FIXED_POINTS {
                let _ = evaluator.forward(x, y, z);
                let _ = evaluator.inverse(x, y, z);
            }
        }
        if let Some(file) = scratch_file("json", content) {
            if let Ok(op) = Operation::create(&format!("+proj=tinshift +file={}", file.name())) {
                exercise_operation(&op, &[(0.5, 1.0, 0.0), (350000.0, 200000.0, 0.0)]);
            }
        }
    } else {
        if let Ok(model) = proj_rs::defmodel::MasterFile::parse(&text) {
            let _ = (model.authority(), model.links(), model.extent(), model.time_extent());
            for component in model.components() {
                let _ = component.description();
                let _ = component.time_function().evaluate_at(2020.5);
                let _ = component.time_function().evaluate_at(f64::NAN);
            }
        }
        if let Some(file) = scratch_file("json", content) {
            if let Ok(op) = Operation::create(&format!("+proj=defmodel +model={}", file.name())) {
                exercise_operation(&op, &[(0.01, 0.8, 0.0)]);
            }
        }
    }
}
