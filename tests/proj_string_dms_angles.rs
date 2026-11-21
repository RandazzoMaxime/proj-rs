//! Angles of a PROJ string written in DMS (`3d30'E`, `45d30'30"N`): PROJ 9.9.0's
//! `PROJStringParser` reads every angular parameter with `getAngularValue`, which is `dmstor` and
//! must read the whole value. GDAL's `ImportFromProj4` passes such strings
//! (`+lon_0=90dw +lat_1=44d4'n`). Expected values: `proj_as_wkt(PJ_WKT2_2019, MULTILINE=NO)` of
//! PROJ 9.9.0's C library.

use proj_rs::{create_from_user_input, DatabaseContext, WktConvention, WktFormatter, WktLines};

fn wkt2(text: &str) -> String {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = create_from_user_input(text, &db).unwrap_or_else(|error| panic!("{text}: {error}"));
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2019).with_database(db);
    formatter.set_lines(WktLines::Single);
    crs.export_to_wkt(&mut formatter).expect("WKT2")
}

#[test]
fn dms_angles_are_read_as_proj_reads_them() {
    assert_eq!(
        wkt2(
            r#"+proj=tmerc +lat_0=0 +lon_0=3d30'E +k=0.9996 +x_0=500000 +y_0=0 +ellps=GRS80 +type=crs"#
        ),
        r#"PROJCRS["unknown",BASEGEOGCRS["unknown",DATUM["Unknown based on GRS 1980 ellipsoid",ELLIPSOID["GRS 1980",6378137,298.257222101,LENGTHUNIT["metre",1],ID["EPSG",7019]]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8901]]],CONVERSION["unknown",METHOD["Transverse Mercator",ID["EPSG",9807]],PARAMETER["Latitude of natural origin",0,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8801]],PARAMETER["Longitude of natural origin",3.5,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8802]],PARAMETER["Scale factor at natural origin",0.9996,SCALEUNIT["unity",1],ID["EPSG",8805]],PARAMETER["False easting",500000,LENGTHUNIT["metre",1],ID["EPSG",8806]],PARAMETER["False northing",0,LENGTHUNIT["metre",1],ID["EPSG",8807]]],CS[Cartesian,2],AXIS["(E)",east,ORDER[1],LENGTHUNIT["metre",1,ID["EPSG",9001]]],AXIS["(N)",north,ORDER[2],LENGTHUNIT["metre",1,ID["EPSG",9001]]]]"#
    );
    assert_eq!(
        wkt2(r#"+proj=merc +lat_ts=45d30'30"N +lon_0=0 +ellps=WGS84 +type=crs"#),
        r#"PROJCRS["unknown",BASEGEOGCRS["unknown",DATUM["Unknown based on WGS 84 ellipsoid",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1],ID["EPSG",7030]]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8901]]],CONVERSION["unknown",METHOD["Mercator (variant B)",ID["EPSG",9805]],PARAMETER["Latitude of 1st standard parallel",45.5083333333333,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8823]],PARAMETER["Longitude of natural origin",0,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8802]],PARAMETER["False easting",0,LENGTHUNIT["metre",1],ID["EPSG",8806]],PARAMETER["False northing",0,LENGTHUNIT["metre",1],ID["EPSG",8807]]],CS[Cartesian,2],AXIS["(E)",east,ORDER[1],LENGTHUNIT["metre",1,ID["EPSG",9001]]],AXIS["(N)",north,ORDER[2],LENGTHUNIT["metre",1,ID["EPSG",9001]]]]"#
    );
}

#[test]
fn a_value_dmstor_does_not_read_whole_is_refused() {
    let db = DatabaseContext::create().expect("proj.db");
    assert!(create_from_user_input(
        "+proj=tmerc +lat_0=0 +lon_0=12dX +ellps=GRS80 +type=crs",
        &db
    )
    .is_err());
}
