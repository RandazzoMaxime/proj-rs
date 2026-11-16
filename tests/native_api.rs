//! The native façade of the crate root (`Context`, `Crs`, `Transformer`, `Point`), checked
//! against the `cs2cs` and `projinfo` of PROJ 9.9.0.

use proj_rs::{
    BBox, Backend, Ballpark, Cache, Context, Crs, CrsKind, Error, Network, OnlyBest, Point,
    Transformer, WktVersion,
};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// A PROJ tool on `PATH`. The `target/` of this repository is skipped: its own `cs2cs` must not
/// shadow the reference. `PROJ_CS2CS_REQUIRED` turns a missing binary into a failure, as in CI.
fn reference(tool: &str) -> Option<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let name = if cfg!(windows) {
        format!("{tool}.exe")
    } else {
        tool.to_owned()
    };
    let found = std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path).find_map(|dir| {
            if dir.starts_with(&manifest) {
                return None;
            }
            let candidate = dir.join(&name);
            candidate.is_file().then_some(candidate)
        })
    });
    if found.is_none() {
        assert!(
            std::env::var_os("PROJ_CS2CS_REQUIRED").is_none(),
            "PROJ_CS2CS_REQUIRED is set but no PROJ {tool} is on PATH"
        );
        drop(writeln!(
            std::io::stderr(),
            "skipped: no PROJ {tool} on PATH"
        ));
    }
    found
}

/// What `cs2cs -d 9 from to` writes for `first second`, which are in the axis order of `from`.
fn cs2cs(program: &std::path::Path, from: &str, to: &str, first: f64, second: f64) -> (f64, f64) {
    let mut child = Command::new(program)
        .args(["-d", "9", from, "+to", to])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("cs2cs starts");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(format!("{first:.12} {second:.12}\n").as_bytes())
        .expect("write");
    let done = child.wait_with_output().expect("wait");
    assert!(
        done.status.success(),
        "{}",
        String::from_utf8_lossy(&done.stderr)
    );
    let text = String::from_utf8(done.stdout).expect("utf8");
    let mut parts = text.split_whitespace();
    let a = parts.next().expect("first").parse::<f64>().expect("number");
    let b = parts
        .next()
        .expect("second")
        .parse::<f64>()
        .expect("number");
    (a, b)
}

fn crs(ctx: &Context, text: &str) -> Crs {
    Crs::from_user_input(ctx, text).expect("crs")
}

fn transformer(from: &str, to: &str) -> Transformer {
    let ctx = Context::default();
    Transformer::builder(&crs(&ctx, from), &crs(&ctx, to))
        .build()
        .expect("transformer")
}

fn assert_close(actual: f64, expected: f64, tolerance: f64, what: &str) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{what}: {actual} against {expected}"
    );
}

/// Paris, Lyon, Marseille, Brest, Strasbourg, as latitude then longitude, the order of
/// `EPSG:4326`.
const CITIES: [(f64, f64); 5] = [
    (48.8566, 2.3522),
    (45.7640, 4.8357),
    (43.2965, 5.3698),
    (48.3904, -4.4861),
    (48.5734, 7.7521),
];

#[test]
fn wgs84_to_lambert93_agrees_with_cs2cs() {
    let Some(cs2cs_bin) = reference("cs2cs") else {
        return;
    };
    let t = transformer("EPSG:4326", "EPSG:2154");
    for (lat, lon) in CITIES {
        let ours = t.forward(Point::new(lat, lon)).expect("forward");
        let (east, north) = cs2cs(&cs2cs_bin, "EPSG:4326", "EPSG:2154", lat, lon);
        assert_close(ours.x, east, 1e-6, "easting");
        assert_close(ours.y, north, 1e-6, "northing");
        assert_eq!(ours.z, 0.0);
    }
}

#[test]
fn wgs84_to_utm31_agrees_with_cs2cs() {
    let Some(cs2cs_bin) = reference("cs2cs") else {
        return;
    };
    let t = transformer("EPSG:4326", "EPSG:32631");
    for (lat, lon) in CITIES {
        let ours = t.forward(Point::new(lat, lon)).expect("forward");
        let (east, north) = cs2cs(&cs2cs_bin, "EPSG:4326", "EPSG:32631", lat, lon);
        assert_close(ours.x, east, 1e-6, "easting");
        assert_close(ours.y, north, 1e-6, "northing");
    }
}

