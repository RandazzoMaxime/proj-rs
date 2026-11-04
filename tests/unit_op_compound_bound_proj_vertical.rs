//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`: compound CRS with a bound projected CRS and
//! a bound vertical CRS to WGS 84 3D.

use proj_rs::{
    CartesianCS, Conversion, CoordinateOperation, CoordinateOperationFactory, Crs, DatabaseContext,
    GeodeticCRS, GeographicCRS, OpProps, ProjLines, ProjectedCRS, Transformation, UnitOfMeasure,
    VerticalCRS, VerticalCS, VerticalReferenceFrame, WktObject, WktParser,
};

const COMPD_FTUS: &str = r####"COMPD_CS["NAD_1983_StatePlane_Alabama_West_FIPS_0102_Feet + NAVD88 height - Geoid12B (US Feet)",
    PROJCS["NAD_1983_StatePlane_Alabama_West_FIPS_0102_Feet",
        GEOGCS["NAD83",
            DATUM["North_American_Datum_1983",
                SPHEROID["GRS 1980",6378137,298.257222101,
                    AUTHORITY["EPSG","7019"]],
                TOWGS84[0,0,0,0,0,0,0],
                AUTHORITY["EPSG","6269"]],
            PRIMEM["Greenwich",0],
            UNIT["Degree",0.0174532925199433]],
        PROJECTION["Transverse_Mercator"],
        PARAMETER["latitude_of_origin",30],
        PARAMETER["central_meridian",-87.5],
        PARAMETER["scale_factor",0.999933333333333],
        PARAMETER["false_easting",1968500],
        PARAMETER["false_northing",0],
        UNIT["US survey foot",0.304800609601219,
            AUTHORITY["EPSG","9003"]],
        AXIS["Easting",EAST],
        AXIS["Northing",NORTH],
        AUTHORITY["ESRI","102630"]],
    VERT_CS["NAVD88 height (ftUS)",
        VERT_DATUM["North American Vertical Datum 1988",2005,
            EXTENSION["PROJ4_GRIDS","foo.gtx"],
            AUTHORITY["EPSG","5103"]],
        UNIT["US survey foot",0.304800609601219,
            AUTHORITY["EPSG","9003"]],
        AXIS["Gravity-related height",UP],
        AUTHORITY["EPSG","6360"]]]"####;

fn pipeline(operation: &CoordinateOperation) -> String {
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

/// `TEST(operation, compoundCRS_with_boundProjCRS_and_boundVerticalCRS_to_geogCRS)`
#[test]
fn operation_compound_with_bound_proj_crs_and_bound_vertical_crs_to_geog() {
    let _ = DatabaseContext::create().expect("proj.db");
    let projected = Crs::Projected(ProjectedCRS::create(
        "",
        GeodeticCRS::Geographic(GeographicCRS::epsg_4807()),
        Conversion::create_utm_north(&OpProps::new(), 31).expect("UTM"),
        CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    ));
    let horizontal = Crs::Bound(
        proj_rs::BoundCRS::from_towgs84(projected, &[1., 2., 3., 4., 5., 6., 7.]).expect("bound"),
    );
    let vertical = Crs::Vertical(VerticalCRS::create(
        "myVertCRS",
        VerticalReferenceFrame::named("myVertDatum"),
        VerticalCS::create_gravity_related_height(UnitOfMeasure::metre()),
    ));
    let hub = Crs::Geographic(GeographicCRS::epsg_4979());
    let transformation = CoordinateOperation::Transformation(
        Transformation::create_gravity_related_height_to_geographic_3d(
            &OpProps::new(),
            vertical.clone(),
            hub.clone(),
            "us_nga_egm08_25.tif",
            None,
        ),
    );
    let bound_vertical = Crs::Bound(proj_rs::BoundCRS::create(
        vertical,
        hub.clone(),
        transformation,
    ));
    let compound = Crs::Compound(
        proj_rs::CompoundCRS::create("", vec![horizontal, bound_vertical]).expect("compound"),
    );
    let operation = CoordinateOperationFactory::create()
        .create_operation(&compound, &hub)
        .expect("operation")
        .expect("some operation");
    assert_eq!(
        pipeline(&operation),
        "+proj=pipeline +step +inv +proj=utm +zone=31 +ellps=clrk80ign +pm=paris \
         +step +proj=push +v_3 +step +proj=cart +ellps=clrk80ign \
         +step +proj=helmert +x=1 +y=2 +z=3 +rx=4 +ry=5 +rz=6 +s=7 \
         +convention=position_vector \
         +step +inv +proj=cart +ellps=WGS84 +step +proj=pop +v_3 \
         +step +proj=vgridshift +grids=us_nga_egm08_25.tif +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
    let inverse = CoordinateOperationFactory::create()
        .create_operation(&hub, &compound)
        .expect("inverse operation")
        .expect("some inverse operation");
    assert!(inverse.inverse().is_equivalent_to(&operation));
}

/// `TEST(operation, compoundCRS_with_boundProjCRS_with_ftus_and_boundVerticalCRS_to_geogCRS)`
#[test]
fn operation_compound_with_bound_proj_crs_with_ftus_and_bound_vertical_crs_to_geog() {
    let crs = match WktParser::new().create_from_wkt(COMPD_FTUS).expect("WKT") {
        WktObject::Crs(crs) => crs,
        other => panic!("expected a CRS: {other:?}"),
    };
    let operation = CoordinateOperationFactory::create()
        .create_operation(&crs, &Crs::Geographic(GeographicCRS::epsg_4979()))
        .expect("operation")
        .expect("some operation");
    assert_eq!(
        pipeline(&operation),
        "+proj=pipeline +step +proj=unitconvert +xy_in=us-ft +xy_out=m \
         +step +inv +proj=tmerc +lat_0=30 +lon_0=-87.5 +k=0.999933333333333 \
         +x_0=600000 +y_0=0 +ellps=GRS80 \
         +step +proj=unitconvert +z_in=us-ft +z_out=m \
         +step +proj=vgridshift +grids=foo.gtx +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}
