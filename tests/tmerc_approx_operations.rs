//! `+approx` on a transverse Mercator CRS. PROJ maps it to the `tmerc approx` method override for
//! `tmerc` only: `createUTM` reads no `+approx`, so a UTM CRS keeps its string as
//! `EXTENSION_PROJ4`, and `etmerc` drops it. The operations between such CRS keep `approx`.
//! Expected values: `proj_as_proj_string` of C PROJ 9.9.0, of the CRS and of
//! `proj_create_crs_to_crs_from_pj`.

use proj_rs::iso19111::create_from_user_input;
use proj_rs::{CoordinateOperationContext, CoordinateOperationFactory, DatabaseContext, ProjLines};

#[test]
fn approx_crs_keep_their_proj_string() {
    let db = DatabaseContext::create().expect("proj.db");
    let cases: &[(&str, &str)] = &[
        (
            r#"+proj=tmerc +ellps=GRS80 +approx +type=crs"#,
            r#"+proj=tmerc +approx +lat_0=0 +lon_0=0 +k=1 +x_0=0 +y_0=0 +ellps=GRS80 +units=m +no_defs +type=crs"#,
        ),
        (
            r#"+proj=utm +zone=31 +datum=WGS84 +approx +type=crs"#,
            r#"+proj=utm +zone=31 +datum=WGS84 +approx +type=crs"#,
        ),
        (
            r#"+proj=utm +zone=31 +south +ellps=GRS80 +approx +type=crs"#,
            r#"+proj=utm +zone=31 +south +ellps=GRS80 +approx +type=crs"#,
        ),
        (
            r#"+proj=etmerc +lon_0=3 +ellps=GRS80 +approx +type=crs"#,
            r#"+proj=tmerc +lat_0=0 +lon_0=3 +k=1 +x_0=0 +y_0=0 +ellps=GRS80 +units=m +no_defs +type=crs"#,
        ),
    ];
    for (text, expected) in cases {
        let crs = create_from_user_input(text, &db).expect(text);
        assert_eq!(
            crs.export_to_proj_string().expect("PROJ string"),
            *expected,
            "{text}"
        );
    }
}

#[test]
fn operations_keep_approx() {
    let db = DatabaseContext::create().expect("proj.db");
    let cases: &[(&str, &str, &str, &str)] = &[
        (
            "into a +approx tmerc CRS",
            r#"+proj=longlat +ellps=GRS80 +type=crs"#,
            r#"+proj=tmerc +ellps=GRS80 +approx +type=crs"#,
            r#"+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=tmerc +approx +lat_0=0 +lon_0=0 +k=1 +x_0=0 +y_0=0 +ellps=GRS80"#,
        ),
        (
            "out of a +approx tmerc CRS",
            r#"+proj=tmerc +ellps=GRS80 +approx +type=crs"#,
            r#"+proj=longlat +ellps=GRS80 +type=crs"#,
            r#"+proj=pipeline +step +inv +proj=tmerc +approx +lat_0=0 +lon_0=0 +k=1 +x_0=0 +y_0=0 +ellps=GRS80 +step +proj=unitconvert +xy_in=rad +xy_out=deg"#,
        ),
        (
            "into a +approx UTM CRS",
            r#"EPSG:4326"#,
            r#"+proj=utm +zone=31 +datum=WGS84 +approx +type=crs"#,
            r#"+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=utm +zone=31 +datum=WGS84 +approx"#,
        ),
        (
            "out of a +approx UTM CRS",
            r#"+proj=utm +zone=31 +datum=WGS84 +approx +type=crs"#,
            r#"EPSG:4326"#,
            r#"+proj=pipeline +step +inv +proj=utm +zone=31 +datum=WGS84 +approx +step +proj=unitconvert +xy_in=rad +xy_out=deg +step +proj=axisswap +order=2,1"#,
        ),
        (
            "+approx tmerc to exact tmerc",
            r#"+proj=tmerc +lon_0=3 +ellps=GRS80 +approx +type=crs"#,
            r#"+proj=tmerc +lon_0=9 +ellps=GRS80 +type=crs"#,
            r#"+proj=pipeline +step +inv +proj=tmerc +approx +lat_0=0 +lon_0=3 +k=1 +x_0=0 +y_0=0 +ellps=GRS80 +step +proj=tmerc +lat_0=0 +lon_0=9 +k=1 +x_0=0 +y_0=0 +ellps=GRS80"#,
        ),
    ];
    for (label, source, target, expected) in cases {
        let source = create_from_user_input(source, &db).expect("source");
        let target = create_from_user_input(target, &db).expect("target");
        let context = CoordinateOperationContext::create(&db, "");
        let operations = CoordinateOperationFactory::create()
            .create_operations(&source, &target, &context)
            .expect("operations");
        let first = operations.first().expect("one operation");
        let text = first
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string");
        assert_eq!(text, *expected, "{label}");
    }
}
