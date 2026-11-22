//! The ellipsoid of a PROJ string given by its axes: PROJ 9.9.0's `Ellipsoid::identify` names it
//! after the PROJ ellipsoid it matches (`GRS 1980` for every `GRS 1980…` entry) and keeps the
//! values as written: `+a=6378206.4 +rf=294.978698` stays rf 294.978698, though Clarke 1866 is
//! defined by its semi-minor axis. GDAL's `ImportFromProj4` passes such strings. Expected values:
//! `proj_as_wkt(PJ_WKT2_2019, MULTILINE=NO)` of PROJ 9.9.0's C library.

use proj_rs::{create_from_user_input, DatabaseContext, WktConvention, WktFormatter, WktLines};

fn wkt2(text: &str) -> String {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = create_from_user_input(text, &db).unwrap_or_else(|error| panic!("{text}: {error}"));
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2019).with_database(db);
    formatter.set_lines(WktLines::Single);
    crs.export_to_wkt(&mut formatter).expect("WKT2")
}

#[test]
fn a_recognised_ellipsoid_keeps_its_values() {
    assert_eq!(
        wkt2(r#"+proj=longlat +a=6378206.4 +rf=294.978698 +type=crs"#),
        r#"GEOGCRS["unknown",DATUM["Unknown based on Clarke 1866 ellipsoid",ELLIPSOID["Clarke 1866",6378206.4,294.978698,LENGTHUNIT["metre",1,ID["EPSG",9001]]]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8901]],CS[ellipsoidal,2],AXIS["longitude",east,ORDER[1],ANGLEUNIT["degree",0.0174532925199433,ID["EPSG",9122]]],AXIS["latitude",north,ORDER[2],ANGLEUNIT["degree",0.0174532925199433,ID["EPSG",9122]]]]"#
    );
    assert_eq!(
        wkt2(r#"+proj=longlat +a=6378137 +rf=298.257222101 +type=crs"#),
        r#"GEOGCRS["unknown",DATUM["Unknown based on GRS 1980 ellipsoid",ELLIPSOID["GRS 1980",6378137,298.257222101,LENGTHUNIT["metre",1,ID["EPSG",9001]]]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8901]],CS[ellipsoidal,2],AXIS["longitude",east,ORDER[1],ANGLEUNIT["degree",0.0174532925199433,ID["EPSG",9122]]],AXIS["latitude",north,ORDER[2],ANGLEUNIT["degree",0.0174532925199433,ID["EPSG",9122]]]]"#
    );
}
