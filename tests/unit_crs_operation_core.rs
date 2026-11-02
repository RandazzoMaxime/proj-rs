//! PROJ `test/unit/test_crs.cpp` and `test/unit/test_operation.cpp`: the `core` class of
//! `docs/UNIT-TRIAGE.md` (WKT, PROJ strings, identification, operations a user sees).
//!
//! Each function is one `TEST()` name, in C++ spelling. Expected text is copied from
//! PROJ 9.9.0 unchanged. Multi-line WKT is built with `concat!` and `\n`.

use proj_rs::{
    create_crs_from_proj_string, CartesianCS, ConcatenatedOperation, Conversion,
    CoordinateOperation, CoordinateOperationFactory, Crs, DatabaseContext, GeodeticCRS,
    GeographicCRS, OpProps, OperationMethod, OperationParameter, ParameterValue, ProjLines,
    ProjectedCRS, Transformation, UnitOfMeasure, WktConvention, WktFormatter, WktParser,
    WktStrictness,
};

fn props() -> OpProps {
    OpProps::new()
}

fn g4326() -> Crs {
    Crs::Geographic(GeographicCRS::epsg_4326())
}

fn g4269() -> Crs {
    Crs::Geographic(GeographicCRS::epsg_4269())
}

fn g4807() -> Crs {
    Crs::Geographic(GeographicCRS::epsg_4807())
}

fn g4979() -> Crs {
    Crs::Geographic(GeographicCRS::epsg_4979())
}

fn strip(text: &str) -> String {
    text.replace([' ', '\n'], "")
}

fn wkt2015() -> WktFormatter {
    WktFormatter::create(WktConvention::Wkt2_2015)
}

fn wkt2019() -> WktFormatter {
    WktFormatter::create(WktConvention::Wkt2_2019)
}

fn wkt_no_id(crs: &Crs) -> String {
    let mut formatter = wkt2015();
    formatter.hide_identifiers().expect("hide");
    crs.export_to_wkt(&mut formatter).expect("wkt")
}

fn wkt2019_of(crs: &Crs) -> String {
    crs.export_to_wkt(&mut wkt2019()).expect("wkt")
}

fn proj_of(transformation: &Transformation) -> String {
    transformation
        .export_to_proj_string(ProjLines::Single)
        .expect("proj")
}

fn filename_transformation(source: Crs, target: Crs, interpolation: Option<Crs>) -> Transformation {
    let method = OperationMethod::create(
        "operationMethodName",
        vec![OperationParameter::create("paramName")],
    )
    .with_identifier("codeSpaceOperationMethod", "codeOperationMethod");
    Transformation::create_with_operation_method(
        &OpProps::new()
            .identifier("codeSpaceTransformation", "codeTransformation")
            .name("transformationName")
            .remarks("my remarks"),
        source,
        target,
        interpolation,
        method,
        vec![ParameterValue::filename("foo.bin")],
        Some(0.1),
    )
    .expect("transformation")
}

#[test]
fn operation_transformation_to_wkt() {
    let transf = filename_transformation(g4326(), g4807(), Some(g4979()));
    let expected = format!(
        concat!(
            "COORDINATEOPERATION[\"transformationName\",\n",
            "    SOURCECRS[{}],\n",
            "    TARGETCRS[{}],\n",
            "    METHOD[\"operationMethodName\",\n",
            "        ID[\"codeSpaceOperationMethod\",\"codeOperationMethod\"]],\n",
            "    PARAMETERFILE[\"paramName\",\"foo.bin\"],\n",
            "    INTERPOLATIONCRS[{}],\n",
            "    OPERATIONACCURACY[0.1],\n",
            "    ID[\"codeSpaceTransformation\",\"codeTransformation\"],\n",
            "    REMARK[\"my remarks\"]]"
        ),
        wkt_no_id(&g4326()),
        wkt_no_id(&g4807()),
        wkt_no_id(&g4979())
    );
    assert_eq!(
        strip(&transf.export_to_wkt(&mut wkt2015()).expect("wkt")),
        strip(&expected)
    );
    assert!(transf
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
        .is_err());
    assert!(transf.is_equivalent_to(&transf));
}

