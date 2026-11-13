//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`: a compound CRS whose vertical part is a
//! derived vertical CRS (vertical offset and slope, ellipsoidal base), and a derived projected CRS
//! whose conversion is a PROJ-based operation method. The C++ `TEST()` names are kept in the doc
//! comments; expected names and PROJ strings are copied from that file, never from the port.

use proj_rs::iso19111::{
    create_from_user_input, AuthorityFactory, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, Crs, DatabaseContext, GeographicCRS, GridAvailabilityUse,
    ProjLines, SpatialCriterion, WktObject, WktParser,
};

/// `exportToPROJString(PROJStringFormatter::create().get())` of an operation.
fn proj(db: &DatabaseContext, operation: &CoordinateOperation) -> String {
    operation
        .export_to_proj_string_with_database(db, ProjLines::Single)
        .expect("PROJ string")
}

/// `WKTParser().createFromWKT(text)` for a CRS, without a database attached.
fn wkt_crs(text: &str) -> Crs {
    match WktParser::new().create_from_wkt(text).expect("WKT") {
        WktObject::Crs(crs) => crs,
        _ => panic!("not a CRS"),
    }
}

/// `TEST(operation, compoundCRS_with_derivedVerticalCRS_ellipsoidal_height_to_geog)`
#[test]
fn operation_compound_with_derived_vertical_crs_ellipsoidal_height_to_geog() {
    let db = DatabaseContext::create().expect("proj.db");
    // Test scenario of https://github.com/OSGeo/PROJ/issues/4175
    // Note that the CompoundCRS with a DerivedVerticalCRS with a "Ellipsoid"
    // datum is an extension, not OGC Topic 2 compliant.
    let dst = wkt_crs(
        r#"COMPOUNDCRS["UTM30 with vertical offset",
    PROJCRS["WGS 84 / UTM zone 30N",
        BASEGEOGCRS["WGS 84",
            ENSEMBLE["World Geodetic System 1984 ensemble",
                MEMBER["World Geodetic System 1984 (Transit)"],
                MEMBER["World Geodetic System 1984 (G730)"],
                MEMBER["World Geodetic System 1984 (G873)"],
                MEMBER["World Geodetic System 1984 (G1150)"],
                MEMBER["World Geodetic System 1984 (G1674)"],
                MEMBER["World Geodetic System 1984 (G1762)"],
                MEMBER["World Geodetic System 1984 (G2139)"],
                MEMBER["World Geodetic System 1984 (G2296)"],
                ELLIPSOID["WGS 84",6378137,298.257223563,
                    LENGTHUNIT["metre",1]],
                ENSEMBLEACCURACY[2.0]],
            PRIMEM["Greenwich",0,
                ANGLEUNIT["degree",0.0174532925199433]],
            ID["EPSG",4326]],
        CONVERSION["UTM zone 30N",
            METHOD["Transverse Mercator",
                ID["EPSG",9807]],
            PARAMETER["Latitude of natural origin",0,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8801]],
            PARAMETER["Longitude of natural origin",-3,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8802]],
            PARAMETER["Scale factor at natural origin",0.9996,
                SCALEUNIT["unity",1],
                ID["EPSG",8805]],
            PARAMETER["False easting",500000,
                LENGTHUNIT["metre",1],
                ID["EPSG",8806]],
            PARAMETER["False northing",0,
                LENGTHUNIT["metre",1],
                ID["EPSG",8807]]],
        CS[Cartesian,2],
            AXIS["(E)",east,
                ORDER[1],
                LENGTHUNIT["metre",1]],
            AXIS["(N)",north,
                ORDER[2],
                LENGTHUNIT["metre",1]],
        ID["EPSG",32630]],
    VERTCRS["Derived verticalCRS",
        BASEVERTCRS["Ellipsoid (metre)",
            VDATUM["Ellipsoid"]],
        DERIVINGCONVERSION["Conv Vertical Offset",
            METHOD["Vertical Offset",
                ID["EPSG",9616]],
            PARAMETER["Vertical Offset",42.3,
                LENGTHUNIT["metre",1],
                ID["EPSG",8603]]],
        CS[vertical,1],
            AXIS["gravity-related height (H)",up,
                LENGTHUNIT["metre",1,
                    ID["EPSG",9001]]]]]"#,
    );
    let src = Crs::Geographic(GeographicCRS::epsg_4979());

    let op = CoordinateOperationFactory::create()
        .create_operation(&src, &dst)
        .expect("operation")
        .expect("some operation");
    assert_eq!(
        proj(&db, &op),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=geogoffset +dh=42.3 \
         +step +proj=utm +zone=30 +ellps=WGS84"
    );
    assert_eq!(op.name(), "Conv Vertical Offset + UTM zone 30N");
    assert!(!op.has_ballpark_transformation());
    // `coordinateOperationAccuracies()` holds "0": the value is read as a number here.
    assert_eq!(op.accuracy(), Some(0.0));
}

