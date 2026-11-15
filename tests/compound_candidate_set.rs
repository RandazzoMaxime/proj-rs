//! The candidate set of a compound CRS to or from a geographic or geocentric CRS (PROJ 9.9.0,
//! `coordinateoperationfactory.cpp` `createOperationsCompoundToGeog`, `createOperationsToGeod`,
//! `createOperationsGeodToGeod`).
//!
//! The expected lists are the ones `projinfo --summary --spatial-test intersects` prints for PROJ
//! 9.9.0, on a database copy with no grid file. They are PROJ's values, not this port's.

use proj_rs::iso19111::{
    create_from_user_input, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, DatabaseContext, SpatialCriterion,
};

/// The candidate names of `projinfo -s source -t target --summary --spatial-test intersects`.
fn names(source: &str, target: &str) -> Vec<String> {
    std::env::set_var("PROJ_DATA", concat!(env!("CARGO_MANIFEST_DIR"), "/data"));
    std::env::set_var("PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY", "YES");
    let db = DatabaseContext::create().expect("database");
    let source = create_from_user_input(source, &db).expect("source");
    let target = create_from_user_input(target, &db).expect("target");
    let mut context = CoordinateOperationContext::create(&db, "");
    context.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations")
        .iter()
        .map(|operation| CoordinateOperation::name(operation).to_owned())
        .collect()
}

/// `projinfo -s EPSG:11087 -t EPSG:7815 --summary --spatial-test intersects` (PROJ 9.9.0): a geographic CRS to a geocentric CRS of another datum: after the registry result, the
/// geographic CRS of the geocentric one and the full `createOperations` from it (the second
/// candidate, coordinateoperationfactory.cpp:5314-5331).
#[test]
fn geographic_to_geocentric_of_another_datum_reads_the_registry() {
    assert_eq!(
        names("EPSG:11087", "EPSG:7815"),
        [
            "ETRS89-FRO [2008] to WGS 84 (1) + WGS 84 to WGS 84 (Transit) + Conversion from WGS 84 (Transit) (geog2D) to WGS 84 (Transit) (geocentric)",
            "ETRS89-FRO [2008] to WGS 84 (1) + WGS 84 to WGS 84 (Transit) + axis order change (geographic3D horizontal) + Conversion from WGS 84 (Transit) (geographic) to WGS 84 (Transit)",
        ]
    );
}

/// `projinfo -s EPSG:12216 -t EPSG:7912 --summary --spatial-test intersects` (PROJ 9.9.0): the geoid model `ETRS89-NLD [AGRS2010] to MSL-NLD depth (1)` uses the method
/// `Geographic3D to Depth (gtx)`, which PROJ cannot write: `PROJBasedOperation::create` throws and
/// its three candidates are dropped.
#[test]
fn a_vertical_grid_without_proj_string_drops_its_candidates() {
    assert_eq!(
        names("EPSG:12216", "EPSG:7912"),
        [
            "Ballpark geographic offset from ETRS89-NLD [AGRS2010] to ITRF2014 + Transformation from MSL-NLD depth to ITRF2014 (ballpark vertical transformation, without ellipsoid height to vertical height correction)",
            "Conversion from ETRS89-NLD [AGRS2010] (geog2D) to ETRS89-NLD [AGRS2010] (geocentric) + ETRS89-NLD [AGRS2010] to ETRF2000 (1) + Inverse of ITRF2014 to ETRF2000 (1) + Conversion from ITRF2014 (geocentric) to ITRF2014 (geog3D) + Transformation from MSL-NLD depth to ITRF2014 (ballpark vertical transformation, without ellipsoid height to vertical height correction)",
            "Conversion from ETRS89-NLD [AGRS2010] (geog2D) to ETRS89-NLD [AGRS2010] (geocentric) + Inverse of ITRF2020 to ETRS89-NLD [AGRS2010] (1) + Inverse of ITRF2014 to ITRF2020 (1) + Conversion from ITRF2014 (geocentric) to ITRF2014 (geog3D) + Transformation from MSL-NLD depth to ITRF2014 (ballpark vertical transformation, without ellipsoid height to vertical height correction)",
        ]
    );
}

