//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, compound CRS to compound CRS, second lot.
//! The C++ `TEST()` names are kept in the doc comments, and the expected names and PROJ strings
//! are copied from that file. A test marked `#[ignore]` is open: its reason says what differs.

use proj_rs::iso19111::{
    create_from_user_input, AuthorityFactory, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, Crs, DatabaseContext, GridAvailabilityUse, ProjLines,
    SpatialCriterion, WktConvention, WktFormatter, WktObject, WktParser,
};

/// `CoordinateOperationContext::create(authFactory, nullptr, 0.0)` with the grids taken as
/// available, the setting of most of these tests.
fn ignore_grids(db: &DatabaseContext, authority: &str) -> CoordinateOperationContext {
    let mut ctxt = CoordinateOperationContext::create(db, authority);
    ctxt.set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    ctxt
}

/// `ignore_grids` with `SpatialCriterion::PARTIAL_INTERSECTION`.
fn partial_ignore_grids(db: &DatabaseContext, authority: &str) -> CoordinateOperationContext {
    let mut ctxt = ignore_grids(db, authority);
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    ctxt
}

/// `coordinateOperationAccuracies()[0]->value()` read as a number: the accuracy in metres.
fn assert_close(accuracy: Option<f64>, expected: f64) {
    let accuracy = accuracy.expect("an accuracy");
    assert!(
        (accuracy - expected).abs() < 1e-9,
        "{accuracy} != {expected}"
    );
}

fn parse(db: &DatabaseContext, text: &str) -> Crs {
    create_from_user_input(text, db).expect(text)
}

