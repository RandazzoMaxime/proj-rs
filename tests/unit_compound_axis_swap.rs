//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, the compound CRS case that
//! checks the axis swap of a projected horizontal part to a 3D geographic CRS.
//!
//! The C++ `TEST()` name is kept in the doc comment. The expected PROJ string is
//! copied from that file. The helpers are copied from `tests/unit_opfac_core_c.rs`.

use proj_rs::iso19111::{
    create_from_user_input, AuthorityFactory, CoordinateOperation, CoordinateOperationContext,
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

fn parse(db: &DatabaseContext, text: &str) -> Crs {
    create_from_user_input(text, db).expect(text)
}

fn operations(
    source: &Crs,
    target: &Crs,
    ctx: &CoordinateOperationContext,
) -> Vec<CoordinateOperation> {
    CoordinateOperationFactory::create()
        .create_operations(source, target, ctx)
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

const UTM31N_FT_PLUS_VERT_FT: &str = r#"COMPOUNDCRS["unknown",PROJCRS["unknown",BASEGEOGCRS["unknown",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1]],ID["EPSG",6326]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8901]]],CONVERSION["UTM zone 31N",METHOD["Transverse Mercator",ID["EPSG",9807]],PARAMETER["Latitude of natural origin",0,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8801]],PARAMETER["Longitude of natural origin",3,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8802]],PARAMETER["Scale factor at natural origin",0.9996,SCALEUNIT["unity",1],ID["EPSG",8805]],PARAMETER["False easting",500000,LENGTHUNIT["metre",1],ID["EPSG",8806]],PARAMETER["False northing",0,LENGTHUNIT["metre",1],ID["EPSG",8807]],ID["EPSG",16031]],CS[Cartesian,2],AXIS["(E)",east,ORDER[1],LENGTHUNIT["US survey foot",0.304800609601219,ID["EPSG",9003]]],AXIS["(N)",north,ORDER[2],LENGTHUNIT["US survey foot",0.304800609601219,ID["EPSG",9003]]]],VERTCRS["unknown",VDATUM["unknown"],CS[vertical,1],AXIS["gravity-related height (H)",up,LENGTHUNIT["US survey foot",0.304800609601219,ID["EPSG",9003]]]]]"#;

/// TEST(operation, compoundCRS_with_non_meter_horiz_and_vertical_to_geog)
#[test]
fn operation_compound_crs_with_non_meter_horiz_and_vertical_to_geog() {
    let db = database();
    let src = parse(&db, UTM31N_FT_PLUS_VERT_FT);
    let ctx = CoordinateOperationContext::create(&db, "EPSG");

    let list = operations(&src, &epsg(&db, "4979"), &ctx);
    assert_eq!(list.len(), 1);
    // Check that vertical unit conversion is done just once
    assert_eq!(
        plain(&list[0]),
        "+proj=pipeline \
         +step +proj=unitconvert +xy_in=us-ft +xy_out=m \
         +step +inv +proj=utm +zone=31 +ellps=WGS84 \
         +step +proj=unitconvert +xy_in=rad +z_in=us-ft \
         +xy_out=deg +z_out=m \
         +step +proj=axisswap +order=2,1"
    );
}
