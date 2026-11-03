//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, further `core` cases of
//! `createOperations`. Each function keeps the C++ `TEST()` name. Expected names,
//! counts and PROJ strings are copied from that file.

use proj_rs::iso19111::{
    AuthorityFactory, BoundCRS, CartesianCS, Conversion, CoordinateOperation,
    CoordinateOperationContext, CoordinateOperationFactory, Crs, CrsExtentUse, DatabaseContext,
    GeodeticCRS, GeographicCRS, GridAvailabilityUse, OpProps, ProjLines, ProjectedCRS,
    SpatialCriterion, Transformation, UnitOfMeasure, WktParser,
};

fn database() -> DatabaseContext {
    DatabaseContext::create().expect("database")
}

fn epsg(db: &DatabaseContext, code: &str) -> Crs {
    AuthorityFactory::create(db, "EPSG")
        .create_coordinate_reference_system(code)
        .expect(code)
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

fn operation(source: &Crs, target: &Crs) -> CoordinateOperation {
    CoordinateOperationFactory::create()
        .create_operation(source, target)
        .expect("operation")
        .expect("an operation")
}

fn proj(db: &DatabaseContext, operation: &CoordinateOperation) -> String {
    operation
        .export_to_proj_string_with_database(db, ProjLines::Single)
        .expect("PROJ string")
}

fn ctx_epsg(db: &DatabaseContext) -> CoordinateOperationContext {
    CoordinateOperationContext::create(db, "EPSG")
}

fn ctx_any(db: &DatabaseContext) -> CoordinateOperationContext {
    CoordinateOperationContext::create(db, "")
}

fn partial_epsg(db: &DatabaseContext) -> CoordinateOperationContext {
    let mut context = ctx_epsg(db);
    context.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    context
}

fn ignore_partial_any(db: &DatabaseContext) -> CoordinateOperationContext {
    let mut context = ctx_any(db);
    context
        .set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    context
}

/// `BoundCRS::createFromTOWGS84` for a geographic source: position vector to WGS 84.
fn bound_towgs84(source: Crs) -> Crs {
    let target = Crs::Geographic(GeographicCRS::epsg_4326());
    let transformation = Transformation::create_position_vector(
        &OpProps::new().name(format!("Transformation from {} to WGS84", source.name())),
        source.clone(),
        target.clone(),
        [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
        None,
    )
    .expect("position vector");
    Crs::Bound(BoundCRS::create(
        source,
        target,
        CoordinateOperation::Transformation(transformation),
    ))
}

fn utm(zone: i32, base: GeographicCRS) -> Crs {
    Crs::Projected(ProjectedCRS::create(
        format!("UTM zone {zone}N"),
        GeodeticCRS::Geographic(base),
        Conversion::create_utm_north(&OpProps::new(), zone).expect("utm"),
        CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    ))
}

#[test]
fn operation_geocentric_crs_to_geocentric_crs_different_datum_context() {
    let db = database();
    let ctxt = ctx_epsg(&db);
    let list = operations(&epsg(&db, "4919"), &epsg(&db, "4896"), &ctxt);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name(), "ITRF2000 to ITRF2005 (1)");
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=helmert +x=-0.0001 \
         +y=0.0008 +z=0.0058 +rx=0 +ry=0 +rz=0 +s=-0.0004 +dx=0.0002 \
         +dy=-0.0001 +dz=0.0018 +drx=0 +dry=0 +drz=0 +ds=-8e-05 \
         +t_epoch=2000 +convention=position_vector"
    );
}

#[test]
fn operation_geog2_d_to_geog3_d_same_datum_but_with_potential_other_pivot_context() {
    let db = database();
    let ctxt = ctx_epsg(&db);
    let list = operations(&epsg(&db, "5365"), &epsg(&db, "5364"), &ctxt);
    assert_eq!(list.len(), 1);
    assert_eq!(proj(&db, &list[0]), "+proj=noop");
}

