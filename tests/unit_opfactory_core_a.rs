//! PROJ `test/unit/test_operationfactory.cpp`, first block of the `core` class
//! (`docs/PORTING-opfactory-core-a.md`). Each function is one `TEST()`; names, counts,
//! EPSG codes and PROJ strings are the expected text of PROJ 9.9.0.

use proj_rs::iso19111::{
    create_crs_from_proj_string, AlternativeGridNames, AuthorityFactory, CartesianCS, CompoundCRS,
    Conversion, CoordinateOperation, CoordinateOperationContext, CoordinateOperationFactory, Crs,
    DatabaseContext, GeodeticCRS, GeographicCRS, GridAvailabilityUse, OpProps, ProjLines,
    ProjectedCRS, SpatialCriterion, Transformation, UnitOfMeasure, WktParser,
};
use proj_rs::Proj;

fn database() -> DatabaseContext {
    DatabaseContext::create().expect("database")
}

fn epsg_crs(db: &DatabaseContext, code: &str) -> Crs {
    AuthorityFactory::create(db, "EPSG")
        .create_coordinate_reference_system(code)
        .expect(code)
}

fn wkt_crs(wkt: &str) -> Crs {
    WktParser::new()
        .create_from_wkt(wkt)
        .expect("wkt")
        .crs()
        .expect("crs")
        .clone()
}

fn operations(
    source: &Crs,
    target: &Crs,
    context: &CoordinateOperationContext,
) -> Vec<CoordinateOperation> {
    CoordinateOperationFactory::create()
        .create_operations(source, target, context)
        .expect("operations")
}

/// `exportToPROJString` with a formatter that has a database.
fn proj(db: &DatabaseContext, operation: &CoordinateOperation) -> String {
    match operation {
        CoordinateOperation::Conversion(conversion)
            if conversion.source_crs().is_none() || conversion.target_crs().is_none() =>
        {
            conversion.export_to_proj_string()
        }
        _ => operation.export_to_proj_string_with_database(db, ProjLines::Single),
    }
    .expect("PROJ string")
}

/// `createOperation(source, target)` without a database.
fn operation(source: &Crs, target: &Crs) -> CoordinateOperation {
    CoordinateOperationFactory::create()
        .create_operation(source, target)
        .expect("operation")
        .expect("an operation")
}

/// `exportToPROJString` of an operation, with no database.
fn proj_nodb(operation: &CoordinateOperation) -> String {
    proj(&database(), operation)
}

fn proj_crs(definition: &str) -> Crs {
    create_crs_from_proj_string(definition).expect(definition)
}

const WGS84_GEOCENTRIC_KM: &str = r#"GEODCRS["Based on WGS 84",
    DATUM["World Geodetic System 1984",
        ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1]],
        ID["EPSG",6326]],
    PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]],
    CS[Cartesian,3],
        AXIS["(X)",geocentricX,LENGTHUNIT["kilometre",1000]],
        AXIS["(Y)",geocentricY,LENGTHUNIT["kilometre",1000]],
        AXIS["(Z)",geocentricZ,LENGTHUNIT["kilometre",1000]]]"#;

/// `createGeocentricDatumWGS84`.
const WGS84_GEOCENTRIC: &str = r#"GEODCRS["WGS 84",
    DATUM["World Geodetic System 1984",
        ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1]],
        ID["EPSG",6326]],
    PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]],
    CS[Cartesian,3],
        AXIS["(X)",geocentricX,LENGTHUNIT["metre",1]],
        AXIS["(Y)",geocentricY,LENGTHUNIT["metre",1]],
        AXIS["(Z)",geocentricZ,LENGTHUNIT["metre",1]],
    ID["EPSG",4328]]"#;

