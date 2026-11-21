//! The deprecated `EPSG:3785` (Popular Visualisation CRS / Mercator), whose conversion `EPSG:19847`
//! uses the deprecated method 9841. `Conversion::_exportToPROJString` writes a conversion named
//! "Popular Visualisation Mercator" as `+proj=webmerc` with the WGS 84 ellipsoid when it has a
//! source CRS (conversion.cpp:4111-4124), not with the sphere of its base. QGIS's extent widgets
//! reproject to it (`test_qgsextentwidget.py`). Expected values: C PROJ 9.9.0.

use proj_rs::iso19111::{create_from_user_input, AuthorityFactory};
use proj_rs::{
    CoordinateOperationContext, CoordinateOperationFactory, DatabaseContext, Proj, ProjLines,
};

fn first_operation(source: &str, target: &str) -> String {
    let db = DatabaseContext::create().expect("proj.db");
    let source = create_from_user_input(source, &db).expect("source");
    let target = create_from_user_input(target, &db).expect("target");
    let context = CoordinateOperationContext::create(&db, "");
    let operations = CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations");
    operations
        .first()
        .expect("one operation")
        .export_to_proj_string_with_database(&db, ProjLines::Single)
        .expect("PROJ string")
}

#[test]
fn operations_write_webmerc_on_wgs84() {
    let forward = "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert \
                   +xy_in=deg +xy_out=rad +step +proj=webmerc +ellps=WGS84";
    assert_eq!(first_operation("EPSG:4326", "EPSG:3785"), forward);
    assert_eq!(first_operation("EPSG:4258", "EPSG:3785"), forward);
    assert_eq!(
        first_operation("EPSG:3785", "EPSG:4326"),
        "+proj=pipeline +step +inv +proj=webmerc +ellps=WGS84 +step +proj=unitconvert +xy_in=rad \
         +xy_out=deg +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        first_operation("EPSG:3785", "EPSG:3857"),
        "+proj=pipeline +step +inv +proj=webmerc +ellps=WGS84 +step +proj=webmerc +lat_0=0 \
         +lon_0=0 +x_0=0 +y_0=0 +ellps=WGS84"
    );
}

#[test]
fn the_conversion_alone_is_webmerc() {
    let db = DatabaseContext::create().expect("proj.db");
    let conversion = AuthorityFactory::create(&db, "EPSG")
        .create_conversion("19847")
        .expect("EPSG:19847");
    assert_eq!(
        conversion.export_to_proj_string().expect("PROJ string"),
        "+proj=webmerc"
    );
}

#[test]
fn transforms_as_libproj() {
    let proj = Proj::new_known_crs("EPSG:4326", "EPSG:3785", None).expect("pair");
    let (x, y) = proj.convert((1.0_f64, 2.0_f64)).expect("forward");
    assert!((x - 111_319.490_793_274).abs() < 1e-6, "x {x}");
    assert!((y - 222_684.208_505_544).abs() < 1e-6, "y {y}");
}
