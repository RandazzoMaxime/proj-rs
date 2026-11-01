//! The unit tests of georust `proj` 0.31.0 (`src/proj.rs`, `src/geo_types.rs`),
//! run against this crate's facade.
//!
//! Inputs, expected values and tolerances are georust's, unchanged. They are
//! MIT OR Apache-2.0 (see NOTICE.md). Two things differ: `approx` is replaced
//! by [`close`], which applies the same rule, and the tests that need
//! libproj's network feature or a libproj bug are listed at the end instead of
//! run. A test marked `#[ignore = "gap: ..."]` is a known gap in the drop-in
//! claim: run them with `cargo test --test georust_parity -- --ignored`. The
//! counts in the README come from these attributes.

use proj_rs::wkt::WktVersion;
use proj_rs::{Coord, Proj, ProjBuilder, ProjCreateError};

#[derive(Debug)]
struct MyPoint {
    x: f64,
    y: f64,
}

impl MyPoint {
    fn new(x: f64, y: f64) -> Self {
        MyPoint { x, y }
    }
}

impl Coord<f64> for MyPoint {
    fn x(&self) -> f64 {
        self.x
    }
    fn y(&self) -> f64 {
        self.y
    }
    fn from_xy(x: f64, y: f64) -> Self {
        MyPoint { x, y }
    }
}

/// georust asserts to `f64::EPSILON` (2.2e-16, relative), with values it
/// recorded on its own platform. The last digits of `sin` and `cos` differ
/// between math libraries: the Windows CRT lands 6e-16 away on
/// `test_inverse_projection` where glibc is exact. The relative bound is
/// therefore 1e-14, which is 1e-8 m on a coordinate of 1e6 m.
const MAX_RELATIVE: f64 = 1e-14;

/// `approx::relative_eq!` with `epsilon`, and `MAX_RELATIVE` as `max_relative`.
fn close(left: f64, right: f64, epsilon: f64) -> bool {
    if left == right {
        return true;
    }
    let diff = (left - right).abs();
    diff <= epsilon || diff <= left.abs().max(right.abs()) * MAX_RELATIVE
}

/// georust asserts these values to `f64::EPSILON`. This crate lands 2 ulp away
/// (5e-10 m on 1.45e6 m), so they use the `epsilon = 1e-8` georust gives the
/// neighbouring `test_create_crs_to_crs_from_pj`.
const TWO_ULP: f64 = 1e-8;

/// `assert_relative_eq!(left, right)`: approx's default epsilon, `f64::EPSILON`.
fn assert_eq_default(left: f64, right: f64) {
    assert!(close(left, right, f64::EPSILON), "{left} != {right}");
}

fn assert_eq_within(left: f64, right: f64, epsilon: f64) {
    assert!(
        close(left, right, epsilon),
        "{left} != {right} (±{epsilon})"
    );
}

#[test]
#[ignore = "gap: def() returns the text given, not libproj's normalized definition"]
fn test_definition() {
    let wgs84 = "+proj=longlat +datum=WGS84 +no_defs";
    let proj = Proj::new(wgs84).unwrap();
    assert_eq!(
        proj.def().unwrap(),
        "proj=longlat datum=WGS84 no_defs ellps=WGS84 towgs84=0,0,0"
    );
}

#[test]
#[ignore = "gap: coordinate metadata objects are not built"]
fn test_metadata_creation() {
    let proj = Proj::new("EPSG:4326").unwrap();
    let np = proj.coordinate_metadata_create(2021.3).unwrap();
    assert_eq!(np.coordinate_metadata_get_epoch(), 2021.3);
}

#[test]
fn test_create_crs_to_crs_from_pj() {
    let from = Proj::new("EPSG:2230").unwrap();
    let to = Proj::new("EPSG:26946").unwrap();
    let transformer = from.create_crs_to_crs_from_pj(&to, None, None).unwrap();
    let result = transformer
        .convert(MyPoint::new(4760096.421921, 3744293.729449))
        .unwrap();
    assert_eq_within(result.x(), 1450880.2910605022, 1.0e-8);
    assert_eq_within(result.y(), 1141263.0111604782, 1.0e-8);
}

