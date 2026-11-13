//! The lists that `proj -l`, `geod -l` and `cs2cs -l` print: PROJ's projections, its
//! ellipsoids (`proj_list_operations`, `proj_list_ellps`) and the linear units of proj.db
//! (`proj_get_units_from_database`).

use crate::projdb::with_db;

/// One projection of PROJ's list: its id and its description, the first line of which is its
/// name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjectionEntry {
    /// The id of the projection, as `+proj=` names it.
    pub id: &'static str,
    /// The description: a name, then lines of parameters, each starting with a tab.
    pub description: &'static str,
}

/// One ellipsoid of PROJ's list: its id, its `a=` and `rf=` (or `b=`) text, and its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EllipsoidEntry {
    /// The id of the ellipsoid, as `+ellps=` names it.
    pub id: &'static str,
    /// The semi-major axis, as `a=` text.
    pub major: &'static str,
    /// The flattening or the semi-minor axis, as `rf=` or `b=` text.
    pub ell: &'static str,
    /// The name of the ellipsoid.
    pub name: &'static str,
}

/// One linear unit of proj.db, as `proj_get_units_from_database` lists it.
#[derive(Clone, Debug, PartialEq)]
pub struct LinearUnit {
    /// The PROJ short name, as `+units=` names it.
    pub short_name: String,
    /// The factor to metres.
    pub conv_factor: f64,
    /// The name of the unit.
    pub name: String,
}

