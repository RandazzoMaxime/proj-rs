//! Interrupted Mollweide and Goode Homolosine layouts.
//!
//! Zone offsets are in units of the semi-major axis, added inside the kernel,
//! the same way `igh.cpp` adds each lobe's `x0` before `fwd.cpp` scales by `a`.

#![allow(
    clippy::pedantic,
    reason = "zone tables and lobe tests are transcribed from the PROJ sources"
)]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use crate::projections::sphere2::{mollweide_forward, mollweide_inverse};

const DEG: f64 = 0.017_453_292_519_943_296;
const EPS: f64 = 1e-10;
const PHI_B: f64 = (40.0 + 44.0 / 60.0 + 11.8 / 3600.0) * DEG;

fn deg(value: f64) -> f64 {
    value * DEG
}

#[derive(Clone, Copy)]
struct Zone {
    moll: bool,
    lam0: f64,
    x0: f64,
    y0: f64,
}

fn lobe(moll: bool, lon_deg: f64, y0: f64) -> Zone {
    let lon = deg(lon_deg);
    Zone {
        moll,
        lam0: lon,
        x0: lon,
        y0,
    }
}

fn zone_forward(zone: Zone, lam: f64, phi: f64) -> (f64, f64) {
    let lam = lam - zone.lam0;
    let (x, y) = if zone.moll {
        mollweide_forward(lam, phi)
    } else {
        (lam * phi.cos(), phi)
    };
    (x + zone.x0, y + zone.y0)
}

fn zone_inverse(zone: Zone, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
    let x = x - zone.x0;
    let y = y - zone.y0;
    let (lam, phi) = if zone.moll {
        mollweide_inverse(x, y)?
    } else {
        (x / y.cos(), y)
    };
    Ok((lam + zone.lam0, phi))
}

fn raw_y_gap() -> f64 {
    let (_, moll_y) = mollweide_forward(0.0, PHI_B);
    PHI_B - moll_y
}

fn covers(lam: f64, lo: f64, hi: f64) -> bool {
    lam >= lo - EPS && lam <= hi + EPS
}

fn seam(forward: &impl Fn(f64, f64) -> (f64, f64), lam: f64, phi: f64) -> f64 {
    let (left, _) = forward(lam - EPS, phi);
    let (right, _) = forward(lam + EPS, phi);
    (left + right) / 2.0
}

fn shift_to(zones: &mut [Zone], from: usize, onto: usize, lam: f64, phi_from: f64, phi_onto: f64) {
    let (x_onto, _) = zone_forward(zones[onto], lam, phi_onto);
    let (x_from, _) = zone_forward(zones[from], lam, phi_from);
    zones[from].x0 += x_onto - x_from;
}

struct Igh {
    zones: [Zone; 12],
    dy0: f64,
}

impl Igh {
    fn new() -> Self {
        let dy0 = raw_y_gap();
        let zones = [
            lobe(true, -100.0, dy0),
            lobe(true, 30.0, dy0),
            lobe(false, -100.0, 0.0),
            lobe(false, 30.0, 0.0),
            lobe(false, -160.0, 0.0),
            lobe(false, -60.0, 0.0),
            lobe(false, 20.0, 0.0),
            lobe(false, 140.0, 0.0),
            lobe(true, -160.0, -dy0),
            lobe(true, -60.0, -dy0),
            lobe(true, 20.0, -dy0),
            lobe(true, 140.0, -dy0),
        ];
        Self { zones, dy0 }
    }

    fn pick_forward(lam: f64, phi: f64) -> usize {
        let d40 = deg(-40.0);
        if phi >= PHI_B {
            usize::from(lam > d40)
        } else if phi >= 0.0 {
            2 + usize::from(lam > d40)
        } else if phi >= -PHI_B {
            if lam <= deg(-100.0) {
                4
            } else if lam <= deg(-20.0) {
                5
            } else if lam <= deg(80.0) {
                6
            } else {
                7
            }
        } else if lam <= deg(-100.0) {
            8
        } else if lam <= deg(-20.0) {
            9
        } else if lam <= deg(80.0) {
            10
        } else {
            11
        }
    }
}

