//! A WKT1 geographic CRS with an authority code and axes equivalent to the authority's: PROJ
//! 9.9.0's `buildGeodeticCRS` takes the authority coordinate system, its axis names included
//! ("geodetic latitude (Lat)"), with the angular unit of the WKT (`alterAngularUnit`). GDAL's
//! `SetWellKnownGeogCS("WGS84")` builds such a WKT1. Expected values: `proj_as_wkt(PJ_WKT2_2019)`
//! of PROJ 9.9.0's C library.

use proj_rs::{create_from_user_input, DatabaseContext, WktConvention, WktFormatter};

fn wkt2_2019(wkt1: &str) -> String {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = create_from_user_input(wkt1, &db).expect("WKT1");
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2019).with_database(db);
    formatter.set_lines(proj_rs::WktLines::Multi);
    crs.export_to_wkt(&mut formatter).expect("WKT2")
}

#[test]
fn equivalent_axes_take_the_authority_names() {
    assert_eq!(
        wkt2_2019(
            r#"GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563,AUTHORITY["EPSG","7030"]],AUTHORITY["EPSG","6326"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]],AXIS["Latitude",NORTH],AXIS["Longitude",EAST],AUTHORITY["EPSG","4326"]]"#
        ),
        r#"GEOGCRS["WGS 84",
    DATUM["World Geodetic System 1984",
        ELLIPSOID["WGS 84",6378137,298.257223563,
            LENGTHUNIT["metre",1]]],
    PRIMEM["Greenwich",0,
        ANGLEUNIT["degree",0.0174532925199433]],
    CS[ellipsoidal,2],
        AXIS["geodetic latitude (Lat)",north,
            ORDER[1],
            ANGLEUNIT["degree",0.0174532925199433]],
        AXIS["geodetic longitude (Lon)",east,
            ORDER[2],
            ANGLEUNIT["degree",0.0174532925199433]],
    ID["EPSG",4326]]"#
    );
}

#[test]
fn the_angular_unit_of_the_wkt_is_kept() {
    assert_eq!(
        wkt2_2019(
            r#"GEOGCS["NTF (Paris)",DATUM["Nouvelle_Triangulation_Francaise_Paris",SPHEROID["Clarke 1880 (IGN)",6378249.2,293.4660212936269,AUTHORITY["EPSG","7011"]],AUTHORITY["EPSG","6807"]],PRIMEM["Paris",2.5969213,AUTHORITY["EPSG","8903"]],UNIT["grad",0.01570796326794897,AUTHORITY["EPSG","9105"]],AXIS["Latitude",NORTH],AXIS["Longitude",EAST],AUTHORITY["EPSG","4807"]]"#
        ),
        r#"GEOGCRS["NTF (Paris)",
    DATUM["Nouvelle Triangulation Francaise (Paris)",
        ELLIPSOID["Clarke 1880 (IGN)",6378249.2,293.466021293627,
            LENGTHUNIT["metre",1]]],
    PRIMEM["Paris",2.5969213,
        ANGLEUNIT["grad",0.015707963267949]],
    CS[ellipsoidal,2],
        AXIS["geodetic latitude (Lat)",north,
            ORDER[1],
            ANGLEUNIT["grad",0.015707963267949]],
        AXIS["geodetic longitude (Lon)",east,
            ORDER[2],
            ANGLEUNIT["grad",0.015707963267949]],
    ID["EPSG",4807]]"#
    );
}
