//! A bound CRS whose base is a projected CRS (UTM zone 55S, `+nadgrids=@GDA94_GDA2020_conformal.gsb`)
//! to a geographic CRS: `createOperationsBoundToGeog` of PROJ 9.9.0.
//!
//! The expected operations are the output of `projinfo` 9.9.0 (`-o PROJ`), the Homebrew bottle of
//! PROJ 9.9.0, not of this crate. The grid is kept when the target datum is the hub of the binding
//! (WGS 84) or when a GDA94 / GDA2020 target is reached through it. When the target datum is an
//! `unknown` GRS 1980 datum, PROJ drops the binding and falls back to a ballpark. The datum name
//! carries ` using nadgrids=<file>`, so it is not the same datum as the plain GRS 1980 one.

use proj_rs::iso19111::{
    create_crs_from_proj_string, AuthorityFactory, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, Crs, DatabaseContext, ProjLines,
};

fn database() -> DatabaseContext {
    DatabaseContext::create().expect("database")
}

fn epsg(db: &DatabaseContext, code: &str) -> Crs {
    AuthorityFactory::create(db, "EPSG")
        .create_coordinate_reference_system(code)
        .expect(code)
}

fn proj_crs(text: &str) -> Crs {
    create_crs_from_proj_string(text).expect(text)
}

/// The bound CRS of the test: UTM zone 55S on GRS 1980, with a NTv2 binding.
fn source() -> Crs {
    proj_crs(
        "+proj=utm +zone=55 +south +ellps=GRS80 +towgs84=0,0,0,0,0,0,0 +units=m +no_defs \
         +nadgrids=@GDA94_GDA2020_conformal.gsb +type=crs",
    )
}

fn operations(db: &DatabaseContext, target: &Crs) -> Vec<CoordinateOperation> {
    let ctx = CoordinateOperationContext::create(db, "");
    CoordinateOperationFactory::create()
        .create_operations(&source(), target, &ctx)
        .expect("operations")
}

/// `exportToPROJString(PROJStringFormatter::create())`, without database.
fn plain(operation: &CoordinateOperation) -> String {
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

/// Bound UTM to `+proj=longlat +ellps=GRS80`: PROJ keeps no grid, only a ballpark.
#[test]
fn operation_bound_utm_nadgrids_to_geog_ellps_only_is_ballpark_as_proj() {
    let db = database();
    let target = proj_crs("+proj=longlat +ellps=GRS80 +type=crs");
    let list = operations(&db, &target);

    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].name(),
        "Inverse of UTM zone 55S + Ballpark geographic offset from unknown to unknown"
    );
    assert_eq!(
        plain(&list[0]),
        "+proj=pipeline +step +inv +proj=utm +zone=55 +south +ellps=GRS80 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg"
    );
}

/// Bound UTM to `+proj=longlat +datum=WGS84`: the binding is the first step, PROJ keeps it.
#[test]
fn operation_bound_utm_nadgrids_to_geog_wgs84_keeps_hgridshift_as_proj() {
    let db = database();
    let target = proj_crs("+proj=longlat +datum=WGS84 +type=crs");
    let list = operations(&db, &target);

    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].name(),
        "Inverse of UTM zone 55S + unknown to WGS84 + axis order change (2D)"
    );
    assert_eq!(
        plain(&list[0]),
        "+proj=pipeline +step +inv +proj=utm +zone=55 +south +ellps=GRS80 \
         +step +proj=hgridshift +grids=@GDA94_GDA2020_conformal.gsb \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg"
    );
}

/// Bound UTM to GDA94 (EPSG:4283): three operations, the grid kept in the first two and in the
/// ballpark one as PROJ lists them.
#[test]
fn operation_bound_utm_nadgrids_to_gda94_keeps_hgridshift_as_proj() {
    let db = database();
    let target = epsg(&db, "4283");
    let list = operations(&db, &target);

    let names: Vec<&str> = list.iter().map(|op| op.name()).collect();
    assert_eq!(
        names,
        [
            "Inverse of UTM zone 55S + unknown to WGS84 + Inverse of GDA94 to WGS 84 (1)",
            "Inverse of UTM zone 55S + unknown to WGS84 + Inverse of GDA94 to WGS 84 (2)",
            "Inverse of UTM zone 55S + unknown to WGS84 + Ballpark geographic offset from WGS 84 to GDA94",
        ]
    );
    assert_eq!(
        plain(&list[0]),
        "+proj=pipeline +step +inv +proj=utm +zone=55 +south +ellps=GRS80 \
         +step +proj=hgridshift +grids=@GDA94_GDA2020_conformal.gsb \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        plain(&list[1]),
        "+proj=pipeline +step +inv +proj=utm +zone=55 +south +ellps=GRS80 \
         +step +proj=hgridshift +grids=@GDA94_GDA2020_conformal.gsb \
         +step +proj=push +v_3 +step +proj=cart +ellps=WGS84 \
         +step +inv +proj=helmert +x=0.06155 +y=-0.01087 +z=-0.04019 \
         +rx=-0.0394924 +ry=-0.0327221 +rz=-0.0328979 +s=-0.009994 \
         +convention=coordinate_frame \
         +step +inv +proj=cart +ellps=GRS80 \
         +step +proj=pop +v_3 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        plain(&list[2]),
        "+proj=pipeline +step +inv +proj=utm +zone=55 +south +ellps=GRS80 \
         +step +proj=hgridshift +grids=@GDA94_GDA2020_conformal.gsb \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}

/// Bound UTM to GDA2020 (EPSG:7844): the grid is the first step of the first two operations.
#[test]
fn operation_bound_utm_nadgrids_to_gda2020_keeps_hgridshift_as_proj() {
    let db = database();
    let target = epsg(&db, "7844");
    let list = operations(&db, &target);

    let names: Vec<&str> = list.iter().map(|op| op.name()).collect();
    assert_eq!(
        names,
        [
            "Inverse of UTM zone 55S + unknown to WGS84 + Inverse of GDA2020 to WGS 84 (2)",
            "Inverse of UTM zone 55S + unknown to WGS84 + WGS 84 to GDA2020 (3)",
            "Inverse of UTM zone 55S + unknown to WGS84 + Ballpark geographic offset from WGS 84 to GDA2020",
        ]
    );
    assert_eq!(
        plain(&list[0]),
        "+proj=pipeline +step +inv +proj=utm +zone=55 +south +ellps=GRS80 \
         +step +proj=hgridshift +grids=@GDA94_GDA2020_conformal.gsb \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        plain(&list[1]),
        "+proj=pipeline +step +inv +proj=utm +zone=55 +south +ellps=GRS80 \
         +step +proj=hgridshift +grids=@GDA94_GDA2020_conformal.gsb \
         +step +proj=push +v_3 +step +proj=cart +ellps=WGS84 \
         +step +proj=helmert +x=0.06155 +y=-0.01087 +z=-0.04019 \
         +rx=-0.0394924 +ry=-0.0327221 +rz=-0.0328979 +s=-0.009994 \
         +convention=coordinate_frame \
         +step +inv +proj=cart +ellps=GRS80 \
         +step +proj=pop +v_3 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        plain(&list[2]),
        "+proj=pipeline +step +inv +proj=utm +zone=55 +south +ellps=GRS80 \
         +step +proj=hgridshift +grids=@GDA94_GDA2020_conformal.gsb \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}
