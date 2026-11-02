//! PROJ 9.9.0 `test/unit/test_io.cpp`, lot io-core-b: the `io.projparse_*` tail,
//! `io.createFromUserInput*`, `wkt_export.*`, `json_import.*` and related names.
//! Strings and numbers are copied from the C++ source without change.

use proj_rs::iso19111::{create_object_from_json, Equivalence, JsonFormat, JsonObject};
use proj_rs::{
    create_crs_from_proj_string, create_from_user_input, Conversion, CoordinateOperationFactory,
    Crs, DatabaseContext, GeodeticCRS, GeographicCRS, OpProps, ProjectedCRS, WktConvention,
    WktFormatter, WktLines, WktObject, WktParser,
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

#[test]
fn json_import_dynamic_geodetic_reference_frame_with_implicit_prime_meridian() {
    let json = r#"{
  "$schema": "foo",
  "type": "DynamicGeodeticReferenceFrame",
  "name": "World Geodetic System 1984",
  "frame_reference_epoch": 1,
  "ellipsoid": {
    "name": "WGS 84",
    "semi_major_axis": 6378137,
    "inverse_flattening": 298.257223563
  }
}"#;
    let object = round_trip(json);
    assert!(matches!(object, JsonObject::GeodeticReferenceFrame(_)));
}

#[test]
fn json_import_invalid_bbox() {
    assert!(create_object_from_json(
        r#"{
  "$schema": "foo",
  "type": "GeodeticReferenceFrame",
  "name": "World Geodetic System 1984",
  "ellipsoid": {
    "name": "WGS 84",
    "semi_major_axis": 6378137,
    "inverse_flattening": 298.257223563
  },
  "bbox": {
    "south_latitude": -90,
    "west_longitude": -180,
    "north_latitude": 90,
    "east_longitude": 180
  }
}"#,
        None
    )
    .is_ok());
    refused(
        r#"{
  "$schema": "foo",
  "type": "GeodeticReferenceFrame",
  "name": "World Geodetic System 1984",
  "ellipsoid": {
    "name": "WGS 84",
    "semi_major_axis": 6378137,
    "inverse_flattening": 298.257223563
  },
  "bbox": {
    "south_latitude": 90,
    "west_longitude": -180,
    "north_latitude": -90,
    "east_longitude": 180
  }
}"#,
    );
}

#[test]
fn json_import_spherical_planetocentric() {
    let json = r#"{
  "$schema": "foo",
  "type": "GeodeticCRS",
  "name": "Mercury (2015) / Ocentric",
  "datum": {
    "type": "GeodeticReferenceFrame",
    "name": "Mercury (2015)",
    "anchor": "Hun Kal: 20.0",
    "ellipsoid": {
      "name": "Mercury (2015)",
      "semi_major_axis": 2440530,
      "inverse_flattening": 1075.12334801762
    },
    "prime_meridian": {
      "name": "Reference Meridian",
      "longitude": 0
    }
  },
  "coordinate_system": {
    "subtype": "spherical",
    "axis": [
      {
        "name": "Planetocentric latitude",
        "abbreviation": "U",
        "direction": "north",
        "unit": "degree"
      },
      {
        "name": "Planetocentric longitude",
        "abbreviation": "V",
        "direction": "east",
        "unit": "degree"
      }
    ]
  },
  "id": {
    "authority": "IAU",
    "code": 19902
  },
  "remarks": "Source of IAU Coordinate systems: doi://10.1007/s10569-017-9805-5"
}"#;
    let object = round_trip(json);
    let JsonObject::Crs(Crs::Geodetic(crs @ GeodeticCRS::Spherical(_))) = &object else {
        panic!("spherical geodetic CRS");
    };
    let coordinate_system = crs.coordinate_system();
    let names: Vec<&str> = coordinate_system
        .axes()
        .iter()
        .map(|axis| axis.name())
        .collect();
    assert_eq!(
        names,
        ["Planetocentric latitude", "Planetocentric longitude"]
    );
}

