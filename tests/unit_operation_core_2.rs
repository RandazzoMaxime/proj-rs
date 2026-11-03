//! PROJ `test/unit/test_operation.cpp` cases still open after `crs-operation-core`.
//!
//! Each function is one `TEST()` name, in snake case. Expected text is copied from
//! PROJ 9.9.0 unchanged. Multi-line WKT is built with `concat!` and `\n`.

use proj_rs::{
    AlternativeGridNames, AuthorityFactory, AxisOrderReversal, BoundCRS, Conversion,
    CoordinateOperation, CoordinateOperationFactory, Crs, DatabaseContext, EllipsoidalCS,
    GeodeticReferenceFrame, GeographicCRS, OpProps, ProjLines, Transformation, UnitKind,
    UnitOfMeasure, VerticalCRS, VerticalCS, VerticalReferenceFrame, WktConvention, WktFormatter,
    WktParser,
};

fn props() -> OpProps {
    OpProps::new()
}

fn wkt2019() -> WktFormatter {
    WktFormatter::create(WktConvention::Wkt2_2019)
}

fn proj_of(transformation: &Transformation) -> String {
    transformation
        .export_to_proj_string(ProjLines::Single)
        .expect("proj")
}

fn proj_op(operation: &CoordinateOperation) -> String {
    match operation {
        CoordinateOperation::Transformation(transformation) => proj_of(transformation),
        CoordinateOperation::Concatenated(concatenated) => concatenated
            .export_to_proj_string(ProjLines::Single)
            .expect("proj"),
        CoordinateOperation::Conversion(conversion)
            if conversion.source_crs().is_some() && conversion.target_crs().is_some() =>
        {
            let db = DatabaseContext::create().expect("database");
            operation
                .export_to_proj_string_with_database(&db, ProjLines::Single)
                .expect("proj")
        }
        CoordinateOperation::Conversion(conversion) => {
            conversion.export_to_proj_string().expect("proj")
        }
    }
}

fn transformation_wkt(wkt: &str) -> Transformation {
    WktParser::new()
        .create_from_wkt(wkt)
        .expect("wkt")
        .transformation()
        .expect("transformation")
        .clone()
}

fn bound_wkt(wkt: &str) -> BoundCRS {
    WktParser::new()
        .create_from_wkt(wkt)
        .expect("wkt")
        .bound()
        .expect("bound")
        .clone()
}

fn geog(cs: EllipsoidalCS) -> Crs {
    Crs::Geographic(
        GeographicCRS::create("", Some(GeodeticReferenceFrame::epsg_6326()), None, cs)
            .expect("geographic"),
    )
}

fn radian() -> UnitOfMeasure {
    UnitOfMeasure::new("radian", 1.0, UnitKind::Angular)
}

/// `operation.transformation_createTOWGS84`
#[test]
fn operation_transformation_create_towgs84() {
    let empty = Transformation::create_towgs84(Crs::Geographic(GeographicCRS::epsg_4326()), &[]);
    assert_eq!(
        empty.expect_err("empty").to_string(),
        "Invalid number of elements in TOWGS84Parameters"
    );
    let vertical = Crs::Vertical(VerticalCRS::create(
        "ODN height",
        VerticalReferenceFrame::named("Ordnance Datum Newlyn"),
        VerticalCS::create_gravity_related_height(UnitOfMeasure::metre()),
    ));
    let no_geodetic = Transformation::create_towgs84(vertical, &[0.0; 7]);
    assert_eq!(
        no_geodetic.expect_err("vertical").to_string(),
        "Cannot find GeodeticCRS in sourceCRS of TOWGS84 transformation"
    );
}