#[test]
fn operation_geog_crs_to_geog_crs_different_datum_though_geocentric_transform_context() {
    let db = database();
    let ctxt = ctx_epsg(&db);
    let list = operations(&epsg(&db, "7909"), &epsg(&db, "7910"), &ctxt);
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].name(),
        "Conversion from ITRF2000 (geog3D) to ITRF2000 (geocentric) + \
         ITRF2000 to ITRF2005 (1) + \
         Conversion from ITRF2005 (geocentric) to ITRF2005 (geog3D)"
    );
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m \
         +step +proj=cart +ellps=GRS80 +step +proj=helmert +x=-0.0001 \
         +y=0.0008 +z=0.0058 +rx=0 +ry=0 +rz=0 +s=-0.0004 +dx=0.0002 \
         +dy=-0.0001 +dz=0.0018 +drx=0 +dry=0 +drz=0 +ds=-8e-05 \
         +t_epoch=2000 +convention=position_vector +step +inv \
         +proj=cart +ellps=GRS80 +step +proj=unitconvert +xy_in=rad \
         +z_in=m +xy_out=deg +z_out=m +step +proj=axisswap +order=2,1"
    );
    assert!(list[0].requires_per_coordinate_input_time());
}

#[test]
fn operation_geog_crs_to_geocentric_crs_different_datum_context() {
    let db = database();
    let ctxt = ctx_epsg(&db);
    let list = operations(&epsg(&db, "7909"), &epsg(&db, "4896"), &ctxt);
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].name(),
        "Conversion from ITRF2000 (geog3D) to ITRF2000 (geocentric) + \
         ITRF2000 to ITRF2005 (1)"
    );
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m \
         +step +proj=cart +ellps=GRS80 +step +proj=helmert +x=-0.0001 \
         +y=0.0008 +z=0.0058 +rx=0 +ry=0 +rz=0 +s=-0.0004 +dx=0.0002 \
         +dy=-0.0001 +dz=0.0018 +drx=0 +dry=0 +drz=0 +ds=-8e-05 \
         +t_epoch=2000 +convention=position_vector"
    );
}

#[test]
fn operation_geocentric_crs_to_geog_crs_different_datum_context() {
    let db = database();
    let ctxt = ctx_epsg(&db);
    let list = operations(&epsg(&db, "4919"), &epsg(&db, "7910"), &ctxt);
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].name(),
        "ITRF2000 to ITRF2005 (1) + \
         Conversion from ITRF2005 (geocentric) to ITRF2005 (geog3D)"
    );
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline +step +proj=helmert +x=-0.0001 \
         +y=0.0008 +z=0.0058 +rx=0 +ry=0 +rz=0 +s=-0.0004 +dx=0.0002 \
         +dy=-0.0001 +dz=0.0018 +drx=0 +dry=0 +drz=0 +ds=-8e-05 \
         +t_epoch=2000 +convention=position_vector +step +inv \
         +proj=cart +ellps=GRS80 +step +proj=unitconvert +xy_in=rad \
         +z_in=m +xy_out=deg +z_out=m +step +proj=axisswap +order=2,1"
    );
}

#[test]
fn operation_proj_crs_to_proj_crs_north_pole_inverted_axis() {
    let db = database();
    let ctxt = ctx_any(&db);
    let list = operations(&epsg(&db, "32661"), &epsg(&db, "5041"), &ctxt);
    assert_eq!(list.len(), 1);
    assert_eq!(proj(&db, &list[0]), "+proj=axisswap +order=2,1");
}

#[test]
fn operation_proj_crs_to_proj_crs_south_pole_inverted_axis() {
    let db = database();
    let ctxt = ctx_any(&db);
    let list = operations(&epsg(&db, "32761"), &epsg(&db, "5042"), &ctxt);
    assert_eq!(list.len(), 1);
    assert_eq!(proj(&db, &list[0]), "+proj=axisswap +order=2,1");
}