#[test]
fn json_import_geographic_crs_with_datum_ensemble() {
    let json = r#"{
  "$schema": "foo",
  "type": "GeographicCRS",
  "name": "WGS 84",
  "datum_ensemble": {
    "name": "WGS 84 ensemble",
    "members": [
      {
        "name": "World Geodetic System 1984 (Transit)"
      },
      {
        "name": "World Geodetic System 1984 (G730)"
      },
      {
        "name": "Some unknown ensemble with unknown id",
        "id": {
          "authority": "UNKNOWN",
          "code": 1234
        }
      }
    ],
    "ellipsoid": {
      "name": "WGS 84",
      "semi_major_axis": 6378137,
      "inverse_flattening": 298.257223563
    },
    "accuracy": "2"
  },
  "coordinate_system": {
    "subtype": "ellipsoidal",
    "axis": [
      {
        "name": "Latitude",
        "abbreviation": "lat",
        "direction": "north",
        "unit": "degree"
      },
      {
        "name": "Longitude",
        "abbreviation": "lon",
        "direction": "east",
        "unit": "degree"
      }
    ]
  }
}"#;
    let expected_json = r#"{
  "$schema": "foo",
  "type": "GeographicCRS",
  "name": "WGS 84",
  "datum_ensemble": {
    "name": "WGS 84 ensemble",
    "members": [
      {
        "name": "World Geodetic System 1984 (Transit)",
        "id": {
          "authority": "EPSG",
          "code": 1166
        }
      },
      {
        "name": "World Geodetic System 1984 (G730)",
        "id": {
          "authority": "EPSG",
          "code": 1152
        }
      },
      {
        "name": "Some unknown ensemble with unknown id",
        "id": {
          "authority": "UNKNOWN",
          "code": 1234
        }
      }
    ],
    "ellipsoid": {
      "name": "WGS 84",
      "semi_major_axis": 6378137,
      "inverse_flattening": 298.257223563,
      "id": {
        "authority": "EPSG",
        "code": 7030
      }
    },
    "accuracy": "2"
  },
  "coordinate_system": {
    "subtype": "ellipsoidal",
    "axis": [
      {
        "name": "Latitude",
        "abbreviation": "lat",
        "direction": "north",
        "unit": "degree"
      },
      {
        "name": "Longitude",
        "abbreviation": "lon",
        "direction": "east",
        "unit": "degree"
      }
    ]
  }
}"#;
    // No database
    let object = round_trip(json);
    assert!(matches!(object, JsonObject::Crs(Crs::Geographic(_))));
    let db = DatabaseContext::create().expect("proj.db");
    let object = create_object_from_json(json, Some(&db)).expect("PROJJSON");
    assert!(matches!(object, JsonObject::Crs(Crs::Geographic(_))));
    assert_eq!(export_foo(&object), expected_json);
    let object = create_object_from_json(expected_json, Some(&db)).expect("PROJJSON");
    assert!(matches!(object, JsonObject::Crs(Crs::Geographic(_))));
    assert_eq!(export_foo(&object), expected_json);
}

#[test]
fn json_import_datum_ensemble_without_ellipsoid() {
    let json = r#"{
  "$schema": "foo",
  "type": "DatumEnsemble",
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
}"#;
    let object = round_trip(json);
    assert!(matches!(object, JsonObject::DatumEnsemble(_)));
}

#[test]
fn json_import_ensemble_without_members() {
    let json = r#"{
  "type": "GeographicCRS",
  "name": "WGS 84",
  "datum_ensemble": {
    "name": "World Geodetic System 1984 ensemble",
    "ellipsoid": {
      "name": "WGS 84",
      "semi_major_axis": 6378137,
      "inverse_flattening": 298.257223563
    },
    "accuracy": "2.0",
    "id": {
      "authority": "EPSG",
      "code": 6326
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
  }
}"#;
    let db = DatabaseContext::create().expect("proj.db");
    let object = create_object_from_json(json, Some(&db)).expect("PROJJSON");
    let JsonObject::Crs(Crs::Geographic(crs)) = &object else {
        panic!("geographic CRS");
    };
    let ensemble = crs.datum_ensemble().expect("ensemble");
    assert!(ensemble.members().len() >= 2);
}

#[test]
fn json_import_ensemble_without_members_no_db() {
    refused(
        r#"{
  "type": "GeographicCRS",
  "name": "WGS 84",
  "datum_ensemble": {
    "name": "World Geodetic System 1984 ensemble",
    "ellipsoid": {
      "name": "WGS 84",
      "semi_major_axis": 6378137,
      "inverse_flattening": 298.257223563
    },
    "accuracy": "2.0",
    "id": {
      "authority": "EPSG",
      "code": 6326
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
  }
}"#,
    );
}

