//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`: compound CRS to compound CRS, and bound CRS to
//! compound CRS, third lot. The C++ `TEST()` names are kept in the doc comments and the expected names
//! and PROJ strings are copied from that file.

use proj_rs::iso19111::{
    create_from_user_input, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, Crs, DatabaseContext, ProjLines, SpatialCriterion, WktObject,
    WktParser,
};

fn ctx_partial(db: &DatabaseContext, authority: &str) -> CoordinateOperationContext {
    let mut ctxt = CoordinateOperationContext::create(db, authority);
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    ctxt
}

/// `createFromUserInput("EPSG:<code>")`, which also reads compound codes such as `4150+5773`.
fn epsg(db: &DatabaseContext, code: &str) -> Crs {
    create_from_user_input(&format!("EPSG:{code}"), db).expect(code)
}

fn wkt_crs(db: &DatabaseContext, text: &str) -> Crs {
    let mut parser = WktParser::new();
    parser.attach_database(db.clone());
    match parser.create_from_wkt(text).expect("WKT") {
        WktObject::Crs(crs) => crs,
        _ => panic!("not a CRS"),
    }
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

/// `exportToPROJString` with the database for the grid names.
fn proj(db: &DatabaseContext, operation: &CoordinateOperation) -> String {
    operation
        .export_to_proj_string_with_database(db, ProjLines::Single)
        .expect("PROJ string")
}

/// `HBOUND_SRC`: the C++ string literal, concatenated.
const HBOUND_SRC: &str = r#"COMPOUNDCRS["AGD66 / AMG zone 55 + AHD height",
    BOUNDCRS[
        SOURCECRS[
            PROJCRS["AGD66 / AMG zone 55 + AHD height",
                BASEGEOGCRS["AGD66",
                    DATUM["User datum (no grid)",
                        ELLIPSOID["Australian National Spheroid",6378160,298.25,
                            LENGTHUNIT["metre",1]]],
                    PRIMEM["Greenwich",0,
                        ANGLEUNIT["degree",0.0174532925199433]]],
                CONVERSION["Australian Map Grid zone 55",
                    METHOD["Transverse Mercator"],
                    PARAMETER["Latitude of natural origin",0,
                        ANGLEUNIT["degree",0.0174532925199433]],
                    PARAMETER["Longitude of natural origin",147,
                        ANGLEUNIT["degree",0.0174532925199433]],
                    PARAMETER["Scale factor at natural origin",0.9996,
                        SCALEUNIT["unity",1]],
                    PARAMETER["False easting",500000,
                        LENGTHUNIT["metre",1]],
                    PARAMETER["False northing",10000000,
                        LENGTHUNIT["metre",1]]],
                CS[Cartesian,2],
                    AXIS["(E)",east,
                        ORDER[1],
                        LENGTHUNIT["metre",1]],
                    AXIS["(N)",north,
                        ORDER[2],
                        LENGTHUNIT["metre",1]],
                USAGE[
                    SCOPE["Engineering survey, topographic mapping."],
                    AREA["Australia - onshore and offshore between 144°E and 150°E. Papua New Guinea (PNG) - onshore between 144°E and 150°E."],
                    BBOX[-47.2,144,-1.3,150.01]]]],
        TARGETCRS[
            GEOGCRS["GDA2020",
                DATUM["Geocentric Datum of Australia 2020",
                    ELLIPSOID["GRS 1980",6378137,298.257222101,
                        LENGTHUNIT["metre",1]]],
                PRIMEM["Greenwich",0,
                    ANGLEUNIT["degree",0.0174532925199433]],
                CS[ellipsoidal,2],
                    AXIS["geodetic latitude (Lat)",north,
                        ORDER[1],
                        ANGLEUNIT["degree",0.0174532925199433]],
                    AXIS["geodetic longitude (Lon)",east,
                        ORDER[2],
                        ANGLEUNIT["degree",0.0174532925199433]]]],
        ABRIDGEDTRANSFORMATION["User-defined Bursa-Wolf Transform",
            METHOD["Position Vector transformation (geog2D domain)"],
            PARAMETER["X-axis translation",-153.501992013804,
                ID["EPSG",8605]],
            PARAMETER["Y-axis translation",91.8979603422544,
                ID["EPSG",8606]],
            PARAMETER["Z-axis translation",-76.7203604254192,
                ID["EPSG",8607]],
            PARAMETER["X-axis rotation",-11.9539464931016,
                ID["EPSG",8608]],
            PARAMETER["Y-axis rotation",13.9315768664084,
                ID["EPSG",8609]],
            PARAMETER["Z-axis rotation",-3.45993996519333,
                ID["EPSG",8610]],
            PARAMETER["Scale difference",0.999965937122937,
                ID["EPSG",8611]]]],
    VERTCRS["AHD height",
        VDATUM["Australian Height Datum"],
        CS[vertical,1],
            AXIS["gravity-related height (H)",up,
                LENGTHUNIT["metre",1]],
        USAGE[
            SCOPE["Cadastre, engineering surveying applications over distances up to 10km."],
            AREA["Australia - Australian Capital Territory, New South Wales, Northern Territory, Queensland, South Australia, Tasmania, Western Australia and Victoria - onshore. Christmas Island - onshore. Cocos and Keeling Islands - onshore."],
            BBOX[-43.7,96.76,-9.86,153.69]],
        ID["EPSG",5711]]]"#;

