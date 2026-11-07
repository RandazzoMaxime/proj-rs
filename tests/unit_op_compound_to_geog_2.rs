//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`: compound CRS to a geographic CRS (2D or 3D),
//! second lot. The C++ `TEST()` names are kept in the doc comments and the expected names and
//! PROJ strings are copied from that file.

use proj_rs::iso19111::{
    create_from_user_input, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, Crs, DatabaseContext, Equivalence, GridAvailabilityUse, ProjLines,
    SpatialCriterion, WktObject, WktParser,
};

fn ctx_partial(db: &DatabaseContext, authority: &str) -> CoordinateOperationContext {
    let mut ctxt = CoordinateOperationContext::create(db, authority);
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    ctxt
}

fn ctx_partial_ignore(db: &DatabaseContext, authority: &str) -> CoordinateOperationContext {
    let mut ctxt = ctx_partial(db, authority);
    ctxt.set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    ctxt
}

/// `createFromUserInput("EPSG:<code>")`, which also reads compound codes such as `4150+5773`.
fn epsg(db: &DatabaseContext, code: &str) -> Crs {
    create_from_user_input(&format!("EPSG:{code}"), db).expect(code)
}

fn from_user_input(db: &DatabaseContext, text: &str) -> Crs {
    create_from_user_input(text, db).expect(text)
}

fn wkt_crs(db: &DatabaseContext, text: &str) -> Crs {
    let mut parser = WktParser::new();
    parser.attach_database(db.clone());
    match parser.create_from_wkt(text).expect("WKT") {
        WktObject::Crs(crs) => crs,
        _ => panic!("not a CRS"),
    }
}

fn operations(
    source: &Crs,
    target: &Crs,
    ctxt: &CoordinateOperationContext,
) -> Vec<CoordinateOperation> {
    CoordinateOperationFactory::create()
        .create_operations(source, target, ctxt)
        .expect("operations")
}

/// `exportToPROJString` with the database for the grid names.
fn proj(db: &DatabaseContext, operation: &CoordinateOperation) -> String {
    operation
        .export_to_proj_string_with_database(db, ProjLines::Single)
        .expect("PROJ string")
}

/// `PROJCRS_NOID_SRC`: the C++ string literal, concatenated.
const PROJCRS_NOID_SRC: &str = r#"COMPOUNDCRS["NAD83 / Pennsylvania South + NAVD88 height",
    PROJCRS["NAD83 / Pennsylvania South",
        BASEGEOGCRS["NAD83",
            DATUM["North American Datum 1983",
                ELLIPSOID["GRS 1980",6378137,298.257222101,
                    LENGTHUNIT["metre",1]]],
            PRIMEM["Greenwich",0,
                ANGLEUNIT["degree",0.0174532925199433]]],
        CONVERSION["SPCS83 Pennsylvania South zone (meters)",
            METHOD["Lambert Conic Conformal (2SP)",
                ID["EPSG",9802]],
            PARAMETER["Latitude of false origin",39.3333333333333,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8821]],
            PARAMETER["Longitude of false origin",-77.75,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8822]],
            PARAMETER["Latitude of 1st standard parallel",40.9666666666667,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8823]],
            PARAMETER["Latitude of 2nd standard parallel",39.9333333333333,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8824]],
            PARAMETER["Easting at false origin",600000,
                LENGTHUNIT["metre",1],
                ID["EPSG",8826]],
            PARAMETER["Northing at false origin",0,
                LENGTHUNIT["metre",1],
                ID["EPSG",8827]]],
        CS[Cartesian,2],
            AXIS["easting (X)",east,
                ORDER[1],
                LENGTHUNIT["metre",1]],
            AXIS["northing (Y)",north,
                ORDER[2],
                LENGTHUNIT["metre",1]]],
    VERTCRS["NAVD88 height",
        VDATUM["North American Vertical Datum 1988"],
        CS[vertical,1],
            AXIS["gravity-related height (H)",up,
                LENGTHUNIT["metre",1]]]]"#;