/// `TEST(operation, compoundCRS_with_derived_vertical_CRS)`
#[test]
fn operation_compound_with_derived_vertical_crs() {
    let db = DatabaseContext::create().expect("proj.db");
    // ETRS89 + EGM2008 height
    let src = create_from_user_input("EPSG:4258+3855", &db).expect("source");

    let dst = create_from_user_input(
        r#"COMPOUNDCRS["WGS 84 + Custom Vertical",
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
        CS[ellipsoidal,2],
            AXIS["geodetic latitude (Lat)",north,
                ORDER[1],
                ANGLEUNIT["degree",0.0174532925199433]],
            AXIS["geodetic longitude (Lon)",east,
                ORDER[2],
                ANGLEUNIT["degree",0.0174532925199433]]],
    VERTCRS["Custom Vertical",
        BASEVERTCRS["EGM2008 height",
            VDATUM["EGM2008 geoid"]],
        DERIVINGCONVERSION["vertical offs. and slope",
            METHOD["Vertical Offset and Slope",
                ID["EPSG",1046]],
            PARAMETER["Ordinate 1 of evaluation point",47,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8617]],
            PARAMETER["Ordinate 2 of evaluation point",8,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8618]],
            PARAMETER["Vertical Offset",-0.245,
                LENGTHUNIT["metre",1],
                ID["EPSG",8603]],
            PARAMETER["Inclination in latitude",-0.21,
                ANGLEUNIT["arc-second",4.84813681109536E-06],
                ID["EPSG",8730]],
            PARAMETER["Inclination in longitude",-0.032,
                ANGLEUNIT["arc-second",4.84813681109536E-06],
                ID["EPSG",8731]],
            PARAMETER["EPSG code for Horizontal CRS",4326,
                ID["EPSG",1037]]],
        CS[vertical,1],
            AXIS["gravity-related height (H)",up,
                LENGTHUNIT["metre",1]],
        USAGE[
            SCOPE["unknown"],
            AREA["World"],
            BBOX[-90,-180,90,180]]]]"#,
        &db,
    )
    .expect("target");

    let mut ctxt = CoordinateOperationContext::create(&db, "EPSG");
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    let list = CoordinateOperationFactory::create()
        .create_operations(&src, &dst, &ctxt)
        .expect("operations");
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].name(),
        "ETRS89 to WGS 84 (1) + vertical offs. and slope"
    );
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=vertoffset +lat_0=47 +lon_0=8 +dh=-0.245 +slope_lat=-0.21 \
         +slope_lon=-0.032 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}