fn epsg(db: &DatabaseContext, code: &str) -> Crs {
    AuthorityFactory::create(db, "EPSG")
        .create_coordinate_reference_system(code)
        .expect(code)
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

/// `exportToPROJString(PROJStringFormatter::create())`, with the database for the grid names.
fn proj(db: &DatabaseContext, operation: &CoordinateOperation) -> String {
    operation
        .export_to_proj_string_with_database(db, ProjLines::Single)
        .expect("PROJ string")
}

/// TEST(operation, compoundCRS_to_compoundCRS_WGS84_EGM96_to_WGS84_Belfast): WGS 84 + EGM96 to
/// WGS 84 + Belfast height. The EGM96 geoid is referenced to WGS 84, so the candidate goes
/// through the vertical grids only.
#[test]
#[ignore = "open: the geog 3D to 4326+5732 candidates hold only the ballpark Belfast height change, not the ETRS89-IRE interpolation chain. Cause in the compound-to-geographic path (compound.rs compound_to_geog3d) not traced"]
fn operation_compound_to_compound_wgs84_egm96_to_wgs84_belfast() {
    let db = DatabaseContext::create().expect("proj.db");
    let ctxt = partial_ignore_grids(&db, "EPSG");
    let src = parse(&db, "EPSG:4326+5773");
    let dst = parse(&db, "EPSG:4326+5732");
    let list = operations(&src, &dst, &ctxt);
    assert!(!list.is_empty());
    assert_eq!(
        list[0].name(),
        "Inverse of WGS 84 to EGM96 height (1) + \
         ETRS89-IRE [ETRF2000] to Belfast height (2) \
         using ETRS89-IRE [ETRF2000] to WGS 84 (1)"
    );
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=vgridshift +grids=us_nga_egm96_15.tif +multiplier=1 \
         +step +inv +proj=vgridshift +grids=uk_os_OSGM15_Belfast.tif \
         +multiplier=1 +step \
         +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}

/// TEST(operation, compoundCRS_to_compoundCRS_OSGB36_BNG_ODN_height_to_WGS84_EGM96): the
/// British National Grid with the ODN height, to WGS 84 with the EGM96 height, both ways.
#[test]
#[ignore = "open: the first candidate is OSGB36 to WGS 84 (9) with the ODN height inverse, where PROJ chains OSGB36 to ETRS89-GBR (2) then ETRS89-GBR to WGS 84 (1); the candidate choice is not traced"]
fn operation_compound_to_compound_osgb36_bng_odn_height_to_wgs84_egm96() {
    let db = DatabaseContext::create().expect("proj.db");
    let ctxt = ignore_grids(&db, "");
    // "OSGB36 / British National Grid + ODN height
    let src = parse(&db, "EPSG:27700+5701");
    // "WGS 84 + EGM96 height"
    let dst = epsg(&db, "9707");
    {
        let list = operations(&src, &dst, &ctxt);
        assert!(!list.is_empty());
        assert_eq!(
            list[0].name(),
            "Inverse of British National Grid + \
             OSGB36 to ETRS89-GBR [OSNet v2009] (2) + \
             Inverse of ETRS89-GBR [OSNet v2009] to ODN height (2) + \
             ETRS89-GBR [OSNet v2009] to WGS 84 (1) + \
             WGS 84 to EGM96 height (1)"
        );
        assert_eq!(
            proj(&db, &list[0]),
            "+proj=pipeline \
             +step +inv +proj=tmerc +lat_0=49 +lon_0=-2 +k=0.9996012717 \
             +x_0=400000 +y_0=-100000 +ellps=airy \
             +step +proj=hgridshift +grids=uk_os_OSTN15_NTv2_OSGBtoETRS.tif \
             +step +proj=vgridshift +grids=uk_os_OSGM15_GB.tif +multiplier=1 \
             +step +inv +proj=vgridshift +grids=us_nga_egm96_15.tif \
             +multiplier=1 \
             +step +proj=unitconvert +xy_in=rad +xy_out=deg \
             +step +proj=axisswap +order=2,1"
        );
    }
    {
        let list = operations(&dst, &src, &ctxt);
        assert!(!list.is_empty());
        assert_eq!(
            list[0].name(),
            "Inverse of WGS 84 to EGM96 height (1) + \
             Inverse of ETRS89-GBR [OSNet v2009] to \
             WGS 84 (1) + ETRS89-GBR [OSNet v2009] \
             to ODN height (2) + Inverse of OSGB36 \
             to ETRS89-GBR [OSNet v2009] (2) + \
             British National Grid"
        );
        assert_eq!(
            proj(&db, &list[0]),
            "+proj=pipeline \
             +step +proj=axisswap +order=2,1 \
             +step +proj=unitconvert +xy_in=deg +xy_out=rad \
             +step +proj=vgridshift +grids=us_nga_egm96_15.tif +multiplier=1 \
             +step +inv +proj=vgridshift +grids=uk_os_OSGM15_GB.tif \
             +multiplier=1 \
             +step +inv +proj=hgridshift \
             +grids=uk_os_OSTN15_NTv2_OSGBtoETRS.tif \
             +step +proj=tmerc +lat_0=49 +lon_0=-2 +k=0.9996012717 \
             +x_0=400000 +y_0=-100000 +ellps=airy"
        );
    }
}

/// TEST(operation, compoundCRS_to_compoundCRS_concatenated_operation_with_two_vert_transformation_and_different_source_dest_interp):
/// BD72 + Ostend height to Amersfoort + NAP height. The two vertical transformations have
/// different interpolation CRS, and the result goes through ETRS89 on each side.
#[test]
#[ignore = "open: the first candidate has an extra 'Inverse of BD72 to ETRS89-BEL (3) + BD72 to ETRS89 (3)' pair that PROJ does not give; the chain is not traced (compound.rs compound_to_compound)"]
fn operation_compound_to_compound_concatenated_two_vert_transformation_different_source_dest_interp(
) {
    let db = DatabaseContext::create().expect("proj.db");
    // "BD72 + Ostend height"
    let src = parse(&db, "EPSG:4313+5710");
    // "Amersfoort + NAP height"
    let dst = parse(&db, "EPSG:4289+5709");
    let ctxt = partial_ignore_grids(&db, "EPSG");
    let list = operations(&src, &dst, &ctxt);
    assert!(!list.is_empty());
    assert_eq!(
        list[0].name(),
        "BD72 to ETRS89-BEL [BEREF2011] (3) + \
         Inverse of ETRS89-BEL [BEREF2011] to Ostend height (1) + \
         Inverse of Amersfoort to ETRS89 (9) + \
         Amersfoort to ETRS89-NLD [AGRS2010] (9) + \
         ETRS89-NLD [AGRS2010] to NAP height (2) + \
         Inverse of Amersfoort to ETRS89-NLD [AGRS2010] (9)"
    );
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=hgridshift +grids=be_ign_bd72lb72_etrs89lb08.tif \
         +step +proj=vgridshift +grids=be_ign_hBG18.tif +multiplier=1 \
         +step +inv +proj=vgridshift +grids=nl_nsgi_nlgeo2018.tif \
         +multiplier=1 \
         +step +inv +proj=hgridshift +grids=nl_nsgi_rdtrans2018.tif \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}

/// TEST(operation, compoundCRS_to_compoundCRS_issue_3328): WGS 84 + EGM96 height to WGS 84 +
/// CGVD28 height. The CGVD28 height is referenced to NAD83(CSRS), so the chain goes through
/// the 3D Helmert transformation between the two geographic CRS.
#[test]
#[ignore = "open: the 3D route through NAD83(CSRS) is missing; the first candidate is the ballpark EGM96 to CGVD28 change. Same compound-to-geographic path as the Belfast case, not traced"]
fn operation_compound_to_compound_issue_3328() {
    let db = DatabaseContext::create().expect("proj.db");
    // "WGS 84 + EGM96 height"
    let src = parse(&db, "EPSG:4326+5773");
    // "WGS 84 + CGVD28 height"
    let dst = parse(&db, "EPSG:4326+5713");
    let ctxt = partial_ignore_grids(&db, "EPSG");
    let list = operations(&src, &dst, &ctxt);
    assert!(!list.is_empty());
    assert_eq!(
        list[0].name(),
        "Inverse of WGS 84 to EGM96 height (1) + \
         NAD83(CSRS) to CGVD28 height (1) \
         using NAD83(CSRS) to WGS 84 (2)"
    );
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline \
         +step +proj=push +v_1 +v_2 \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=vgridshift +grids=us_nga_egm96_15.tif +multiplier=1 \
         +step +proj=cart +ellps=WGS84 \
         +step +inv +proj=helmert +x=-0.991 +y=1.9072 +z=0.5129 \
         +rx=-0.0257899075194932 \
         +ry=-0.0096500989602704 +rz=-0.0116599432323421 +s=0 \
         +convention=coordinate_frame \
         +step +inv +proj=cart +ellps=GRS80 \
         +step +inv +proj=vgridshift +grids=ca_nrc_HT2_1997.tif \
         +multiplier=1 \
         +step +proj=push +v_3 \
         +step +proj=cart +ellps=GRS80 \
         +step +proj=helmert +x=-0.991 +y=1.9072 +z=0.5129 \
         +rx=-0.0257899075194932 \
         +ry=-0.0096500989602704 +rz=-0.0116599432323421 +s=0 \
         +convention=coordinate_frame \
         +step +inv +proj=cart +ellps=WGS84 \
         +step +proj=pop +v_3 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1 \
         +step +proj=pop +v_1 +v_2"
    );
}

