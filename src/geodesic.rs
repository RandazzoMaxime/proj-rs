//! Karney's geodesic, the C implementation PROJ 9.9 vendors.
//!
//! The ellipsoid is the unit sphere scaled by the flattening only (`a = 1`).
//! Callers multiply the resulting lengths by the real semi-major axis.

use std::mem::size_of;

const DEG_TO_RAD: f64 = 0.017_453_292_519_943_296;

const GEOD_LATITUDE: u32 = 1 << 7;
const GEOD_LONGITUDE: u32 = (1 << 8) | (1 << 3);
const GEOD_DISTANCE_IN: u32 = (1 << 11) | (1 << 0) | (1 << 1);
const GEOD_REDUCEDLENGTH: u32 = (1 << 12) | (1 << 0) | (1 << 2);
const GEOD_GEODESICSCALE: u32 = (1 << 13) | (1 << 0) | (1 << 2);

/// Capabilities `gnom` asks of a geodesic line.
pub(crate) const GNOM_CAPS: u32 =
    GEOD_LATITUDE | GEOD_LONGITUDE | GEOD_DISTANCE_IN | GEOD_REDUCEDLENGTH | GEOD_GEODESICSCALE;

#[repr(C)]
pub(crate) struct Geodesic {
    a: f64,
    f: f64,
    f1: f64,
    e2: f64,
    ep2: f64,
    n: f64,
    b: f64,
    c2: f64,
    etol2: f64,
    a3x: [f64; 6],
    c3x: [f64; 15],
    c4x: [f64; 21],
}

#[repr(C)]
pub(crate) struct GeodesicLine {
    lat1: f64,
    lon1: f64,
    azi1: f64,
    a: f64,
    f: f64,
    salp1: f64,
    calp1: f64,
    a13: f64,
    s13: f64,
    b: f64,
    c2: f64,
    f1: f64,
    salp0: f64,
    calp0: f64,
    k2: f64,
    ssig1: f64,
    csig1: f64,
    dn1: f64,
    stau1: f64,
    ctau1: f64,
    somg1: f64,
    comg1: f64,
    a1m1: f64,
    a2m1: f64,
    a3c: f64,
    b11: f64,
    b21: f64,
    b31: f64,
    a4: f64,
    b41: f64,
    c1a: [f64; 7],
    c1pa: [f64; 7],
    c2a: [f64; 7],
    c3a: [f64; 6],
    c4a: [f64; 6],
    caps: u32,
}

const _: () = assert!(size_of::<Geodesic>() == 408);
const _: () = assert!(size_of::<GeodesicLine>() == 512);

extern "C" {
    fn geod_init(g: *mut Geodesic, a: f64, f: f64);
    fn geod_inverse(
        g: *const Geodesic,
        lat1: f64,
        lon1: f64,
        lat2: f64,
        lon2: f64,
        s12: *mut f64,
        azi1: *mut f64,
        azi2: *mut f64,
    );
    fn geod_direct(
        g: *const Geodesic,
        lat1: f64,
        lon1: f64,
        azi1: f64,
        s12: f64,
        lat2: *mut f64,
        lon2: *mut f64,
        azi2: *mut f64,
    );
    fn geod_geninverse(
        g: *const Geodesic,
        lat1: f64,
        lon1: f64,
        lat2: f64,
        lon2: f64,
        s12: *mut f64,
        azi1: *mut f64,
        azi2: *mut f64,
        m12: *mut f64,
        big_m12: *mut f64,
        big_m21: *mut f64,
        area: *mut f64,
    ) -> f64;
    fn geod_lineinit(
        line: *mut GeodesicLine,
        g: *const Geodesic,
        lat1: f64,
        lon1: f64,
        azi1: f64,
        caps: u32,
    );
    fn geod_genposition(
        line: *const GeodesicLine,
        flags: u32,
        s12_a12: f64,
        lat2: *mut f64,
        lon2: *mut f64,
        azi2: *mut f64,
        s12: *mut f64,
        m12: *mut f64,
        big_m12: *mut f64,
        big_m21: *mut f64,
        area: *mut f64,
    ) -> f64;
}

impl Geodesic {
    /// Unit ellipsoid of flattening `f`, matching `geod_init(&g, 1, f)`.
    pub(crate) fn unit(f: f64) -> Self {
        let mut g = Self::zero();
        // SAFETY: `g` is a complete `Geodesic` and `geod_init` writes every field.
        unsafe { geod_init(&mut g, 1.0, f) };
        g
    }

