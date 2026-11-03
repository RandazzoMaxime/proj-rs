//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, lot A of the `operation` suite
//! (`docs/PORTING-opfac-core-a.md`). Each function is one `TEST(operation, ...)`; the C++
//! name is in its doc comment. Names, counts, EPSG codes and PROJ strings are the expected
//! text of PROJ 9.9.0.

use proj_rs::iso19111::{
    AuthorityFactory, BoundCRS, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, Crs, DatabaseContext, GeographicCRS, GridAvailabilityUse,
    ProjLines, SpatialCriterion, WktParser,
};

fn database() -> DatabaseContext {
    DatabaseContext::create().expect("database")
}

fn authority_crs(db: &DatabaseContext, authority: &str, code: &str) -> Crs {
    AuthorityFactory::create(db, authority)
        .create_coordinate_reference_system(code)
        .expect(code)
}

fn epsg(db: &DatabaseContext, code: &str) -> Crs {
    authority_crs(db, "EPSG", code)
}

fn wkt_crs(wkt: &str) -> Crs {
    WktParser::new()
        .create_from_wkt(wkt)
        .expect("wkt")
        .crs()
        .expect("crs")
        .clone()
}

fn context(db: &DatabaseContext, authority: &str) -> CoordinateOperationContext {
    CoordinateOperationContext::create(db, authority)
}

/// `CoordinateOperationContext` with the spatial criterion of most tests.
fn partial_context(db: &DatabaseContext, authority: &str) -> CoordinateOperationContext {
    let mut ctxt = context(db, authority);
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    ctxt
}

/// Context with grid availability ignored and a partial-intersection spatial criterion.
fn ignore_grids_context(db: &DatabaseContext, authority: &str) -> CoordinateOperationContext {
    let mut ctxt = partial_context(db, authority);
    ctxt.set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    ctxt
}

fn operations(
    source: &Crs,
    target: &Crs,
    context: &CoordinateOperationContext,
) -> Vec<CoordinateOperation> {
    CoordinateOperationFactory::create()
        .create_operations(source, target, context)
        .expect("operations")
}

/// `createOperation(source, target)` without a database search.
fn operation(source: &Crs, target: &Crs) -> CoordinateOperation {
    CoordinateOperationFactory::create()
        .create_operation(source, target)
        .expect("operation")
        .expect("an operation")
}

/// `exportToPROJString` of an operation, with a formatter that has a database.
fn proj(db: &DatabaseContext, operation: &CoordinateOperation) -> String {
    match operation {
        CoordinateOperation::Conversion(conversion)
            if conversion.source_crs().is_none() || conversion.target_crs().is_none() =>
        {
            conversion.export_to_proj_string()
        }
        _ => operation.export_to_proj_string_with_database(db, ProjLines::Single),
    }
    .expect("PROJ string")
}

/// `BoundCRS::createFromTOWGS84(source, {1, 2, 3, 4, 5, 6, 7})`.
fn bound_towgs84_seven(source: Crs) -> Crs {
    Crs::Bound(
        BoundCRS::from_towgs84(source, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]).expect("bound CRS"),
    )
}