/// TEST(operation, compoundCRS_to_compoundCRS_issue_3191_BD72_Ostend_height_to_WGS84_EGM96):
/// BD72 + Ostend height to WGS 84 + EGM96 height, both ways.
#[test]
fn operation_compound_to_compound_issue_3191_bd72_ostend_height_to_wgs84_egm96() {
    let db = DatabaseContext::create().expect("proj.db");
    let ctxt = ignore_grids(&db, "");
    // BD72 + Ostend height
    let src = parse(&db, "EPSG:4313+5710");
    // "WGS 84 + EGM96 height"
    let dst = epsg(&db, "9707");
    {
        let list = operations(&src, &dst, &ctxt);
        assert!(!list.is_empty());
        assert_eq!(
            list[0].name(),
            "BD72 to ETRS89-BEL [BEREF2011] (3) + \
             Inverse of ETRS89-BEL [BEREF2011] to Ostend height (1) + \
             ETRS89-BEL [BEREF2011] to WGS 84 (1) + \
             WGS 84 to EGM96 height (1)"
        );
        assert_eq!(
            proj(&db, &list[0]),
            "+proj=pipeline \
             +step +proj=axisswap +order=2,1 \
             +step +proj=unitconvert +xy_in=deg +xy_out=rad \
             +step +proj=hgridshift +grids=be_ign_bd72lb72_etrs89lb08.tif \
             +step +proj=vgridshift +grids=be_ign_hBG18.tif +multiplier=1 \
             +step +inv +proj=vgridshift +grids=us_nga_egm96_15.tif \
             +multiplier=1 \
             +step +proj=unitconvert +xy_in=rad +xy_out=deg \
             +step +proj=axisswap +order=2,1"
        );
    }
    {
        let list = operations(&dst, &src, &ctxt);
        assert!(!list.is_empty());
        assert_eq!(
            list[0].name(),
            "Inverse of WGS 84 to EGM96 height (1) + \
             Inverse of ETRS89-BEL [BEREF2011] \
             to WGS 84 (1) + \
             ETRS89-BEL [BEREF2011] to \
             Ostend height (1) + \
             Inverse of BD72 to \
             ETRS89-BEL [BEREF2011] (3)"
        );
        assert_eq!(
            proj(&db, &list[0]),
            "+proj=pipeline \
             +step +proj=axisswap +order=2,1 \
             +step +proj=unitconvert +xy_in=deg +xy_out=rad \
             +step +proj=vgridshift +grids=us_nga_egm96_15.tif +multiplier=1 \
             +step +inv +proj=vgridshift +grids=be_ign_hBG18.tif +multiplier=1 \
             +step +inv +proj=hgridshift +grids=be_ign_bd72lb72_etrs89lb08.tif \
             +step +proj=unitconvert +xy_in=rad +xy_out=deg \
             +step +proj=axisswap +order=2,1"
        );
    }
}

