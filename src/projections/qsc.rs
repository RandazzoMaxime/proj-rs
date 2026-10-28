//! Quadrilateralized Spherical Cube.
//!
//! Cube-face selection uses `lat_0` / `lon_0`. The kernel returns coordinates
//! in units of the semi-major axis. On an ellipsoid the latitude is the
//! geocentric latitude from Lambers and Kolb before the spherical cube map.

#![allow(
    clippy::pedantic,
    reason = "face and area selection follow qsc.cpp branch for branch"
)]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

const EPS10: f64 = 1e-10;
const PI_HALFPI: f64 = PI + FRAC_PI_2;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Face {
    Front,
    Right,
    Back,
    Left,
    Top,
    Bottom,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Area {
    A0,
    A1,
    A2,
    A3,
}

struct Qsc {
    face: Face,
    ellipsoidal: bool,
    a: f64,
    b: f64,
    one_minus_f: f64,
    one_minus_f_sq: f64,
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "qsc" {
        Some(&[])
    } else {
        None
    }
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    if name != "qsc" {
        return Ok(None);
    }
    let _ = params;
    let earth = frame.earth;
    let es = earth.es();
    let ellipsoidal = es != 0.0;
    let one_minus_f = if ellipsoidal { (1.0 - es).sqrt() } else { 1.0 };
    let face = if frame.phi0 >= FRAC_PI_2 - FRAC_PI_4 / 2.0 {
        Face::Top
    } else if frame.phi0 <= -(FRAC_PI_2 - FRAC_PI_4 / 2.0) {
        Face::Bottom
    } else if frame.lam0.abs() <= FRAC_PI_4 {
        Face::Front
    } else if frame.lam0.abs() <= FRAC_PI_2 + FRAC_PI_4 {
        if frame.lam0 > 0.0 {
            Face::Right
        } else {
            Face::Left
        }
    } else {
        Face::Back
    };
    Ok(Some(classic_step(
        frame,
        Qsc {
            face,
            ellipsoidal,
            a: earth.a(),
            b: earth.a() * one_minus_f,
            one_minus_f,
            one_minus_f_sq: one_minus_f * one_minus_f,
        },
    )))
}

impl Kernel for Qsc {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let lat = if self.ellipsoidal {
            (self.one_minus_f_sq * phi.tan()).atan()
        } else {
            phi
        };
        let (phi_face, theta, area) = self.face_angles(lat, lam);
        let mu_area =
            ((12.0 / PI) * (theta + (theta.sin() * FRAC_PI_4.cos()).acos() - FRAC_PI_2)).atan();
        let denom = 1.0 - (1.0 / theta.cos()).atan().cos();
        let t = ((1.0 - phi_face.cos()) / (mu_area.cos() * mu_area.cos()) / denom).sqrt();
        let mu = match area {
            Area::A0 => mu_area,
            Area::A1 => mu_area + FRAC_PI_2,
            Area::A2 => mu_area + PI,
            Area::A3 => mu_area + PI_HALFPI,
        };
        Ok((t * mu.cos(), t * mu.sin()))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let nu = (x * x + y * y).sqrt().atan();
        let mut mu = y.atan2(x);
        let area = if x >= 0.0 && x >= y.abs() {
            Area::A0
        } else if y >= 0.0 && y >= x.abs() {
            mu -= FRAC_PI_2;
            Area::A1
        } else if x < 0.0 && -x >= y.abs() {
            mu = if mu < 0.0 { mu + PI } else { mu - PI };
            Area::A2
        } else {
            mu += FRAC_PI_2;
            Area::A3
        };
        let t = (PI / 12.0) * mu.tan();
        let tantheta = t.sin() / (t.cos() - 1.0 / 2.0_f64.sqrt());
        let theta = tantheta.atan();
        let cosmu = mu.cos();
        let tannu = nu.tan();
        let mut cosphi =
            1.0 - cosmu * cosmu * tannu * tannu * (1.0 - (1.0 / theta.cos()).atan().cos());
        cosphi = cosphi.clamp(-1.0, 1.0);
        let (lam, mut phi) = self.invert_face(cosphi, theta, area);
        if self.ellipsoidal {
            let invert = phi < 0.0;
            let tanphi = phi.tan();
            let xa = self.b / (tanphi * tanphi + self.one_minus_f_sq).sqrt();
            phi = ((self.a * self.a - xa * xa).sqrt() / (self.one_minus_f * xa)).atan();
            if invert {
                phi = -phi;
            }
        }
        Ok((lam, phi))
    }
}