/// The derived projected CRS `Custom Site Calibrated CRS` of the C++ test, as its WKT.
const DERIVED_PROJECTED_WKT: &str = r#"DERIVEDPROJCRS["Custom Site Calibrated CRS",
    BASEPROJCRS["NAD83(2011) / Mississippi East (ftUS)",
        BASEGEOGCRS["NAD83(2011)",
            DATUM["NAD83 (National Spatial Reference System 2011)",
                ELLIPSOID["GRS 1980",6378137,298.257222101,
                    LENGTHUNIT["metre",1]]],
            PRIMEM["Greenwich",0,
                ANGLEUNIT["degree",0.0174532925199433]]],
        CONVERSION["SPCS83 Mississippi East zone (US Survey feet)",
            METHOD["Transverse Mercator",
                ID["EPSG",9807]],
            PARAMETER["Latitude of natural origin",29.5,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8801]],
            PARAMETER["Longitude of natural origin",-88.8333333333333,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8802]],
            PARAMETER["Scale factor at natural origin",0.99995,
                SCALEUNIT["unity",1],
                ID["EPSG",8805]],
            PARAMETER["False easting",984250,
                LENGTHUNIT["US survey foot",0.304800609601219],
                ID["EPSG",8806]],
            PARAMETER["False northing",0,
                LENGTHUNIT["US survey foot",0.304800609601219],
                ID["EPSG",8807]]]],
    DERIVINGCONVERSION["Affine transformation as PROJ-based",
        METHOD["PROJ-based operation method: +proj=pipeline +step +proj=unitconvert +xy_in=m +xy_out=us-ft +step +proj=affine +xoff=20 +step +proj=unitconvert +xy_in=us-ft +xy_out=m"]],
    CS[Cartesian,2],
        AXIS["northing (Y)",north,
            LENGTHUNIT["US survey foot",0.304800609601219]],
        AXIS["easting (X)",east,
            LENGTHUNIT["US survey foot",0.304800609601219]],
    REMARK["EPSG:6507 with 20 feet offset and axis inversion"]]"#;

/// `TEST(operation, createOperation_derived_projected_crs)`
#[test]
fn operation_create_operation_derived_projected_crs() {
    let db = DatabaseContext::create().expect("proj.db");
    let authority = AuthorityFactory::create(&db, "EPSG");
    let src = authority
        .create_coordinate_reference_system("6507")
        .expect("6507");

    let dst = wkt_crs(DERIVED_PROJECTED_WKT);

    let op = CoordinateOperationFactory::create()
        .create_operation(&src, &dst)
        .expect("operation")
        .expect("some operation");
    assert_eq!(
        proj(&db, &op),
        "+proj=pipeline \
         +step +proj=affine +xoff=20 \
         +step +proj=axisswap +order=2,1"
    );
}

/// `TEST(operation, geogCRS_to_compoundCRS_with_boundVerticalCRS_and_derivedProjected)`
#[test]
fn operation_geog_to_compound_with_bound_vertical_and_derived_projected() {
    let db = DatabaseContext::create().expect("proj.db");
    // The bound vertical CRS of `createBoundVerticalCRS()`: ODN height (EPSG:5701) bound to
    // EPSG:4979 through the EGM2008 geoid grid.
    let compound = wkt_crs(&format!(
        r#"COMPOUNDCRS["derived projected + bound vertical",
    {DERIVED_PROJECTED_WKT},
    BOUNDCRS[
        SOURCECRS[
            VERTCRS["ODN height",
                VDATUM["Ordnance Datum Newlyn",
                    ID["EPSG",5101]],
                CS[vertical,1],
                    AXIS["gravity-related height (H)",up,
                        LENGTHUNIT["metre",1]],
                ID["EPSG",5701]]],
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
        ABRIDGEDTRANSFORMATION["Transformation from ODN height to WGS 84",
            METHOD["GravityRelatedHeight to Geographic3D"],
            PARAMETERFILE["Geoid (height correction) model file",
                "us_nga_egm08_25.tif"]]]]"#
    ));
    let src = Crs::Geographic(GeographicCRS::epsg_4979());

    let op = CoordinateOperationFactory::create()
        .create_operation(&src, &compound)
        .expect("operation")
        .expect("some operation");
    assert_eq!(
        proj(&db, &op),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +inv +proj=vgridshift +grids=us_nga_egm08_25.tif +multiplier=1 \
         +step +proj=tmerc +lat_0=29.5 +lon_0=-88.8333333333333 \
         +k=0.99995 +x_0=300000 +y_0=0 +ellps=GRS80 \
         +step +proj=unitconvert +xy_in=m +xy_out=us-ft \
         +step +proj=affine +xoff=20 \
         +step +proj=axisswap +order=2,1"
    );
}

