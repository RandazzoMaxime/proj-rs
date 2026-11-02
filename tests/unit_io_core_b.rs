//! PROJ 9.9.0 `test/unit/test_io.cpp`, lot io-core-b: the `io.projparse_*` tail,
//! `io.createFromUserInput*`, `wkt_export.*`, `json_import.*` and related names.
//! Strings and numbers are copied from the C++ source without change.

use proj_rs::iso19111::{create_object_from_json, Equivalence, JsonFormat, JsonObject};
use proj_rs::{
    create_crs_from_proj_string, create_from_user_input, Conversion, CoordinateOperationFactory,
    Crs, DatabaseContext, GeographicCRS, OpProps, ProjectedCRS, WktConvention, WktFormatter,
    WktLines, WktObject, WktParser,
};

fn proj_crs(definition: &str) -> Crs {
    create_crs_from_proj_string(definition).expect("PROJ string")
}

/// `createOperation(EPSG_4326, crs)->exportToPROJString()`.
fn op_from_4326(crs: &Crs) -> String {
    CoordinateOperationFactory::create()
        .create_operation_proj_string(&Crs::Geographic(GeographicCRS::epsg_4326()), crs)
        .expect("operation")
}

fn wkt2_single(crs: &Crs) -> String {
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2015);
    formatter.simul_cur_node_has_id();
    formatter.set_lines(WktLines::Single);
    crs.export_to_wkt(&mut formatter).expect("WKT2")
}

fn parse_crs(wkt: &str) -> Crs {
    match WktParser::new().create_from_wkt(wkt).expect("wkt") {
        WktObject::Crs(crs) => crs,
        _ => panic!("not a CRS"),
    }
}

#[test]
fn io_projparse_longlat_unitconvert_deg() {
    let crs = proj_crs(
        "+type=crs +proj=pipeline +step +proj=longlat +ellps=GRS80 +step \
         +proj=unitconvert +xy_in=rad +xy_out=deg",
    );
    assert!(matches!(crs, Crs::Geographic(_)));
    assert_eq!(op_from_4326(&crs), "+proj=axisswap +order=2,1");
}

#[test]
fn io_projparse_longlat_unitconvert_grad() {
    let crs = proj_crs(
        "+type=crs +proj=pipeline +step +proj=longlat +ellps=GRS80 +step \
         +proj=unitconvert +xy_in=rad +xy_out=grad",
    );
    assert!(matches!(crs, Crs::Geographic(_)));
    assert_eq!(
        op_from_4326(&crs),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step \
         +proj=unitconvert +xy_in=rad +xy_out=grad"
    );
}

#[test]
fn io_projparse_longlat_unitconvert_rad() {
    let crs = proj_crs(
        "+type=crs +proj=pipeline +step +proj=longlat +ellps=GRS80 +step \
         +proj=unitconvert +xy_in=rad +xy_out=rad",
    );
    assert!(matches!(crs, Crs::Geographic(_)));
    assert_eq!(
        op_from_4326(&crs),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad"
    );
}

#[test]
fn io_projparse_longlat_axisswap() {
    for order1 in [1_i32, -1, 2, -2] {
        for order2 in [1_i32, -1, 2, -2] {
            if (order1 * order2).abs() == 2 && !(order1 == 1 && order2 == 2) {
                let text = format!(
                    "+type=crs +proj=pipeline +step +proj=longlat +ellps=GRS80 \
                     +step +proj=axisswap +order={order1},{order2}"
                );
                let crs = proj_crs(&text);
                assert!(matches!(crs, Crs::Geographic(_)), "{text}");
                let expected = if order1 == 2 && order2 == 1 {
                    "+proj=noop".to_owned()
                } else if order1 == 2 && order2 == -1 {
                    "+proj=axisswap +order=1,-2".to_owned()
                } else {
                    format!(
                        "+proj=pipeline +step +proj=axisswap +order=2,1 \
                         +step +proj=axisswap +order={order1},{order2}"
                    )
                };
                assert_eq!(op_from_4326(&crs), expected, "{text}");
            }
        }
    }
}