/// The linear units of proj.db that are not deprecated and have a PROJ short name, in the order
/// of `unit_of_measure` by authority and code (`getUnitList`, `c_api.cpp`). `None` when proj.db
/// cannot be read.
#[must_use]
pub fn linear_units() -> Option<Vec<LinearUnit>> {
    with_db(|connection| {
        let mut statement = connection.prepare(
            "SELECT name, type, COALESCE(conv_factor, 0), COALESCE(proj_short_name, ''), \
             CAST(deprecated AS TEXT) FROM unit_of_measure ORDER BY auth_name, code",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;
        let mut units = Vec::new();
        for row in rows {
            let (name, kind, conv_factor, short_name, deprecated) = row?;
            // `getUnitList` gives a length per time the category `linear_per_time`.
            let linear = kind == "length" && !name.contains(" per ");
            if !linear || deprecated == "1" || short_name.is_empty() {
                continue;
            }
            units.push(LinearUnit {
                short_name,
                conv_factor,
                name,
            });
        }
        Ok(units)
    })
    .ok()
}

// Generated from PROJ 9.9.0 `src/pj_list.h`, `src/projections/*.cpp` (PROJ_HEAD) and
// `src/ellps.cpp`. Do not edit by hand.

/// One projection of PROJ's list: its id and its description, the first line of which is
/// its name (`proj_list_operations`).
pub const PROJECTIONS: &[ProjectionEntry] = &[
    ProjectionEntry {
        id: "adams_hemi",
        description: "Adams Hemisphere in a Square\n\tMisc Sph No inv",
    },
    ProjectionEntry {
        id: "adams_ws1",
        description: "Adams World in a Square I\n\tMisc Sph No inv",
    },
    ProjectionEntry {
        id: "adams_ws2",
        description: "Adams World in a Square II\n\tMisc Sph No inv",
    },
    ProjectionEntry {
        id: "aea",
        description: "Albers Equal Area\n\tConic Sph&Ell\n\tlat_1= lat_2=",
    },
    ProjectionEntry {
        id: "aeqd",
        description: "Azimuthal Equidistant\n\tAzi, Sph&Ell\n\tlat_0 guam",
    },
    ProjectionEntry {
        id: "affine",
        description: "Affine transformation",
    },
    ProjectionEntry {
        id: "airy",
        description: "Airy\n\tMisc Sph, no inv\n\tno_cut lat_b=",
    },
    ProjectionEntry {
        id: "aitoff",
        description: "Aitoff\n\tMisc Sph",
    },
    ProjectionEntry {
        id: "alsk",
        description: "Modified Stereographic of Alaska\n\tAzi(mod)",
    },
    ProjectionEntry {
        id: "apian",
        description: "Apian Globular I\n\tMisc Sph, no inv",
    },
    ProjectionEntry {
        id: "august",
        description: "August Epicycloidal\n\tMisc Sph, no inv",
    },
    ProjectionEntry {
        id: "axisswap",
        description: "Axis ordering",
    },
    ProjectionEntry {
        id: "bacon",
        description: "Bacon Globular\n\tMisc Sph, no inv",
    },
    ProjectionEntry {
        id: "bertin1953",
        description: "Bertin 1953\n\tMisc Sph no inv.",
    },
    ProjectionEntry {
        id: "bipc",
        description: "Bipolar conic of western hemisphere\n\tConic Sph",
    },
    ProjectionEntry {
        id: "boggs",
        description: "Boggs Eumorphic\n\tPCyl, no inv, Sph",
    },
    ProjectionEntry {
        id: "bonne",
        description: "Bonne (Werner lat_1=90)\n\tConic Sph&Ell\n\tlat_1=",
    },
    ProjectionEntry {
        id: "calcofi",
        description: "Cal Coop Ocean Fish Invest Lines/Stations\n\tCyl, Sph&Ell",
    },
    ProjectionEntry {
        id: "cart",
        description: "Geodetic/cartesian conversions",
    },
    ProjectionEntry {
        id: "cass",
        description: "Cassini\n\tCyl, Sph&Ell",
    },
    ProjectionEntry {
        id: "cc",
        description: "Central Cylindrical\n\tCyl, Sph",
    },
    ProjectionEntry {
        id: "ccon",
        description: "Central Conic\n\tCentral Conic, Sph\n\tlat_1=",
    },
    ProjectionEntry {
        id: "cea",
        description: "Equal Area Cylindrical\n\tCyl, Sph&Ell\n\tlat_ts=",
    },
    ProjectionEntry {
        id: "chamb",
        description: "Chamberlin Trimetric\n\tMisc Sph, no inv\n\tlat_1= lon_1= lat_2= lon_2= lat_3= lon_3=",
    },
    ProjectionEntry {
        id: "collg",
        description: "Collignon\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "col_urban",
        description: "Colombia Urban\n\tMisc\n\th_0=",
    },
    ProjectionEntry {
        id: "comill",
        description: "Compact Miller\n\tCyl, Sph",
    },
    ProjectionEntry {
        id: "crast",
        description: "Craster Parabolic (Putnins P4)\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "defmodel",
        description: "Deformation model",
    },
    ProjectionEntry {
        id: "deformation",
        description: "Kinematic grid shift",
    },
    ProjectionEntry {
        id: "denoy",
        description: "Denoyer Semi-Elliptical\n\tPCyl, no inv, Sph",
    },
    ProjectionEntry {
        id: "dsea",
        description: "Dodecahedral Snyder Equal Area\n\tSph&Ell\n\tnet= dual orient_lat= orient_lon= azi= lat_0= lon_0=",
    },
    ProjectionEntry {
        id: "airocean",
        description: "Airocean\n\tMisc, Sph&Ell",
    },
    ProjectionEntry {
        id: "eck1",
        description: "Eckert I\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "eck2",
        description: "Eckert II\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "eck3",
        description: "Eckert III\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "eck4",
        description: "Eckert IV\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "eck5",
        description: "Eckert V\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "eck6",
        description: "Eckert VI\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "eqearth",
        description: "Equal Earth\n\tPCyl, Sph&Ell",
    },
    ProjectionEntry {
        id: "eqc",
        description: "Equidistant Cylindrical (Plate Carree)\n\tCyl, Sph&Ell\n\tlat_ts=[, lat_0=0]",
    },
    ProjectionEntry {
        id: "eqdc",
        description: "Equidistant Conic\n\tConic, Sph&Ell\n\tlat_1= lat_2=",
    },
    ProjectionEntry {
        id: "euler",
        description: "Euler\n\tConic, Sph\n\tlat_1= and lat_2=",
    },
    ProjectionEntry {
        id: "etmerc",
        description: "Extended Transverse Mercator\n\tCyl, Sph",
    },
    ProjectionEntry {
        id: "fahey",
        description: "Fahey\n\tPcyl, Sph",
    },
    ProjectionEntry {
        id: "fouc",
        description: "Foucaut\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "fouc_s",
        description: "Foucaut Sinusoidal\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "gall",
        description: "Gall (Gall Stereographic)\n\tCyl, Sph",
    },
    ProjectionEntry {
        id: "geoc",
        description: "Geocentric Latitude",
    },
    ProjectionEntry {
        id: "geocent",
        description: "Geocentric\n\t",
    },
    ProjectionEntry {
        id: "geogoffset",
        description: "Geographic Offset",
    },
    ProjectionEntry {
        id: "geos",
        description: "Geostationary Satellite View\n\tAzi, Sph&Ell\n\th=",
    },
    ProjectionEntry {
        id: "gins8",
        description: "Ginsburg VIII (TsNIIGAiK)\n\tPCyl, Sph, no inv",
    },
    ProjectionEntry {
        id: "gn_sinu",
        description: "General Sinusoidal Series\n\tPCyl, Sph\n\tm= n=",
    },
    ProjectionEntry {
        id: "gnom",
        description: "Gnomonic\n\tAzi, Sph",
    },
    ProjectionEntry {
        id: "goode",
        description: "Goode Homolosine\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "gridshift",
        description: "Generic grid shift",
    },
    ProjectionEntry {
        id: "gs48",
        description: "Modified Stereographic of 48 U.S.\n\tAzi(mod)",
    },
    ProjectionEntry {
        id: "gs50",
        description: "Modified Stereographic of 50 U.S.\n\tAzi(mod)",
    },
    ProjectionEntry {
        id: "guyou",
        description: "Guyou\n\tMisc Sph No inv",
    },
    ProjectionEntry {
        id: "hammer",
        description: "Hammer & Eckert-Greifendorff\n\tMisc Sph, \n\tW= M=",
    },
    ProjectionEntry {
        id: "hatano",
        description: "Hatano Asymmetrical Equal Area\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "healpix",
        description: "HEALPix\n\tSph&Ell\n\trot_xy=",
    },
    ProjectionEntry {
        id: "rhealpix",
        description: "rHEALPix\n\tSph&Ell\n\tnorth_square= south_square=",
    },
    ProjectionEntry {
        id: "helmert",
        description: "3(6)-, 4(8)- and 7(14)-parameter Helmert shift",
    },
    ProjectionEntry {
        id: "hgridshift",
        description: "Horizontal grid shift",
    },
    ProjectionEntry {
        id: "horner",
        description: "Horner polynomial evaluation",
    },
    ProjectionEntry {
        id: "hourglass",
        description: "Hourglass\n\tSph, Ellips",
    },
    ProjectionEntry {
        id: "igh",
        description: "Interrupted Goode Homolosine\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "igh_o",
        description: "Interrupted Goode Homolosine Oceanic View\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "imoll",
        description: "Interrupted Mollweide\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "imoll_o",
        description: "Interrupted Mollweide Oceanic View\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "imw_p",
        description: "International Map of the World Polyconic\n\tMod. Polyconic, Ell\n\tlat_1= and lat_2= [lon_1=]",
    },
    ProjectionEntry {
        id: "interrupted",
        description: "Interrupted\n\tSph&Ell",
    },
    ProjectionEntry {
        id: "isea",
        description: "Icosahedral Snyder Equal Area\n\tSph&Ell\n\tnet= dual orient= orient_lat= orient_lon= azi= lat_0= lon_0=",
    },
    ProjectionEntry {
        id: "ivea",
        description: "Icosahedral Vertex Equal Area\n\tSph&Ell\n\torient= orient_lat= orient_lon= azi= lat_0= lon_0=",
    },
    ProjectionEntry {
        id: "kav5",
        description: "Kavrayskiy V\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "kav7",
        description: "Kavrayskiy VII\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "krovak",
        description: "Krovak\n\tPCyl, Ell",
    },
    ProjectionEntry {
        id: "labrd",
        description: "Laborde\n\tCyl, Sph\n\tSpecial for Madagascar\n\tlat_0=",
    },
    ProjectionEntry {
        id: "laea",
        description: "Lambert Azimuthal Equal Area\n\tAzi, Sph&Ell",
    },
    ProjectionEntry {
        id: "lagrng",
        description: "Lagrange\n\tMisc Sph\n\tW=",
    },
    ProjectionEntry {
        id: "larr",
        description: "Larrivee\n\tMisc Sph, no inv",
    },
    ProjectionEntry {
        id: "lask",
        description: "Laskowski\n\tMisc Sph, no inv",
    },
    ProjectionEntry {
        id: "lonlat",
        description: "Lat/long (Geodetic)\n\t",
    },
    ProjectionEntry {
        id: "latlon",
        description: "Lat/long (Geodetic alias)\n\t",
    },
    ProjectionEntry {
        id: "latlong",
        description: "Lat/long (Geodetic alias)\n\t",
    },
    ProjectionEntry {
        id: "longlat",
        description: "Lat/long (Geodetic alias)\n\t",
    },
    ProjectionEntry {
        id: "lcc",
        description: "Lambert Conformal Conic\n\tConic, Sph&Ell\n\tlat_1= and lat_2= or lat_0, k_0=",
    },
    ProjectionEntry {
        id: "lcca",
        description: "Lambert Conformal Conic Alternative\n\tConic, Sph&Ell\n\tlat_0=",
    },
    ProjectionEntry {
        id: "leac",
        description: "Lambert Equal Area Conic\n\tConic, Sph&Ell\n\tlat_1= south",
    },
    ProjectionEntry {
        id: "lee_os",
        description: "Lee Oblated Stereographic\n\tAzi(mod)",
    },
    ProjectionEntry {
        id: "loxim",
        description: "Loximuthal\n\tPCyl Sph",
    },
    ProjectionEntry {
        id: "lsat",
        description: "Space oblique for LANDSAT\n\tCyl, Sph&Ell\n\tlsat= path=",
    },
    ProjectionEntry {
        id: "mbt_s",
        description: "McBryde-Thomas Flat-Polar Sine (No. 1)\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "mbt_fps",
        description: "McBryde-Thomas Flat-Pole Sine (No. 2)\n\tCyl, Sph",
    },
    ProjectionEntry {
        id: "mbtfpp",
        description: "McBride-Thomas Flat-Polar Parabolic\n\tCyl, Sph",
    },
    ProjectionEntry {
        id: "mbtfpq",
        description: "McBryde-Thomas Flat-Polar Quartic\n\tCyl, Sph",
    },
    ProjectionEntry {
        id: "mbtfps",
        description: "McBryde-Thomas Flat-Polar Sinusoidal\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "merc",
        description: "Mercator\n\tCyl, Sph&Ell\n\tlat_ts=",
    },
    ProjectionEntry {
        id: "mil_os",
        description: "Miller Oblated Stereographic\n\tAzi(mod)",
    },
    ProjectionEntry {
        id: "mill",
        description: "Miller Cylindrical\n\tCyl, Sph",
    },
    ProjectionEntry {
        id: "misrsom",
        description: "Space oblique for MISR\n\tCyl, Sph&Ell\n\tpath=",
    },
    ProjectionEntry {
        id: "mod_krovak",
        description: "Modified Krovak\n\tPCyl, Ell",
    },
    ProjectionEntry {
        id: "moll",
        description: "Mollweide\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "molobadekas",
        description: "Molodensky-Badekas transformation",
    },
    ProjectionEntry {
        id: "molodensky",
        description: "Molodensky transform",
    },
    ProjectionEntry {
        id: "murd1",
        description: "Murdoch I\n\tConic, Sph\n\tlat_1= and lat_2=",
    },
    ProjectionEntry {
        id: "murd2",
        description: "Murdoch II\n\tConic, Sph\n\tlat_1= and lat_2=",
    },
    ProjectionEntry {
        id: "murd3",
        description: "Murdoch III\n\tConic, Sph\n\tlat_1= and lat_2=",
    },
    ProjectionEntry {
        id: "natearth",
        description: "Natural Earth\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "natearth2",
        description: "Natural Earth 2\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "nell",
        description: "Nell\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "nell_h",
        description: "Nell-Hammer\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "nicol",
        description: "Nicolosi Globular\n\tMisc Sph, no inv",
    },
    ProjectionEntry {
        id: "nsper",
        description: "Near-sided perspective\n\tAzi, Sph\n\th=",
    },
    ProjectionEntry {
        id: "nzmg",
        description: "New Zealand Map Grid\n\tfixed Earth",
    },
    ProjectionEntry {
        id: "noop",
        description: "No operation",
    },
    ProjectionEntry {
        id: "ob_tran",
        description: "General Oblique Transformation\n\tMisc Sph\n\to_proj= plus parameters for projection\n\to_lat_p= o_lon_p= (new pole) or\n\to_alpha= o_lon_c= o_lat_c= or\n\to_lon_1= o_lat_1= o_lon_2= o_lat_2=",
    },
    ProjectionEntry {
        id: "ocea",
        description: "Oblique Cylindrical Equal Area\n\tCyl, Sphlonc= alpha= or\n\tlat_1= lat_2= lon_1= lon_2=",
    },
    ProjectionEntry {
        id: "oea",
        description: "Oblated Equal Area\n\tMisc Sph\n\tn= m= theta=",
    },
    ProjectionEntry {
        id: "omerc",
        description: "Oblique Mercator\n\tCyl, Sph&Ell no_rot\n\talpha= [gamma=] [no_off] lonc= or\n\t lon_1= lat_1= lon_2= lat_2=",
    },
    ProjectionEntry {
        id: "ortel",
        description: "Ortelius Oval\n\tMisc Sph, no inv",
    },
    ProjectionEntry {
        id: "ortho",
        description: "Orthographic\n\tAzi, Sph&Ell",
    },
    ProjectionEntry {
        id: "pconic",
        description: "Perspective Conic\n\tConic, Sph\n\tlat_1= and lat_2=",
    },
    ProjectionEntry {
        id: "patterson",
        description: "Patterson Cylindrical\n\tCyl",
    },
    ProjectionEntry {
        id: "peirce_q",
        description: "Peirce Quincuncial\n\tMisc Sph No inv",
    },
    ProjectionEntry {
        id: "pipeline",
        description: "Transformation pipeline manager",
    },
    ProjectionEntry {
        id: "poly",
        description: "Polyconic (American)\n\tConic, Sph&Ell",
    },
    ProjectionEntry {
        id: "pop",
        description: "Retrieve coordinate value from pipeline stack",
    },
    ProjectionEntry {
        id: "push",
        description: "Save coordinate value on pipeline stack",
    },
    ProjectionEntry {
        id: "putp1",
        description: "Putnins P1\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "putp2",
        description: "Putnins P2\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "putp3",
        description: "Putnins P3\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "putp3p",
        description: "Putnins P3'\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "putp4p",
        description: "Putnins P4'\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "putp5",
        description: "Putnins P5\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "putp5p",
        description: "Putnins P5'\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "putp6",
        description: "Putnins P6\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "putp6p",
        description: "Putnins P6'\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "qua_aut",
        description: "Quartic Authalic\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "qsc",
        description: "Quadrilateralized Spherical Cube\n\tAzi, Sph",
    },
    ProjectionEntry {
        id: "robin",
        description: "Robinson\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "rouss",
        description: "Roussilhe Stereographic\n\tAzi, Ell",
    },
    ProjectionEntry {
        id: "rpoly",
        description: "Rectangular Polyconic\n\tConic, Sph, no inv\n\tlat_ts=",
    },
    ProjectionEntry {
        id: "s2",
        description: "S2\n\tMisc, Sph&Ell",
    },
    ProjectionEntry {
        id: "sch",
        description: "Spherical Cross-track Height\n\tMisc\n\tplat_0= plon_0= phdg_0= [h_0=]",
    },
    ProjectionEntry {
        id: "set",
        description: "Set coordinate value",
    },
    ProjectionEntry {
        id: "sinu",
        description: "Sinusoidal (Sanson-Flamsteed)\n\tPCyl, Sph&Ell",
    },
    ProjectionEntry {
        id: "som",
        description: "Space Oblique Mercator\n\tCyl, Sph&Ell\n\tinc_angle= ps_rev= asc_lon= ",
    },
    ProjectionEntry {
        id: "somerc",
        description: "Swiss. Obl. Mercator\n\tCyl, Ell\n\tFor CH1903",
    },
    ProjectionEntry {
        id: "spilhaus",
        description: "Spilhaus\n\tSph&Ell",
    },
    ProjectionEntry {
        id: "stere",
        description: "Stereographic\n\tAzi, Sph&Ell\n\tlat_ts=",
    },
    ProjectionEntry {
        id: "sterea",
        description: "Oblique Stereographic Alternative\n\tAzimuthal, Sph&Ell",
    },
    ProjectionEntry {
        id: "gstmerc",
        description: "Gauss-Schreiber Transverse Mercator (aka Gauss-Laborde Reunion)\n\tCyl, Sph&Ell\n\tlat_0= lon_0= k_0=",
    },
    ProjectionEntry {
        id: "tcc",
        description: "Transverse Central Cylindrical\n\tCyl, Sph, no inv",
    },
    ProjectionEntry {
        id: "tcea",
        description: "Transverse Cylindrical Equal Area\n\tCyl, Sph",
    },
    ProjectionEntry {
        id: "times",
        description: "Times\n\tCyl, Sph",
    },
    ProjectionEntry {
        id: "tinshift",
        description: "Triangulation based transformation",
    },
    ProjectionEntry {
        id: "tissot",
        description: "Tissot\n\tConic, Sph\n\tlat_1= and lat_2=",
    },
    ProjectionEntry {
        id: "tmerc",
        description: "Transverse Mercator\n\tCyl, Sph&Ell\n\tapprox",
    },
    ProjectionEntry {
        id: "tmerczoned",
        description: "Transverse Mercator Zoned Grid System",
    },
    ProjectionEntry {
        id: "tobmerc",
        description: "Tobler-Mercator\n\tCyl, Sph",
    },
    ProjectionEntry {
        id: "topocentric",
        description: "Geocentric/Topocentric conversion",
    },
    ProjectionEntry {
        id: "tpeqd",
        description: "Two Point Equidistant\n\tMisc Sph\n\tlat_1= lon_1= lat_2= lon_2=",
    },
    ProjectionEntry {
        id: "tpers",
        description: "Tilted perspective\n\tAzi, Sph\n\ttilt= azi= h=",
    },
    ProjectionEntry {
        id: "tsea",
        description: "Tetrahedral Snyder Equal Area\n\tSph&Ell\n\tnet= dual orient_lat= orient_lon= azi= lat_0= lon_0=",
    },
    ProjectionEntry {
        id: "unitconvert",
        description: "Unit conversion",
    },
    ProjectionEntry {
        id: "ups",
        description: "Universal Polar Stereographic\n\tAzi, Ell\n\tsouth",
    },
    ProjectionEntry {
        id: "urm5",
        description: "Urmaev V\n\tPCyl, Sph, no inv\n\tn= q= alpha=",
    },
    ProjectionEntry {
        id: "urmfps",
        description: "Urmaev Flat-Polar Sinusoidal\n\tPCyl, Sph\n\tn=",
    },
    ProjectionEntry {
        id: "utm",
        description: "Universal Transverse Mercator (UTM)\n\tCyl, Ell\n\tzone= south approx",
    },
    ProjectionEntry {
        id: "vandg",
        description: "van der Grinten (I)\n\tMisc Sph",
    },
    ProjectionEntry {
        id: "vandg2",
        description: "van der Grinten II\n\tMisc Sph, no inv",
    },
    ProjectionEntry {
        id: "vandg3",
        description: "van der Grinten III\n\tMisc Sph, no inv",
    },
    ProjectionEntry {
        id: "vandg4",
        description: "van der Grinten IV\n\tMisc Sph, no inv",
    },
    ProjectionEntry {
        id: "vertoffset",
        description: "Vertical Offset and Slope\n\tTransformation\n\tlat_0= lon_0= dh= slope_lat= slope_lon=",
    },
    ProjectionEntry {
        id: "vitk1",
        description: "Vitkovsky I\n\tConic, Sph\n\tlat_1= and lat_2=",
    },
    ProjectionEntry {
        id: "vgridshift",
        description: "Vertical grid shift",
    },
    ProjectionEntry {
        id: "wag1",
        description: "Wagner I (Kavrayskiy VI)\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "wag2",
        description: "Wagner II\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "wag3",
        description: "Wagner III\n\tPCyl, Sph\n\tlat_ts=",
    },
    ProjectionEntry {
        id: "wag4",
        description: "Wagner IV\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "wag5",
        description: "Wagner V\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "wag6",
        description: "Wagner VI\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "wag7",
        description: "Wagner VII\n\tMisc Sph, no inv",
    },
    ProjectionEntry {
        id: "webmerc",
        description: "Web Mercator / Pseudo Mercator\n\tCyl, Ell\n\t",
    },
    ProjectionEntry {
        id: "weren",
        description: "Werenskiold I\n\tPCyl, Sph",
    },
    ProjectionEntry {
        id: "wink1",
        description: "Winkel I\n\tPCyl, Sph\n\tlat_ts=",
    },
    ProjectionEntry {
        id: "wink2",
        description: "Winkel II\n\tPCyl, Sph\n\tlat_1=",
    },
    ProjectionEntry {
        id: "wintri",
        description: "Winkel Tripel\n\tMisc Sph\n\tlat_1",
    },
    ProjectionEntry {
        id: "xyzgridshift",
        description: "Geocentric grid shift",
    },
];

/// One ellipsoid of PROJ's list (`proj_list_ellps`): its id, its `a=` and `rf=` or `b=`
/// text, and its name.
pub const ELLIPSOIDS: &[EllipsoidEntry] = &[
    EllipsoidEntry {
        id: "MERIT",
        major: "a=6378137.0",
        ell: "rf=298.257",
        name: "MERIT 1983",
    },
    EllipsoidEntry {
        id: "SGS85",
        major: "a=6378136.0",
        ell: "rf=298.257",
        name: "Soviet Geodetic System 85",
    },
    EllipsoidEntry {
        id: "GRS80",
        major: "a=6378137.0",
        ell: "rf=298.257222101",
        name: "GRS 1980(IUGG, 1980)",
    },
    EllipsoidEntry {
        id: "IAU76",
        major: "a=6378140.0",
        ell: "rf=298.257",
        name: "IAU 1976",
    },
    EllipsoidEntry {
        id: "airy",
        major: "a=6377563.396",
        ell: "rf=299.3249646",
        name: "Airy 1830",
    },
    EllipsoidEntry {
        id: "APL4.9",
        major: "a=6378137.0",
        ell: "rf=298.25",
        name: "Appl. Physics. 1965",
    },
    EllipsoidEntry {
        id: "NWL9D",
        major: "a=6378145.0",
        ell: "rf=298.25",
        name: "Naval Weapons Lab., 1965",
    },
    EllipsoidEntry {
        id: "mod_airy",
        major: "a=6377340.189",
        ell: "b=6356034.446",
        name: "Modified Airy",
    },
    EllipsoidEntry {
        id: "andrae",
        major: "a=6377104.43",
        ell: "rf=300.0",
        name: "Andrae 1876 (Den., Iclnd.)",
    },
    EllipsoidEntry {
        id: "danish",
        major: "a=6377019.2563",
        ell: "rf=300.0",
        name: "Andrae 1876 (Denmark, Iceland)",
    },
    EllipsoidEntry {
        id: "aust_SA",
        major: "a=6378160.0",
        ell: "rf=298.25",
        name: "Australian Natl & S. Amer. 1969",
    },
    EllipsoidEntry {
        id: "GRS67",
        major: "a=6378160.0",
        ell: "rf=298.2471674270",
        name: "GRS 67(IUGG 1967)",
    },
    EllipsoidEntry {
        id: "GSK2011",
        major: "a=6378136.5",
        ell: "rf=298.2564151",
        name: "GSK-2011",
    },
    EllipsoidEntry {
        id: "bessel",
        major: "a=6377397.155",
        ell: "rf=299.1528128",
        name: "Bessel 1841",
    },
    EllipsoidEntry {
        id: "bess_nam",
        major: "a=6377483.865",
        ell: "rf=299.1528128",
        name: "Bessel 1841 (Namibia)",
    },
    EllipsoidEntry {
        id: "clrk66",
        major: "a=6378206.4",
        ell: "b=6356583.8",
        name: "Clarke 1866",
    },
    EllipsoidEntry {
        id: "clrk80",
        major: "a=6378249.145",
        ell: "rf=293.4663",
        name: "Clarke 1880 mod.",
    },
    EllipsoidEntry {
        id: "clrk80ign",
        major: "a=6378249.2",
        ell: "rf=293.4660212936269",
        name: "Clarke 1880 (IGN).",
    },
    EllipsoidEntry {
        id: "CPM",
        major: "a=6375738.7",
        ell: "rf=334.29",
        name: "Comm. des Poids et Mesures 1799",
    },
    EllipsoidEntry {
        id: "delmbr",
        major: "a=6376428.",
        ell: "rf=311.5",
        name: "Delambre 1810 (Belgium)",
    },
    EllipsoidEntry {
        id: "engelis",
        major: "a=6378136.05",
        ell: "rf=298.2566",
        name: "Engelis 1985",
    },
    EllipsoidEntry {
        id: "evrst30",
        major: "a=6377276.345",
        ell: "rf=300.8017",
        name: "Everest 1830",
    },
    EllipsoidEntry {
        id: "evrst48",
        major: "a=6377304.063",
        ell: "rf=300.8017",
        name: "Everest 1948",
    },
    EllipsoidEntry {
        id: "evrst56",
        major: "a=6377301.243",
        ell: "rf=300.8017",
        name: "Everest 1956",
    },
    EllipsoidEntry {
        id: "evrst69",
        major: "a=6377295.664",
        ell: "rf=300.8017",
        name: "Everest 1969",
    },
    EllipsoidEntry {
        id: "evrstSS",
        major: "a=6377298.556",
        ell: "rf=300.8017",
        name: "Everest (Sabah & Sarawak)",
    },
    EllipsoidEntry {
        id: "fschr60",
        major: "a=6378166.",
        ell: "rf=298.3",
        name: "Fischer (Mercury Datum) 1960",
    },
    EllipsoidEntry {
        id: "fschr60m",
        major: "a=6378155.",
        ell: "rf=298.3",
        name: "Modified Fischer 1960",
    },
    EllipsoidEntry {
        id: "fschr68",
        major: "a=6378150.",
        ell: "rf=298.3",
        name: "Fischer 1968",
    },
    EllipsoidEntry {
        id: "helmert",
        major: "a=6378200.",
        ell: "rf=298.3",
        name: "Helmert 1906",
    },
    EllipsoidEntry {
        id: "hough",
        major: "a=6378270.0",
        ell: "rf=297.",
        name: "Hough",
    },
    EllipsoidEntry {
        id: "intl",
        major: "a=6378388.0",
        ell: "rf=297.",
        name: "International 1924 (Hayford 1909, 1910)",
    },
    EllipsoidEntry {
        id: "krass",
        major: "a=6378245.0",
        ell: "rf=298.3",
        name: "Krassovsky, 1942",
    },
    EllipsoidEntry {
        id: "kaula",
        major: "a=6378163.",
        ell: "rf=298.24",
        name: "Kaula 1961",
    },
    EllipsoidEntry {
        id: "lerch",
        major: "a=6378139.",
        ell: "rf=298.257",
        name: "Lerch 1979",
    },
    EllipsoidEntry {
        id: "mprts",
        major: "a=6397300.",
        ell: "rf=191.",
        name: "Maupertius 1738",
    },
    EllipsoidEntry {
        id: "new_intl",
        major: "a=6378157.5",
        ell: "b=6356772.2",
        name: "New International 1967",
    },
    EllipsoidEntry {
        id: "plessis",
        major: "a=6376523.",
        ell: "b=6355863.",
        name: "Plessis 1817 (France)",
    },
    EllipsoidEntry {
        id: "PZ90",
        major: "a=6378136.0",
        ell: "rf=298.25784",
        name: "PZ-90",
    },
    EllipsoidEntry {
        id: "SEasia",
        major: "a=6378155.0",
        ell: "b=6356773.3205",
        name: "Southeast Asia",
    },
    EllipsoidEntry {
        id: "walbeck",
        major: "a=6376896.0",
        ell: "b=6355834.8467",
        name: "Walbeck",
    },
    EllipsoidEntry {
        id: "WGS60",
        major: "a=6378165.0",
        ell: "rf=298.3",
        name: "WGS 60",
    },
    EllipsoidEntry {
        id: "WGS66",
        major: "a=6378145.0",
        ell: "rf=298.25",
        name: "WGS 66",
    },
    EllipsoidEntry {
        id: "WGS72",
        major: "a=6378135.0",
        ell: "rf=298.26",
        name: "WGS 72",
    },
    EllipsoidEntry {
        id: "WGS84",
        major: "a=6378137.0",
        ell: "rf=298.257223563",
        name: "WGS 84",
    },
    EllipsoidEntry {
        id: "sphere",
        major: "a=6370997.0",
        ell: "b=6370997.0",
        name: "Normal Sphere (r=6370997)",
    },
];