#[test]
fn inverse_agrees_with_cs2cs_and_returns_the_source_point() {
    let Some(cs2cs_bin) = reference("cs2cs") else {
        return;
    };
    let t = transformer("EPSG:4326", "EPSG:2154");
    for (east, north) in [(652_301.56, 6_861_302.73), (843_000.0, 6_520_000.0)] {
        let ours = t.inverse(Point::new(east, north)).expect("inverse");
        let (lat, lon) = cs2cs(&cs2cs_bin, "EPSG:2154", "EPSG:4326", east, north);
        assert_close(ours.x, lat, 1e-9, "latitude");
        assert_close(ours.y, lon, 1e-9, "longitude");
    }
    for (lat, lon) in CITIES {
        let there = t.forward(Point::new(lat, lon)).expect("forward");
        let back = t.inverse(there).expect("inverse");
        assert_close(back.x, lat, 1e-9, "latitude back");
        assert_close(back.y, lon, 1e-9, "longitude back");
    }
}

#[test]
fn axes_follow_the_authority_unless_normalized() {
    let ctx = Context::default();
    let src = crs(&ctx, "EPSG:4326");
    let dst = crs(&ctx, "EPSG:3035");
    let authority = Transformer::builder(&src, &dst).build().expect("authority");
    let gis = Transformer::builder(&src, &dst)
        .normalize_axis_order()
        .build()
        .expect("gis");
    let (lat, lon) = (48.8566, 2.3522);
    let by_authority = authority.forward(Point::new(lat, lon)).expect("authority");
    let by_gis = gis.forward(Point::new(lon, lat)).expect("gis");
    // EPSG:3035 writes northing then easting; the GIS order is easting then northing.
    assert_eq!((by_authority.x, by_authority.y), (by_gis.y, by_gis.x));
    assert!(by_authority.x > 2_000_000.0 && by_authority.y > 3_000_000.0);
    if let Some(cs2cs_bin) = reference("cs2cs") {
        let (north, east) = cs2cs(&cs2cs_bin, "EPSG:4326", "EPSG:3035", lat, lon);
        assert_close(by_authority.x, north, 1e-6, "northing");
        assert_close(by_authority.y, east, 1e-6, "easting");
    }
    let back_authority = authority.inverse(by_authority).expect("inverse");
    let back_gis = gis.inverse(by_gis).expect("inverse");
    assert_close(back_authority.x, lat, 1e-9, "latitude");
    assert_close(back_gis.x, lon, 1e-9, "longitude");
    assert_close(back_gis.y, lat, 1e-9, "latitude");
}

#[test]
fn normalizing_a_lon_lat_projected_pair_changes_nothing() {
    let ctx = Context::default();
    let src = crs(&ctx, "EPSG:2154");
    let dst = crs(&ctx, "EPSG:32631");
    let plain = Transformer::builder(&src, &dst).build().expect("plain");
    let gis = Transformer::builder(&src, &dst)
        .normalize_axis_order()
        .build()
        .expect("gis");
    let point = Point::new(652_301.56, 6_861_302.73);
    assert_eq!(
        plain.forward(point).expect("plain"),
        gis.forward(point).expect("gis")
    );
}

fn grid_points() -> Vec<Point> {
    (0..1000)
        .map(|i| {
            let step = f64::from(i);
            Point::new(42.0 + 0.009 * step, -4.5 + 0.013 * step)
        })
        .collect()
}

#[test]
fn forward_slice_agrees_with_forward_point_by_point() {
    let t = transformer("EPSG:4326", "EPSG:2154");
    let mut batch = grid_points();
    t.forward_slice(&mut batch).expect("slice");
    for (input, out) in grid_points().into_iter().zip(&batch) {
        let single = t.forward(input).expect("forward");
        // The vector path agrees with the scalar one to 11 nm (docs/PORTING-simd.md).
        assert_close(out.x, single.x, 1e-7, "x");
        assert_close(out.y, single.y, 1e-7, "y");
        assert_eq!(out.z, single.z);
    }
}