#[test]
fn operation_proj_crs_to_proj_crs_through_geog3_d() {
    let db = database();
    let ctxt = ctx_epsg(&db);
    let list = operations(&epsg(&db, "5367"), &epsg(&db, "8908"), &ctxt);
    assert_eq!(list.len(), 1);
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +inv +proj=tmerc +lat_0=0 +lon_0=-84 +k=0.9999 \
         +x_0=500000 +y_0=0 +ellps=WGS84 \
         +step +proj=push +v_3 \
         +step +proj=cart +ellps=WGS84 \
         +step +proj=helmert +x=-0.16959 +y=0.35312 +z=0.51846 \
         +rx=-0.03385 +ry=0.16325 +rz=-0.03446 +s=0.03693 \
         +convention=coordinate_frame \
         +step +inv +proj=cart +ellps=GRS80 \
         +step +proj=pop +v_3 \
         +step +proj=tmerc +lat_0=0 +lon_0=-84 +k=0.9999 +x_0=500000 \
         +y_0=0 +ellps=GRS80"
    );
}

#[test]
fn operation_transform_from_amersfoort_rd_new_to_epsg_4326() {
    let db = database();
    let ctxt = partial_epsg(&db);
    let list = operations(&epsg(&db, "28992"), &epsg(&db, "4326"), &ctxt);
    assert!(!list.is_empty());
    assert_eq!(list[0].name(), "Inverse of RD + Amersfoort to WGS 84 (4)");
}

#[test]
fn operation_bound_crs_of_geog_crs_to_geog_crs() {
    let db = database();
    let bound = bound_towgs84(Crs::Geographic(GeographicCRS::epsg_4807()));
    let op = operation(&bound, &Crs::Geographic(GeographicCRS::epsg_4326()));
    assert_eq!(
        proj(&db, &op),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=grad +xy_out=rad +step +inv \
         +proj=longlat +ellps=clrk80ign +pm=paris +step +proj=push +v_3 \
         +step +proj=cart +ellps=clrk80ign +step +proj=helmert +x=1 +y=2 \
         +z=3 +rx=4 +ry=5 +rz=6 +s=7 +convention=position_vector +step \
         +inv +proj=cart +ellps=WGS84 +step +proj=pop +v_3 +step \
         +proj=unitconvert +xy_in=rad +xy_out=deg +step +proj=axisswap \
         +order=2,1"
    );
}

#[test]
fn operation_bound_crs_of_geog_crs_to_geog_crs_with_area() {
    let db = database();
    let bound = bound_towgs84(Crs::Geographic(GeographicCRS::epsg_4267()));
    let op = operation(&bound, &epsg(&db, "4326"));
    assert_eq!(
        proj(&db, &op),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=push +v_3 \
         +step +proj=cart +ellps=clrk66 +step +proj=helmert +x=1 +y=2 \
         +z=3 +rx=4 +ry=5 +rz=6 +s=7 +convention=position_vector +step \
         +inv +proj=cart +ellps=WGS84 +step +proj=pop +v_3 +step \
         +proj=unitconvert +xy_in=rad +xy_out=deg +step +proj=axisswap \
         +order=2,1"
    );
}

#[test]
fn operation_bound_crs_of_geog_crs_to_unrelated_geog_crs() {
    let db = database();
    let ntf = Crs::Geographic(GeographicCRS::epsg_4807());
    let nad = Crs::Geographic(GeographicCRS::epsg_4269());
    let bound = bound_towgs84(ntf.clone());
    assert_eq!(
        proj(&db, &operation(&bound, &nad)),
        proj(&db, &operation(&ntf, &nad))
    );
}

#[test]
fn operation_bound_crs_of_geog_crs_to_proj_crs() {
    let db = database();
    let bound = bound_towgs84(Crs::Geographic(GeographicCRS::epsg_4807()));
    let utm31 = utm(31, GeographicCRS::epsg_4326());
    let op = operation(&bound, &utm31);
    assert_eq!(
        proj(&db, &op),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=grad +xy_out=rad +step +inv \
         +proj=longlat +ellps=clrk80ign +pm=paris +step +proj=push +v_3 \
         +step +proj=cart +ellps=clrk80ign +step +proj=helmert +x=1 +y=2 \
         +z=3 +rx=4 +ry=5 +rz=6 +s=7 +convention=position_vector +step \
         +inv +proj=cart +ellps=WGS84 +step +proj=pop +v_3 +step \
         +proj=utm +zone=31 +ellps=WGS84"
    );
}

