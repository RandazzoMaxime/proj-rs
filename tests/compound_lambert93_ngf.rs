//! `Lambert-93 + NGF-IGN69 height` (EPSG:2154+5720) to `WGS 84` 3D (EPSG:4979), as
//! `projinfo -s EPSG:2154+5720 -t EPSG:4979 --spatial-test intersects` lists it. The expected
//! names and PROJ strings are those of the PROJ 9.9.0 `projinfo` on the same database
//! (`docs/PORTING-compound-lambert93-ngf.md`).

use proj_rs::iso19111::{
    create_from_user_input, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, Crs, DatabaseContext, ProjLines, SpatialCriterion,
};

fn epsg(db: &DatabaseContext, code: &str) -> Crs {
    create_from_user_input(&format!("EPSG:{code}"), db).expect(code)
}

/// The context of `projinfo --spatial-test intersects`.
fn operations(db: &DatabaseContext, source: &str, target: &str) -> Vec<CoordinateOperation> {
    let mut ctxt = CoordinateOperationContext::create(db, "");
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    CoordinateOperationFactory::create()
        .create_operations(&epsg(db, source), &epsg(db, target), &ctxt)
        .expect("operations")
}

/// PROJ's first and third candidates take the `X using Y` name of
/// `createHorizVerticalHorizPROJBased`: the leg from the interpolation CRS to the target,
/// inverted, is equivalent to the leg to it (`SingleOperation::_isEquivalentTo`, method and
/// values, no accuracy). The two extra `RGF93 v2b to NGF-IGN69 height (4)` candidates of the port
/// are still open (see the sheet), so only these names and the first PROJ string are pinned.
#[test]
fn lambert93_ngf_ign69_to_wgs84_3d_using_names() {
    let db = DatabaseContext::create().expect("proj.db");
    let ops = operations(&db, "2154+5720", "4979");
    let first = "Inverse of Lambert-93 + Inverse of ETRS89-FRA [RGF93 v2] to NGF-IGN69 height (3) \
                 using ETRS89-FRA [RGF93 v1] to ETRS89-FRA [RGF93 v2] (1)";
    let third = "Inverse of Lambert-93 + Inverse of ETRS89-FRA [RGF93 v2] to NGF-IGN69 height (2) \
                 using ETRS89-FRA [RGF93 v1] to ETRS89-FRA [RGF93 v2] (1)";
    assert_eq!(ops[0].name(), first);
    assert!(ops.iter().any(|op| op.name() == third));
    assert_eq!(
        ops[0]
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline +step +inv +proj=lcc +lat_0=46.5 +lon_0=3 +lat_1=49 +lat_2=44 \
         +x_0=700000 +y_0=6600000 +ellps=GRS80 +step +proj=vgridshift +grids=fr_ign_RAF18.tif \
         +multiplier=1 +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}