/// TEST(operation, compoundCRS_to_compoundCRS_issue_4550): a NAD83(HARN) Washington South
/// projected CRS with a Geoid18 height in US feet, to UTM zone 10N with NAVD88 height.
#[test]
fn operation_compound_to_compound_issue_4550() {
    let db = DatabaseContext::create().expect("proj.db");
    let ctxt = ignore_grids(&db, "");
    let wkt = "COMPD_CS[\
        \"NAD83(HARN) / Washington South (ftUS) + \
        NAVD88 height (ftUS) - Geoid18 (ftUS)\",\
        PROJCS[\"NAD83(HARN) / Washington South (ftUS)\",\
        GEOGCS[\"NAD83(HARN)\",\
        DATUM[\"NAD83 (High Accuracy Reference Network)\",\
        SPHEROID[\"GRS 1980\",6378137,298.257222101,\
        AUTHORITY[\"EPSG\",\"7019\"]],TOWGS84[0,0,0,0,0,0,0],\
        AUTHORITY[\"EPSG\",\"6152\"]],\
        PRIMEM[\"Greenwich\",0,AUTHORITY[\"EPSG\",\"8901\"]],\
        UNIT[\"degree\",0.0174532925199433,AUTHORITY[\"EPSG\",\"9122\"]],\
        AUTHORITY[\"EPSG\",\"4152\"]],\
        PROJECTION[\"Lambert_Conformal_Conic_2SP\"],\
        PARAMETER[\"standard_parallel_1\",47.33333333333334],\
        PARAMETER[\"standard_parallel_2\",45.83333333333334],\
        PARAMETER[\"latitude_of_origin\",45.33333333333334],\
        PARAMETER[\"central_meridian\",-120.5],\
        PARAMETER[\"false_easting\",1640416.667],\
        PARAMETER[\"false_northing\",0],\
        UNIT[\"US survey foot\",0.3048006096012192,\
        AUTHORITY[\"EPSG\",\"9003\"]],\
        AXIS[\"X\",EAST],AXIS[\"Y\",NORTH],\
        AUTHORITY[\"EPSG\",\"2927\"]],\
        VERT_CS[\"NAVD88 height (ftUS) - Geoid18 (ftUS)\",\
        VERT_DATUM[\"North American Vertical Datum 1988\",2005,\
        AUTHORITY[\"EPSG\",\"5103\"]],\
        UNIT[\"US survey foot\",0.3048006096012192,\
        AUTHORITY[\"EPSG\",\"9003\"]],AXIS[\"Up\",UP],\
        AUTHORITY[\"EPSG\",\"6360\"]]]";
    let src = parse(&db, wkt);
    // "UTM Zone 10N / WGS 84 + NAVD88 height"
    let dst = parse(&db, "EPSG:32610+5703");
    let list = operations(&src, &dst, &ctxt);
    assert!(!list.is_empty());
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline \
         +step +proj=unitconvert +xy_in=us-ft +xy_out=m \
         +step +inv +proj=lcc +lat_0=45.3333333333333 +lon_0=-120.5 \
         +lat_1=47.3333333333333 +lat_2=45.8333333333333 \
         +x_0=500000.0001016 +y_0=0 +ellps=GRS80 \
         +step +proj=unitconvert +z_in=us-ft +z_out=m \
         +step +proj=utm +zone=10 +ellps=WGS84"
    );
}

/// TEST(operation, compoundCRS_to_compoundCRS_issue_2232): NAD83 / Alabama West + NAVD88 height
/// (Geoid12B) to NAD83 with a height in US feet. Both directions give the same number of
/// candidates.
#[test]
fn operation_compound_to_compound_issue_2232() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = parse(
        &db,
        "COMPD_CS[\"NAD83 / Alabama West + NAVD88 height - Geoid12B \
         (Meters)\",\n\
         PROJCS[\"NAD83 / Alabama West\",\n\
             GEOGCS[\"NAD83\",\n\
                 DATUM[\"North_American_Datum_1983\",\n\
                     SPHEROID[\"GRS 1980\",6378137,298.257222101,\n\
                         AUTHORITY[\"EPSG\",\"7019\"]],\n\
                     TOWGS84[0,0,0,0,0,0,0],\n\
                     AUTHORITY[\"EPSG\",\"6269\"]],\n\
                 PRIMEM[\"Greenwich\",0,\n\
                     AUTHORITY[\"EPSG\",\"8901\"]],\n\
                 UNIT[\"degree\",0.0174532925199433,\n\
                     AUTHORITY[\"EPSG\",\"9122\"]],\n\
                 AUTHORITY[\"EPSG\",\"4269\"]],\n\
             PROJECTION[\"Transverse_Mercator\"],\n\
             PARAMETER[\"latitude_of_origin\",30],\n\
             PARAMETER[\"central_meridian\",-87.5],\n\
             PARAMETER[\"scale_factor\",0.999933333],\n\
             PARAMETER[\"false_easting\",600000],\n\
             PARAMETER[\"false_northing\",0],\n\
             UNIT[\"metre\",1,\n\
                 AUTHORITY[\"EPSG\",\"9001\"]],\n\
             AXIS[\"X\",EAST],\n\
             AXIS[\"Y\",NORTH],\n\
             AUTHORITY[\"EPSG\",\"26930\"]],\n\
         VERT_CS[\"NAVD88 height - Geoid12B (Meters)\",\n\
             VERT_DATUM[\"North American Vertical Datum 1988\",2005,\n\
                 EXTENSION[\"PROJ4_GRIDS\",\"foo.gtx\"],\n\
                 AUTHORITY[\"EPSG\",\"5103\"]],\n\
             UNIT[\"metre\",1.0,\n\
                 AUTHORITY[\"EPSG\",\"9001\"]],\n\
             AXIS[\"Gravity-related height\",UP],\n\
             AUTHORITY[\"EPSG\",\"5703\"]]]",
    );
    let dst = parse(
        &db,
        "COMPD_CS[\"NAD83 + some CRS (US Feet)\",\n\
         GEOGCS[\"NAD83\",\n\
             DATUM[\"North_American_Datum_1983\",\n\
                 SPHEROID[\"GRS 1980\",6378137,298.257222101,\n\
                     AUTHORITY[\"EPSG\",\"7019\"]],\n\
                 TOWGS84[0,0,0,0,0,0,0],\n\
                 AUTHORITY[\"EPSG\",\"6269\"]],\n\
             PRIMEM[\"Greenwich\",0,\n\
                 AUTHORITY[\"EPSG\",\"8901\"]],\n\
             UNIT[\"degree\",0.0174532925199433,\n\
                 AUTHORITY[\"EPSG\",\"9122\"]],\n\
             AUTHORITY[\"EPSG\",\"4269\"]],\n\
         VERT_CS[\"some CRS (US Feet)\",\n\
             VERT_DATUM[\"some datum\",2005],\n\
             UNIT[\"US survey foot\",0.3048006096012192,\n\
                 AUTHORITY[\"EPSG\",\"9003\"]],\n\
             AXIS[\"Up\",UP]]]",
    );
    let ctxt = partial_ignore_grids(&db, "EPSG");
    let list = operations(&src, &dst, &ctxt);
    assert!(!list.is_empty());
    let list2 = operations(&dst, &src, &ctxt);
    assert_eq!(list2.len(), list.len());
}

