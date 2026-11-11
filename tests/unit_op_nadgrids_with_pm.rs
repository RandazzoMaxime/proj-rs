//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, `nadgrids_with_pm`, first assertion only
//! (lines 4054-4065): a tmerc with `+nadgrids` and a non-Greenwich prime meridian, to EPSG:4326.
//! The ETRS89 (EPSG:4258) assertion and the WKT2 round trip of the C++ test are not in this file;
//! they stay open in the sheet.

use proj_rs::iso19111::{
    create_crs_from_proj_string, CoordinateOperationContext, CoordinateOperationFactory, Crs,
    DatabaseContext, GeographicCRS, ProjLines,
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