/// `projinfo -s EPSG:8401 -t EPSG:12216 --summary --spatial-test intersects` (PROJ 9.9.0): the inverse of the compound CRS to the geocentric CRS, with the same context: the
/// pivots through ETRF2000 and ITRF2020 of the intermediate search are kept.
#[test]
fn geocentric_to_compound_searches_intermediates() {
    assert_eq!(
        names("EPSG:8401", "EPSG:12216"),
        [
            "Conversion from ETRF2014 (geocentric) to ETRF2014 (geog3D) + Inverse of Transformation from MSL-NLD depth to ETRF2014 (ballpark vertical transformation, without ellipsoid height to vertical height correction) + Inverse of Ballpark geographic offset from ETRS89-NLD [AGRS2010] to ETRF2014",
            "Conversion from ETRF2014 (geocentric) to ETRF2014 (geog3D) + Inverse of Transformation from MSL-NLD depth to ETRF2014 (ballpark vertical transformation, without ellipsoid height to vertical height correction) + Inverse of Conversion from ETRF2014 (geocentric) to ETRF2014 (geog3D) + ETRF2014 to ETRF2000 (1) + Inverse of ETRS89-NLD [AGRS2010] to ETRF2000 (1) + Inverse of Conversion from ETRS89-NLD [AGRS2010] (geog2D) to ETRS89-NLD [AGRS2010] (geocentric)",
            "Conversion from ETRF2014 (geocentric) to ETRF2014 (geog3D) + Inverse of Transformation from MSL-NLD depth to ETRF2014 (ballpark vertical transformation, without ellipsoid height to vertical height correction) + Inverse of Conversion from ETRF2014 (geocentric) to ETRF2014 (geog3D) + Inverse of ITRF2020 to ETRF2014 (1) + ITRF2020 to ETRS89-NLD [AGRS2010] (1) + Inverse of Conversion from ETRS89-NLD [AGRS2010] (geog2D) to ETRS89-NLD [AGRS2010] (geocentric)",
        ]
    );
}

/// `projinfo -s EPSG:7423 -t EPSG:10283 --summary --spatial-test intersects` (PROJ 9.9.0): the vertical chains through DHHN2016 and LN02 have no identifier of their own, but
/// their steps have (`hasIdentifiers`), so the vertical operations to the source geographic CRS
/// are added.
#[test]
fn registered_vertical_chain_counts_as_registered() {
    assert_eq!(
        names("EPSG:7423", "EPSG:10283"),
        [
            "ETRS89 to ETRS89-DEU [ETRS89/DREF91/2016] + Inverse of DHHN2016 height to EVRF2007 height (1) + Inverse of ETRS89-DEU [ETRS89/DREF91/2016] to DHHN2016 height (1)",
            "ETRS89 to ETRS89-DEU [ETRS89/DREF91/2016] + Inverse of LN02 height to EVRF2007 height (1) + Inverse of ETRS89 to LN02 height",
            "Ballpark geographic offset from ETRS89 to ETRS89-DEU [ETRS89/DREF91/2016] + Inverse of LN02 height to EVRF2007 height (1) + Inverse of ETRS89 to LN02 height",
            "ETRS89 to ETRS89-DEU [ETRS89/DREF91/2016] + Transformation from EVRF2007 height to ETRS89 (ballpark vertical transformation, without ellipsoid height to vertical height correction)",
            "Ballpark geographic offset from ETRS89 to ETRS89-DEU [ETRS89/DREF91/2016] + Transformation from EVRF2007 height to ETRS89 (ballpark vertical transformation, without ellipsoid height to vertical height correction)",
        ]
    );
}

