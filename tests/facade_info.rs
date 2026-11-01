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
fn a_definition_the_crate_cannot_read_is_an_error_not_text() {
    // libproj reads these; this crate does not yet (single-step pipeline, `cart`,
    // `+init=`). The facade must refuse them, not hand back the input.
    for input in [
        "+proj=longlat +datum=GRS80",
        "+proj=pipeline +step +proj=longlat +datum=NAD27",
        "+proj=cart +ellps=WGS84",
        "+init=epsg:4326",
    ] {
        let proj = Proj::new(input).expect(input);
        assert!(
            matches!(proj.def(), Err(ProjError::Definition)),
            "{input} gave {:?}",
            proj.def()
        );
    }
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
