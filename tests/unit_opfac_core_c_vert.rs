//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, the vertical and geoid cases of
//! lot C: a compound CRS whose vertical CRS is a height, depth, or geoid-model CRS,
//! to a geographic CRS.
//!
//! Each function keeps the C++ `TEST()` name in its doc comment. Expected names and
//! PROJ strings are copied from that file. The cases still missing are listed in
//! `docs/PORTING-lotc-vertical.md`.

use proj_rs::iso19111::{
    create_from_user_input, AuthorityFactory, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, Crs, DatabaseContext, GridAvailabilityUse, ProjLines,
    SpatialCriterion,
};

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

/// TEST(operation, compoundCRS_of_vertCRS_with_geoid_model_by_name_and_several_records_to_geogCRS)
#[test]
fn operation_compound_crs_of_vert_crs_with_geoid_model_by_name_and_several_records_to_geog_crs() {
    let db = database();
    let ctx = partial_ignore_ctx(&db, "EPSG");
    let src = parse(
        &db,
        r#"COMPOUNDCRS["Compound CRS NAD83(2011) / Puerto Rico and Virgin Is. + VIVD09 height + PROJ us_noaa_g2012bp0.tif",PROJCRS["NAD83(2011) / Puerto Rico and Virgin Is.",BASEGEOGCRS["NAD83(2011)",DATUM["NAD83 (National Spatial Reference System 2011)",ELLIPSOID["GRS 1980",6378137,298.257222101,LENGTHUNIT["metre",1]]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]],ID["EPSG",6318]],CONVERSION["SPCS83Puerto Rico & Virgin Islands zone (meter)",METHOD["Lambert Conic Conformal (2SP)",ID["EPSG",9802]],PARAMETER["Latitude of false origin",17.8333333333333,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8821]],PARAMETER["Longitude of false  origin",-66.4333333333333,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8822]],PARAMETER["Latitude of 1st standard parallel",18.4333333333333,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8823]],PARAMETER["Latitude of 2nd standard parallel",18.0333333333333,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8824]],PARAMETER["Easting at false origin",200000,LENGTHUNIT["metre",1],ID["EPSG",8826]],PARAMETER["Northing at false origin",200000,LENGTHUNIT["metre",1],ID["EPSG",8827]],ID["EPSG",15230]],CS[Cartesian,2],AXIS["easting (X)",east,ORDER[1],LENGTHUNIT["metre",1,ID["EPSG",9001]]],AXIS["northing (Y)",north,ORDER[2],LENGTHUNIT["metre",1,ID["EPSG",9001]]]],VERTCRS["VIVD09 height + PROJ us_noaa_g2012bp0.tif",VDATUM["Virgin Islands Vertical Datum of 2009",ID["EPSG",1124]],CS[vertical,1],AXIS["gravity-related height (H)",up,LENGTHUNIT["metre",1,ID["EPSG",9001]]],GEOIDMODEL["PROJ us_noaa_g2012bp0.tif"]]]"#,
    );
    // NAD83(2011) 3d
    let dst = epsg(&db, "6319");

    let list = operations(&src, &dst, &ctx);
    assert!(!list.is_empty());
    assert_eq!(
        list[0].name(),
        "Inverse of SPCS83Puerto Rico & Virgin Islands zone (meter) + \
         Transformation from VIVD09 height + \
         PROJ us_noaa_g2012bp0.tif to NAD83(2011)"
    );
    assert_eq!(
        plain(&list[0]),
        "+proj=pipeline \
         +step +inv +proj=lcc +lat_0=17.8333333333333 \
         +lon_0=-66.4333333333333 +lat_1=18.4333333333333 \
         +lat_2=18.0333333333333 +x_0=200000 +y_0=200000 +ellps=GRS80 \
         +step +proj=vgridshift +grids=us_noaa_g2012bp0.tif \
         +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
    let domain = list[0].domains();
    assert_eq!(domain.len(), 1);
    assert_eq!(
        domain[0].description(),
        Some("US Virgin Islands - onshore - St Croix, St John, and St Thomas.")
    );
}
