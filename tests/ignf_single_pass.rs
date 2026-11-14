//! The datum pivot between two IGNF CRS does one single pass (PROJ 9.9.0,
//! `coordinateoperationfactory.cpp:3425-3440`, `nIters = (isIGNF(source) && isIGNF(target)) ? 1 : 3`):
//! the full search runs at once, with no direct-only pass and no kind filter. The three-pass search
//! used for every other pair found the EPSG operations `NTF (1)` and `NTF (2)` first between
//! `IGNF:LAMBE` and `IGNF:LAMB93`, which PROJ never lists.
//!
//! The expected lists are what `projinfo -s S -t T --summary --spatial-test intersects` prints for
//! PROJ 9.9.0, on a database copy with no grid file. They are PROJ's values, not this port's.

use proj_rs::iso19111::{
    create_from_user_input, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, DatabaseContext, SpatialCriterion,
};

/// The candidate names of `projinfo -s source -t target --summary --spatial-test intersects`.
fn candidate_names(source: &str, target: &str) -> Vec<String> {
    // The database only: the oracle runs on a copy with no grid file.
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
        .map(CoordinateOperation::name)
        .map(str::to_owned)
        .collect()
}

const NTF_PARIS_TO_WGS84: &str = "Inverse of LAMBERT II ETENDU + NTF geographiques Paris (gr) vers NTF GEOGRAPHIQUES GREENWICH (DMS) + NOUVELLE TRIANGULATION DE LA FRANCE (NTF) vers WGS 84 + Inverse of RGF93G to WGS 84 + ";
const NTF_PARIS_TO_RGF93: &str = "Inverse of LAMBERT II ETENDU + NTF geographiques Paris (gr) vers NTF GEOGRAPHIQUES GREENWICH (DMS) + NOUVELLE TRIANGULATION DE LA FRANCE (NTF) vers RGF93 (ETRS89) + ";

/// `projinfo -s IGNF:LAMBE -t IGNF:LAMB93 --summary --spatial-test intersects` (PROJ 9.9.0): two
/// candidates, both through `NTF geographiques Paris (gr)`; the WGS 84 one needs no grid.
#[test]
fn lambe_to_lamb93_lists_the_ignf_candidates_only() {
    assert_eq!(
        candidate_names("IGNF:LAMBE", "IGNF:LAMB93"),
        vec![
            format!("{NTF_PARIS_TO_WGS84}LAMBERT-93"),
            format!("{NTF_PARIS_TO_RGF93}LAMBERT-93"),
        ]
    );
}

/// `projinfo -s IGNF:LAMBE -t IGNF:GEOPORTALFXX --summary --spatial-test intersects` (PROJ 9.9.0).
#[test]
fn lambe_to_geoportalfxx_lists_the_ignf_candidates_only() {
    assert_eq!(
        candidate_names("IGNF:LAMBE", "IGNF:GEOPORTALFXX"),
        vec![
            format!("{NTF_PARIS_TO_WGS84}EQUIRECTANGULAIRE FRANCE"),
            format!("{NTF_PARIS_TO_RGF93}EQUIRECTANGULAIRE FRANCE"),
        ]
    );
}
