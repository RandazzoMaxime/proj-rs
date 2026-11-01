//! PROJ `test/unit/test_operation.cpp` projection exports: the conversion
//! constructors of one projection, its PROJ string and its WKT.
//!
//! Each function is one `TEST()` name. The expected WKT and PROJ strings are
//! copied from PROJ 9.9.0 unchanged. Multi-line WKT is built with `concat!`
//! and `\n`, so the text does not depend on the line endings of this file.
//! The numeric checks keep PROJ's own tolerance, if any.

use proj_rs::{Conversion, Equivalence, OpProps, WktConvention, WktFormatter, WktParser};

fn props() -> OpProps {
    OpProps::new()
}

fn wkt2(conv: &Conversion) -> String {
    conv.export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("wkt2")
}

fn wkt1(conv: &Conversion) -> String {
    conv.export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
        .expect("wkt1")
}

fn proj(conv: &Conversion) -> String {
    conv.export_to_proj_string().expect("proj")
}

/// WKT2 export with `simulCurNodeHasId`, as PROJ does for the tests that use it.
fn wkt2_simul(conv: &Conversion) -> String {
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2019);
    formatter.simul_cur_node_has_id();
    conv.export_to_wkt(&mut formatter).expect("wkt2")
}

#[test]
fn operation_tmerc_south_oriented_export() {
    let conv =
        Conversion::create_transverse_mercator_south_oriented(&props(), 1.0, 2.0, 3.0, 4.0, 5.0)
            .expect("tmso");
    assert!(conv.validate_parameters().is_empty());
    // False easting/northing != 0 not supported
    assert!(conv.export_to_proj_string().is_err());

    assert_eq!(
        wkt2(&conv),
        concat!(
            "CONVERSION[\"Transverse Mercator (South Orientated)\",\n",
            "    METHOD[\"Transverse Mercator (South Orientated)\",\n",
            "        ID[\"EPSG\",9808]],\n",
            "    PARAMETER[\"Latitude of natural origin\",1,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
            "        ID[\"EPSG\",8801]],\n",
            "    PARAMETER[\"Longitude of natural origin\",2,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
            "        ID[\"EPSG\",8802]],\n",
            "    PARAMETER[\"Scale factor at natural origin\",3,\n",
            "        SCALEUNIT[\"unity\",1],\n",
            "        ID[\"EPSG\",8805]],\n",
            "    PARAMETER[\"False easting\",4,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8806]],\n",
            "    PARAMETER[\"False northing\",5,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8807]]]"
        )
    );

    assert_eq!(
        wkt1(&conv),
        concat!(
            "PROJECTION[\"Transverse_Mercator_South_Orientated\"],\n",
            "PARAMETER[\"latitude_of_origin\",1],\n",
            "PARAMETER[\"central_meridian\",2],\n",
            "PARAMETER[\"scale_factor\",3],\n",
            "PARAMETER[\"false_easting\",4],\n",
            "PARAMETER[\"false_northing\",5]"
        )
    );

    let wkt = concat!(
        "PROJCRS[\"Hartebeesthoek94 / Lo29\",",
        "  BASEGEODCRS[\"Hartebeesthoek94\",",
        "    DATUM[\"Hartebeesthoek94\",",
        "      ELLIPSOID[\"WGS 84\",6378137,298.257223563,LENGTHUNIT[\"metre\",1.0]]]],",
        "  CONVERSION[\"South African Survey Grid zone 29\",",
        "    METHOD[\"Transverse Mercator (South Orientated)\",ID[\"EPSG\",9808]],",
        "    PARAMETER[\"Latitude of natural origin\",0,ANGLEUNIT[\"degree\",0.01745329252]],",
        "    PARAMETER[\"Longitude of natural origin\",29,ANGLEUNIT[\"degree\",0.01745329252]],",
        "    PARAMETER[\"Scale factor at natural origin\",1,SCALEUNIT[\"unity\",1.0]],",
        "    PARAMETER[\"False easting\",0,LENGTHUNIT[\"metre\",1.0]],",
        "    PARAMETER[\"False northing\",0,LENGTHUNIT[\"metre\",1.0]]],",
        "  CS[cartesian,2],",
        "    AXIS[\"westing (Y)\",west,ORDER[1]],",
        "    AXIS[\"southing (X)\",south,ORDER[2]],",
        "    LENGTHUNIT[\"metre\",1.0],",
        "  ID[\"EPSG\",2053]]"
    );
    let obj = WktParser::new().create_from_wkt(wkt).expect("wkt");
    let crs = obj.projected().expect("projected");
    assert_eq!(
        crs.export_to_proj_string().expect("proj"),
        "+proj=tmerc +axis=wsu +lat_0=0 +lon_0=29 +k=1 +x_0=0 +y_0=0 \
         +ellps=WGS84 +units=m +no_defs +type=crs"
    );
}

