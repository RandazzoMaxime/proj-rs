//! Definition, `id`, and area of use of the georust facade, against libproj.
//!
//! The expected strings are libproj's `proj_pj_info().definition` and
//! `proj_get_area_of_use`, read through pyproj 3.8.0, which bundles PROJ 9.8.1.
//! PROJ 9.9.0 is the reference of this crate; the strings did not change between
//! the two for these inputs. See `docs/PORTING-facade-info.md`.

use proj_rs::{Area, Proj, ProjError};

#[test]
fn definitions_follow_libproj() {
    let cases = [
        (
            "+proj=longlat +datum=WGS84 +no_defs",
            "proj=longlat datum=WGS84 no_defs ellps=WGS84 towgs84=0,0,0",
        ),
        (
            "+proj=longlat",
            "proj=longlat datum=WGS84 no_defs ellps=WGS84 towgs84=0,0,0",
        ),
        (
            "+proj=longlat +ellps=GRS80 +no_defs",
            "proj=longlat ellps=GRS80 no_defs",
        ),
        (
            "+proj=longlat +a=6378137 +b=6356752.314 +no_defs",
            "proj=longlat a=6378137 b=6356752.314 no_defs",
        ),
        (
            "+proj=longlat +datum=WGS84 +towgs84=1,2,3",
            "proj=longlat ellps=WGS84 towgs84=1,2,3,0,0,0,0 no_defs",
        ),
        (
            "+proj=longlat +datum=potsdam",
            "proj=longlat ellps=bessel no_defs",
        ),
        (
            "+proj=longlat +datum=NAD27 +no_defs",
            "proj=longlat datum=NAD27 no_defs ellps=clrk66 nadgrids=@conus,@alaska,@ntv2_0.gsb,@ntv1_can.dat",
        ),
        (
            "+proj=longlat +datum=ire65",
            "proj=longlat a=6377340.189 rf=299.3249646 no_defs",
        ),
        (
            "+proj=utm +zone=32",
            "proj=utm zone=32 datum=WGS84 units=m no_defs ellps=WGS84 towgs84=0,0,0",
        ),
        (
            "+proj=utm +zone=32 +datum=NAD83",
            "proj=utm zone=32 datum=NAD83 units=m no_defs ellps=GRS80 towgs84=0,0,0",
        ),
        (
            "+proj=utm +zone=31 +south +datum=WGS84",
            "proj=utm zone=31 south datum=WGS84 units=m no_defs ellps=WGS84 towgs84=0,0,0",
        ),
        (
            "+proj=tmerc +lat_0=0 +lon_0=9 +k=0.9996 +x_0=500000 +y_0=0 +ellps=GRS80 +units=m +no_defs",
            "proj=utm zone=32 ellps=GRS80 units=m no_defs",
        ),
        (
            "+proj=merc +a=6378137 +b=6378137 +lat_ts=0 +lon_0=0 +x_0=0 +y_0=0 +k=1 +units=m +nadgrids=@null +wktext +no_defs",
            "proj=merc a=6378137 b=6378137 lat_ts=0 lon_0=0 x_0=0 y_0=0 k=1 units=m nadgrids=@null no_defs",
        ),
        (
            "+proj=lcc +lat_1=49 +lat_2=44 +lat_0=46.5 +lon_0=3 +x_0=700000 +y_0=6600000 +ellps=GRS80 +units=m +no_defs",
            "proj=lcc lat_0=46.5 lon_0=3 lat_1=49 lat_2=44 x_0=700000 y_0=6600000 ellps=GRS80 units=m no_defs",
        ),
    ];
    for (input, expected) in cases {
        let proj = Proj::new(input).expect(input);
        assert_eq!(proj.def().expect(input), expected, "{input}");
    }
}

#[test]
fn a_definition_proj_does_not_name_is_an_error() {
    // PROJ 9.8.1 `proj_create` of this datum is error 1027. The facade refuses
    // it instead of returning the input.
    let proj = Proj::new("+proj=longlat +datum=GRS80").expect("created");
    assert!(matches!(proj.def(), Err(ProjError::Definition)));
}