#[test]
fn json_import_ensemble_without_members_unknown_name() {
    let db = DatabaseContext::create().expect("proj.db");
    assert!(create_object_from_json(
        r#"{
  "type": "GeographicCRS",
  "name": "WGS 84",
  "datum_ensemble": {
    "name": "i do not exist",
    "ellipsoid": {
      "name": "WGS 84",
      "semi_major_axis": 6378137,
      "inverse_flattening": 298.257223563
    },
    "accuracy": "2.0",
    "id": {
      "authority": "EPSG",
      "code": 6326
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
  }
}"#,
        Some(&db)
    )
    .is_err());
}
#[test]
fn json_import_projected_crs() {
    let json = r#"{
  "$schema": "foo",
  "type": "ProjectedCRS",
  "name": "WGS 84 / UTM zone 31N",
  "base_crs": {
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
  "conversion": {
    "name": "UTM zone 31N",
    "method": {
      "name": "Transverse Mercator",
      "id": {
        "authority": "EPSG",
        "code": 9807
      }
    },
    "parameters": [
      {
        "name": "Latitude of natural origin",
        "value": 0,
        "unit": "degree",
        "id": {
          "authority": "EPSG",
          "code": 8801
        }
      },
      {
        "name": "Longitude of natural origin",
        "value": 3,
        "unit": "degree",
        "id": {
          "authority": "EPSG",
          "code": 8802
        }
      },
      {
        "name": "Scale factor at natural origin",
        "value": 0.9996,
        "unit": "unity",
        "id": {
          "authority": "EPSG",
          "code": 8805
        }
      },
      {
        "name": "False easting",
        "value": 500000,
        "unit": "metre",
        "id": {
          "authority": "EPSG",
          "code": 8806
        }
      },
      {
        "name": "False northing",
        "value": 0,
        "unit": "metre",
        "id": {
          "authority": "EPSG",
          "code": 8807
        }
      }
    ]
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
}"#;
    let object = import_json(json);
    assert!(matches!(object, JsonObject::Crs(Crs::Projected(_))));
    let got = export_foo(&object);
    // The base CRS has no `type` member in the C++ output; the test removes it.
    let marker = "    \"type\": \"GeographicCRS\",\n";
    let position = got.find(marker).expect("base CRS type");
    let got = format!("{}{}", &got[..position], &got[position + marker.len()..]);
    assert_eq!(got, json);
}

#[test]
fn json_import_projected_crs_with_geocentric_base() {
    let json = r#"{
  "$schema": "foo",
  "type": "ProjectedCRS",
  "name": "EPSG topocentric example B",
  "base_crs": {
    "name": "WGS 84",
    "datum_ensemble": {
      "name": "World Geodetic System 1984 ensemble",
      "members": [
        {
          "name": "World Geodetic System 1984 (Transit)"
        },
        {
          "name": "World Geodetic System 1984 (G730)"
        },
        {
          "name": "World Geodetic System 1984 (G873)"
        },
        {
          "name": "World Geodetic System 1984 (G1150)"
        },
        {
          "name": "World Geodetic System 1984 (G1674)"
        },
        {
          "name": "World Geodetic System 1984 (G1762)"
        }
      ],
      "ellipsoid": {
        "name": "WGS 84",
        "semi_major_axis": 6378137,
        "inverse_flattening": 298.257223563
      },
      "accuracy": "2.0"
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
    },
    "id": {
      "authority": "EPSG",
      "code": 4978
    }
  },
  "conversion": {
    "name": "EPSG topocentric example B",
    "method": {
      "name": "Geocentric/topocentric conversions",
      "id": {
        "authority": "EPSG",
        "code": 9836
      }
    },
    "parameters": [
      {
        "name": "Geocentric X of topocentric origin",
        "value": 3771793.97,
        "unit": "metre",
        "id": {
          "authority": "EPSG",
          "code": 8837
        }
      },
      {
        "name": "Geocentric Y of topocentric origin",
        "value": 140253.34,
        "unit": "metre",
        "id": {
          "authority": "EPSG",
          "code": 8838
        }
      },
      {
        "name": "Geocentric Z of topocentric origin",
        "value": 5124304.35,
        "unit": "metre",
        "id": {
          "authority": "EPSG",
          "code": 8839
        }
      }
    ]
  },
  "coordinate_system": {
    "subtype": "Cartesian",
    "axis": [
      {
        "name": "Topocentric East",
        "abbreviation": "U",
        "direction": "east",
        "unit": "metre"
      },
      {
        "name": "Topocentric North",
        "abbreviation": "V",
        "direction": "north",
        "unit": "metre"
      },
      {
        "name": "Topocentric height",
        "abbreviation": "W",
        "direction": "up",
        "unit": "metre"
      }
    ]
  },
  "scope": "Example only (fictitious).",
  "area": "Description of the extent of the CRS.",
  "bbox": {
    "south_latitude": -90,
    "west_longitude": -180,
    "north_latitude": 90,
    "east_longitude": 180
  },
  "id": {
    "authority": "EPSG",
    "code": 5820
  }
}"#;
    let object = import_json(json);
    let JsonObject::Crs(Crs::Projected(crs)) = &object else {
        panic!("projected CRS");
    };
    assert!(matches!(crs.base_crs(), GeodeticCRS::Geocentric(_)));
    let got = export_foo(&object);
    let marker = "    \"type\": \"GeodeticCRS\",\n";
    let position = got.find(marker).expect("base CRS type");
    let got = format!("{}{}", &got[..position], &got[position + marker.len()..]);
    assert_eq!(got, json);
}

#[test]
fn json_import_vertical_crs() {
    let json = r#"{
  "$schema": "foo",
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
  }
}"#;
    let object = round_trip(json);
    let JsonObject::Crs(Crs::Vertical(crs)) = &object else {
        panic!("vertical CRS");
    };
    let datum = crs.datum().expect("datum").clone();
    let datum_json = export_foo(&JsonObject::VerticalReferenceFrame(datum));
    assert!(matches!(
        import_json(&datum_json),
        JsonObject::VerticalReferenceFrame(_)
    ));
}