#[test]
fn io_projparse_tmerc_south_oriented() {
    let crs = proj_crs("+proj=tmerc +axis=wsu +x_0=1 +lat_0=1 +k_0=2 +type=crs");
    let Crs::Projected(projected) = &crs else {
        panic!("projected");
    };
    let mut f = WktFormatter::create(WktConvention::Wkt2_2015);
    f.simul_cur_node_has_id();
    let expected = "PROJCRS[\"unknown\",\n\
        \x20   BASEGEODCRS[\"unknown\",\n\
        \x20       DATUM[\"World Geodetic System 1984\",\n\
        \x20           ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n\
        \x20               LENGTHUNIT[\"metre\",1]]],\n\
        \x20       PRIMEM[\"Greenwich\",0,\n\
        \x20           ANGLEUNIT[\"degree\",0.0174532925199433]]],\n\
        \x20   CONVERSION[\"unknown\",\n\
        \x20       METHOD[\"Transverse Mercator (South Orientated)\",\n\
        \x20           ID[\"EPSG\",9808]],\n\
        \x20       PARAMETER[\"Latitude of natural origin\",1,\n\
        \x20           ANGLEUNIT[\"degree\",0.0174532925199433],\n\
        \x20           ID[\"EPSG\",8801]],\n\
        \x20       PARAMETER[\"Longitude of natural origin\",0,\n\
        \x20           ANGLEUNIT[\"degree\",0.0174532925199433],\n\
        \x20           ID[\"EPSG\",8802]],\n\
        \x20       PARAMETER[\"Scale factor at natural origin\",2,\n\
        \x20           SCALEUNIT[\"unity\",1],\n\
        \x20           ID[\"EPSG\",8805]],\n\
        \x20       PARAMETER[\"False easting\",1,\n\
        \x20           LENGTHUNIT[\"metre\",1],\n\
        \x20           ID[\"EPSG\",8806]],\n\
        \x20       PARAMETER[\"False northing\",0,\n\
        \x20           LENGTHUNIT[\"metre\",1],\n\
        \x20           ID[\"EPSG\",8807]]],\n\
        \x20   CS[Cartesian,2],\n\
        \x20       AXIS[\"westing\",west,\n\
        \x20           ORDER[1],\n\
        \x20           LENGTHUNIT[\"metre\",1]],\n\
        \x20       AXIS[\"southing\",south,\n\
        \x20           ORDER[2],\n\
        \x20           LENGTHUNIT[\"metre\",1]]]";
    assert_eq!(crs.export_to_wkt(&mut f).expect("WKT2"), expected);
    assert_eq!(
        projected.deriving_conversion().method().name(),
        "Transverse Mercator (South Orientated)"
    );

    let crs = proj_crs(
        "+type=crs +proj=pipeline +step +proj=tmerc +x_0=1 +lat_0=1 +k_0=2 \
         +step +proj=axisswap +order=-1,-2",
    );
    let Crs::Projected(projected) = &crs else {
        panic!("projected");
    };
    assert_eq!(
        projected.deriving_conversion().method().name(),
        "Transverse Mercator (South Orientated)"
    );
}

fn check_czech_krovak(definition: &str, proj: &str, method: &str) {
    let crs = proj_crs(definition);
    assert!(matches!(crs, Crs::Projected(_)));
    assert_eq!(crs.export_to_proj_string().expect("PROJ string"), proj);
    let wkt = wkt2_single(&crs);
    assert!(wkt.contains(method), "{wkt}");
    assert!(wkt.contains(",AXIS[\"westing\",west,ORDER[1]"), "{wkt}");
    assert!(wkt.contains(",AXIS[\"southing\",south,ORDER[2]"), "{wkt}");
}

#[test]
fn io_projparse_krovak_czech() {
    check_czech_krovak(
        "+proj=krovak +czech +type=crs",
        "+proj=krovak +czech +lat_0=49.5 +lon_0=24.8333333333333 \
         +alpha=30.2881397527778 +k=0.9999 +x_0=0 +y_0=0 \
         +ellps=bessel +units=m +no_defs +type=crs",
        "METHOD[\"Krovak\",ID[\"EPSG\",9819]]",
    );
}

#[test]
fn io_projparse_krovak_modified_czech() {
    check_czech_krovak(
        "+proj=mod_krovak +czech +x_0=5000000 +y_0=5000000 +type=crs",
        "+proj=mod_krovak +czech +lat_0=49.5 +lon_0=24.8333333333333 \
         +alpha=30.2881397527778 +k=0.9999 +x_0=5000000 +y_0=5000000 \
         +ellps=bessel +units=m +no_defs +type=crs",
        "METHOD[\"Krovak Modified\",ID[\"EPSG\",1042]]",
    );
}

#[test]
fn io_projparse_tmerc_approx() {
    let crs = proj_crs("+proj=tmerc +approx +type=crs");
    assert!(matches!(crs, Crs::Projected(_)));
    let wkt2 = wkt2_single(&crs);
    assert!(
        wkt2.contains("METHOD[\"Transverse Mercator\",ID[\"EPSG\",9807]]"),
        "{wkt2}"
    );
    assert_eq!(
        crs.export_to_proj_string().expect("PROJ string"),
        "+proj=tmerc +approx +lat_0=0 +lon_0=0 +k=1 +x_0=0 +y_0=0 \
         +datum=WGS84 +units=m +no_defs +type=crs"
    );
    let wkt1 = crs
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
        .expect("WKT1");
    assert!(
        wkt1.contains(
            "EXTENSION[\"PROJ4\",\"+proj=tmerc +approx +lat_0=0 \
             +lon_0=0 +k=1 +x_0=0 +y_0=0 +datum=WGS84 +units=m \
             +no_defs\"]"
        ),
        "{wkt1}"
    );
}

#[test]
fn io_projparse_peirce_q_horizontal() {
    let input = "+proj=peirce_q +shape=horizontal +datum=WGS84 +units=m +no_defs +type=crs";
    let crs = proj_crs(input);
    assert!(matches!(crs, Crs::Projected(_)));
    assert_eq!(crs.export_to_proj_string().expect("PROJ string"), input);
}

fn projected_op_from_4326(input: &str, expected: &str) {
    let crs = proj_crs(input);
    assert!(matches!(crs, Crs::Projected(_)));
    assert_eq!(op_from_4326(&crs), expected);
}

#[test]
fn io_projparse_axisswap_unitconvert_longlat_proj() {
    projected_op_from_4326(
        "+type=crs +proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=grad +xy_out=rad +step +inv +proj=longlat \
         +ellps=clrk80ign +pm=paris +step +proj=lcc +lat_1=49.5 \
         +lat_0=49.5 +lon_0=0 +k_0=0.999877341 +x_0=600000 +y_0=200000 \
         +ellps=clrk80ign +pm=paris",
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=lcc \
         +lat_1=49.5 +lat_0=49.5 +lon_0=0 +k_0=0.999877341 +x_0=600000 \
         +y_0=200000 +ellps=clrk80ign +pm=paris",
    );
}