#[test]
fn operation_concatenated_operation() {
    let transf_1 = Transformation::create(
        &OpProps::new()
            .identifier("codeSpaceTransformation", "codeTransformation")
            .name("transformationName")
            .remarks("my remarks"),
        g4326(),
        g4807(),
        None,
        "operationMethodName",
        vec![OperationParameter::create("paramName")],
        vec![ParameterValue::filename("foo.bin")],
        None,
    )
    .expect("transf_1");
    let transf_2 = Transformation::create(
        &OpProps::new()
            .identifier("codeSpaceTransformation", "codeTransformation")
            .name("transformationName")
            .remarks("my remarks"),
        g4807(),
        g4979(),
        None,
        "operationMethodName",
        vec![OperationParameter::create("paramName")],
        vec![ParameterValue::filename("foo.bin")],
        None,
    )
    .expect("transf_2");
    let step = |transf: &Transformation| CoordinateOperation::Transformation(transf.clone());

    let concat = ConcatenatedOperation::create(
        &OpProps::new()
            .identifier("codeSpace", "code")
            .name("name")
            .remarks("my remarks"),
        vec![step(&transf_1), step(&transf_2)],
        Some(0.1),
    )
    .expect("concat");

    let expected = format!(
        concat!(
            "CONCATENATEDOPERATION[\"name\",\n",
            "    SOURCECRS[{}],\n",
            "    TARGETCRS[{}],\n",
            "    STEP[{}],\n",
            "    STEP[{}],\n",
            "    OPERATIONACCURACY[0.1],\n",
            "    ID[\"codeSpace\",\"code\"],\n",
            "    REMARK[\"my remarks\"]]"
        ),
        wkt2019_of(&g4326()),
        wkt2019_of(&g4979()),
        transf_1.export_to_wkt(&mut wkt2019()).expect("step 1"),
        transf_2.export_to_wkt(&mut wkt2019()).expect("step 2"),
    );
    assert_eq!(
        strip(&concat.export_to_wkt(&mut wkt2019()).expect("wkt")),
        strip(&expected)
    );
    assert!(concat.export_to_wkt(&mut wkt2015()).is_err());

    assert!(ConcatenatedOperation::create(
        &OpProps::new().name("name"),
        vec![step(&transf_1), step(&transf_1)],
        None
    )
    .is_err());

    let inv = concat.inverse();
    assert_eq!(inv.name(), "Inverse of name");
    assert_eq!(
        inv.source_crs().map(Crs::name),
        concat.target_crs().map(Crs::name)
    );
    assert_eq!(
        inv.target_crs().map(Crs::name),
        concat.source_crs().map(Crs::name)
    );
    assert_eq!(inv.steps().len(), 2);
    assert_eq!(
        inv.steps()[0].operation().name(),
        "Inverse of transformationName"
    );
    assert_eq!(
        inv.steps()[1].operation().name(),
        "Inverse of transformationName"
    );

    assert!(concat.is_equivalent_to(&concat));
    assert!(concat.is_equivalent_to(&concat.clone()));
    let chain = |steps: Vec<CoordinateOperation>| {
        ConcatenatedOperation::create(&props(), steps, None).expect("chain")
    };
    let inverse_1 = CoordinateOperation::Transformation(transf_1.inverse());
    assert!(!chain(vec![step(&transf_1), inverse_1.clone()])
        .is_equivalent_to(&chain(vec![inverse_1.clone(), step(&transf_1)])));
    assert!(
        !chain(vec![step(&transf_1), inverse_1.clone()]).is_equivalent_to(&chain(vec![
            step(&transf_1),
            inverse_1,
            step(&transf_1)
        ]))
    );
}

