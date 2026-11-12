//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, `compoundCRS_to_compoundCRS_with_vertical_transform`
//! (line 5451). The C++ objects are written as WKT2; the expected PROJ strings are copied from the C++.
//! The `+approx` blocks are not ported: the operation-level export has no approximate-TM option yet.

use proj_rs::iso19111::{
    create_from_user_input, CoordinateOperationFactory, DatabaseContext, ProjLines,
};

const ELLPS_WGS84: &str = "BASEGEOGCRS[\"WGS 84\",\n\
    DATUM[\"World Geodetic System 1984\",\n\
        ELLIPSOID[\"WGS 84\",6378137,298.257223563,LENGTHUNIT[\"metre\",1]]],\n\
    PRIMEM[\"Greenwich\",0,ANGLEUNIT[\"degree\",0.0174532925199433]]]";

const DEGREE: &str = "ANGLEUNIT[\"degree\",0.0174532925199433]";

fn compound_1() -> String {
    format!(
        "COMPOUNDCRS[\"compound1\",\n\
        PROJCRS[\"unknown\",\n\
            {ELLPS_WGS84},\n\
            CONVERSION[\"unknown\",\n\
                METHOD[\"Transverse Mercator\"],\n\
                PARAMETER[\"Latitude of natural origin\",1,{DEGREE}],\n\
                PARAMETER[\"Longitude of natural origin\",2,{DEGREE}],\n\
                PARAMETER[\"Scale factor at natural origin\",3,SCALEUNIT[\"unity\",1]],\n\
                PARAMETER[\"False easting\",4,LENGTHUNIT[\"metre\",1]],\n\
                PARAMETER[\"False northing\",5,LENGTHUNIT[\"metre\",1]]],\n\
            CS[Cartesian,2],\n\
                AXIS[\"easting\",east],\n\
                AXIS[\"northing\",north],\n\
                LENGTHUNIT[\"metre\",1]],\n\
        BOUNDCRS[\n\
            SOURCECRS[\n\
                VERTCRS[\"ODN height\",\n\
                    VDATUM[\"Ordnance Datum Newlyn\",ID[\"EPSG\",5101]],\n\
                    CS[vertical,1],\n\
                        AXIS[\"gravity-related height (H)\",up],\n\
                        LENGTHUNIT[\"metre\",1],\n\
                    ID[\"EPSG\",5701]]],\n\
            TARGETCRS[\n\
                VERTCRS[\"unknown\",\n\
                    VDATUM[\"unknown\"],\n\
                    CS[vertical,1],\n\
                        AXIS[\"gravity-related height (H)\",up],\n\
                        LENGTHUNIT[\"metre\",1]]],\n\
            ABRIDGEDTRANSFORMATION[\"unknown\",\n\
                METHOD[\"Vertical Offset by Grid Interpolation (VERTCON)\",ID[\"EPSG\",9658]],\n\
                PARAMETERFILE[\"Vertical offset file\",\"bla.gtx\",ID[\"EPSG\",8732]]]]]"
    )
}

fn compound_2() -> String {
    format!(
        "COMPOUNDCRS[\"compound2\",\n\
        PROJCRS[\"WGS 84 / UTM zone 32N\",\n\
            {ELLPS_WGS84},\n\
            CONVERSION[\"UTM zone 32N\",\n\
                METHOD[\"Transverse Mercator\"],\n\
                PARAMETER[\"Latitude of natural origin\",0,{DEGREE}],\n\
                PARAMETER[\"Longitude of natural origin\",9,{DEGREE}],\n\
                PARAMETER[\"Scale factor at natural origin\",0.9996,SCALEUNIT[\"unity\",1]],\n\
                PARAMETER[\"False easting\",500000,LENGTHUNIT[\"metre\",1]],\n\
                PARAMETER[\"False northing\",0,LENGTHUNIT[\"metre\",1]]],\n\
            CS[Cartesian,2],\n\
                AXIS[\"easting\",east],\n\
                AXIS[\"northing\",north],\n\
                LENGTHUNIT[\"metre\",1]],\n\
        VERTCRS[\"unknown\",\n\
            VDATUM[\"unknown\"],\n\
            CS[vertical,1],\n\
                AXIS[\"gravity-related height (H)\",up],\n\
                LENGTHUNIT[\"metre\",1]]]"
    )
}

fn parse(db: &DatabaseContext, text: &str) -> proj_rs::iso19111::Crs {
    create_from_user_input(text, db).expect(text)
}

/// `TEST(operation, compoundCRS_to_compoundCRS_with_vertical_transform)`
#[test]
fn operation_compound_to_compound_with_vertical_transform() {
    let db = DatabaseContext::create().expect("proj.db");
    let compound1 = parse(&db, &compound_1());
    let compound2 = parse(&db, &compound_2());

    let op = CoordinateOperationFactory::create()
        .create_operation(&compound1, &compound2)
        .expect("operation")
        .expect("an operation");
    assert_eq!(
        op.export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline +step +inv +proj=tmerc +lat_0=1 +lon_0=2 +k=3 \
         +x_0=4 +y_0=5 +ellps=WGS84 +step \
         +proj=vgridshift +grids=bla.gtx +multiplier=0.001 +step \
         +proj=utm +zone=32 +ellps=WGS84"
    );

    let op_inverse = CoordinateOperationFactory::create()
        .create_operation(&compound2, &compound1)
        .expect("operation")
        .expect("an operation");
    assert_eq!(
        op_inverse
            .inverse()
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        op.export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
    );
}
