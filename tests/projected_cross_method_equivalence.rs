//! Projected CRS whose conversions use two forms of one projection: Mercator variant A and B,
//! LCC 1SP and 2SP. `DerivedCRS::_isEquivalentTo` compares the conversion with the standard
//! criterion on the base CRS, so `SingleOperation::_isEquivalentTo` meets the two forms through
//! `convertToOtherMethod`. The WKT come from GDAL's `test_osr_basic_21` to `_23`. Each expected
//! triple is `STRICT`, `EQUIVALENT` with a database and `EQUIVALENT_EXCEPT_AXIS_ORDER_GEOGCRS`, as
//! `proj_is_equivalent_to` of C PROJ 9.9.0 answers it.

use proj_rs::iso19111::create_from_user_input;
use proj_rs::{DatabaseContext, Equivalence};

#[test]
fn one_projection_in_two_forms_is_equivalent() {
    let db = DatabaseContext::create().expect("proj.db");
    let pairs: &[(&str, &str, &str)] = &[
        (CRS_1, CRS_2, "011"),
        (CRS_1, CRS_3, "000"),
        (CRS_2, CRS_3, "000"),
        (CRS_4, CRS_8, "011"),
        (CRS_5, CRS_9, "011"),
        (CRS_6, CRS_7, "011"),
        (CRS_6, CRS_10, "011"),
        (CRS_11, CRS_12, "011"),
        (CRS_4, CRS_5, "000"),
    ];
    for (index, (left, right, expected)) in pairs.iter().enumerate() {
        let left = create_from_user_input(left, &db).expect("left");
        let right = create_from_user_input(right, &db).expect("right");
        let flags = [
            left.is_equivalent_to_with(&right, Equivalence::Strict),
            left.is_equivalent_to_db(&right, &db),
            left.is_equivalent_to_with(&right, Equivalence::ExceptAxisOrder),
        ];
        let got: String = flags
            .iter()
            .map(|flag| if *flag { '1' } else { '0' })
            .collect();
        assert_eq!(got, *expected, "pair {index}");
    }
}

const CRS_1: &str = r#"PROJCS["unnamed", GEOGCS["Segara (Jakarta)", DATUM["Gunung_Segara_Jakarta", SPHEROID["Bessel 1841",6377397.155,299.1528128]], PRIMEM["Jakarta",106.8077194444444], UNIT["degree",0.0174532925199433]], PROJECTION["Mercator_2SP"], PARAMETER["central_meridian",110], PARAMETER["false_easting",3900000], PARAMETER["false_northing",900000], PARAMETER["standard_parallel_1",4.45405154589751], UNIT["metre",1], AXIS["Easting",EAST], AXIS["Northing",NORTH]]"#;

const CRS_2: &str = r#"PROJCS["unnamed", GEOGCS["Segara (Jakarta)", DATUM["Gunung_Segara_Jakarta", SPHEROID["Bessel 1841",6377397.155,299.1528128]], PRIMEM["Jakarta",106.8077194444444], UNIT["degree",0.0174532925199433]], PROJECTION["Mercator_1SP"], PARAMETER["central_meridian",110], PARAMETER["scale_factor",0.997], PARAMETER["false_easting",3900000], PARAMETER["false_northing",900000], UNIT["metre",1], AXIS["Easting",EAST], AXIS["Northing",NORTH]]"#;

const CRS_3: &str = r#"PROJCS["unnamed", GEOGCS["Segara (Jakarta)", DATUM["Gunung_Segara_Jakarta", SPHEROID["Bessel 1841",6377397.155,299.1528128]], PRIMEM["Jakarta",106.8077194444444], UNIT["degree",0.0174532925199433]], PROJECTION["Mercator_1SP"], PARAMETER["central_meridian",110], PARAMETER["scale_factor",0.998], PARAMETER["false_easting",3900000], PARAMETER["false_northing",900000], UNIT["metre",1], AXIS["Easting",EAST], AXIS["Northing",NORTH]]"#;

