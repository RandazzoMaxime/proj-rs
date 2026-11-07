//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`: compound CRS whose PROJ string, PROJJSON input,
//! or velocity-grid method is checked. The C++ `TEST()` names are kept in the doc comments and the
//! expected names and PROJ strings are copied from that file.

use proj_rs::iso19111::{
    create_from_user_input, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, Crs, DatabaseContext, GridAvailabilityUse, ProjLines,
    SpatialCriterion, WktObject, WktParser,
};

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

/// `VELOCITY_SRC`: the C++ string literal, concatenated.
const VELOCITY_SRC: &str = r#"COMPOUNDCRS["NAD83(CSRS)v3 / MTM zone 7 + CGVD28 height",
    PROJCRS["NAD83(CSRS)v3 / MTM zone 7",
        BASEGEOGCRS["NAD83(CSRS)v3",
            DATUM["North American Datum of 1983 (CSRS) version 3",
                ELLIPSOID["GRS 1980",6378137,298.257222101,
                    LENGTHUNIT["metre",1]]],
            PRIMEM["Greenwich",0,
                ANGLEUNIT["degree",0.0174532925199433]],
            ID["EPSG",8240]],
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
                LENGTHUNIT["metre",1,
                    ID["EPSG",9001]]],
            AXIS["northing (N(Y))",north,
                ORDER[2],
                LENGTHUNIT["metre",1,
                    ID["EPSG",9001]]]],
    VERTCRS["CGVD28 height",
        VDATUM["Canadian Geodetic Vertical Datum of 1928"],
        CS[vertical,1],
            AXIS["gravity-related height (H)",up,
                LENGTHUNIT["metre",1]],
        GEOIDMODEL["HT2_1997",
            ID["EPSG",9983]],
        ID["EPSG",5713]]]"#;

/// `TEST(operation, createOperation_compound_to_compound_with_Geographic3D_Offset_by_velocity_grid)`
#[test]
fn operation_create_operation_compound_to_compound_with_geographic_3d_offset_by_velocity_grid() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = wkt_crs(&db, VELOCITY_SRC);
    let dst = from_user_input(&db, "NAD83(CSRS)v7 / UTM zone 19 + CGVD2013a(2010) height");
    let ctxt = {
        let mut ctxt = CoordinateOperationContext::create(&db, "");
        ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection);
        ctxt.set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
        ctxt
    };
    let list = operations(&src, &dst, &ctxt);
    assert!(!list.is_empty());
    assert!(!list[0].has_ballpark_transformation());
    // Very similar output pipeline as
    // createOperation_compound_to_compound_with_point_motion_operation
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline \
         +step +inv +proj=tmerc +lat_0=0 +lon_0=-70.5 +k=0.9999 \
         +x_0=304800 +y_0=0 +ellps=GRS80 \
         +step +proj=vgridshift +grids=ca_nrc_HT2_1997.tif +multiplier=1 \
         +step +proj=cart +ellps=GRS80 \
         +step +proj=deformation +dt=13 \
         +grids=ca_nrc_NAD83v70VG.tif \
         +ellps=GRS80 \
         +step +inv +proj=cart +ellps=GRS80 \
         +step +inv +proj=vgridshift +grids=ca_nrc_CGG2013an83.tif \
         +multiplier=1 \
         +step +proj=utm +zone=19 +ellps=GRS80"
    );
}