#[test]
fn io_projparse_projected_unknown() {
    let crs = proj_crs(
        "+proj=mbt_s +unused_flag +lat_0=45 +lon_0=0 +k=1 +x_0=10 +y_0=0 \
         +datum=WGS84 +type=crs",
    );
    let Crs::Projected(_) = &crs else {
        panic!("projected");
    };
    let wkt2 = wkt2_single(&crs);
    assert!(
        wkt2.contains(
            "CONVERSION[\"unknown\",METHOD[\"PROJ mbt_s\"],\
             PARAMETER[\"lat_0\",45,ANGLEUNIT[\"degree\",0.0174532925199433]],\
             PARAMETER[\"lon_0\",0,ANGLEUNIT[\"degree\",0.0174532925199433]],\
             PARAMETER[\"k\",1,SCALEUNIT[\"unity\",1]],\
             PARAMETER[\"x_0\",10,LENGTHUNIT[\"metre\",1]],\
             PARAMETER[\"y_0\",0,LENGTHUNIT[\"metre\",1]]]"
        ),
        "{wkt2}"
    );
    let expected_wkt1 = "PROJCS[\"unknown\",GEOGCS[\"unknown\",DATUM[\"WGS_1984\",SPHEROID[\"WGS \
         84\",6378137,298.257223563,AUTHORITY[\"EPSG\",\"7030\"]],AUTHORITY[\"EPSG\",\"6326\"]],\
         PRIMEM[\"Greenwich\",0,AUTHORITY[\"EPSG\",\"8901\"]],UNIT[\"degree\",0.0174532925199433,\
         AUTHORITY[\"EPSG\",\"9122\"]]],PROJECTION[\"custom_proj4\"],UNIT[\"metre\",1,\
         AUTHORITY[\"EPSG\",\"9001\"]],AXIS[\"Easting\",EAST],AXIS[\"Northing\",NORTH],\
         EXTENSION[\"PROJ4\",\"+proj=mbt_s +lat_0=45 +lon_0=0 +k=1 +x_0=10 +y_0=0 +datum=WGS84\"]]";
    let mut wkt1 = WktFormatter::create(WktConvention::Wkt1Gdal);
    wkt1.simul_cur_node_has_id();
    wkt1.set_lines(WktLines::Single);
    let exported = crs.export_to_wkt(&mut wkt1).expect("WKT1");
    assert_eq!(exported, expected_wkt1);
    assert_eq!(
        crs.export_to_proj_string().expect("PROJ string"),
        "+proj=mbt_s +lat_0=45 +lon_0=0 +k=1 +x_0=10 +y_0=0 +datum=WGS84 +type=crs"
    );
    let again = parse_crs(&exported);
    let again_wkt2 = wkt2_single(&again);
    assert!(
        again_wkt2.contains("METHOD[\"PROJ mbt_s\"]"),
        "{again_wkt2}"
    );
}

#[test]
fn io_projparse_geoc() {
    let crs = proj_crs("+proj=longlat +geoc +datum=WGS84 +no_defs +type=crs");
    assert!(matches!(crs, Crs::Geodetic(GeodeticCRS::Spherical(_))));
    assert!(crs.export_to_proj_string().is_err());
}

#[test]
fn io_extension_proj4() {
    let crs = proj_crs("+proj=utm +datum=NAD27 +zone=11 +over +type=crs");
    let Crs::Projected(_) = &crs else {
        panic!("projected");
    };
    assert_eq!(
        crs.remarks(),
        "PROJ CRS string: +proj=utm +datum=NAD27 +zone=11 +over"
    );
    let mut wkt2 = WktFormatter::create(WktConvention::Wkt2_2015);
    let exported = crs.export_to_wkt(&mut wkt2).expect("WKT2");
    let again = parse_crs(&exported);
    assert_eq!(
        again.export_to_proj_string().expect("PROJ string"),
        "+proj=utm +datum=NAD27 +zone=11 +over +type=crs"
    );
    let wkt3 = "PROJCRS[\"unknown\",\n\
        \x20   BASEGEOGCRS[\"unknown\",\n\
        \x20       DATUM[\"North American Datum 1927\",\n\
        \x20           ELLIPSOID[\"Clarke 1866\",6378206.4,294.978698213898,\n\
        \x20               LENGTHUNIT[\"metre\",1]],\n\
        \x20           ID[\"EPSG\",6267]],\n\
        \x20       PRIMEM[\"Greenwich\",0,\n\
        \x20           ANGLEUNIT[\"degree\",0.0174532925199433],\n\
        \x20           ID[\"EPSG\",8901]]],\n\
        \x20   CONVERSION[\"UTM zone 11N\",\n\
        \x20       METHOD[\"Transverse Mercator\",\n\
        \x20           ID[\"EPSG\",9807]],\n\
        \x20       PARAMETER[\"Latitude of natural origin\",0,\n\
        \x20           ANGLEUNIT[\"degree\",0.0174532925199433],\n\
        \x20           ID[\"EPSG\",8801]],\n\
        \x20       PARAMETER[\"Longitude of natural origin\",-117,\n\
        \x20           ANGLEUNIT[\"degree\",0.0174532925199433],\n\
        \x20           ID[\"EPSG\",8802]],\n\
        \x20       PARAMETER[\"Scale factor at natural origin\",0.9996,\n\
        \x20           SCALEUNIT[\"unity\",1],\n\
        \x20           ID[\"EPSG\",8805]],\n\
        \x20       PARAMETER[\"False easting\",500000,\n\
        \x20           LENGTHUNIT[\"metre\",1],\n\
        \x20           ID[\"EPSG\",8806]],\n\
        \x20       PARAMETER[\"False northing\",0,\n\
        \x20           LENGTHUNIT[\"metre\",1],\n\
        \x20           ID[\"EPSG\",8807]],\n\
        \x20       ID[\"EPSG\",16011]],\n\
        \x20   CS[Cartesian,2],\n\
        \x20       AXIS[\"(E)\",east,\n\
        \x20           ORDER[1],\n\
        \x20           LENGTHUNIT[\"metre\",1,\n\
        \x20               ID[\"EPSG\",9001]]],\n\
        \x20       AXIS[\"(N)\",north,\n\
        \x20           ORDER[2],\n\
        \x20           LENGTHUNIT[\"metre\",1,\n\
        \x20               ID[\"EPSG\",9001]]],\n\
        \x20   REMARK[\"Prefix. PROJ CRS string: +proj=utm +datum=NAD27 +zone=11 \
         +over. Suffix\"]]";
    let crs3 = parse_crs(wkt3);
    assert_eq!(
        crs3.remarks(),
        "Prefix. PROJ CRS string: +proj=utm +datum=NAD27 +zone=11 +over. Suffix"
    );
    assert_eq!(
        crs3.export_to_proj_string().expect("PROJ string"),
        "+proj=utm +datum=NAD27 +zone=11 +over +type=crs"
    );
}