const NAD83_GEOCENTRIC_UNKNOWN: &str = r#"GEODCRS["unknown",
    DATUM["North American Datum 1983",
        ELLIPSOID["GRS 1980",6378137,298.257222101,LENGTHUNIT["metre",1]],
        ID["EPSG",6269]],
    PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]],
    CS[Cartesian,3],
        AXIS["(X)",geocentricX,LENGTHUNIT["metre",1]],
        AXIS["(Y)",geocentricY,LENGTHUNIT["metre",1]],
        AXIS["(Z)",geocentricZ,LENGTHUNIT["metre",1]],
    ID["EPSG",4328]]"#;

#[test]
fn operation_geog_crs_to_geog_crs() {
    let db = database();
    let op = operation(&epsg_crs(&db, "4807"), &epsg_crs(&db, "4326"));
    assert_eq!(
        proj_nodb(&op),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=grad +xy_out=rad +step +inv +proj=longlat \
         +ellps=clrk80ign +pm=paris +step +proj=unitconvert +xy_in=rad \
         +xy_out=deg +step +proj=axisswap +order=2,1"
    );
}

#[test]
fn operation_geog_crs_to_geog_crs_noop() {
    let db = database();
    let wgs84 = epsg_crs(&db, "4326");
    let op = operation(&wgs84, &wgs84);
    assert_eq!(op.name(), "Null geographic offset from WGS 84 to WGS 84");
    assert_eq!(proj_nodb(&op), "+proj=noop");
    assert_eq!(op.inverse().name(), op.name());
}

#[test]
fn operation_geog_crs_to_geog_crs_longitude_rotation() {
    let src = wkt_crs(
        r#"GEOGCRS["A",
            DATUM["unknown",ELLIPSOID["WGS 84",6378137,298.257223563]],
            PRIMEM["Greenwich",0],
            CS[ellipsoidal,2],
                AXIS["latitude",north,ANGLEUNIT["degree",0.0174532925199433]],
                AXIS["longitude",east,ANGLEUNIT["degree",0.0174532925199433]]]"#,
    );
    let dest = wkt_crs(
        r#"GEOGCRS["B",
            DATUM["unknown",ELLIPSOID["WGS 84",6378137,298.257223563]],
            PRIMEM["Paris",2.5969213,ANGLEUNIT["grad",0.0157079632679489]],
            CS[ellipsoidal,2],
                AXIS["latitude",north,ANGLEUNIT["degree",0.0174532925199433]],
                AXIS["longitude",east,ANGLEUNIT["degree",0.0174532925199433]]]"#,
    );
    let op = operation(&src, &dest);
    assert_eq!(
        proj_nodb(&op),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=longlat \
         +ellps=WGS84 +pm=paris +step +proj=unitconvert +xy_in=rad \
         +xy_out=deg +step +proj=axisswap +order=2,1"
    );
    let reverse = operation(&dest, &src);
    assert_eq!(proj_nodb(&op.inverse()), proj_nodb(&reverse));
    assert_eq!(op.inverse().name(), reverse.name());
}

#[test]
fn operation_geog_crs_to_geog_crs_3d() {
    let geogcrs_m = proj_crs("+proj=longlat +vunits=m +type=crs");
    let geogcrs_ft = proj_crs("+proj=longlat +vunits=ft +type=crs");
    assert_eq!(
        proj_nodb(&operation(&geogcrs_m, &geogcrs_ft)),
        "+proj=unitconvert +z_in=m +z_out=ft"
    );
    assert_eq!(
        proj_nodb(&operation(&geogcrs_ft, &geogcrs_m)),
        "+proj=unitconvert +z_in=ft +z_out=m"
    );
    let geogcrs_m_with_pm = proj_crs("+proj=longlat +pm=paris +vunits=m +type=crs");
    assert_eq!(
        proj_nodb(&operation(&geogcrs_m_with_pm, &geogcrs_ft)),
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +z_in=m \
         +xy_out=rad +z_out=m +step +inv +proj=longlat +ellps=WGS84 \
         +pm=paris +step +proj=unitconvert +xy_in=rad +z_in=m \
         +xy_out=deg +z_out=ft"
    );
}