    fn zero() -> Self {
        Self {
            a: 0.0,
            f: 0.0,
            f1: 0.0,
            e2: 0.0,
            ep2: 0.0,
            n: 0.0,
            b: 0.0,
            c2: 0.0,
            etol2: 0.0,
            a3x: [0.0; 6],
            c3x: [0.0; 15],
            c4x: [0.0; 21],
        }
    }

    /// Distance and forward azimuth, in metres on this ellipsoid and degrees.
    pub(crate) fn inverse(&self, lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> (f64, f64) {
        let mut distance = 0.0;
        let mut azimuth = 0.0;
        // SAFETY: `self` was initialized and the output pointers are live locals.
        unsafe {
            geod_inverse(
                self,
                lat1,
                lon1,
                lat2,
                lon2,
                &mut distance,
                &mut azimuth,
                std::ptr::null_mut(),
            );
        }
        (distance, azimuth)
    }

    /// Point reached by travelling `distance` along `azimuth` degrees.
    pub(crate) fn direct(&self, lat1: f64, lon1: f64, azimuth: f64, distance: f64) -> (f64, f64) {
        let mut lat2 = 0.0;
        let mut lon2 = 0.0;
        // SAFETY: `self` was initialized and the output pointers are live locals.
        unsafe {
            geod_direct(
                self,
                lat1,
                lon1,
                azimuth,
                distance,
                &mut lat2,
                &mut lon2,
                std::ptr::null_mut(),
            );
        }
        (lat2, lon2)
    }

    /// Forward azimuth in degrees, reduced length `m12`, and scale `M12`.
    pub(crate) fn inverse_scale(
        &self,
        lat1: f64,
        lon1: f64,
        lat2: f64,
        lon2: f64,
    ) -> (f64, f64, f64) {
        let mut azimuth = 0.0;
        let mut reduced = 0.0;
        let mut scale = 0.0;
        // SAFETY: `self` was initialized and the output pointers are live locals.
        unsafe {
            geod_geninverse(
                self,
                lat1,
                lon1,
                lat2,
                lon2,
                std::ptr::null_mut(),
                &mut azimuth,
                std::ptr::null_mut(),
                &mut reduced,
                &mut scale,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
        }
        (azimuth, reduced, scale)
    }
}

impl GeodesicLine {
    pub(crate) fn from_azimuth(earth: &Geodesic, lat1: f64, lon1: f64, azimuth: f64) -> Self {
        let mut line = Self::zero();
        // SAFETY: both structs match the C layout and `geod_lineinit` fills the line.
        unsafe { geod_lineinit(&mut line, earth, lat1, lon1, azimuth, GNOM_CAPS) };
        line
    }

    fn zero() -> Self {
        Self {
            lat1: 0.0,
            lon1: 0.0,
            azi1: 0.0,
            a: 0.0,
            f: 0.0,
            salp1: 0.0,
            calp1: 0.0,
            a13: 0.0,
            s13: 0.0,
            b: 0.0,
            c2: 0.0,
            f1: 0.0,
            salp0: 0.0,
            calp0: 0.0,
            k2: 0.0,
            ssig1: 0.0,
            csig1: 0.0,
            dn1: 0.0,
            stau1: 0.0,
            ctau1: 0.0,
            somg1: 0.0,
            comg1: 0.0,
            a1m1: 0.0,
            a2m1: 0.0,
            a3c: 0.0,
            b11: 0.0,
            b21: 0.0,
            b31: 0.0,
            a4: 0.0,
            b41: 0.0,
            c1a: [0.0; 7],
            c1pa: [0.0; 7],
            c2a: [0.0; 7],
            c3a: [0.0; 6],
            c4a: [0.0; 6],
            caps: 0,
        }
    }

    /// Position at distance `distance`. Returns latitude, longitude, the
    /// updated distance, reduced length, and scale.
    pub(crate) fn position(&self, distance: f64) -> (f64, f64, f64, f64, f64) {
        let mut lat = 0.0;
        let mut lon = 0.0;
        let mut along = distance;
        let mut reduced = 0.0;
        let mut scale = 0.0;
        // SAFETY: `self` was initialized with the gnomonic capability mask.
        unsafe {
            geod_genposition(
                self,
                0,
                distance,
                &mut lat,
                &mut lon,
                std::ptr::null_mut(),
                &mut along,
                &mut reduced,
                &mut scale,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
        }
        (lat, lon, along, reduced, scale)
    }
}

/// Radians to degrees with PROJ's `DEG_TO_RAD`.
pub(crate) fn degrees(radians: f64) -> f64 {
    radians / DEG_TO_RAD
}

/// Degrees to radians with PROJ's `DEG_TO_RAD`.
pub(crate) fn radians(degrees: f64) -> f64 {
    degrees * DEG_TO_RAD
}
