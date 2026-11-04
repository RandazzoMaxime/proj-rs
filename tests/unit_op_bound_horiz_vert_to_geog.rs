//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`: compound CRS of bound horizontal and vertical
//! CRS to a geographic CRS.

use proj_rs::{
    create_from_user_input, AuthorityFactory, CoordinateOperationContext,
    CoordinateOperationFactory, DatabaseContext, GridAvailabilityUse, ProjLines,
};

const BOUND_COMPOUND: &str = r####"COMPOUNDCRS["CH1903 / LV03 + EGM96 height",
    BOUNDCRS[
        SOURCECRS[
            PROJCRS["CH1903 / LV03",
                BASEGEOGCRS["CH1903",
                    DATUM["CH1903",
                        ELLIPSOID["Bessel 1841",6377397.155,299.1528128,
                            LENGTHUNIT["metre",1]]],
                    PRIMEM["Greenwich",0,
                        ANGLEUNIT["degree",0.0174532925199433]],
                    ID["EPSG",4149]],
                CONVERSION["Swiss Oblique Mercator 1903M",
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
                CS[Cartesian,2],
                    AXIS["easting (Y)",east,
                        ORDER[1],
                        LENGTHUNIT["metre",1]],
                    AXIS["northing (X)",north,
                        ORDER[2],
                        LENGTHUNIT["metre",1]]]],
        TARGETCRS[
            GEOGCRS["WGS 84",
                DATUM["World Geodetic System 1984",
                    ELLIPSOID["WGS 84",6378137,298.257223563,
                        LENGTHUNIT["metre",1]]],
                PRIMEM["Greenwich",0,
                    ANGLEUNIT["degree",0.0174532925199433]],
                CS[ellipsoidal,2],
                    AXIS["latitude",north,
                        ORDER[1],
                        ANGLEUNIT["degree",0.0174532925199433]],
                    AXIS["longitude",east,
                        ORDER[2],
                        ANGLEUNIT["degree",0.0174532925199433]],
                ID["EPSG",4326]]],
        ABRIDGEDTRANSFORMATION["CH1903 to WGS 84 (2)",
            VERSION["BfL-CH 2"],
            METHOD["Geocentric translations (geog2D domain)",
                ID["EPSG",9603]],
            PARAMETER["X-axis translation",674.374,
                ID["EPSG",8605]],
            PARAMETER["Y-axis translation",15.056,
                ID["EPSG",8606]],
            PARAMETER["Z-axis translation",405.346,
                ID["EPSG",8607]]]],
    BOUNDCRS[
        SOURCECRS[
            VERTCRS["EGM96 height",
                VDATUM["EGM96 geoid"],
                CS[vertical,1],
                    AXIS["gravity-related height (H)",up,
                        LENGTHUNIT["metre",1]],
                USAGE[
                    SCOPE["Geodesy."],
                    AREA["World."],
                    BBOX[-90,-180,90,180]],
                ID["EPSG",5773]]],
        TARGETCRS[
            GEOGCRS["WGS 84",
                DATUM["World Geodetic System 1984",
                    ELLIPSOID["WGS 84",6378137,298.257223563,
                        LENGTHUNIT["metre",1]]],
                PRIMEM["Greenwich",0,
                    ANGLEUNIT["degree",0.0174532925199433]],
                CS[ellipsoidal,3],
                    AXIS["latitude",north,
                        ORDER[1],
                        ANGLEUNIT["degree",0.0174532925199433]],
                    AXIS["longitude",east,
                        ORDER[2],
                        ANGLEUNIT["degree",0.0174532925199433]],
                    AXIS["ellipsoidal height",up,
                        ORDER[3],
                        LENGTHUNIT["metre",1]],
                ID["EPSG",4979]]],
        ABRIDGEDTRANSFORMATION["WGS 84 to EGM96 height (1)",
            METHOD["Geographic3D to GravityRelatedHeight (EGM)",
                ID["EPSG",9661]],
            PARAMETERFILE["Geoid (height correction) model file","us_nga_egm96_15.tif"]]]]"####;

/// `TEST(operation, compoundCRS_of_bound_horizCRS_and_bound_vertCRS_to_geogCRS)`
#[test]
fn operation_compound_of_bound_horiz_crs_and_bound_vert_crs_to_geog() {
    let db = DatabaseContext::create().expect("proj.db");
    let mut context = CoordinateOperationContext::create(&db, "EPSG");
    context.set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    let source = create_from_user_input(BOUND_COMPOUND, &db).expect("source");
    let target = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_reference_system("4979")
        .expect("4979");
    let list = CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations");
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0]
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline +step +inv +proj=somerc +lat_0=46.9524055555556 \
         +lon_0=7.43958333333333 +k_0=1 +x_0=600000 +y_0=200000 +ellps=bessel \
         +step +proj=push +v_3 +step +proj=cart +ellps=bessel \
         +step +proj=helmert +x=674.374 +y=15.056 +z=405.346 \
         +step +inv +proj=cart +ellps=WGS84 +step +proj=pop +v_3 \
         +step +proj=vgridshift +grids=us_nga_egm96_15.tif +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}