#[test]
fn io_projparse_axisswap_unitconvert_proj_axisswap() {
    let input = "+type=crs +proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=igh \
         +lon_0=0 +x_0=0 +y_0=0 +ellps=GRS80 +step +proj=axisswap +order=2,1";
    projected_op_from_4326(
        input,
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=igh \
         +lon_0=0 +x_0=0 +y_0=0 +ellps=GRS80 +step +proj=axisswap \
         +order=2,1",
    );
}

#[test]
fn io_projparse_axisswap_unitconvert_proj_unitconvert() {
    projected_op_from_4326(
        "+type=crs +proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=igh \
         +lon_0=0 +x_0=0 +y_0=0 +ellps=GRS80 +step +proj=unitconvert +xy_in=m \
         +xy_out=ft",
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=igh \
         +lon_0=0 +x_0=0 +y_0=0 +ellps=GRS80 +step +proj=unitconvert \
         +xy_in=m +xy_out=ft",
    );
}

#[test]
fn io_projparse_axisswap_unitconvert_proj_unitconvert_numeric_axisswap() {
    projected_op_from_4326(
        "+type=crs +proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=igh \
         +lon_0=0 +x_0=0 +y_0=0 +ellps=GRS80 +step +proj=unitconvert +xy_in=m \
         +xy_out=2.5 +step +proj=axisswap +order=-2,-1",
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=igh \
         +lon_0=0 +x_0=0 +y_0=0 +ellps=GRS80 +step +proj=unitconvert \
         +xy_in=m +xy_out=2.5 +step +proj=axisswap \
         +order=-2,-1",
    );
}

#[test]
fn io_projparse_cart_unit() {
    let crs = proj_crs(
        "+proj=pipeline +step +proj=cart +ellps=WGS84 +step \
         +proj=unitconvert +xy_in=m +z_in=m +xy_out=km +z_out=km",
    );
    assert!(matches!(crs, Crs::Geodetic(_)));
    assert_eq!(
        op_from_4326(&crs),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m \
         +step +proj=cart +ellps=WGS84 \
         +step +proj=unitconvert +xy_in=m +z_in=m +xy_out=km +z_out=km"
    );
}

#[test]
fn io_projparse_cart_unit_numeric() {
    let crs = proj_crs(
        "+proj=pipeline +step +proj=cart +ellps=WGS84 +step \
         +proj=unitconvert +xy_in=m +z_in=m +xy_out=500 +z_out=500",
    );
    assert!(matches!(crs, Crs::Geodetic(_)));
    assert_eq!(
        op_from_4326(&crs),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m \
         +step +proj=cart +ellps=WGS84 \
         +step +proj=unitconvert +xy_in=m +z_in=m +xy_out=500 +z_out=500"
    );
}

#[test]
fn io_projparse_ob_tran_rhealpix() {
    let input = "+proj=ob_tran +o_proj=rhealpix +o_lat_p=90 +o_lon_p=-180 +lon_0=180 \
         +north_square=1 +south_square=0 +ellps=WGS84 +type=crs";
    let crs = proj_crs(input);
    assert!(matches!(crs, Crs::Projected(_)));
    assert_eq!(crs.export_to_proj_string().expect("PROJ string"), input);
}

#[test]
fn io_projparse_ob_tran_longlat() {
    for o_proj in ["longlat", "lonlat", "latlong", "latlon"] {
        let input = format!(
            "+type=crs +proj=pipeline +step +proj=axisswap +order=2,1 +step \
             +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=ob_tran \
             +o_proj={o_proj} +o_lat_p=52 +o_lon_p=-30 +lon_0=-25 +ellps=WGS84 \
             +step +proj=axisswap +order=2,1"
        );
        let crs = proj_crs(&input);
        assert_eq!(
            op_from_4326(&crs),
            format!(
                "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
                 +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=ob_tran \
                 +o_proj={o_proj} +o_lat_p=52 +o_lon_p=-30 +lon_0=-25 \
                 +ellps=WGS84 +step +proj=unitconvert +xy_in=rad +xy_out=deg \
                 +step +proj=axisswap +order=2,1"
            )
        );
    }
}

fn projected(crs: Crs) -> ProjectedCRS {
    match crs {
        Crs::Projected(projected) => projected,
        _ => panic!("projected CRS"),
    }
}

fn method_code(crs: &ProjectedCRS) -> String {
    crs.deriving_conversion().method().code().to_owned()
}

#[test]
fn io_projparse_laea_spherical() {
    let crs = projected(proj_crs("+proj=laea +R=6371228 +type=crs"));
    assert_eq!(method_code(&crs), "1027");
    let conversion =
        Conversion::create_lambert_azimuthal_equal_area(&OpProps::new(), 0.0, 0.0, 0.0, 0.0)
            .expect("conversion");
    let crs2 = ProjectedCRS::create(
        "unknown",
        crs.base_crs().clone(),
        conversion,
        crs.coordinate_system().clone(),
    );
    assert_eq!(method_code(&crs2), "9820");
    assert!(crs.is_equivalent_to_with(&crs2, Equivalence::Equivalent));
    assert!(crs2.is_equivalent_to_with(&crs, Equivalence::Equivalent));
}