#[test]
fn pipeline_cart_and_init_definitions_follow_proj_pj_info() {
    // `proj_pj_info().definition` from PROJ 9.8.1 (pyproj 3.8.0's libproj),
    // `proj_create` not the CRS (`+type=crs`) reading. A one-step pipeline is
    // the step, with `pj_datum_set` and the default ellipsoid. `cart` stays
    // `cart`. `+init=` is rejected by that `proj_create`. The string below is
    // `proj_pj_info` of the expansion, whose inverse flattening is the database
    // value printed at 15 digits.
    let cases = [
        (
            "+proj=pipeline +step +proj=longlat +datum=NAD27",
            "proj=longlat datum=NAD27 ellps=clrk66 nadgrids=@conus,@alaska,@ntv2_0.gsb,@ntv1_can.dat",
        ),
        (
            "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad",
            "proj=unitconvert xy_in=deg xy_out=rad ellps=GRS80",
        ),
        (
            "+proj=pipeline +step +proj=longlat +ellps=GRS80 +step +proj=utm +zone=31",
            "proj=pipeline step proj=longlat ellps=GRS80 step proj=utm zone=31",
        ),
        (
            "+proj=pipeline +step +proj=cart +ellps=WGS84",
            "proj=cart ellps=WGS84",
        ),
        ("+proj=cart +ellps=WGS84", "proj=cart ellps=WGS84"),
        (
            "+init=epsg:4326",
            // `format_f64` prints the database `inv_flattening` at 15 digits.
            // PROJ 9.8.1 `proj_pj_info` of that expansion keeps the digits.
            "proj=longlat a=6378137.0 rf=298.257223563000025 no_defs",
        ),
    ];
    for (input, expected) in cases {
        let proj = Proj::new(input).expect(input);
        assert_eq!(proj.def().expect(input), expected, "{input}");
        let info = proj.proj_info();
        assert_eq!(info.definition.as_deref(), Some(expected), "{input}");
    }
}

#[test]
fn a_single_crs_has_no_operation_definition() {
    // PROJ 9.8.1 `proj_pj_info` of `proj_create("EPSG:4326")` has a null definition.
    let crs = Proj::new("EPSG:4326").expect("crs");
    assert!(matches!(crs.def(), Err(ProjError::Definition)));
    assert!(crs.proj_info().definition.is_none());
}

#[test]
fn pair_definitions_follow_proj_pj_info() {
    // PROJ 9.8.1 `proj_create_crs_to_crs` + `proj_pj_info`.
    let one = Proj::new_known_crs("EPSG:4326", "EPSG:4326", None).expect("noop");
    assert_eq!(one.def().expect("noop"), "proj=noop ellps=GRS80");
    assert_eq!(one.proj_info().id.as_deref(), Some("noop"));
    // georust `new_known_crs` calls `proj_normalize_for_visualization`. PROJ 9.8.1
    // then drops the authority `axisswap` and keeps this pipeline.
    let web = Proj::new_known_crs("EPSG:4326", "EPSG:3857", None).expect("webmerc");
    assert_eq!(
        web.def().expect("webmerc"),
        "proj=pipeline step proj=unitconvert xy_in=deg xy_out=rad step proj=webmerc lat_0=0 lon_0=0 x_0=0 y_0=0 ellps=WGS84"
    );
    let many = Proj::new_known_crs("EPSG:4269", "EPSG:4326", None).expect("nad83");
    assert_eq!(
        many.def().expect("several operations"),
        "unavailable until proj_trans is called"
    );
    assert_eq!(many.proj_info().id.as_deref(), Some("unknown"));
}

#[test]
fn proj_info_of_a_proj_string_names_its_operation() {
    let proj = Proj::new(
        "+proj=tmerc +lat_0=0 +lon_0=9 +k=0.9996 +x_0=500000 +y_0=0 +ellps=GRS80 +units=m +no_defs",
    )
    .expect("tmerc");
    let info = proj.proj_info();
    assert_eq!(info.id.as_deref(), Some("utm"));
    assert_eq!(
        info.description.as_deref(),
        Some("PROJ-based coordinate operation")
    );
    assert_eq!(
        info.definition.as_deref(),
        Some("proj=utm zone=32 ellps=GRS80 units=m no_defs")
    );
    assert_eq!(info.accuracy, -1.0);
}

#[test]
fn area_of_use_is_the_first_usage_of_a_crs() {
    let proj = Proj::new("EPSG:4326").expect("EPSG:4326");
    let (area, name) = proj.area_of_use().expect("area");
    assert_eq!(area, Some(Area::new(-180.0, -90.0, 180.0, 90.0)));
    assert_eq!(name.as_deref(), Some("World."));
}

#[test]
fn area_of_use_of_a_proj_string_is_still_an_error() {
    let proj = Proj::new("+proj=longlat +datum=WGS84 +no_defs").expect("longlat");
    assert!(matches!(
        proj.area_of_use(),
        Err(ProjError::UnknownAreaOfUse)
    ));
}