impl Kernel for Igh {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        Ok(zone_forward(
            self.zones[Self::pick_forward(lam, phi)],
            lam,
            phi,
        ))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let y90 = self.dy0 + 2.0_f64.sqrt();
        let d40 = deg(40.0);
        let zone = if y > y90 + EPS || y < -y90 + EPS {
            None
        } else if y >= PHI_B {
            Some(usize::from(x > -d40))
        } else if y >= 0.0 {
            Some(2 + usize::from(x > -d40))
        } else if y >= -PHI_B {
            Some(if x <= deg(-100.0) {
                4
            } else if x <= deg(-20.0) {
                5
            } else if x <= deg(80.0) {
                6
            } else {
                7
            })
        } else {
            Some(if x <= deg(-100.0) {
                8
            } else if x <= deg(-20.0) {
                9
            } else if x <= deg(80.0) {
                10
            } else {
                11
            })
        };
        let Some(index) = zone else {
            return Err(TransformError::OutsideDomain);
        };
        let (lam, phi) = zone_inverse(self.zones[index], x, y)?;
        let d180 = deg(180.0);
        let ok = match index {
            0 => {
                covers(lam, -d180, -d40)
                    || (covers(lam, -d40, deg(-10.0)) && covers(phi, deg(60.0), deg(90.0)))
            }
            1 => {
                covers(lam, -d40, d180)
                    || (covers(lam, -d180, deg(-160.0)) && covers(phi, deg(50.0), deg(90.0)))
                    || (covers(lam, deg(-50.0), -d40) && covers(phi, deg(60.0), deg(90.0)))
            }
            2 => covers(lam, -d180, -d40),
            3 => covers(lam, -d40, d180),
            4 | 8 => covers(lam, -d180, deg(-100.0)),
            5 | 9 => covers(lam, deg(-100.0), deg(-20.0)),
            6 | 10 => covers(lam, deg(-20.0), deg(80.0)),
            _ => covers(lam, deg(80.0), d180),
        };
        if ok {
            Ok((lam, phi))
        } else {
            Err(TransformError::OutsideDomain)
        }
    }
}

struct IghO {
    zones: [Zone; 12],
    dy0: f64,
}

impl IghO {
    fn new() -> Self {
        let dy0 = raw_y_gap();
        let zones = [
            lobe(true, -140.0, dy0),
            lobe(true, -10.0, dy0),
            lobe(true, 130.0, dy0),
            lobe(false, -140.0, 0.0),
            lobe(false, -10.0, 0.0),
            lobe(false, 130.0, 0.0),
            lobe(false, -110.0, 0.0),
            lobe(false, 20.0, 0.0),
            lobe(false, 150.0, 0.0),
            lobe(true, -110.0, -dy0),
            lobe(true, 20.0, -dy0),
            lobe(true, 150.0, -dy0),
        ];
        Self { zones, dy0 }
    }
}

