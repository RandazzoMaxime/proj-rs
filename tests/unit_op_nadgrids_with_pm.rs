//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, `nadgrids_with_pm`: a tmerc with `+nadgrids`
//! and a non-Greenwich prime meridian, to EPSG:4326 (lines 4054-4065) and to EPSG:4258 (lines
//! 4067-4079). The WKT2 round trip of the C++ test (lines 4081-4097) is not in this file; it stays
//! open in the sheet.

use proj_rs::iso19111::{
    create_crs_from_proj_string, create_from_user_input, AuthorityFactory,
    CoordinateOperationContext, CoordinateOperationFactory, Crs, DatabaseContext, GeographicCRS,
    ProjLines, WktConvention, WktFormatter,
};

/// `TEST(operation, nadgrids_with_pm)`, first assertion: one candidate, and no extra
/// `+step +proj=longlat +pm=lisbon` between the inverse tmerc and the grid.
#[test]
fn operation_nadgrids_with_pm_to_epsg_4326() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = create_crs_from_proj_string(
        "+proj=tmerc +lat_0=39.66666666666666 +lon_0=1 +k=1 +x_0=200000 +y_0=300000 \
         +ellps=intl +nadgrids=foo.gsb +pm=lisbon +units=m +type=crs",
    )
    .expect("source");
    let dst = Crs::Geographic(GeographicCRS::epsg_4326());
    let ctxt = CoordinateOperationContext::create(&db, "");
    let list = CoordinateOperationFactory::create()
        .create_operations(&src, &dst, &ctxt)
        .expect("operations");
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0]
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ"),
        "+proj=pipeline +step +inv +proj=tmerc +lat_0=39.6666666666667 +lon_0=1 \
         +k=1 +x_0=200000 +y_0=300000 +ellps=intl +pm=lisbon \
         +step +proj=hgridshift +grids=foo.gsb \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}

/// `TEST(operation, nadgrids_with_pm)`, the WKT2_2019 round trip (lines 4081-4097): the source
/// written as a BOUNDCRS, read back, then to EPSG:4258. The prime meridian stays on the projected
/// step, with no extra `+proj=longlat +pm=lisbon` step.
#[test]
fn operation_nadgrids_with_pm_wkt_round_trip_to_epsg_4258() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = create_crs_from_proj_string(
        "+proj=tmerc +lat_0=39.66666666666666 +lon_0=1 +k=1 +x_0=200000 +y_0=300000 \
         +ellps=intl +nadgrids=foo.gsb +pm=lisbon +units=m +type=crs",
    )
    .expect("source");
    let wkt = src
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("WKT2_2019");
    let from_wkt = create_from_user_input(&wkt, &db).expect("BOUNDCRS read back");
    let dst = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_reference_system("4258")
        .expect("EPSG:4258");
    let ctxt = CoordinateOperationContext::create(&db, "");
    let list = CoordinateOperationFactory::create()
        .create_operations(&from_wkt, &dst, &ctxt)
        .expect("operations");
    assert!(!list.is_empty());
    assert_eq!(
        list[0]
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ"),
        "+proj=pipeline +step +inv +proj=tmerc +lat_0=39.6666666666667 +lon_0=1 \
         +k=1 +x_0=200000 +y_0=300000 +ellps=intl +pm=lisbon \
         +step +proj=hgridshift +grids=foo.gsb \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}

/// `TEST(operation, nadgrids_with_pm)`, the ETRS89 assertion (lines 4067-4079): the first candidate
/// to EPSG:4258 is the same string as to EPSG:4326, with no extra longlat step.
#[test]
fn operation_nadgrids_with_pm_to_epsg_4258() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = create_crs_from_proj_string(
        "+proj=tmerc +lat_0=39.66666666666666 +lon_0=1 +k=1 +x_0=200000 +y_0=300000 \
         +ellps=intl +nadgrids=foo.gsb +pm=lisbon +units=m +type=crs",
    )
    .expect("source");
    let dst = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_reference_system("4258")
        .expect("EPSG:4258");
    let ctxt = CoordinateOperationContext::create(&db, "");
    let list = CoordinateOperationFactory::create()
        .create_operations(&src, &dst, &ctxt)
        .expect("operations");
    assert!(!list.is_empty());
    assert_eq!(
        list[0]
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ"),
        "+proj=pipeline +step +inv +proj=tmerc +lat_0=39.6666666666667 +lon_0=1 \
         +k=1 +x_0=200000 +y_0=300000 +ellps=intl +pm=lisbon \
         +step +proj=hgridshift +grids=foo.gsb \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}
