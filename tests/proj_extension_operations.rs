//! A CRS with an `EXTENSION_PROJ4` (`+lon_wrap`, `+lon_0` or `+over` on a geographic CRS) is
//! transformed by one PROJ-based operation: the inverse of the source's legacy CRS export, then
//! the target's (`createOperationsFromProj4Ext`). That string is the operation's PROJ string
//! (`PROJBasedOperation::_exportToPROJString`). Expected values: `proj_as_proj_string` of
//! `proj_create_crs_to_crs_from_pj` in C PROJ 9.9.0.

use proj_rs::iso19111::create_from_user_input;
use proj_rs::{CoordinateOperationContext, CoordinateOperationFactory, DatabaseContext, ProjLines};

#[test]
fn proj_extension_operation_keeps_its_string() {
    let db = DatabaseContext::create().expect("proj.db");
    let cases: &[(&str, &str, &str, &str)] = &[
        (
            "into a +lon_wrap geographic CRS (GDAL test_osr_ct_lon_wrap)",
            r#"+proj=longlat +ellps=GRS80 +type=crs"#,
            r#"+proj=longlat +ellps=GRS80 +lon_wrap=180 +type=crs"#,
            r#"+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=longlat +ellps=GRS80 +lon_wrap=180 +step +proj=unitconvert +xy_in=rad +xy_out=deg"#,
        ),
        (
            "out of a +lon_wrap geographic CRS",
            r#"+proj=longlat +ellps=GRS80 +lon_wrap=180 +type=crs"#,
            r#"+proj=longlat +ellps=GRS80 +type=crs"#,
            r#"+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +inv +proj=longlat +ellps=GRS80 +lon_wrap=180 +step +proj=unitconvert +xy_in=rad +xy_out=deg"#,
        ),
        (
            "between two +lon_wrap values",
            r#"+proj=longlat +ellps=GRS80 +lon_wrap=180 +type=crs"#,
            r#"+proj=longlat +ellps=GRS80 +lon_wrap=0 +type=crs"#,
            r#"+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +inv +proj=longlat +ellps=GRS80 +lon_wrap=180 +step +proj=longlat +ellps=GRS80 +lon_wrap=0 +step +proj=unitconvert +xy_in=rad +xy_out=deg"#,
        ),
        (
            "into a +lon_0 geographic CRS",
            r#"+proj=longlat +datum=WGS84 +type=crs"#,
            r#"+proj=longlat +datum=WGS84 +lon_0=10 +type=crs"#,
            r#"+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=longlat +datum=WGS84 +lon_0=10 +step +proj=unitconvert +xy_in=rad +xy_out=deg"#,
        ),
        (
            "into a +over geographic CRS",
            r#"+proj=longlat +datum=WGS84 +type=crs"#,
            r#"+proj=longlat +datum=WGS84 +over +type=crs"#,
            r#"+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=longlat +datum=WGS84 +over +step +proj=unitconvert +xy_in=rad +xy_out=deg"#,
        ),
        (
            "+lon_wrap geographic to UTM",
            r#"+proj=longlat +datum=WGS84 +lon_wrap=180 +type=crs"#,
            r#"+proj=utm +zone=31 +datum=WGS84 +type=crs"#,
            r#"+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +inv +proj=longlat +datum=WGS84 +lon_wrap=180 +step +proj=utm +zone=31 +datum=WGS84"#,
        ),
        (
            "Paris meridian to +lon_wrap",
            r#"+proj=longlat +ellps=GRS80 +pm=paris +type=crs"#,
            r#"+proj=longlat +ellps=GRS80 +lon_wrap=180 +type=crs"#,
            r#"+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +inv +proj=longlat +ellps=GRS80 +pm=paris +step +proj=longlat +ellps=GRS80 +lon_wrap=180 +step +proj=unitconvert +xy_in=rad +xy_out=deg"#,
        ),
        (
            "geocentric to +lon_wrap",
            r#"+proj=geocent +ellps=GRS80 +type=crs"#,
            r#"+proj=longlat +ellps=GRS80 +lon_wrap=180 +type=crs"#,
            r#"+proj=pipeline +step +inv +proj=geocent +ellps=GRS80 +units=m +step +proj=longlat +ellps=GRS80 +lon_wrap=180 +step +proj=unitconvert +xy_in=rad +xy_out=deg"#,
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