const CRS_4: &str = r#"PROJCS["unnamed", GEOGCS["RGF93", DATUM["Reseau_Geodesique_Francais_1993", SPHEROID["GRS 1980",6378137,298.257222101, AUTHORITY["EPSG","7019"]], TOWGS84[0,0,0,0,0,0,0], AUTHORITY["EPSG","6171"]], PRIMEM["Greenwich",0, AUTHORITY["EPSG","8901"]], UNIT["degree",0.0174532925199433, AUTHORITY["EPSG","9122"]], AUTHORITY["EPSG","4171"]], PROJECTION["Lambert_Conformal_Conic_1SP"], PARAMETER["latitude_of_origin",46.5194302239868], PARAMETER["central_meridian",3], PARAMETER["scale_factor",0.9990510286374693], PARAMETER["false_easting",700000], PARAMETER["false_northing",6602157.83881033], UNIT["metre",1], AXIS["Easting",EAST], AXIS["Northing",NORTH]]"#;

const CRS_5: &str = r#"PROJCS["unnamed", GEOGCS["RGF93", DATUM["Reseau_Geodesique_Francais_1993", SPHEROID["GRS 1980",6378137,298.257222101, AUTHORITY["EPSG","7019"]], TOWGS84[0,0,0,0,0,0,0], AUTHORITY["EPSG","6171"]], PRIMEM["Greenwich",0, AUTHORITY["EPSG","8901"]], UNIT["degree",0.0174532925199433, AUTHORITY["EPSG","9122"]], AUTHORITY["EPSG","4171"]], PROJECTION["Lambert_Conformal_Conic_1SP"], PARAMETER["latitude_of_origin",46.5], PARAMETER["central_meridian",3], PARAMETER["scale_factor",1], PARAMETER["false_easting",700000], PARAMETER["false_northing",6600000], UNIT["metre",1], AXIS["Easting",EAST], AXIS["Northing",NORTH]]"#;

const CRS_6: &str = r#"PROJCS["unnamed", GEOGCS["RGF93", DATUM["Reseau_Geodesique_Francais_1993", SPHEROID["GRS 1980",6378137,298.257222101, AUTHORITY["EPSG","7019"]], TOWGS84[0,0,0,0,0,0,0], AUTHORITY["EPSG","6171"]], PRIMEM["Greenwich",0, AUTHORITY["EPSG","8901"]], UNIT["degree",0.0174532925199433, AUTHORITY["EPSG","9122"]], AUTHORITY["EPSG","4171"]], PROJECTION["Lambert_Conformal_Conic_1SP"], PARAMETER["latitude_of_origin",46.4567], PARAMETER["central_meridian",3], PARAMETER["scale_factor",1], PARAMETER["false_easting",700000], PARAMETER["false_northing",6637093.292952879], UNIT["metre",1], AXIS["Easting",EAST], AXIS["Northing",NORTH]]"#;

const CRS_7: &str = r#"PROJCS["unnamed", GEOGCS["RGF93", DATUM["Reseau_Geodesique_Francais_1993", SPHEROID["GRS 1980",6378137,298.257222101, AUTHORITY["EPSG","7019"]], TOWGS84[0,0,0,0,0,0,0], AUTHORITY["EPSG","6171"]], PRIMEM["Greenwich",0, AUTHORITY["EPSG","8901"]], UNIT["degree",0.0174532925199433, AUTHORITY["EPSG","9122"]], AUTHORITY["EPSG","4171"]], PROJECTION["Lambert_Conformal_Conic_2SP"], PARAMETER["standard_parallel_1",46.4567], PARAMETER["standard_parallel_2",46.4567], PARAMETER["latitude_of_origin",46.4567], PARAMETER["central_meridian",3], PARAMETER["false_easting",700000], PARAMETER["false_northing",6637093.292952879], UNIT["metre",1], AXIS["Easting",EAST], AXIS["Northing",NORTH]]"#;

const CRS_8: &str = r#"PROJCS["unnamed", GEOGCS["RGF93", DATUM["Reseau_Geodesique_Francais_1993", SPHEROID["GRS 1980",6378137,298.257222101, AUTHORITY["EPSG","7019"]], TOWGS84[0,0,0,0,0,0,0], AUTHORITY["EPSG","6171"]], PRIMEM["Greenwich",0, AUTHORITY["EPSG","8901"]], UNIT["degree",0.0174532925199433, AUTHORITY["EPSG","9122"]], AUTHORITY["EPSG","4171"]], PROJECTION["Lambert_Conformal_Conic_2SP"], PARAMETER["standard_parallel_1",49], PARAMETER["standard_parallel_2",44], PARAMETER["latitude_of_origin",46.5], PARAMETER["central_meridian",3], PARAMETER["false_easting",700000], PARAMETER["false_northing",6600000], UNIT["metre",1, AUTHORITY["EPSG","9001"]], AXIS["X",EAST], AXIS["Y",NORTH], AUTHORITY["EPSG","2154"]]"#;

