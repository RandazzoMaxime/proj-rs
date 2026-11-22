//! A WKT1 geographic CRS with an authority code and no AXIS: PROJ 9.9.0's `buildGeodeticCRS`
//! takes the authority coordinate system, latitude first, with the angular unit of the WKT's
//! UNIT (`alterAngularUnit`). A unit other than the authority's unsets the identifier. Expected
//! values come from `proj_as_wkt` of PROJ 9.9.0's C library.

use proj_rs::{create_from_user_input, DatabaseContext, WktConvention, WktFormatter, WktLines};

fn export(wkt1: &str, convention: WktConvention) -> String {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = create_from_user_input(wkt1, &db).expect("WKT1");
    let mut formatter = WktFormatter::create(convention);
    formatter.set_lines(WktLines::Single);
    crs.export_to_wkt(&mut formatter).expect("export")
}

/// The WKT's `UNIT["degree",...]` has no AUTHORITY, so the unit keeps none (no `EPSG:9122`).
#[test]
fn same_unit_keeps_the_wkt_unit() {
    for wkt1 in [
        r#"GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433],AUTHORITY["EPSG","4326"]]"#,
        r#"PROJCS["WGS 84 / UTM zone 31N",GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433],AUTHORITY["EPSG","4326"]],PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",0],PARAMETER["central_meridian",3],PARAMETER["scale_factor",0.9996],PARAMETER["false_easting",500000],PARAMETER["false_northing",0],UNIT["metre",1],AXIS["Easting",EAST],AXIS["Northing",NORTH],AUTHORITY["EPSG","32631"]]"#,
    ] {
        let exported = export(wkt1, WktConvention::Wkt1Gdal);
        assert!(
            exported.contains(r#"UNIT["degree",0.0174532925199433],AUTHORITY["EPSG","4326"]"#),
            "{exported}"
        );
    }
}

#[test]
fn other_unit_takes_the_authority_axes_without_identifier() {
    let cases = [
        (
            r#"GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["grad",0.015707963267949],AUTHORITY["EPSG","4326"]]"#,
            r#"GEOGCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1,ID["EPSG",9001]]]],PRIMEM["Greenwich",0,ANGLEUNIT["grad",0.015707963267949]],CS[ellipsoidal,2],AXIS["geodetic latitude (Lat)",north,ORDER[1],ANGLEUNIT["grad",0.015707963267949]],AXIS["geodetic longitude (Lon)",east,ORDER[2],ANGLEUNIT["grad",0.015707963267949]]]"#,
        ),
        (
            r#"GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["radian",1],AUTHORITY["EPSG","4326"]]"#,
            r#"GEOGCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1,ID["EPSG",9001]]]],PRIMEM["Greenwich",0,ANGLEUNIT["radian",1]],CS[ellipsoidal,2],AXIS["geodetic latitude (Lat)",north,ORDER[1],ANGLEUNIT["radian",1]],AXIS["geodetic longitude (Lon)",east,ORDER[2],ANGLEUNIT["radian",1]]]"#,
        ),
    ];
    for (wkt1, expected) in cases {
        assert_eq!(export(wkt1, WktConvention::Wkt2_2019), expected);
    }
}