impl Kernel for IghO {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let index = if phi >= PHI_B {
            if lam <= deg(-90.0) {
                0
            } else if lam >= deg(60.0) {
                2
            } else {
                1
            }
        } else if phi >= 0.0 {
            if lam <= deg(-90.0) {
                3
            } else if lam >= deg(60.0) {
                5
            } else {
                4
            }
        } else if phi >= -PHI_B {
            if lam <= deg(-60.0) {
                6
            } else if lam >= deg(90.0) {
                8
            } else {
                7
            }
        } else if lam <= deg(-60.0) {
            9
        } else if lam >= deg(90.0) {
            11
        } else {
            10
        };
        Ok(zone_forward(self.zones[index], lam, phi))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let y90 = self.dy0 + 2.0_f64.sqrt();
        let index = if y > y90 + EPS || y < -y90 + EPS {
            None
        } else if y >= PHI_B {
            Some(if x <= deg(-90.0) {
                0
            } else if x >= deg(60.0) {
                2
            } else {
                1
            })
        } else if y >= 0.0 {
            Some(if x <= deg(-90.0) {
                3
            } else if x >= deg(60.0) {
                5
            } else {
                4
            })
        } else if y >= -PHI_B {
            Some(if x <= deg(-60.0) {
                6
            } else if x >= deg(90.0) {
                8
            } else {
                7
            })
        } else {
            Some(if x <= deg(-60.0) {
                9
            } else if x >= deg(90.0) {
                11
            } else {
                10
            })
        };
        let Some(index) = index else {
            return Err(TransformError::OutsideDomain);
        };
        let (lam, phi) = zone_inverse(self.zones[index], x, y)?;
        let d180 = deg(180.0);
        let ok = match index {
            0 => {
                covers(lam, -d180, deg(-90.0))
                    || (covers(lam, deg(160.0), d180) && covers(phi, deg(50.0), deg(90.0)))
            }
            1 => covers(lam, deg(-90.0), deg(60.0)),
            2 => {
                covers(lam, deg(60.0), d180)
                    || (covers(lam, -d180, deg(-160.0)) && covers(phi, deg(50.0), deg(90.0)))
            }
            3 => covers(lam, -d180, deg(-90.0)),
            4 => covers(lam, deg(-90.0), deg(60.0)),
            5 => covers(lam, deg(60.0), d180),
            6 | 9 => covers(lam, -d180, deg(-60.0)),
            7 => covers(lam, deg(-60.0), deg(90.0)),
            8 | 11 => covers(lam, deg(90.0), d180),
            _ => {
                covers(lam, deg(-60.0), deg(90.0))
                    || (covers(lam, deg(90.0), deg(100.0)) && covers(phi, deg(-90.0), deg(-40.0)))
            }
        };
        if ok {
            Ok((lam, phi))
        } else {
            Err(TransformError::OutsideDomain)
        }
    }
}

struct Imoll {
    zones: [Zone; 6],
    boundary12: f64,
    boundary34: f64,
    boundary45: f64,
    boundary56: f64,
}

impl Imoll {
    fn new() -> Self {
        let mut zones = [
            lobe(true, -100.0, 0.0),
            lobe(true, 30.0, 0.0),
            lobe(true, -160.0, 0.0),
            lobe(true, -60.0, 0.0),
            lobe(true, 20.0, 0.0),
            lobe(true, 140.0, 0.0),
        ];
        shift_to(&mut zones, 2, 0, deg(-160.0), -EPS, EPS);
        shift_to(&mut zones, 1, 0, deg(-40.0), EPS, EPS);
        shift_to(&mut zones, 3, 0, deg(-100.0), -EPS, EPS);
        shift_to(&mut zones, 4, 1, deg(-20.0), -EPS, EPS);
        shift_to(&mut zones, 5, 1, deg(80.0), -EPS, EPS);
        let forward = |lam, phi| zone_forward(pick_imoll(&zones, lam, phi), lam, phi);
        let boundary12 = seam(&forward, deg(-40.0), EPS);
        let boundary34 = seam(&forward, deg(-100.0), -EPS);
        let boundary45 = seam(&forward, deg(-20.0), -EPS);
        let boundary56 = seam(&forward, deg(80.0), -EPS);
        Self {
            zones,
            boundary12,
            boundary34,
            boundary45,
            boundary56,
        }
    }
}

fn pick_imoll(zones: &[Zone; 6], lam: f64, phi: f64) -> Zone {
    let index = if phi >= 0.0 {
        usize::from(lam > deg(-40.0))
    } else if lam <= deg(-100.0) {
        2
    } else if lam <= deg(-20.0) {
        3
    } else if lam <= deg(80.0) {
        4
    } else {
        5
    };
    zones[index]
}

impl Kernel for Imoll {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        Ok(zone_forward(pick_imoll(&self.zones, lam, phi), lam, phi))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let y90 = 2.0_f64.sqrt();
        if y > y90 + EPS || y < -y90 + EPS {
            return Err(TransformError::OutsideDomain);
        }
        let index = if y >= 0.0 {
            usize::from(x > self.boundary12)
        } else if x <= self.boundary34 {
            2
        } else if x <= self.boundary45 {
            3
        } else if x <= self.boundary56 {
            4
        } else {
            5
        };
        let (lam, phi) = zone_inverse(self.zones[index], x, y)?;
        let d180 = deg(180.0);
        let ok = match index {
            0 => covers(lam, -d180, deg(-40.0)) && phi >= -EPS,
            1 => covers(lam, deg(-40.0), d180) && phi >= -EPS,
            2 => covers(lam, -d180, deg(-100.0)) && phi <= EPS,
            3 => covers(lam, deg(-100.0), deg(-20.0)) && phi <= EPS,
            4 => covers(lam, deg(-20.0), deg(80.0)) && phi <= EPS,
            _ => covers(lam, deg(80.0), d180) && phi <= EPS,
        };
        if ok {
            Ok((lam, phi))
        } else {
            Err(TransformError::OutsideDomain)
        }
    }
}