const CRS_9: &str = r#"PROJCS["unnamed", GEOGCS["RGF93", DATUM["Reseau_Geodesique_Francais_1993", SPHEROID["GRS 1980",6378137,298.257222101, AUTHORITY["EPSG","7019"]], TOWGS84[0,0,0,0,0,0,0], AUTHORITY["EPSG","6171"]], PRIMEM["Greenwich",0, AUTHORITY["EPSG","8901"]], UNIT["degree",0.0174532925199433, AUTHORITY["EPSG","9122"]], AUTHORITY["EPSG","4171"]], PROJECTION["Lambert_Conformal_Conic_2SP"], PARAMETER["standard_parallel_1",46.5], PARAMETER["standard_parallel_2",46.5], PARAMETER["latitude_of_origin",46.5], PARAMETER["central_meridian",3], PARAMETER["false_easting",700000], PARAMETER["false_northing",6600000], UNIT["metre",1], AXIS["Easting",EAST], AXIS["Northing",NORTH]]"#;

const CRS_10: &str = r#"PROJCS["unnamed", GEOGCS["RGF93", DATUM["Reseau_Geodesique_Francais_1993", SPHEROID["GRS 1980",6378137,298.257222101, AUTHORITY["EPSG","7019"]], TOWGS84[0,0,0,0,0,0,0], AUTHORITY["EPSG","6171"]], PRIMEM["Greenwich",0, AUTHORITY["EPSG","8901"]], UNIT["degree",0.0174532925199433, AUTHORITY["EPSG","9122"]], AUTHORITY["EPSG","4171"]], PROJECTION["Lambert_Conformal_Conic_2SP"], PARAMETER["standard_parallel_1",46.4567], PARAMETER["standard_parallel_2",46.4567], PARAMETER["latitude_of_origin",46.123], PARAMETER["central_meridian",3], PARAMETER["false_easting",700000], PARAMETER["false_northing",6600000], UNIT["metre",1], AXIS["Easting",EAST], AXIS["Northing",NORTH]]"#;

const CRS_11: &str = r#"PROJCS["unnamed", GEOGCS["NTF (Paris)", DATUM["Nouvelle_Triangulation_Francaise_Paris", SPHEROID["Clarke 1880 (IGN)",6378249.2,293.4660212936269, AUTHORITY["EPSG","7011"]], TOWGS84[-168,-60,320,0,0,0,0], AUTHORITY["EPSG","6807"]], PRIMEM["Paris",2.33722917, AUTHORITY["EPSG","8903"]], UNIT["grad",0.01570796326794897, AUTHORITY["EPSG","9105"]], AUTHORITY["EPSG","4807"]], PROJECTION["Lambert_Conformal_Conic_2SP"], PARAMETER["standard_parallel_1",47.51962607709162], PARAMETER["standard_parallel_2",46.17820871246364], PARAMETER["latitude_of_origin",46.85], PARAMETER["central_meridian",0], PARAMETER["false_easting",234.358], PARAMETER["false_northing",4185861.369], UNIT["metre",1], AXIS["Easting",EAST], AXIS["Northing",NORTH]]"#;

const CRS_12: &str = r#"PROJCS["unnamed", GEOGCS["NTF (Paris)", DATUM["Nouvelle_Triangulation_Francaise_Paris", SPHEROID["Clarke 1880 (IGN)",6378249.2,293.4660212936269, AUTHORITY["EPSG","7011"]], TOWGS84[-168,-60,320,0,0,0,0], AUTHORITY["EPSG","6807"]], PRIMEM["Paris",2.33722917, AUTHORITY["EPSG","8903"]], UNIT["grad",0.01570796326794897, AUTHORITY["EPSG","9105"]], AUTHORITY["EPSG","4807"]], PROJECTION["Lambert_Conformal_Conic_1SP"], PARAMETER["latitude_of_origin",46.85], PARAMETER["central_meridian",0], PARAMETER["scale_factor",0.99994471], PARAMETER["false_easting",234.358], PARAMETER["false_northing",4185861.369], UNIT["metre",1, AUTHORITY["EPSG","9001"]], AXIS["X",EAST], AXIS["Y",NORTH], AUTHORITY["EPSG","27584"]]"#;