#[test]
fn io_projparse_eqc_spherical() {
    let crs = projected(proj_crs("+proj=eqc +R=6371228 +type=crs"));
    assert_eq!(method_code(&crs), "1029");
    let conversion =
        Conversion::create_equidistant_cylindrical(&OpProps::new(), 0.0, 0.0, 0.0, 0.0)
            .expect("conversion");
    let crs2 = ProjectedCRS::create(
        "unknown",
        crs.base_crs().clone(),
        conversion,
        crs.coordinate_system().clone(),
    );
    assert_eq!(method_code(&crs2), "1028");
    assert!(crs.is_equivalent_to_with(&crs2, Equivalence::Equivalent));
    assert!(crs2.is_equivalent_to_with(&crs, Equivalence::Equivalent));
}

#[test]
fn io_createfromuserinput_hack_epsg_102100() {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = create_from_user_input("EPSG:102100", &db).expect("crs");
    let ids = crs.identifiers();
    assert_eq!(ids.len(), 1);
    // we do not lie on the real authority
    assert_eq!(ids[0].code_space(), "ESRI");
    assert_eq!(ids[0].code(), "102100");
}

#[test]
fn wkt_export_precision() {
    let wkt = "PROJCS[\"RGF93 / Lambert-93\",\n\
        \x20   GEOGCS[\"RGF93\",\n\
        \x20       DATUM[\"Reseau_Geodesique_Francais_1993\",\n\
        \x20           SPHEROID[\"GRS 80\",6378137,298.257222101],\n\
        \x20           AUTHORITY[\"EPSG\",\"6171\"]],\n\
        \x20       PRIMEM[\"Greenwich\",0],\n\
        \x20       UNIT[\"degree\",0.0174532925199433]],\n\
        \x20   PROJECTION[\"Lambert_Conformal_Conic_2SP\"],\n\
        \x20   PARAMETER[\"standard_parallel_1\",49.00000000001],\n\
        \x20   PARAMETER[\"standard_parallel_2\",44],\n\
        \x20   PARAMETER[\"latitude_of_origin\",46.5],\n\
        \x20   PARAMETER[\"central_meridian\",3],\n\
        \x20   PARAMETER[\"false_easting\",700000],\n\
        \x20   PARAMETER[\"false_northing\",6600000],\n\
        \x20   UNIT[\"Meter\",1],\n\
        \x20   AXIS[\"Easting\",EAST],\n\
        \x20   AXIS[\"Northing\",NORTH]]";
    let crs = parse_crs(wkt);
    assert!(matches!(crs, Crs::Projected(_)));
    let out = crs
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
        .expect("WKT1");
    assert_eq!(out, wkt);
}

fn invalid_unit_wkt(angular: &str, linear: &str) -> String {
    format!(
        "PROJCS[\"WGS 84 / UTM zone 31N\",\n\
        \x20   GEOGCS[\"WGS 84\",\n\
        \x20       DATUM[\"WGS_1984\",\n\
        \x20           SPHEROID[\"WGS 84\",6378137,298.257223563]],\n\
        \x20       PRIMEM[\"Greenwich\",0],\n\
        \x20       UNIT[{angular}]],\n\
        \x20   PROJECTION[\"Transverse_Mercator\"],\n\
        \x20   PARAMETER[\"latitude_of_origin\",0],\n\
        \x20   PARAMETER[\"central_meridian\",3],\n\
        \x20   PARAMETER[\"scale_factor\",0.9996],\n\
        \x20   PARAMETER[\"false_easting\",500000],\n\
        \x20   PARAMETER[\"false_northing\",0],\n\
        \x20   UNIT[{linear}]]"
    )
}

#[test]
fn wkt_export_invalid_linear_unit() {
    let crs = parse_crs(&invalid_unit_wkt(
        "\"degree\",0.0174532925199433",
        "\"foo\",0",
    ));
    assert!(matches!(crs, Crs::Projected(_)));
    assert!(crs
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
        .is_err());
}

#[test]
fn wkt_export_invalid_angular_unit() {
    let crs = parse_crs(&invalid_unit_wkt("\"foo\",0", "\"meter\",1"));
    assert!(matches!(crs, Crs::Projected(_)));
    assert!(crs
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
        .is_err());
}

fn import_json(json: &str) -> JsonObject {
    create_object_from_json(json, None).expect("PROJJSON")
}

fn export_foo(object: &JsonObject) -> String {
    let db = DatabaseContext::create().expect("proj.db");
    object
        .export_to_json_with(&db, &JsonFormat::default().with_schema("foo"))
        .expect("PROJJSON export")
}

fn refused(json: &str) {
    assert!(create_object_from_json(json, None).is_err(), "{json}");
}

/// Import `json` and write it back: the PROJJSON must be unchanged.
fn round_trip(json: &str) -> JsonObject {
    let object = import_json(json);
    assert_eq!(export_foo(&object), json);
    object
}

#[test]
fn json_import_ellipsoid_flattened_sphere() {
    let json = r#"{
  "$schema": "foo",
  "type": "Ellipsoid",
  "name": "WGS 84",
  "semi_major_axis": 6378137,
  "inverse_flattening": 298.257223563,
  "id": {
    "authority": "EPSG",
    "code": 7030
  }
}"#;
    let obj = round_trip(json);
    assert!(matches!(obj, JsonObject::Ellipsoid(_)));
}