#[test]
fn operation_geog_crs_3d_lat_long_non_metre_to_geog_crs_longlat() {
    let src = wkt_crs(
        r#"GEOGCRS["my CRS",
            DATUM["World Geodetic System 1984",
                ELLIPSOID["WGS 84",6378137,298.257223563],
                ID["EPSG",6326]],
            CS[ellipsoidal,3],
                AXIS["latitude",north,ANGLEUNIT["degree",0.0174532925199433]],
                AXIS["longitude",east,ANGLEUNIT["degree",0.0174532925199433]],
                AXIS["ellipsoidal height",up,LENGTHUNIT["my_vunit",0.3]]]"#,
    );
    let dst = proj_crs("+proj=longlat +datum=WGS84 +type=crs");
    assert_eq!(
        proj_nodb(&operation(&src, &dst)),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +z_in=0.3 +z_out=m"
    );
}

#[test]
fn operation_geocentric_crs_to_geog_crs_same_datum() {
    let db = database();
    let op = operation(&wkt_crs(WGS84_GEOCENTRIC), &epsg_crs(&db, "4326"));
    assert_eq!(
        proj_nodb(&op),
        "+proj=pipeline +step +inv +proj=cart +ellps=WGS84 +step \
         +proj=unitconvert +xy_in=rad +xy_out=deg +step +proj=axisswap \
         +order=2,1"
    );
}