/// `GEOIDMODEL_ID_SRC`: the C++ string literal, concatenated.
const GEOIDMODEL_ID_SRC: &str = r#"COMPOUNDCRS["NAD83(CSRS) / MTM zone 7 + CGVD28 height",
    PROJCRS["NAD83(CSRS) / MTM zone 7",
        BASEGEOGCRS["NAD83(CSRS)",
            DATUM["North American Datum of 1983 (CSRS)",
                ELLIPSOID["GRS 1980",6378137,298.257222101,
                    LENGTHUNIT["metre",1]]],
            PRIMEM["Greenwich",0,
                ANGLEUNIT["degree",0.0174532925199433]]],
        CONVERSION["MTM zone 7",
            METHOD["Transverse Mercator",
                ID["EPSG",9807]],
            PARAMETER["Latitude of natural origin",0,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8801]],
            PARAMETER["Longitude of natural origin",-70.5,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8802]],
            PARAMETER["Scale factor at natural origin",0.9999,
                SCALEUNIT["unity",1],
                ID["EPSG",8805]],
            PARAMETER["False easting",304800,
                LENGTHUNIT["metre",1],
                ID["EPSG",8806]],
            PARAMETER["False northing",0,
                LENGTHUNIT["metre",1],
                ID["EPSG",8807]]],
        CS[Cartesian,2],
            AXIS["easting (E(X))",east,
                ORDER[1],
                LENGTHUNIT["metre",1]],
            AXIS["northing (N(Y))",north,
                ORDER[2],
                LENGTHUNIT["metre",1]]],
    VERTCRS["CGVD28 height",
        VDATUM["Canadian Geodetic Vertical Datum of 1928"],
        CS[vertical,1],
            AXIS["gravity-related height (H)",up,
                LENGTHUNIT["metre",1]],
        GEOIDMODEL["HT2_2002v70",
            ID["EPSG",9985]],
        ID["EPSG",5713]]]"#;

/// `ENSEMBLE_SRC`: the C++ string literal, concatenated.
const ENSEMBLE_SRC: &str = r#"COMPOUNDCRS["Compound CRS OSGB36 / British National Grid + ODN height + PROJ uk_os_OSGM15_GB.tif",
    PROJCRS["OSGB36 / British National Grid",
        BASEGEOGCRS["OSGB36",
            DATUM["Ordnance Survey of Great Britain 1936",
                ELLIPSOID["Airy 1830",6377563.396,299.3249646,
                    LENGTHUNIT["metre",1]]],
            PRIMEM["Greenwich",0,
                ANGLEUNIT["degree",0.0174532925199433]],
            ID["EPSG",4277]],
        CONVERSION["British National Grid",
            METHOD["Transverse Mercator",
                ID["EPSG",9807]],
            PARAMETER["Latitude of natural origin",49,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8801]],
            PARAMETER["Longitude of natural origin",-2,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8802]],
            PARAMETER["Scale factor at natural origin",0.9996012717,
                SCALEUNIT["unity",1],
                ID["EPSG",8805]],
            PARAMETER["False easting",400000,
                LENGTHUNIT["metre",1],
                ID["EPSG",8806]],
            PARAMETER["False northing",-100000,
                LENGTHUNIT["metre",1],
                ID["EPSG",8807]]],
        CS[Cartesian,2],
            AXIS["(E)",east,
                ORDER[1],
                LENGTHUNIT["metre",1]],
            AXIS["(N)",north,
                ORDER[2],
                LENGTHUNIT["metre",1]],
        USAGE[
            SCOPE["Engineering survey, topographic mapping."],
            AREA["United Kingdom (UK) - offshore to boundary of UKCS within 49°45'N to 61°N and 9°W to 2°E; onshore Great Britain (England, Wales and Scotland). Isle of Man onshore."],
            BBOX[49.75,-9.01,61.01,2.01]],
        ID["EPSG",27700]],
    VERTCRS["ODN height + PROJ uk_os_OSGM15_GB.tif",
        VDATUM["Ordnance Datum Newlyn",
            ID["EPSG",5101]],
        CS[vertical,1],
            AXIS["gravity-related height (H)",up,
                LENGTHUNIT["metre",1]],
        GEOIDMODEL["PROJ uk_os_OSGM15_GB.tif"]]]"#;

