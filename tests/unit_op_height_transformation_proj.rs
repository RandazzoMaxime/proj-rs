//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, `transformation_height_to_PROJ_string`.

use proj_rs::{
    create_from_user_input, CoordinateOperation, Crs, DatabaseContext, GeographicCRS,
    GridAvailabilityUse, OpProps, ProjLines, Transformation,
};

/// `TEST(operation, transformation_height_to_PROJ_string)`
#[test]
fn operation_transformation_height_to_proj_string() {
    let db = DatabaseContext::create().expect("proj.db");
    let vertical = create_from_user_input(
        r#"VERTCRS["ODN height",VDATUM["Ordnance Datum Newlyn",ID["EPSG",5101]],CS[vertical,1],AXIS["gravity-related height (H)",up,LENGTHUNIT["metre",1]],ID["EPSG",5701]]"#,
        &db,
    )
    .expect("vertical CRS");
    let transformation = CoordinateOperation::Transformation(
        Transformation::create_gravity_related_height_to_geographic_3d(
            &OpProps::new(),
            vertical,
            Crs::Geographic(GeographicCRS::epsg_4979()),
            "us_nga_egm08_25.tif",
            None,
        ),
    );
    assert_eq!(
        transformation
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=vgridshift +grids=us_nga_egm08_25.tif +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
    let grids = transformation
        .grids_needed(&db, GridAvailabilityUse::UseForSorting)
        .expect("grids");
    assert_eq!(grids.len(), 1);
    let grid = &grids[0];
    assert_eq!(grid.short_name, "us_nga_egm08_25.tif");
    assert!(grid.info.package_name.is_empty());
    assert_eq!(grid.info.url, "https://cdn.proj.org/us_nga_egm08_25.tif");
    if grid.info.available {
        assert!(!grid.info.full_name.is_empty());
        assert!(grid.info.full_name.contains(&grid.short_name));
    } else {
        assert!(grid.info.full_name.is_empty());
    }
    assert!(grid.info.direct_download);
    assert!(grid.info.open_license);
}