/// `operation.createAxisOrderReversal`
#[test]
fn operation_create_axis_order_reversal() {
    let two_d = Conversion::create_axis_order_reversal(AxisOrderReversal::TwoD).expect("2d");
    assert!(two_d.validate_parameters().is_empty());
    let three_d =
        Conversion::create_axis_order_reversal(AxisOrderReversal::Geographic3d).expect("3d");
    assert!(three_d.validate_parameters().is_empty());

    let lat_long_deg = geog(EllipsoidalCS::create_latitude_longitude(
        UnitOfMeasure::degree(),
    ));
    let long_lat_deg = geog(EllipsoidalCS::create_longitude_latitude(
        UnitOfMeasure::degree(),
    ));
    let factory = CoordinateOperationFactory::create();
    let swapped = factory
        .create_operation(&lat_long_deg, &long_lat_deg)
        .expect("factory")
        .expect("axis order");
    assert_eq!(proj_op(&swapped), "+proj=axisswap +order=2,1");

    let long_lat_rad = geog(EllipsoidalCS::create_longitude_latitude(radian()));
    let radians = factory
        .create_operation(&long_lat_rad, &lat_long_deg)
        .expect("factory")
        .expect("radians");
    assert_eq!(
        proj_op(&radians),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=rad +xy_out=deg"
    );
}

/// `operation.PROJ_based`
#[test]
fn operation_proj_based() {
    let conv = Conversion::create_proj_based(&props(), "+proj=merc").expect("merc");
    assert_eq!(conv.export_to_proj_string().expect("proj"), "+proj=merc");
    assert_eq!(
        conv.export_to_wkt(&mut wkt2019()).expect("wkt"),
        concat!(
            "CONVERSION[\"PROJ-based coordinate operation\",\n",
            "    METHOD[\"PROJ-based operation method: +proj=merc\"]]"
        )
    );
    assert_eq!(
        conv.inverse().export_to_proj_string().expect("inverse"),
        "+proj=pipeline +step +inv +proj=merc"
    );

    let pipeline = "+proj=pipeline +step +proj=unitconvert +xy_in=grad +xy_out=rad \
                    +step +proj=axisswap +order=2,1 +step +proj=longlat \
                    +ellps=clrk80ign +pm=paris +step +proj=axisswap +order=2,1";
    let pipeline = pipeline.split_whitespace().collect::<Vec<_>>().join(" ");
    assert_eq!(
        Conversion::create_proj_based(&props(), &pipeline)
            .expect("pipeline")
            .export_to_proj_string()
            .expect("proj"),
        pipeline
    );
    assert!(
        Conversion::create_proj_based(&props(), "+proj=pipeline +step +proj=pipeline").is_err()
    );
}

/// `operation.PROJ_based_empty`
#[test]
fn operation_proj_based_empty() {
    let conv = Conversion::create_proj_based(&props(), "").expect("empty");
    assert_eq!(conv.export_to_proj_string().expect("proj"), "+proj=noop");
    assert_eq!(
        conv.export_to_wkt(&mut wkt2019()).expect("wkt"),
        concat!(
            "CONVERSION[\"PROJ-based coordinate operation\",\n",
            "    METHOD[\"PROJ-based operation method: \"]]"
        )
    );
    assert!(conv
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
        .is_err());
    assert_eq!(
        conv.inverse().export_to_proj_string().expect("inverse"),
        "+proj=noop"
    );
}

/// `operation.PROJ_based_with_global_parameters`
#[test]
fn operation_proj_based_with_global_parameters() {
    let conv =
        Conversion::create_proj_based(&props(), "+proj=pipeline +ellps=WGS84 +step +proj=longlat")
            .expect("pipeline");
    assert_eq!(
        conv.export_to_proj_string().expect("proj"),
        "+proj=pipeline +ellps=WGS84 +step +proj=longlat"
    );
}