#[test]
fn operation_geocentric_crs_to_geog_crs_different_datum() {
    let db = database();
    let op = operation(&wkt_crs(WGS84_GEOCENTRIC), &epsg_crs(&db, "4269"));
    assert_eq!(
        op.name(),
        "Conversion from WGS 84 to WGS 84 (geographic) + \
         Ballpark geographic offset from WGS 84 (geographic) to NAD83"
    );
    assert_eq!(
        proj_nodb(&op),
        "+proj=pipeline \
         +step +inv +proj=cart +ellps=WGS84 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}

#[test]
fn operation_geog_crs_to_geocentric_crs_different_datum() {
    let db = database();
    let op = operation(&epsg_crs(&db, "4269"), &wkt_crs(WGS84_GEOCENTRIC));
    assert_eq!(
        op.name(),
        "Ballpark geographic offset from NAD83 to WGS 84 (geographic) + \
         Conversion from WGS 84 (geographic) to WGS 84"
    );
    assert_eq!(
        proj_nodb(&op),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m \
         +step +proj=cart +ellps=WGS84"
    );
}

#[test]
fn operation_geocentric_crs_to_geocentric_crs_same_noop() {
    let geocentric = wkt_crs(WGS84_GEOCENTRIC);
    let op = operation(&geocentric, &geocentric);
    assert_eq!(
        op.name(),
        "Null geocentric translation from WGS 84 to WGS 84"
    );
    assert_eq!(proj_nodb(&op), "+proj=noop");
    assert_eq!(op.inverse().name(), op.name());
}

#[test]
fn operation_geocentric_crs_to_geocentric_crs_different_ballpark() {
    let op = operation(
        &wkt_crs(WGS84_GEOCENTRIC_KM),
        &wkt_crs(NAD83_GEOCENTRIC_UNKNOWN),
    );
    assert_eq!(
        op.name(),
        "Ballpark geocentric translation from Based on WGS 84 to unknown"
    );
    assert_eq!(
        proj_nodb(&op),
        "+proj=unitconvert +xy_in=km +z_in=km +xy_out=m +z_out=m"
    );
}

#[test]
fn operation_geog_crs_to_geog_crs_context_ntv1_ntv2_ctable2() {
    let db = database();
    let mut ctxt = CoordinateOperationContext::create(&db, "EPSG");
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    let list = operations(&epsg_crs(&db, "4267"), &epsg_crs(&db, "4269"), &ctxt);
    assert_eq!(list.len(), 10);
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=hgridshift \
         +grids=ca_nrc_ntv2_0.tif +step +proj=unitconvert +xy_in=rad \
         +xy_out=deg +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        proj(&db, &list[1]),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=hgridshift \
         +grids=ca_nrc_ntv1_can.tif +step +proj=unitconvert +xy_in=rad \
         +xy_out=deg +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        proj(&db, &list[2]),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=hgridshift \
         +grids=us_noaa_conus.tif +step +proj=unitconvert +xy_in=rad \
         +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}

#[test]
fn operation_geog_crs_to_geog_crs_context_nad27_to_wgs84() {
    let db = database();
    let mut ctxt = CoordinateOperationContext::create(&db, "EPSG");
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    let list = operations(&epsg_crs(&db, "4267"), &epsg_crs(&db, "4326"), &ctxt);
    assert_eq!(list.len(), 79);
    assert_eq!(list[0].name(), "NAD27 to WGS 84 (33)"); // 1.0 m, Canada - NAD27
    assert_eq!(list[1].name(), "NAD27 to WGS 84 (3)"); // 20.0 m, Canada - NAD27
    assert_eq!(list[2].name(), "NAD27 to WGS 84 (79)"); // 5.0 m, USA - CONUS including EEZ
    assert_eq!(list[3].name(), "NAD27 to WGS 84 (4)"); // 10.0 m, USA - CONUS - onshore
}

#[test]
fn operation_geog_crs_to_geog_crs_context_helmert_geog3d_crs() {
    let db = database();
    let ctxt = CoordinateOperationContext::create(&db, "EPSG");
    // GDA94 3D to GDA2020 3D
    let list = operations(&epsg_crs(&db, "4939"), &epsg_crs(&db, "7843"), &ctxt);
    assert_eq!(list.len(), 1);
    // Check there is no push / pop of v_3
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m \
         +step +proj=cart +ellps=GRS80 \
         +step +proj=helmert +x=0.06155 +y=-0.01087 +z=-0.04019 \
         +rx=-0.0394924 +ry=-0.0327221 +rz=-0.0328979 +s=-0.009994 \
         +convention=coordinate_frame \
         +step +inv +proj=cart +ellps=GRS80 \
         +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m \
         +step +proj=axisswap +order=2,1"
    );
}

#[test]
fn operation_geog_crs_to_geog_crs_context_helmert_geocentric_3d() {
    let db = database();
    let ctxt = CoordinateOperationContext::create(&db, "EPSG");
    // GDA94 geocentric to GDA2020 geocentric
    let list = operations(&epsg_crs(&db, "4348"), &epsg_crs(&db, "7842"), &ctxt);
    assert_eq!(list.len(), 1);
    // Check there is no push / pop of v_3
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=helmert +x=0.06155 +y=-0.01087 +z=-0.04019 \
         +rx=-0.0394924 +ry=-0.0327221 +rz=-0.0328979 +s=-0.009994 \
         +convention=coordinate_frame"
    );
    assert_eq!(
        proj(&db, &list[0].inverse()),
        "+proj=pipeline \
         +step +inv +proj=helmert +x=0.06155 +y=-0.01087 +z=-0.04019 \
         +rx=-0.0394924 +ry=-0.0327221 +rz=-0.0328979 +s=-0.009994 \
         +convention=coordinate_frame"
    );
}

#[test]
fn operation_geog_crs_to_geog_crs_context_helmert_geog3d_to_geocentirc() {
    let db = database();
    let ctxt = CoordinateOperationContext::create(&db, "EPSG");
    // GDA94 3D to GDA2020 geocentric
    let list = operations(&epsg_crs(&db, "4939"), &epsg_crs(&db, "7842"), &ctxt);
    assert!(!list.is_empty());
    // Check there is no push / pop of v_3
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m \
         +step +proj=cart +ellps=GRS80 \
         +step +proj=helmert +x=0.06155 +y=-0.01087 +z=-0.04019 \
         +rx=-0.0394924 +ry=-0.0327221 +rz=-0.0328979 +s=-0.009994 \
         +convention=coordinate_frame"
    );
}