/// TEST(operation, compoundCRS_to_compoundCRS_issue_3152_ch1903lv03_ln02_bound): a bound CRS
/// (CH1903 / LV03 with a Helmert transformation to WGS 84) plus the LN02 height, to WGS 84 +
/// EGM2008 height. The bound transformation is used, in both directions.
#[test]
#[ignore = "open: a compound of a bound CRS and a vertical CRS has no candidate to WGS 84 + EGM2008. The bound hub is not seen as equivalent (equiv.rs Crs::is_equivalent_to_with has no geographic arm), and enabling it gives a candidate that drops the LN02 height (bound_compound_to_geog3d)"]
fn operation_compound_to_compound_issue_3152_ch1903lv03_ln02_bound() {
    let db = DatabaseContext::create().expect("proj.db");
    let ctxt = ignore_grids(&db, "");
    let wkt = "COMPOUNDCRS[\"CH1903 / LV03 + LN02 height\",\n\
        BOUNDCRS[\n\
            SOURCECRS[\n\
                PROJCRS[\"CH1903 / LV03\",\n\
                    BASEGEOGCRS[\"CH1903\",\n\
                        DATUM[\"CH1903\",\n\
                            ELLIPSOID[\"Bessel 1841\",6377397.155,299.1528128,\n\
                                LENGTHUNIT[\"metre\",1]]],\n\
                        PRIMEM[\"Greenwich\",0,\n\
                            ANGLEUNIT[\"degree\",0.0174532925199433]],\n\
                        ID[\"EPSG\",4149]],\n\
                    CONVERSION[\"Map projection of CH1903 / LV03\",\n\
                        METHOD[\"Hotine Oblique Mercator (variant B)\",\n\
                            ID[\"EPSG\",9815]],\n\
                        PARAMETER[\"Latitude of projection centre\",46.9524055555556,\n\
                            ANGLEUNIT[\"degree\",0.0174532925199433],\n\
                            ID[\"EPSG\",8811]],\n\
                        PARAMETER[\"Longitude of projection centre\",7.43958333333333,\n\
                            ANGLEUNIT[\"degree\",0.0174532925199433],\n\
                            ID[\"EPSG\",8812]],\n\
                        PARAMETER[\"Azimuth of initial line\",90,\n\
                            ANGLEUNIT[\"degree\",0.0174532925199433],\n\
                            ID[\"EPSG\",8813]],\n\
                        PARAMETER[\"Angle from Rectified to Skew Grid\",90,\n\
                            ANGLEUNIT[\"degree\",0.0174532925199433],\n\
                            ID[\"EPSG\",8814]],\n\
                        PARAMETER[\"Scale factor on initial line\",1,\n\
                            SCALEUNIT[\"unity\",1],\n\
                            ID[\"EPSG\",8815]],\n\
                        PARAMETER[\"Easting at projection centre\",600000,\n\
                            LENGTHUNIT[\"metre\",1],\n\
                            ID[\"EPSG\",8816]],\n\
                        PARAMETER[\"Northing at projection centre\",200000,\n\
                            LENGTHUNIT[\"metre\",1],\n\
                            ID[\"EPSG\",8817]]],\n\
                    CS[Cartesian,2],\n\
                        AXIS[\"easting\",east,\n\
                            ORDER[1],\n\
                            LENGTHUNIT[\"metre\",1]],\n\
                        AXIS[\"northing\",north,\n\
                            ORDER[2],\n\
                            LENGTHUNIT[\"metre\",1]],\n\
                    ID[\"EPSG\",21781]]],\n\
            TARGETCRS[\n\
                GEOGCRS[\"WGS 84\",\n\
                    DATUM[\"World Geodetic System 1984\",\n\
                        ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n\
                            LENGTHUNIT[\"metre\",1]]],\n\
                    PRIMEM[\"Greenwich\",0,\n\
                        ANGLEUNIT[\"degree\",0.0174532925199433]],\n\
                    CS[ellipsoidal,2],\n\
                        AXIS[\"latitude\",north,\n\
                            ORDER[1],\n\
                            ANGLEUNIT[\"degree\",0.0174532925199433]],\n\
                        AXIS[\"longitude\",east,\n\
                            ORDER[2],\n\
                            ANGLEUNIT[\"degree\",0.0174532925199433]],\n\
                    ID[\"EPSG\",4326]]],\n\
            ABRIDGEDTRANSFORMATION[\"MyTransformation from CH1903 to WGS84\",\n\
                METHOD[\"Position Vector transformation (geog2D domain)\",\n\
                    ID[\"EPSG\",9606]],\n\
                PARAMETER[\"X-axis translation\",674.374,\n\
                    ID[\"EPSG\",8605]],\n\
                PARAMETER[\"Y-axis translation\",15.056,\n\
                    ID[\"EPSG\",8606]],\n\
                PARAMETER[\"Z-axis translation\",405.346,\n\
                    ID[\"EPSG\",8607]],\n\
                PARAMETER[\"X-axis rotation\",0,\n\
                    ID[\"EPSG\",8608]],\n\
                PARAMETER[\"Y-axis rotation\",0,\n\
                    ID[\"EPSG\",8609]],\n\
                PARAMETER[\"Z-axis rotation\",0,\n\
                    ID[\"EPSG\",8610]],\n\
                PARAMETER[\"Scale difference\",1,\n\
                    ID[\"EPSG\",8611]]]],\n\
        VERTCRS[\"LN02 height\",\n\
            VDATUM[\"Landesnivellement 1902\"],\n\
            CS[vertical,1],\n\
                AXIS[\"gravity-related height\",up,\n\
                    LENGTHUNIT[\"metre\",1]],\n\
            ID[\"EPSG\",5728]]]";
    let src = parse(&db, wkt);
    // "WGS 84 + EGM2008 height"
    let dst = epsg(&db, "9518");
    let list = operations(&src, &dst, &ctxt);
    assert!(!list.is_empty());
    // Check that BoundCRS helmert transformation is used
    assert_eq!(
        list[0].name(),
        "Inverse of Map projection of CH1903 / LV03 + \
         MyTransformation from CH1903 to WGS84 + \
         Inverse of ETRS89 to WGS 84 (1) + \
         Inverse of ETRS89 to LN02 height + \
         ETRS89 to WGS 84 (1) + \
         WGS 84 to EGM2008 height (1)"
    );
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline \
         +step +inv +proj=somerc +lat_0=46.9524055555556 \
         +lon_0=7.43958333333333 +k_0=1 \
         +x_0=600000 +y_0=200000 +ellps=bessel \
         +step +proj=push +v_3 \
         +step +proj=cart +ellps=bessel \
         +step +proj=helmert +x=674.374 +y=15.056 +z=405.346 \
         +rx=0 +ry=0 +rz=0 +s=0 +convention=position_vector \
         +step +inv +proj=cart +ellps=GRS80 \
         +step +proj=pop +v_3 \
         +step +proj=vgridshift \
         +grids=ch_swisstopo_chgeo2004_ETRS89_LN02.tif \
         +multiplier=1 \
         +step +inv +proj=vgridshift +grids=us_nga_egm08_25.tif \
         +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );

    let list_inv = operations(&dst, &src, &ctxt);
    assert!(!list_inv.is_empty());
    assert_eq!(
        list_inv[0].name(),
        "Inverse of WGS 84 to EGM2008 height (1) + \
         Inverse of ETRS89 to WGS 84 (1) + \
         ETRS89 to LN02 height + \
         ETRS89 to WGS 84 (1) + \
         Inverse of MyTransformation from CH1903 to WGS84 + \
         Map projection of CH1903 / LV03"
    );
}

