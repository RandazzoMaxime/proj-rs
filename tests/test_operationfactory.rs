//! PROJ `test/unit/test_operationfactory.cpp` checks on `createOperations` between
//! registered geographic CRS, with an EPSG `CoordinateOperationContext`.
//!
//! Each function is one `TEST()` name. The expected names, EPSG codes, counts and PROJ
//! strings are copied from the C++ file (PROJ 9.9.0). The other `TEST()`s of that file
//! are open in `docs/UNPORTED-UNIT.md`.

use proj_rs::iso19111::{
    create_from_user_input, AuthorityFactory, BoundingBox, CoordinateOperation,
    CoordinateOperationContext, CoordinateOperationFactory, Crs, DatabaseContext,
    IntermediateCrsUse, ObjectDomain, ProjLines, SpatialCriterion,
};

fn database() -> DatabaseContext {
    DatabaseContext::create().expect("database")
}

fn epsg_crs(db: &DatabaseContext, code: &str) -> Crs {
    AuthorityFactory::create(db, "EPSG")
        .create_coordinate_reference_system(code)
        .expect(code)
}

fn proj_string(operation: &CoordinateOperation) -> String {
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

fn epsg_code(operation: &CoordinateOperation) -> Option<i32> {
    operation
        .identifiers()
        .iter()
        .find(|identifier| identifier.code_space() == "EPSG")
        .and_then(|identifier| identifier.code().parse().ok())
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

#[test]
fn operation_geog_crs_to_geog_crs_context_default() {
    let db = database();
    let mut ctxt = CoordinateOperationContext::create(&db, "");
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_intermediate_crs_use(IntermediateCrsUse::Never);

    // Directly found in database
    {
        let list = operations(&epsg_crs(&db, "4179"), &epsg_crs(&db, "4258"), &ctxt);
        assert_eq!(list.len(), 3);
        // Romania has a larger area than Poland (given our approx formula)
        assert_eq!(list[0].name(), "Pulkovo 1942(58) to ETRS89 (4)"); // Romania - 3m
        assert_eq!(epsg_code(&list[0]), Some(15994)); // Romania - 3m
        assert_eq!(epsg_code(&list[1]), Some(1644)); // Poland - 1m
        assert_eq!(
            list[2].name(),
            "Ballpark geographic offset from Pulkovo 1942(58) to ETRS89"
        );

        assert_eq!(
            proj_string(&list[0]),
            "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
             +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=push +v_3 \
             +step +proj=cart +ellps=krass +step +proj=helmert +x=2.3287 \
             +y=-147.0425 +z=-92.0802 +rx=0.3092483 +ry=-0.32482185 \
             +rz=-0.49729934 +s=5.68906266 +convention=coordinate_frame +step \
             +inv +proj=cart +ellps=GRS80 +step +proj=pop +v_3 +step \
             +proj=unitconvert +xy_in=rad +xy_out=deg +step +proj=axisswap \
             +order=2,1"
        );
    }

    // Reverse case
    {
        let list = operations(&epsg_crs(&db, "4258"), &epsg_crs(&db, "4179"), &ctxt);
        assert_eq!(list.len(), 3);
        // Romania has a larger area than Poland (given our approx formula)
        assert_eq!(list[0].name(), "Inverse of Pulkovo 1942(58) to ETRS89 (4)");
        assert_eq!(
            proj_string(&list[0]),
            "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
             +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=push +v_3 \
             +step +proj=cart +ellps=GRS80 +step +inv +proj=helmert +x=2.3287 \
             +y=-147.0425 +z=-92.0802 +rx=0.3092483 +ry=-0.32482185 \
             +rz=-0.49729934 +s=5.68906266 +convention=coordinate_frame +step \
             +inv +proj=cart +ellps=krass +step +proj=pop +v_3 +step \
             +proj=unitconvert +xy_in=rad +xy_out=deg +step +proj=axisswap \
             +order=2,1"
        );
    }
}

#[test]
fn operation_geog_crs_to_geog_crs_context_filter_accuracy() {
    let db = database();

    {
        let mut ctxt = CoordinateOperationContext::create(&db, "");
        ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection)
            .set_desired_accuracy(1.0);
        let list = operations(&epsg_crs(&db, "4179"), &epsg_crs(&db, "4258"), &ctxt);
        assert_eq!(list.len(), 1);
        assert_eq!(epsg_code(&list[0]), Some(1644)); // Poland - 1m
    }
    {
        let mut ctxt = CoordinateOperationContext::create(&db, "");
        ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection)
            .set_desired_accuracy(0.9);
        let list = operations(&epsg_crs(&db, "4179"), &epsg_crs(&db, "4258"), &ctxt);
        assert_eq!(list.len(), 0);
    }
}

