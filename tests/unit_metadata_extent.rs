//! PROJ 9.9.0 `test/unit/test_metadata.cpp`, `TEST(metadata, extent)`: the single-box cases
//! about the antimeridian. The expected boxes are the ones PROJ's test checks with `equals`
//! (`contains` both ways), written here as exact bounds (west, south, east, north).

use proj_rs::BoundingBox;

/// `TEST(metadata, extent)`, the `getBBox(...)->intersection(...)` cases and the antimeridian
/// cases of `nz`, `nz_pos_long`, `nz_neg_long`, `nz_shifted`, `nz_smaller`, `world_smaller`,
/// `world_smaller_east`, `east_hemisphere` and `minus_180_to_156`.
#[test]
fn metadata_extent_antimeridian_boxes() {
    let world = BoundingBox::new(-180.0, -90.0, 180.0, 90.0);
    let nz = BoundingBox::new(155.0, -60.0, -170.0, -25.0);
    let nz_pos_long = BoundingBox::new(155.0, -60.0, 180.0, -25.0);
    let nz_neg_long = BoundingBox::new(-180.0, -60.0, -170.0, -25.0);
    let nz_smaller = BoundingBox::new(160.0, -55.0, -175.0, -30.0);
    let world_smaller = BoundingBox::new(-179.0, -90.0, 179.0, 90.0);
    let world_smaller_east = BoundingBox::new(-179.0, -90.0, 150.0, 90.0);
    let east_hemisphere = BoundingBox::new(0.0, -90.0, 180.0, 90.0);
    let minus_180_to_156 = BoundingBox::new(-180.0, -90.0, 156.0, 90.0);

    // world_inter_nz, nz_inter_world (the two box orders)
    assert_eq!(world.intersection(&nz), Some(nz));
    assert_eq!(nz.intersection(&world), Some(nz));

    // nz_inter_nz_pos_long, nz_pos_long_inter_nz
    assert_eq!(nz.intersection(&nz_pos_long), Some(nz_pos_long));
    assert_eq!(nz_pos_long.intersection(&nz), Some(nz_pos_long));

    // nz_inter_nz_neg_long, nz_neg_long_inter_nz
    assert_eq!(nz.intersection(&nz_neg_long), Some(nz_neg_long));
    assert_eq!(nz_neg_long.intersection(&nz), Some(nz_neg_long));

    // nz_inter_nz_shifted: 165..-170
    let nz_shifted = BoundingBox::new(165.0, -60.0, -160.0, -25.0);
    assert_eq!(
        nz.intersection(&nz_shifted),
        Some(BoundingBox::new(165.0, -60.0, -170.0, -25.0))
    );

    // nz_inter_nz_smaller
    assert_eq!(nz.intersection(&nz_smaller), Some(nz_smaller));
    assert_eq!(nz_smaller.intersection(&nz), Some(nz_smaller));

    // nz_inter_world_smaller, world_smaller_inter_nz: 155..179 (the larger part of 155..180)
    let nz_part = BoundingBox::new(155.0, -60.0, 179.0, -25.0);
    assert_eq!(nz.intersection(&world_smaller), Some(nz_part));
    assert_eq!(world_smaller.intersection(&nz), Some(nz_part));

    // nz_inter_world_smaller_east: -179..-170, the larger part of the split
    let nz_west_part = BoundingBox::new(-179.0, -60.0, -170.0, -25.0);
    assert_eq!(nz.intersection(&world_smaller_east), Some(nz_west_part));
    assert_eq!(world_smaller_east.intersection(&nz), Some(nz_west_part));

    // east_hemisphere_inter_nz: 155..180
    assert_eq!(east_hemisphere.intersection(&nz), Some(nz_pos_long));

    // minus_180_to_156_inter_nz: -180..-170, the larger part of the split
    assert_eq!(minus_180_to_156.intersection(&nz), Some(nz_neg_long));
}