/// `WKT2_NOID`: the C++ string literal, concatenated.
const WKT2_NOID: &str = r#"COMPOUNDCRS["unknown",
  PROJCRS["unknown",
    BASEGEOGCRS["Amersfoort",
            DATUM["Amersfoort",
                ELLIPSOID["Bessel 1841",6377397.155,299.1528128]]],
    CONVERSION["unknown",
        METHOD["Oblique Stereographic"],
        PARAMETER["Latitude of natural origin",52.1561605555556],
        PARAMETER["Longitude of natural origin",5.38763888888889],
        PARAMETER["Scale factor at natural origin",0.9999079],
        PARAMETER["False easting",155000],
        PARAMETER["False northing",463000]],
    CS[Cartesian,2],
        AXIS["(E)",east],
        AXIS["(N)",north],
    LENGTHUNIT["metre",1]],
  VERTCRS["NAP height",
    VDATUM["Normaal Amsterdams Peil"],
    CS[vertical,1],
        AXIS["gravity-related height (H)",up,
        LENGTHUNIT["metre",1]]],
    USAGE[
        SCOPE["unknown"],
        AREA["Netherlands - onshore"],
        BBOX[50.75,3.2,53.7,7.22]]]"#;

/// `VERTBOUND_SRC`: the C++ string literal, concatenated.
const VERTBOUND_SRC: &str = r#"COMPOUNDCRS["ENU (-77.410692720411:39.4145340892321) + EGM96 geoid height",
    PROJCRS["ENU (-77.410692720411:39.4145340892321)",
        BASEGEOGCRS["WGS 84",
            DATUM["unknown",
                ELLIPSOID["WGS84",6378137,298.257223563,
                    LENGTHUNIT["metre",1,
                        ID["EPSG",9001]]]],
            PRIMEM["Greenwich",0,
                ANGLEUNIT["Degree",0.0174532925199433]]],
        CONVERSION["unnamed",
            METHOD["Orthographic",
                ID["EPSG",9840]],
            PARAMETER["Latitude of natural origin",39.4145340892321,
                ANGLEUNIT["Degree",0.0174532925199433],
                ID["EPSG",8801]],
            PARAMETER["Longitude of natural origin",-77.410692720411,
                ANGLEUNIT["Degree",0.0174532925199433],
                ID["EPSG",8802]],
            PARAMETER["False easting",0,
                LENGTHUNIT["metre",1],
                ID["EPSG",8806]],
            PARAMETER["False northing",0,
                LENGTHUNIT["metre",1],
                ID["EPSG",8807]]],
        CS[Cartesian,2],
            AXIS["easting",east,
                ORDER[1],
                LENGTHUNIT["metre",1,
                    ID["EPSG",9001]]],
            AXIS["northing",north,
                ORDER[2],
                LENGTHUNIT["metre",1,
                    ID["EPSG",9001]]]],
    BOUNDCRS[
        SOURCECRS[
            VERTCRS["EGM96 geoid height",
                VDATUM["EGM96 geoid"],
                CS[vertical,1],
                    AXIS["up",up,
                        LENGTHUNIT["m",1]],
                ID["EPSG",5773]]],
        TARGETCRS[
            GEOGCRS["WGS 84",
                DATUM["World Geodetic System 1984",
                    ELLIPSOID["WGS 84",6378137,298.257223563,
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
                ID["EPSG",4979]]],
        ABRIDGEDTRANSFORMATION["EGM96 geoid height to WGS 84 ellipsoidal height",
            METHOD["GravityRelatedHeight to Geographic3D"],
            PARAMETERFILE["Geoid (height correction) model file","egm96_15.gtx",
                ID["EPSG",8666]]]]]"#;

/// `VERTBOUND_DST`: the C++ string literal, concatenated.
const VERTBOUND_DST: &str = r#"BOUNDCRS[
    SOURCECRS[
        GEOGCRS["WGS84 Coordinate System",
            DATUM["World Geodetic System 1984",
                ELLIPSOID["WGS 1984",6378137,298.257223563,
                    LENGTHUNIT["metre",1]],
                ID["EPSG",6326]],
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
                    LENGTHUNIT["metre",1,
                        ID["EPSG",9001]]],
            REMARK["Promoted to 3D from EPSG:4326"]]],
    TARGETCRS[
        GEOGCRS["WGS 84",
            ENSEMBLE["World Geodetic System 1984 ensemble",
                MEMBER["World Geodetic System 1984 (Transit)"],
                MEMBER["World Geodetic System 1984 (G730)"],
                MEMBER["World Geodetic System 1984 (G873)"],
                MEMBER["World Geodetic System 1984 (G1150)"],
                MEMBER["World Geodetic System 1984 (G1674)"],
                MEMBER["World Geodetic System 1984 (G1762)"],
                MEMBER["World Geodetic System 1984 (G2139)"],
                ELLIPSOID["WGS 84",6378137,298.257223563,
                    LENGTHUNIT["metre",1]],
                ENSEMBLEACCURACY[2.0]],
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
            USAGE[
                SCOPE["Geodesy. Navigation and positioning using GPS satellite system."],
                AREA["World."],
                BBOX[-90,-180,90,180]],
            ID["EPSG",4979]]],
    ABRIDGEDTRANSFORMATION["Transformation from WGS84 Coordinate System to WGS84",
        METHOD["Position Vector transformation (geog2D domain)",
            ID["EPSG",9606]],
        PARAMETER["X-axis translation",0,
            ID["EPSG",8605]],
        PARAMETER["Y-axis translation",0,
            ID["EPSG",8606]],
        PARAMETER["Z-axis translation",0,
            ID["EPSG",8607]],
        PARAMETER["X-axis rotation",0,
            ID["EPSG",8608]],
        PARAMETER["Y-axis rotation",0,
            ID["EPSG",8609]],
        PARAMETER["Z-axis rotation",0,
            ID["EPSG",8610]],
        PARAMETER["Scale difference",1,
            ID["EPSG",8611]]]]"#;