#[test]
fn operation_tmerc_south_oriented_export_esri_102470() {
    // South Orientated TMerc presented as regular TMerc but with
    // Scale_Factor=-1
    let wkt = concat!(
        "PROJCS[\"Cape_Lo15\",GEOGCS[\"GCS_Cape\",DATUM[\"D_Cape\",",
        "SPHEROID[\"Clarke_1880_Arc\",6378249.145,293.4663077]],",
        "PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],",
        "PROJECTION[\"Transverse_Mercator\"],",
        "PARAMETER[\"False_Easting\",0.0],",
        "PARAMETER[\"False_Northing\",0.0],",
        "PARAMETER[\"Central_Meridian\",15.0],",
        "PARAMETER[\"Scale_Factor\",-1.0],",
        "PARAMETER[\"Latitude_Of_Origin\",0.0],",
        "UNIT[\"Meter\",1.0]]"
    );
    let obj = WktParser::new().create_from_wkt(wkt).expect("wkt");
    let crs = obj.projected().expect("projected");
    assert_eq!(
        crs.export_to_proj_string().expect("proj"),
        "+proj=tmerc +axis=wsu +lat_0=0 +lon_0=15 +k=1 +x_0=0 +y_0=0 \
         +a=6378249.145 +rf=293.4663077 +units=m +no_defs +type=crs"
    );
}

#[test]
fn operation_bonne_export() {
    let conv = Conversion::create_bonne(&props(), 1.0, 2.0, 3.0, 4.0).expect("bonne");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(proj(&conv), "+proj=bonne +lat_1=1 +lon_0=2 +x_0=3 +y_0=4");
    assert_eq!(
        wkt2(&conv),
        concat!(
            "CONVERSION[\"Bonne\",\n",
            "    METHOD[\"Bonne\",\n",
            "        ID[\"EPSG\",9827]],\n",
            "    PARAMETER[\"Latitude of natural origin\",1,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
            "        ID[\"EPSG\",8801]],\n",
            "    PARAMETER[\"Longitude of natural origin\",2,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
            "        ID[\"EPSG\",8802]],\n",
            "    PARAMETER[\"False easting\",3,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8806]],\n",
            "    PARAMETER[\"False northing\",4,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8807]]]"
        )
    );
    assert_eq!(
        wkt1(&conv),
        concat!(
            "PROJECTION[\"Bonne\"],\n",
            "PARAMETER[\"standard_parallel_1\",1],\n",
            "PARAMETER[\"central_meridian\",2],\n",
            "PARAMETER[\"false_easting\",3],\n",
            "PARAMETER[\"false_northing\",4]"
        )
    );

    let obj = WktParser::new()
        .create_from_wkt(concat!(
            "PROJCS[\"unnamed\",",
            "GEOGCS[\"unnamed ellipse\",",
            "    DATUM[\"unknown\",",
            "        SPHEROID[\"unnamed\",6378137,298.257223563]],",
            "    PRIMEM[\"Greenwich\",0],",
            "    UNIT[\"degree\",0.0174532925199433]],",
            "PROJECTION[\"Bonne\"],",
            "PARAMETER[\"standard_parallel_1\",1],",
            "PARAMETER[\"central_meridian\",2],",
            "PARAMETER[\"false_easting\",3],",
            "PARAMETER[\"false_northing\",4],",
            "UNIT[\"metre\",1]]"
        ))
        .expect("wkt");
    let crs = obj.projected().expect("projected");
    assert_eq!(
        crs.export_to_proj_string().expect("proj"),
        "+proj=bonne +lat_1=1 +lon_0=2 +x_0=3 +y_0=4 +ellps=WGS84 \
         +units=m +no_defs +type=crs"
    );
}