/// `operation.export_of_Geographic3D_to_GravityRelatedHeight_gtx_unknown_grid`
#[test]
fn operation_export_of_geographic3d_to_gravity_related_height_gtx_unknown_grid() {
    let wkt = concat!(
        "COORDINATEOPERATION[\"bla\",\n",
        "    SOURCECRS[\n",
        "        GEOGCRS[\"ETRS89\",\n",
        "            DATUM[\"European Terrestrial Reference System 1989\",\n",
        "                ELLIPSOID[\"GRS 1980\",6378137,298.257222101,\n",
        "                    LENGTHUNIT[\"metre\",1]]],\n",
        "            PRIMEM[\"Greenwich\",0,\n",
        "                ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "            CS[ellipsoidal,3],\n",
        "                AXIS[\"geodetic latitude (Lat)\",north,\n",
        "                    ORDER[1],\n",
        "                    ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "                AXIS[\"geodetic longitude (Lon)\",east,\n",
        "                    ORDER[2],\n",
        "                    ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "                AXIS[\"ellipsoidal height (h)\",up,\n",
        "                    ORDER[3],\n",
        "                    LENGTHUNIT[\"metre\",1]],\n",
        "            ID[\"EPSG\",4937]]],\n",
        "    TARGETCRS[\n",
        "        VERTCRS[\"bar\",\n",
        "            VDATUM[\"bar\"],\n",
        "            CS[vertical,1],\n",
        "                AXIS[\"gravity-related height (H)\",up,\n",
        "                    LENGTHUNIT[\"metre\",1]]]],\n",
        "    METHOD[\"Geographic3D to GravityRelatedHeight (gtx)\",\n",
        "        ID[\"EPSG\",9665]],\n",
        "    PARAMETERFILE[\"Geoid (height correction) model file\",\"foo.gtx\"]]"
    );
    let transformation = transformation_wkt(wkt);
    assert_eq!(
        proj_of(&transformation),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +inv +proj=vgridshift +grids=foo.gtx +multiplier=1 +step +proj=unitconvert +xy_in=rad +xy_out=deg +step +proj=axisswap +order=2,1"
    );
    assert!(!transformation.requires_per_coordinate_input_time());
}

/// `operation.export_of_boundCRS_with_proj_string_method`
#[test]
fn operation_export_of_bound_crs_with_proj_string_method() {
    let wkt = concat!(
        "BOUNDCRS[\n",
        "    SOURCECRS[\n",
        "        GEOGCRS[\"unknown\",\n",
        "            DATUM[\"Unknown based on GRS80 ellipsoid\",\n",
        "                ELLIPSOID[\"GRS 1980\",6378137,298.257222101,\n",
        "                    LENGTHUNIT[\"metre\",1],\n",
        "                    ID[\"EPSG\",7019]]],\n",
        "            PRIMEM[\"Greenwich\",0,\n",
        "                ANGLEUNIT[\"degree\",0.0174532925199433],\n",
        "                ID[\"EPSG\",8901]],\n",
        "            CS[ellipsoidal,2],\n",
        "                AXIS[\"longitude\",east,\n",
        "                    ORDER[1],\n",
        "                    ANGLEUNIT[\"degree\",0.0174532925199433,\n",
        "                        ID[\"EPSG\",9122]]],\n",
        "                AXIS[\"latitude\",north,\n",
        "                    ORDER[2],\n",
        "                    ANGLEUNIT[\"degree\",0.0174532925199433,\n",
        "                        ID[\"EPSG\",9122]]]]],\n",
        "    TARGETCRS[\n",
        "        GEOGCRS[\"WGS 84\",\n",
        "            DATUM[\"World Geodetic System 1984\",\n",
        "                ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n",
        "                    LENGTHUNIT[\"metre\",1]]],\n",
        "            PRIMEM[\"Greenwich\",0,\n",
        "                ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "            CS[ellipsoidal,2],\n",
        "                AXIS[\"geodetic latitude (Lat)\",north,\n",
        "                    ORDER[1],\n",
        "                    ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "                AXIS[\"geodetic longitude (Lon)\",east,\n",
        "                    ORDER[2],\n",
        "                    ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "            ID[\"EPSG\",4326]]],\n",
        "    ABRIDGEDTRANSFORMATION[\"Transformation from unknown to WGS84\",\n",
        "        METHOD[\"PROJ-based operation method: +proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=axisswap +order=2,1 +step +proj=cart +ellps=GRS80 +step +proj=helmert +convention=coordinate_frame +exact +step +inv +proj=cart +ellps=WGS84 +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=rad +xy_out=deg\"]]]"
    );
    let bound = bound_wkt(wkt);
    assert_eq!(
        proj_op(bound.transformation()),
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=axisswap +order=2,1 +step +proj=cart +ellps=GRS80 +step +proj=helmert +convention=coordinate_frame +exact +step +inv +proj=cart +ellps=WGS84 +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=rad +xy_out=deg"
    );
}

/// `operation.Geographic3DToGravityRelatedHeight`
#[test]
fn operation_geographic3d_to_gravity_related_height() {
    let wkt = concat!(
        "COORDINATEOPERATION[\"test\",\n",
        "    SOURCECRS[\n",
        "        GEOGCRS[\"foo\",\n",
        "            DATUM[\"foo\",\n",
        "                ELLIPSOID[\"International 1924\",6378388,297,\n",
        "                    LENGTHUNIT[\"metre\",1]]],\n",
        "            PRIMEM[\"Greenwich\",0,\n",
        "                ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "            CS[ellipsoidal,3],\n",
        "                AXIS[\"geodetic latitude (Lat)\",north,\n",
        "                    ORDER[1],\n",
        "                    ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "                AXIS[\"geodetic longitude (Lon)\",east,\n",
        "                    ORDER[2],\n",
        "                    ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "                AXIS[\"ellipsoidal height (h)\",up,\n",
        "                    ORDER[3],\n",
        "                    LENGTHUNIT[\"metre\",1]]]],\n",
        "    TARGETCRS[\n",
        "        VERTCRS[\"foo height\",\n",
        "            VDATUM[\"foo Vertical Datum\"],\n",
        "            CS[vertical,1],\n",
        "                AXIS[\"gravity-related height (H)\",up,\n",
        "                    LENGTHUNIT[\"metre\",1]]]],\n",
        "    METHOD[\"Geographic3D to GravityRelatedHeight\",\n",
        "        ID[\"EPSG\",1136]],\n",
        "    PARAMETER[\"Geoid height\",10,\n",
        "        LENGTHUNIT[\"metre\",1],\n",
        "        ID[\"EPSG\",8604]]]"
    );
    let transformation = transformation_wkt(wkt);
    assert_eq!(proj_of(&transformation), "+proj=affine +zoff=-10");
    assert_eq!(
        proj_of(&transformation.inverse()),
        "+proj=pipeline +step +inv +proj=affine +zoff=-10"
    );
}

/// `operation.inverse_of_Geographic3DToGravityRelatedHeight`
#[test]
fn operation_inverse_of_geographic3d_to_gravity_related_height() {
    let wkt = concat!(
        "COORDINATEOPERATION[\"test\",\n",
        "    SOURCECRS[\n",
        "        VERTCRS[\"foo height\",\n",
        "            VDATUM[\"foo Vertical Datum\"],\n",
        "            CS[vertical,1],\n",
        "                AXIS[\"gravity-related height (H)\",up,\n",
        "                    LENGTHUNIT[\"metre\",1]]]],\n",
        "    TARGETCRS[\n",
        "        GEOGCRS[\"foo\",\n",
        "            DATUM[\"foo\",\n",
        "                ELLIPSOID[\"International 1924\",6378388,297,\n",
        "                    LENGTHUNIT[\"metre\",1]]],\n",
        "            PRIMEM[\"Greenwich\",0,\n",
        "                ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "            CS[ellipsoidal,3],\n",
        "                AXIS[\"geodetic latitude (Lat)\",north,\n",
        "                    ORDER[1],\n",
        "                    ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "                AXIS[\"geodetic longitude (Lon)\",east,\n",
        "                    ORDER[2],\n",
        "                    ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "                AXIS[\"ellipsoidal height (h)\",up,\n",
        "                    ORDER[3],\n",
        "                    LENGTHUNIT[\"metre\",1]]]],\n",
        "    METHOD[\"Inverse of Geographic3D to GravityRelatedHeight\",\n",
        "        ID[\"INVERSE(EPSG)\",1136]],\n",
        "    PARAMETER[\"Geoid height\",10,\n",
        "        LENGTHUNIT[\"metre\",1],\n",
        "        ID[\"EPSG\",8604]]]"
    );
    let transformation = transformation_wkt(wkt);
    assert_eq!(proj_of(&transformation), "+proj=affine +zoff=10");
    assert_eq!(
        proj_of(&transformation.inverse()),
        "+proj=pipeline +step +inv +proj=affine +zoff=10"
    );
}

/// `operation.CoordinateFrameRotationFullMatrixGeog2D`
#[test]
fn operation_coordinate_frame_rotation_full_matrix_geog2d() {
    let wkt = concat!(
        "COORDINATEOPERATION[\"Saba to WGS 84 (1)\",\n",
        "    VERSION[\"IOGP-Bes Saba\"],\n",
        "    SOURCECRS[\n",
        "        GEOGCRS[\"Saba\",\n",
        "            DATUM[\"Saba\",\n",
        "                ELLIPSOID[\"International 1924\",6378388,297,\n",
        "                    LENGTHUNIT[\"metre\",1]]],\n",
        "            PRIMEM[\"Greenwich\",0,\n",
        "                ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "            CS[ellipsoidal,2],\n",
        "                AXIS[\"geodetic latitude (Lat)\",north,\n",
        "                    ORDER[1],\n",
        "                    ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "                AXIS[\"geodetic longitude (Lon)\",east,\n",
        "                    ORDER[2],\n",
        "                    ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "            ID[\"EPSG\",10636]]],\n",
        "    TARGETCRS[\n",
        "        GEOGCRS[\"WGS 84\",\n",
        "            ENSEMBLE[\"World Geodetic System 1984 ensemble\",\n",
        "                MEMBER[\"World Geodetic System 1984 (Transit)\"],\n",
        "                MEMBER[\"World Geodetic System 1984 (G730)\"],\n",
        "                MEMBER[\"World Geodetic System 1984 (G873)\"],\n",
        "                MEMBER[\"World Geodetic System 1984 (G1150)\"],\n",
        "                MEMBER[\"World Geodetic System 1984 (G1674)\"],\n",
        "                MEMBER[\"World Geodetic System 1984 (G1762)\"],\n",
        "                MEMBER[\"World Geodetic System 1984 (G2139)\"],\n",
        "                MEMBER[\"World Geodetic System 1984 (G2296)\"],\n",
        "                ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n",
        "                    LENGTHUNIT[\"metre\",1]],\n",
        "                ENSEMBLEACCURACY[2.0]],\n",
        "            PRIMEM[\"Greenwich\",0,\n",
        "                ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "            CS[ellipsoidal,2],\n",
        "                AXIS[\"geodetic latitude (Lat)\",north,\n",
        "                    ORDER[1],\n",
        "                    ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "                AXIS[\"geodetic longitude (Lon)\",east,\n",
        "                    ORDER[2],\n",
        "                    ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "            ID[\"EPSG\",4326]]],\n",
        "    METHOD[\"Coordinate Frame rotation full matrix (geog2D)\",\n",
        "        ID[\"EPSG\",1133]],\n",
        "    PARAMETER[\"X-axis translation\",1138.7432,\n",
        "        LENGTHUNIT[\"metre\",1],\n",
        "        ID[\"EPSG\",8605]],\n",
        "    PARAMETER[\"Y-axis translation\",-2064.4761,\n",
        "        LENGTHUNIT[\"metre\",1],\n",
        "        ID[\"EPSG\",8606]],\n",
        "    PARAMETER[\"Z-axis translation\",110.7016,\n",
        "        LENGTHUNIT[\"metre\",1],\n",
        "        ID[\"EPSG\",8607]],\n",
        "    PARAMETER[\"X-axis rotation\",-214.615206,\n",
        "        ANGLEUNIT[\"arc-second\",4.84813681109536E-06],\n",
        "        ID[\"EPSG\",8608]],\n",
        "    PARAMETER[\"Y-axis rotation\",479.360036,\n",
        "        ANGLEUNIT[\"arc-second\",4.84813681109536E-06],\n",
        "        ID[\"EPSG\",8609]],\n",
        "    PARAMETER[\"Z-axis rotation\",-164.703951,\n",
        "        ANGLEUNIT[\"arc-second\",4.84813681109536E-06],\n",
        "        ID[\"EPSG\",8610]],\n",
        "    PARAMETER[\"Scale difference\",-402.32073,\n",
        "        SCALEUNIT[\"parts per million\",1E-06],\n",
        "        ID[\"EPSG\",8611]],\n",
        "    OPERATIONACCURACY[1.0]]"
    );
    let transformation = transformation_wkt(wkt);
    assert_eq!(
        proj_of(&transformation),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=push +v_3 +step +proj=cart +ellps=intl +step +proj=helmert +exact +x=1138.7432 +y=-2064.4761 +z=110.7016 +rx=-214.615206 +ry=479.360036 +rz=-164.703951 +s=-402.32073 +convention=coordinate_frame +step +inv +proj=cart +ellps=WGS84 +step +proj=pop +v_3 +step +proj=unitconvert +xy_in=rad +xy_out=deg +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        proj_of(&transformation.inverse()),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=push +v_3 +step +proj=cart +ellps=WGS84 +step +inv +proj=helmert +exact +x=1138.7432 +y=-2064.4761 +z=110.7016 +rx=-214.615206 +ry=479.360036 +rz=-164.703951 +s=-402.32073 +convention=coordinate_frame +step +inv +proj=cart +ellps=intl +step +proj=pop +v_3 +step +proj=unitconvert +xy_in=rad +xy_out=deg +step +proj=axisswap +order=2,1"
    );
}

/// `operation.operation_Geog3D_to_Geog2D_GravityRelatedHeight_with_compoundCRS`
#[test]
fn operation_operation_geog3d_to_geog2d_gravity_related_height_with_compound_crs() {
    let db = DatabaseContext::create().expect("database");
    let operation = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_operation("10753", AlternativeGridNames::Official)
        .expect("10753");
    assert_eq!(proj_op(&operation), "+proj=noop");
    assert_eq!(proj_op(&operation.inverse()), "+proj=noop");
}

/// `operation.helmert_between_geog3D_and_compound`
#[test]
fn operation_helmert_between_geog3d_and_compound() {
    let wkt = concat!(
        "COORDINATEOPERATION[\"ETRS89/DREF91/2016 to Asse 2025 + Asse 2025 height (1)\",\n",
        "    VERSION[\"BGE-Deu Asse\"],\n",
        "    SOURCECRS[\n",
        "        GEOGCRS[\"ETRS89/DREF91/2016\",\n",
        "            DATUM[\"ETRS89/DREF91 Realization 2016\",\n",
        "                ELLIPSOID[\"GRS 1980\",6378137,298.257222101,\n",
        "                    LENGTHUNIT[\"metre\",1]]],\n",
        "            PRIMEM[\"Greenwich\",0,\n",
        "                ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "            CS[ellipsoidal,3],\n",
        "                AXIS[\"geodetic latitude (Lat)\",north,\n",
        "                    ORDER[1],\n",
        "                    ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "                AXIS[\"geodetic longitude (Lon)\",east,\n",
        "                    ORDER[2],\n",
        "                    ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "                AXIS[\"ellipsoidal height (h)\",up,\n",
        "                    ORDER[3],\n",
        "                    LENGTHUNIT[\"metre\",1]],\n",
        "            ID[\"EPSG\",10283]]],\n",
        "    TARGETCRS[\n",
        "        COMPOUNDCRS[\"Asse 2025 + Asse 2025 height\",\n",
        "            GEOGCRS[\"Asse 2025\",\n",
        "                DATUM[\"Asse geodetic datum 2025\",\n",
        "                    ELLIPSOID[\"Bessel 1841\",6377397.155,299.1528128,\n",
        "                        LENGTHUNIT[\"metre\",1]]],\n",
        "                PRIMEM[\"Greenwich\",0,\n",
        "                    ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "                CS[ellipsoidal,2],\n",
        "                    AXIS[\"geodetic latitude (Lat)\",north,\n",
        "                        ORDER[1],\n",
        "                        ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "                    AXIS[\"geodetic longitude (Lon)\",east,\n",
        "                        ORDER[2],\n",
        "                        ANGLEUNIT[\"degree\",0.0174532925199433]]],\n",
        "            VERTCRS[\"Asse 2025 height\",\n",
        "                VDATUM[\"Asse vertical datum 2025\"],\n",
        "                CS[vertical,1],\n",
        "                    AXIS[\"gravity-related height (H)\",up,\n",
        "                        LENGTHUNIT[\"metre\",1]]],\n",
        "            ID[\"EPSG\",10904]]],\n",
        "    METHOD[\"Coordinate Frame rotation (geog3D domain)\",\n",
        "        ID[\"EPSG\",1038]],\n",
        "    PARAMETER[\"X-axis translation\",-646.6552,\n",
        "        LENGTHUNIT[\"metre\",1],\n",
        "        ID[\"EPSG\",8605]],\n",
        "    PARAMETER[\"Y-axis translation\",-165.0859,\n",
        "        LENGTHUNIT[\"metre\",1],\n",
        "        ID[\"EPSG\",8606]],\n",
        "    PARAMETER[\"Z-axis translation\",-437.6858,\n",
        "        LENGTHUNIT[\"metre\",1],\n",
        "        ID[\"EPSG\",8607]],\n",
        "    PARAMETER[\"X-axis rotation\",4.77773,\n",
        "        ANGLEUNIT[\"arc-second\",4.84813681109536E-06],\n",
        "        ID[\"EPSG\",8608]],\n",
        "    PARAMETER[\"Y-axis rotation\",-0.39139,\n",
        "        ANGLEUNIT[\"arc-second\",4.84813681109536E-06],\n",
        "        ID[\"EPSG\",8609]],\n",
        "    PARAMETER[\"Z-axis rotation\",-1.07485,\n",
        "        ANGLEUNIT[\"arc-second\",4.84813681109536E-06],\n",
        "        ID[\"EPSG\",8610]],\n",
        "    PARAMETER[\"Scale difference\",2.0025,\n",
        "        SCALEUNIT[\"parts per million\",1E-06],\n",
        "        ID[\"EPSG\",8611]],\n",
        "    OPERATIONACCURACY[0.04],\n",
        "    USAGE[\n",
        "        SCOPE[\"Engineering survey, GIS, topographic mapping.\"],\n",
        "        AREA[\"Germany - Lower Saxony - Asse mining area.\"],\n",
        "        BBOX[52.11,10.6,52.16,10.7]],\n",
        "    ID[\"EPSG\",10905],\n",
        "    REMARK[\"3-dimensional transformation defining the  horizontal and vertical CRSs for the Asse 2 mining area.\"]]"
    );
    let transformation = transformation_wkt(wkt);
    assert_eq!(
        proj_of(&transformation),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m +step +proj=cart +ellps=GRS80 +step +proj=helmert +x=-646.6552 +y=-165.0859 +z=-437.6858 +rx=4.77773 +ry=-0.39139 +rz=-1.07485 +s=2.0025 +convention=coordinate_frame +step +inv +proj=cart +ellps=bessel +step +proj=unitconvert +xy_in=rad +xy_out=deg +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        proj_of(&transformation.inverse()),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=cart +ellps=bessel +step +inv +proj=helmert +x=-646.6552 +y=-165.0859 +z=-437.6858 +rx=4.77773 +ry=-0.39139 +rz=-1.07485 +s=2.0025 +convention=coordinate_frame +step +inv +proj=cart +ellps=GRS80 +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m +step +proj=axisswap +order=2,1"
    );
}

/// `operation.operation_Geographic2D_Offsets_by_TIN_Interpolation_JSON`
#[test]
fn operation_operation_geographic2d_offsets_by_tin_interpolation_json() {
    let db = DatabaseContext::create().expect("database");
    let operation = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_operation("10854", AlternativeGridNames::Official)
        .expect("10854");
    assert_eq!(
        proj_op(&operation),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=tinshift +file=no_kv_ETRS89NO_NGO48_TIN.json +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        proj_op(&operation.inverse()),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +inv +proj=tinshift +file=no_kv_ETRS89NO_NGO48_TIN.json +step +proj=axisswap +order=2,1"
    );
}