#[test]
fn json_import_conversion_utm_zone_south_wrong_id() {
    let json = r#"{
  "type": "Conversion",
  "name": "UTM zone 55S",
  "method": {
    "name": "Transverse Mercator",
    "id": {
      "authority": "EPSG",
      "code": 9807
    }
  },
  "parameters": [
    {
      "name": "Latitude of natural origin",
      "value": 0,
      "unit": {
        "type": "AngularUnit",
        "name": "Degree",
        "conversion_factor": 0.0174532925199433
      },
      "id": {
        "authority": "EPSG",
        "code": 8801
      }
    },
    {
      "name": "Longitude of natural origin",
      "value": 147,
      "unit": {
        "type": "AngularUnit",
        "name": "Degree",
        "conversion_factor": 0.0174532925199433
      },
      "id": {
        "authority": "EPSG",
        "code": 8802
      }
    },
    {
      "name": "Scale factor at natural origin",
      "value": 0.9996,
      "unit": "unity",
      "id": {
        "authority": "EPSG",
        "code": 8805
      }
    },
    {
      "name": "False easting",
      "value": 500000,
      "unit": "metre",
      "id": {
        "authority": "EPSG",
        "code": 8806
      }
    },
    {
      "name": "False northing",
      "value": 10000000,
      "unit": "metre",
      "id": {
        "authority": "EPSG",
        "code": 8807
      }
    }
  ],
  "id": {
    "authority": "EPSG",
    "code": 17055
  }
}"#;
    let JsonObject::Conversion(conversion) = import_json(json) else {
        panic!("conversion");
    };
    assert_eq!(conversion.identifiers()[0].code(), "16155");
}

#[test]
fn json_import_invalid_coordinatemetadata() {
    for json in [
        "{\n  \"$schema\": \"foo\",\n  \"type\": \"CoordinateMetadata\"\n}",
        "{\n  \"$schema\": \"foo\",\n  \"type\": \"CoordinateMetadata\",\n  \"crs\": \"not quite a CRS...\"\n}",
        "{\n  \"type\": \"CoordinateMetadata\",\n  \"coordinateEpoch\": \"this should be a number\"\n}",
    ] {
        refused(json);
    }
}

#[test]
fn io_createfromuserinput_ogc_crs_url() {
    let db = DatabaseContext::create().expect("proj.db");
    let crs =
        create_from_user_input("http://www.opengis.net/def/crs/EPSG/0/4326", &db).expect("4326");
    assert!(matches!(
        crs,
        Crs::Geographic(_) | Crs::Geodetic(GeodeticCRS::Geographic(_))
    ));
    let iau = create_from_user_input("http://www.opengis.net/def/crs/IAU/2015/49900", &db)
        .expect("IAU 2015");
    assert!(matches!(
        iau,
        Crs::Geographic(_) | Crs::Geodetic(GeodeticCRS::Geographic(_))
    ));
    let latest = create_from_user_input("http://www.opengis.net/def/crs/IAU/0/49900", &db)
        .expect("IAU latest");
    assert!(matches!(
        latest,
        Crs::Geographic(_) | Crs::Geodetic(GeodeticCRS::Geographic(_))
    ));
    assert!(create_from_user_input("http://www.opengis.net/def/crs", &db).is_err());
    assert!(create_from_user_input("http://www.opengis.net/def/crs/EPSG/0", &db).is_err());
    assert!(create_from_user_input("http://www.opengis.net/def/crs/EPSG/0/XXXX", &db).is_err());
    assert!(
        create_from_user_input("http://www.opengis.net/def/crs/IAU/2015/invalid", &db).is_err()
    );
    let compound = create_from_user_input(
        "http://www.opengis.net/def/crs-compound?1=http://www.opengis.net/def/crs/EPSG/0/4326&2=http://www.opengis.net/def/crs/EPSG/0/3855",
        &db,
    )
    .expect("compound");
    let Crs::Compound(compound) = compound else {
        panic!("compound");
    };
    assert_eq!(compound.name(), "WGS 84 + EGM2008 height");
    assert!(create_from_user_input("http://www.opengis.net/def/crs-compound?", &db).is_err());
    assert!(create_from_user_input(
        "http://www.opengis.net/def/crs-compound?1=http://www.opengis.net/def/crs/EPSG/0/4326",
        &db
    )
    .is_err());
    assert!(create_from_user_input(
        "http://www.opengis.net/def/crs-compound?1=http://www.opengis.net/def/crs/EPSG/0/4326&2=http://www.opengis.net/def/crs/EPSG/0/4326",
        &db
    )
    .is_err());
    assert!(create_from_user_input(
        "http://www.opengis.net/def/crs-compound?1=http://www.opengis.net/def/crs/EPSG/0/4326&3=http://www.opengis.net/def/crs/EPSG/0/3855",
        &db
    )
    .is_err());
    assert!(create_from_user_input(
        "http://www.opengis.net/def/crs-compound?1=http://www.opengis.net/def/crs/EPSG/0/4326&bla",
        &db
    )
    .is_err());
    assert!(create_from_user_input(
        "http://www.opengis.net/def/crs-compound?1=http://www.opengis.net/def/crs/EPSG/0/4326&two=http://www.opengis.net/def/crs/EPSG/0/3855",
        &db
    )
    .is_err());
}