#[test]
fn operation_nzmg_export() {
    let conv =
        Conversion::create_new_zealand_mapping_grid(&props(), 1.0, 2.0, 4.0, 5.0).expect("nzmg");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(proj(&conv), "+proj=nzmg +lat_0=1 +lon_0=2 +x_0=4 +y_0=5");
    assert_eq!(
        wkt2(&conv),
        concat!(
            "CONVERSION[\"New Zealand Map Grid\",\n",
            "    METHOD[\"New Zealand Map Grid\",\n",
            "        ID[\"EPSG\",9811]],\n",
            "    PARAMETER[\"Latitude of natural origin\",1,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
            "        ID[\"EPSG\",8801]],\n",
            "    PARAMETER[\"Longitude of natural origin\",2,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
            "        ID[\"EPSG\",8802]],\n",
            "    PARAMETER[\"False easting\",4,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8806]],\n",
            "    PARAMETER[\"False northing\",5,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8807]]]"
        )
    );
    assert_eq!(
        wkt1(&conv),
        concat!(
            "PROJECTION[\"New_Zealand_Map_Grid\"],\n",
            "PARAMETER[\"latitude_of_origin\",1],\n",
            "PARAMETER[\"central_meridian\",2],\n",
            "PARAMETER[\"false_easting\",4],\n",
            "PARAMETER[\"false_northing\",5]"
        )
    );
}

#[test]
fn operation_oblique_stereographic_export() {
    let conv =
        Conversion::create_oblique_stereographic(&props(), 1.0, 2.0, 3.0, 4.0, 5.0).expect("os");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(
        proj(&conv),
        "+proj=sterea +lat_0=1 +lon_0=2 +k=3 +x_0=4 +y_0=5"
    );
    assert_eq!(
        wkt2(&conv),
        concat!(
            "CONVERSION[\"Oblique Stereographic\",\n",
            "    METHOD[\"Oblique Stereographic\",\n",
            "        ID[\"EPSG\",9809]],\n",
            "    PARAMETER[\"Latitude of natural origin\",1,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
            "        ID[\"EPSG\",8801]],\n",
            "    PARAMETER[\"Longitude of natural origin\",2,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
            "        ID[\"EPSG\",8802]],\n",
            "    PARAMETER[\"Scale factor at natural origin\",3,\n",
            "        SCALEUNIT[\"unity\",1],\n",
            "        ID[\"EPSG\",8805]],\n",
            "    PARAMETER[\"False easting\",4,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8806]],\n",
            "    PARAMETER[\"False northing\",5,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8807]]]"
        )
    );
    assert_eq!(
        wkt1(&conv),
        concat!(
            "PROJECTION[\"Oblique_Stereographic\"],\n",
            "PARAMETER[\"latitude_of_origin\",1],\n",
            "PARAMETER[\"central_meridian\",2],\n",
            "PARAMETER[\"scale_factor\",3],\n",
            "PARAMETER[\"false_easting\",4],\n",
            "PARAMETER[\"false_northing\",5]"
        )
    );
}

#[test]
fn operation_orthographic_export() {
    let conv = Conversion::create_orthographic(&props(), 1.0, 2.0, 4.0, 5.0).expect("ortho");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(proj(&conv), "+proj=ortho +lat_0=1 +lon_0=2 +x_0=4 +y_0=5");
    assert_eq!(
        wkt2(&conv),
        concat!(
            "CONVERSION[\"Orthographic\",\n",
            "    METHOD[\"Orthographic\",\n",
            "        ID[\"EPSG\",9840]],\n",
            "    PARAMETER[\"Latitude of natural origin\",1,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
            "        ID[\"EPSG\",8801]],\n",
            "    PARAMETER[\"Longitude of natural origin\",2,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
            "        ID[\"EPSG\",8802]],\n",
            "    PARAMETER[\"False easting\",4,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8806]],\n",
            "    PARAMETER[\"False northing\",5,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8807]]]"
        )
    );
    assert_eq!(
        wkt1(&conv),
        concat!(
            "PROJECTION[\"Orthographic\"],\n",
            "PARAMETER[\"latitude_of_origin\",1],\n",
            "PARAMETER[\"central_meridian\",2],\n",
            "PARAMETER[\"false_easting\",4],\n",
            "PARAMETER[\"false_northing\",5]"
        )
    );
}

