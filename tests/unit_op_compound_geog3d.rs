//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, `compoundCRS_to_geogCRS_3D_context`: the five
//! blocks of the C++ `TEST()`, in one test. A compound CRS (NAD27 or NAD83 horizontal part with a
//! vertical CRS) to WGS 84 (3D), in the EPSG context, grids ignored. The expected names, counts
//! and PROJ strings are copied from the C++ file. The reference commands are in
//! `docs/PORTING-unit-op-compound-geog-3d-ctx.md`.

use proj_rs::iso19111::{
    create_from_user_input, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, Crs, DatabaseContext, GridAvailabilityUse, ProjLines,
    SpatialCriterion,
};

/// `createFromUserInput("EPSG:<code>")`: a compound code such as `4269+5703` is accepted.
fn epsg(db: &DatabaseContext, code: &str) -> Crs {
    create_from_user_input(&format!("EPSG:{code}"), db).expect(code)
}

/// The EPSG context of the blocks: grids ignored, the spatial criterion of the block.
fn context(db: &DatabaseContext, criterion: SpatialCriterion) -> CoordinateOperationContext {
    let mut ctxt = CoordinateOperationContext::create(db, "EPSG");
    ctxt.set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability)
        .set_spatial_criterion(criterion);
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

/// `exportToPROJString` with the database: the grid names come from it.
fn proj_with_db(db: &DatabaseContext, operation: &CoordinateOperation) -> String {
    operation
        .export_to_proj_string_with_database(db, ProjLines::Single)
        .expect("PROJ string")
}