/// Source of `TEST(operation, compoundCRS_with_horizontal_boundCRS_to_compoundCRS_with_derived_vertical)`:
/// a bound horizontal part, and a derived vertical part (`PROJ affine`, zoff 1.055) over AHD height.
const HBOUND_DERIVED_VERTICAL_SRC: &str = r#"COMPOUNDCRS["AGD84 / AMG zone 55 + AHD height + local offset",
    BOUNDCRS[
        SOURCECRS[
            PROJCRS["AGD84 / AMG zone 55",
                BASEGEOGCRS["AGD84",
                    DATUM["User datum (no grid)",
                        ELLIPSOID["Australian National Spheroid",6378160,298.25,
                            LENGTHUNIT["metre",1]]],
                    PRIMEM["Greenwich",0,
                        ANGLEUNIT["degree",0.0174532925199433]]],
                CONVERSION["Australian Map Grid zone 55",
                    METHOD["Transverse Mercator"],
                    PARAMETER["Latitude of natural origin",0,
                        ANGLEUNIT["degree",0.0174532925199433]],
                    PARAMETER["Longitude of natural origin",147,
                        ANGLEUNIT["degree",0.0174532925199433]],
                    PARAMETER["Scale factor at natural origin",0.9996,
                        SCALEUNIT["unity",1]],
                    PARAMETER["False easting",500000,
                        LENGTHUNIT["metre",1]],
                    PARAMETER["False northing",10000000,
                        LENGTHUNIT["metre",1]]],
                CS[Cartesian,2],
                    AXIS["(E)",east,
                        ORDER[1],
                        LENGTHUNIT["metre",1]],
                    AXIS["(N)",north,
                        ORDER[2],
                        LENGTHUNIT["metre",1]]]],
        TARGETCRS[
            GEOGCRS["GDA2020",
                DATUM["Geocentric Datum of Australia 2020",
                    ELLIPSOID["GRS 1980",6378137,298.257222101,
                        LENGTHUNIT["metre",1]]],
                PRIMEM["Greenwich",0,
                    ANGLEUNIT["degree",0.0174532925199433]],
                CS[ellipsoidal,2],
                    AXIS["geodetic latitude (Lat)",north,
                        ORDER[1],
                        ANGLEUNIT["degree",0.0174532925199433]],
                    AXIS["geodetic longitude (Lon)",east,
                        ORDER[2],
                        ANGLEUNIT["degree",0.0174532925199433]]]],
        ABRIDGEDTRANSFORMATION["User-defined Bursa-Wolf Transform",
            METHOD["Position Vector transformation (geog2D domain)"],
            PARAMETER["X-axis translation",-153.306840928477,
                ID["EPSG",8605]],
            PARAMETER["Y-axis translation",92.4132660160741,
                ID["EPSG",8606]],
            PARAMETER["Z-axis translation",-76.481435572882,
                ID["EPSG",8607]],
            PARAMETER["X-axis rotation",-11.9644937558611,
                ID["EPSG",8608]],
            PARAMETER["Y-axis rotation",13.9284887447329,
                ID["EPSG",8609]],
            PARAMETER["Z-axis rotation",-3.44390727325759,
                ID["EPSG",8610]],
            PARAMETER["Scale difference",0.999965772441661,
                ID["EPSG",8611]]]],
    VERTCRS["AHD height + local offset",
        BASEVERTCRS["AHD height",
            VDATUM["Australian Height Datum"]],
        DERIVINGCONVERSION["vertical offset",
            METHOD["PROJ affine"],
            PARAMETER["zoff",1.055,
                LENGTHUNIT["metre",1]]],
        CS[vertical,1],
            AXIS["gravity-related height (H)",up,
                LENGTHUNIT["metre",1]]]]"#;