#[test]
fn operation_local_orthographic_export() {
    let conv = Conversion::create_local_orthographic(&props(), 1.0, 2.0, 3.0, 1.25, 4.0, 5.0)
        .expect("local ortho");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(
        proj(&conv),
        "+proj=ortho +lat_0=1 +lon_0=2 +alpha=3 +k=1.25 +x_0=4 +y_0=5"
    );
    assert_eq!(
        wkt2(&conv),
        concat!(
            "CONVERSION[\"Local Orthographic\",\n",
            "    METHOD[\"Local Orthographic\",\n",
            "        ID[\"EPSG\",1130]],\n",
            "    PARAMETER[\"Latitude of projection centre\",1,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
            "        ID[\"EPSG\",8811]],\n",
            "    PARAMETER[\"Longitude of projection centre\",2,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
            "        ID[\"EPSG\",8812]],\n",
            "    PARAMETER[\"Azimuth at projection centre\",3,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
            "        ID[\"EPSG\",8813]],\n",
            "    PARAMETER[\"Scale factor at projection centre\",1.25,\n",
            "        SCALEUNIT[\"unity\",1],\n",
            "        ID[\"EPSG\",8815]],\n",
            "    PARAMETER[\"Easting at projection centre\",4,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8816]],\n",
            "    PARAMETER[\"Northing at projection centre\",5,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8817]]]"
        )
    );
    assert_eq!(
        wkt1(&conv),
        concat!(
            "PROJECTION[\"Local Orthographic\"],\n",
            "PARAMETER[\"latitude_of_center\",1],\n",
            "PARAMETER[\"longitude_of_center\",2],\n",
            "PARAMETER[\"azimuth\",3],\n",
            "PARAMETER[\"scale_factor\",1.25],\n",
            "PARAMETER[\"false_easting\",4],\n",
            "PARAMETER[\"false_northing\",5]"
        )
    );
}

#[test]
fn operation_american_polyconic_export() {
    let conv =
        Conversion::create_american_polyconic(&props(), 1.0, 2.0, 4.0, 5.0).expect("polyconic");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(proj(&conv), "+proj=poly +lat_0=1 +lon_0=2 +x_0=4 +y_0=5");
    assert_eq!(
        wkt2(&conv),
        concat!(
            "CONVERSION[\"American Polyconic\",\n",
            "    METHOD[\"American Polyconic\",\n",
            "        ID[\"EPSG\",9818]],\n",
            "    PARAMETER[\"Latitude of natural origin\",1,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
            "        ID[\"EPSG\",8801]],\n",
            "    PARAMETER[\"Longitude of natural origin\",2,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
            "        ID[\"EPSG\",8802]],\n",
            "    PARAMETER[\"False easting\",4,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8806]],\n",
            "    PARAMETER[\"False northing\",5,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8807]]]"
        )
    );
    assert_eq!(
        wkt1(&conv),
        concat!(
            "PROJECTION[\"Polyconic\"],\n",
            "PARAMETER[\"latitude_of_origin\",1],\n",
            "PARAMETER[\"central_meridian\",2],\n",
            "PARAMETER[\"false_easting\",4],\n",
            "PARAMETER[\"false_northing\",5]"
        )
    );
}

#[test]
fn operation_robinson_export() {
    let conv = Conversion::create_robinson(&props(), 1.0, 2.0, 3.0).expect("robinson");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(proj(&conv), "+proj=robin +lon_0=1 +x_0=2 +y_0=3");
    assert_eq!(
        wkt2(&conv),
        concat!(
            "CONVERSION[\"Robinson\",\n",
            "    METHOD[\"Robinson\"],\n",
            "    PARAMETER[\"Longitude of natural origin\",1,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
            "        ID[\"EPSG\",8802]],\n",
            "    PARAMETER[\"False easting\",2,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8806]],\n",
            "    PARAMETER[\"False northing\",3,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8807]]]"
        )
    );
    assert_eq!(
        wkt1(&conv),
        concat!(
            "PROJECTION[\"Robinson\"],\n",
            "PARAMETER[\"longitude_of_center\",1],\n",
            "PARAMETER[\"false_easting\",2],\n",
            "PARAMETER[\"false_northing\",3]"
        )
    );
}