#[test]
fn json_import_bound_crs() {
    let json = r#"{
  "$schema": "foo",
  "type": "BoundCRS",
  "source_crs": {
    "type": "GeographicCRS",
    "name": "unknown",
    "datum": {
      "type": "GeodeticReferenceFrame",
      "name": "Unknown based on GRS80 ellipsoid",
      "ellipsoid": {
        "name": "GRS 1980",
        "semi_major_axis": 6378137,
        "inverse_flattening": 298.257222101,
        "id": {
          "authority": "EPSG",
          "code": 7019
        }
      }
    },
    "coordinate_system": {
      "subtype": "ellipsoidal",
      "axis": [
        {
          "name": "Longitude",
          "abbreviation": "lon",
          "direction": "east",
          "unit": "degree"
        },
        {
          "name": "Latitude",
          "abbreviation": "lat",
          "direction": "north",
          "unit": "degree"
        }
      ]
    }
  },
  "target_crs": {
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
          "name": "Latitude",
          "abbreviation": "lat",
          "direction": "north",
          "unit": "degree"
        },
        {
          "name": "Longitude",
          "abbreviation": "lon",
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
  "transformation": {
    "name": "unknown to WGS84",
    "method": {
      "name": "NTv2",
      "id": {
        "authority": "EPSG",
        "code": 9615
      }
    },
    "parameters": [
      {
        "name": "Latitude and longitude difference file",
        "value": "@foo",
        "id": {
          "authority": "EPSG",
          "code": 8656
        }
      }
    ]
  }
}"#;
    let obj = round_trip(json);
    assert!(matches!(obj, JsonObject::Crs(Crs::Bound(_))));
}

#[test]
fn json_import_transformation() {
    let json = r#"{
  "$schema": "foo",
  "type": "Transformation",
  "name": "GDA94 to GDA2020 (1)",
  "source_crs": {
    "type": "GeographicCRS",
    "name": "GDA94",
    "datum": {
      "type": "GeodeticReferenceFrame",
      "name": "Geocentric Datum of Australia 1994",
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
        }
      ]
    }
  },
  "target_crs": {
    "type": "GeographicCRS",
    "name": "GDA2020",
    "datum": {
      "type": "GeodeticReferenceFrame",
      "name": "Geocentric Datum of Australia 2020",
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
        }
      ]
    }
  },
  "method": {
    "name": "Coordinate Frame rotation (geog2D domain)",
    "id": {
      "authority": "EPSG",
      "code": 9607
    }
  },
  "parameters": [
    {
      "name": "X-axis translation",
      "value": 61.55,
      "unit": {
        "type": "LinearUnit",
        "name": "millimetre",
        "conversion_factor": 0.001
      },
      "id": {
        "authority": "EPSG",
        "code": 8605
      }
    },
    {
      "name": "Y-axis translation",
      "value": -10.87,
      "unit": {
        "type": "LinearUnit",
        "name": "millimetre",
        "conversion_factor": 0.001
      },
      "id": {
        "authority": "EPSG",
        "code": 8606
      }
    },
    {
      "name": "Z-axis translation",
      "value": -40.19,
      "unit": {
        "type": "LinearUnit",
        "name": "millimetre",
        "conversion_factor": 0.001
      },
      "id": {
        "authority": "EPSG",
        "code": 8607
      }
    },
    {
      "name": "X-axis rotation",
      "value": -39.4924,
      "unit": {
        "type": "AngularUnit",
        "name": "milliarc-second",
        "conversion_factor": 4.84813681109536e-09
      },
      "id": {
        "authority": "EPSG",
        "code": 8608
      }
    },
    {
      "name": "Y-axis rotation",
      "value": -32.7221,
      "unit": {
        "type": "AngularUnit",
        "name": "milliarc-second",
        "conversion_factor": 4.84813681109536e-09
      },
      "id": {
        "authority": "EPSG",
        "code": 8609
      }
    },
    {
      "name": "Z-axis rotation",
      "value": -32.8979,
      "unit": {
        "type": "AngularUnit",
        "name": "milliarc-second",
        "conversion_factor": 4.84813681109536e-09
      },
      "id": {
        "authority": "EPSG",
        "code": 8610
      }
    },
    {
      "name": "Scale difference",
      "value": -9.994,
      "unit": {
        "type": "ScaleUnit",
        "name": "parts per billion",
        "conversion_factor": 1e-09
      },
      "id": {
        "authority": "EPSG",
        "code": 8611
      }
    }
  ],
  "accuracy": "0.01",
  "scope": "scope",
  "area": "Australia - GDA",
  "bbox": {
    "south_latitude": -60.56,
    "west_longitude": 93.41,
    "north_latitude": -8.47,
    "east_longitude": 173.35
  },
  "id": {
    "authority": "EPSG",
    "code": 8048
  },
  "remarks": "foo"
}"#;
    let object = round_trip(json);
    assert!(matches!(object, JsonObject::Transformation(_)));
}

