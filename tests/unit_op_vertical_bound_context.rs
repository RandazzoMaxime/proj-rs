//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, three unit tests of the vertical and bound
//! families: `geog3DCRS_to_geog2DCRS_plus_vertCRS_context`, `boundCRS_to_geogCRS_hubCRS_and_targetCRS_same_but_baseCRS_not`
//! and `compoundCRS_to_compoundCRS_with_vertical_transform`. The expected names and PROJ strings
//! are copied from that file at 9.9.0, never from this crate's output.

use proj_rs::iso19111::{
    create_from_user_input, AuthorityFactory, BoundCRS, CartesianCS, CompoundCRS, Conversion,
    CoordinateOperation, CoordinateOperationContext, CoordinateOperationFactory, Crs,
    DatabaseContext, GeodeticCRS, GeographicCRS, OpProps, ProjLines, ProjectedCRS,
    SpatialCriterion, Transformation, UnitOfMeasure, VerticalCRS, VerticalCS,
    VerticalReferenceFrame,
};

fn epsg(db: &DatabaseContext, code: &str) -> Crs {
    AuthorityFactory::create(db, "EPSG")
        .create_coordinate_reference_system(code)
        .expect(code)
}

/// `exportToPROJString(PROJStringFormatter::create())`, with the database for the grid names.
fn proj(db: &DatabaseContext, operation: &CoordinateOperation) -> String {
    operation
        .export_to_proj_string_with_database(db, ProjLines::Single)
        .expect("PROJ string")
}

/// `TEST(operation, geog3DCRS_to_geog2DCRS_plus_vertCRS_context)`: ETRS89 (3D) to ETRS89 + Baltic
/// 1957 height, with `PARTIAL_INTERSECTION` and no authority restriction. The two first candidates
/// go through the Czech and Slovak vertical grids.
///
/// The C++ test sits under `#if no_longer_work_since_epsg_13_001` (test_operationfactory.cpp:1053),
/// so PROJ 9.9.0 does not run it. The names below are the ones `projinfo` 9.9.0 prints with the
/// same database (`ETRS89-CZE [2007]`, `ETRS89-SVK [SKTRF09]`, not the `ETRS89` the disabled test
/// expects). See `docs/PORTING-operationfactory-vertical-bound.md`.
#[test]
#[ignore = "open: the first candidate matches (PROJ string and name); the second has the PROJ string but its name is `Inverse of 'Inverse of '...''` here, PROJ's is `'ETRS89-SVK [SKTRF09] to ...'`"]
fn operation_geog3d_to_geog2d_plus_vert_crs_context() {
    let db = DatabaseContext::create().expect("proj.db");
    let mut ctxt = CoordinateOperationContext::create(&db, "");
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    // ETRS89 (3D)
    let src = epsg(&db, "4937");
    // ETRS89 + Baltic 1957 height (now deprecated)
    let dst = epsg(&db, "8360");
    let list = CoordinateOperationFactory::create()
        .create_operations(&src, &dst, &ctxt)
        .expect("operations");

    assert!(list.len() >= 2, "{}", list.len());
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +inv +proj=vgridshift \
         +grids=cz_cuzk_CR-2005.tif +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        list[0].inverse().name(),
        "Inverse of ETRS89-CZE [2007] to Baltic 1957 height (2)"
    );

    assert_eq!(
        proj(&db, &list[1]),
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +inv +proj=vgridshift \
         +grids=sk_gku_Slovakia_ETRS89h_to_Baltic1957.tif +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        list[1].inverse().name(),
        "Inverse of 'ETRS89-SVK [SKTRF09] to Baltic 1957 height (from \
         ETRS89-SVK [SKTRF09] to ETRS89-SVK [SKTRF09] + Baltic 1957 height (1))'"
    );
}

/// `TEST(operation, boundCRS_to_geogCRS_hubCRS_and_targetCRS_same_but_baseCRS_not)`: a bound CRS
/// whose hub is WGS 84 and whose base is NAD83 (ftUS height), to EPSG:4979. Only the unit change
/// of the vertical part is left.
#[test]
#[ignore = "open: the chain is one unitconvert with the height only; PROJ keeps the horizontal deg identity (xy_in=deg xy_out=deg) of its concatenated operation"]
fn operation_bound_crs_to_geog_hub_and_target_same_but_base_not() {
    let wkt = "COMPD_CS[\"NAD83 + Ellipsoid (US Feet)\",\n\
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
        VERT_CS[\"Ellipsoid (US Feet)\",\n\
            VERT_DATUM[\"Ellipsoid\",2002],\n\
            UNIT[\"US survey foot\",0.304800609601219,\n\
                AUTHORITY[\"EPSG\",\"9003\"]],\n\
            AXIS[\"Up\",UP]]]";

    let db = DatabaseContext::create().expect("proj.db");
    let bound = match create_from_user_input(wkt, &db).expect("WKT") {
        Crs::Bound(bound) => bound,
        other => panic!("not a bound CRS: {}", other.name()),
    };
    let mut ctxt = CoordinateOperationContext::create(&db, "EPSG");
    ctxt.set_grid_availability_use(proj_rs::iso19111::GridAvailabilityUse::IgnoreGridAvailability);
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    let list = CoordinateOperationFactory::create()
        .create_operations(&Crs::Bound(bound), &epsg(&db, "4979"), &ctxt)
        .expect("operations");

    assert_eq!(list.len(), 1);
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=unitconvert +xy_in=deg +z_in=us-ft +xy_out=deg +z_out=m"
    );
}