/// `TEST(operation, projCRS_3D_to_geogCRS_3D_context)`.
#[test]
fn operation_proj_crs_3d_to_geog_crs_3d_context() {
    let db = database();
    let ctxt = partial_context(&db, "EPSG");
    let src = wkt_crs(
        "PROJCRS[\"NAD83(HARN) / Oregon GIC Lambert (ft)\",\n\
         BASEGEOGCRS[\"NAD83(HARN)\",\n\
         DATUM[\"NAD83 (High Accuracy Reference Network)\",\n\
         ELLIPSOID[\"GRS 1980\",6378137,298.257222101,\n\
         LENGTHUNIT[\"metre\",1]]],\n\
         PRIMEM[\"Greenwich\",0,\n\
         ANGLEUNIT[\"degree\",0.0174532925199433]],\n\
         ID[\"EPSG\",4957]],\n\
         CONVERSION[\"unnamed\",\n\
         METHOD[\"Lambert Conic Conformal (2SP)\",\n\
         ID[\"EPSG\",9802]],\n\
         PARAMETER[\"Latitude of false origin\",41.75,\n\
         ANGLEUNIT[\"degree\",0.0174532925199433],\n\
         ID[\"EPSG\",8821]],\n\
         PARAMETER[\"Longitude of false origin\",-120.5,\n\
         ANGLEUNIT[\"degree\",0.0174532925199433],\n\
         ID[\"EPSG\",8822]],\n\
         PARAMETER[\"Latitude of 1st standard parallel\",43,\n\
         ANGLEUNIT[\"degree\",0.0174532925199433],\n\
         ID[\"EPSG\",8823]],\n\
         PARAMETER[\"Latitude of 2nd standard parallel\",45.5,\n\
         ANGLEUNIT[\"degree\",0.0174532925199433],\n\
         ID[\"EPSG\",8824]],\n\
         PARAMETER[\"Easting at false origin\",1312335.958,\n\
         LENGTHUNIT[\"foot\",0.3048],\n\
         ID[\"EPSG\",8826]],\n\
         PARAMETER[\"Northing at false origin\",0,\n\
         LENGTHUNIT[\"foot\",0.3048],\n\
         ID[\"EPSG\",8827]]],\n\
         CS[Cartesian,3],\n\
         AXIS[\"easting\",east,\n\
         ORDER[1],\n\
         LENGTHUNIT[\"foot\",0.3048]],\n\
         AXIS[\"northing\",north,\n\
         ORDER[2],\n\
         LENGTHUNIT[\"foot\",0.3048]],\n\
         AXIS[\"ellipsoidal height (h)\",up,\n\
         ORDER[3],\n\
         LENGTHUNIT[\"foot\",0.3048]]]",
    );
    let dst = epsg(&db, "4957");
    let list = operations(&src, &dst, &ctxt);
    assert_eq!(list.len(), 1);
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline \
         +step +proj=unitconvert +xy_in=ft +z_in=ft +xy_out=m +z_out=m \
         +step +inv +proj=lcc +lat_0=41.75 +lon_0=-120.5 +lat_1=43 \
         +lat_2=45.5 +x_0=399999.9999984 +y_0=0 +ellps=GRS80 \
         +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m \
         +step +proj=axisswap +order=2,1"
    );
}

/// `TEST(operation, geogCRS_to_geogCRS_context_WGS84_G1674_to_WGS84_G1762)`.
/// `TEST(operation, boundCRS_of_geogCRS_to_unrelated_geogCRS_context)`.
#[test]
fn operation_bound_crs_of_geog_crs_to_unrelated_geog_crs_context() {
    let db = database();
    let src = bound_towgs84_seven(Crs::Geographic(GeographicCRS::epsg_4807()));
    let dst = epsg(&db, "4258");
    let ctxt = context(&db, "EPSG");
    let list = operations(&src, &dst, &ctxt);
    assert_eq!(list.len(), 1);
    assert!(matches!(list[0], CoordinateOperation::Concatenated(_)));
    assert_eq!(
        list[0].name(),
        "Transformation from NTF (Paris) to WGS84 + Inverse of ETRS89 to WGS 84 (1)"
    );
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=grad +xy_out=rad \
         +step +inv +proj=longlat +ellps=clrk80ign +pm=paris \
         +step +proj=push +v_3 +step +proj=cart +ellps=clrk80ign \
         +step +proj=helmert +x=1 +y=2 +z=3 +rx=4 +ry=5 +rz=6 +s=7 \
         +convention=position_vector \
         +step +inv +proj=cart +ellps=GRS80 +step +proj=pop +v_3 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}

/// `TEST(operation, boundCRS_to_geogCRS_same_datum_context)`.
#[test]
fn operation_bound_crs_to_geog_crs_same_datum_context() {
    let db = database();
    let bound = bound_towgs84_seven(Crs::Geographic(GeographicCRS::epsg_4269()));
    let ctxt = ignore_grids_context(&db, "EPSG");
    let list = operations(&bound, &epsg(&db, "4269"), &ctxt);
    assert_eq!(list.len(), 1);
    assert_eq!(proj(&db, &list[0]), "+proj=noop");
}

/// `TEST(operation, geogCRS_to_boundCRS_of_geogCRS)`.
#[test]
fn operation_geog_crs_to_bound_crs_of_geog_crs() {
    let db = database();
    let bound = bound_towgs84_seven(Crs::Geographic(GeographicCRS::epsg_4807()));
    let op = operation(&epsg(&db, "4326"), &bound);
    assert_eq!(
        proj(&db, &op),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=push +v_3 \
         +step +proj=cart +ellps=WGS84 +step +inv +proj=helmert +x=1 \
         +y=2 +z=3 +rx=4 +ry=5 +rz=6 +s=7 +convention=position_vector \
         +step +inv +proj=cart +ellps=clrk80ign +step +proj=pop +v_3 \
         +step +proj=longlat +ellps=clrk80ign +pm=paris +step \
         +proj=unitconvert +xy_in=rad +xy_out=grad +step +proj=axisswap \
         +order=2,1"
    );
}
