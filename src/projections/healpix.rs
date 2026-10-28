//! HEALPix and rHEALPix.
//!
//! The sphere formulas return radians. On an ellipsoid PROJ replaces `a` with
//! the authalic radius `a * sqrt(qp / 2)` after the authalic-latitude step, so
//! the kernel scales by that factor and the classic wrapper still multiplies
//! by the original `a`.

#![allow(
    clippy::pedantic,
    reason = "cap assembly and the image polygon follow healpix.cpp"
)]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use crate::projmath::{authalic_forward, authalic_inverse, authalic_q};
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};

const EPS: f64 = 1e-15;

const ROT: [[[f64; 2]; 2]; 7] = [
    [[1.0, 0.0], [0.0, 1.0]],
    [[0.0, -1.0], [1.0, 0.0]],
    [[-1.0, 0.0], [0.0, -1.0]],
    [[0.0, 1.0], [-1.0, 0.0]],
    [[0.0, 1.0], [-1.0, 0.0]],
    [[-1.0, 0.0], [0.0, -1.0]],
    [[0.0, -1.0], [1.0, 0.0]],
];

fn sign(v: f64) -> f64 {
    if v > 0.0 {
        1.0
    } else if v < 0.0 {
        -1.0
    } else {
        0.0
    }
}

fn rotate_index(index: i32) -> usize {
    match index {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 3,
        -1 => 4,
        -2 => 5,
        -3 => 6,
        _ => 0,
    }
}

fn rotate(x: f64, y: f64, angle: f64) -> (f64, f64) {
    let (c, s) = (angle.cos(), angle.sin());
    (x * c - y * s, y * c + x * s)
}

fn pnpoly(vert: &[[f64; 2]], testx: f64, testy: f64) -> bool {
    let nvert = vert.len();
    for corner in vert {
        if testx == corner[0] && testy == corner[1] {
            return true;
        }
    }
    let mut counter = 0;
    let mut p1 = vert[0];
    for i in 1..nvert {
        let p2 = vert[i % nvert];
        if testy > p1[1].min(p2[1])
            && testy <= p1[1].max(p2[1])
            && testx <= p1[0].max(p2[0])
            && p1[1] != p2[1]
        {
            let xinters = (testy - p1[1]) * (p2[0] - p1[0]) / (p2[1] - p1[1]) + p1[0];
            if p1[0] == p2[0] || testx <= xinters {
                counter += 1;
            }
        }
        p1 = p2;
    }
    counter % 2 == 1
}

fn in_image(x: f64, y: f64, rhealpix: bool, north_square: i32, south_square: i32) -> bool {
    if !rhealpix {
        let v = [
            [-PI - EPS, FRAC_PI_4],
            [-3.0 * FRAC_PI_4, FRAC_PI_2 + EPS],
            [-FRAC_PI_2, FRAC_PI_4 + EPS],
            [-FRAC_PI_4, FRAC_PI_2 + EPS],
            [0.0, FRAC_PI_4 + EPS],
            [FRAC_PI_4, FRAC_PI_2 + EPS],
            [FRAC_PI_2, FRAC_PI_4 + EPS],
            [3.0 * FRAC_PI_4, FRAC_PI_2 + EPS],
            [PI + EPS, FRAC_PI_4],
            [PI + EPS, -FRAC_PI_4],
            [3.0 * FRAC_PI_4, -FRAC_PI_2 - EPS],
            [FRAC_PI_2, -FRAC_PI_4 - EPS],
            [FRAC_PI_4, -FRAC_PI_2 - EPS],
            [0.0, -FRAC_PI_4 - EPS],
            [-FRAC_PI_4, -FRAC_PI_2 - EPS],
            [-FRAC_PI_2, -FRAC_PI_4 - EPS],
            [-3.0 * FRAC_PI_4, -FRAC_PI_2 - EPS],
            [-PI - EPS, -FRAC_PI_4],
            [-PI - EPS, FRAC_PI_4],
        ];
        pnpoly(&v, x, y)
    } else {
        let north = f64::from(north_square);
        let south = f64::from(south_square);
        let v = [
            [-PI - EPS, FRAC_PI_4 + EPS],
            [-PI + north * FRAC_PI_2 - EPS, FRAC_PI_4 + EPS],
            [-PI + north * FRAC_PI_2 - EPS, 3.0 * FRAC_PI_4 + EPS],
            [-PI + (north + 1.0) * FRAC_PI_2 + EPS, 3.0 * FRAC_PI_4 + EPS],
            [-PI + (north + 1.0) * FRAC_PI_2 + EPS, FRAC_PI_4 + EPS],
            [PI + EPS, FRAC_PI_4 + EPS],
            [PI + EPS, -FRAC_PI_4 - EPS],
            [-PI + (south + 1.0) * FRAC_PI_2 + EPS, -FRAC_PI_4 - EPS],
            [
                -PI + (south + 1.0) * FRAC_PI_2 + EPS,
                -3.0 * FRAC_PI_4 - EPS,
            ],
            [-PI + south * FRAC_PI_2 - EPS, -3.0 * FRAC_PI_4 - EPS],
            [-PI + south * FRAC_PI_2 - EPS, -FRAC_PI_4 - EPS],
            [-PI - EPS, -FRAC_PI_4 - EPS],
        ];
        pnpoly(&v, x, y)
    }
}