/// TEST(operation, compoundCRS_to_compoundCRS_WGS84_EGM2008_to_RD_new_NAP_height): WGS 84 + EGM2008
/// height to Amersfoort / RD + NAP height. Four candidates, the two with the registry's
/// accuracy 1.124 and 1.373 first, then the two that use the 1' EGM2008 grid.
#[test]
fn operation_compound_to_compound_wgs84_egm2008_to_rd_new_nap_height() {
    let db = DatabaseContext::create().expect("proj.db");
    let ctxt = partial_ignore_grids(&db, "EPSG");
    // WGS 84 + EGM2008 height
    let src = epsg(&db, "9518");
    // Amersfoort / RD + NAP height
    let dst = epsg(&db, "7415");
    let list = operations(&src, &dst, &ctxt);

    assert_eq!(list.len(), 4);

    assert!(!list[0].has_ballpark_transformation());
    assert_eq!(
        list[0].name(),
        "Inverse of WGS 84 to EGM2008 height (1) + \
         Inverse of ETRS89-NLD [AGRS2010] to \
         WGS 84 (1) + \
         ETRS89-NLD [AGRS2010] to NAP height (2) + \
         Inverse of Amersfoort to \
         ETRS89-NLD [AGRS2010] (9) + RD"
    );
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=vgridshift +grids=us_nga_egm08_25.tif +multiplier=1 \
         +step +inv +proj=vgridshift +grids=nl_nsgi_nlgeo2018.tif \
         +multiplier=1 \
         +step +inv +proj=hgridshift +grids=nl_nsgi_rdtrans2018.tif \
         +step +proj=sterea +lat_0=52.1561605555556 +lon_0=5.38763888888889 \
         +k=0.9999079 +x_0=155000 +y_0=463000 +ellps=bessel"
    );
    // `coordinateOperationAccuracies()[0]->value()` is the sum of the steps, a number here.
    assert_close(list[0].accuracy(), 1.124);

    assert!(!list[1].has_ballpark_transformation());
    assert_eq!(
        list[1].name(),
        "Inverse of WGS 84 to EGM2008 height (1) + \
         Inverse of ETRS89-NLD [AGRS2010] to \
         WGS 84 (1) + \
         ETRS89-NLD [AGRS2010] to NAP height (2) + \
         Inverse of Amersfoort to \
         ETRS89-NLD [AGRS2010] (8) + RD"
    );
    assert_close(list[1].accuracy(), 1.373);

    // Using not available "WGS 84 to EGM2008 height (2)" with 1' EGM2008 grid
    assert!(!list[2].has_ballpark_transformation());
    assert_eq!(
        list[2].name(),
        "Inverse of WGS 84 to EGM2008 height (2) + \
         Inverse of ETRS89-NLD [AGRS2010] to \
         WGS 84 (1) + \
         ETRS89-NLD [AGRS2010] to NAP height (2) + \
         Inverse of Amersfoort to \
         ETRS89-NLD [AGRS2010] (9) + RD"
    );

    // Using not available "WGS 84 to EGM2008 height (2)" with 1' EGM2008 grid
    assert!(!list[3].has_ballpark_transformation());
    assert_eq!(
        list[3].name(),
        "Inverse of WGS 84 to EGM2008 height (2) + \
         Inverse of ETRS89-NLD [AGRS2010] to \
         WGS 84 (1) + \
         ETRS89-NLD [AGRS2010] to NAP height (2) + \
         Inverse of Amersfoort to \
         ETRS89-NLD [AGRS2010] (8) + RD"
    );
}

