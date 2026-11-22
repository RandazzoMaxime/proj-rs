//! The structures of PROJ's `geodesic.h`, field for field, for the C library (`capi/`).
//!
//! A C caller allocates `struct geod_geodesic`, `struct geod_geodesicline` and
//! `struct geod_polygon` itself and reads their public fields (`a`, `f`, `lat1`, `caps`, `num`...),
//! so the layout must be the one of `geodesic.h` 9.9.0. Each type converts to and from the Rust type
//! that computes, without loss: every field of the C structure is a field of the Rust one.

use super::{Geodesic, GeodesicLine, Polygon, NC, NC3X, NC4X, N_SERIES};

/// `struct geod_geodesic`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GeodGeodesic {
    pub a: f64,
    pub f: f64,
    pub f1: f64,
    pub e2: f64,
    pub ep2: f64,
    pub n: f64,
    pub b: f64,
    pub c2: f64,
    pub etol2: f64,
    pub a3x: [f64; N_SERIES],
    pub c3x: [f64; NC3X],
    pub c4x: [f64; NC4X],
}

/// `struct geod_geodesicline`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GeodGeodesicLine {
    pub lat1: f64,
    pub lon1: f64,
    pub azi1: f64,
    pub a: f64,
    pub f: f64,
    pub salp1: f64,
    pub calp1: f64,
    pub a13: f64,
    pub s13: f64,
    pub b: f64,
    pub c2: f64,
    pub f1: f64,
    pub salp0: f64,
    pub calp0: f64,
    pub k2: f64,
    pub ssig1: f64,
    pub csig1: f64,
    pub dn1: f64,
    pub stau1: f64,
    pub ctau1: f64,
    pub somg1: f64,
    pub comg1: f64,
    pub a1m1: f64,
    pub a2m1: f64,
    pub a3c: f64,
    pub b11: f64,
    pub b21: f64,
    pub b31: f64,
    pub a4: f64,
    pub b41: f64,
    pub c1a: [f64; NC],
    pub c1pa: [f64; NC],
    pub c2a: [f64; NC],
    pub c3a: [f64; N_SERIES],
    pub c4a: [f64; N_SERIES],
    pub caps: u32,
}

/// `struct geod_polygon`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GeodPolygon {
    pub lat: f64,
    pub lon: f64,
    pub lat0: f64,
    pub lon0: f64,
    pub area: [f64; 2],
    pub perimeter: [f64; 2],
    pub polyline: i32,
    pub crossings: i32,
    pub num: u32,
}

impl From<&Geodesic> for GeodGeodesic {
    fn from(g: &Geodesic) -> Self {
        Self {
            a: g.a,
            f: g.f,
            f1: g.f1,
            e2: g.e2,
            ep2: g.ep2,
            n: g.n,
            b: g.b,
            c2: g.c2,
            etol2: g.etol2,
            a3x: g.a3x,
            c3x: g.c3x,
            c4x: g.c4x,
        }
    }
}

impl From<&GeodGeodesic> for Geodesic {
    fn from(g: &GeodGeodesic) -> Self {
        Self {
            a: g.a,
            f: g.f,
            f1: g.f1,
            ep2: g.ep2,
            n: g.n,
            b: g.b,
            e2: g.e2,
            c2: g.c2,
            etol2: g.etol2,
            a3x: g.a3x,
            c3x: g.c3x,
            c4x: g.c4x,
        }
    }
}

impl From<&GeodesicLine> for GeodGeodesicLine {
    fn from(l: &GeodesicLine) -> Self {
        Self {
            lat1: l.lat1,
            lon1: l.lon1,
            azi1: l.azi1,
            a: l.a,
            f: l.f,
            salp1: l.salp1,
            calp1: l.calp1,
            a13: l.a13,
            s13: l.s13,
            b: l.b,
            c2: l.c2,
            f1: l.f1,
            salp0: l.salp0,
            calp0: l.calp0,
            k2: l.k2,
            ssig1: l.ssig1,
            csig1: l.csig1,
            dn1: l.dn1,
            stau1: l.stau1,
            ctau1: l.ctau1,
            somg1: l.somg1,
            comg1: l.comg1,
            a1m1: l.a1m1,
            a2m1: l.a2m1,
            a3c: l.a3c,
            b11: l.b11,
            b21: l.b21,
            b31: l.b31,
            a4: l.a4,
            b41: l.b41,
            c1a: l.c1a,
            c1pa: l.c1pa,
            c2a: l.c2a,
            c3a: l.c3a,
            c4a: l.c4a,
            caps: l.caps,
        }
    }
}

