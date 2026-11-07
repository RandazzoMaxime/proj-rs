//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, geographic CRS to geographic CRS
//! operations found through a context: a pivot through datum intermediates, a lon/lat
//! source, and a 3D promotion of a 2D source.
//!
//! Each function keeps the C++ `TEST()` name in its doc comment. The names are copied from
//! that file.

use proj_rs::iso19111::{
    AuthorityFactory, CoordinateOperation, CoordinateOperationContext, CoordinateOperationFactory,
    Crs, DatabaseContext, GridAvailabilityUse, SpatialCriterion, WktParser,
};

fn database() -> DatabaseContext {
    DatabaseContext::create().expect("database")
}

fn epsg(db: &DatabaseContext, code: &str) -> Crs {
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

/// `TEST(operation, geogCRS_to_geogCRS_context_lonlat_vs_latlon_crs)`
#[test]
fn operation_geog_crs_to_geog_crs_context_lonlat_vs_latlon_crs() {
    let db = database();
    let mut ctx = CoordinateOperationContext::create(&db, "EPSG");
    ctx.set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability)
        .set_spatial_criterion(SpatialCriterion::PartialIntersection);
    let list = operations(&epsg(&db, "4749"), &epsg(&db, "10310"), &ctx);
    assert_eq!(list.len(), 5);
    assert_eq!(
        list[0].name(),
        "RGNC91-93 to WGS 84 (1) + Inverse of RGNC15 to WGS 84 (1)"
    );
    // Check that we get direct transformation, and not only through WGS 84
    // The difficulty here is that the transformation is registered between
    // "RGNC91-93 (lon-lat)" et "RGNC15 (lon-lat)"
    assert_eq!(
        list[1].name(),
        "axis order change (2D) + RGNC91-93 to RGNC15 (2) + axis order change (2D)"
    );
    assert_eq!(
        list[2].name(),
        "axis order change (2D) + RGNC91-93 to RGNC15 (1) + axis order change (2D)"
    );
}

/// `TEST(operation, geogCRS_to_geogCRS_with_intermediate_no_ids)`
#[test]
fn operation_geog_crs_to_geog_crs_with_intermediate_no_ids() {
    let db = database();
    let mut ctx = CoordinateOperationContext::create(&db, "");
    ctx.set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);

    let src = wkt_crs(concat!(
        "GEOGCRS[\"input\",\n",
        "    DATUM[\"International Terrestrial Reference Frame 2014\",\n",
        "        ELLIPSOID[\"GRS 1980\",6378137,298.257222101,\n",
        "            LENGTHUNIT[\"metre\",1]],\n",
        "        ID[\"EPSG\",1165]],\n",
        "    PRIMEM[\"Greenwich\",0,\n",
        "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
        "        ID[\"EPSG\",8901]],\n",
        "    CS[ellipsoidal,3],\n",
        "        AXIS[\"longitude\",east,\n",
        "            ORDER[1],\n",
        "            ANGLEUNIT[\"degree\",0.0174532925199433,\n",
        "                ID[\"EPSG\",9122]]],\n",
        "        AXIS[\"latitude\",north,\n",
        "            ORDER[2],\n",
        "            ANGLEUNIT[\"degree\",0.0174532925199433,\n",
        "                ID[\"EPSG\",9122]]],\n",
        "        AXIS[\"ellipsoidal height (h)\",up,\n",
        "            ORDER[3],\n",
        "            LENGTHUNIT[\"metre\",1]]]"
    ));
    let dest = wkt_crs(concat!(
        "GEOGCRS[\"output\",\n",
        "    DATUM[\"Estonia 1997\",\n",
        "        ELLIPSOID[\"GRS 1980\",6378137,298.257222101,\n",
        "            LENGTHUNIT[\"metre\",1]],\n",
        "        ID[\"EPSG\",6180]],\n",
        "    PRIMEM[\"Greenwich\",0,\n",
        "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
        "        ID[\"EPSG\",8901]],\n",
        "    CS[ellipsoidal,3],\n",
        "        AXIS[\"longitude\",east,\n",
        "            ORDER[1],\n",
        "            ANGLEUNIT[\"degree\",0.0174532925199433,\n",
        "                ID[\"EPSG\",9122]]],\n",
        "        AXIS[\"latitude\",north,\n",
        "            ORDER[2],\n",
        "            ANGLEUNIT[\"degree\",0.0174532925199433,\n",
        "                ID[\"EPSG\",9122]]],\n",
        "        AXIS[\"ellipsoidal height (h)\",up,\n",
        "            ORDER[3],\n",
        "            LENGTHUNIT[\"metre\",1]]]"
    ));

    let list = operations(&src, &dest, &ctx);
    assert!(!list.is_empty());
    // Test that a non-noop operation is returned
    assert_eq!(
        list[0].name(),
        concat!(
            "Conversion from input to ITRF2014 + ITRF2014 to ETRF2014 (2) + ",
            "ETRF2014 to NKG_ETRF14 (1) + ",
            "NKG_ETRF14 to ETRS89-EST [EST97] (1) + ",
            "Conversion from ETRS89-EST [EST97] to output"
        ),
    );
}

/// `TEST(operation, geogCRS_promoted_to_3D_to_geogCRS_3D_context)`
#[test]
fn operation_geog_crs_promoted_to_3d_to_geog_crs_3d_context() {
    let db = database();
    let mut ctx = CoordinateOperationContext::create(&db, "");
    ctx.set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    {
        // NAD83 (86) promoted to 3D
        let source = epsg(&db, "4269").promote_to_3d(Some(&db));
        // NAD83 (2011) 3D
        let list = operations(&source, &epsg(&db, "6319"), &ctx);
        assert!(!list.is_empty());
        // Check we don't get an ESRI operation
        assert_eq!(
            list[0].name(),
            concat!(
                "NAD83 to NAD83(HARN) (47) + ",
                "NAD83(HARN) to NAD83(FBN) (1) + ",
                "NAD83(FBN) to NAD83(NSRS2007) (1) + ",
                "NAD83(NSRS2007) to NAD83(2011) (1)"
            ),
        );
    }
    {
        // NAD83 (86) promoted to 3D
        let target = epsg(&db, "4269").promote_to_3d(Some(&db));
        let list = operations(&epsg(&db, "6319"), &target, &ctx);
        assert!(!list.is_empty());
        // Check we don't get an ESRI operation
        assert_eq!(
            list[0].name(),
            concat!(
                "Inverse of NAD83(NSRS2007) to NAD83(2011) (1) + ",
                "Inverse of NAD83(FBN) to NAD83(NSRS2007) (1) + ",
                "Inverse of NAD83(HARN) to NAD83(FBN) (1) + ",
                "Inverse of NAD83 to NAD83(HARN) (47)"
            ),
        );
    }
}