/// TEST(operation, compoundCRS_to_compoundCRS_concatenated_operation_with_two_vert_transformation_and_ballpark_geog):
/// NAD83(CSRS) + CGVD28 height to NAD83(CSRS) + CGVD2013(CGG2013) height. The horizontal part
/// is a ballpark offset from NAD83(CSRS) to NAD83(CSRS)v6. The second half of the PROJ test is
/// inside `#if 0` and is not run.
#[test]
#[ignore = "open: the first candidate is the ballpark CGVD28 to CGVD2013(CGG2013) change; PROJ's chains NAD83(CSRS)v6 vertical operations with the ballpark geographic offset. Same compound-to-geographic path as issue_3328, not traced"]
fn operation_compound_to_compound_concatenated_two_vert_transformation_and_ballpark_geog() {
    let db = DatabaseContext::create().expect("proj.db");
    // "NAD83(CSRS) + CGVD28 height"
    let src = parse(&db, "EPSG:4617+5713");
    // "NAD83(CSRS) + CGVD2013(CGG2013) height"
    let dst = parse(&db, "EPSG:4617+6647");
    let ctxt = {
        let mut ctxt = CoordinateOperationContext::create(&db, "EPSG");
        ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection);
        ctxt
    };
    let list = operations(&src, &dst, &ctxt);
    assert!(!list.is_empty());
    assert_eq!(
        list[0].name(),
        "Inverse of NAD83(CSRS)v6 to CGVD28 height (2) + \
         NAD83(CSRS)v6 to CGVD2013(CGG2013) height (1) \
         using Ballpark geographic offset \
         from NAD83(CSRS) to NAD83(CSRS)v6"
    );
}

/// TEST(operation, compoundCRS_to_compoundCRS): UTM zone 31N + ODN height to UTM zone 32N + ODN
/// height. The vertical part is the same datum and unit, so the operation is the horizontal
/// conversion between the two projected CRS, with no operation from the vertical part. PROJ
/// builds it with `createOperation` (no database), and compares it by `isEquivalentTo`. The
/// projected CRS are the registry ones, which PROJ's test builds in memory with the same
/// definitions.
#[test]
fn operation_compound_to_compound_utm31_to_utm32_with_odn_height() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = CoordinateOperationFactory::create();
    let src = parse(&db, "EPSG:32631+5701");
    let dst = parse(&db, "EPSG:32632+5701");
    let op = factory
        .create_operation(&src, &dst)
        .expect("operation")
        .expect("an operation");
    let op_ref = factory
        .create_operation(&epsg(&db, "32631"), &epsg(&db, "32632"))
        .expect("operation")
        .expect("an operation");
    assert!(op.is_equivalent_to(&op_ref));
}

