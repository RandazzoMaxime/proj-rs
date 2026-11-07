//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`: a bound CRS to a compound CRS. The C++
//! `TEST()` names are kept in the doc comments, and the expected PROJ strings are copied from
//! that file.

use proj_rs::iso19111::{
    create_crs_from_proj_string, create_from_user_input, AuthorityFactory, CoordinateOperation,
    CoordinateOperationContext, CoordinateOperationFactory, Crs, DatabaseContext,
    GridAvailabilityUse, ProjLines, SpatialCriterion,
};

/// `CoordinateOperationContext::create(authFactory, nullptr, 0.0)` with the partial
/// intersection criterion and the grids taken as available.
fn partial_ignore_grids(db: &DatabaseContext, authority: &str) -> CoordinateOperationContext {
    let mut ctxt = CoordinateOperationContext::create(db, authority);
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    ctxt.set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    ctxt
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

/// `TEST(operation, boundCRS_to_compoundCRS)`: `+proj=longlat +ellps=GRS67 +nadgrids=@foo.gsb`
/// (a bound geographic CRS) to a compound of a horizontal CRS with the `@bar.gsb` grid and a
/// vertical `@bar.gtx` geoid, both ways.
#[test]
fn operation_bound_crs_to_compound() {
    let db = DatabaseContext::create().expect("proj.db");
    let src =
        create_crs_from_proj_string("+proj=longlat +ellps=GRS67 +nadgrids=@foo.gsb +type=crs")
            .expect("source");
    let dst = create_crs_from_proj_string(
        "+proj=longlat +ellps=GRS80 +nadgrids=@bar.gsb +geoidgrids=@bar.gtx \
         +geoid_crs=horizontal_crs +type=crs",
    )
    .expect("target");
    let factory = CoordinateOperationFactory::create();
    let op = factory
        .create_operation(&src, &dst)
        .expect("operation")
        .expect("an operation");
    assert_eq!(
        proj(&db, &op),
        "+proj=pipeline \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=hgridshift +grids=@foo.gsb \
         +step +inv +proj=hgridshift +grids=@bar.gsb \
         +step +inv +proj=vgridshift +grids=@bar.gtx +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg"
    );
    let op_inverse = factory
        .create_operation(&dst, &src)
        .expect("operation")
        .expect("an operation");
    // PROJ's `_isEquivalentTo(op)` uses the EQUIVALENT criterion. This crate has no operation
    // method for that criterion: its strict `is_equivalent_to` is the one checked here.
    assert!(op_inverse.inverse().is_equivalent_to(&op));
}

/// `TEST(operation, boundCRS_to_compoundCRS_with_hubCRS_same_as_compound_geographicCRS)`: the
/// bound CH1903 / LV03 with a Helmert to WGS 84 (hub WGS 84 3D) to "WGS 84 + EGM2008 height".
#[test]
fn operation_bound_crs_to_compound_with_hub_crs_same_as_compound_geographic_crs() {
    let db = DatabaseContext::create().expect("proj.db");
    let ctxt = partial_ignore_grids(&db, "EPSG");
    let wkt = r#"BOUNDCRS[
    SOURCECRS[
        PROJCRS["CH1903 / LV03",
            BASEGEOGCRS["CH1903",
                DATUM["CH1903",
                    ELLIPSOID["Bessel 1841",6377397.155,299.1528128,
                        LENGTHUNIT["metre",1]],
                    ID["EPSG",6149]],
                PRIMEM["Greenwich",0,
                    ANGLEUNIT["degree",0.0174532925199433],
                    ID["EPSG",8901]]],
            CONVERSION["unnamed",
                METHOD["Hotine Oblique Mercator (variant B)",
                    ID["EPSG",9815]],
                PARAMETER["Latitude of projection centre",46.9524055555556,
                    ANGLEUNIT["degree",0.0174532925199433],
                    ID["EPSG",8811]],
                PARAMETER["Longitude of projection centre",7.43958333333333,
                    ANGLEUNIT["degree",0.0174532925199433],
                    ID["EPSG",8812]],
                PARAMETER["Azimuth of initial line",90,
                    ANGLEUNIT["degree",0.0174532925199433],
                    ID["EPSG",8813]],
                PARAMETER["Angle from Rectified to Skew Grid",90,
                    ANGLEUNIT["degree",0.0174532925199433],
                    ID["EPSG",8814]],
                PARAMETER["Scale factor on initial line",1,
                    SCALEUNIT["unity",1],
                    ID["EPSG",8815]],
                PARAMETER["Easting at projection centre",600000,
                    LENGTHUNIT["metre",1],
                    ID["EPSG",8816]],
                PARAMETER["Northing at projection centre",200000,
                    LENGTHUNIT["metre",1],
                    ID["EPSG",8817]]],
            CS[Cartesian,3],
                AXIS["y",east,
                    ORDER[1],
                    LENGTHUNIT["metre",1,
                        ID["EPSG",9001]]],
                AXIS["x",north,
                    ORDER[2],
                    LENGTHUNIT["metre",1,
                        ID["EPSG",9001]]],
                AXIS["ellipsoidal height (h)",up,
                    ORDER[3],
                    LENGTHUNIT["metre",1,
                        ID["EPSG",9001]]]]],
    TARGETCRS[
        GEOGCRS["WGS 84",
            ENSEMBLE["World Geodetic System 1984 ensemble",
                MEMBER["World Geodetic System 1984 (Transit)"],
                MEMBER["World Geodetic System 1984 (G730)"],
                MEMBER["World Geodetic System 1984 (G873)"],
                MEMBER["World Geodetic System 1984 (G1150)"],
                MEMBER["World Geodetic System 1984 (G1674)"],
                MEMBER["World Geodetic System 1984 (G1762)"],
                MEMBER["World Geodetic System 1984 (G2139)"],
                ELLIPSOID["WGS 84",6378137,298.257223563,
                    LENGTHUNIT["metre",1]],
                ENSEMBLEACCURACY[2.0]],
            PRIMEM["Greenwich",0,
                ANGLEUNIT["degree",0.0174532925199433]],
            CS[ellipsoidal,3],
                AXIS["geodetic latitude (Lat)",north,
                    ORDER[1],
                    ANGLEUNIT["degree",0.0174532925199433]],
                AXIS["geodetic longitude (Lon)",east,
                    ORDER[2],
                    ANGLEUNIT["degree",0.0174532925199433]],
                AXIS["ellipsoidal height (h)",up,
                    ORDER[3],
                    LENGTHUNIT["metre",1]],
            ID["EPSG",4979]]],
    ABRIDGEDTRANSFORMATION["Transformation from CH1903 to WGS84",
        METHOD["Position Vector transformation (geog2D domain)",
            ID["EPSG",9606]],
        PARAMETER["X-axis translation",674.4,
            ID["EPSG",8605]],
        PARAMETER["Y-axis translation",15.1,
            ID["EPSG",8606]],
        PARAMETER["Z-axis translation",405.3,
            ID["EPSG",8607]],
        PARAMETER["X-axis rotation",0,
            ID["EPSG",8608]],
        PARAMETER["Y-axis rotation",0,
            ID["EPSG",8609]],
        PARAMETER["Z-axis rotation",0,
            ID["EPSG",8610]],
        PARAMETER["Scale difference",1,
            ID["EPSG",8611]]]]"#;
    let src = create_from_user_input(wkt, &db).expect("source");
    // "WGS 84 + EGM2008 height"
    let dst = epsg(&db, "9518");
    let list = operations(&src, &dst, &ctxt);
    assert!(!list.is_empty());
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline \
         +step +inv +proj=somerc +lat_0=46.9524055555556 \
         +lon_0=7.43958333333333 +k_0=1 \
         +x_0=600000 +y_0=200000 +ellps=bessel \
         +step +proj=cart +ellps=bessel \
         +step +proj=helmert +x=674.4 +y=15.1 +z=405.3 +rx=0 +ry=0 +rz=0 \
         +s=0 +convention=position_vector \
         +step +inv +proj=cart +ellps=WGS84 \
         +step +inv +proj=vgridshift +grids=us_nga_egm08_25.tif \
         +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}

/// `TEST(operation, WGS84_G1762_to_compoundCRS_with_bound_vertCRS)`
#[test]
fn operation_wgs84_g1762_to_compound_with_bound_vert_crs() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = epsg(&db, "7665");
    let dst =
        create_crs_from_proj_string("+proj=longlat +datum=NAD83 +geoidgrids=@foo.gtx +type=crs")
            .expect("dst");
    let ctxt = partial_ignore_grids(&db, "");
    let list = operations(&src, &dst, &ctxt);
    assert!(list.len() >= 53, "{} operations", list.len());
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +inv +proj=vgridshift +grids=@foo.gtx +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg"
    );
}