#[test]
fn operation_geog_crs_to_geog_crs_context_same_grid_name() {
    let db = database();
    let mut ctxt = CoordinateOperationContext::create(&db, "EPSG");
    ctxt.set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    // DHDN to ETRS89
    let list = operations(&epsg_crs(&db, "4314"), &epsg_crs(&db, "4258"), &ctxt);
    assert!(!list.is_empty());
    assert_eq!(list[0].name(), "DHDN to ETRS89 (8)");
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=hgridshift \
         +grids=de_adv_BETA2007.tif +step +proj=unitconvert +xy_in=rad \
         +xy_out=deg +step +proj=axisswap +order=2,1"
    );
}

#[test]
fn operation_geog_crs_to_geog_crs_geographic_offset_context() {
    let db = database();
    let ctxt = CoordinateOperationContext::create(&db, "EPSG");
    let list = operations(&epsg_crs(&db, "4120"), &epsg_crs(&db, "4121"), &ctxt);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name(), "Greek to GGRS87 (1)");
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=geogoffset \
         +dlat=-5.86 +dlon=0.28 +step +proj=unitconvert +xy_in=rad \
         +xy_out=deg +step +proj=axisswap +order=2,1"
    );
}

fn wgs84() -> Crs {
    Crs::Geographic(GeographicCRS::epsg_4326())
}

fn crs84() -> Crs {
    Crs::Geographic(GeographicCRS::ogc_crs84())
}

fn ntf_paris() -> Crs {
    Crs::Geographic(GeographicCRS::epsg_4807())
}

/// `createUTM31_WGS84` and the zone 32 twin.
fn utm_wgs84(zone: i32) -> Crs {
    Crs::Projected(ProjectedCRS::create(
        format!("WGS 84 / UTM zone {zone}N"),
        GeodeticCRS::Geographic(GeographicCRS::epsg_4326()),
        Conversion::create_utm_north(&OpProps::new(), zone).expect("utm"),
        CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    ))
}

fn utm31() -> Crs {
    utm_wgs84(31)
}

fn props() -> OpProps {
    OpProps::new()
}

#[test]
fn operation_geog_crs_to_proj_crs() {
    let op = operation(&wgs84(), &utm31());
    let CoordinateOperation::Conversion(_) = &op else {
        panic!("conversion");
    };
    let text = proj_nodb(&op);
    assert_eq!(
        text,
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=utm \
         +zone=31 +ellps=WGS84"
    );
    let (east, north) = Proj::new(&text)
        .expect("pipeline")
        .convert((49.0_f64, 2.0_f64))
        .expect("transform");
    assert!((east - 426_857.987_717_28).abs() < 1e-8, "{east}");
    assert!((north - 5_427_937.523_464_92).abs() < 1e-8, "{north}");
}

#[test]
fn operation_geog_crs_longlat_to_geog_cs_latlong() {
    let source = crs84();
    let target = wgs84();
    let op = operation(&source, &target);
    let CoordinateOperation::Conversion(conv) = &op else {
        panic!("conversion");
    };
    assert!(conv.source_crs().unwrap().is_equivalent_to(&source));
    assert!(conv.target_crs().unwrap().is_equivalent_to(&target));
    assert_eq!(proj_nodb(&op), "+proj=axisswap +order=2,1");
    let inverse = conv.inverse();
    assert!(inverse.source_crs().unwrap().is_equivalent_to(&target));
    assert!(inverse.target_crs().unwrap().is_equivalent_to(&source));
    assert_eq!(conv.method().name(), inverse.method().name());
    assert_eq!(conv.method().code(), inverse.method().code());
}

#[test]
fn operation_geog_crs_longlat_to_geog_cs_latlong_database() {
    let db = database();
    let ctxt = CoordinateOperationContext::create(&db, "");
    let source = AuthorityFactory::create(&db, "OGC")
        .create_coordinate_reference_system("CRS84")
        .expect("CRS84");
    let target = epsg_crs(&db, "4326");
    let list = operations(&source, &target, &ctxt);
    assert_eq!(list.len(), 1);
    assert_eq!(proj(&db, &list[0]), "+proj=axisswap +order=2,1");
}