#[test]
fn operation_proj_crs_to_proj_crs_context_compatible_area() {
    let db = database();
    let ctxt = ctx_epsg(&db);
    let list = operations(&epsg(&db, "32634"), &epsg(&db, "2171"), &ctxt);
    assert_eq!(list.len(), 2);
    assert_eq!(
        list[0].name(),
        "Inverse of UTM zone 34N + Inverse of Pulkovo 1942(58) to WGS 84 \
         (1) + Poland zone I"
    );
    assert_eq!(list[0].accuracy(), Some(1.0));
}

#[test]
fn operation_proj_crs_to_proj_crs_context_compatible_area_bis() {
    let db = database();
    let ctxt = ctx_epsg(&db);
    let list = operations(&epsg(&db, "3844"), &epsg(&db, "32634"), &ctxt);
    assert_eq!(list.len(), 3);
    assert_eq!(
        list[0].name(),
        "Inverse of Stereo 70 + Pulkovo 1942(58) to WGS 84 (19) + UTM zone 34N"
    );
    assert_eq!(list[0].accuracy(), Some(3.0));
}

#[test]
fn operation_proj_crs_to_proj_crs_context_one_incompatible_area() {
    let db = database();
    let ctxt = ctx_epsg(&db);
    let list = operations(&epsg(&db, "32631"), &epsg(&db, "2171"), &ctxt);
    assert_eq!(list.len(), 2);
    assert_eq!(
        list[0].name(),
        "Inverse of UTM zone 31N + Inverse of Pulkovo 1942(58) to WGS 84 \
         (1) + Poland zone I"
    );
    assert_eq!(list[0].accuracy(), Some(1.0));
}

#[test]
fn operation_proj_crs_to_proj_crs_context_incompatible_areas() {
    let db = database();
    let ctxt = ctx_epsg(&db);
    let list = operations(&epsg(&db, "32631"), &epsg(&db, "32633"), &ctxt);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name(), "Inverse of UTM zone 31N + UTM zone 33N");
    assert_eq!(list[0].accuracy(), Some(0.0));
}

#[test]
fn operation_proj_crs_to_proj_crs_context_incompatible_areas_ballpark() {
    let db = database();
    let ctxt = ctx_epsg(&db);
    let list = operations(&epsg(&db, "26711"), &epsg(&db, "3034"), &ctxt);
    assert!(!list.is_empty());
    assert!(list[0].has_ballpark_transformation());
}

#[test]
fn operation_geog_crs_to_geog_crs_context_concatenated_operation_egypt1907_to_wgs84() {
    let db = database();
    let ctxt = ctx_epsg(&db);
    let list = operations(&epsg(&db, "4229"), &epsg(&db, "4326"), &ctxt);
    assert_eq!(list.len(), 3);
    assert_eq!(list[1].name(), "Egypt 1907 to WGS 84 (2)");
    assert_eq!(list[1].accuracy(), Some(6.0));
}

#[test]
fn operation_geog_crs_to_geog_crs_wgs84_to_gda2020() {
    let db = database();
    let mut ctxt = partial_epsg(&db);
    ctxt.set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    let forward = operations(&epsg(&db, "7844"), &epsg(&db, "4326"), &ctxt);
    assert!(!forward.is_empty());
    assert_eq!(forward[0].name(), "GDA2020 to WGS 84 (2)");
    let inverse = operations(&epsg(&db, "4326"), &epsg(&db, "7844"), &ctxt);
    assert!(!inverse.is_empty());
    assert_eq!(inverse[0].name(), "Inverse of GDA2020 to WGS 84 (2)");
}