#[test]
fn operation_geog_crs_to_geog_crs_context_filter_bbox() {
    let db = database();

    // INSERT INTO "area" VALUES('EPSG','1197','Romania','Romania - onshore and
    // offshore.',43.44,48.27,20.26,31.41,0);
    {
        let mut ctxt = CoordinateOperationContext::create(&db, "");
        ctxt.set_area_of_interest(Some(ObjectDomain::from_bounding_box(BoundingBox::new(
            20.26, 43.44, 31.41, 48.27,
        ))));
        let list = operations(&epsg_crs(&db, "4179"), &epsg_crs(&db, "4258"), &ctxt);
        assert_eq!(list.len(), 1);
        assert_eq!(epsg_code(&list[0]), Some(15994)); // Romania - 3m
    }
    {
        let mut ctxt = CoordinateOperationContext::create(&db, "");
        ctxt.set_area_of_interest(Some(ObjectDomain::from_bounding_box(BoundingBox::new(
            20.26 + 0.1,
            43.44 + 0.1,
            31.41 - 0.1,
            48.27 - 0.1,
        ))));
        let list = operations(&epsg_crs(&db, "4179"), &epsg_crs(&db, "4258"), &ctxt);
        assert_eq!(list.len(), 1);
        assert_eq!(epsg_code(&list[0]), Some(15994)); // Romania - 3m
    }
    {
        let mut ctxt = CoordinateOperationContext::create(&db, "");
        ctxt.set_area_of_interest(Some(ObjectDomain::from_bounding_box(BoundingBox::new(
            20.26 - 0.1,
            43.44 - 0.1,
            31.41 + 0.1,
            48.27 + 0.1,
        ))));
        let list = operations(&epsg_crs(&db, "4179"), &epsg_crs(&db, "4258"), &ctxt);
        assert_eq!(list.len(), 1);
        assert_eq!(proj_string(&list[0]), "+proj=noop");
    }
}

#[test]
fn operation_geog_crs_to_geog_crs_context_incompatible_area() {
    let db = database();
    let ctxt = CoordinateOperationContext::create(&db, "EPSG");
    let list = operations(
        &epsg_crs(&db, "4267"), // NAD27
        &epsg_crs(&db, "4258"), // ETRS 89
        &ctxt,
    );
    assert_eq!(list.len(), 1);
    assert_eq!(proj_string(&list[0]), "+proj=noop");
}

#[test]
fn operation_geog_crs_to_geog_crs_longitude_rotation_context() {
    let db = database();
    let ctxt = CoordinateOperationContext::create(&db, "EPSG");
    let list = operations(
        &epsg_crs(&db, "4807"), // NTF(Paris)
        &epsg_crs(&db, "4275"), // NTF
        &ctxt,
    );
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].name(), "NTF (Paris) to NTF (1)");
    assert_eq!(
        proj_string(&list[0]),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=grad +xy_out=rad +step +inv +proj=longlat \
         +ellps=clrk80ign +pm=paris +step +proj=unitconvert +xy_in=rad \
         +xy_out=deg +step +proj=axisswap +order=2,1"
    );
    assert_eq!(list[1].name(), "NTF (Paris) to NTF (2)");
    assert_eq!(
        proj_string(&list[1]),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=grad +xy_out=rad +step +inv +proj=longlat \
         +ellps=clrk80ign +pm=2.33720833333333 +step \
         +proj=unitconvert +xy_in=rad +xy_out=deg +step +proj=axisswap \
         +order=2,1"
    );
}