/// `TEST(operation, compoundCRS_to_geogCRS_3D_context)`
#[test]
fn operation_compound_to_geog_crs_3d_context() {
    let db = DatabaseContext::create().expect("proj.db");

    // CompoundCRS to Geog3DCRS, with vertical unit change, but without
    // ellipsoid height <--> vertical height correction
    {
        let list = operations(
            &epsg(&db, "7406"), // NAD27 + NGVD29 height (ftUS)
            &epsg(&db, "4979"), // WGS 84
            &context(&db, SpatialCriterion::StrictContainment),
        );
        assert!(!list.is_empty());
        assert!(list[0].has_ballpark_transformation());
        assert_eq!(
            list[0].name(),
            "NAD27 to WGS 84 (79) + Transformation from NGVD29 height (ftUS) to WGS 84 (ballpark vertical transformation, without ellipsoid height to vertical height correction)"
        );
        assert_eq!(
            proj_with_db(&db, &list[0]),
            "+proj=pipeline +step +proj=axisswap +order=2,1 \
             +step +proj=unitconvert +xy_in=deg +xy_out=rad \
             +step +proj=hgridshift +grids=us_noaa_conus.tif \
             +step +proj=unitconvert \
             +xy_in=rad +z_in=us-ft +xy_out=deg +z_out=m +step \
             +proj=axisswap +order=2,1"
        );
    }

    // CompoundCRS to Geog3DCRS, with same vertical unit, and with
    // direct ellipsoid height <--> vertical height correction and
    // direct horizontal transform (no-op here)
    {
        let list = operations(
            &epsg(&db, "5500"), // NAD83(NSRS2007) + NAVD88 height
            &epsg(&db, "4979"), // WGS 84
            &context(&db, SpatialCriterion::StrictContainment),
        );
        assert!(list.len() >= 2);
        assert_eq!(
            list[1].name(),
            "Inverse of NAD83(NSRS2007) to NAVD88 height (1) + NAD83(NSRS2007) to WGS 84 (1)"
        );
        assert_eq!(
            proj_with_db(&db, &list[1]),
            "+proj=pipeline \
             +step +proj=axisswap +order=2,1 \
             +step +proj=unitconvert +xy_in=deg +xy_out=rad \
             +step +proj=vgridshift +grids=us_noaa_geoid09_conus.tif \
             +multiplier=1 \
             +step +proj=unitconvert +xy_in=rad +xy_out=deg \
             +step +proj=axisswap +order=2,1"
        );
        assert_eq!(
            list[1].remarks(),
            "For NAD83(NSRS2007) to NAVD88 height (1) (EPSG:9173): Uses \
             Geoid09 hybrid model. Replaced by 2012 model (CT code 6326).\n\
             For NAD83(NSRS2007) to WGS 84 (1) (EPSG:15931): Approximation \
             assuming that NAD83(NSRS2007) is equivalent to WGS 84 within the \
             accuracy of the transformation."
        );
    }

    // NAD83 + NAVD88 height --> WGS 84
    {
        let list = operations(
            &epsg(&db, "4269+5703"), // NAD83 + NAVD88 height
            &epsg(&db, "4979"),      // WGS 84
            &context(&db, SpatialCriterion::PartialIntersection),
        );
        assert!(list.len() >= 2);
        assert_eq!(
            list[0].name(),
            "NAD83 to NAD83(HARN) (47) + \
             NAD83(HARN) to NAD83(FBN) (1) + \
             Inverse of NAD83(FBN) to NAVD88 height (1) + \
             Inverse of NAD83(HARN) to NAD83(FBN) (1) + \
             NAD83(HARN) to WGS 84 (3)"
        );
        assert_eq!(
            proj_with_db(&db, &list[0]),
            "+proj=pipeline \
             +step +proj=axisswap +order=2,1 \
             +step +proj=unitconvert +xy_in=deg +xy_out=rad \
             +step +proj=gridshift \
             +grids=us_noaa_nadcon5_nad83_1986_nad83_harn_conus.tif \
             +step +proj=gridshift +no_z_transform \
             +grids=us_noaa_nadcon5_nad83_harn_nad83_fbn_conus.tif \
             +step +proj=vgridshift +grids=us_noaa_geoid03_conus.tif \
             +multiplier=1 \
             +step +inv +proj=gridshift \
             +grids=us_noaa_nadcon5_nad83_harn_nad83_fbn_conus.tif \
             +step +proj=cart +ellps=GRS80 \
             +step +proj=helmert +x=-0.991 +y=1.9072 +z=0.5129 \
             +rx=-0.0257899075194932 \
             +ry=-0.0096500989602704 +rz=-0.0116599432323421 +s=0 \
             +convention=coordinate_frame \
             +step +inv +proj=cart +ellps=WGS84 \
             +step +proj=unitconvert +xy_in=rad +xy_out=deg \
             +step +proj=axisswap +order=2,1"
        );
    }

    // Another variation, but post horizontal adjustment is in two steps
    {
        let list = operations(
            &epsg(&db, "6318+5703"), // NAD83(2011) + NAVD88 height
            &epsg(&db, "4985"),      // WGS 72 3D
            &context(&db, SpatialCriterion::PartialIntersection),
        );
        assert!(!list.is_empty());
        assert_eq!(
            list[0].name(),
            "Inverse of NAD83(2011) to NAVD88 height (3) + \
             NAD83(2011) to WGS 84 (1) + \
             Inverse of WGS 72 to WGS 84 (2)"
        );
        assert_eq!(
            proj_with_db(&db, &list[0]),
            "+proj=pipeline \
             +step +proj=axisswap +order=2,1 \
             +step +proj=unitconvert +xy_in=deg +xy_out=rad \
             +step +proj=vgridshift +grids=us_noaa_g2018u0.tif \
             +multiplier=1 \
             +step +proj=cart +ellps=WGS84 \
             +step +inv +proj=helmert +x=0 +y=0 +z=4.5 +rx=0 +ry=0 \
             +rz=0.554 +s=0.219 +convention=position_vector \
             +step +inv +proj=cart +ellps=WGS72 \
             +step +proj=unitconvert +xy_in=rad +xy_out=deg \
             +step +proj=axisswap +order=2,1"
        );
    }

    // Check that we can handle vertical transformations where there is a
    // mix of available ones in the PROJ namespace (mx_inegi_ggm10) and in
    // in the EPSG namespace (us_noaa_g2018u0)
    // This test might no longer test this scenario if mx_inegi_ggm10 is
    // referenced one day by EPSG, but at least this tests a common use case.
    {
        // authority "" (`std::string()` in the C++ test)
        let mut ctxt = CoordinateOperationContext::create(&db, "");
        ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection)
            .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
        let list = operations(
            &epsg(&db, "6318+5703"), // NAD83(2011) + NAVD88 height
            &epsg(&db, "4979"),      // WGS 84 3D
            &ctxt,
        );
        let mut found_geoid2018 = false;
        let mut found_ggm10 = false;
        for op in &list {
            // The C++ test skips an operation whose PROJ string cannot be written.
            if let Ok(proj) = op.export_to_proj_string_with_database(&db, ProjLines::Single) {
                found_geoid2018 |= proj.contains("us_noaa_g2018u0.tif");
                found_ggm10 |= proj.contains("mx_inegi_ggm10.tif");
            }
        }
        assert!(found_geoid2018);
        assert!(found_ggm10);
    }
}