const TIME_DEPENDENT_VALUES: ([f64; 7], [f64; 7], f64) = (
    [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
    [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7],
    2018.5,
);

fn assert_time_dependent_wkt(wkt: &str, method: &str, code: &str) {
    assert!(
        wkt.contains("Transformation from WGS 84 to NAD83 (approx. inversion)"),
        "{wkt}"
    );
    assert!(wkt.contains(method), "{wkt}");
    assert!(wkt.contains(&format!("ID[\"EPSG\",{code}]]")), "{wkt}");
    for needle in [
        "\"X-axis translation\",-1",
        "\"Y-axis translation\",-2",
        "\"Z-axis translation\",-3",
        "\"X-axis rotation\",-4",
        "\"Y-axis rotation\",-5",
        "\"Z-axis rotation\",-6",
        "\"Scale difference\",-7",
        "\"Rate of change of X-axis translation\",-0.1",
        "\"Rate of change of Y-axis translation\",-0.2",
        "\"Rate of change of Z-axis translation\",-0.3",
        "\"Rate of change of X-axis rotation\",-0.4",
        "\"Rate of change of Y-axis rotation\",-0.5",
        "\"Rate of change of Z-axis rotation\",-0.6",
        "\"Rate of change of Scale difference\",-0.7",
        "\"Parameter reference epoch\",2018.5",
    ] {
        assert!(wkt.contains(needle), "{needle} in {wkt}");
    }
}

#[test]
fn operation_transformation_create_time_dependent_position_vector() {
    let (helmert, rates, epoch) = TIME_DEPENDENT_VALUES;
    let transf = Transformation::create_time_dependent_position_vector(
        &props(),
        g4269(),
        g4326(),
        helmert,
        rates,
        epoch,
        None,
    )
    .expect("transf");
    assert!(transf.validate_parameters().is_empty());
    assert!(transf.requires_per_coordinate_input_time());
    let inv = transf.inverse();
    assert_eq!(
        transf.source_crs().map(Crs::name),
        inv.target_crs().map(Crs::name)
    );
    assert_eq!(
        transf.target_crs().map(Crs::name),
        inv.source_crs().map(Crs::name)
    );
    assert!(inv.requires_per_coordinate_input_time());
    let proj = proj_of(&inv);
    assert!(
        proj.contains(
            "+proj=helmert +x=1 +y=2 +z=3 +rx=4 +ry=5 +rz=6 +s=7 +dx=0.1 +dy=0.2 +dz=0.3 \
             +drx=0.4 +dry=0.5 +drz=0.6 +ds=0.7 +t_epoch=2018.5 +convention=position_vector"
        ),
        "{proj}"
    );
    let wkt = inv.export_to_wkt(&mut wkt2015()).expect("wkt");
    assert_time_dependent_wkt(&wkt, "Time-dependent Position Vector tfm (geog2D)", "1054");
}

#[test]
fn operation_transformation_create_time_dependent_coordinate_frame_rotation() {
    let (helmert, rates, epoch) = TIME_DEPENDENT_VALUES;
    let transf = Transformation::create_time_dependent_coordinate_frame_rotation(
        &props(),
        g4269(),
        g4326(),
        helmert,
        rates,
        epoch,
        None,
    )
    .expect("transf");
    assert!(transf.validate_parameters().is_empty());
    let inv = transf.inverse();
    assert_eq!(
        transf.source_crs().map(Crs::name),
        inv.target_crs().map(Crs::name)
    );
    assert_eq!(
        transf.target_crs().map(Crs::name),
        inv.source_crs().map(Crs::name)
    );
    let proj = proj_of(&inv);
    assert!(
        proj.contains(
            "+proj=helmert +x=1 +y=2 +z=3 +rx=4 +ry=5 +rz=6 +s=7 +dx=0.1 +dy=0.2 +dz=0.3 \
             +drx=0.4 +dry=0.5 +drz=0.6 +ds=0.7 +t_epoch=2018.5 +convention=coordinate_frame"
        ),
        "{proj}"
    );
    let wkt = inv.export_to_wkt(&mut wkt2015()).expect("wkt");
    assert_time_dependent_wkt(
        &wkt,
        "Time-dependent Coordinate Frame rotation (geog2D)",
        "1057",
    );
}

fn position_vector(source: Crs, target: Crs, values: [f64; 7]) -> CoordinateOperation {
    CoordinateOperation::Transformation(
        Transformation::create_position_vector(&props(), source, target, values, None)
            .expect("position vector"),
    )
}

fn chain(steps: Vec<CoordinateOperation>) -> ConcatenatedOperation {
    ConcatenatedOperation::create(&props(), steps, None).expect("concatenated")
}

fn chain_proj(concat: &ConcatenatedOperation) -> String {
    concat
        .export_to_proj_string(ProjLines::Single)
        .expect("proj")
}

#[test]
fn operation_transformation_successive_helmert_noop() {
    let concat = chain(vec![
        position_vector(g4326(), g4269(), [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]),
        position_vector(g4269(), g4326(), [-1.0, -2.0, -3.0, -4.0, -5.0, -6.0, -7.0]),
    ]);
    assert_eq!(chain_proj(&concat), "+proj=noop");
}

#[test]
fn operation_transformation_successive_helmert_non_trivial_1() {
    let concat = chain(vec![
        position_vector(g4326(), g4269(), [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]),
        position_vector(g4269(), g4326(), [-1.0, -2.0, -3.0, -4.0, -5.0, -6.0, 7.0]),
    ]);
    assert_ne!(chain_proj(&concat), "");
}

#[test]
fn operation_transformation_successive_helmert_non_trivial_2() {
    let transf_2 = Transformation::create_coordinate_frame_rotation(
        &props(),
        g4269(),
        g4326(),
        [-1.0, -2.0, -3.0, -4.0, -5.0, -6.0, -7.0],
        None,
    )
    .expect("cf");
    let concat = chain(vec![
        position_vector(g4326(), g4269(), [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]),
        CoordinateOperation::Transformation(transf_2),
    ]);
    assert_ne!(chain_proj(&concat), "");
}

#[test]
fn operation_transformation_create_molodensky() {
    let transf = Transformation::create_molodensky(
        &props(),
        g4326(),
        g4269(),
        [1.0, 2.0, 3.0, 4.0, 5.0],
        None,
    )
    .expect("molodensky");
    assert!(transf.validate_parameters().is_empty());
    let wkt = transf.export_to_wkt(&mut wkt2015()).expect("wkt");
    assert!(
        strip(&wkt).contains("METHOD[\"Molodensky\",ID[\"EPSG\",9604]]"),
        "{wkt}"
    );
    let inv = transf.inverse();
    assert_eq!(
        transf.source_crs().map(Crs::name),
        inv.target_crs().map(Crs::name)
    );
    assert_eq!(
        proj_of(&inv),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert \
         +xy_in=deg +xy_out=rad +step +proj=molodensky +ellps=GRS80 +dx=-1 +dy=-2 \
         +dz=-3 +da=-4 +df=-5 +step +proj=unitconvert +xy_in=rad +xy_out=deg +step \
         +proj=axisswap +order=2,1"
    );
}

#[test]
fn operation_transformation_create_abridged_molodensky() {
    let transf = Transformation::create_abridged_molodensky(
        &props(),
        g4326(),
        g4269(),
        [1.0, 2.0, 3.0, 4.0, 5.0],
        None,
    )
    .expect("abridged");
    let wkt = transf.export_to_wkt(&mut wkt2015()).expect("wkt");
    assert!(
        strip(&wkt).contains(&strip("METHOD[\"Abridged Molodensky\",ID[\"EPSG\",9605]]")),
        "{wkt}"
    );
    let inv = transf.inverse();
    assert_eq!(
        transf.target_crs().map(Crs::name),
        inv.source_crs().map(Crs::name)
    );
    assert_eq!(
        proj_of(&inv),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert \
         +xy_in=deg +xy_out=rad +step +proj=molodensky +ellps=GRS80 +dx=-1 +dy=-2 \
         +dz=-3 +da=-4 +df=-5 +abridged +step +proj=unitconvert +xy_in=rad \
         +xy_out=deg +step +proj=axisswap +order=2,1"
    );
}

#[test]
fn operation_transformation_inverse() {
    let method = OperationMethod::create(
        "my operation",
        vec![OperationParameter::create("paramName")],
    )
    .with_identifier("my codeSpace", "my code");
    let transf = Transformation::create_with_operation_method(
        &OpProps::new()
            .name("my transformation")
            .identifier("my codeSpace", "my code"),
        g4326(),
        g4269(),
        None,
        method,
        vec![ParameterValue::filename("foo.bin")],
        Some(0.1),
    )
    .expect("transf");
    let inv = transf.inverse();
    assert_eq!(inv.inverse().name(), transf.name());
    assert_eq!(
        inv.export_to_wkt(&mut wkt2015()).expect("wkt"),
        concat!(
            "COORDINATEOPERATION[\"Inverse of my transformation\",\n",
            "    SOURCECRS[\n",
            "        GEODCRS[\"NAD83\",\n",
            "            DATUM[\"North American Datum 1983\",\n",
            "                ELLIPSOID[\"GRS 1980\",6378137,298.257222101,\n",
            "                    LENGTHUNIT[\"metre\",1]]],\n",
            "            PRIMEM[\"Greenwich\",0,\n",
            "                ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
            "            CS[ellipsoidal,2],\n",
            "                AXIS[\"latitude\",north,\n",
            "                    ORDER[1],\n",
            "                    ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
            "                AXIS[\"longitude\",east,\n",
            "                    ORDER[2],\n",
            "                    ANGLEUNIT[\"degree\",0.0174532925199433]]]],\n",
            "    TARGETCRS[\n",
            "        GEODCRS[\"WGS 84\",\n",
            "            DATUM[\"World Geodetic System 1984\",\n",
            "                ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n",
            "                    LENGTHUNIT[\"metre\",1]]],\n",
            "            PRIMEM[\"Greenwich\",0,\n",
            "                ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
            "            CS[ellipsoidal,2],\n",
            "                AXIS[\"latitude\",north,\n",
            "                    ORDER[1],\n",
            "                    ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
            "                AXIS[\"longitude\",east,\n",
            "                    ORDER[2],\n",
            "                    ANGLEUNIT[\"degree\",0.0174532925199433]]]],\n",
            "    METHOD[\"Inverse of my operation\",\n",
            "        ID[\"INVERSE(my codeSpace)\",\"my code\"]],\n",
            "    PARAMETERFILE[\"paramName\",\"foo.bin\"],\n",
            "    OPERATIONACCURACY[0.1],\n",
            "    ID[\"INVERSE(my codeSpace)\",\"my code\"]]"
        )
    );
    assert!(inv.export_to_proj_string(ProjLines::Single).is_err());
}

fn conv_wkt2(conv: &Conversion) -> String {
    conv.export_to_wkt(&mut wkt2015()).expect("wkt")
}

fn projected_wkt(crs: &ProjectedCRS, convention: WktConvention) -> String {
    Crs::Projected(crs.clone())
        .export_to_wkt(&mut WktFormatter::create(convention))
        .expect("wkt")
}

fn parse_projected(wkt: &str) -> ProjectedCRS {
    WktParser::new()
        .create_from_wkt(wkt)
        .expect("wkt")
        .projected()
        .expect("projected")
        .clone()
}

fn wgs84_projected(name: &str, conv: Conversion) -> ProjectedCRS {
    ProjectedCRS::create(
        name,
        GeodeticCRS::Geographic(GeographicCRS::epsg_4326()),
        conv,
        CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    )
}

#[test]
fn operation_equidistant_cylindrical_lat_0() {
    let crs = create_crs_from_proj_string(
        "+proj=eqc +ellps=sphere +lat_0=-2 +lat_ts=1 +lon_0=-10 +type=crs",
    )
    .expect("crs");
    let Crs::Projected(crs) = crs else {
        panic!("not a projected CRS");
    };
    let wkt = projected_wkt(&crs, WktConvention::Wkt2_2015);
    assert!(
        wkt.contains("PARAMETER[\"Latitude of natural origin\",-2"),
        "{wkt}"
    );
    assert_eq!(
        crs.export_to_proj_string().expect("proj"),
        "+proj=eqc +lat_ts=1 +lat_0=-2 +lon_0=-10 +x_0=0 +y_0=0 +ellps=sphere \
         +units=m +no_defs +type=crs"
    );
}

#[test]
fn operation_geostationary_satellite_sweep_x_export() {
    let conv = Conversion::create_geostationary_satellite_sweep_x(&props(), 1.0, 2.0, 3.0, 4.0)
        .expect("geos");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(
        conv.export_to_proj_string().expect("proj"),
        "+proj=geos +sweep=x +lon_0=1 +h=2 +x_0=3 +y_0=4"
    );
    assert_eq!(
        conv_wkt2(&conv),
        concat!(
            "CONVERSION[\"Geostationary Satellite (Sweep X)\",\n",
            "    METHOD[\"Geostationary Satellite (Sweep X)\"],\n",
            "    PARAMETER[\"Longitude of natural origin\",1,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
            "        ID[\"EPSG\",8802]],\n",
            "    PARAMETER[\"Satellite Height\",2,\n",
            "        LENGTHUNIT[\"metre\",1,\n",
            "            ID[\"EPSG\",9001]]],\n",
            "    PARAMETER[\"False easting\",3,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8806]],\n",
            "    PARAMETER[\"False northing\",4,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8807]]]"
        )
    );
    let crs = wgs84_projected("", conv);
    let wkt1 = projected_wkt(&crs, WktConvention::Wkt1Gdal);
    assert!(
        wkt1.contains("PROJECTION[\"Geostationary_Satellite\"]"),
        "{wkt1}"
    );
    assert!(
        wkt1.contains(
            "EXTENSION[\"PROJ4\",\"+proj=geos +sweep=x +lon_0=1 +h=2 +x_0=3 +y_0=4 \
             +datum=WGS84 +units=m +no_defs\"]]"
        ),
        "{wkt1}"
    );
}

const WEBMERC_DEFAULT: &str = concat!(
    "CONVERSION[\"unknown\",\n",
    "    METHOD[\"Popular Visualisation Pseudo Mercator\",\n",
    "        ID[\"EPSG\",1024]],\n",
    "    PARAMETER[\"Latitude of natural origin\",0,\n",
    "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
    "        ID[\"EPSG\",8801]],\n",
    "    PARAMETER[\"Longitude of natural origin\",0,\n",
    "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
    "        ID[\"EPSG\",8802]],\n",
    "    PARAMETER[\"False easting\",0,\n",
    "        LENGTHUNIT[\"metre\",1],\n",
    "        ID[\"EPSG\",8806]],\n",
    "    PARAMETER[\"False northing\",0,\n",
    "        LENGTHUNIT[\"metre\",1],\n",
    "        ID[\"EPSG\",8807]]]"
);

const WEBMERC_PROJ: &str = "+proj=merc +a=6378137 +b=6378137 +lat_ts=0 +lon_0=0 +x_0=0 \
                            +y_0=0 +k=1 +units=m +nadgrids=@null +wktext +no_defs +type=crs";

#[test]
fn operation_webmerc_export() {
    let conv =
        Conversion::create_popular_visualisation_pseudo_mercator(&props(), 0.0, 2.0, 3.0, 4.0)
            .expect("webmerc");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(
        conv.export_to_proj_string().expect("proj"),
        "+proj=webmerc +lat_0=0 +lon_0=2 +x_0=3 +y_0=4"
    );
    assert_eq!(
        conv_wkt2(&conv),
        concat!(
            "CONVERSION[\"Popular Visualisation Pseudo Mercator\",\n",
            "    METHOD[\"Popular Visualisation Pseudo Mercator\",\n",
            "        ID[\"EPSG\",1024]],\n",
            "    PARAMETER[\"Latitude of natural origin\",0,\n",
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
    let proj_crs = wgs84_projected("Pseudo-Mercator", conv);
    assert_eq!(
        projected_wkt(&proj_crs, WktConvention::Wkt1Gdal),
        concat!(
            "PROJCS[\"Pseudo-Mercator\",\n",
            "    GEOGCS[\"WGS 84\",\n",
            "        DATUM[\"WGS_1984\",\n",
            "            SPHEROID[\"WGS 84\",6378137,298.257223563,\n",
            "                AUTHORITY[\"EPSG\",\"7030\"]],\n",
            "            AUTHORITY[\"EPSG\",\"6326\"]],\n",
            "        PRIMEM[\"Greenwich\",0,\n",
            "            AUTHORITY[\"EPSG\",\"8901\"]],\n",
            "        UNIT[\"degree\",0.0174532925199433,\n",
            "            AUTHORITY[\"EPSG\",\"9122\"]],\n",
            "        AUTHORITY[\"EPSG\",\"4326\"]],\n",
            "    PROJECTION[\"Mercator_1SP\"],\n",
            "    PARAMETER[\"central_meridian\",2],\n",
            "    PARAMETER[\"scale_factor\",1],\n",
            "    PARAMETER[\"false_easting\",3],\n",
            "    PARAMETER[\"false_northing\",4],\n",
            "    UNIT[\"metre\",1,\n",
            "        AUTHORITY[\"EPSG\",\"9001\"]],\n",
            "    AXIS[\"Easting\",EAST],\n",
            "    AXIS[\"Northing\",NORTH],\n",
            "    EXTENSION[\"PROJ4\",\"+proj=merc +a=6378137 +b=6378137 +lat_ts=0 +lon_0=2 \
             +x_0=3 +y_0=4 +k=1 +units=m +nadgrids=@null +wktext +no_defs\"]]"
        )
    );
    let op = CoordinateOperationFactory::create()
        .create_operation_proj_string(&g4326(), &Crs::Projected(proj_crs.clone()))
        .expect("operation");
    assert_eq!(
        op,
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert \
         +xy_in=deg +xy_out=rad +step +proj=webmerc +lat_0=0 +lon_0=2 +x_0=3 +y_0=4 \
         +ellps=WGS84"
    );
    assert_eq!(
        proj_crs.export_to_proj_string().expect("proj"),
        "+proj=merc +a=6378137 +b=6378137 +lat_ts=0 +lon_0=2 +x_0=3 +y_0=4 +k=1 \
         +units=m +nadgrids=@null +wktext +no_defs +type=crs"
    );
}

#[test]
fn operation_webmerc_import_from_gdal_wkt1() {
    let conv =
        Conversion::create_popular_visualisation_pseudo_mercator(&props(), 0.0, 0.0, 0.0, 0.0)
            .expect("webmerc");
    let proj_crs = wgs84_projected("Pseudo-Mercator", conv);
    let wkt1 = projected_wkt(&proj_crs, WktConvention::Wkt1Gdal);
    let crs = parse_projected(&wkt1);
    assert_eq!(conv_wkt2(crs.deriving_conversion()), WEBMERC_DEFAULT);
}

#[test]
fn operation_webmerc_import_from_gdal_wkt1_with_epsg_code() {
    let conv =
        Conversion::create_popular_visualisation_pseudo_mercator(&props(), 0.0, 0.0, 0.0, 0.0)
            .expect("webmerc");
    let proj_crs = wgs84_projected("Pseudo-Mercator", conv).with_identifier("EPSG", "3857");
    let wkt1 = projected_wkt(&proj_crs, WktConvention::Wkt1Gdal);
    assert!(wkt1.contains("3857"), "{wkt1}");
    let crs = parse_projected(&wkt1);
    assert_eq!(crs.identifiers().len(), 1);
}

#[test]
fn operation_webmerc_import_from_gdal_wkt1_epsg_3785_deprecated() {
    let wkt1 = concat!(
        "PROJCS[\"Popular Visualisation CRS / Mercator (deprecated)\",",
        "    GEOGCS[\"Popular Visualisation CRS\",",
        "        DATUM[\"Popular_Visualisation_Datum\",",
        "            SPHEROID[\"Popular Visualisation Sphere\",6378137,0,",
        "                AUTHORITY[\"EPSG\",\"7059\"]],",
        "            TOWGS84[0,0,0,0,0,0,0],",
        "            AUTHORITY[\"EPSG\",\"6055\"]],",
        "        PRIMEM[\"Greenwich\",0,",
        "            AUTHORITY[\"EPSG\",\"8901\"]],",
        "        UNIT[\"degree\",0.0174532925199433,",
        "            AUTHORITY[\"EPSG\",\"9122\"]],",
        "        AUTHORITY[\"EPSG\",\"4055\"]],",
        "    PROJECTION[\"Mercator_1SP\"],",
        "    PARAMETER[\"central_meridian\",0],",
        "    PARAMETER[\"scale_factor\",1],",
        "    PARAMETER[\"false_easting\",0],",
        "    PARAMETER[\"false_northing\",0],",
        "    UNIT[\"metre\",1,",
        "        AUTHORITY[\"EPSG\",\"9001\"]],",
        "    AXIS[\"X\",EAST],",
        "    AXIS[\"Y\",NORTH],",
        "    EXTENSION[\"PROJ4\",\"+proj=merc +a=6378137 +b=6378137 ",
        "+lat_ts=0.0 +lon_0=0.0 +x_0=0.0 +y_0=0 +k=1.0 +units=m ",
        "+nadgrids=@null +wktext  +no_defs\"]]"
    );
    let crs = parse_projected(wkt1);
    assert_eq!(crs.export_to_proj_string().expect("proj"), WEBMERC_PROJ);
    assert_eq!(conv_wkt2(crs.deriving_conversion()), WEBMERC_DEFAULT);
}

const WEBMERC_WKT2_3785: &str = concat!(
    "PROJCRS[\"Popular Visualisation CRS / Mercator\",\n",
    "    BASEGEODCRS[\"Popular Visualisation CRS\",\n",
    "        DATUM[\"Popular Visualisation Datum\",\n",
    "            ELLIPSOID[\"Popular Visualisation Sphere\",6378137,0,\n",
    "                LENGTHUNIT[\"metre\",1]]],\n",
    "        PRIMEM[\"Greenwich\",0,\n",
    "            ANGLEUNIT[\"degree\",0.0174532925199433]]],\n",
    "    CONVERSION[\"Popular Visualisation Mercator\",\n",
    "        METHOD[\"Mercator (1SP) (Spherical)\",\n",
    "            ID[\"EPSG\",9841]],\n",
    "        PARAMETER[\"Latitude of natural origin\",0,\n",
    "            ANGLEUNIT[\"degree\",0.0174532925199433],\n",
    "            ID[\"EPSG\",8801]],\n",
    "        PARAMETER[\"Longitude of natural origin\",0,\n",
    "            ANGLEUNIT[\"degree\",0.0174532925199433],\n",
    "            ID[\"EPSG\",8802]],\n",
    "        PARAMETER[\"Scale factor at natural origin\",1,\n",
    "            SCALEUNIT[\"unity\",1],\n",
    "            ID[\"EPSG\",8805]],\n",
    "        PARAMETER[\"False easting\",0,\n",
    "            LENGTHUNIT[\"metre\",1],\n",
    "            ID[\"EPSG\",8806]],\n",
    "        PARAMETER[\"False northing\",0,\n",
    "            LENGTHUNIT[\"metre\",1],\n",
    "            ID[\"EPSG\",8807]]],\n",
    "    CS[Cartesian,2],\n",
    "        AXIS[\"easting (X)\",east,\n",
    "            ORDER[1],\n",
    "            LENGTHUNIT[\"metre\",1]],\n",
    "        AXIS[\"northing (Y)\",north,\n",
    "            ORDER[2],\n",
    "            LENGTHUNIT[\"metre\",1]]]"
);

#[test]
fn operation_webmerc_import_from_wkt2_epsg_3785_deprecated() {
    let crs = parse_projected(WEBMERC_WKT2_3785);
    assert_eq!(crs.export_to_proj_string().expect("proj"), WEBMERC_PROJ);
    assert_eq!(
        projected_wkt(&crs, WktConvention::Wkt2_2015),
        WEBMERC_WKT2_3785
    );
    let db = DatabaseContext::create().expect("db");
    let wkt1 = Crs::Projected(crs)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal).with_database(db))
        .expect("wkt1");
    assert_eq!(
        wkt1,
        concat!(
            "PROJCS[\"Popular Visualisation CRS / Mercator\",\n",
            "    GEOGCS[\"Popular Visualisation CRS\",\n",
            "        DATUM[\"Popular_Visualisation_Datum\",\n",
            "            SPHEROID[\"Popular Visualisation Sphere\",6378137,0],\n",
            "            TOWGS84[0,0,0,0,0,0,0]],\n",
            "        PRIMEM[\"Greenwich\",0],\n",
            "        UNIT[\"degree\",0.0174532925199433]],\n",
            "    PROJECTION[\"Mercator_1SP\"],\n",
            "    PARAMETER[\"central_meridian\",0],\n",
            "    PARAMETER[\"scale_factor\",1],\n",
            "    PARAMETER[\"false_easting\",0],\n",
            "    PARAMETER[\"false_northing\",0],\n",
            "    UNIT[\"metre\",1],\n",
            "    AXIS[\"Easting\",EAST],\n",
            "    AXIS[\"Northing\",NORTH],\n",
            "    EXTENSION[\"PROJ4\",\"+proj=merc +a=6378137 +b=6378137 +lat_ts=0 +lon_0=0 \
             +x_0=0 +y_0=0 +k=1 +units=m +nadgrids=@null +wktext +no_defs\"]]"
        )
    );
}

#[test]
fn operation_webmerc_import_from_broken_esri_wgs_84_pseudo_mercator() {
    // Likely the result of a broken export of GDAL morphToESRI()
    let wkt1 = concat!(
        "PROJCS[\"WGS_84_Pseudo_Mercator\",GEOGCS[\"GCS_WGS_1984\",",
        "DATUM[\"D_WGS_1984\",SPHEROID[\"WGS_1984\",",
        "6378137,298.257223563]],PRIMEM[\"Greenwich\",0],",
        "UNIT[\"Degree\",0.017453292519943295]],",
        "PROJECTION[\"Mercator\"],PARAMETER[\"central_meridian\",0],",
        "PARAMETER[\"false_easting\",0],",
        "PARAMETER[\"false_northing\",0],UNIT[\"Meter\",1],",
        "PARAMETER[\"standard_parallel_1\",0.0]]"
    );
    let mut parser = WktParser::new();
    parser.set_strictness(WktStrictness::Lenient);
    let crs = parser
        .create_from_wkt(wkt1)
        .expect("wkt")
        .projected()
        .expect("projected")
        .clone();
    assert_eq!(conv_wkt2(crs.deriving_conversion()), WEBMERC_DEFAULT);
}

#[test]
fn operation_mercator_spherical_import() {
    let wkt2 = concat!(
        "PROJCRS[\"unknown\",\n",
        "    BASEGEOGCRS[\"unknown\",\n",
        "        DATUM[\"World Geodetic System 1984\",\n",
        "            ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n",
        "                LENGTHUNIT[\"metre\",1]],\n",
        "            ID[\"EPSG\",6326]],\n",
        "        PRIMEM[\"Greenwich\",0,\n",
        "            ANGLEUNIT[\"degree\",0.0174532925199433],\n",
        "            ID[\"EPSG\",8901]]],\n",
        "    CONVERSION[\"unknown\",\n",
        "        METHOD[\"Mercator (Spherical)\",\n",
        "            ID[\"EPSG\",1026]],\n",
        "        PARAMETER[\"Latitude of natural origin\",0,\n",
        "            ANGLEUNIT[\"degree\",0.0174532925199433],\n",
        "            ID[\"EPSG\",8801]],\n",
        "        PARAMETER[\"Longitude of natural origin\",0,\n",
        "            ANGLEUNIT[\"degree\",0.0174532925199433],\n",
        "            ID[\"EPSG\",8802]],\n",
        "        PARAMETER[\"False easting\",0,\n",
        "            LENGTHUNIT[\"metre\",1],\n",
        "            ID[\"EPSG\",8806]],\n",
        "        PARAMETER[\"False northing\",0,\n",
        "            LENGTHUNIT[\"metre\",1],\n",
        "            ID[\"EPSG\",8807]]],\n",
        "    CS[Cartesian,2],\n",
        "        AXIS[\"(E)\",east,\n",
        "            ORDER[1],\n",
        "            LENGTHUNIT[\"metre\",1,\n",
        "                ID[\"EPSG\",9001]]],\n",
        "        AXIS[\"(N)\",north,\n",
        "            ORDER[2],\n",
        "            LENGTHUNIT[\"metre\",1,\n",
        "                ID[\"EPSG\",9001]]]]"
    );
    let crs = parse_projected(wkt2);
    assert_eq!(
        crs.export_to_proj_string().expect("proj"),
        "+proj=merc +R_C +lat_0=0 +lon_0=0 +x_0=0 +y_0=0 +datum=WGS84 +units=m +no_defs +type=crs"
    );
    assert_eq!(projected_wkt(&crs, WktConvention::Wkt2_2019), wkt2);
}