impl From<&GeodGeodesicLine> for GeodesicLine {
    fn from(l: &GeodGeodesicLine) -> Self {
        Self {
            lat1: l.lat1,
            lon1: l.lon1,
            azi1: l.azi1,
            salp1: l.salp1,
            calp1: l.calp1,
            a: l.a,
            b: l.b,
            c2: l.c2,
            f: l.f,
            f1: l.f1,
            salp0: l.salp0,
            calp0: l.calp0,
            k2: l.k2,
            ssig1: l.ssig1,
            csig1: l.csig1,
            dn1: l.dn1,
            stau1: l.stau1,
            ctau1: l.ctau1,
            somg1: l.somg1,
            comg1: l.comg1,
            a1m1: l.a1m1,
            a2m1: l.a2m1,
            a3c: l.a3c,
            b11: l.b11,
            b21: l.b21,
            b31: l.b31,
            c1a: l.c1a,
            c1pa: l.c1pa,
            c2a: l.c2a,
            c3a: l.c3a,
            a4: l.a4,
            b41: l.b41,
            c4a: l.c4a,
            a13: l.a13,
            s13: l.s13,
            caps: l.caps,
        }
    }
}

impl From<&Polygon> for GeodPolygon {
    fn from(p: &Polygon) -> Self {
        Self {
            lat: p.lat,
            lon: p.lon,
            lat0: p.lat0,
            lon0: p.lon0,
            area: p.area,
            perimeter: p.perimeter,
            polyline: i32::from(p.polyline),
            crossings: p.crossings,
            num: p.num,
        }
    }
}

impl From<&GeodPolygon> for Polygon {
    fn from(p: &GeodPolygon) -> Self {
        Self {
            polyline: p.polyline != 0,
            lat0: p.lat0,
            lon0: p.lon0,
            lat: p.lat,
            lon: p.lon,
            perimeter: p.perimeter,
            area: p.area,
            num: p.num,
            crossings: p.crossings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geodesic::{AreaRange, Orientation, PolygonKind};

    /// `GEOD_ALL` of geodesic.h.
    const ALL: u32 = 0x7F80 | 0x1F;

    #[test]
    fn the_sizes_are_those_of_geodesic_h() {
        // 9 + 6 + 15 + 21 doubles; 30 + 7*3 + 6*2 doubles and an unsigned; 8 doubles and 3 ints.
        assert_eq!(std::mem::size_of::<GeodGeodesic>(), 51 * 8);
        assert_eq!(std::mem::size_of::<GeodGeodesicLine>(), 63 * 8 + 8);
        assert_eq!(std::mem::size_of::<GeodPolygon>(), 8 * 8 + 16);
    }

    #[test]
    fn a_round_trip_through_the_c_layout_changes_no_result() {
        let g = Geodesic::wgs84();
        let back = Geodesic::from(&GeodGeodesic::from(&g));
        assert_eq!(
            g.gen_inverse(10.0, 20.0, -30.0, 140.0, ALL),
            back.gen_inverse(10.0, 20.0, -30.0, 140.0, ALL)
        );
        let line = g.inverse_line(10.0, 20.0, -30.0, 140.0, ALL);
        let line_back = GeodesicLine::from(&GeodGeodesicLine::from(&line));
        assert_eq!(
            line.gen_position(0, 1.0e6, ALL),
            line_back.gen_position(0, 1.0e6, ALL)
        );
        let mut polygon = Polygon::new(PolygonKind::Polygon);
        for (lat, lon) in [(0.0, 0.0), (0.0, 10.0), (10.0, 10.0)] {
            polygon.add_point(&g, lat, lon);
        }
        let polygon_back = Polygon::from(&GeodPolygon::from(&polygon));
        assert_eq!(
            polygon.compute(&g, Orientation::CounterClockwise, AreaRange::Signed),
            polygon_back.compute(&g, Orientation::CounterClockwise, AreaRange::Signed)
        );
    }
}
