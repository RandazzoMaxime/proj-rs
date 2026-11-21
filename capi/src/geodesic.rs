//! The geodesic API of PROJ, `geodesic.h`: the `geod_*` functions over the Rust port of
//! `geodesic.c` (`proj_rs::geodesic`). The caller allocates the structures; each call reads them in
//! their C layout, computes, and writes back the ones it changes. An output pointer may be NULL.

use std::ffi::{c_int, c_uint};

use proj_rs::geodesic::c::{GeodGeodesic, GeodGeodesicLine, GeodPolygon};
use proj_rs::geodesic::{
    AreaRange, Geodesic, GeodesicLine, Orientation, Polygon, PolygonKind, Position, GEOD_AREA,
    GEOD_AZIMUTH, GEOD_DISTANCE, GEOD_GEODESICSCALE, GEOD_LATITUDE, GEOD_LONGITUDE,
    GEOD_REDUCEDLENGTH,
};

use crate::util::{deref, deref_mut, guard, put, slice};

fn geodesic(g: *const GeodGeodesic) -> Option<Geodesic> {
    deref(g).map(Geodesic::from)
}

fn line(l: *const GeodGeodesicLine) -> Option<GeodesicLine> {
    deref(l).map(GeodesicLine::from)
}

fn store_line(l: *mut GeodGeodesicLine, value: &GeodesicLine) {
    if let Some(target) = deref_mut(l) {
        *target = GeodGeodesicLine::from(value);
    }
}

fn polygon(p: *const GeodPolygon) -> Option<Polygon> {
    deref(p).map(Polygon::from)
}

fn store_polygon(p: *mut GeodPolygon, value: &Polygon) {
    if let Some(target) = deref_mut(p) {
        *target = GeodPolygon::from(value);
    }
}

fn orientation(reverse: c_int) -> Orientation {
    if reverse != 0 {
        Orientation::Clockwise
    } else {
        Orientation::CounterClockwise
    }
}

fn range(sign: c_int) -> AreaRange {
    if sign != 0 {
        AreaRange::Signed
    } else {
        AreaRange::Unsigned
    }
}

/// The output pointers of `geod_genposition` and `geod_gendirect`.
struct Outputs {
    lat2: *mut f64,
    lon2: *mut f64,
    azi2: *mut f64,
    s12: *mut f64,
    m12: *mut f64,
    big_m12: *mut f64,
    big_m21: *mut f64,
    big_s12: *mut f64,
}

impl Outputs {
    /// The outputs asked for, as `geodesic.c` builds `outmask`: one per pointer that is not NULL.
    fn wanted(&self) -> u32 {
        let mut want = 0;
        for (pointer, mask) in [
            (self.lat2, GEOD_LATITUDE),
            (self.lon2, GEOD_LONGITUDE),
            (self.azi2, GEOD_AZIMUTH),
            (self.s12, GEOD_DISTANCE),
            (self.m12, GEOD_REDUCEDLENGTH),
            (self.big_s12, GEOD_AREA),
        ] {
            if !pointer.is_null() {
                want |= mask;
            }
        }
        if !self.big_m12.is_null() || !self.big_m21.is_null() {
            want |= GEOD_GEODESICSCALE;
        }
        want
    }