/// `TEST(operation, compoundCRS_with_vert_bound_to_bound_geog3D)`: issue 3927.
#[test]
fn operation_compound_with_vert_bound_to_bound_geog3d() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = wkt_crs(&db, VERTBOUND_SRC);
    let dst = wkt_crs(&db, VERTBOUND_DST);
    let ctxt = ctx_partial_ignore(&db, "");
    let list = operations(&src, &dst, &ctxt);
    assert_eq!(list.len(), 1);
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline \
         +step +inv +proj=ortho +lat_0=39.4145340892321 \
         +lon_0=-77.410692720411 +x_0=0 +y_0=0 +ellps=WGS84 \
         +step +proj=vgridshift +grids=egm96_15.gtx +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}

/// `TEST(operation, compoundCRS_to_geogCRS_3D_with_null_helmert_same_geog_src_target_context)`
#[test]
fn operation_compound_to_geog_3d_with_null_helmert_same_geog_src_target_context() {
    let db = DatabaseContext::create().expect("proj.db");
    // ETRS89 + EGM96 height
    let src = epsg(&db, "4258+5773");
    // ETRS89 3D
    let dst = epsg(&db, "4937");
    let ctxt = ctx_partial_ignore(&db, "EPSG");
    let list = operations(&src, &dst, &ctxt);
    assert!(!list.is_empty());
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=vgridshift +grids=us_nga_egm96_15.tif +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}

