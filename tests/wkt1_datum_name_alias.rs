//! The WKT1 GDAL name of a datum without identifier: PROJ 9.9.0 (`GeodeticReferenceFrame::
//! _exportToWKT`) looks the name up among the datums and their aliases other than EPSG_OLD, and
//! writes it the GDAL way (`World_Geodetic_System_1984` is `WGS_1984`) when one EPSG datum
//! answers under an equivalent name; an ensemble answers under its datum name. GDAL builds such a
//! datum for `CRS:84`. Expected values: `proj_as_wkt(PJ_WKT1_GDAL)` with MULTILINE=NO, of PROJ 9.9.0's C library.

use proj_rs::{create_from_user_input, DatabaseContext, WktConvention, WktFormatter, WktLines};

fn wkt1_gdal(wkt: &str) -> String {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = create_from_user_input(wkt, &db).expect("WKT1");
    let mut formatter = WktFormatter::create(WktConvention::Wkt1Gdal).with_database(db);
    formatter.set_lines(WktLines::Single);
    crs.export_to_wkt(&mut formatter).expect("WKT1 GDAL")
}

#[test]
fn a_datum_name_found_as_an_alias() {
    assert_eq!(
        wkt1_gdal(
            r#"GEOGCS["x",DATUM["World Geodetic System 1984",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]]"#
        ),
        r#"GEOGCS["x",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]]"#
    );
    assert_eq!(
        wkt1_gdal(
            r#"GEOGCS["x",DATUM["European Terrestrial Reference System 1989",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]]"#
        ),
        r#"GEOGCS["x",DATUM["European_Terrestrial_Reference_System_1989",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]]"#
    );
    assert_eq!(
        wkt1_gdal(
            r#"GEOGCS["x",DATUM["WGS 84",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]]"#
        ),
        r#"GEOGCS["x",DATUM["WGS 84",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]]"#
    );
}
