//! `+datum=` with `+R`, `+a`, `+b`, `+rf` or `+ellps`. `pj_datum_set` appends the datum's
//! `ellps=` at the end of the list, so an earlier `+ellps` wins, and `pj_ellipsoid` then lets
//! `+R`, `+a` and the shape keys override it. `buildDatum` turns such a string into an `unknown`
//! datum, the SRS GMT writes for its grids. Expected values: `proj -f %.9f` and
//! `projinfo -o WKT1_GDAL` of PROJ 9.9.0.

use proj_rs::wkt::WktVersion;
use proj_rs::Proj;

fn forward(definition: &str) -> (f64, f64) {
    let proj = Proj::new(definition).expect(definition);
    proj.convert((3.0_f64.to_radians(), 45.0_f64.to_radians()))
        .expect("point")
}

#[test]
fn numbers_after_a_datum_override_its_ellipsoid() {
    let cases = [
        (
            "+proj=merc +datum=WGS84 +a=6378000",
            (333_951.299_076_595, 5_591_175.819_605_871),
        ),
        (
            "+proj=merc +datum=WGS84 +a=6378137 +b=6356752.314",
            (333_958.472_379_821, 5_591_295.918_207_046),
        ),
        (
            "+proj=merc +datum=NAD27 +rf=300",
            (333_962.106_155_323, 5_591_532.248_804_304),
        ),
        (
            "+proj=merc +datum=WGS84 +R=6370000",
            (333_532.420_056_116, 5_614_349.749_314_489),
        ),
        (
            "+proj=merc +datum=WGS84 +ellps=intl",
            (333_971.614_709_088, 5_591_388.073_922_344),
        ),
    ];
    for (definition, expected) in cases {
        let got = forward(definition);
        assert!(
            (got.0 - expected.0).abs() <= 1e-6 && (got.1 - expected.1).abs() <= 1e-6,
            "{definition}: {got:?} vs {expected:?}"
        );
    }
}

#[test]
fn a_datum_with_its_own_axes_is_an_unknown_datum() {
    let crs =
        Proj::new("+proj=longlat +a=6378137 +b=6356752.314 +datum=WGS84 +type=crs").expect("crs");
    let wkt = crs
        .as_wkt(Some(WktVersion::Wkt1_Gdal), None)
        .expect("WKT1_GDAL");
    assert_eq!(
        wkt,
        "GEOGCS[\"unknown\",\n    DATUM[\"unknown\",\n        \
         SPHEROID[\"unknown\",6378137,298.257220143428]],\n    \
         PRIMEM[\"Greenwich\",0,\n        AUTHORITY[\"EPSG\",\"8901\"]],\n    \
         UNIT[\"degree\",0.0174532925199433,\n        AUTHORITY[\"EPSG\",\"9122\"]]]"
    );
}