#[test]
fn json_import_ellipsoid_major_minor_custom_unit() {
    let json = r#"{
  "$schema": "foo",
  "type": "Ellipsoid",
  "name": "foo",
  "semi_major_axis": 6378137,
  "semi_minor_axis": {
    "value": 6370000,
    "unit": {
      "type": "LinearUnit",
      "name": "my_unit",
      "conversion_factor": 2
    }
  }
}"#;
    let obj = round_trip(json);
    assert!(matches!(obj, JsonObject::Ellipsoid(_)));
}

#[test]
fn json_import_ellipsoid_sphere() {
    let json = r#"{
  "$schema": "foo",
  "type": "Ellipsoid",
  "name": "Sphere",
  "radius": 6371000,
  "id": {
    "authority": "EPSG",
    "code": 7035
  }
}"#;
    let obj = round_trip(json);
    assert!(matches!(obj, JsonObject::Ellipsoid(_)));
}

#[test]
fn json_import_axis_with_meridian() {
    let json = r#"{
  "$schema": "foo",
  "type": "Axis",
  "name": "Northing",
  "abbreviation": "N",
  "direction": "south",
  "meridian": {
    "longitude": 180
  },
  "unit": "metre"
}"#;
    let obj = round_trip(json);
    assert!(matches!(obj, JsonObject::Axis(_)));
}

#[test]
fn json_import_axis_with_meridian_with_unit() {
    let json = r#"{
  "$schema": "foo",
  "type": "Axis",
  "name": "Northing",
  "abbreviation": "N",
  "direction": "south",
  "meridian": {
    "longitude": {
      "value": 200,
      "unit": {
        "type": "AngularUnit",
        "name": "grad",
        "conversion_factor": 0.0157079632679489
      }
    }
  },
  "unit": "metre"
}"#;
    let obj = round_trip(json);
    assert!(matches!(obj, JsonObject::Axis(_)));
}

#[test]
fn json_import_axis_with_minimum_value_maximum_value_range_meaning() {
    let json = r#"{
  "$schema": "foo",
  "type": "Axis",
  "name": "Longitude",
  "abbreviation": "lon",
  "direction": "east",
  "unit": "degree",
  "minimum_value": 0,
  "maximum_value": 360,
  "range_meaning": "wraparound"
}"#;
    let obj = round_trip(json);
    assert!(matches!(obj, JsonObject::Axis(_)));
}

#[test]
fn json_import_prime_meridian() {
    let json = r#"{
  "$schema": "foo",
  "type": "PrimeMeridian",
  "name": "Paris",
  "longitude": {
    "value": 2.5969213,
    "unit": {
      "type": "AngularUnit",
      "name": "grad",
      "conversion_factor": 0.0157079632679489
    }
  }
}"#;
    let obj = round_trip(json);
    assert!(matches!(obj, JsonObject::PrimeMeridian(_)));
}

#[test]
fn json_import_geodetic_reference_frame_with_implicit_prime_meridian() {
    let json = r#"{
  "$schema": "foo",
  "type": "GeodeticReferenceFrame",
  "name": "World Geodetic System 1984",
  "ellipsoid": {
    "name": "WGS 84",
    "semi_major_axis": 6378137,
    "inverse_flattening": 298.257223563
  }
}"#;
    let obj = round_trip(json);
    assert!(matches!(obj, JsonObject::GeodeticReferenceFrame(_)));
}

#[test]
fn json_import_geodetic_reference_frame_with_explicit_prime_meridian() {
    let json = r#"{
  "$schema": "foo",
  "type": "GeodeticReferenceFrame",
  "name": "Nouvelle Triangulation Francaise (Paris)",
  "ellipsoid": {
    "name": "Clarke 1880 (IGN)",
    "semi_major_axis": 6378249.2,
    "semi_minor_axis": 6356515
  },
  "prime_meridian": {
    "name": "Paris",
    "longitude": {
      "value": 2.5969213,
      "unit": {
        "type": "AngularUnit",
        "name": "grad",
        "conversion_factor": 0.0157079632679489
      }
    }
  }
}"#;
    let obj = round_trip(json);
    assert!(matches!(obj, JsonObject::GeodeticReferenceFrame(_)));
}

#[test]
fn json_import_geodetic_reference_frame_with_anchor_epoch() {
    let json = r#"{
  "$schema": "foo",
  "type": "GeodeticReferenceFrame",
  "name": "my_name",
  "anchor_epoch": 0,
  "ellipsoid": {
    "name": "WGS 84",
    "semi_major_axis": 6378137,
    "inverse_flattening": 298.257223563
  }
}"#;
    let obj = round_trip(json);
    assert!(matches!(obj, JsonObject::GeodeticReferenceFrame(_)));
}

#[test]
fn json_import_vertical_extent() {
    let json = r#"{
  "$schema": "foo",
  "type": "GeodeticReferenceFrame",
  "name": "World Geodetic System 1984",
  "ellipsoid": {
    "name": "WGS 84",
    "semi_major_axis": 6378137,
    "inverse_flattening": 298.257223563
  },
  "vertical_extent": {
    "minimum": -1000,
    "maximum": 0
  }
}"#;
    let obj = round_trip(json);
    assert!(matches!(obj, JsonObject::GeodeticReferenceFrame(_)));
}

#[test]
fn json_import_vertical_extent_with_unit() {
    let json = r#"{
  "$schema": "foo",
  "type": "GeodeticReferenceFrame",
  "name": "World Geodetic System 1984",
  "ellipsoid": {
    "name": "WGS 84",
    "semi_major_axis": 6378137,
    "inverse_flattening": 298.257223563
  },
  "vertical_extent": {
    "minimum": -1000,
    "maximum": 0,
    "unit": {
      "type": "LinearUnit",
      "name": "my_metre",
      "conversion_factor": 1
    }
  }
}"#;
    let obj = round_trip(json);
    assert!(matches!(obj, JsonObject::GeodeticReferenceFrame(_)));
}