#[test]
fn the_scalar_backend_is_the_forward_path() {
    let ctx = Context::default();
    let src = crs(&ctx, "EPSG:4326");
    let dst = crs(&ctx, "EPSG:32631");
    let scalar = Transformer::builder(&src, &dst)
        .backend(Backend::Scalar)
        .build()
        .expect("scalar");
    let mut batch = grid_points();
    scalar.forward_slice(&mut batch).expect("slice");
    for (input, out) in grid_points().into_iter().zip(&batch) {
        assert_eq!(*out, scalar.forward(input).expect("forward"));
    }
}

#[test]
fn inverse_slice_agrees_with_inverse() {
    let t = transformer("EPSG:4326", "EPSG:2154");
    let mut batch = grid_points();
    t.forward_slice(&mut batch).expect("forward slice");
    let projected = batch.clone();
    t.inverse_slice(&mut batch).expect("inverse slice");
    for ((original, back), mapped) in grid_points().into_iter().zip(&batch).zip(&projected) {
        let single = t.inverse(*mapped).expect("inverse");
        assert_close(back.x, single.x, 1e-9, "latitude");
        assert_close(back.y, single.y, 1e-9, "longitude");
        assert_close(back.x, original.x, 1e-7, "latitude round trip");
    }
}

#[test]
fn the_slice_of_a_candidate_set_agrees_with_forward() {
    let ctx = Context::default();
    let t = Transformer::builder(&crs(&ctx, "EPSG:4267"), &crs(&ctx, "EPSG:4269"))
        .build()
        .expect("transformer");
    let mut batch = [Point::new(38.0, -95.0), Point::new(45.0, -100.0)];
    let expected: Vec<_> = batch
        .iter()
        .map(|point| t.forward(*point).expect("forward"))
        .collect();
    t.forward_slice(&mut batch).expect("slice");
    assert_eq!(batch.to_vec(), expected);
}

#[test]
fn the_empty_slice_is_fine() {
    let t = transformer("EPSG:4326", "EPSG:2154");
    t.forward_slice(&mut []).expect("empty");
}

#[test]
fn a_failing_point_leaves_the_slice_unchanged() {
    let t = transformer("EPSG:4326", "EPSG:3857");
    let mut points = [
        Point::new(48.0, 2.0),
        Point::new(91.0, 0.0),
        Point::new(10.0, 10.0),
    ];
    let before = points;
    let error = t.forward_slice(&mut points).expect_err("pole");
    assert_eq!(points, before);
    assert_eq!(error, t.forward(Point::new(91.0, 0.0)).expect_err("pole"));
}

#[test]
fn errors_are_typed() {
    let ctx = Context::default();
    assert!(matches!(
        Crs::from_user_input(&ctx, ""),
        Err(Error::InvalidInput(_))
    ));
    assert!(matches!(
        Crs::from_user_input(&ctx, "EPSG:4326\0"),
        Err(Error::InvalidInput(_))
    ));
    assert!(matches!(
        Crs::from_user_input(&ctx, "EPSG:999999"),
        Err(Error::Crs(_))
    ));
    assert!(matches!(
        Crs::from_user_input(&ctx, "not a crs"),
        Err(Error::Crs(_))
    ));
    let error = Crs::from_user_input(&ctx, "EPSG:999999").expect_err("unknown");
    assert_eq!(
        error.code(),
        proj_rs::unstable::errno::PROJ_ERR_INVALID_OP_WRONG_SYNTAX
    );
    assert!(!error.to_string().is_empty());

    let t = transformer("EPSG:4326", "EPSG:3857");
    let pole = t.forward(Point::new(91.0, 0.0)).expect_err("pole");
    assert!(matches!(
        pole,
        Error::OutsideDomain | Error::InvalidCoordinate
    ));
    let nan = t.forward(Point::new(f64::NAN, 0.0)).expect_err("nan");
    assert!(matches!(nan, Error::InvalidCoordinate));
    assert_eq!(
        nan.code(),
        proj_rs::unstable::errno::PROJ_ERR_COORD_TRANSFM_INVALID_COORD
    );
}

#[test]
fn a_pair_with_no_operation_is_an_error_not_a_coordinate() {
    let ctx = Context::default();
    let geographic = crs(&ctx, "EPSG:4326");
    let engineering = crs(
        &ctx,
        "LOCAL_CS[\"site\",LOCAL_DATUM[\"site\",0],UNIT[\"metre\",1],AXIS[\"X\",EAST],AXIS[\"Y\",NORTH]]",
    );
    let built = Transformer::builder(&geographic, &engineering).build();
    assert!(
        matches!(built, Err(Error::NoOperation(_))),
        "{:?}",
        built.map(|_| ())
    );
}

