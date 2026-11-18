//! `helmert` over a slice against `helmert` point by point (`docs/PORTING-batch-datum.md`).

use crate::engine::batch_check::{assert_agrees, assert_vector_taken, unit_source, Way, METRES};
use crate::engine::{Coord, Operation};

/// Position vector and coordinate frame, three and seven parameters, `+exact`, a scale alone,
/// Molodensky-Badekas: these take the vector path.
const FIXED: [&str; 6] = [
    "+proj=helmert +x=-28 +y=121 +z=77",
    "+proj=helmert +x=-106.8686 +y=52.2978 +z=-103.7239 +rx=-0.3366 +ry=0.457 +rz=-1.8422 \
     +s=-1.2747 +convention=coordinate_frame",
    "+proj=helmert +x=482.5 +y=-130.6 +z=564.6 +rx=-1.042 +ry=-0.214 +rz=-0.631 +s=8.15 \
     +convention=position_vector",
    "+proj=helmert +x=1 +y=2 +z=3 +rx=100 +ry=-200 +rz=300 +s=5 +convention=position_vector \
     +exact",
    "+proj=helmert +s=15",
    "+proj=molobadekas +x=-265.8867 +y=76.9851 +z=20.2667 +rx=0.33746 +ry=3.09264 \
     +rz=-2.53861 +s=0.4598 +px=4103620.3943 +py=440486.4235 +pz=4846923.4558 \
     +convention=coordinate_frame",
];

/// Rates in time, and the planar four parameters: the scalar loop.
const SCALAR: [&str; 3] = [
    "+proj=helmert +x=0.1 +y=0.2 +z=0.3 +rx=0.01 +ry=0.02 +rz=0.03 +s=0.5 \
     +convention=position_vector +t_epoch=2000 +dx=0.001",
    "+proj=helmert +x=0.01 +y=0.02 +z=0.03 +dx=0.001 +dy=-0.002 +dz=0.003 +drx=0.001 \
     +dry=0.002 +drz=-0.003 +ds=0.01 +rx=0.1 +ry=0.2 +rz=0.3 +s=1 +t_epoch=2010 \
     +convention=position_vector",
    "+proj=helmert +x=10 +y=-20 +theta=30 +s=1.0001",
];

fn at(x: f64, y: f64, z: f64, t: f64) -> Coord {
    Coord { x, y, z, t }
}

fn points() -> Vec<Coord> {
    let mut unit = unit_source(31);
    let mut times = [f64::NAN, 2000.0, 2020.5, f64::INFINITY]
        .into_iter()
        .cycle();
    let mut points: Vec<Coord> = (0..3_001)
        .map(|_| {
            at(
                (unit() - 0.5) * 1.4e7,
                (unit() - 0.5) * 1.4e7,
                (unit() - 0.5) * 1.4e7,
                times.next().unwrap_or(0.0),
            )
        })
        .collect();
    points.extend([
        at(0.0, 0.0, 0.0, 0.0),
        at(-0.0, -0.0, -0.0, 0.0),
        at(f64::NAN, 1.0, 2.0, 0.0),
        at(1.0, f64::INFINITY, 2.0, 0.0),
        at(1.0, 2.0, f64::NEG_INFINITY, 0.0),
        at(1e300, -1e300, 1e300, 0.0),
        at(4.1e6, 4.4e5, 4.8e6, f64::NAN),
    ]);
    points
}

#[test]
fn a_fixed_transformation_agrees_with_one_by_one() {
    let points = points();
    for definition in FIXED {
        let op = Operation::create(definition).expect(definition);
        for way in [Way::Forward, Way::Inverse] {
            let differing = assert_agrees(&op, &points, way, [METRES; 3]);
            // A translation and a scale are exact in both paths: only a rotation shows.
            if definition.contains("+rx") {
                assert_vector_taken(definition, differing);
            }
        }
    }
}

#[test]
fn a_transformation_with_rates_keeps_the_scalar_loop() {
    let points = points();
    for definition in SCALAR {
        let op = Operation::create(definition).expect(definition);
        for way in [Way::Forward, Way::Inverse] {
            let differing = assert_agrees(&op, &points, way, [METRES; 3]);
            assert_eq!(differing, 0, "{definition}: bit for bit with one by one");
        }
    }
}

#[test]
fn a_unit_is_applied_around_the_kernel() {
    let points = points();
    for definition in [
        "+proj=helmert +x=-28 +y=121 +z=77 +units=km",
        "+proj=helmert +x=1 +y=2 +z=3 +rx=1 +ry=2 +rz=3 +s=1 +convention=position_vector +units=ft",
    ] {
        let op = Operation::create(definition).expect(definition);
        for way in [Way::Forward, Way::Inverse] {
            assert_agrees(&op, &points, way, [METRES; 3]);
        }
    }
}