impl Qsc {
    fn face_angles(&self, lat: f64, longitude: f64) -> (f64, f64, Area) {
        match self.face {
            Face::Top => {
                let phi = FRAC_PI_2 - lat;
                if longitude >= FRAC_PI_4 && longitude <= FRAC_PI_2 + FRAC_PI_4 {
                    (phi, longitude - FRAC_PI_2, Area::A0)
                } else if longitude > FRAC_PI_2 + FRAC_PI_4 || longitude <= -(FRAC_PI_2 + FRAC_PI_4)
                {
                    let theta = if longitude > 0.0 {
                        longitude - PI
                    } else {
                        longitude + PI
                    };
                    (phi, theta, Area::A1)
                } else if longitude > -(FRAC_PI_2 + FRAC_PI_4) && longitude <= -FRAC_PI_4 {
                    (phi, longitude + FRAC_PI_2, Area::A2)
                } else {
                    (phi, longitude, Area::A3)
                }
            }
            Face::Bottom => {
                let phi = FRAC_PI_2 + lat;
                if longitude >= FRAC_PI_4 && longitude <= FRAC_PI_2 + FRAC_PI_4 {
                    (phi, -longitude + FRAC_PI_2, Area::A0)
                } else if longitude < FRAC_PI_4 && longitude >= -FRAC_PI_4 {
                    (phi, -longitude, Area::A1)
                } else if longitude < -FRAC_PI_4 && longitude >= -(FRAC_PI_2 + FRAC_PI_4) {
                    (phi, -longitude - FRAC_PI_2, Area::A2)
                } else {
                    let theta = if longitude > 0.0 {
                        -longitude + PI
                    } else {
                        -longitude - PI
                    };
                    (phi, theta, Area::A3)
                }
            }
            Face::Front | Face::Right | Face::Back | Face::Left => {
                let shifted = match self.face {
                    Face::Right => shift_lon(longitude, FRAC_PI_2),
                    Face::Back => shift_lon(longitude, PI),
                    Face::Left => shift_lon(longitude, -FRAC_PI_2),
                    Face::Front | Face::Top | Face::Bottom => longitude,
                };
                let (sinlat, coslat) = (lat.sin(), lat.cos());
                let (sinlon, coslon) = (shifted.sin(), shifted.cos());
                let q = coslat * coslon;
                let r = coslat * sinlon;
                let s = sinlat;
                let (phi, y, x) = match self.face {
                    Face::Front => (q.acos(), s, r),
                    Face::Right => (r.acos(), s, -q),
                    Face::Back => ((-q).acos(), s, -r),
                    Face::Left => ((-r).acos(), s, q),
                    Face::Top | Face::Bottom => (0.0, 0.0, 0.0),
                };
                let (theta, area) = equat_theta(phi, y, x);
                (phi, theta, area)
            }
        }
    }

    fn invert_face(&self, cosphi: f64, theta: f64, area: Area) -> (f64, f64) {
        match self.face {
            Face::Top => {
                let phi = FRAC_PI_2 - cosphi.acos();
                let lam = match area {
                    Area::A0 => theta + FRAC_PI_2,
                    Area::A1 => {
                        if theta < 0.0 {
                            theta + PI
                        } else {
                            theta - PI
                        }
                    }
                    Area::A2 => theta - FRAC_PI_2,
                    Area::A3 => theta,
                };
                (lam, phi)
            }
            Face::Bottom => {
                let phi = cosphi.acos() - FRAC_PI_2;
                let lam = match area {
                    Area::A0 => -theta + FRAC_PI_2,
                    Area::A1 => -theta,
                    Area::A2 => -theta - FRAC_PI_2,
                    Area::A3 => {
                        if theta < 0.0 {
                            -theta - PI
                        } else {
                            -theta + PI
                        }
                    }
                };
                (lam, phi)
            }
            Face::Front | Face::Right | Face::Back | Face::Left => {
                let mut q = cosphi;
                let mut t = q * q;
                let mut s = if t >= 1.0 {
                    0.0
                } else {
                    (1.0 - t).sqrt() * theta.sin()
                };
                t += s * s;
                let mut r = if t >= 1.0 { 0.0 } else { (1.0 - t).sqrt() };
                match area {
                    Area::A0 => {}
                    Area::A1 => {
                        let swap = r;
                        r = -s;
                        s = swap;
                    }
                    Area::A2 => {
                        r = -r;
                        s = -s;
                    }
                    Area::A3 => {
                        let swap = r;
                        r = s;
                        s = -swap;
                    }
                }
                match self.face {
                    Face::Right => {
                        let swap = q;
                        q = -r;
                        r = swap;
                    }
                    Face::Back => {
                        q = -q;
                        r = -r;
                    }
                    Face::Left => {
                        let swap = q;
                        q = r;
                        r = -swap;
                    }
                    Face::Front | Face::Top | Face::Bottom => {}
                }
                let mut lam = r.atan2(q);
                lam = match self.face {
                    Face::Right => shift_lon(lam, -FRAC_PI_2),
                    Face::Back => shift_lon(lam, -PI),
                    Face::Left => shift_lon(lam, FRAC_PI_2),
                    Face::Front | Face::Top | Face::Bottom => lam,
                };
                (lam, (-s).acos() - FRAC_PI_2)
            }
        }
    }
}

fn equat_theta(phi: f64, y: f64, x: f64) -> (f64, Area) {
    if phi < EPS10 {
        return (0.0, Area::A0);
    }
    let mut theta = y.atan2(x);
    let area = if theta.abs() <= FRAC_PI_4 {
        Area::A0
    } else if theta > FRAC_PI_4 && theta <= FRAC_PI_2 + FRAC_PI_4 {
        theta -= FRAC_PI_2;
        Area::A1
    } else if theta > FRAC_PI_2 + FRAC_PI_4 || theta <= -(FRAC_PI_2 + FRAC_PI_4) {
        theta = if theta >= 0.0 { theta - PI } else { theta + PI };
        Area::A2
    } else {
        theta += FRAC_PI_2;
        Area::A3
    };
    (theta, area)
}

fn shift_lon(longitude: f64, offset: f64) -> f64 {
    let mut shifted = longitude + offset;
    if shifted < -PI {
        shifted += TAU;
    } else if shifted > PI {
        shifted -= TAU;
    }
    shifted
}