#[test]
fn json_import_concatenated_operation() {
    let json = r#"{
  "$schema": "foo",
  "type": "ConcatenatedOperation",
  "name": "Inverse of Vicgrid + GDA94 to GDA2020 (1)",
  "source_crs": {
    "type": "ProjectedCRS",
    "name": "GDA94 / Vicgrid",
    "base_crs": {
      "type": "GeographicCRS",
      "name": "GDA94",
      "datum": {
        "type": "GeodeticReferenceFrame",
        "name": "Geocentric Datum of Australia 1994",
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
          }
        ]
      },
      "id": {
        "authority": "EPSG",
        "code": 4283
      }
    },
    "conversion": {
      "name": "Vicgrid",
      "method": {
        "name": "Lambert Conic Conformal (2SP)",
        "id": {
          "authority": "EPSG",
          "code": 9802
        }
      },
      "parameters": [
        {
          "name": "Latitude of false origin",
          "value": -37,
          "unit": "degree",
          "id": {
            "authority": "EPSG",
            "code": 8821
          }
        },
        {
          "name": "Longitude of false origin",
          "value": 145,
          "unit": "degree",
          "id": {
            "authority": "EPSG",
            "code": 8822
          }
        },
        {
          "name": "Latitude of 1st standard parallel",
          "value": -36,
          "unit": "degree",
          "id": {
            "authority": "EPSG",
            "code": 8823
          }
        },
        {
          "name": "Latitude of 2nd standard parallel",
          "value": -38,
          "unit": "degree",
          "id": {
            "authority": "EPSG",
            "code": 8824
          }
        },
        {
          "name": "Easting at false origin",
          "value": 2500000,
          "unit": "metre",
          "id": {
            "authority": "EPSG",
            "code": 8826
          }
        },
        {
          "name": "Northing at false origin",
          "value": 2500000,
          "unit": "metre",
          "id": {
            "authority": "EPSG",
            "code": 8827
          }
        }
      ]
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
    },
    "id": {
      "authority": "EPSG",
      "code": 3111
    }
  },
  "target_crs": {
    "type": "GeographicCRS",
    "name": "GDA2020",
    "datum": {
      "type": "GeodeticReferenceFrame",
      "name": "Geocentric Datum of Australia 2020",
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
        }
      ]
    },
    "id": {
      "authority": "EPSG",
      "code": 7844
    }
  },
  "steps": [
    {
      "type": "Conversion",
      "name": "Inverse of Vicgrid",
      "method": {
        "name": "Inverse of Lambert Conic Conformal (2SP)",
        "id": {
          "authority": "INVERSE(EPSG)",
          "code": 9802
        }
      },
      "parameters": [
        {
          "name": "Latitude of false origin",
          "value": -37,
          "unit": "degree",
          "id": {
            "authority": "EPSG",
            "code": 8821
          }
        },
        {
          "name": "Longitude of false origin",
          "value": 145,
          "unit": "degree",
          "id": {
            "authority": "EPSG",
            "code": 8822
          }
        },
        {
          "name": "Latitude of 1st standard parallel",
          "value": -36,
          "unit": "degree",
          "id": {
            "authority": "EPSG",
            "code": 8823
          }
        },
        {
          "name": "Latitude of 2nd standard parallel",
          "value": -38,
          "unit": "degree",
          "id": {
            "authority": "EPSG",
            "code": 8824
          }
        },
        {
          "name": "Easting at false origin",
          "value": 2500000,
          "unit": "metre",
          "id": {
            "authority": "EPSG",
            "code": 8826
          }
        },
        {
          "name": "Northing at false origin",
          "value": 2500000,
          "unit": "metre",
          "id": {
            "authority": "EPSG",
            "code": 8827
          }
        }
      ],
      "id": {
        "authority": "INVERSE(EPSG)",
        "code": 17361
      }
    },
    {
      "type": "Transformation",
      "name": "GDA94 to GDA2020 (1)",
      "source_crs": {
        "type": "GeographicCRS",
        "name": "GDA94",
        "datum": {
          "type": "GeodeticReferenceFrame",
          "name": "Geocentric Datum of Australia 1994",
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
            }
          ]
        },
        "id": {
          "authority": "EPSG",
          "code": 4283
        }
      },
      "target_crs": {
        "type": "GeographicCRS",
        "name": "GDA2020",
        "datum": {
          "type": "GeodeticReferenceFrame",
          "name": "Geocentric Datum of Australia 2020",
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
            }
          ]
        },
        "id": {
          "authority": "EPSG",
          "code": 7844
        }
      },
      "method": {
        "name": "Coordinate Frame rotation (geog2D domain)",
        "id": {
          "authority": "EPSG",
          "code": 9607
        }
      },
      "parameters": [
        {
          "name": "X-axis translation",
          "value": 61.55,
          "unit": {
            "type": "LinearUnit",
            "name": "millimetre",
            "conversion_factor": 0.001
          },
          "id": {
            "authority": "EPSG",
            "code": 8605
          }
        },
        {
          "name": "Y-axis translation",
          "value": -10.87,
          "unit": {
            "type": "LinearUnit",
            "name": "millimetre",
            "conversion_factor": 0.001
          },
          "id": {
            "authority": "EPSG",
            "code": 8606
          }
        },
        {
          "name": "Z-axis translation",
          "value": -40.19,
          "unit": {
            "type": "LinearUnit",
            "name": "millimetre",
            "conversion_factor": 0.001
          },
          "id": {
            "authority": "EPSG",
            "code": 8607
          }
        },
        {
          "name": "X-axis rotation",
          "value": -39.4924,
          "unit": {
            "type": "AngularUnit",
            "name": "milliarc-second",
            "conversion_factor": 4.84813681109536e-09
          },
          "id": {
            "authority": "EPSG",
            "code": 8608
          }
        },
        {
          "name": "Y-axis rotation",
          "value": -32.7221,
          "unit": {
            "type": "AngularUnit",
            "name": "milliarc-second",
            "conversion_factor": 4.84813681109536e-09
          },
          "id": {
            "authority": "EPSG",
            "code": 8609
          }
        },
        {
          "name": "Z-axis rotation",
          "value": -32.8979,
          "unit": {
            "type": "AngularUnit",
            "name": "milliarc-second",
            "conversion_factor": 4.84813681109536e-09
          },
          "id": {
            "authority": "EPSG",
            "code": 8610
          }
        },
        {
          "name": "Scale difference",
          "value": -9.994,
          "unit": {
            "type": "ScaleUnit",
            "name": "parts per billion",
            "conversion_factor": 1e-09
          },
          "id": {
            "authority": "EPSG",
            "code": 8611
          }
        }
      ],
      "accuracy": "0.01",
      "id": {
        "authority": "EPSG",
        "code": 8048
      },
      "remarks": "remarks"
    }
  ],
  "accuracy": "0.02",
  "area": "Australia - GDA",
  "bbox": {
    "south_latitude": -60.56,
    "west_longitude": 93.41,
    "north_latitude": -8.47,
    "east_longitude": 173.35
  }
}"#;
    let object = round_trip(json);
    assert!(matches!(object, JsonObject::Concatenated(_)));
}