    /// Write what was computed; an output the line could not give is left untouched, as in C.
    fn write(&self, position: &Position) {
        let done = position.computed;
        let pairs = [
            (GEOD_LATITUDE, self.lat2, position.lat2),
            (GEOD_LONGITUDE, self.lon2, position.lon2),
            (GEOD_AZIMUTH, self.azi2, position.azi2),
            (GEOD_DISTANCE, self.s12, position.s12),
            (GEOD_REDUCEDLENGTH, self.m12, position.m12),
            (GEOD_GEODESICSCALE, self.big_m12, position.big_m12),
            (GEOD_GEODESICSCALE, self.big_m21, position.big_m21),
            (GEOD_AREA, self.big_s12, position.big_s12),
        ];
        for (mask, pointer, value) in pairs {
            if done & mask & 0x7F80 != 0 {
                put(pointer, value);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn geod_init(g: *mut GeodGeodesic, a: f64, f: f64) {
    guard((), || {
        if let Some(target) = deref_mut(g) {
            *target = GeodGeodesic::from(&Geodesic::new(a, f));
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn geod_direct(
    g: *const GeodGeodesic,
    lat1: f64,
    lon1: f64,
    azi1: f64,
    s12: f64,
    plat2: *mut f64,
    plon2: *mut f64,
    pazi2: *mut f64,
) {
    let null = std::ptr::null_mut();
    geod_gendirect(
        g, lat1, lon1, azi1, 0, s12, plat2, plon2, pazi2, null, null, null, null, null,
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn geod_gendirect(
    g: *const GeodGeodesic,
    lat1: f64,
    lon1: f64,
    azi1: f64,
    flags: c_uint,
    s12_a12: f64,
    plat2: *mut f64,
    plon2: *mut f64,
    pazi2: *mut f64,
    ps12: *mut f64,
    pm12: *mut f64,
    p_big_m12: *mut f64,
    p_big_m21: *mut f64,
    p_big_s12: *mut f64,
) -> f64 {
    guard(f64::NAN, || {
        let Some(g) = geodesic(g) else {
            return f64::NAN;
        };
        let outputs = Outputs {
            lat2: plat2,
            lon2: plon2,
            azi2: pazi2,
            s12: ps12,
            m12: pm12,
            big_m12: p_big_m12,
            big_m21: p_big_m21,
            big_s12: p_big_s12,
        };
        let position = g.gen_direct(lat1, lon1, azi1, flags, s12_a12, outputs.wanted());
        outputs.write(&position);
        position.a12
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn geod_inverse(
    g: *const GeodGeodesic,
    lat1: f64,
    lon1: f64,
    lat2: f64,
    lon2: f64,
    ps12: *mut f64,
    pazi1: *mut f64,
    pazi2: *mut f64,
) {
    let null = std::ptr::null_mut();
    geod_geninverse(
        g, lat1, lon1, lat2, lon2, ps12, pazi1, pazi2, null, null, null, null,
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn geod_geninverse(
    g: *const GeodGeodesic,
    lat1: f64,
    lon1: f64,
    lat2: f64,
    lon2: f64,
    ps12: *mut f64,
    pazi1: *mut f64,
    pazi2: *mut f64,
    pm12: *mut f64,
    p_big_m12: *mut f64,
    p_big_m21: *mut f64,
    p_big_s12: *mut f64,
) -> f64 {
    guard(f64::NAN, || {
        let Some(g) = geodesic(g) else {
            return f64::NAN;
        };
        let mut want = 0;
        for (pointer, mask) in [
            (ps12, GEOD_DISTANCE),
            (pm12, GEOD_REDUCEDLENGTH),
            (p_big_s12, GEOD_AREA),
        ] {
            if !pointer.is_null() {
                want |= mask;
            }
        }
        if !p_big_m12.is_null() || !p_big_m21.is_null() {
            want |= GEOD_GEODESICSCALE;
        }
        let solution = g.gen_inverse(lat1, lon1, lat2, lon2, want);
        put(ps12, solution.s12);
        put(pazi1, solution.azi1);
        put(pazi2, solution.azi2);
        put(pm12, solution.m12);
        put(p_big_m12, solution.big_m12);
        put(p_big_m21, solution.big_m21);
        put(p_big_s12, solution.big_s12);
        solution.a12
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn geod_lineinit(
    l: *mut GeodGeodesicLine,
    g: *const GeodGeodesic,
    lat1: f64,
    lon1: f64,
    azi1: f64,
    caps: c_uint,
) {
    guard((), || {
        if let Some(g) = geodesic(g) {
            store_line(l, &GeodesicLine::from_start(&g, lat1, lon1, azi1, caps));
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn geod_directline(
    l: *mut GeodGeodesicLine,
    g: *const GeodGeodesic,
    lat1: f64,
    lon1: f64,
    azi1: f64,
    s12: f64,
    caps: c_uint,
) {
    geod_gendirectline(l, g, lat1, lon1, azi1, 0, s12, caps);
}

#[unsafe(no_mangle)]
pub extern "C" fn geod_gendirectline(
    l: *mut GeodGeodesicLine,
    g: *const GeodGeodesic,
    lat1: f64,
    lon1: f64,
    azi1: f64,
    flags: c_uint,
    s12_a12: f64,
    caps: c_uint,
) {
    guard((), || {
        if let Some(g) = geodesic(g) {
            store_line(
                l,
                &g.gen_direct_line(lat1, lon1, azi1, flags, s12_a12, caps),
            );
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn geod_inverseline(
    l: *mut GeodGeodesicLine,
    g: *const GeodGeodesic,
    lat1: f64,
    lon1: f64,
    lat2: f64,
    lon2: f64,
    caps: c_uint,
) {
    guard((), || {
        if let Some(g) = geodesic(g) {
            store_line(l, &g.inverse_line(lat1, lon1, lat2, lon2, caps));
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn geod_position(
    l: *const GeodGeodesicLine,
    s12: f64,
    plat2: *mut f64,
    plon2: *mut f64,
    pazi2: *mut f64,
) {
    let null = std::ptr::null_mut();
    geod_genposition(l, 0, s12, plat2, plon2, pazi2, null, null, null, null, null);
}

#[unsafe(no_mangle)]
pub extern "C" fn geod_genposition(
    l: *const GeodGeodesicLine,
    flags: c_uint,
    s12_a12: f64,
    plat2: *mut f64,
    plon2: *mut f64,
    pazi2: *mut f64,
    ps12: *mut f64,
    pm12: *mut f64,
    p_big_m12: *mut f64,
    p_big_m21: *mut f64,
    p_big_s12: *mut f64,
) -> f64 {
    guard(f64::NAN, || {
        let Some(line) = line(l) else {
            return f64::NAN;
        };
        let outputs = Outputs {
            lat2: plat2,
            lon2: plon2,
            azi2: pazi2,
            s12: ps12,
            m12: pm12,
            big_m12: p_big_m12,
            big_m21: p_big_m21,
            big_s12: p_big_s12,
        };
        let position = line.gen_position(flags, s12_a12, outputs.wanted());
        outputs.write(&position);
        position.a12
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn geod_setdistance(l: *mut GeodGeodesicLine, s13: f64) {
    guard((), || {
        if let Some(mut line) = line(l) {
            line.set_distance(s13);
            store_line(l, &line);
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn geod_gensetdistance(l: *mut GeodGeodesicLine, flags: c_uint, s13_a13: f64) {
    guard((), || {
        if let Some(mut line) = line(l) {
            line.gen_set_distance(flags, s13_a13);
            store_line(l, &line);
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn geod_polygon_init(p: *mut GeodPolygon, polylinep: c_int) {
    guard((), || {
        let kind = if polylinep != 0 {
            PolygonKind::Polyline
        } else {
            PolygonKind::Polygon
        };
        store_polygon(p, &Polygon::new(kind));
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn geod_polygon_clear(p: *mut GeodPolygon) {
    guard((), || {
        if let Some(mut polygon) = polygon(p) {
            polygon.clear();
            store_polygon(p, &polygon);
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn geod_polygon_addpoint(
    g: *const GeodGeodesic,
    p: *mut GeodPolygon,
    lat: f64,
    lon: f64,
) {
    guard((), || {
        if let (Some(g), Some(mut polygon)) = (geodesic(g), polygon(p)) {
            polygon.add_point(&g, lat, lon);
            store_polygon(p, &polygon);
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn geod_polygon_addedge(
    g: *const GeodGeodesic,
    p: *mut GeodPolygon,
    azi: f64,
    s: f64,
) {
    guard((), || {
        if let (Some(g), Some(mut polygon)) = (geodesic(g), polygon(p)) {
            polygon.add_edge(&g, azi, s);
            store_polygon(p, &polygon);
        }
    });
}

/// The area (not for a polyline) and the perimeter, as `geodesic.c` writes them.
fn write_totals(totals: &proj_rs::geodesic::PolygonTotals, pa: *mut f64, pp: *mut f64) -> c_uint {
    if let Some(area) = totals.area {
        put(pa, area);
    }
    put(pp, totals.perimeter);
    totals.num
}

#[unsafe(no_mangle)]
pub extern "C" fn geod_polygon_compute(
    g: *const GeodGeodesic,
    p: *const GeodPolygon,
    reverse: c_int,
    sign: c_int,
    pa: *mut f64,
    pp: *mut f64,
) -> c_uint {
    guard(0, || match (geodesic(g), polygon(p)) {
        (Some(g), Some(polygon)) => write_totals(
            &polygon.compute(&g, orientation(reverse), range(sign)),
            pa,
            pp,
        ),
        _ => 0,
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn geod_polygon_testpoint(
    g: *const GeodGeodesic,
    p: *const GeodPolygon,
    lat: f64,
    lon: f64,
    reverse: c_int,
    sign: c_int,
    pa: *mut f64,
    pp: *mut f64,
) -> c_uint {
    guard(0, || match (geodesic(g), polygon(p)) {
        (Some(g), Some(polygon)) => write_totals(
            &polygon.test_point(&g, lat, lon, orientation(reverse), range(sign)),
            pa,
            pp,
        ),
        _ => 0,
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn geod_polygon_testedge(
    g: *const GeodGeodesic,
    p: *const GeodPolygon,
    azi: f64,
    s: f64,
    reverse: c_int,
    sign: c_int,
    pa: *mut f64,
    pp: *mut f64,
) -> c_uint {
    guard(0, || match (geodesic(g), polygon(p)) {
        (Some(g), Some(polygon)) => write_totals(
            &polygon.test_edge(&g, azi, s, orientation(reverse), range(sign)),
            pa,
            pp,
        ),
        _ => 0,
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn geod_polygonarea(
    g: *const GeodGeodesic,
    lats: *const f64,
    lons: *const f64,
    n: c_int,
    pa: *mut f64,
    pp: *mut f64,
) {
    guard((), || {
        let Some(g) = geodesic(g) else {
            return;
        };
        let count = usize::try_from(n).unwrap_or(0);
        let points: Vec<(f64, f64)> = slice(lats, count)
            .iter()
            .zip(slice(lons, count))
            .map(|(&lat, &lon)| (lat, lon))
            .collect();
        let (area, perimeter) = g.polygon_area(&points);
        put(pa, area);
        put(pp, perimeter);
    });
}