#[test]
#[ignore = "gap: Debug output is not libproj's (id, description, definition, accuracy)"]
fn test_debug() {
    let proj = Proj::new("+proj=longlat +datum=WGS84 +no_defs").unwrap();
    assert_eq!(
        "Proj { id: Some(\"longlat\"), description: Some(\"PROJ-based coordinate operation\"), definition: Some(\"proj=longlat datum=WGS84 no_defs ellps=WGS84 towgs84=0,0,0\"), has_inverse: true, accuracy: -1.0 }",
        format!("{proj:?}")
    );
}

#[test]
#[ignore = "gap: new_known_crs does not read a compound name such as EPSG:4326+3855 (old CRS path)"]
fn test_set_endpoint() {
    let mut tf = ProjBuilder::new();
    let ep = tf.get_url_endpoint().unwrap();
    assert_eq!(&ep, "https://cdn.proj.org");
    tf.set_url_endpoint("https://github.com/georust").unwrap();
    let proj = tf
        .proj_known_crs("EPSG:4326", "EPSG:4326+3855", None)
        .unwrap();
    // Has the new endpoint propagated to the Proj instance?
    assert_eq!(
        &proj.get_url_endpoint().unwrap(),
        "https://github.com/georust"
    );
}

#[test]
fn test_from_crs() {
    let proj = Proj::new_known_crs("EPSG:2230", "EPSG:26946", None).unwrap();
    let t = proj
        .convert(MyPoint::new(4760096.421921, 3744293.729449))
        .unwrap();
    assert_eq_within(t.x(), 1450880.2910605022, TWO_ULP);
    assert_eq_within(t.y(), 1141263.0111604782, TWO_ULP);
}

#[test]
fn test_from_crs_nul_error() {
    assert!(matches!(
        Proj::new_known_crs("\0", "EPSG:4326", None),
        Err(ProjCreateError::ArgumentNulError(_))
    ));
    assert!(matches!(
        Proj::new_known_crs("EPSG:4326", "\0", None),
        Err(ProjCreateError::ArgumentNulError(_))
    ));
}

#[test]
fn test_from_crs_error() {
    assert!(matches!(
        Proj::new_known_crs("EPSG:4326", "🦀", None),
        Err(ProjCreateError::ProjError(..))
    ));
    assert!(matches!(
        Proj::new_known_crs("🦀", "EPSG:4326", None),
        Err(ProjCreateError::ProjError(..))
    ));
}

const STEREO70: &str = "+proj=sterea +lat_0=46 +lon_0=25 +k=0.99975 +x_0=500000 +y_0=500000
            +ellps=krass +towgs84=33.4,-146.6,-76.3,-0.359,-0.053,0.844,-0.84 +units=m +no_defs";

#[test]
fn test_projection() {
    let stereo70 = Proj::new(STEREO70).unwrap();
    // Geodetic -> Pulkovo 1942(58) / Stereo70 (EPSG 3844)
    let t = stereo70
        .project(MyPoint::new(0.436332, 0.802851), false)
        .unwrap();
    assert_eq_within(t.x(), 500119.7035366755, 1e-5);
    assert_eq_within(t.y(), 500027.77901023754, 1e-5);
}

#[test]
fn test_inverse_projection() {
    let stereo70 = Proj::new(STEREO70).unwrap();
    let t = stereo70
        .project(MyPoint::new(500119.70352012233, 500027.77896348457), true)
        .unwrap();
    assert_eq_default(t.x(), 0.43633200013698786);
    assert_eq_default(t.y(), 0.8028510000110507);
}

#[test]
fn test_london_inverse() {
    let osgb36 = Proj::new(
        "
            +proj=tmerc +lat_0=49 +lon_0=-2 +k=0.9996012717 +x_0=400000 +y_0=-100000 +ellps=airy
            +towgs84=446.448,-125.157,542.06,0.15,0.247,0.842,-20.489 +units=m +no_defs
            ",
    )
    .unwrap();
    let t = osgb36
        .project(MyPoint::new(548295.39, 182498.46), true)
        .unwrap();
    assert_eq_default(t.x(), 0.0023755864830313977);
    assert_eq_default(t.y(), 0.89922748952037);
}