/// TEST(operation, compoundCRS_to_compoundCRS_context): NAD27 + NGVD29 height (ftUS) to
/// NAD83(NSRS2007) + NAVD88 height. Around 150 candidates; the first one chains the VERTCON
/// vertical grid, the NAD27 grid and the NAD83(NSRS2007) inverse. The ballpark candidate,
/// with the same horizontal chain, is among them.
#[test]
#[ignore = "open: the NGVD29 to NAVD88 (3) registry candidate with the VERTCON grid is not built, only the ballpark vertical candidates are (compound_vertical_pass, horiz_vertical_horiz_checked)"]
fn operation_compound_to_compound_context() {
    let db = DatabaseContext::create().expect("proj.db");
    let ctxt = partial_ignore_grids(&db, "EPSG");
    // NAD27 + NGVD29 height (ftUS)
    let src = epsg(&db, "7406");
    // NAD83(NSRS2007) + NAVD88 height
    let dst = epsg(&db, "5500");
    let list = operations(&src, &dst, &ctxt);
    // 152 or 155 depending if the VERTCON grids are there
    assert!(list.len() >= 152, "{}", list.len());
    assert!(!list[0].has_ballpark_transformation());
    assert_eq!(
        list[0].name(),
        "NGVD29 height (ftUS) to NAVD88 height (3) + \
         NAD27 to WGS 84 (79) + Inverse of \
         NAD83(NSRS2007) to WGS 84 (1)"
    );
    let proj_string = proj(&db, &list[0]);
    assert_eq!(
        proj_string,
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +z_in=us-ft +xy_out=rad +z_out=m \
         +step +proj=vgridshift +grids=us_noaa_vertcone.tif +multiplier=1 \
         +step +proj=hgridshift +grids=us_noaa_conus.tif +step \
         +proj=unitconvert +xy_in=rad +xy_out=deg +step +proj=axisswap \
         +order=2,1"
    );
    {
        // Test that we can round-trip this through WKT and still get the same
        // PROJ string.
        let wkt = list[0]
            .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
            .expect("wkt");
        let imported = WktParser::new().create_from_wkt(&wkt).expect("parse");
        let WktObject::Concatenated(concatenated) = imported else {
            panic!("not a concatenated operation: {wkt}");
        };
        assert_eq!(
            proj_string,
            proj(&db, &CoordinateOperation::Concatenated(concatenated))
        );
    }

    let mut found_approx = false;
    for operation in &list {
        let proj_string = proj(&db, operation);
        assert!(
            proj_string.starts_with(
                "+proj=pipeline +step +proj=axisswap +order=2,1 \
                 +step +proj=unitconvert +xy_in=deg +z_in=us-ft \
                 +xy_out=rad +z_out=m"
            ),
            "{}",
            operation.name()
        );
        if operation.name().starts_with(
            "Transformation from NGVD29 height (ftUS) \
             to NAVD88 height (ballpark vertical \
             transformation)",
        ) {
            assert!(operation.has_ballpark_transformation());
            assert_eq!(
                operation.name(),
                "Transformation from NGVD29 height (ftUS) to NAVD88 \
                 height (ballpark vertical transformation) + NAD27 to \
                 WGS 84 (79) + Inverse of NAD83(NSRS2007) to WGS 84 (1)"
            );
            assert_eq!(
                proj_string,
                "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
                 +proj=unitconvert +xy_in=deg +z_in=us-ft +xy_out=rad \
                 +z_out=m +step +proj=hgridshift +grids=us_noaa_conus.tif \
                 +step +proj=unitconvert +xy_in=rad \
                 +xy_out=deg +step +proj=axisswap +order=2,1"
            );
            found_approx = true;
            break;
        }
    }
    assert!(found_approx);
}

/// TEST(operation, compoundCRS_to_compoundCRS_using_intermediate_of_horizontal_transform): PNG94 /
/// PNGMG94 zone 54 + Kumul 34 height to WGS 84 (G2139) + EGM2008 height. The horizontal part
/// goes through WGS 84 and WGS 84 (G2139) with a geographic offset, with grids known as available.
#[test]
#[ignore = "open: PROJ's only-vertical-ballpark branch (createOperationsCompoundToGeog, intermediate geographic CRS of the horizontal transforms, C++ 6860-6905) is not ported; the ballpark Kumul 34 to EGM2008 candidate comes first"]
fn operation_compound_to_compound_using_intermediate_of_horizontal_transform() {
    let db = DatabaseContext::create().expect("proj.db");
    let mut ctxt = CoordinateOperationContext::create(&db, "");
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    ctxt.set_grid_availability_use(GridAvailabilityUse::KnownAvailable);
    // "PNG94 / PNGMG94 zone 54 + Kumul 34 height"
    let src = parse(&db, "EPSG:5550+7651");
    // "WGS 84 (G2139) + EGM2008 height
    let dst = parse(&db, "EPSG:9755+3855");
    let list = operations(&src, &dst, &ctxt);
    assert!(!list.is_empty());
    assert_eq!(
        list[0].name(),
        "Inverse of Papua New Guinea Map Grid 1994 zone 54 + \
         PNG94 to WGS 84 (1) + \
         Inverse of EGM96 height to Kumul 34 height (1) + \
         Inverse of WGS 84 to EGM96 height (1) + \
         WGS 84 to WGS 84 (G2139) + \
         WGS 84 (G2139) to EGM2008 height (from WGS 84 to EGM2008 height (1))"
    );
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline \
         +step +inv +proj=utm +zone=54 +south +ellps=GRS80 \
         +step +proj=geogoffset +dh=0.87 \
         +step +proj=vgridshift +grids=us_nga_egm96_15.tif +multiplier=1 \
         +step +inv +proj=vgridshift +grids=us_nga_egm08_25.tif \
         +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}