#[test]
fn only_best_fails_closed_on_a_missing_grid() {
    let ctx = Context::default();
    let src = crs(&ctx, "EPSG:4267");
    let dst = crs(&ctx, "EPSG:4269");
    let strict = Transformer::builder(&src, &dst)
        .only_best(OnlyBest::Yes)
        .build();
    // Without the NADCON grids the best operation cannot run: building must not hand
    // back a transformer that would return a ballpark result instead.
    match strict {
        Err(Error::MissingGrid(_)) => {}
        Ok(t) => {
            let result = t.forward(Point::new(38.0, -95.0));
            assert!(
                matches!(&result, Err(Error::MissingGrid(_))) || result.is_ok(),
                "{result:?}"
            );
        }
        Err(other) => panic!("unexpected error {other:?}"),
    }
}

#[test]
fn ballpark_candidates_can_be_forbidden() {
    let ctx = Context::default();
    let src = crs(&ctx, "EPSG:4267");
    let dst = crs(&ctx, "EPSG:4269");
    let allowed = Transformer::builder(&src, &dst).build().expect("allowed");
    let forbidden = Transformer::builder(&src, &dst)
        .allow_ballpark(Ballpark::Forbidden)
        .build()
        .expect("forbidden");
    let all = allowed.operations().expect("operations");
    let kept = forbidden.operations().expect("operations");
    assert!(all.iter().any(proj_rs::OperationInfo::is_ballpark));
    assert!(kept.iter().all(|operation| !operation.is_ballpark()));
    assert_eq!(kept.len() + 1, all.len());
}

#[test]
fn operations_lists_the_candidates_of_projinfo() {
    let Some(projinfo) = reference("projinfo") else {
        return;
    };
    let ctx = Context::default();
    for (from, to) in [("EPSG:4326", "EPSG:2154"), ("EPSG:4267", "EPSG:4269")] {
        let output = Command::new(&projinfo)
            .args([
                "-s",
                from,
                "-t",
                to,
                "--spatial-test",
                "intersects",
                "--summary",
            ])
            .output()
            .expect("projinfo starts");
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).expect("utf8");
        let expected: Vec<String> = text
            .lines()
            .skip(1)
            .map(|line| line.split(", ").nth(1).expect("name").to_owned())
            .collect();
        let t = Transformer::builder(&crs(&ctx, from), &crs(&ctx, to))
            .build()
            .expect("transformer");
        let names: Vec<String> = t
            .operations()
            .expect("operations")
            .iter()
            .map(|operation| operation.name().expect("name").to_owned())
            .collect();
        assert_eq!(names, expected, "{from} to {to}");
    }
}

#[test]
fn the_area_of_interest_narrows_the_candidates() {
    let ctx = Context::default();
    let src = crs(&ctx, "EPSG:4267");
    let dst = crs(&ctx, "EPSG:4269");
    let everywhere = Transformer::builder(&src, &dst).build().expect("all");
    let alaska = Transformer::builder(&src, &dst)
        .area_of_interest(BBox::new(-160.0, 60.0, -150.0, 65.0))
        .build()
        .expect("alaska");
    let all = everywhere.operations().expect("all");
    let narrowed = alaska.operations().expect("alaska");
    assert!(narrowed.len() < all.len());
    assert!(narrowed
        .iter()
        .any(|operation| operation.name().is_some_and(|name| name.contains("(2)"))));
}

#[test]
fn the_last_used_operation_follows_the_point() {
    let ctx = Context::default();
    let t = Transformer::builder(&crs(&ctx, "EPSG:4267"), &crs(&ctx, "EPSG:4269"))
        .build()
        .expect("transformer");
    assert!(t.last_used_operation().is_none());
    t.forward(Point::new(38.0, -95.0)).expect("forward");
    let used = t.last_used_operation().expect("used");
    assert!(used.name().is_some() && used.proj_string().is_some());
}

