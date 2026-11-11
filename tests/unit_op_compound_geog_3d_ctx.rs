//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`: three `TEST()`s of the compound to geographic
//! 3D family, with the database context. The expected counts, names and PROJ strings are copied
//! from the C++ file. The reference commands are in `docs/PORTING-unit-op-compound-geog-3d-ctx.md`.

use proj_rs::iso19111::{
    create_from_user_input, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, Crs, DatabaseContext, GridAvailabilityUse, ProjLines,
    SpatialCriterion, WktObject, WktParser,
};

/// `createFromUserInput("EPSG:<code>")`: a compound code such as `2225+6360` is accepted.
fn epsg(db: &DatabaseContext, code: &str) -> Crs {
    create_from_user_input(&format!("EPSG:{code}"), db).expect(code)
}

/// The context of the three tests: partial intersection, grids ignored.
fn context(db: &DatabaseContext, authority: &str) -> CoordinateOperationContext {
    let mut ctxt = CoordinateOperationContext::create(db, authority);
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    ctxt
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

/// `exportToPROJString(PROJStringFormatter::create())`: no database, so the grid names are the
/// ones the operation carries.
fn proj_without_db(operation: &CoordinateOperation) -> String {
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

fn wkt_crs(db: &DatabaseContext, text: &str) -> Crs {
    let mut parser = WktParser::new();
    parser.attach_database(db.clone());
    match parser.create_from_wkt(text).expect("WKT") {
        WktObject::Crs(crs) => crs,
        _ => panic!("not a CRS"),
    }
}

/// `TEST(operation, compoundCRS_of_projCRS_to_geogCRS_3D_context)`
#[test]
fn operation_compound_of_proj_crs_to_geog_crs_3d_context() {
    let db = DatabaseContext::create().expect("proj.db");
    // SPCS83 California zone 1 (US Survey feet) + NAVD88 height (ftUS)
    let src = epsg(&db, "2225+6360");
    // NAD83, promoted to 3D
    let dst = epsg(&db, "4269").promote_to_3d(Some(&db));
    let list = operations(&src, &dst, &context(&db, "EPSG"));
    // The checked value is not that important, but in case this changes,
    // likely due to a EPSG upgrade, worth checking
    // We want to make sure that the horizontal adjustments before and after
    // the vertical transformation are the reverse of each other, and there are
    // not mixes with different alternative operations (like California grid
    // forward and Nevada grid reverse)
    assert_eq!(list.len(), 16);
    assert!(!list.is_empty());

    // Check that unit conversion is OK
    assert_eq!(
        proj_without_db(&list[0]),
        "+proj=pipeline \
         +step +proj=unitconvert +xy_in=us-ft +xy_out=m \
         +step +inv +proj=lcc +lat_0=39.3333333333333 +lon_0=-122 \
         +lat_1=41.6666666666667 +lat_2=40 +x_0=2000000.0001016 \
         +y_0=500000.0001016 +ellps=GRS80 \
         +step +proj=hgridshift +grids=us_noaa_cnhpgn.tif \
         +step +proj=gridshift +no_z_transform \
         +grids=us_noaa_nadcon5_nad83_harn_nad83_fbn_conus.tif \
         +step +proj=unitconvert +z_in=us-ft +z_out=m \
         +step +proj=vgridshift +grids=us_noaa_geoid03_conus.tif \
         +multiplier=1 \
         +step +inv +proj=gridshift \
         +grids=us_noaa_nadcon5_nad83_harn_nad83_fbn_conus.tif \
         +step +inv +proj=hgridshift +grids=us_noaa_cnhpgn.tif \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}

/// `TEST(operation, WGS84_G1762_to_compoundCRS_with_bound_vertCRS)`
#[test]
fn operation_wgs84_g1762_to_compound_crs_with_bound_vert_crs() {
    let db = DatabaseContext::create().expect("proj.db");
    // WGS 84 (G1762) 3D
    let src = epsg(&db, "7665");
    // A geographic CRS with a bound vertical part, through a geoid grid named with `@`
    let dst = create_from_user_input(
        "+proj=longlat +datum=NAD83 +geoidgrids=@foo.gtx +type=crs",
        &db,
    )
    .expect("bound geographic CRS");
    // authority "" (`std::string()` in the C++ test)
    let list = operations(&src, &dst, &context(&db, ""));
    assert!(list.len() >= 53);
    assert_eq!(
        proj_without_db(&list[0]),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +inv +proj=vgridshift +grids=@foo.gtx +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg"
    );
}

/// `TEST(operation, compoundCRS_with_boundGeogCRS_and_geoid_to_geodCRS_NAD2011_ctxt)`
#[test]
fn operation_compound_with_bound_geog_crs_and_geoid_to_geod_crs_nad2011_ctxt() {
    let db = DatabaseContext::create().expect("proj.db");
    // NAD83 / California zone 5 (ftUS) + NAVD88 height (Geoid12B, ftUS), with a bound datum
    let src = wkt_crs(
        &db,
        r#"COMPD_CS["NAD83 / California zone 5 (ftUS) + NAVD88 height - Geoid12B (ftUS)",
    PROJCS["NAD83 / California zone 5 (ftUS)",
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
        PROJECTION["Lambert_Conformal_Conic_2SP"],
        PARAMETER["standard_parallel_1",35.46666666666667],
        PARAMETER["standard_parallel_2",34.03333333333333],
        PARAMETER["latitude_of_origin",33.5],
        PARAMETER["central_meridian",-118],
        PARAMETER["false_easting",6561666.667],
        PARAMETER["false_northing",1640416.667],
        UNIT["US survey foot",0.3048006096012192,
            AUTHORITY["EPSG","9003"]],
        AXIS["X",EAST],
        AXIS["Y",NORTH],
        AUTHORITY["EPSG","2229"]],
    VERT_CS["NAVD88 height - Geoid12B (ftUS)",
        VERT_DATUM["North American Vertical Datum 1988",2005,
            AUTHORITY["EPSG","5103"]],
        UNIT["US survey foot",0.3048006096012192,
            AUTHORITY["EPSG","9003"]],
        AXIS["Gravity-related height",UP],
        AUTHORITY["EPSG","6360"]]]"#,
    );
    // NAD83(2011) geocentric
    let dst = epsg(&db, "6317");
    // authority "" (`std::string()` in the C++ test)
    let list = operations(&src, &dst, &context(&db, ""));
    let mut found = false;
    for op in &list {
        if op.name()
            == "Inverse of unknown + \
             Transformation from NAD83 to WGS84 + \
             Inverse of NAD83(2011) to WGS 84 (1) + \
             Conversion from NAVD88 height (ftUS) to NAVD88 height + \
             Inverse of NAD83(2011) to NAVD88 height (1) + \
             Conversion from NAD83(2011) (geog3D) to NAD83(2011) (geocentric)"
        {
            found = true;
            assert_eq!(
                proj_without_db(op),
                "+proj=pipeline \
                 +step +proj=unitconvert +xy_in=us-ft +xy_out=m \
                 +step +inv +proj=lcc +lat_0=33.5 +lon_0=-118 \
                 +lat_1=35.4666666666667 +lat_2=34.0333333333333 \
                 +x_0=2000000.0001016 +y_0=500000.0001016 +ellps=GRS80 \
                 +step +proj=unitconvert +z_in=us-ft +z_out=m \
                 +step +proj=vgridshift +grids=us_noaa_g2012bu0.tif \
                 +multiplier=1 \
                 +step +proj=cart +ellps=GRS80"
            );
        }
    }
    assert!(found);
}