#[test]
fn operation_geog_crs_longlat_to_proj_crs() {
    let op = operation(&crs84(), &utm31());
    assert_eq!(
        proj_nodb(&op),
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=utm +zone=31 +ellps=WGS84"
    );
}

#[test]
fn operation_geog_crs_different_from_base_crs_to_proj_crs() {
    let op = operation(&ntf_paris(), &utm31());
    assert_eq!(
        proj_nodb(&op),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=grad +xy_out=rad +step +inv +proj=longlat \
         +ellps=clrk80ign +pm=paris +step +proj=utm +zone=31 \
         +ellps=WGS84"
    );
}

#[test]
fn operation_geocentric_crs_to_proj_crs() {
    let op = operation(&wkt_crs(WGS84_GEOCENTRIC), &utm31());
    assert_eq!(
        proj_nodb(&op),
        "+proj=pipeline +step +inv +proj=cart +ellps=WGS84 +step \
         +proj=utm +zone=31 +ellps=WGS84"
    );
}

#[test]
fn operation_proj_crs_to_geog_crs() {
    let op = operation(&utm31(), &wgs84());
    assert_eq!(
        proj_nodb(&op),
        "+proj=pipeline +step +inv +proj=utm +zone=31 +ellps=WGS84 +step \
         +proj=unitconvert +xy_in=rad +xy_out=deg +step +proj=axisswap \
         +order=2,1"
    );
}

#[test]
fn operation_compound_crs_to_geog_crs() {
    let db = database();
    let horizontal = wgs84();
    let vertical = epsg_crs(&db, "5701");
    let compound = Crs::Compound(
        CompoundCRS::create("", vec![horizontal.clone(), vertical]).expect("compound"),
    );
    let target = ntf_paris();
    assert_eq!(
        proj_nodb(&operation(&compound, &target)),
        proj_nodb(&operation(&horizontal, &target))
    );
}

#[test]
fn operation_transformation_longitude_rotation_to_proj_string() {
    let src = wgs84();
    let transformation =
        Transformation::create_longitude_rotation(&props(), src.clone(), src, 10.0)
            .expect("longitude rotation");
    assert!(transformation.validate_parameters().is_empty());
    assert_eq!(
        proj_nodb(&CoordinateOperation::Transformation(transformation.clone())),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +inv \
         +proj=longlat +ellps=WGS84 +pm=10 +step +proj=unitconvert \
         +xy_in=rad +xy_out=deg +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        proj_nodb(&CoordinateOperation::Transformation(
            transformation.inverse()
        )),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +inv \
         +proj=longlat +ellps=WGS84 +pm=-10 +step +proj=unitconvert \
         +xy_in=rad +xy_out=deg +step +proj=axisswap +order=2,1"
    );
}

#[test]
fn operation_transformation_geographic2d_offsets_to_proj_string() {
    let src = wgs84();
    let transformation =
        Transformation::create_geographic_2d_offsets(&props(), src.clone(), src, 0.5, -1.0, None)
            .expect("offsets");
    assert!(transformation.validate_parameters().is_empty());
    assert_eq!(
        proj_nodb(&CoordinateOperation::Transformation(transformation.clone())),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=geogoffset \
         +dlat=1800 +dlon=-3600 +step +proj=unitconvert +xy_in=rad \
         +xy_out=deg +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        proj_nodb(&CoordinateOperation::Transformation(
            transformation.inverse()
        )),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=geogoffset \
         +dlat=-1800 +dlon=3600 +step +proj=unitconvert +xy_in=rad \
         +xy_out=deg +step +proj=axisswap +order=2,1"
    );
}

