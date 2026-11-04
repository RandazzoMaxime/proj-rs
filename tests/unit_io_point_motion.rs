//! PROJ 9.9.0 `test/unit/test_operation.cpp`, a point motion operation with epochs read from
//! WKT.

use proj_rs::{ProjLines, WktObject, WktParser};

/// `TEST(operation, PointMotionOperation_with_epochs)`
#[test]
fn operation_point_motion_operation_with_epochs() {
    let wkt = r#"POINTMOTIONOPERATION["Canada velocity grid v7 from epoch 2010 to epoch 2002",
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
    PARAMETERFILE["Point motion velocity grid file","ca_nrc_NAD83v70VG.tif"],
    OPERATIONACCURACY[0.01],
    ID["DERIVED_FROM(EPSG)",9483],
    REMARK["File initially published with name cvg70.cvb, later renamed to NAD83v70VG.gvb with no change of content."]]"#;
    let WktObject::Transformation(pmo) = WktParser::new().create_from_wkt(wkt).expect("wkt") else {
        panic!("expected a point motion operation");
    };
    assert_eq!(
        pmo.export_to_proj_string(ProjLines::Single).expect("export"),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m +step +proj=cart +ellps=GRS80 +step +proj=set +v_4=2010 +omit_fwd +step +proj=deformation +dt=-8 +grids=ca_nrc_NAD83v70VG.tif +ellps=GRS80 +step +proj=set +v_4=2002 +omit_inv +step +inv +proj=cart +ellps=GRS80 +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m +step +proj=axisswap +order=2,1"
    );
    let inverse = pmo.inverse();
    assert_eq!(
        inverse.name(),
        "Canada velocity grid v7 from epoch 2002 to epoch 2010"
    );
    assert_eq!(
        inverse.export_to_proj_string(ProjLines::Single).expect("export"),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m +step +proj=cart +ellps=GRS80 +step +proj=set +v_4=2002 +omit_fwd +step +proj=deformation +dt=8 +grids=ca_nrc_NAD83v70VG.tif +ellps=GRS80 +step +proj=set +v_4=2010 +omit_inv +step +inv +proj=cart +ellps=GRS80 +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m +step +proj=axisswap +order=2,1"
    );
}