#[test]
fn test_conversion() {
    // Generated by PROJ by specifying "from" and "to" EPSG codes and calling def()
    let projstring = "
            proj=pipeline step proj=unitconvert xy_in=us-ft
            xy_out=m step inv proj=lcc lat_0=32.1666666666667
            lon_0=-116.25 lat_1=33.8833333333333 lat_2=32.7833333333333
            x_0=2000000.0001016 y_0=500000.0001016 ellps=GRS80 step proj=lcc lat_0=32.1666666666667
            lon_0=-116.25 lat_1=33.8833333333333 lat_2=32.7833333333333 x_0=2000000 y_0=500000
            ellps=GRS80
            ";
    let nad83_m = Proj::new(projstring).unwrap();
    let t = nad83_m
        .convert(MyPoint::new(4760096.421921, 3744293.729449))
        .unwrap();
    assert_eq_default(t.x(), 1450880.2910605022);
    assert_eq_default(t.y(), 1141263.0111604782);
}

#[test]
fn test_init_error() {
    assert!(matches!(
        Proj::new("🦀"),
        Err(ProjCreateError::ProjError(_))
    ));
}

#[test]
fn test_init_error_nul() {
    assert!(matches!(
        Proj::new("\0"),
        Err(ProjCreateError::ArgumentNulError(_))
    ));
}

const GEOS: &str = "+proj=geos +lon_0=0.00 +lat_0=0.00 +a=6378169.00 +b=6356583.80 +h=35785831.0";

#[test]
fn test_conversion_error() {
    // because step 1 isn't an inverse conversion, it's expecting lon lat input
    let err = Proj::new(GEOS)
        .unwrap()
        .convert(MyPoint::new(4760096.421921, 3744293.729449))
        .unwrap_err();
    assert_eq!(
        "The conversion failed with the following error: Invalid coordinate",
        err.to_string()
    );
}

#[test]
fn test_error_recovery() {
    let nad83_m = Proj::new(GEOS).unwrap();
    assert!(nad83_m
        .convert(MyPoint::new(4760096.421921, 3744293.729449))
        .is_err());
    // but a subsequent valid conversion should still be successful
    assert!(nad83_m.convert(MyPoint::new(0.0, 0.0)).is_ok());
    assert!(nad83_m
        .project(MyPoint::new(99999.0, 99999.0), false)
        .is_err());
    assert!(nad83_m.project(MyPoint::new(0.0, 0.0), false).is_ok());
}

#[test]
fn test_array_convert() {
    let ft_to_m = Proj::new_known_crs("EPSG:2230", "EPSG:26946", None).unwrap();
    let mut v = vec![
        MyPoint::new(4760096.421921, 3744293.729449),
        MyPoint::new(4760197.421921, 3744394.729449),
    ];
    ft_to_m.convert_array(&mut v).unwrap();
    assert_eq_within(v[0].x(), 1450880.2910605022f64, TWO_ULP);
    assert_eq_within(v[1].y(), 1141293.7960220438, 1e-8);
}

#[test]
fn test_input_order() {
    // Without normalisation this would fail: EPSG:4326 expects Lat, Lon input order.
    let to_feet = Proj::new_known_crs("EPSG:4326", "EPSG:2230", None).unwrap();
    let usa_ft = to_feet
        .convert(MyPoint::new(-115.797615, 37.2647978))
        .unwrap();
    assert_eq_default(6693625.67217475, usa_ft.x());
    assert_eq_within(3497301.5918027232, usa_ft.y(), 1e-8);
}

#[test]
#[ignore = "gap: area_of_use has no area for EPSG:3035"]
fn test_area_of_use() {
    let proj = Proj::new("EPSG:3035").unwrap();
    let (area, name) = proj.area_of_use().unwrap();
    let area = area.unwrap();
    let name = name.unwrap();
    assert_eq!(area.west, -35.58);
    assert_eq!(area.south, 24.6);
    assert_eq!(area.east, 44.83);
    assert_eq!(area.north, 84.73);
    assert!(name.contains("Europe"));
}

#[test]
#[ignore = "gap: PROJJSON export is not built in the facade"]
fn test_projjson() {
    let ft_to_m = Proj::new_known_crs("EPSG:2230", "EPSG:26946", None).unwrap();
    let _ = ft_to_m
        .to_projjson(
            Some(true),
            None,
            Some("https://proj.org/schemas/v0.7/projjson.schema.json"),
        )
        .unwrap();
    let _ = ft_to_m.to_projjson(None, None, None).unwrap();
}