#[test]
fn operation_transformation_geographic3d_offsets_to_proj_string() {
    let src = wgs84();
    let transformation = Transformation::create_geographic_3d_offsets(
        &props(),
        src.clone(),
        src,
        0.5,
        -1.0,
        2.0,
        None,
    )
    .expect("offsets");
    assert!(transformation.validate_parameters().is_empty());
    assert_eq!(
        proj_nodb(&CoordinateOperation::Transformation(transformation.clone())),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=geogoffset \
         +dlat=1800 +dlon=-3600 +dh=2 +step +proj=unitconvert +xy_in=rad \
         +xy_out=deg +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        proj_nodb(&CoordinateOperation::Transformation(
            transformation.inverse()
        )),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=geogoffset \
         +dlat=-1800 +dlon=3600 +dh=-2 +step +proj=unitconvert \
         +xy_in=rad +xy_out=deg +step +proj=axisswap +order=2,1"
    );
}

#[test]
fn operation_transformation_geographic2d_with_height_offsets_to_proj_string() {
    let db = database();
    let horizontal = wgs84();
    let compound = Crs::Compound(
        CompoundCRS::create("", vec![horizontal.clone(), epsg_crs(&db, "5701")]).expect("compound"),
    );
    let transformation = Transformation::create_geographic_2d_with_height_offsets(
        &props(),
        compound,
        horizontal,
        0.5,
        -1.0,
        2.0,
        None,
    )
    .expect("offsets");
    assert!(transformation.validate_parameters().is_empty());
    assert_eq!(
        proj_nodb(&CoordinateOperation::Transformation(transformation.clone())),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=geogoffset \
         +dlat=1800 +dlon=-3600 +dh=2 +step +proj=unitconvert +xy_in=rad \
         +xy_out=deg +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        proj_nodb(&CoordinateOperation::Transformation(
            transformation.inverse()
        )),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=geogoffset \
         +dlat=-1800 +dlon=3600 +dh=-2 +step +proj=unitconvert \
         +xy_in=rad +xy_out=deg +step +proj=axisswap +order=2,1"
    );
}

#[test]
fn operation_transformation_vertical_offset_to_proj_string() {
    let db = database();
    let vertical = epsg_crs(&db, "5701");
    let transformation =
        Transformation::create_vertical_offset(&props(), vertical.clone(), vertical, 1.0, None)
            .expect("vertical offset");
    assert!(transformation.validate_parameters().is_empty());
    assert_eq!(
        proj_nodb(&CoordinateOperation::Transformation(transformation.clone())),
        "+proj=geogoffset +dh=1"
    );
    assert_eq!(
        proj_nodb(&CoordinateOperation::Transformation(
            transformation.inverse()
        )),
        "+proj=geogoffset +dh=-1"
    );
}

#[test]
fn operation_transformation_ntv2_to_proj_string() {
    let transformation =
        Transformation::create_ntv2(&props(), ntf_paris(), wgs84(), "foo.gsb", None).expect("ntv2");
    assert_eq!(
        proj_nodb(&CoordinateOperation::Transformation(transformation)),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=grad +xy_out=rad +step \
         +proj=hgridshift +grids=foo.gsb +step +proj=unitconvert \
         +xy_in=rad +xy_out=deg +step +proj=axisswap +order=2,1"
    );
}

#[test]
fn operation_transformation_vertcon_to_proj_string() {
    let db = database();
    let vertical = epsg_crs(&db, "5701");
    let other = epsg_crs(&db, "5702");
    let transformation = Transformation::create_vertcon(&props(), vertical, other, "bla.gtx", None)
        .expect("vertcon");
    assert_eq!(
        proj_nodb(&CoordinateOperation::Transformation(transformation)),
        "+proj=vgridshift +grids=bla.gtx +multiplier=0.001"
    );
}

fn authority_proj(code: &str) -> String {
    let db = database();
    let operation = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_operation(code, AlternativeGridNames::Official)
        .expect(code);
    operation
        .export_to_proj_string_with_database(&db, ProjLines::Single)
        .expect("PROJ string")
}

#[test]
fn operation_transformation_nzlvd_to_proj_string() {
    assert_eq!(
        authority_proj("7860"),
        "+proj=vgridshift +grids=nz_linz_auckht1946-nzvd2016.tif +multiplier=1"
    );
}