#[test]
fn json_import_dynamic_vertical_reference_frame() {
    let json = r#"{
  "$schema": "foo",
  "type": "DynamicVerticalReferenceFrame",
  "name": "bar",
  "frame_reference_epoch": 1
}"#;
    let obj = round_trip(json);
    assert!(matches!(obj, JsonObject::VerticalReferenceFrame(_)));
}

#[test]
fn json_import_geographic_crs() {
    let json = r#"{
  "$schema": "foo",
  "type": "GeographicCRS",
  "name": "WGS 84",
  "datum": {
    "type": "GeodeticReferenceFrame",
    "name": "World Geodetic System 1984",
    "ellipsoid": {
      "name": "WGS 84",
      "semi_major_axis": 6378137,
      "inverse_flattening": 298.257223563
    }
  },
  "coordinate_system": {
    "subtype": "ellipsoidal",
    "axis": [
      {
        "name": "Geodetic latitude",
        "abbreviation": "Lat",
        "direction": "north",
        "unit": "degree"
      },
      {
        "name": "Geodetic longitude",
        "abbreviation": "Lon",
        "direction": "east",
        "unit": "degree"
      }
    ]
  },
  "area": "World",
  "bbox": {
    "south_latitude": -90,
    "west_longitude": -180,
    "north_latitude": 90,
    "east_longitude": 180
  },
  "id": {
    "authority": "EPSG",
    "code": 4326
  },
  "remarks": "my_remarks"
}"#;
    let obj = round_trip(json);
    assert!(matches!(obj, JsonObject::Crs(Crs::Geographic(_))));
}

#[test]
fn json_import_geographic_crs_with_deformation_models() {
    let json = r#"{
  "$schema": "foo",
  "type": "GeographicCRS",
  "name": "test",
  "datum": {
    "type": "DynamicGeodeticReferenceFrame",
    "name": "test",
    "frame_reference_epoch": 2005,
    "ellipsoid": {
      "name": "WGS 84",
      "semi_major_axis": 6378137,
      "inverse_flattening": 298.257223563
    }
  },
  "coordinate_system": {
    "subtype": "ellipsoidal",
    "axis": [
      {
        "name": "Geodetic latitude",
        "abbreviation": "Lat",
        "direction": "north",
        "unit": "degree"
      },
      {
        "name": "Geodetic longitude",
        "abbreviation": "Lon",
        "direction": "east",
        "unit": "degree"
      }
    ]
  },
  "deformation_models": [
    {
      "name": "my_model"
    }
  ]
}"#;
    let obj = round_trip(json);
    assert!(matches!(obj, JsonObject::Crs(Crs::Geographic(_))));
}

#[test]
fn json_import_geocentric_crs() {
    let json = r#"{
  "$schema": "foo",
  "type": "GeodeticCRS",
  "name": "WGS 84",
  "datum": {
    "type": "GeodeticReferenceFrame",
    "name": "World Geodetic System 1984",
    "ellipsoid": {
      "name": "WGS 84",
      "semi_major_axis": 6378137,
      "inverse_flattening": 298.257223563
    }
  },
  "coordinate_system": {
    "subtype": "Cartesian",
    "axis": [
      {
        "name": "Geocentric X",
        "abbreviation": "X",
        "direction": "geocentricX",
        "unit": "metre"
      },
      {
        "name": "Geocentric Y",
        "abbreviation": "Y",
        "direction": "geocentricY",
        "unit": "metre"
      },
      {
        "name": "Geocentric Z",
        "abbreviation": "Z",
        "direction": "geocentricZ",
        "unit": "metre"
      }
    ]
  }
}"#;
    let obj = round_trip(json);
    assert!(matches!(obj, JsonObject::Crs(Crs::Geodetic(_))));
}

#[test]
fn json_import_compound_crs() {
    let json = r#"{
  "$schema": "foo",
  "type": "CompoundCRS",
  "name": "WGS 84 + EGM2008 height",
  "components": [
    {
      "type": "GeographicCRS",
      "name": "WGS 84",
      "datum": {
        "type": "GeodeticReferenceFrame",
        "name": "World Geodetic System 1984",
        "ellipsoid": {
          "name": "WGS 84",
          "semi_major_axis": 6378137,
          "inverse_flattening": 298.257223563
        }
      },
      "coordinate_system": {
        "subtype": "ellipsoidal",
        "axis": [
          {
            "name": "Geodetic latitude",
            "abbreviation": "Lat",
            "direction": "north",
            "unit": "degree"
          },
          {
            "name": "Geodetic longitude",
            "abbreviation": "Lon",
            "direction": "east",
            "unit": "degree"
          }
        ]
      },
      "id": {
        "authority": "EPSG",
        "code": 4326
      }
    },
    {
      "type": "VerticalCRS",
      "name": "EGM2008 height",
      "datum": {
        "type": "VerticalReferenceFrame",
        "name": "EGM2008 geoid"
      },
      "coordinate_system": {
        "subtype": "vertical",
        "axis": [
          {
            "name": "Gravity-related height",
            "abbreviation": "H",
            "direction": "up",
            "unit": "metre"
          }
        ]
      },
      "id": {
        "authority": "EPSG",
        "code": 3855
      }
    }
  ]
}"#;
    let obj = round_trip(json);
    assert!(matches!(obj, JsonObject::Crs(Crs::Compound(_))));
}

