//! PROJ 9.9.0 `test/unit/test_crs.cpp`, derived geographic CRS exported to a PROJ string.

use proj_rs::{CoordinateOperationFactory, Crs, DatabaseContext, ProjLines, WktParser};

fn parse_derived(wkt: &str) -> Crs {
    let obj = WktParser::new().create_from_wkt(wkt).expect("wkt");
    obj.crs().cloned().expect("crs")
}

/// `TEST(crs, derivedGeographicCRS_to_PROJ)`
#[test]
fn crs_derived_geographic_crs_to_proj() {
    let wkt = "GEODCRS[\"WMO Atlantic Pole\",\n\
        BASEGEODCRS[\"WGS 84\",\n\
            DATUM[\"World Geodetic System 1984\",\n\
                ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n\
                    LENGTHUNIT[\"metre\",1]]],\n\
            PRIMEM[\"Greenwich\",0,\n\
                ANGLEUNIT[\"degree\",0.0174532925199433]]],\n\
        DERIVINGCONVERSION[\"unnamed\",\n\
            METHOD[\"PROJ ob_tran o_proj=longlat\"],\n\
            PARAMETER[\"o_lat_p\",52,\n\
                UNIT[\"degree\",0.0174532925199433]],\n\
            PARAMETER[\"o_lon_p\",-30,\n\
                UNIT[\"degree\",0.0174532925199433]],\n\
            PARAMETER[\"lon_0\",-25,\n\
                UNIT[\"degree\",0.0174532925199433]]],\n\
        CS[ellipsoidal,2],\n\
            AXIS[\"latitude\",north,\n\
                ORDER[1],\n\
                ANGLEUNIT[\"degree\",0.0174532925199433]],\n\
            AXIS[\"longitude\",east,\n\
                ORDER[2],\n\
                ANGLEUNIT[\"degree\",0.0174532925199433]]]";

    let crs = parse_derived(wkt);
    assert_eq!(
        crs.export_to_proj_string().expect("proj string"),
        "+proj=ob_tran +o_proj=longlat +o_lat_p=52 +o_lon_p=-30 +lon_0=-25 \
         +datum=WGS84 +no_defs +type=crs"
    );

    if let Crs::Derived(d) = &crs {
        let dirs: Vec<String> = match d.base_crs() {
            Crs::Geographic(g) => g
                .coordinate_system()
                .axes()
                .iter()
                .map(|a| a.direction().to_owned())
                .collect(),
            _ => vec![],
        };
        eprintln!("PROBE base {:?}", dirs);
    }
    let db = DatabaseContext::create().expect("proj.db");
    let base = match &crs {
        Crs::Derived(derived) => derived.base_crs().clone(),
        _ => panic!("not a derived CRS"),
    };
    let op = CoordinateOperationFactory::create()
        .create_operation(&base, &crs)
        .expect("operation")
        .expect("some operation");
    assert_eq!(
        op.export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("proj string"),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=ob_tran \
         +o_proj=longlat +o_lat_p=52 +o_lon_p=-30 +lon_0=-25 +ellps=WGS84 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg +step \
         +proj=axisswap +order=2,1"
    );
}

/// `TEST(crs, derivedGeographicCRS_with_affine_transform_to_PROJ)`
#[test]
fn crs_derived_geographic_crs_with_affine_transform_to_proj() {
    let wkt = "GEODCRS[\"WGS 84 Translated\",\n\
        BASEGEODCRS[\"WGS 84\",\n\
            DATUM[\"World Geodetic System 1984\",\n\
                ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n\
                    LENGTHUNIT[\"metre\",1]]],\n\
            PRIMEM[\"Greenwich\",0]],\n\
        DERIVINGCONVERSION[\"Translation\",\n\
            METHOD[\"Affine parametric transformation\",\n\
                ID[\"EPSG\",9624]],\n\
            PARAMETER[\"A0\",0.5,\n\
                ANGLEUNIT[\"degree\",0.0174532925199433],\n\
                ID[\"EPSG\",8623]],\n\
            PARAMETER[\"A1\",1,\n\
                SCALEUNIT[\"coefficient\",1],\n\
                ID[\"EPSG\",8624]],\n\
            PARAMETER[\"A2\",0,\n\
                SCALEUNIT[\"coefficient\",1],\n\
                ID[\"EPSG\",8625]],\n\
            PARAMETER[\"B0\",2.5,\n\
                ANGLEUNIT[\"degree\",0.0174532925199433],\n\
                ID[\"EPSG\",8639]],\n\
            PARAMETER[\"B1\",0,\n\
                SCALEUNIT[\"coefficient\",1],\n\
                ID[\"EPSG\",8640]],\n\
            PARAMETER[\"B2\",1,\n\
                SCALEUNIT[\"coefficient\",1],\n\
                ID[\"EPSG\",8641]]],\n\
        CS[ellipsoidal,2],\n\
            AXIS[\"latitude\",north],\n\
            AXIS[\"longitude\",east],\n\
            ANGLEUNIT[\"degree\",0.0174532925199433]]";

    let crs = parse_derived(wkt);
    let Crs::Derived(derived) = &crs else {
        panic!("not a derived CRS");
    };
    assert!(derived
        .deriving_conversion()
        .validate_parameters()
        .is_empty());

    let db = DatabaseContext::create().expect("proj.db");
    let base = derived.base_crs().clone();
    let op = CoordinateOperationFactory::create()
        .create_operation(&base, &crs)
        .expect("operation")
        .expect("some operation");
    assert_eq!(
        op.export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("proj string"),
        "+proj=affine +xoff=0.5 +s11=1 +s12=0 +yoff=2.5 +s21=0 +s22=1"
    );
}