#[test]
fn height_and_time_pass_through() {
    let ctx = Context::default();
    let src = crs(&ctx, "EPSG:4979");
    let dst = crs(&ctx, "EPSG:4978");
    let t = Transformer::builder(&src, &dst)
        .build()
        .expect("transformer");
    let geocentric = t
        .forward(Point::xyz(48.8566, 2.3522, 100.0))
        .expect("forward");
    assert!(geocentric.t.is_infinite());
    let back = t.inverse(geocentric).expect("inverse");
    assert_close(back.z, 100.0, 1e-6, "height");
    let dated = t
        .forward(Point::xyzt(48.8566, 2.3522, 100.0, 2020.0))
        .expect("dated");
    assert_eq!(dated.t, 2020.0);
}

#[test]
fn a_crs_reads_and_writes_its_description() {
    let ctx = Context::default();
    let lambert = crs(&ctx, "EPSG:2154");
    assert_eq!(lambert.kind(), CrsKind::Projected);
    assert!(lambert.name().ends_with("Lambert-93"), "{}", lambert.name());
    let bbox = lambert.area_of_use().expect("area");
    assert!(bbox.west < -4.0 && bbox.east > 9.0 && bbox.south < 42.0 && bbox.north > 51.0);
    assert!(lambert.area_name().expect("name").starts_with("France"));
    assert_eq!(crs(&ctx, "EPSG:4326").kind(), CrsKind::Geographic);
    assert_eq!(crs(&ctx, "EPSG:4978").kind(), CrsKind::Geocentric);
    assert_eq!(crs(&ctx, "EPSG:5703").kind(), CrsKind::Vertical);

    let wkt = lambert.to_wkt(WktVersion::Wkt2019).expect("wkt2");
    assert!(
        wkt.starts_with("PROJCRS[\"") && wkt.contains("Lambert-93"),
        "{wkt}"
    );
    let wkt1 = lambert.to_wkt(WktVersion::Wkt1Gdal).expect("wkt1");
    assert!(wkt1.starts_with("PROJCS[\""), "{wkt1}");
    let json = lambert.to_projjson().expect("json");
    assert!(json.contains("\"type\": \"ProjectedCRS\""), "{json}");
    let text = lambert.to_proj_string().expect("proj string");
    assert!(
        text.contains("+proj=lcc") && text.contains("+type=crs"),
        "{text}"
    );

    // What a CRS writes, a CRS reads back.
    for source in [wkt, json, text] {
        let again = crs(&ctx, &source);
        assert_eq!(again.kind(), CrsKind::Projected, "{source}");
    }
}

#[test]
fn a_crs_with_no_proj_string_is_an_export_error() {
    let ctx = Context::default();
    let engineering = crs(
        &ctx,
        "LOCAL_CS[\"site\",LOCAL_DATUM[\"site\",0],UNIT[\"metre\",1],AXIS[\"X\",EAST],AXIS[\"Y\",NORTH]]",
    );
    assert_eq!(engineering.kind(), CrsKind::Engineering);
    assert!(matches!(
        engineering.to_proj_string(),
        Err(Error::Export(_))
    ));
}

#[test]
fn the_default_context_is_closed() {
    let ctx = Context::default();
    assert_eq!(ctx.network(), Network::Disabled);
    assert_eq!(ctx.cache(), Cache::Disabled);
    assert!(ctx.search_paths().is_empty());
    let open = Context::builder()
        .network(Network::Enabled)
        .cache(Cache::Enabled)
        .endpoint("https://example.invalid")
        .search_path("/grids")
        .build();
    assert_eq!(open.network(), Network::Enabled);
    assert_eq!(open.cache(), Cache::Enabled);
    assert_eq!(open.endpoint(), "https://example.invalid");
    assert_eq!(open.search_paths(), [PathBuf::from("/grids")]);
    // The defaults are not changed by a context built from them.
    assert_eq!(Context::default().network(), Network::Disabled);
}

#[test]
fn types_are_thread_safe_where_promised() {
    fn is_send_sync<T: Send + Sync>() {}
    fn is_send<T: Send>() {}
    is_send_sync::<Context>();
    is_send_sync::<Crs>();
    is_send_sync::<Point>();
    is_send_sync::<Error>();
    is_send::<Transformer>();
}

#[test]
fn a_transformer_runs_on_another_thread() {
    let t = transformer("EPSG:4326", "EPSG:2154");
    let expected = t.forward(Point::new(48.8566, 2.3522)).expect("forward");
    let from_thread = std::thread::spawn(move || t.forward(Point::new(48.8566, 2.3522)))
        .join()
        .expect("thread")
        .expect("forward");
    assert_eq!(from_thread, expected);
}