#[test]
fn operation_sinusoidal_export() {
    let conv = Conversion::create_sinusoidal(&props(), 1.0, 2.0, 3.0).expect("sinu");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(proj(&conv), "+proj=sinu +lon_0=1 +x_0=2 +y_0=3");
    assert_eq!(
        wkt2(&conv),
        concat!(
            "CONVERSION[\"Sinusoidal\",\n",
            "    METHOD[\"Sinusoidal\"],\n",
            "    PARAMETER[\"Longitude of natural origin\",1,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
            "        ID[\"EPSG\",8802]],\n",
            "    PARAMETER[\"False easting\",2,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8806]],\n",
            "    PARAMETER[\"False northing\",3,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8807]]]"
        )
    );
    assert_eq!(
        wkt1(&conv),
        concat!(
            "PROJECTION[\"Sinusoidal\"],\n",
            "PARAMETER[\"longitude_of_center\",1],\n",
            "PARAMETER[\"false_easting\",2],\n",
            "PARAMETER[\"false_northing\",3]"
        )
    );
}

#[test]
fn operation_stereographic_export() {
    let conv = Conversion::create_stereographic(&props(), 1.0, 2.0, 3.0, 4.0, 5.0).expect("stere");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(
        proj(&conv),
        "+proj=stere +lat_0=1 +lon_0=2 +k=3 +x_0=4 +y_0=5"
    );
    assert_eq!(
        wkt2(&conv),
        concat!(
            "CONVERSION[\"Stereographic\",\n",
            "    METHOD[\"Stereographic\"],\n",
            "    PARAMETER[\"Latitude of natural origin\",1,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
            "        ID[\"EPSG\",8801]],\n",
            "    PARAMETER[\"Longitude of natural origin\",2,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
            "        ID[\"EPSG\",8802]],\n",
            "    PARAMETER[\"Scale factor at natural origin\",3,\n",
            "        SCALEUNIT[\"unity\",1],\n",
            "        ID[\"EPSG\",8805]],\n",
            "    PARAMETER[\"False easting\",4,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8806]],\n",
            "    PARAMETER[\"False northing\",5,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8807]]]"
        )
    );
    assert_eq!(
        wkt1(&conv),
        concat!(
            "PROJECTION[\"Stereographic\"],\n",
            "PARAMETER[\"latitude_of_origin\",1],\n",
            "PARAMETER[\"central_meridian\",2],\n",
            "PARAMETER[\"scale_factor\",3],\n",
            "PARAMETER[\"false_easting\",4],\n",
            "PARAMETER[\"false_northing\",5]"
        )
    );
}

#[test]
fn operation_vandergrinten_export() {
    let conv = Conversion::create_van_der_grinten(&props(), 1.0, 2.0, 3.0).expect("vandg");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(proj(&conv), "+proj=vandg +R_A +lon_0=1 +x_0=2 +y_0=3");
    assert_eq!(
        wkt2(&conv),
        concat!(
            "CONVERSION[\"Van Der Grinten\",\n",
            "    METHOD[\"Van Der Grinten\"],\n",
            "    PARAMETER[\"Longitude of natural origin\",1,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
            "        ID[\"EPSG\",8802]],\n",
            "    PARAMETER[\"False easting\",2,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8806]],\n",
            "    PARAMETER[\"False northing\",3,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8807]]]"
        )
    );
    assert_eq!(
        wkt1(&conv),
        concat!(
            "PROJECTION[\"VanDerGrinten\"],\n",
            "PARAMETER[\"central_meridian\",1],\n",
            "PARAMETER[\"false_easting\",2],\n",
            "PARAMETER[\"false_northing\",3]"
        )
    );
}