#[test]
fn json_import_vertical_crs_with_datum_ensemble() {
    let json = r#"{
  "$schema": "foo",
  "type": "VerticalCRS",
  "name": "foo",
  "datum_ensemble": {
    "name": "ensemble",
    "members": [
      {
        "name": "member1"
      },
      {
        "name": "member2"
      }
    ],
    "accuracy": "2"
  },
  "coordinate_system": {
    "subtype": "vertical",
    "axis": [
      {
        "name": "Gravity-related height",
        "abbreviation": "H",
        "direction": "up",
        "unit": "metre"
      }
    ]
  }
}"#;
    let obj = round_trip(json);
    assert!(matches!(obj, JsonObject::Crs(Crs::Vertical(_))));
}

#[test]
fn json_import_vertical_crs_with_geoid_model() {
    let json = r#"{
  "$schema": "foo",
  "type": "VerticalCRS",
  "name": "CGVD2013",
  "datum": {
    "type": "VerticalReferenceFrame",
    "name": "Canadian Geodetic Vertical Datum of 2013"
  },
  "coordinate_system": {
    "subtype": "vertical",
    "axis": [
      {
        "name": "Gravity-related height",
        "abbreviation": "H",
        "direction": "up",
        "unit": "metre"
      }
    ]
  },
  "geoid_model": {
    "name": "CGG2013",
    "id": {
      "authority": "EPSG",
      "code": 6648
    }
  }
}"#;
    let obj = round_trip(json);
    assert!(matches!(obj, JsonObject::Crs(Crs::Vertical(_))));
}

#[test]
fn json_import_vertical_crs_with_geoid_models() {
    let json = r#"{
  "$schema": "foo",
  "type": "VerticalCRS",
  "name": "CGVD2013",
  "datum": {
    "type": "VerticalReferenceFrame",
    "name": "Canadian Geodetic Vertical Datum of 2013"
  },
  "coordinate_system": {
    "subtype": "vertical",
    "axis": [
      {
        "name": "Gravity-related height",
        "abbreviation": "H",
        "direction": "up",
        "unit": "metre"
      }
    ]
  },
  "geoid_models": [
    {
      "name": "CGG2013",
      "id": {
        "authority": "EPSG",
        "code": 6648
      }
    },
    {
      "name": "other"
    }
  ]
}"#;
    let obj = round_trip(json);
    assert!(matches!(obj, JsonObject::Crs(Crs::Vertical(_))));
}

#[test]
fn json_import_vertical_crs_with_deformation_models() {
    let json = r#"{
  "$schema": "foo",
  "type": "VerticalCRS",
  "name": "test",
  "datum": {
    "type": "DynamicVerticalReferenceFrame",
    "name": "test",
    "frame_reference_epoch": 2005
  },
  "coordinate_system": {
    "subtype": "vertical",
    "axis": [
      {
        "name": "Gravity-related height",
        "abbreviation": "H",
        "direction": "up",
        "unit": "metre"
      }
    ]
  },
  "deformation_models": [
    {
      "name": "my_model"
    }
  ]
}"#;
    let obj = round_trip(json);
    assert!(matches!(obj, JsonObject::Crs(Crs::Vertical(_))));
}

#[test]
fn json_import_vertical_crs_with_geoid_model_and_interpolation_crs() {
    let json = r#"{
  "$schema": "foo",
  "type": "VerticalCRS",
  "name": "foo",
  "datum": {
    "type": "VerticalReferenceFrame",
    "name": "bar"
  },
  "coordinate_system": {
    "subtype": "vertical",
    "axis": [
      {
        "name": "Gravity-related height",
        "abbreviation": "H",
        "direction": "up",
        "unit": "metre"
      }
    ]
  },
  "geoid_model": {
    "name": "baz",
    "interpolation_crs": {
      "type": "GeographicCRS",
      "name": "NAD83(2011)",
      "datum": {
        "type": "GeodeticReferenceFrame",
        "name": "NAD83 (National Spatial Reference System 2011)",
        "ellipsoid": {
          "name": "GRS 1980",
          "semi_major_axis": 6378137,
          "inverse_flattening": 298.257222101
        }
      },
      "coordinate_system": {
        "subtype": "ellipsoidal",
        "axis": [
          {
            "name": "Geodetic latitude",
            "abbreviation": "Lat",
            "direction": "north",
            "unit": "degree"
          },
          {
            "name": "Geodetic longitude",
            "abbreviation": "Lon",
            "direction": "east",
            "unit": "degree"
          },
          {
            "name": "Ellipsoidal height",
            "abbreviation": "h",
            "direction": "up",
            "unit": "metre"
          }
        ]
      },
      "id": {
        "authority": "EPSG",
        "code": 6319
      }
    }
  }
}"#;
    let obj = round_trip(json);
    assert!(matches!(obj, JsonObject::Crs(Crs::Vertical(_))));
}

#[test]
fn json_import_vertical_reference_frame_with_anchor_epoch() {
    let json = r#"{
  "$schema": "foo",
  "type": "VerticalReferenceFrame",
  "name": "my_name",
  "anchor": "my_anchor_definition",
  "anchor_epoch": 0
}"#;
    let obj = round_trip(json);
    assert!(matches!(obj, JsonObject::VerticalReferenceFrame(_)));
}