#[test]
fn json_import_pointmotionoperation() {
    let json = r#"{
  "$schema": "foo",
  "type": "PointMotionOperation",
  "name": "Canada velocity grid v7",
  "source_crs": {
    "type": "GeographicCRS",
    "name": "NAD83(CSRS)v7",
    "datum": {
      "type": "GeodeticReferenceFrame",
      "name": "North American Datum of 1983 (CSRS) version 7",
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
      "code": 8254
    }
  },
  "method": {
    "name": "Point motion by grid (Canada NTv2_Vel)",
    "id": {
      "authority": "EPSG",
      "code": 1070
    }
  },
  "parameters": [
    {
      "name": "Point motion velocity grid file",
      "value": "foo.tif"
    }
  ],
  "accuracy": "0.01",
  "scope": "scope",
  "area": "area",
  "bbox": {
    "south_latitude": 38.21,
    "west_longitude": -141.01,
    "north_latitude": 86.46,
    "east_longitude": -40.73
  },
  "id": {
    "authority": "DERIVED_FROM(EPSG)",
    "code": 9483
  },
  "remarks": "remark."
}"#;
    let object = round_trip(json);
    assert!(matches!(object, JsonObject::Transformation(_)));
}

#[test]
fn wkt_parse_pointmotionoperation() {
    let wkt = r#"POINTMOTIONOPERATION["Canada velocity grid v7",
    SOURCECRS[
        GEOGCRS["NAD83(CSRS)v7",
            DATUM["North American Datum of 1983 (CSRS) version 7",
                ELLIPSOID["GRS 1980",6378137,298.257222101,
                    LENGTHUNIT["metre",1]]],
            PRIMEM["Greenwich",0,
                ANGLEUNIT["degree",0.0174532925199433]],
            CS[ellipsoidal,3],
                AXIS["geodetic latitude (Lat)",north,
                    ORDER[1],
                    ANGLEUNIT["degree",0.0174532925199433]],
                AXIS["geodetic longitude (Lon)",east,
                    ORDER[2],
                    ANGLEUNIT["degree",0.0174532925199433]],
                AXIS["ellipsoidal height (h)",up,
                    ORDER[3],
                    LENGTHUNIT["metre",1]],
            ID["EPSG",8254]]],
    METHOD["Point motion by grid (Canada NTv2_Vel)",
        ID["EPSG",1070]],
    PARAMETERFILE["Point motion velocity grid file","foo.tif"],
    OPERATIONACCURACY[0.01],
    USAGE[
        SCOPE["scope"],
        AREA["area"],
        BBOX[38.21,-141.01,86.46,-40.73]],
    ID["DERIVED_FROM(EPSG)",9483],
    REMARK["remark."]]"#;
    let WktObject::Transformation(transformation) =
        WktParser::new().create_from_wkt(wkt).expect("wkt")
    else {
        panic!("point motion");
    };
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2019);
    let got = proj_rs::CoordinateOperation::Transformation(transformation)
        .export_to_wkt(&mut formatter)
        .expect("export");
    assert_eq!(got, wkt);
}
