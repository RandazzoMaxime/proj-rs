//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`: compound CRS read from PROJ strings with a
//! bound horizontal part (`+nadgrids` or `+towgs84`) and a bound vertical part (`+geoidgrids`).

use proj_rs::{
    create_crs_from_proj_string, CoordinateOperationFactory, DatabaseContext, ProjLines,
};

fn pipeline(source: &str, target: &str) -> String {
    let db = DatabaseContext::create().expect("proj.db");
    let source = create_crs_from_proj_string(source).expect("source");
    let target = create_crs_from_proj_string(target).expect("target");
    CoordinateOperationFactory::create()
        .create_operation(&source, &target)
        .expect("operation")
        .expect("some operation")
        .export_to_proj_string_with_database(&db, ProjLines::Single)
        .expect("PROJ string")
}

/// `TEST(operation, compoundCRS_to_compoundCRS_with_bound_crs_in_horiz_and_vert)`
#[test]
fn operation_compound_to_compound_with_bound_crs_in_horiz_and_vert() {
    assert_eq!(
        pipeline(
            "+proj=longlat +ellps=GRS67 +nadgrids=@foo.gsb +geoidgrids=@foo.gtx \
             +geoid_crs=horizontal_crs +type=crs",
            "+proj=longlat +ellps=GRS80 +nadgrids=@bar.gsb +geoidgrids=@bar.gtx \
             +geoid_crs=horizontal_crs +type=crs",
        ),
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=vgridshift +grids=@foo.gtx +multiplier=1 \
         +step +proj=hgridshift +grids=@foo.gsb \
         +step +inv +proj=hgridshift +grids=@bar.gsb \
         +step +inv +proj=vgridshift +grids=@bar.gtx +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg"
    );
}

/// `TEST(operation, compoundCRS_to_compoundCRS_with_bound_crs_in_horiz_and_vert_same_geoidgrids)`
#[test]
fn operation_compound_to_compound_with_bound_crs_same_geoidgrids() {
    assert_eq!(
        pipeline(
            "+proj=longlat +ellps=GRS67 +nadgrids=@foo.gsb +geoidgrids=@foo.gtx \
             +geoid_crs=horizontal_crs +type=crs",
            "+proj=longlat +ellps=GRS80 +nadgrids=@bar.gsb +geoidgrids=@foo.gtx \
             +geoid_crs=horizontal_crs +type=crs",
        ),
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=vgridshift +grids=@foo.gtx +multiplier=1 \
         +step +proj=hgridshift +grids=@foo.gsb \
         +step +inv +proj=hgridshift +grids=@bar.gsb \
         +step +inv +proj=vgridshift +grids=@foo.gtx +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg"
    );
}

/// `TEST(operation,
/// compoundCRS_to_compoundCRS_with_bound_crs_in_horiz_and_vert_same_geoidgrids_different_vunits)`
#[test]
fn operation_compound_to_compound_with_bound_crs_same_geoidgrids_different_vunits() {
    assert_eq!(
        pipeline(
            "+proj=longlat +ellps=GRS67 +nadgrids=@foo.gsb +geoidgrids=@foo.gtx \
             +geoid_crs=horizontal_crs +type=crs",
            "+proj=longlat +ellps=GRS80 +nadgrids=@bar.gsb +geoidgrids=@foo.gtx \
             +geoid_crs=horizontal_crs +vunits=us-ft +type=crs",
        ),
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=vgridshift +grids=@foo.gtx +multiplier=1 \
         +step +proj=hgridshift +grids=@foo.gsb \
         +step +inv +proj=hgridshift +grids=@bar.gsb \
         +step +inv +proj=vgridshift +grids=@foo.gtx +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=us-ft"
    );
}

/// `TEST(operation,
/// compoundCRS_to_compoundCRS_with_bound_crs_in_horiz_and_vert_same_nadgrids_same_geoidgrids)`
#[test]
fn operation_compound_to_compound_with_bound_crs_same_nadgrids_same_geoidgrids() {
    assert_eq!(
        pipeline(
            "+proj=longlat +ellps=GRS67 +nadgrids=@foo.gsb +geoidgrids=@foo.gtx \
             +geoid_crs=horizontal_crs +type=crs",
            "+proj=longlat +ellps=GRS80 +nadgrids=@foo.gsb +geoidgrids=@foo.gtx \
             +geoid_crs=horizontal_crs +type=crs",
        ),
        "+proj=noop"
    );
}

/// `TEST(operation,
/// compoundCRS_to_compoundCRS_with_bound_crs_in_horiz_and_vert_same_towgs84_same_geoidgrids)`
#[test]
fn operation_compound_to_compound_with_bound_crs_same_towgs84_same_geoidgrids() {
    assert_eq!(
        pipeline(
            "+proj=longlat +ellps=GRS67 +towgs84=0,0,0 +geoidgrids=@foo.gtx \
             +geoid_crs=horizontal_crs +type=crs",
            "+proj=longlat +ellps=GRS80 +towgs84=0,0,0 +geoidgrids=@foo.gtx \
             +geoid_crs=horizontal_crs +type=crs",
        ),
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=vgridshift +grids=@foo.gtx +multiplier=1 \
         +step +proj=cart +ellps=GRS67 \
         +step +inv +proj=cart +ellps=GRS80 \
         +step +inv +proj=vgridshift +grids=@foo.gtx +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg"
    );
}

/// `TEST(operation,
/// compoundCRS_to_compoundCRS_with_bound_crs_in_horiz_and_vert_same_ellsp_but_different_towgs84_different_geoidgrids)`
#[test]
fn operation_compound_to_compound_with_bound_crs_same_ellps_different_towgs84() {
    assert_eq!(
        pipeline(
            "+proj=longlat +ellps=GRS80 +towgs84=1,2,3 +geoidgrids=@foo.gtx \
             +geoid_crs=horizontal_crs +type=crs",
            "+proj=longlat +ellps=GRS80 +towgs84=4,5,6 +geoidgrids=@bar.gtx \
             +geoid_crs=horizontal_crs +type=crs",
        ),
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=vgridshift +grids=@foo.gtx +multiplier=1 \
         +step +proj=cart +ellps=GRS80 \
         +step +proj=helmert +x=-3 +y=-3 +z=-3 \
         +step +inv +proj=cart +ellps=GRS80 \
         +step +inv +proj=vgridshift +grids=@bar.gtx +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg"
    );
}
