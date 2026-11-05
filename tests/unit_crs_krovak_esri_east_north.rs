//! PROJ 9.9.0 `test/unit/test_crs_990.cpp`, `projectedCRS_from_WKT1_ESRI_as_WKT1_ESRI_s_jtsk03_krovak_east_north`
//! (EPSG:8353): an ESRI WKT1 Krovak CRS, read and written back in WKT2 and in ESRI WKT1.

use proj_rs::{Crs, DatabaseContext, WktConvention, WktFormatter, WktObject, WktParser};

/// `TEST(crs, projectedCRS_from_WKT1_ESRI_as_WKT1_ESRI_s_jtsk03_krovak_east_north)`
#[test]
fn crs_projected_crs_from_wkt1_esri_krovak_east_north() {
    let db = DatabaseContext::create().expect("proj.db");
    let wkt = "PROJCS[\"S-JTSK_[JTSK03]_Krovak_East_North\",\
        GEOGCS[\"S-JTSK_[JTSK03]\",DATUM[\"S-JTSK_[JTSK03]\",\
        SPHEROID[\"Bessel_1841\",6377397.155,299.1528128]],\
        PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],\
        PROJECTION[\"Krovak\"],\
        PARAMETER[\"False_Easting\",0.0],\
        PARAMETER[\"False_Northing\",0.0],\
        PARAMETER[\"Pseudo_Standard_Parallel_1\",78.5],\
        PARAMETER[\"Scale_Factor\",0.9999],\
        PARAMETER[\"Azimuth\",30.2881397527778],\
        PARAMETER[\"Longitude_Of_Center\",24.8333333333333],\
        PARAMETER[\"Latitude_Of_Center\",49.5],\
        PARAMETER[\"X_Scale\",-1.0],\
        PARAMETER[\"Y_Scale\",1.0],\
        PARAMETER[\"XY_Plane_Rotation\",90.0],\
        UNIT[\"Meter\",1.0]]";
    let mut parser = WktParser::new();
    parser.attach_database(db.clone());
    let crs = match parser.create_from_wkt(wkt).expect("WKT") {
        WktObject::Crs(crs @ Crs::Projected(_)) => crs,
        other => panic!("expected a projected CRS: {other:?}"),
    };
    let expected_wkt2 = "PROJCRS[\"S-JTSK [JTSK03] / Krovak East North\",\n\
        \x20   BASEGEOGCRS[\"S-JTSK [JTSK03]\",\n\
        \x20       DATUM[\"System of the Unified Trigonometrical Cadastral Network [JTSK03]\",\n\
        \x20           ELLIPSOID[\"Bessel 1841\",6377397.155,299.1528128,\n\
        \x20               LENGTHUNIT[\"metre\",1]],\n\
        \x20           ID[\"EPSG\",1201]],\n\
        \x20       PRIMEM[\"Greenwich\",0,\n\
        \x20           ANGLEUNIT[\"Degree\",0.0174532925199433]]],\n\
        \x20   CONVERSION[\"unknown\",\n\
        \x20       METHOD[\"Krovak (North Orientated)\",\n\
        \x20           ID[\"EPSG\",1041]],\n\
        \x20       PARAMETER[\"Latitude of projection centre\",49.5,\n\
        \x20           ANGLEUNIT[\"Degree\",0.0174532925199433],\n\
        \x20           ID[\"EPSG\",8811]],\n\
        \x20       PARAMETER[\"Longitude of origin\",24.8333333333333,\n\
        \x20           ANGLEUNIT[\"Degree\",0.0174532925199433],\n\
        \x20           ID[\"EPSG\",8833]],\n\
        \x20       PARAMETER[\"Co-latitude of cone axis\",30.2881397527778,\n\
        \x20           ANGLEUNIT[\"Degree\",0.0174532925199433],\n\
        \x20           ID[\"EPSG\",1036]],\n\
        \x20       PARAMETER[\"Latitude of pseudo standard parallel\",78.5,\n\
        \x20           ANGLEUNIT[\"Degree\",0.0174532925199433],\n\
        \x20           ID[\"EPSG\",8818]],\n\
        \x20       PARAMETER[\"Scale factor on pseudo standard parallel\",0.9999,\n\
        \x20           SCALEUNIT[\"unity\",1],\n\
        \x20           ID[\"EPSG\",8819]],\n\
        \x20       PARAMETER[\"False easting\",0,\n\
        \x20           LENGTHUNIT[\"metre\",1],\n\
        \x20           ID[\"EPSG\",8806]],\n\
        \x20       PARAMETER[\"False northing\",0,\n\
        \x20           LENGTHUNIT[\"metre\",1],\n\
        \x20           ID[\"EPSG\",8807]]],\n\
        \x20   CS[Cartesian,2],\n\
        \x20       AXIS[\"(E)\",east,\n\
        \x20           ORDER[1],\n\
        \x20           LENGTHUNIT[\"metre\",1,\n\
        \x20               ID[\"EPSG\",9001]]],\n\
        \x20       AXIS[\"(N)\",north,\n\
        \x20           ORDER[2],\n\
        \x20           LENGTHUNIT[\"metre\",1,\n\
        \x20               ID[\"EPSG\",9001]]]]";
    let wkt2 = crs
        .export_to_wkt(
            &mut WktFormatter::create(WktConvention::Wkt2_2019).with_database(db.clone()),
        )
        .expect("WKT2");
    assert_eq!(wkt2, expected_wkt2);
    let esri = crs
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri).with_database(db))
        .expect("ESRI WKT1");
    assert_eq!(esri, wkt);
}
