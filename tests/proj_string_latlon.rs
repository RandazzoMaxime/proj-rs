//! `+proj=latlon` is one of the four names of the geographic `longlat` step in PROJ's operation
//! list, with `lonlat` and `latlong`: `createFromPROJString` builds a geographic CRS from it.
//! pyproj's `test_is_derived` reads `+proj=latlon`. Expected: C PROJ 9.9.0, `proj_get_type` is
//! `PJ_TYPE_GEOGRAPHIC_2D_CRS` for the four names.

use proj_rs::iso19111::{create_crs_from_proj_string, Crs, Equivalence};

#[test]
fn every_geographic_name_builds_a_geographic_crs() {
    let reference =
        create_crs_from_proj_string("+proj=longlat +ellps=GRS80 +type=crs").expect("longlat");
    for name in ["latlon", "lonlat", "latlong", "longlat"] {
        let crs = create_crs_from_proj_string(&format!("+proj={name} +ellps=GRS80 +type=crs"))
            .expect(name);
        assert!(matches!(crs, Crs::Geographic(_)), "{name} gives {crs:?}");
        assert!(
            crs.is_equivalent_to_with(&reference, Equivalence::Equivalent),
            "{name}"
        );
    }
}