#[test]
fn json_import_id() {
    let json = r#"{
  "$schema": "foo",
  "type": "Ellipsoid",
  "name": "WGS 84",
  "semi_major_axis": 6378137,
  "inverse_flattening": 298.257223563,
  "id": {
    "authority": "EPSG",
    "code": 6326,
    "version": 1,
    "authority_citation": "my citation",
    "uri": "my uri"
  }
}"#;
    let obj = round_trip(json);
    assert!(matches!(obj, JsonObject::Ellipsoid(_)));
}

#[test]
fn json_import_id_code_string_version_string() {
    let json = r#"{
  "$schema": "foo",
  "type": "Ellipsoid",
  "name": "WGS 84",
  "semi_major_axis": 6378137,
  "inverse_flattening": 298.257223563,
  "id": {
    "authority": "EPSG",
    "code": "abc",
    "version": "def",
    "authority_citation": "my citation",
    "uri": "my uri"
  }
}"#;
    let obj = round_trip(json);
    assert!(matches!(obj, JsonObject::Ellipsoid(_)));
}

#[test]
fn json_import_id_code_string_version_double() {
    let json = r#"{
  "$schema": "foo",
  "type": "Ellipsoid",
  "name": "WGS 84",
  "semi_major_axis": 6378137,
  "inverse_flattening": 298.257223563,
  "id": {
    "authority": "EPSG",
    "code": "abc",
    "version": 9.8,
    "authority_citation": "my citation",
    "uri": "my uri"
  }
}"#;
    let obj = round_trip(json);
    assert!(matches!(obj, JsonObject::Ellipsoid(_)));
}

#[test]
fn json_import_ellipsoid_errors() {
    refused(r#"{"#);
    refused(r#"{}"#);
    refused(r#"{ "type": "Ellipsoid" }"#);
    refused(r#"{ "type": "Ellipsoid", "name": "foo" }"#);
    refused(r#"{ "type": "Ellipsoid", "name": "foo", "radius": null }"#);
    refused(r#"{ "type": "Ellipsoid", "name": "foo", "semi_major_axis": 1 }"#);
}

#[test]
fn json_import_axis_with_invalid_minimum_value() {
    refused(
        r#"{
  "$schema": "foo",
  "type": "Axis",
  "name": "Longitude",
  "abbreviation": "lon",
  "direction": "east",
  "unit": "degree",
  "minimum_value": "invalid"
}"#,
    );
}

#[test]
fn json_import_axis_with_invalid_maximum_value() {
    refused(
        r#"{
  "$schema": "foo",
  "type": "Axis",
  "name": "Longitude",
  "abbreviation": "lon",
  "direction": "east",
  "unit": "degree",
  "maximum_value": "invalid"
}"#,
    );
}

#[test]
fn json_import_axis_with_invalid_range_meaning_str() {
    refused(
        r#"{
  "$schema": "foo",
  "type": "Axis",
  "name": "Longitude",
  "abbreviation": "lon",
  "direction": "east",
  "unit": "degree",
  "range_meaning": "invalid"
}"#,
    );
}

#[test]
fn json_import_axis_with_invalid_range_meaning_number() {
    refused(
        r#"{
  "$schema": "foo",
  "type": "Axis",
  "name": "Longitude",
  "abbreviation": "lon",
  "direction": "east",
  "unit": "degree",
  "range_meaning": 1
}"#,
    );
}

#[test]
fn json_import_prime_meridian_errors() {
    refused(r#"{ "type": "PrimeMeridian", "name": "foo" }"#);
    refused(r#"{ "type": "PrimeMeridian", "name": "foo", "longitude": null }"#);
}

#[test]
fn json_import_geodetic_reference_frame_with_invalid_anchor_epoch() {
    refused(
        r#"{
  "$schema": "foo",
  "type": "GeodeticReferenceFrame",
  "name": "my_name",
  "anchor_epoch": "invalid",
  "ellipsoid": {
    "name": "WGS 84",
    "semi_major_axis": 6378137,
    "inverse_flattening": 298.257223563
  }
}"#,
    );
}

#[test]
fn json_import_geodetic_reference_frame_errors() {
    refused(r#"{ "type": "GeodeticReferenceFrame", "name": "foo" }"#);
}

#[test]
fn json_import_geographic_crs_errors() {
    refused(r#"{ "type": "GeographicCRS", "name": "foo" }"#);
    refused(
        r#"{
  "type": "GeographicCRS",
  "name": "WGS 84",
  "datum": {
    "type": "GeodeticReferenceFrame",
    "name": "World Geodetic System 1984",
    "ellipsoid": {
      "name": "WGS 84",
      "semi_major_axis": 6378137,
      "inverse_flattening": 298.257223563
    }
  }
}"#,
    );
    refused(
        r#"{
  "type": "GeographicCRS",
  "name": "WGS 84",
  "datum": {
    "type": "GeodeticReferenceFrame",
    "name": "World Geodetic System 1984",
    "ellipsoid": {
      "name": "WGS 84",
      "semi_major_axis": 6378137,
      "inverse_flattening": 298.257223563
    }
  },
  "coordinate_system": {
    "subtype": "Cartesian",
    "axis": [
      {
        "name": "Easting",
        "abbreviation": "E",
        "direction": "east",
        "unit": "metre"
      },
      {
        "name": "Northing",
        "abbreviation": "N",
        "direction": "north",
        "unit": "metre"
      }
    ]
  }
}"#,
    );
}
