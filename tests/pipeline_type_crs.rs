//! A `+proj=pipeline` string with `+type=crs` whose steps have no CRS shape is a PROJ-based
//! operation (io.cpp:12924-12974), and `proj_pj_info` keeps an inverted single step as a
//! pipeline (io.cpp:10045-10050). Expected values: PROJ 9.9.0 `proj_create` and
//! `proj_pj_info`, as GRASS 8.5 `r.latlong` reaches them.

use proj_rs::iso19111::proj_based_operation_of_text;
use proj_rs::Proj;

const GRASS: &str = "+proj=pipeline +step +inv +proj=lcc +lat_0=33.75 +lon_0=-79 \
+lat_1=36.1666666666667 +lat_2=34.3333333333333 +x_0=609601.22 +y_0=0 +no_defs +a=6378137 \
+rf=298.257222101 +towgs84=0.000,0.000,0.000 +type=crs";

fn definition(text: &str) -> String {
    Proj::new(text)
        .expect("PROJ instantiates the string")
        .proj_info()
        .definition
        .expect("PROJ reports a definition")
}

#[test]
fn inverted_pipeline_with_type_crs_is_an_operation() {
    assert!(matches!(proj_based_operation_of_text(GRASS), Some(Ok(_))));
    assert_eq!(
        definition(GRASS),
        "proj=pipeline step inv proj=lcc lat_0=33.75 lon_0=-79 lat_1=36.1666666666667 \
lat_2=34.3333333333333 x_0=609601.22 y_0=0 no_defs a=6378137 rf=298.257222101 \
towgs84=0.000,0.000,0.000 type=crs"
    );
}

#[test]
fn crs_shaped_pipeline_stays_a_crs() {
    let projected = "+proj=pipeline +type=crs +step +proj=longlat +ellps=GRS80 \
+step +proj=utm +zone=31";
    assert!(proj_based_operation_of_text(projected).is_none());
    let operation = "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad \
+step +proj=lcc +lat_1=36 +lat_2=34 +ellps=GRS80 +type=crs";
    assert!(matches!(
        proj_based_operation_of_text(operation),
        Some(Ok(_))
    ));
}

#[test]
fn single_step_definition_follows_the_formatter() {
    let cases = [
        (
            "+proj=pipeline +step +inv +proj=lcc +lat_1=36 +lat_2=34 +ellps=GRS80",
            "proj=pipeline step inv proj=lcc lat_1=36 lat_2=34 ellps=GRS80",
        ),
        (
            "+proj=pipeline +type=crs +step +inv +proj=lcc +lat_1=36 +lat_2=34 +ellps=GRS80",
            "proj=pipeline type=crs step inv proj=lcc lat_1=36 lat_2=34 ellps=GRS80",
        ),
        (
            "+proj=pipeline +step +proj=axisswap +order=2,1 +type=crs",
            "proj=axisswap order=2,1 ellps=GRS80",
        ),
        (
            "+proj=pipeline +ellps=GRS80 +step +proj=axisswap +order=2,1",
            "proj=pipeline ellps=GRS80 step proj=axisswap order=2,1",
        ),
        (
            "+proj=pipeline +step +proj=axisswap +order=2,1 +omit_inv",
            "proj=pipeline step proj=axisswap order=2,1 omit_inv",
        ),
        (
            "+proj=pipeline +step +proj=utm +zone=31 +datum=WGS84",
            "proj=utm zone=31 datum=WGS84 ellps=WGS84 towgs84=0,0,0",
        ),
    ];
    for (text, expected) in cases {
        assert_eq!(definition(text), expected, "{text}");
    }
}