fn healpix_sphere(lam: f64, phi: f64) -> (f64, f64) {
    let phi0 = (2.0_f64 / 3.0).asin();
    if phi.abs() <= phi0 {
        (lam, 3.0 * PI / 8.0 * phi.sin())
    } else {
        let sigma = (3.0 * (1.0 - phi.sin().abs())).sqrt();
        let mut cn = (2.0 * lam / PI + 2.0).floor();
        if cn >= 4.0 {
            cn = 3.0;
        }
        let lamc = -3.0 * FRAC_PI_4 + FRAC_PI_2 * cn;
        (
            lamc + (lam - lamc) * sigma,
            sign(phi) * FRAC_PI_4 * (2.0 - sigma),
        )
    }
}

fn healpix_inverse_sphere(x: f64, y: f64) -> (f64, f64) {
    if y.abs() <= FRAC_PI_4 {
        (x, (8.0 * y / (3.0 * PI)).asin())
    } else if y.abs() < FRAC_PI_2 {
        let mut cn = (2.0 * x / PI + 2.0).floor();
        if cn >= 4.0 {
            cn = 3.0;
        }
        let xc = -3.0 * FRAC_PI_4 + FRAC_PI_2 * cn;
        let tau = 2.0 - 4.0 * y.abs() / PI;
        (
            xc + (x - xc) / tau,
            sign(y) * (1.0 - tau.powi(2) / 3.0).asin(),
        )
    } else {
        (-PI, sign(y) * FRAC_PI_2)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Region {
    North,
    South,
    Equatorial,
}

struct Cap {
    cn: i32,
    x: f64,
    y: f64,
    region: Region,
}

fn get_cap(mut x: f64, y: f64, north_square: i32, south_square: i32, inverse: bool) -> Cap {
    if !inverse {
        let (region, c) = if y > FRAC_PI_4 {
            (Region::North, FRAC_PI_2)
        } else if y < -FRAC_PI_4 {
            (Region::South, -FRAC_PI_2)
        } else {
            return Cap {
                cn: 0,
                x,
                y,
                region: Region::Equatorial,
            };
        };
        let (cn, tip_x) = if x < -FRAC_PI_2 {
            (0, -3.0 * FRAC_PI_4)
        } else if x < 0.0 {
            (1, -FRAC_PI_4)
        } else if x < FRAC_PI_2 {
            (2, FRAC_PI_4)
        } else {
            (3, 3.0 * FRAC_PI_4)
        };
        Cap {
            cn,
            x: tip_x,
            y: c,
            region,
        }
    } else if y > FRAC_PI_4 {
        let tip_x = -3.0 * FRAC_PI_4 + f64::from(north_square) * FRAC_PI_2;
        x -= f64::from(north_square) * FRAC_PI_2;
        let cn = if y >= -x - FRAC_PI_4 - EPS && y < x + 5.0 * FRAC_PI_4 - EPS {
            (north_square + 1) % 4
        } else if y > -x - FRAC_PI_4 + EPS && y >= x + 5.0 * FRAC_PI_4 - EPS {
            (north_square + 2) % 4
        } else if y <= -x - FRAC_PI_4 + EPS && y > x + 5.0 * FRAC_PI_4 + EPS {
            (north_square + 3) % 4
        } else {
            north_square
        };
        Cap {
            cn,
            x: tip_x,
            y: FRAC_PI_2,
            region: Region::North,
        }
    } else if y < -FRAC_PI_4 {
        let tip_x = -3.0 * FRAC_PI_4 + f64::from(south_square) * FRAC_PI_2;
        x -= f64::from(south_square) * FRAC_PI_2;
        let cn = if y <= x + FRAC_PI_4 + EPS && y > -x - 5.0 * FRAC_PI_4 + EPS {
            (south_square + 1) % 4
        } else if y < x + FRAC_PI_4 - EPS && y <= -x - 5.0 * FRAC_PI_4 + EPS {
            (south_square + 2) % 4
        } else if y >= x + FRAC_PI_4 - EPS && y < -x - 5.0 * FRAC_PI_4 - EPS {
            (south_square + 3) % 4
        } else {
            south_square
        };
        Cap {
            cn,
            x: tip_x,
            y: -FRAC_PI_2,
            region: Region::South,
        }
    } else {
        Cap {
            cn: 0,
            x,
            y,
            region: Region::Equatorial,
        }
    }
}

fn combine_caps(x: f64, y: f64, north_square: i32, south_square: i32, inverse: bool) -> (f64, f64) {
    let cap = get_cap(x, y, north_square, south_square, inverse);
    if cap.region == Region::Equatorial {
        return (cap.x, cap.y);
    }
    let pole = if cap.region == Region::North {
        north_square
    } else {
        south_square
    };
    let index = if inverse {
        if cap.region == Region::North {
            rotate_index(-(cap.cn - pole))
        } else {
            rotate_index(cap.cn - pole)
        }
    } else if cap.region == Region::North {
        rotate_index(cap.cn - pole)
    } else {
        rotate_index(-(cap.cn - pole))
    };
    let rot = ROT[index];
    let dx = x - cap.x;
    let dy = y - cap.y;
    let rx = rot[0][0] * dx + rot[0][1] * dy;
    let ry = rot[1][0] * dx + rot[1][1] * dy;
    let dest = if inverse { cap.cn } else { pole };
    let ax = -3.0 * FRAC_PI_4 + f64::from(dest) * FRAC_PI_2;
    let ay = if cap.region == Region::North {
        FRAC_PI_2
    } else {
        -FRAC_PI_2
    };
    (rx + ax, ry + ay)
}

struct Healpix {
    rhealpix: bool,
    north_square: i32,
    south_square: i32,
    rot: f64,
    ellipsoidal: bool,
    scale: f64,
    n: f64,
    e: f64,
    es: f64,
    one_es: f64,
    qp: f64,
}

impl Healpix {
    fn auth_forward(&self, phi: f64) -> f64 {
        if self.ellipsoidal {
            authalic_forward(phi, phi.sin(), self.n, self.e, self.one_es, self.qp)
        } else {
            phi
        }
    }

    fn auth_inverse(&self, phi: f64) -> f64 {
        if self.ellipsoidal {
            authalic_inverse(phi, self.n, self.e, self.es, self.one_es, self.qp)
        } else {
            phi
        }
    }
}

impl Kernel for Healpix {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let phi = self.auth_forward(phi);
        let (x, y) = healpix_sphere(lam, phi);
        let (x, y) = if self.rhealpix {
            combine_caps(x, y, self.north_square, self.south_square, false)
        } else {
            rotate(x, y, -self.rot)
        };
        Ok((x * self.scale, y * self.scale))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let x = x / self.scale;
        let y = y / self.scale;
        let (x, y) = if self.rhealpix {
            if !in_image(x, y, true, self.north_square, self.south_square) {
                return Err(TransformError::OutsideDomain);
            }
            combine_caps(x, y, self.north_square, self.south_square, true)
        } else {
            let (x, y) = rotate(x, y, self.rot);
            if !in_image(x, y, false, 0, 0) {
                return Err(TransformError::OutsideDomain);
            }
            (x, y)
        };
        let (lam, phi) = healpix_inverse_sphere(x, y);
        Ok((lam, self.auth_inverse(phi)))
    }
}

fn square(params: &Params, key: &'static str) -> Result<i32, CreateError> {
    let value = params.integer(key)?.unwrap_or(0);
    if (0..=3).contains(&value) {
        Ok(value)
    } else {
        Err(CreateError::IllegalValue(key))
    }
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "healpix" => Some(&["rot_xy", "lat_1", "lat_2"]),
        "rhealpix" => Some(&["north_square", "south_square"]),
        _ => None,
    }
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    let earth = frame.earth;
    let ellipsoidal = earth.es() != 0.0;
    let qp = if ellipsoidal {
        authalic_q(1.0, earth.e(), earth.one_es())
    } else {
        2.0
    };
    let scale = if ellipsoidal { (0.5 * qp).sqrt() } else { 1.0 };
    let kernel = match name {
        "healpix" => Healpix {
            rhealpix: false,
            north_square: 0,
            south_square: 0,
            rot: params.radians("rot_xy")?.unwrap_or(0.0),
            ellipsoidal,
            scale,
            n: earth.n(),
            e: earth.e(),
            es: earth.es(),
            one_es: earth.one_es(),
            qp,
        },
        "rhealpix" => Healpix {
            rhealpix: true,
            north_square: square(params, "north_square")?,
            south_square: square(params, "south_square")?,
            rot: 0.0,
            ellipsoidal,
            scale,
            n: earth.n(),
            e: earth.e(),
            es: earth.es(),
            one_es: earth.one_es(),
            qp,
        },
        _ => return Ok(None),
    };
    Ok(Some(classic_step(frame, kernel)))
}
