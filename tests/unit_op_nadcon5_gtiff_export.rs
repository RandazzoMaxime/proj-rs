//! The PROJ string of a NADCON5 (3D) transformation once `substitutePROJAlternativeGridNames`
//! names its grid as PROJ does (`singleoperation.cpp:2461-2476`), in both directions.
//!
//! Expected values: PROJ 9.9.0 `projinfo` on the operation (`EPSG:8559`, `NAD83(NSRS2007) to
//! NAD83(2011) (1)`), forward (`-s EPSG:4759 -t EPSG:6318`, operation No. 1 `DERIVED_FROM(EPSG):8559`)
//! and inverse (`-s EPSG:6318 -t EPSG:4759`, operation No. 1 `INVERSE(DERIVED_FROM(EPSG)):8559`).

use proj_rs::iso19111::{
    AlternativeGridNames, AuthorityFactory, CoordinateOperation, DatabaseContext, ProjLines,
};

fn operation_8559(db: &DatabaseContext) -> CoordinateOperation {
    AuthorityFactory::create(db, "EPSG")
        .create_coordinate_operation("8559", AlternativeGridNames::Proj)
        .expect("8559")
}

/// Forward: the grid is read as `gridshift` with `no_z_transform`, since the source is 2D.
#[test]
fn operation_8559_nadcon5_3d_forward_proj_string() {
    let db = DatabaseContext::create().expect("proj.db");
    let operation = operation_8559(&db);
    assert_eq!(
        operation
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=gridshift +no_z_transform \
         +grids=us_noaa_nadcon5_nad83_2007_nad83_2011_conus.tif \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}

/// Inverse: the grid step of the substituted operation is written with `+inv`, and the same
/// `no_z_transform` flag as the forward one.
#[test]
fn operation_8559_nadcon5_3d_inverse_proj_string() {
    let db = DatabaseContext::create().expect("proj.db");
    let inverse = match operation_8559(&db) {
        CoordinateOperation::Transformation(transformation) => {
            CoordinateOperation::Transformation(transformation.inverse())
        }
        other => panic!("8559 is not a transformation: {}", other.name()),
    };
    assert_eq!(
        inverse
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +inv +proj=gridshift +no_z_transform \
         +grids=us_noaa_nadcon5_nad83_2007_nad83_2011_conus.tif \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}