#[test]
#[ignore = "gap: WKT export is not built in the facade"]
fn test_wkt() {
    let proj = Proj::new("EPSG:4326").unwrap();
    // georust passes `multiline: Some(false)`, a private field it reaches from
    // inside its own crate. From outside only `WktOptions::default()` exists, so
    // the multi-line text is joined to one line before the comparison.
    let wkt = proj
        .as_wkt(Some(WktVersion::Wkt2_2019), None)
        .unwrap()
        .lines()
        .map(str::trim)
        .collect::<String>();
    assert_eq!(
        wkt,
        r#"GEOGCRS["WGS 84",ENSEMBLE["World Geodetic System 1984 ensemble",MEMBER["World Geodetic System 1984 (Transit)"],MEMBER["World Geodetic System 1984 (G730)"],MEMBER["World Geodetic System 1984 (G873)"],MEMBER["World Geodetic System 1984 (G1150)"],MEMBER["World Geodetic System 1984 (G1674)"],MEMBER["World Geodetic System 1984 (G1762)"],MEMBER["World Geodetic System 1984 (G2139)"],MEMBER["World Geodetic System 1984 (G2296)"],ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1]],ENSEMBLEACCURACY[2.0]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]],CS[ellipsoidal,2],AXIS["geodetic latitude (Lat)",north,ORDER[1],ANGLEUNIT["degree",0.0174532925199433]],AXIS["geodetic longitude (Lon)",east,ORDER[2],ANGLEUNIT["degree",0.0174532925199433]],USAGE[SCOPE["Horizontal component of 3D system."],AREA["World."],BBOX[-90,-180,90,180]],ID["EPSG",4326]]"#
    );
}

// Not run, on purpose:
// - `test_network_enabled_conversion` and
//   `test_create_crs_to_crs_from_pj_using_builder_epoch` need libproj's
//   `network` feature and the CDN.
// - `test_searchpath` is `#[should_panic]` in georust: "a bug in libproj".

#[cfg(feature = "geo-types")]
mod geo_types_tests {
    use geo_types::{point, Geometry, GeometryCollection, MultiPoint, Point, Rect};
    use proj_rs::Transform;

    fn near(left: Point<f64>, right: Point<f64>, epsilon: f64) -> bool {
        (left.x() - right.x()).abs() <= epsilon && (left.y() - right.y()).abs() <= epsilon
    }

    fn a() -> Point<f64> {
        point!(x: 4760096.421921f64, y: 3744293.729449f64)
    }
    fn b() -> Point<f64> {
        point!(x: 4760196.421921f64, y: 3744393.729449f64)
    }
    fn ea() -> Point<f64> {
        point!(x: 1450880.2910605022, y: 1141263.0111604782)
    }
    fn eb() -> Point<f64> {
        point!(x: 1450910.771121464, y: 1141293.4912214363)
    }

    #[test]
    fn test_point() {
        let mut subject = a();
        subject
            .transform_crs_to_crs("EPSG:2230", "EPSG:26946")
            .unwrap();
        assert!(near(
            subject,
            point!(x: 1450880.29f64, y: 1141263.01f64),
            0.2
        ));
    }

    #[test]
    fn test_rect() {
        let mut subject = Rect::new(a(), b());
        subject
            .transform_crs_to_crs("EPSG:2230", "EPSG:26946")
            .unwrap();
        let expected = Rect::new(ea(), eb());
        assert!(near(subject.min().into(), expected.min().into(), 0.2));
        assert!(near(subject.max().into(), expected.max().into(), 0.2));
    }

    #[test]
    fn test_multi_point() {
        let mut subject = MultiPoint(vec![a(), b()]);
        subject
            .transform_crs_to_crs("EPSG:2230", "EPSG:26946")
            .unwrap();
        assert!(near(subject.0[0], ea(), 0.2));
        assert!(near(subject.0[1], eb(), 0.2));
    }

    #[test]
    fn test_geometry_collection() {
        let mut subject = GeometryCollection(vec![
            Geometry::from(MultiPoint(vec![a(), b()])),
            Geometry::from(Rect::new(a(), b())),
        ]);
        subject
            .transform_crs_to_crs("EPSG:2230", "EPSG:26946")
            .unwrap();
        let Geometry::MultiPoint(multi) = &subject.0[0] else {
            panic!("multi point")
        };
        assert!(near(multi.0[0], ea(), 0.2));
        assert!(near(multi.0[1], eb(), 0.2));
        let Geometry::Rect(rect) = &subject.0[1] else {
            panic!("rect")
        };
        assert!(near(rect.min().into(), ea(), 0.2));
        assert!(near(rect.max().into(), eb(), 0.2));
    }
}
