//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, lot `opfac-c`: the compound,
//! bound and PROJ-string `createOperation(s)` cases that this crate reproduces.
//!
//! Each function keeps the C++ `TEST()` name in its doc comment. Expected names
//! and PROJ strings are copied from that file. The other cases of the lot are
//! listed in `docs/PORTING-opfac-core-c.md`.

use proj_rs::iso19111::{
    create_crs_from_proj_string, create_from_user_input, AuthorityFactory, CoordinateOperation,
    CoordinateOperationContext, CoordinateOperationFactory, Crs, DatabaseContext,
    GridAvailabilityUse, ProjLines, SpatialCriterion,
};

const NAD83_CSRS_HT2_0_COMPD: &str = r#"COMPD_CS["NAD83(CSRS) + CGVD28 height - HT2_0",GEOGCS["NAD83(CSRS)",DATUM["NAD83_Canadian_Spatial_Reference_System",SPHEROID["GRS 1980",6378137,298.257222101,AUTHORITY["EPSG","7019"]],TOWGS84[0,0,0,0,0,0,0],AUTHORITY["EPSG","6140"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]],AUTHORITY["EPSG","4617"]],VERT_CS["CGVD28 height - HT2_0",VERT_DATUM["Canadian Geodetic Vertical Datum of 1928",2005,EXTENSION["PROJ4_GRIDS","HT2_0.gtx"],AUTHORITY["EPSG","5114"]],UNIT["metre",1,AUTHORITY["EPSG","9001"]],AXIS["Gravity-related height",UP],AUTHORITY["EPSG","5713"]]]"#;

fn database() -> DatabaseContext {
    DatabaseContext::create().expect("database")
}

fn epsg(db: &DatabaseContext, code: &str) -> Crs {
    AuthorityFactory::create(db, "EPSG")
        .create_coordinate_reference_system(code)
        .expect(code)
}

fn parse(db: &DatabaseContext, text: &str) -> Crs {
    create_from_user_input(text, db).expect(text)
}

fn proj_crs(text: &str) -> Crs {
    create_crs_from_proj_string(text).expect(text)
}

/// Context on `authority`, partial intersection, grids taken as available.
fn partial_ignore_ctx(db: &DatabaseContext, authority: &str) -> CoordinateOperationContext {
    let mut ctx = CoordinateOperationContext::create(db, authority);
    ctx.set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    ctx
}

fn operations(
    source: &Crs,
    target: &Crs,
    ctx: &CoordinateOperationContext,
) -> Vec<CoordinateOperation> {
    CoordinateOperationFactory::create()
        .create_operations(source, target, ctx)
        .expect("operations")
}

/// `exportToPROJString(PROJStringFormatter::create())`, without database.
fn plain(operation: &CoordinateOperation) -> String {
    match operation {
        CoordinateOperation::Conversion(conversion) => conversion.export_to_proj_string(),
        CoordinateOperation::Transformation(transformation) => {
            transformation.export_to_proj_string(ProjLines::Single)
        }
        CoordinateOperation::Concatenated(concatenated) => {
            concatenated.export_to_proj_string(ProjLines::Single)
        }
    }
    .expect("PROJ string")
}