struct ImollO {
    zones: [Zone; 6],
    boundary12: f64,
    boundary23: f64,
    boundary45: f64,
    boundary56: f64,
}

impl ImollO {
    fn new() -> Self {
        let mut zones = [
            lobe(true, -140.0, 0.0),
            lobe(true, -10.0, 0.0),
            lobe(true, 130.0, 0.0),
            lobe(true, -110.0, 0.0),
            lobe(true, 20.0, 0.0),
            lobe(true, 150.0, 0.0),
        ];
        shift_to(&mut zones, 1, 0, deg(-90.0), EPS, EPS);
        shift_to(&mut zones, 2, 1, deg(60.0), EPS, EPS);
        shift_to(&mut zones, 3, 0, deg(-180.0), -EPS, EPS);
        shift_to(&mut zones, 4, 1, deg(-60.0), -EPS, EPS);
        shift_to(&mut zones, 5, 2, deg(90.0), -EPS, EPS);
        let forward = |lam, phi| zone_forward(pick_imoll_o(&zones, lam, phi), lam, phi);
        let boundary12 = seam(&forward, deg(-90.0), EPS);
        let boundary23 = seam(&forward, deg(60.0), EPS);
        let boundary45 = seam(&forward, deg(-60.0), -EPS);
        let boundary56 = seam(&forward, deg(90.0), -EPS);
        Self {
            zones,
            boundary12,
            boundary23,
            boundary45,
            boundary56,
        }
    }
}

fn pick_imoll_o(zones: &[Zone; 6], lam: f64, phi: f64) -> Zone {
    let index = if phi >= 0.0 {
        if lam <= deg(-90.0) {
            0
        } else if lam >= deg(60.0) {
            2
        } else {
            1
        }
    } else if lam <= deg(-60.0) {
        3
    } else if lam >= deg(90.0) {
        5
    } else {
        4
    };
    zones[index]
}

impl Kernel for ImollO {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        Ok(zone_forward(pick_imoll_o(&self.zones, lam, phi), lam, phi))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let y90 = 2.0_f64.sqrt();
        if y > y90 + EPS || y < -y90 + EPS {
            return Err(TransformError::OutsideDomain);
        }
        let index = if y >= 0.0 {
            if x <= self.boundary12 {
                0
            } else if x >= self.boundary23 {
                2
            } else {
                1
            }
        } else if x <= self.boundary45 {
            3
        } else if x >= self.boundary56 {
            5
        } else {
            4
        };
        let (lam, phi) = zone_inverse(self.zones[index], x, y)?;
        let d180 = deg(180.0);
        let ok = match index {
            0 => covers(lam, -d180, deg(-90.0)) && phi >= -EPS,
            1 => covers(lam, deg(-90.0), deg(60.0)) && phi >= -EPS,
            2 => covers(lam, deg(60.0), d180) && phi >= -EPS,
            3 => covers(lam, -d180, deg(-60.0)) && phi <= EPS,
            4 => covers(lam, deg(-60.0), deg(90.0)) && phi <= EPS,
            _ => covers(lam, deg(90.0), d180) && phi <= EPS,
        };
        if ok {
            Ok((lam, phi))
        } else {
            Err(TransformError::OutsideDomain)
        }
    }
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "igh" || name == "igh_o" || name == "imoll" || name == "imoll_o" {
        Some(&[])
    } else {
        None
    }
}

pub(crate) fn build(
    name: &str,
    _params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    let built = match name {
        "igh" => classic_step(frame, Igh::new()),
        "igh_o" => classic_step(frame, IghO::new()),
        "imoll" => classic_step(frame, Imoll::new()),
        "imoll_o" => classic_step(frame, ImollO::new()),
        _ => return Ok(None),
    };
    Ok(Some(built))
}