/// `TEST(operation, compoundCRS_of_projCRS_from_wkt_without_id_or_extent_to_geogCRS)`
#[test]
fn operation_compound_of_proj_crs_from_wkt_without_id_or_extent_to_geog() {
    let db = DatabaseContext::create().expect("proj.db");
    let ctxt = ctx_partial_ignore(&db, "EPSG");
    let src = from_user_input(&db, PROJCRS_NOID_SRC);
    let dst = epsg(&db, "4269"); // NAD83
    let list = operations(&src, &dst, &ctxt);
    // NAD83 / Pennsylvania South + NAVD88 height
    let src_ref = from_user_input(&db, "EPSG:32129+5703");
    assert!(src.is_equivalent_to_with(&src_ref, Equivalence::Equivalent));
    let list_ref = operations(&src_ref, &dst, &ctxt);
    assert_eq!(list.len(), list_ref.len());
}

/// `TEST(operation, compoundCRS_of_vertCRS_with_geoid_model_by_id_to_geogCRS)`
#[test]
fn operation_compound_of_vert_crs_with_geoid_model_by_id_to_geog() {
    let db = DatabaseContext::create().expect("proj.db");
    let ctxt = ctx_partial_ignore(&db, "EPSG");
    let src = wkt_crs(&db, GEOIDMODEL_ID_SRC);
    let dst = epsg(&db, "4955").promote_to_3d(Some(&db)); // NAD83(CSRS) 3d
    let list = operations(&src, &dst, &ctxt);
    assert!(!list.is_empty());
    assert_eq!(
        list[0].name(),
        "Inverse of MTM zone 7 + \
         Ballpark geographic offset from NAD83(CSRS) to NAD83(CSRS)v4 + \
         Inverse of NAD83(CSRS)v4 to CGVD28 height (1) + \
         Ballpark geographic offset from NAD83(CSRS)v4 to NAD83(CSRS)"
    );
}

/// `TEST(operation, compoundCRS_of_vertCRS_with_geoid_model_by_name_and_datum_ensemble)`
#[test]
fn operation_compound_of_vert_crs_with_geoid_model_by_name_and_datum_ensemble() {
    let db = DatabaseContext::create().expect("proj.db");
    let ctxt = ctx_partial_ignore(&db, "EPSG");
    let src = wkt_crs(&db, ENSEMBLE_SRC);
    // ETRS89 geocentric
    let dst = epsg(&db, "11007");
    let list = operations(&src, &dst, &ctxt);
    assert!(!list.is_empty());
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline \
         +step +inv +proj=tmerc +lat_0=49 +lon_0=-2 +k=0.9996012717 \
         +x_0=400000 +y_0=-100000 +ellps=airy \
         +step +proj=hgridshift \
         +grids=uk_os_OSTN15_NTv2_OSGBtoETRS.tif \
         +step +proj=vgridshift \
         +grids=uk_os_OSGM15_GB.tif +multiplier=1 \
         +step +proj=cart +ellps=GRS80"
    );
}

/// `TEST(operation, compoundCRS_from_WKT2_no_id_to_geogCRS_3D_context)`
#[test]
fn operation_compound_from_wkt2_no_id_to_geog_3d_context() {
    let db = DatabaseContext::create().expect("proj.db");
    let ctxt = ctx_partial(&db, "EPSG");
    // Amersfoort / RD + NAP height
    let src = epsg(&db, "7415");
    // ETRS89-NLD [AGRS2010] 3D
    let dst = epsg(&db, "11036");
    let list = operations(&src, &dst, &ctxt);
    assert!(!list.is_empty());
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline +step +inv +proj=sterea +lat_0=52.1561605555556 \
         +lon_0=5.38763888888889 +k=0.9999079 +x_0=155000 +y_0=463000 \
         +ellps=bessel \
         +step +proj=hgridshift +grids=nl_nsgi_rdtrans2018.tif \
         +step +proj=vgridshift +grids=nl_nsgi_nlgeo2018.tif +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
    let src_from_wkt2 = match WktParser::new().create_from_wkt(WKT2_NOID).expect("WKT") {
        WktObject::Crs(crs) => crs,
        _ => panic!("not a CRS"),
    };
    let list2 = operations(&src_from_wkt2, &dst, &ctxt);
    assert!(list.len() >= 4);
    assert!(list2.len() >= 4);
    for i in 0..4 {
        assert_eq!(proj(&db, &list[i]), proj(&db, &list2[i]), "op {}", i);
    }
}