/// TEST(operation, compoundCRS_of_horizCRS_with_TOWGS84_vertCRS_with_geoid_model_to_geogCRS)
#[test]
fn operation_compound_crs_of_horiz_crs_with_towgs84_vert_crs_with_geoid_model_to_geog_crs() {
    let db = database();
    let ctx = partial_ignore_ctx(&db, "EPSG");
    let src = parse(&db, NAD83_CSRS_HT2_0_COMPD);
    // NAD83(CSRS) 3D
    let dst = epsg(&db, "4955");

    let list = operations(&src, &dst, &ctx);
    assert_eq!(list.len(), 1);
    assert_eq!(
        plain(&list[0]),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=vgridshift +grids=HT2_0.gtx +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}

/// TEST(operation, createOperation_ETRS89_to_Amersfoort)
#[test]
fn operation_create_operation_etrs89_to_amersfoort() {
    let db = database();
    let ctx = partial_ignore_ctx(&db, "");
    // ETRS89
    let etrs89 = epsg(&db, "4258");
    // Amersfoort
    let amersfoort = epsg(&db, "4289");

    let list = operations(&etrs89, &amersfoort, &ctx);
    assert!(!list.is_empty());
    // The most precise "Amersfoort to ETRS89 (9)" operation.
    assert_eq!(list[0].name(), "Inverse of Amersfoort to ETRS89 (9)");

    let list = operations(&amersfoort, &etrs89, &ctx);
    assert!(!list.is_empty());
    assert_eq!(list[0].name(), "Amersfoort to ETRS89 (9)");
}

/// TEST(operation, proj3DCRS_with_non_meter_horiz_and_vertical_to_geog)
#[test]
fn operation_proj3_d_crs_with_non_meter_horiz_and_vertical_to_geog() {
    let db = database();
    let src = proj_crs("+proj=utm +zone=31 +datum=WGS84 +units=us-ft +vunits=us-ft +type=crs");
    let ctx = CoordinateOperationContext::create(&db, "EPSG");

    let list = operations(&src, &epsg(&db, "4326"), &ctx);
    assert_eq!(list.len(), 1);
    // The vertical unit conversion is done once.
    assert_eq!(
        plain(&list[0]),
        "+proj=pipeline \
         +step +proj=unitconvert +xy_in=us-ft +z_in=us-ft \
         +xy_out=m +z_out=m \
         +step +inv +proj=utm +zone=31 +ellps=WGS84 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}

/// Operation from `source` to `target` with the default context, the first one.
fn first_operation(db: &DatabaseContext, source: &Crs, target: &Crs) -> CoordinateOperation {
    let ctx = CoordinateOperationContext::create(db, "");
    operations(source, target, &ctx)
        .into_iter()
        .next()
        .expect("operation")
}

/// TEST(operation, createOperation_fallback_to_proj4_strings)
#[test]
fn operation_create_operation_fallback_to_proj4_strings() {
    let db = database();
    let dest = proj_crs("+proj=longlat +over +datum=WGS84 +type=crs");
    let op = first_operation(&db, &epsg(&db, "4326"), &dest);
    assert_eq!(
        plain(&op),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=longlat +over +datum=WGS84 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg"
    );
}

/// TEST(operation, createOperation_fallback_to_proj4_strings_bound_of_geog)
#[test]
fn operation_create_operation_fallback_to_proj4_strings_bound_of_geog() {
    let db = database();
    let src = proj_crs("+proj=longlat +over +ellps=GRS80 +towgs84=0,0,0 +type=crs");
    let dest = proj_crs("+proj=longlat +over +ellps=clrk66 +towgs84=0,0,0 +type=crs");
    let op = first_operation(&db, &src, &dest);
    assert_eq!(
        plain(&op),
        "+proj=pipeline \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +inv +proj=longlat +over +ellps=GRS80 +towgs84=0,0,0 \
         +step +proj=longlat +over +ellps=clrk66 +towgs84=0,0,0 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg"
    );
}

/// TEST(operation, createOperation_fallback_to_proj4_strings_regular_with_datum_to_projliteral)
#[test]
fn operation_create_operation_fallback_to_proj4_strings_regular_with_datum_to_projliteral() {
    let db = database();
    let src = proj_crs("+proj=utm +zone=11 +datum=NAD27 +type=crs");
    let dst = proj_crs("+proj=longlat +datum=WGS84 +over +type=crs");
    let op = first_operation(&db, &src, &dst);
    assert_eq!(
        plain(&op),
        "+proj=pipeline \
         +step +inv +proj=utm +zone=11 +datum=NAD27 \
         +step +proj=longlat +datum=WGS84 +over \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg"
    );
}

/// TEST(operation, createOperation_fallback_to_proj4_strings_geog_NAD83_to_projliteral)
#[test]
fn operation_create_operation_fallback_to_proj4_strings_geog_nad83_to_projliteral() {
    let db = database();
    let src = proj_crs("+proj=longlat +datum=NAD83 +type=crs");
    let dst = proj_crs("+proj=longlat +datum=WGS84 +over +type=crs");
    let op = first_operation(&db, &src, &dst);
    assert_eq!(
        plain(&op),
        "+proj=pipeline \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=longlat +datum=WGS84 +over \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg"
    );
}

/// TEST(operation, createOperation_fallback_to_proj4_strings_regular_with_nadgrids_to_projliteral)
#[test]
fn operation_create_operation_fallback_to_proj4_strings_regular_with_nadgrids_to_projliteral() {
    let db = database();
    let src = proj_crs("+proj=utm +zone=11 +ellps=clrk66 +nadgrids=@conus +type=crs");
    let dst = proj_crs("+proj=longlat +datum=WGS84 +over +type=crs");
    let op = first_operation(&db, &src, &dst);
    assert_eq!(
        plain(&op),
        "+proj=pipeline \
         +step +inv +proj=utm +zone=11 +ellps=clrk66 +nadgrids=@conus \
         +step +proj=longlat +datum=WGS84 +over \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg"
    );
}

/// TEST(operation, createOperation_fallback_to_proj4_strings_projliteral_to_projliteral)
#[test]
fn operation_create_operation_fallback_to_proj4_strings_projliteral_to_projliteral() {
    let db = database();
    let src = proj_crs("+proj=utm +zone=11 +datum=NAD27 +over +type=crs");
    let dst = proj_crs("+proj=longlat +datum=WGS84 +over +type=crs");
    let op = first_operation(&db, &src, &dst);
    assert_eq!(
        plain(&op),
        "+proj=pipeline \
         +step +inv +proj=utm +zone=11 +datum=NAD27 +over \
         +step +proj=longlat +datum=WGS84 +over \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg"
    );
}

/// TEST(operation, createOperation_fallback_to_proj4_strings_regular_to_projliteral_with_towgs84)
#[test]
fn operation_create_operation_fallback_to_proj4_strings_regular_to_projliteral_with_towgs84() {
    let db = database();
    let src = epsg(&db, "4326");
    let dst = proj_crs("+proj=utm +zone=31 +ellps=GRS80 +towgs84=1,2,3 +over +type=crs");
    let op = first_operation(&db, &src, &dst);
    assert_eq!(
        plain(&op),
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=utm +zone=31 +ellps=GRS80 +towgs84=1,2,3 +over"
    );
}
