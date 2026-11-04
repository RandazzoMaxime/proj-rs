//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`: a NAD83(2011) compound CRS with a NAVD88
//! height in metres, feet, or US feet, to NAD83(2011) 3D, in metres or feet.

use proj_rs::{
    create_from_user_input, AuthorityFactory, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, Crs, DatabaseContext, GridAvailabilityUse, ProjLines,
    SpatialCriterion, UnitOfMeasure,
};

struct Fixture {
    db: DatabaseContext,
}

impl Fixture {
    fn new() -> Self {
        Self {
            db: DatabaseContext::create().expect("proj.db"),
        }
    }

    fn context(&self) -> CoordinateOperationContext {
        let mut context = CoordinateOperationContext::create(&self.db, "EPSG");
        context
            .set_spatial_criterion(SpatialCriterion::PartialIntersection)
            .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
        context
    }

    fn compound(&self, text: &str) -> Crs {
        create_from_user_input(text, &self.db).expect("compound")
    }

    fn nad83_2011_3d(&self) -> Crs {
        AuthorityFactory::create(&self.db, "EPSG")
            .create_coordinate_reference_system("6319")
            .expect("6319")
    }

    fn operations(&self, source: &Crs, target: &Crs) -> Vec<CoordinateOperation> {
        CoordinateOperationFactory::create()
            .create_operations(source, target, &self.context())
            .expect("operations")
    }

    fn proj(&self, operation: &CoordinateOperation) -> String {
        operation
            .export_to_proj_string_with_database(&self.db, ProjLines::Single)
            .expect("PROJ string")
    }
}

/// `TEST(operation, compoundCRS_ft_to_geogCRS_ft)`
#[test]
fn operation_compound_ft_to_geog_ft() {
    let f = Fixture::new();
    let source = f.compound("EPSG:6318+8228");
    let target = f
        .nad83_2011_3d()
        .alter_cs_linear_unit(&UnitOfMeasure::foot());
    let res = f.operations(&source, &target);
    assert!(!res.is_empty());
    assert_eq!(
        f.proj(&res[0]),
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +z_in=ft +xy_out=rad +z_out=m \
         +step +proj=vgridshift +grids=us_noaa_g2018u0.tif +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=ft \
         +step +proj=axisswap +order=2,1"
    );
    assert_eq!(f.proj(res.last().expect("last")), "+proj=noop");
    let inverse = f.operations(&target, &source);
    assert!(!inverse.is_empty());
    assert_eq!(
        f.proj(&inverse[0]),
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +z_in=ft +xy_out=rad +z_out=m \
         +step +inv +proj=vgridshift +grids=us_noaa_g2018u0.tif +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=ft \
         +step +proj=axisswap +order=2,1"
    );
    assert_eq!(f.proj(inverse.last().expect("last")), "+proj=noop");
}

/// `TEST(operation, compoundCRS_m_to_geogCRS_ft)`
#[test]
fn operation_compound_m_to_geog_ft() {
    let f = Fixture::new();
    let source = f.compound("EPSG:6318+5703");
    let target = f
        .nad83_2011_3d()
        .alter_cs_linear_unit(&UnitOfMeasure::foot());
    let res = f.operations(&source, &target);
    assert!(!res.is_empty());
    assert_eq!(
        f.proj(&res[0]),
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=vgridshift +grids=us_noaa_g2018u0.tif +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=ft \
         +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        f.proj(res.last().expect("last")),
        "+proj=unitconvert +z_in=m +z_out=ft"
    );
    let inverse = f.operations(&target, &source);
    assert!(!inverse.is_empty());
    assert_eq!(
        f.proj(&inverse[0]),
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +z_in=ft +xy_out=rad +z_out=m \
         +step +inv +proj=vgridshift +grids=us_noaa_g2018u0.tif +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        f.proj(inverse.last().expect("last")),
        "+proj=unitconvert +z_in=ft +z_out=m"
    );
}

/// `TEST(operation, compoundCRS_to_geogCRS_with_vertical_unit_change)`
#[test]
fn operation_compound_to_geog_with_vertical_unit_change() {
    let f = Fixture::new();
    let target = f.nad83_2011_3d();
    let source = f.compound("EPSG:6318+6360");
    let metre = f.compound("EPSG:6318+5703");
    let list = f.operations(&source, &target);
    assert!(!list.is_empty());
    let list_metre = f.operations(&metre, &target);
    assert_eq!(list.len(), list_metre.len());
    assert_eq!(
        list[0].name(),
        format!(
            "Conversion from NAVD88 height (ftUS) to NAVD88 height + {}",
            list_metre[0].name()
        )
    );
    assert_eq!(
        f.proj(&list[0]),
        f.proj(&list_metre[0]).replace(
            "+step +proj=unitconvert +xy_in=deg +xy_out=rad",
            "+step +proj=unitconvert +xy_in=deg +z_in=us-ft +xy_out=rad +z_out=m"
        )
    );
    assert_eq!(f.operations(&target, &source).len(), list.len());
}