#[test]
fn operation_create_operation_epsg_10622_local_orthographic() {
    let db = database();
    let ctxt = ignore_partial_any(&db);
    let list = operations(&epsg(&db, "6318"), &epsg(&db, "10622"), &ctxt);
    assert!(!list.is_empty());
    assert!(!list[0].has_ballpark_transformation());
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=ortho \
         +lat_0=37.6289686531 +lon_0=-122.3939412704 \
         +alpha=27.7928209333 +k=0.9999968 +x_0=0 +y_0=0 +ellps=GRS80 \
         +step +proj=unitconvert +xy_in=m +xy_out=us-ft"
    );
}

#[test]
fn operation_proj_crs_to_geog_crs_crs_extent_use_none() {
    let db = database();
    let strict = ctx_epsg(&db);
    let strict_list = operations(&epsg(&db, "23031"), &epsg(&db, "4326"), &strict);
    assert!(strict_list
        .iter()
        .all(|operation| !operation.name().contains("ED50 to WGS 84 (42)")));
    let mut ignored = ctx_epsg(&db);
    ignored.set_crs_extent_use(CrsExtentUse::Ignore);
    let open = operations(&epsg(&db, "23031"), &epsg(&db, "4326"), &ignored);
    assert!(open
        .iter()
        .any(|operation| operation.name().contains("ED50 to WGS 84 (42)")));
}

#[test]
fn operation_proj_crs_to_proj_crs_context_incompatible_areas_crs_extent_use_intersection() {
    // C++ only asserts `list.size() >= 0`: INTERSECTION must not fail the search.
    let db = database();
    let mut context = ctx_epsg(&db);
    context.set_crs_extent_use(CrsExtentUse::Intersection);
    assert!(CoordinateOperationFactory::create()
        .create_operations(&epsg(&db, "26711"), &epsg(&db, "3034"), &context)
        .is_ok());
}

#[test]
fn operation_proj_crs_3_d_to_proj_crs_2_d_context() {
    let db = database();
    let mut context = ctx_epsg(&db);
    context.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    let wkt = r#"PROJCRS["Projected 3d CRS",
    BASEGEOGCRS["JGD2000",
        DATUM["Japanese Geodetic Datum 2000",
            ELLIPSOID["GRS 1980",6378137,298.257222101,
                LENGTHUNIT["metre",1]]],
        PRIMEM["Greenwich",0,
            ANGLEUNIT["degree",0.0174532925199433]],
        ID["EPSG",4947]],
    CONVERSION["Japan Plane Rectangular CS zone VII",
        METHOD["Transverse Mercator",
            ID["EPSG",9807]],
        PARAMETER["Latitude of natural origin",36,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",8801]],
        PARAMETER["Longitude of natural origin",137.166666666667,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",8802]],
        PARAMETER["Scale factor at natural origin",0.9999,
            SCALEUNIT["unity",1],
            ID["EPSG",8805]],
        PARAMETER["False easting",0,
            LENGTHUNIT["metre",1],
            ID["EPSG",8806]],
        PARAMETER["False northing",0,
            LENGTHUNIT["metre",1],
            ID["EPSG",8807]],
        ID["EPSG",17807]],
    CS[Cartesian,3],
        AXIS["northing (X)",north,
            ORDER[1],
            LENGTHUNIT["metre",1,
                ID["EPSG",9001]]],
        AXIS["easting (Y)",east,
            ORDER[2],
            LENGTHUNIT["metre",1,
                ID["EPSG",9001]]],
        AXIS["ellipsoidal height (h)",up,
            ORDER[3],
            LENGTHUNIT["metre",1,
                ID["EPSG",9001]]]]"#;
    let source = WktParser::new()
        .create_from_wkt(wkt)
        .expect("wkt")
        .crs()
        .expect("crs")
        .clone();
    let list = operations(&source, &epsg(&db, "32653"), &context);
    assert!(!list.is_empty());
}