/// `TEST(operation, compoundCRS_to_compoundCRS_with_derivedVerticalCRS_ellipsoidal_height)`:
/// `EPSG:4326+5773` (WGS 84 + EGM96 height) to the compound with the derived vertical CRS of the
/// same test, through the registry, with the grids taken as available.
#[test]
fn operation_compound_to_compound_with_derived_vertical_crs_ellipsoidal_height() {
    let db = DatabaseContext::create().expect("proj.db");
    let mut ctxt = CoordinateOperationContext::create(&db, "");
    ctxt.set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    // WGS84 + EGM96
    let src = create_from_user_input("EPSG:4326+5773", &db).expect("source");
    let dst = create_from_user_input(
        r#"COMPOUNDCRS["UTM30 with vertical offset",
    PROJCRS["WGS 84 / UTM zone 30N",
        BASEGEOGCRS["WGS 84",
            ENSEMBLE["World Geodetic System 1984 ensemble",
                MEMBER["World Geodetic System 1984 (Transit)"],
                MEMBER["World Geodetic System 1984 (G730)"],
                MEMBER["World Geodetic System 1984 (G873)"],
                MEMBER["World Geodetic System 1984 (G1150)"],
                MEMBER["World Geodetic System 1984 (G1674)"],
                MEMBER["World Geodetic System 1984 (G1762)"],
                MEMBER["World Geodetic System 1984 (G2139)"],
                MEMBER["World Geodetic System 1984 (G2296)"],
                ELLIPSOID["WGS 84",6378137,298.257223563,
                    LENGTHUNIT["metre",1]],
                ENSEMBLEACCURACY[2.0]],
            PRIMEM["Greenwich",0,
                ANGLEUNIT["degree",0.0174532925199433]],
            ID["EPSG",4326]],
        CONVERSION["UTM zone 30N",
            METHOD["Transverse Mercator",
                ID["EPSG",9807]],
            PARAMETER["Latitude of natural origin",0,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8801]],
            PARAMETER["Longitude of natural origin",-3,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8802]],
            PARAMETER["Scale factor at natural origin",0.9996,
                SCALEUNIT["unity",1],
                ID["EPSG",8805]],
            PARAMETER["False easting",500000,
                LENGTHUNIT["metre",1],
                ID["EPSG",8806]],
            PARAMETER["False northing",0,
                LENGTHUNIT["metre",1],
                ID["EPSG",8807]]],
        CS[Cartesian,2],
            AXIS["(E)",east,
                ORDER[1],
                LENGTHUNIT["metre",1]],
            AXIS["(N)",north,
                ORDER[2],
                LENGTHUNIT["metre",1]],
        ID["EPSG",32630]],
    VERTCRS["Derived verticalCRS",
        BASEVERTCRS["Ellipsoid (metre)",
            VDATUM["Ellipsoid"]],
        DERIVINGCONVERSION["Conv Vertical Offset",
            METHOD["Vertical Offset",
                ID["EPSG",9616]],
            PARAMETER["Vertical Offset",42.3,
                LENGTHUNIT["metre",1],
                ID["EPSG",8603]]],
        CS[vertical,1],
            AXIS["gravity-related height (H)",up,
                LENGTHUNIT["metre",1,
                    ID["EPSG",9001]]]]]"#,
        &db,
    )
    .expect("target");
    let list = CoordinateOperationFactory::create()
        .create_operations(&src, &dst, &ctxt)
        .expect("operations");
    assert!(!list.is_empty());
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=vgridshift +grids=us_nga_egm96_15.tif +multiplier=1 \
         +step +proj=geogoffset +dh=42.3 \
         +step +proj=utm +zone=30 +ellps=WGS84"
    );
    assert_eq!(
        list[0].name(),
        "Inverse of WGS 84 to EGM96 height (1) + Conv Vertical Offset + UTM zone 30N"
    );
    assert!(!list[0].has_ballpark_transformation());
}
