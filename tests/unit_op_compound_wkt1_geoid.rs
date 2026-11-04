//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`: compound CRS read from WKT1 with a
//! `PROJ4_GRIDS` vertical datum, in the EPSG context.

use proj_rs::{
    CoordinateOperationContext, CoordinateOperationFactory, Crs, DatabaseContext,
    GridAvailabilityUse, ProjLines, SpatialCriterion, WktObject, WktParser,
};

const SOURCE: &str = r####"COMPD_CS["NAD83 / Alabama West + NAVD88 height - Geoid12B (Meters)",
    PROJCS["NAD83 / Alabama West",
        GEOGCS["NAD83",
            DATUM["North_American_Datum_1983",
                SPHEROID["GRS 1980",6378137,298.257222101,
                    AUTHORITY["EPSG","7019"]],
                TOWGS84[0,0,0,0,0,0,0],
                AUTHORITY["EPSG","6269"]],
            PRIMEM["Greenwich",0,
                AUTHORITY["EPSG","8901"]],
            UNIT["degree",0.0174532925199433,
                AUTHORITY["EPSG","9122"]],
            AUTHORITY["EPSG","4269"]],
        PROJECTION["Transverse_Mercator"],
        PARAMETER["latitude_of_origin",30],
        PARAMETER["central_meridian",-87.5],
        PARAMETER["scale_factor",0.999933333],
        PARAMETER["false_easting",600000],
        PARAMETER["false_northing",0],
        UNIT["metre",1,
            AUTHORITY["EPSG","9001"]],
        AXIS["X",EAST],
        AXIS["Y",NORTH],
        AUTHORITY["EPSG","26930"]],
    VERT_CS["NAVD88 height",
        VERT_DATUM["North American Vertical Datum 1988",2005,
            EXTENSION["PROJ4_GRIDS","g2012a_alaska.gtx,g2012a_hawaii.gtx,g2012a_conus.gtx"],
            AUTHORITY["EPSG","5103"]],
        UNIT["metre",1,
            AUTHORITY["EPSG","9001"]],
        AXIS["Gravity-related height",UP],
        AUTHORITY["EPSG","5703"]]]"####;
const DESTINATION: &str = r####"COMPD_CS["NAD_1983_StatePlane_Alabama_West_FIPS_0102_Feet + NAVD88 height - Geoid12B (US Feet)",
    PROJCS["NAD_1983_StatePlane_Alabama_West_FIPS_0102_Feet",
        GEOGCS["NAD83",
            DATUM["North_American_Datum_1983",
                SPHEROID["GRS 1980",6378137,298.257222101,
                    AUTHORITY["EPSG","7019"]],
                TOWGS84[0,0,0,0,0,0,0],
                AUTHORITY["EPSG","6269"]],
            PRIMEM["Greenwich",0],
            UNIT["Degree",0.0174532925199433]],
        PROJECTION["Transverse_Mercator"],
        PARAMETER["latitude_of_origin",30],
        PARAMETER["central_meridian",-87.5],
        PARAMETER["scale_factor",0.999933333333333],
        PARAMETER["false_easting",1968500],
        PARAMETER["false_northing",0],
        UNIT["US survey foot",0.304800609601219,
            AUTHORITY["EPSG","9003"]],
        AXIS["Easting",EAST],
        AXIS["Northing",NORTH],
        AUTHORITY["ESRI","102630"]],
    VERT_CS["NAVD88 height (ftUS)",
        VERT_DATUM["North American Vertical Datum 1988",2005,
            EXTENSION["PROJ4_GRIDS","g2012a_alaska.gtx,g2012a_hawaii.gtx,g2012a_conus.gtx"],
            AUTHORITY["EPSG","5103"]],
        UNIT["US survey foot",0.304800609601219,
            AUTHORITY["EPSG","9003"]],
        AXIS["Gravity-related height",UP],
        AUTHORITY["EPSG","6360"]]]"####;
fn parse(wkt: &str) -> Crs {
    match WktParser::new().create_from_wkt(wkt).expect("WKT") {
        WktObject::Crs(crs) => crs,
        other => panic!("expected a CRS: {other:?}"),
    }
}

fn context(db: &DatabaseContext, authority: &str) -> CoordinateOperationContext {
    let mut context = CoordinateOperationContext::create(db, authority);
    context
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability)
        .set_spatial_criterion(SpatialCriterion::PartialIntersection);
    context
}

/// `TEST(operation, compoundCRS_to_compoundCRS_with_bound_crs_in_horiz_and_vert_WKT1_same_geoidgrids_context)`
#[test]
fn operation_compound_to_compound_with_bound_crs_wkt1_same_geoidgrids_context() {
    let db = DatabaseContext::create().expect("proj.db");
    let list = CoordinateOperationFactory::create()
        .create_operations(&parse(SOURCE), &parse(DESTINATION), &context(&db, "EPSG"))
        .expect("operations");
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].export_to_proj_string_with_database(&db, ProjLines::Single).expect("PROJ string"),
        "+proj=pipeline +step +inv +proj=tmerc +lat_0=30 +lon_0=-87.5 +k=0.999933333 +x_0=600000 +y_0=0 +ellps=GRS80 +step +proj=unitconvert +z_in=m +z_out=us-ft +step +proj=tmerc +lat_0=30 +lon_0=-87.5 +k=0.999933333333333 +x_0=600000 +y_0=0 +ellps=GRS80 +step +proj=unitconvert +xy_in=m +xy_out=us-ft"
    );
}