/// `TEST(operation, compoundCRS_with_horizontal_boundCRS_to_compoundCRS_with_derived_vertical)`:
/// both directions, against the PROJ 9.9.0 strings of that test.
#[test]
fn operation_compound_with_horizontal_bound_crs_to_compound_with_derived_vertical() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = wkt_crs(&db, HBOUND_DERIVED_VERTICAL_SRC);
    // GDA2020 / MGA zone 55 + AHD height
    let dst = epsg(&db, "7855+5711");
    let ctxt = ctx_partial(&db, "");
    let list = operations(&src, &dst, &ctxt);
    assert!(!list.is_empty());
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline \
         +step +inv +proj=utm +zone=55 +south +ellps=aust_SA \
         +step +proj=push +v_3 \
         +step +proj=cart +ellps=aust_SA \
         +step +proj=helmert +x=-153.306840928477 +y=92.4132660160741 \
         +z=-76.481435572882 +rx=-11.9644937558611 +ry=13.9284887447329 \
         +rz=-3.44390727325759 +s=-34.2275583390395 \
         +convention=position_vector \
         +step +inv +proj=cart +ellps=GRS80 \
         +step +proj=pop +v_3 \
         +step +inv +proj=affine +zoff=1.055 \
         +step +proj=utm +zone=55 +south +ellps=GRS80"
    );
    let list = operations(&dst, &src, &ctxt);
    assert!(!list.is_empty());
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline \
         +step +inv +proj=utm +zone=55 +south +ellps=GRS80 \
         +step +proj=affine +zoff=1.055 \
         +step +proj=push +v_3 \
         +step +proj=cart +ellps=GRS80 \
         +step +inv +proj=helmert +x=-153.306840928477 \
         +y=92.4132660160741 +z=-76.481435572882 \
         +rx=-11.9644937558611 +ry=13.9284887447329 \
         +rz=-3.44390727325759 +s=-34.2275583390395 \
         +convention=position_vector \
         +step +inv +proj=cart +ellps=aust_SA \
         +step +proj=pop +v_3 \
         +step +proj=utm +zone=55 +south +ellps=aust_SA"
    );
}

/// `TEST(operation, compoundCRS_with_horizontal_boundCRS_to_compoundCRS_and_same_vertical)`
#[test]
fn operation_compound_with_horizontal_bound_crs_to_compound_and_same_vertical() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = wkt_crs(&db, HBOUND_SRC);
    // GDA2020 / MGA zone 55 + AHD height
    let dst = epsg(&db, "7855+5711");
    let ctxt = ctx_partial(&db, "");
    let list = operations(&src, &dst, &ctxt);
    assert!(!list.is_empty());
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline \
         +step +inv +proj=utm +zone=55 +south +ellps=aust_SA \
         +step +proj=push +v_3 \
         +step +proj=cart +ellps=aust_SA \
         +step +proj=helmert +x=-153.501992013804 +y=91.8979603422544 \
         +z=-76.7203604254192 +rx=-11.9539464931016 +ry=13.9315768664084 \
         +rz=-3.45993996519333 +s=-34.0628770629792 \
         +convention=position_vector \
         +step +inv +proj=cart +ellps=GRS80 \
         +step +proj=pop +v_3 \
         +step +proj=utm +zone=55 +south +ellps=GRS80"
    );
}