#[test]
fn operation_geog_crs_to_geog_crs_context_epsg_4240_indian1975_to_epsg_4326() {
    let db = database();
    let mut ctxt = CoordinateOperationContext::create(&db, "EPSG");
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection);

    let list = operations(&epsg_crs(&db, "4240"), &epsg_crs(&db, "4326"), &ctxt);
    assert_eq!(list.len(), 3);

    // Indian 1975 to WGS 84 (4), 3.0 m, Thailand - onshore
    assert_eq!(epsg_code(&list[0]), Some(1812));
    // Indian 1975 to WGS 84 (2), 5.0 m, Thailand - onshore and Gulf of Thailand
    assert_eq!(epsg_code(&list[1]), Some(1304));
    // Indian 1975 to WGS 84 (3), 1.0 m, Thailand - Bongkot field
    assert_eq!(epsg_code(&list[2]), Some(1537));
}

#[test]
fn operation_geog_crs_to_geog_crs_context_invalid_epsg_id() {
    let db = database();
    let ctxt = CoordinateOperationContext::create(&db, "EPSG");
    // EPSG:4656 is incorrect. Should be EPSG:8997
    let crs = create_from_user_input(
        "GEOGCS[\"ITRF2000\",\
         DATUM[\"International_Terrestrial_Reference_Frame_2000\",\
         SPHEROID[\"GRS 1980\",6378137,298.257222101,\
         AUTHORITY[\"EPSG\",\"7019\"]],AUTHORITY[\"EPSG\",\"6656\"]],\
         PRIMEM[\"Greenwich\",0],UNIT[\"Degree\",0.0174532925199433],\
         AUTHORITY[\"EPSG\",\"4656\"]]",
        &db,
    )
    .expect("ITRF2000");

    let list = operations(&crs, &epsg_crs(&db, "4326"), &ctxt);
    assert_eq!(list.len(), 1);
}

#[test]
fn operation_geog_crs_to_geog_crs_context_wgs84_g1674_to_wgs84_g1762() {
    // Check that particular behavior with WGS 84 (Gxxx) related to
    // 'geodetic_datum_preferred_hub' table and custom no-op transformations
    // between WGS 84 and WGS 84 (Gxxx) doesn't affect direct transformations
    // to those realizations.
    let db = database();
    let ctxt = CoordinateOperationContext::create(&db, "");

    let list = operations(
        &epsg_crs(&db, "9056"), // WGS84 (G1674)
        &epsg_crs(&db, "9057"), // WGS84 (G1762)
        &ctxt,
    );
    assert_eq!(list.len(), 1);
    assert_eq!(
        proj_string(&list[0]),
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=cart +ellps=WGS84 \
         +step +proj=helmert +x=-0.004 +y=0.003 +z=0.004 +rx=0.00027 \
         +ry=-0.00027 +rz=0.00038 +s=-0.0069 \
         +convention=coordinate_frame \
         +step +inv +proj=cart +ellps=WGS84 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}

#[test]
fn operation_geog_crs_to_geog_crs_context_deprecated() {
    let db = database();
    let ctxt = CoordinateOperationContext::create(&db, "EPSG");

    let list = operations(
        &epsg_crs(&db, "4226"), // Cote d'Ivoire (deprecated)
        &epsg_crs(&db, "4258"), // ETRS89
        &ctxt,
    );
    assert!(!list.is_empty());
    assert_eq!(
        list[0].name(),
        "Ballpark geographic offset from Cote d'Ivoire to ETRS89"
    );
}