/// `projinfo -s EPSG:4911 -t EPSG:10157 --summary --spatial-test intersects` (PROJ 9.9.0): the intermediate geographic 3D CRS of `createOperationsToGeod` has no identifier: `buildCRSIds` (coordinateoperationfactory.cpp:1775-1870) gives it the one of its name, so the datum based intermediates find the ETRF realizations.
#[test]
fn geocentric_to_compound_names_the_intermediate_by_its_name() {
    assert_eq!(
        names("EPSG:4911", "EPSG:10157"),
        [
            "Conversion from ITRF89 (geocentric) to ITRF89 (geog3D) + Inverse of Transformation from CD UK & Ireland VORF08 depth to ITRF89 (ballpark vertical transformation, without ellipsoid height to vertical height correction) + Inverse of Ballpark geographic offset from ETRS89 to ITRF89",
            "Conversion from ITRF89 (geocentric) to ITRF89 (geog3D) + Inverse of Transformation from CD UK & Ireland VORF08 depth to ITRF89 (ballpark vertical transformation, without ellipsoid height to vertical height correction) + Inverse of Conversion from ITRF89 (geocentric) to ITRF89 (geog3D) + ITRF89 to ETRF89 (1) + Inverse of Conversion from ETRF89 (geog2D) to ETRF89 (geocentric) + Inverse of ETRS89 to ETRF89",
            "Conversion from ITRF89 (geocentric) to ITRF89 (geog3D) + Inverse of Transformation from CD UK & Ireland VORF08 depth to ITRF89 (ballpark vertical transformation, without ellipsoid height to vertical height correction) + Inverse of Conversion from ITRF89 (geocentric) to ITRF89 (geog3D) + ITRF89 to ETRF2020 (1) + Inverse of Conversion from ETRF2020 (geog2D) to ETRF2020 (geocentric) + Inverse of ETRS89 to ETRF2020",
            "Conversion from ITRF89 (geocentric) to ITRF89 (geog3D) + Inverse of Transformation from CD UK & Ireland VORF08 depth to ITRF89 (ballpark vertical transformation, without ellipsoid height to vertical height correction) + Inverse of Conversion from ITRF89 (geocentric) to ITRF89 (geog3D) + ITRF89 to ETRF2014 (1) + Inverse of Conversion from ETRF2014 (geog2D) to ETRF2014 (geocentric) + Inverse of ETRS89 to ETRF2014",
            "Conversion from ITRF89 (geocentric) to ITRF89 (geog3D) + Inverse of Transformation from CD UK & Ireland VORF08 depth to ITRF89 (ballpark vertical transformation, without ellipsoid height to vertical height correction) + Inverse of Conversion from ITRF89 (geocentric) to ITRF89 (geog3D) + ITRF89 to ETRF2000 (1) + Inverse of Conversion from ETRF2000 (geog2D) to ETRF2000 (geocentric) + Inverse of ETRS89 to ETRF2000",
            "Conversion from ITRF89 (geocentric) to ITRF89 (geog3D) + Inverse of Transformation from CD UK & Ireland VORF08 depth to ITRF89 (ballpark vertical transformation, without ellipsoid height to vertical height correction) + Inverse of Conversion from ITRF89 (geocentric) to ITRF89 (geog3D) + ITRF89 to ITRF2014 (1) + ITRF2014 to ETRF2014 (1) + Inverse of NKG_ETRF14 to ETRF2014 + NKG_ETRF14 to ETRF92@2000.0 + ETRF92@2000.0 to ETRF92@1994.704 using nkgrf17vel + Inverse of Conversion from ETRS89 (geog2D) to ETRS89 (geocentric)",
            "Conversion from ITRF89 (geocentric) to ITRF89 (geog3D) + Inverse of Transformation from CD UK & Ireland VORF08 depth to ITRF89 (ballpark vertical transformation, without ellipsoid height to vertical height correction) + Inverse of Conversion from ITRF89 (geocentric) to ITRF89 (geog3D) + ITRF89 to ITRF2000 (1) + ITRF2000 to ETRF2000 (2) + Inverse of NKG_ETRF00 to ETRF2000 + NKG_ETRF00 to ETRF92@2000.0 + ETRF92@2000.0 to ETRF92@1994.704 using nkgrf03vel_realigned + Inverse of Conversion from ETRS89 (geog2D) to ETRS89 (geocentric)",
        ]
    );
}