#[test]
fn operation_wagner_export() {
    // (PROJ number, roman numeral)
    for (index, number, roman) in [
        (1, "1", "I"),
        (2, "2", "II"),
        (4, "4", "IV"),
        (5, "5", "V"),
        (6, "6", "VI"),
        (7, "7", "VII"),
    ] {
        let conv = match index {
            1 => Conversion::create_wagner_i(&props(), 1.0, 2.0, 3.0),
            2 => Conversion::create_wagner_ii(&props(), 1.0, 2.0, 3.0),
            4 => Conversion::create_wagner_iv(&props(), 1.0, 2.0, 3.0),
            5 => Conversion::create_wagner_v(&props(), 1.0, 2.0, 3.0),
            6 => Conversion::create_wagner_vi(&props(), 1.0, 2.0, 3.0),
            _ => Conversion::create_wagner_vii(&props(), 1.0, 2.0, 3.0),
        }
        .expect("wagner");
        assert!(conv.validate_parameters().is_empty());
        assert_eq!(
            proj(&conv),
            format!("+proj=wag{number} +lon_0=1 +x_0=2 +y_0=3")
        );
        assert_eq!(
            wkt2_simul(&conv),
            format!(
                "CONVERSION[\"Wagner {roman}\",\n\
                 \x20   METHOD[\"Wagner {roman}\"],\n\
                 \x20   PARAMETER[\"Longitude of natural origin\",1,\n\
                 \x20       ANGLEUNIT[\"degree\",0.0174532925199433],\n\
                 \x20       ID[\"EPSG\",8802]],\n\
                 \x20   PARAMETER[\"False easting\",2,\n\
                 \x20       LENGTHUNIT[\"metre\",1],\n\
                 \x20       ID[\"EPSG\",8806]],\n\
                 \x20   PARAMETER[\"False northing\",3,\n\
                 \x20       LENGTHUNIT[\"metre\",1],\n\
                 \x20       ID[\"EPSG\",8807]]]"
            )
        );
        assert_eq!(
            wkt1(&conv),
            format!(
                "PROJECTION[\"Wagner_{roman}\"],\n\
                 PARAMETER[\"central_meridian\",1],\n\
                 PARAMETER[\"false_easting\",2],\n\
                 PARAMETER[\"false_northing\",3]"
            )
        );
    }
}

#[test]
fn operation_wagner_iii_export() {
    let conv = Conversion::create_wagner_iii(&props(), 1.0, 2.0, 3.0, 4.0).expect("wagner iii");
    assert_eq!(proj(&conv), "+proj=wag3 +lat_ts=1 +lon_0=2 +x_0=3 +y_0=4");
    assert_eq!(
        wkt2_simul(&conv),
        concat!(
            "CONVERSION[\"Wagner III\",\n",
            "    METHOD[\"Wagner III\"],\n",
            "    PARAMETER[\"Latitude of true scale\",1,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
            "    PARAMETER[\"Longitude of natural origin\",2,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
            "        ID[\"EPSG\",8802]],\n",
            "    PARAMETER[\"False easting\",3,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8806]],\n",
            "    PARAMETER[\"False northing\",4,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8807]]]"
        )
    );
    assert_eq!(
        wkt1(&conv),
        concat!(
            "PROJECTION[\"Wagner_III\"],\n",
            "PARAMETER[\"latitude_of_origin\",1],\n",
            "PARAMETER[\"central_meridian\",2],\n",
            "PARAMETER[\"false_easting\",3],\n",
            "PARAMETER[\"false_northing\",4]"
        )
    );
}

#[test]
fn operation_qsc_export() {
    let conv = Conversion::create_quadrilateralized_spherical_cube(&props(), 1.0, 2.0, 3.0, 4.0)
        .expect("qsc");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(proj(&conv), "+proj=qsc +lat_0=1 +lon_0=2 +x_0=3 +y_0=4");
    assert_eq!(
        wkt2(&conv),
        concat!(
            "CONVERSION[\"Quadrilateralized Spherical Cube\",\n",
            "    METHOD[\"Quadrilateralized Spherical Cube\"],\n",
            "    PARAMETER[\"Latitude of natural origin\",1,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
            "        ID[\"EPSG\",8801]],\n",
            "    PARAMETER[\"Longitude of natural origin\",2,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
            "        ID[\"EPSG\",8802]],\n",
            "    PARAMETER[\"False easting\",3,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8806]],\n",
            "    PARAMETER[\"False northing\",4,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8807]]]"
        )
    );
    assert_eq!(
        wkt1(&conv),
        concat!(
            "PROJECTION[\"Quadrilateralized_Spherical_Cube\"],\n",
            "PARAMETER[\"latitude_of_origin\",1],\n",
            "PARAMETER[\"central_meridian\",2],\n",
            "PARAMETER[\"false_easting\",3],\n",
            "PARAMETER[\"false_northing\",4]"
        )
    );
}