#[test]
fn operation_transformation_bev_at_to_proj_string() {
    assert_eq!(
        authority_proj("9275"),
        "+proj=vgridshift +grids=at_bev_GV_Hoehengrid_V1.tif +multiplier=1"
    );
}

#[test]
fn operation_geocentric_crs_to_geog_crs_same_datum_context() {
    let db = database();
    let ctxt = CoordinateOperationContext::create(&db, "EPSG");
    let list = operations(&epsg_crs(&db, "4326"), &epsg_crs(&db, "4978"), &ctxt);
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].name(),
        "Conversion from WGS 84 (geog2D) to WGS 84 (geocentric)"
    );
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=cart \
         +ellps=WGS84"
    );
    assert_eq!(
        list[0].inverse().name(),
        "Conversion from WGS 84 (geocentric) to WGS 84 (geog2D)"
    );
    assert_eq!(
        proj(&db, &list[0].inverse()),
        "+proj=pipeline +step +inv +proj=cart +ellps=WGS84 +step \
         +proj=unitconvert +xy_in=rad +xy_out=deg +step +proj=axisswap \
         +order=2,1"
    );
}

#[test]
fn operation_geocentric_crs_to_geog_crs_same_datum_context_all_auth() {
    let db = database();
    let ctxt = CoordinateOperationContext::create(&db, "");
    let list = operations(&epsg_crs(&db, "4326"), &epsg_crs(&db, "4978"), &ctxt);
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].name(),
        "Conversion from WGS 84 (geog2D) to WGS 84 (geocentric)"
    );
}

#[test]
fn operation_geog_crs_geocentric_crs_same_datum_to_context() {
    let db = database();
    let ctxt = CoordinateOperationContext::create(&db, "EPSG");
    let list = operations(&epsg_crs(&db, "4978"), &epsg_crs(&db, "4326"), &ctxt);
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].name(),
        "Conversion from WGS 84 (geocentric) to WGS 84 (geog2D)"
    );
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline +step +inv +proj=cart +ellps=WGS84 +step \
         +proj=unitconvert +xy_in=rad +xy_out=deg +step +proj=axisswap \
         +order=2,1"
    );
}

#[test]
fn operation_proj_crs_to_proj_crs() {
    let op = operation(&utm31(), &utm_wgs84(32));
    assert_eq!(
        proj_nodb(&op),
        "+proj=pipeline +step +inv +proj=utm +zone=31 +ellps=WGS84 +step \
         +proj=utm +zone=32 +ellps=WGS84"
    );
}

#[test]
fn operation_transformation_geographic3d_to_gravity_related_height_gtx() {
    let wkt = r#"COORDINATEOPERATION["ETRS89 to NAP height (1)",
    VERSION["RDNAP-Nld 2008"],
    SOURCECRS[
        GEOGCRS["ETRS89",
            DATUM["European Terrestrial Reference System 1989",
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
            ID["EPSG",4937]]],
    TARGETCRS[
        VERTCRS["NAP height",
            VDATUM["Normaal Amsterdams Peil"],
            CS[vertical,1],
                AXIS["gravity-related height (H)",up,
                    LENGTHUNIT["metre",1]],
            ID["EPSG",5709]]],
    METHOD["Geographic3D to GravityRelatedHeight (US .gtx)",
        ID["EPSG",9665]],
    PARAMETERFILE["Geoid (height correction) model file","naptrans2008.gtx"],
    OPERATIONACCURACY[0.01],
    USAGE[
        SCOPE["unknown"],
        AREA["Netherlands - onshore"],
        BBOX[50.75,3.2,53.7,7.22]],
    ID["EPSG",7001]]"#;
    let transformation = WktParser::new()
        .create_from_wkt(wkt)
        .expect("wkt")
        .transformation()
        .expect("transformation")
        .clone();
    assert_eq!(
        proj_nodb(&CoordinateOperation::Transformation(transformation)),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +inv +proj=vgridshift \
         +grids=naptrans2008.gtx +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}