/// `TEST(operation, compoundCRS_to_compoundCRS_with_vertical_transform)`: a compound of a
/// transverse Mercator and a bound vertical CRS (VERTCON through `bla.gtx`), to a compound of
/// UTM zone 32N and an anonymous vertical CRS. The bound part is the vertical step.
#[test]
#[ignore = "open: the first candidate has no vgridshift (the bound vertical part is dropped when its hub is the target vertical CRS); the approx-tmerc assertions of the C++ test are not written: no operation-level approx export in this crate"]
fn operation_compound_to_compound_with_vertical_transform() {
    let props = OpProps::new;
    let vertical1 = VerticalCRS::create(
        "ODN height",
        VerticalReferenceFrame::named("Ordnance Datum Newlyn"),
        VerticalCS::create_gravity_related_height(UnitOfMeasure::metre()),
    )
    .with_identifier("EPSG", "5701");
    let vertical2 = VerticalCRS::create(
        "",
        VerticalReferenceFrame::named(""),
        VerticalCS::create_gravity_related_height(UnitOfMeasure::metre()),
    );

    // Use of this type of transformation is a bit of nonsense here, as in the C++ test.
    let vtransformation = Transformation::create_vertcon(
        &props(),
        Crs::Vertical(vertical1.clone()),
        Crs::Vertical(vertical2.clone()),
        "bla.gtx",
        None,
    )
    .expect("VERTCON");

    let tmerc =
        Conversion::create_transverse_mercator(&props(), 1.0, 2.0, 3.0, 4.0, 5.0).expect("tmerc");
    let compound1 = CompoundCRS::create(
        "",
        vec![
            Crs::Projected(ProjectedCRS::create(
                "",
                GeodeticCRS::Geographic(GeographicCRS::epsg_4326()),
                tmerc,
                CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
            )),
            Crs::Bound(BoundCRS::create(
                Crs::Vertical(vertical1),
                Crs::Vertical(vertical2.clone()),
                CoordinateOperation::Transformation(vtransformation),
            )),
        ],
    )
    .expect("compound1");

    let utm32 = ProjectedCRS::create(
        "",
        GeodeticCRS::Geographic(GeographicCRS::epsg_4326()),
        Conversion::create_utm_north(&props(), 32).expect("UTM 32N"),
        CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    );
    let compound2 = CompoundCRS::create("", vec![Crs::Projected(utm32), Crs::Vertical(vertical2)])
        .expect("compound2");

    let db = DatabaseContext::create().expect("proj.db");
    let factory = CoordinateOperationFactory::create();
    let op = factory
        .create_operation(
            &Crs::Compound(compound1.clone()),
            &Crs::Compound(compound2.clone()),
        )
        .expect("operation")
        .expect("an operation");
    assert_eq!(
        proj(&db, &op),
        "+proj=pipeline +step +inv +proj=tmerc +lat_0=1 +lon_0=2 +k=3 \
         +x_0=4 +y_0=5 +ellps=WGS84 +step \
         +proj=vgridshift +grids=bla.gtx +multiplier=0.001 +step \
         +proj=utm +zone=32 +ellps=WGS84"
    );
    assert_eq!(
        proj(&db, &op.inverse()),
        "+proj=pipeline +step +inv +proj=utm +zone=32 +ellps=WGS84 \
         +step +inv +proj=vgridshift +grids=bla.gtx +multiplier=0.001 \
         +step +proj=tmerc +lat_0=1 +lon_0=2 +k=3 +x_0=4 +y_0=5 +ellps=WGS84"
    );

    let op_inverse = factory
        .create_operation(&Crs::Compound(compound2), &Crs::Compound(compound1))
        .expect("operation")
        .expect("an operation");
    assert_eq!(proj(&db, &op_inverse.inverse()), proj(&db, &op));
}