#[test]
fn operation_sch_export() {
    let conv =
        Conversion::create_spherical_cross_track_height(&props(), 1.0, 2.0, 3.0, 4.0).expect("sch");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(
        proj(&conv),
        "+proj=sch +plat_0=1 +plon_0=2 +phdg_0=3 +h_0=4"
    );
    assert_eq!(
        wkt2_simul(&conv),
        concat!(
            "CONVERSION[\"Spherical Cross-Track Height\",\n",
            "    METHOD[\"Spherical Cross-Track Height\"],\n",
            "    PARAMETER[\"Peg point latitude\",1,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
            "    PARAMETER[\"Peg point longitude\",2,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
            "    PARAMETER[\"Peg point heading\",3,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
            "    PARAMETER[\"Peg point height\",4,\n",
            "        LENGTHUNIT[\"metre\",1]]]"
        )
    );
    assert_eq!(
        wkt1(&conv),
        concat!(
            "PROJECTION[\"Spherical_Cross_Track_Height\"],\n",
            "PARAMETER[\"peg_point_latitude\",1],\n",
            "PARAMETER[\"peg_point_longitude\",2],\n",
            "PARAMETER[\"peg_point_heading\",3],\n",
            "PARAMETER[\"peg_point_height\",4]"
        )
    );
}

#[test]
fn operation_eqearth_export() {
    let conv = Conversion::create_equal_earth(&props(), 1.0, 2.0, 3.0).expect("eqearth");
    assert_eq!(proj(&conv), "+proj=eqearth +lon_0=1 +x_0=2 +y_0=3");
    assert_eq!(
        wkt2(&conv),
        concat!(
            "CONVERSION[\"Equal Earth\",\n",
            "    METHOD[\"Equal Earth\",\n",
            "        ID[\"EPSG\",1078]],\n",
            "    PARAMETER[\"Longitude of natural origin\",1,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
            "        ID[\"EPSG\",8802]],\n",
            "    PARAMETER[\"False easting\",2,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8806]],\n",
            "    PARAMETER[\"False northing\",3,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8807]]]"
        )
    );
}

#[test]
fn operation_vertical_perspective_export() {
    let conv = Conversion::create_vertical_perspective(&props(), 1.0, 2.0, 3.0, 4.0, 5.0, 6.0)
        .expect("vp");
    assert_eq!(
        proj(&conv),
        "+proj=nsper +lat_0=1 +lon_0=2 +h=4 +x_0=5 +y_0=6"
    );
    assert_eq!(
        wkt2(&conv),
        concat!(
            "CONVERSION[\"Vertical Perspective\",\n",
            "    METHOD[\"Vertical Perspective\",\n",
            "        ID[\"EPSG\",9838]],\n",
            "    PARAMETER[\"Latitude of topocentric origin\",1,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
            "        ID[\"EPSG\",8834]],\n",
            "    PARAMETER[\"Longitude of topocentric origin\",2,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
            "        ID[\"EPSG\",8835]],\n",
            "    PARAMETER[\"Ellipsoidal height of topocentric origin\",3,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8836]],\n",
            "    PARAMETER[\"Viewpoint height\",4,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8840]],\n",
            "    PARAMETER[\"False easting\",5,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8806]],\n",
            "    PARAMETER[\"False northing\",6,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8807]]]"
        )
    );
}

#[test]
fn operation_hotine_oblique_mercator_variant_a_export_equivalent_modulo_360() {
    let conv1 = Conversion::create_hotine_oblique_mercator_variant_a(
        &props(),
        1.0,
        2.0,
        -3.0,
        -4.0,
        5.0,
        6.0,
        7.0,
    )
    .expect("hom a");
    let conv2 = Conversion::create_hotine_oblique_mercator_variant_a(
        &props(),
        1.0,
        2.0,
        -3.0 + 360.0,
        -4.0 + 360.0,
        5.0,
        6.0,
        7.0,
    )
    .expect("hom a +360");
    assert!(conv1.is_equivalent_to_with(&conv2, Equivalence::Equivalent));
}
