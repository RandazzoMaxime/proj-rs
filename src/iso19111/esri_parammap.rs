//! ESRI method and parameter names for WKT1 ESRI, from PROJ `esriparammappings.cpp`.
//!
//! The table is generated from the PROJ 9.9.0 sources and keeps their order: the
//! first row that matches a method wins. `ESRIParamMapping::is_fixed_value` is not
//! kept, because only the WKT1 ESRI import reads it, and that import is not ported.

use super::buildop::{names_match, OpProps};
use super::model::{
    Conversion, Ellipsoid, FactoryError, Measure, Parameter, ParameterValue, UnitOfMeasure,
};

/// One `PARAMETER` of an ESRI method.
#[derive(Clone, Copy, Debug)]
pub(crate) enum EsriParam {
    /// Value of the conversion parameter, found by EPSG code, else by WKT2 name.
    Value {
        esri_name: &'static str,
        wkt2_name: &'static str,
        epsg_code: i32,
    },
    /// Fixed text, for an ESRI parameter with no WKT2 counterpart.
    Fixed {
        esri_name: &'static str,
        text: &'static str,
    },
}

impl EsriParam {
    const fn value(esri_name: &'static str, wkt2_name: &'static str, epsg_code: i32) -> Self {
        Self::Value {
            esri_name,
            wkt2_name,
            epsg_code,
        }
    }

    const fn fixed(esri_name: &'static str, text: &'static str) -> Self {
        Self::Fixed { esri_name, text }
    }
}

/// One row of the ESRI method table.
#[derive(Clone, Copy, Debug)]
pub(crate) struct EsriMethod {
    pub(crate) esri_name: &'static str,
    pub(crate) wkt2_name: &'static str,
    pub(crate) epsg_code: i32,
    pub(crate) params: &'static [EsriParam],
}

/// The ESRI method name and parameter list a conversion is written with.
pub(crate) struct EsriChoice {
    pub(crate) name: &'static str,
    pub(crate) params: &'static [EsriParam],
}

static PARAMS_EQUIDISTANT_CYLINDRICAL: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::value(
        "Standard_Parallel_1",
        "Latitude of 1st standard parallel",
        8823,
    ),
];

static PARAMS_PLATE_CARREE: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
];

static PARAMS_MILLER_CYLINDRICAL: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
];

static PARAMS_MERCATOR: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::value(
        "Standard_Parallel_1",
        "Latitude of 1st standard parallel",
        8823,
    ),
];

static PARAMS_GAUSS_KRUGER: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::value("Scale_Factor", "Scale factor at natural origin", 8805),
    EsriParam::value("Latitude_Of_Origin", "Latitude of natural origin", 8801),
];

static PARAMS_TRANSVERSE_MERCATOR: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::value("Scale_Factor", "Scale factor at natural origin", 8805),
    EsriParam::value("Latitude_Of_Origin", "Latitude of natural origin", 8801),
];

static PARAMS_TRANSVERSE_MERCATOR_COMPLEX: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::value("Scale_Factor", "Scale factor at natural origin", 8805),
    EsriParam::value("Latitude_Of_Origin", "Latitude of natural origin", 8801),
];

static PARAMS_ALBERS: &[EsriParam] = &[
    EsriParam::value("False_Easting", "Easting at false origin", 8826),
    EsriParam::value("False_Northing", "Northing at false origin", 8827),
    EsriParam::value("Central_Meridian", "Longitude of false origin", 8822),
    EsriParam::value(
        "Standard_Parallel_1",
        "Latitude of 1st standard parallel",
        8823,
    ),
    EsriParam::value(
        "Standard_Parallel_2",
        "Latitude of 2nd standard parallel",
        8824,
    ),
    EsriParam::value("Latitude_Of_Origin", "Latitude of false origin", 8821),
];

static PARAMS_SINUSOIDAL: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
];

static PARAMS_MOLLWEIDE: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
];

static PARAMS_ECKERT_I: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
];

static PARAMS_ECKERT_II: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
];

static PARAMS_ECKERT_III: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
];

static PARAMS_ECKERT_IV: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
];

static PARAMS_ECKERT_V: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
];

static PARAMS_ECKERT_VI: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
];

static PARAMS_GALL_STEREOGRAPHIC: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
];

static PARAMS_WINKEL_I: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::value(
        "Standard_Parallel_1",
        "Latitude of 1st standard parallel",
        8823,
    ),
];

static PARAMS_WINKEL_II: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::value(
        "Standard_Parallel_1",
        "Latitude of 1st standard parallel",
        8823,
    ),
];

static PARAMS_LAMBERT_CONFORMAL_CONIC_ALT1: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::value("Standard_Parallel_1", "Latitude of natural origin", 8801),
    EsriParam::value("Scale_Factor", "Scale factor at natural origin", 8805),
    EsriParam::value("Latitude_Of_Origin", "Latitude of natural origin", 8801),
];

static PARAMS_LAMBERT_CONFORMAL_CONIC_ALT2: &[EsriParam] = &[
    EsriParam::value("False_Easting", "Easting at false origin", 8826),
    EsriParam::value("False_Northing", "Northing at false origin", 8827),
    EsriParam::value("Central_Meridian", "Longitude of false origin", 8822),
    EsriParam::value(
        "Standard_Parallel_1",
        "Latitude of 1st standard parallel",
        8823,
    ),
    EsriParam::value(
        "Standard_Parallel_2",
        "Latitude of 2nd standard parallel",
        8824,
    ),
    EsriParam::value("Latitude_Of_Origin", "Latitude of false origin", 8821),
];

static PARAMS_LAMBERT_CONFORMAL_CONIC_ALT3: &[EsriParam] = &[
    EsriParam::value("False_Easting", "Easting at false origin", 8826),
    EsriParam::value("False_Northing", "Northing at false origin", 8827),
    EsriParam::value("Central_Meridian", "Longitude of false origin", 8822),
    EsriParam::value(
        "Standard_Parallel_1",
        "Latitude of 1st standard parallel",
        8823,
    ),
    EsriParam::value(
        "Standard_Parallel_2",
        "Latitude of 2nd standard parallel",
        8824,
    ),
    EsriParam::fixed("Scale_Factor", "1.0"),
    EsriParam::value("Latitude_Of_Origin", "Latitude of false origin", 8821),
];

static PARAMS_LAMBERT_CONFORMAL_CONIC_ALT4: &[EsriParam] = &[
    EsriParam::value("False_Easting", "Easting at false origin", 8826),
    EsriParam::value("False_Northing", "Northing at false origin", 8827),
    EsriParam::value("Central_Meridian", "Longitude of false origin", 8822),
    EsriParam::value(
        "Standard_Parallel_1",
        "Latitude of 1st standard parallel",
        8823,
    ),
    EsriParam::value(
        "Standard_Parallel_2",
        "Latitude of 2nd standard parallel",
        8824,
    ),
    EsriParam::value("Scale_Factor", "Ellipsoid scaling factor", 1038),
    EsriParam::value("Latitude_Of_Origin", "Latitude of false origin", 8821),
];

static PARAMS_POLYCONIC: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::value("Latitude_Of_Origin", "Latitude of natural origin", 8801),
];

static PARAMS_QUARTIC_AUTHALIC: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
];

static PARAMS_LOXIMUTHAL: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::value("Central_Parallel", "Latitude of natural origin", 8801),
];

static PARAMS_BONNE: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::value("Standard_Parallel_1", "Latitude of natural origin", 8801),
];

static PARAMS_HOTINE_OBLIQUE_MERCATOR_TWO_POINT_NATURAL_ORIGIN: &[EsriParam] = &[
    EsriParam::value("False_Easting", "Easting at projection centre", 8816),
    EsriParam::value("False_Northing", "Northing at projection centre", 8817),
    EsriParam::value("Latitude_Of_1st_Point", "Latitude of 1st point", 0),
    EsriParam::value("Latitude_Of_2nd_Point", "Latitude of 2nd point", 0),
    EsriParam::value("Scale_Factor", "Scale factor at projection centre", 8815),
    EsriParam::value("Longitude_Of_1st_Point", "Longitude of 1st point", 0),
    EsriParam::value("Longitude_Of_2nd_Point", "Longitude of 2nd point", 0),
    EsriParam::value("Latitude_Of_Center", "Latitude of projection centre", 8811),
];

static PARAMS_STEREOGRAPHIC: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::value("Scale_Factor", "Scale factor at natural origin", 8805),
    EsriParam::value("Latitude_Of_Origin", "Latitude of natural origin", 8801),
];

static PARAMS_POLAR_STEREOGRAPHIC_VARIANT_A: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::value("Scale_Factor", "Scale factor at natural origin", 8805),
    EsriParam::value("Latitude_Of_Origin", "Latitude of natural origin", 8801),
];

static PARAMS_EQUIDISTANT_CONIC: &[EsriParam] = &[
    EsriParam::value("False_Easting", "Easting at false origin", 8826),
    EsriParam::value("False_Northing", "Northing at false origin", 8827),
    EsriParam::value("Central_Meridian", "Longitude of false origin", 8822),
    EsriParam::value(
        "Standard_Parallel_1",
        "Latitude of 1st standard parallel",
        8823,
    ),
    EsriParam::value(
        "Standard_Parallel_2",
        "Latitude of 2nd standard parallel",
        8824,
    ),
    EsriParam::value("Latitude_Of_Origin", "Latitude of false origin", 8821),
];

static PARAMS_CASSINI: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::fixed("Scale_Factor", "1.0"),
    EsriParam::value("Latitude_Of_Origin", "Latitude of natural origin", 8801),
];

static PARAMS_VAN_DER_GRINTEN_I: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
];

static PARAMS_ROBINSON: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
];

static PARAMS_TWO_POINT_EQUIDISTANT: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Latitude_Of_1st_Point", "Latitude of 1st point", 0),
    EsriParam::value("Latitude_Of_2nd_Point", "Latitude of 2nd point", 0),
    EsriParam::value("Longitude_Of_1st_Point", "Longitude of 1st point", 0),
    EsriParam::value("Longitude_Of_2nd_Point", "Longitude of 2nd point", 0),
];

static PARAMS_AZIMUTHAL_EQUIDISTANT: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::value("Latitude_Of_Origin", "Latitude of natural origin", 8801),
];

static PARAMS_LAMBERT_AZIMUTHAL_EQUAL_AREA: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::value("Latitude_Of_Origin", "Latitude of natural origin", 8801),
];

static PARAMS_CYLINDRICAL_EQUAL_AREA: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::value(
        "Standard_Parallel_1",
        "Latitude of 1st standard parallel",
        8823,
    ),
];

static PARAMS_BEHRMANN: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::value(
        "Standard_Parallel_1",
        "Latitude of 1st standard parallel",
        8823,
    ),
];

static PARAMS_HOTINE_OBLIQUE_MERCATOR_TWO_POINT_CENTER: &[EsriParam] = &[
    EsriParam::value("False_Easting", "Easting at projection centre", 8816),
    EsriParam::value("False_Northing", "Northing at projection centre", 8817),
    EsriParam::value("Latitude_Of_1st_Point", "Latitude of 1st point", 0),
    EsriParam::value("Latitude_Of_2nd_Point", "Latitude of 2nd point", 0),
    EsriParam::value("Scale_Factor", "Scale factor at projection centre", 8815),
    EsriParam::value("Longitude_Of_1st_Point", "Longitude of 1st point", 0),
    EsriParam::value("Longitude_Of_2nd_Point", "Longitude of 2nd point", 0),
    EsriParam::value("Latitude_Of_Origin", "Latitude of natural origin", 8801),
];

static PARAMS_HOTINE_OBLIQUE_MERCATOR_AZIMUTH_NATURAL_ORIGIN: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Scale_Factor", "Scale factor at projection centre", 8815),
    EsriParam::value("Azimuth", "Azimuth at projection centre", 8813),
    EsriParam::value(
        "Longitude_Of_Center",
        "Longitude of projection centre",
        8812,
    ),
    EsriParam::value("Latitude_Of_Center", "Latitude of projection centre", 8811),
];

static PARAMS_HOTINE_OBLIQUE_MERCATOR_AZIMUTH_CENTER: &[EsriParam] = &[
    EsriParam::value("False_Easting", "Easting at projection centre", 8816),
    EsriParam::value("False_Northing", "Northing at projection centre", 8817),
    EsriParam::value("Scale_Factor", "Scale factor at projection centre", 8815),
    EsriParam::value("Azimuth", "Azimuth at projection centre", 8813),
    EsriParam::value(
        "Longitude_Of_Center",
        "Longitude of projection centre",
        8812,
    ),
    EsriParam::value("Latitude_Of_Center", "Latitude of projection centre", 8811),
];

static PARAMS_DOUBLE_STEREOGRAPHIC: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::value("Scale_Factor", "Scale factor at natural origin", 8805),
    EsriParam::value("Latitude_Of_Origin", "Latitude of natural origin", 8801),
];

static PARAMS_KROVAK_ALT1: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value(
        "Pseudo_Standard_Parallel_1",
        "Latitude of pseudo standard parallel",
        8818,
    ),
    EsriParam::value(
        "Scale_Factor",
        "Scale factor on pseudo standard parallel",
        8819,
    ),
    EsriParam::value("Azimuth", "Co-latitude of cone axis", 1036),
    EsriParam::value("Longitude_Of_Center", "Longitude of origin", 8833),
    EsriParam::value("Latitude_Of_Center", "Latitude of projection centre", 8811),
    EsriParam::fixed("X_Scale", "1.0"),
    EsriParam::fixed("Y_Scale", "1.0"),
    EsriParam::fixed("XY_Plane_Rotation", "0.0"),
];

static PARAMS_KROVAK_ALT2: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value(
        "Pseudo_Standard_Parallel_1",
        "Latitude of pseudo standard parallel",
        8818,
    ),
    EsriParam::value(
        "Scale_Factor",
        "Scale factor on pseudo standard parallel",
        8819,
    ),
    EsriParam::value("Azimuth", "Co-latitude of cone axis", 1036),
    EsriParam::value("Longitude_Of_Center", "Longitude of origin", 8833),
    EsriParam::value("Latitude_Of_Center", "Latitude of projection centre", 8811),
    EsriParam::fixed("X_Scale", "-1.0"),
    EsriParam::fixed("Y_Scale", "1.0"),
    EsriParam::fixed("XY_Plane_Rotation", "90.0"),
];

static PARAMS_NEW_ZEALAND_MAP_GRID: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Longitude_Of_Origin", "Longitude of natural origin", 8802),
    EsriParam::value("Latitude_Of_Origin", "Latitude of natural origin", 8801),
];

static PARAMS_ORTHOGRAPHIC: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Longitude_Of_Center", "Longitude of natural origin", 8802),
    EsriParam::value("Latitude_Of_Center", "Latitude of natural origin", 8801),
];

static PARAMS_LOCAL: &[EsriParam] = &[
    EsriParam::value("False_Easting", "Easting at projection centre", 8816),
    EsriParam::value("False_Northing", "Northing at projection centre", 8817),
    EsriParam::value("Scale_Factor", "Scale factor at projection centre", 8815),
    EsriParam::value("Azimuth", "Azimuth at projection centre", 8813),
    EsriParam::value(
        "Longitude_Of_Center",
        "Longitude of projection centre",
        8812,
    ),
    EsriParam::value("Latitude_Of_Center", "Latitude of projection centre", 8811),
];

static PARAMS_WINKEL_TRIPEL: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::value(
        "Standard_Parallel_1",
        "Latitude of 1st standard parallel",
        8823,
    ),
];

static PARAMS_AITOFF: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
];

static PARAMS_FLAT_POLAR_QUARTIC: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
];

static PARAMS_CRASTER_PARABOLIC: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
];

static PARAMS_GNOMONIC: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Longitude_Of_Center", "Longitude of natural origin", 8802),
    EsriParam::value("Latitude_Of_Center", "Latitude of natural origin", 8801),
];

static PARAMS_TIMES: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
];

static PARAMS_VERTICAL_NEAR_SIDE_PERSPECTIVE: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value(
        "Longitude_Of_Center",
        "Longitude of topocentric origin",
        8835,
    ),
    EsriParam::value("Latitude_Of_Center", "Latitude of topocentric origin", 8834),
    EsriParam::value("Height", "Viewpoint height", 8840),
];

static PARAMS_STEREOGRAPHIC_NORTH_POLE: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of origin", 8833),
    EsriParam::value("Standard_Parallel_1", "Latitude of standard parallel", 8832),
];

static PARAMS_STEREOGRAPHIC_SOUTH_POLE: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of origin", 8833),
    EsriParam::value("Standard_Parallel_1", "Latitude of standard parallel", 8832),
];

static PARAMS_RECTIFIED_SKEW_ORTHOMORPHIC_NATURAL_ORIGIN: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Scale_Factor", "Scale factor at projection centre", 8815),
    EsriParam::value("Azimuth", "Azimuth at projection centre", 8813),
    EsriParam::value(
        "Longitude_Of_Center",
        "Longitude of projection centre",
        8812,
    ),
    EsriParam::value("Latitude_Of_Center", "Latitude of projection centre", 8811),
    EsriParam::value(
        "XY_Plane_Rotation",
        "Angle from Rectified to Skew Grid",
        8814,
    ),
];

static PARAMS_RECTIFIED_SKEW_ORTHOMORPHIC_CENTER: &[EsriParam] = &[
    EsriParam::value("False_Easting", "Easting at projection centre", 8816),
    EsriParam::value("False_Northing", "Northing at projection centre", 8817),
    EsriParam::value("Scale_Factor", "Scale factor at projection centre", 8815),
    EsriParam::value("Azimuth", "Azimuth at projection centre", 8813),
    EsriParam::value(
        "Longitude_Of_Center",
        "Longitude of projection centre",
        8812,
    ),
    EsriParam::value("Latitude_Of_Center", "Latitude of projection centre", 8811),
    EsriParam::value(
        "XY_Plane_Rotation",
        "Angle from Rectified to Skew Grid",
        8814,
    ),
];

static PARAMS_GOODE_HOMOLOSINE_ALT1: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::fixed("Option", "0.0"),
];

static PARAMS_GOODE_HOMOLOSINE_ALT2: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::fixed("Option", "1.0"),
];

static PARAMS_GOODE_HOMOLOSINE_ALT3: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::fixed("Option", "2.0"),
];

static PARAMS_EQUIDISTANT_CYLINDRICAL_ELLIPSOIDAL: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::value(
        "Standard_Parallel_1",
        "Latitude of 1st standard parallel",
        8823,
    ),
];

static PARAMS_LABORDE_OBLIQUE_MERCATOR: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Scale_Factor", "Scale factor at projection centre", 8815),
    EsriParam::value("Azimuth", "Azimuth at projection centre", 8813),
    EsriParam::value(
        "Longitude_Of_Center",
        "Longitude of projection centre",
        8812,
    ),
    EsriParam::value("Latitude_Of_Center", "Latitude of projection centre", 8811),
];

static PARAMS_GNOMONIC_ELLIPSOIDAL: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Longitude_Of_Center", "Longitude of natural origin", 8802),
    EsriParam::value("Latitude_Of_Center", "Latitude of natural origin", 8801),
];

static PARAMS_WAGNER_IV: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::fixed("Latitude_Of_Origin", "0.0"),
];

static PARAMS_WAGNER_V: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
];

static PARAMS_WAGNER_VII: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
];

static PARAMS_NATURAL_EARTH: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
];

static PARAMS_NATURAL_EARTH_II: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
];

static PARAMS_PATTERSON: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
];

static PARAMS_COMPACT_MILLER: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
];

static PARAMS_GEOSTATIONARY_SATELLITE: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Longitude_Of_Center", "Longitude of natural origin", 8802),
    EsriParam::value("Height", "Satellite Height", 0),
    EsriParam::fixed("Option", "0.0"),
];

static PARAMS_MERCATOR_AUXILIARY_SPHERE: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::value("Standard_Parallel_1", "Latitude of natural origin", 8801),
    EsriParam::fixed("Auxiliary_Sphere_Type", "0.0"),
];

static PARAMS_MERCATOR_VARIANT_A: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::value("Scale_Factor", "Scale factor at natural origin", 8805),
];

static PARAMS_MERCATOR_VARIANT_C: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::value(
        "Standard_Parallel_1",
        "Latitude of 1st standard parallel",
        8823,
    ),
    EsriParam::fixed("Latitude_Of_Origin", "0.0"),
];

static PARAMS_TRANSVERSE_CYLINDRICAL_EQUAL_AREA: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::value("Scale_Factor", "Scale factor at natural origin", 8805),
    EsriParam::value("Latitude_Of_Origin", "Latitude of natural origin", 8801),
];

static PARAMS_IGAC_PLANO_CARTESIANO: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Longitude_Of_Center", "Longitude of natural origin", 8802),
    EsriParam::value("Latitude_Of_Center", "Latitude of natural origin", 8801),
    EsriParam::value("Height", "Projection plane origin height", 1039),
];

static PARAMS_EQUAL_EARTH: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
];

static PARAMS_PEIRCE_QUINCUNCIAL_ALT1: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::value("Scale_Factor", "Scale factor at natural origin", 8805),
    EsriParam::value("Latitude_Of_Origin", "Latitude of natural origin", 8801),
    EsriParam::fixed("Option", "0.0"),
];

static PARAMS_PEIRCE_QUINCUNCIAL_ALT2: &[EsriParam] = &[
    EsriParam::value("False_Easting", "False easting", 8806),
    EsriParam::value("False_Northing", "False northing", 8807),
    EsriParam::value("Central_Meridian", "Longitude of natural origin", 8802),
    EsriParam::value("Scale_Factor", "Scale factor at natural origin", 8805),
    EsriParam::value("Latitude_Of_Origin", "Latitude of natural origin", 8801),
    EsriParam::fixed("Option", "1.0"),
];

pub(crate) static ESRI_METHODS: &[EsriMethod] = &[
    EsriMethod {
        esri_name: "Equidistant_Cylindrical",
        wkt2_name: "Equidistant Cylindrical",
        epsg_code: 1028,
        params: PARAMS_EQUIDISTANT_CYLINDRICAL,
    },
    EsriMethod {
        esri_name: "Plate_Carree",
        wkt2_name: "Equidistant Cylindrical",
        epsg_code: 1028,
        params: PARAMS_PLATE_CARREE,
    },
    EsriMethod {
        esri_name: "Plate_Carree",
        wkt2_name: "Equidistant Cylindrical (Spherical)",
        epsg_code: 1029,
        params: PARAMS_PLATE_CARREE,
    },
    EsriMethod {
        esri_name: "Miller_Cylindrical",
        wkt2_name: "Miller Cylindrical",
        epsg_code: 0,
        params: PARAMS_MILLER_CYLINDRICAL,
    },
    EsriMethod {
        esri_name: "Mercator",
        wkt2_name: "Mercator (variant B)",
        epsg_code: 9805,
        params: PARAMS_MERCATOR,
    },
    EsriMethod {
        esri_name: "Gauss_Kruger",
        wkt2_name: "Transverse Mercator",
        epsg_code: 9807,
        params: PARAMS_GAUSS_KRUGER,
    },
    EsriMethod {
        esri_name: "Transverse_Mercator",
        wkt2_name: "Transverse Mercator",
        epsg_code: 9807,
        params: PARAMS_TRANSVERSE_MERCATOR,
    },
    EsriMethod {
        esri_name: "Transverse_Mercator_Complex",
        wkt2_name: "Transverse Mercator",
        epsg_code: 9807,
        params: PARAMS_TRANSVERSE_MERCATOR_COMPLEX,
    },
    EsriMethod {
        esri_name: "Albers",
        wkt2_name: "Albers Equal Area",
        epsg_code: 9822,
        params: PARAMS_ALBERS,
    },
    EsriMethod {
        esri_name: "Sinusoidal",
        wkt2_name: "Sinusoidal",
        epsg_code: 0,
        params: PARAMS_SINUSOIDAL,
    },
    EsriMethod {
        esri_name: "Mollweide",
        wkt2_name: "Mollweide",
        epsg_code: 0,
        params: PARAMS_MOLLWEIDE,
    },
    EsriMethod {
        esri_name: "Eckert_I",
        wkt2_name: "Eckert I",
        epsg_code: 0,
        params: PARAMS_ECKERT_I,
    },
    EsriMethod {
        esri_name: "Eckert_II",
        wkt2_name: "Eckert II",
        epsg_code: 0,
        params: PARAMS_ECKERT_II,
    },
    EsriMethod {
        esri_name: "Eckert_III",
        wkt2_name: "Eckert III",
        epsg_code: 0,
        params: PARAMS_ECKERT_III,
    },
    EsriMethod {
        esri_name: "Eckert_IV",
        wkt2_name: "Eckert IV",
        epsg_code: 0,
        params: PARAMS_ECKERT_IV,
    },
    EsriMethod {
        esri_name: "Eckert_V",
        wkt2_name: "Eckert V",
        epsg_code: 0,
        params: PARAMS_ECKERT_V,
    },
    EsriMethod {
        esri_name: "Eckert_VI",
        wkt2_name: "Eckert VI",
        epsg_code: 0,
        params: PARAMS_ECKERT_VI,
    },
    EsriMethod {
        esri_name: "Gall_Stereographic",
        wkt2_name: "Gall Stereographic",
        epsg_code: 0,
        params: PARAMS_GALL_STEREOGRAPHIC,
    },
    EsriMethod {
        esri_name: "Winkel_I",
        wkt2_name: "Winkel I",
        epsg_code: 0,
        params: PARAMS_WINKEL_I,
    },
    EsriMethod {
        esri_name: "Winkel_II",
        wkt2_name: "Winkel II",
        epsg_code: 0,
        params: PARAMS_WINKEL_II,
    },
    EsriMethod {
        esri_name: "Lambert_Conformal_Conic",
        wkt2_name: "Lambert Conic Conformal (1SP)",
        epsg_code: 9801,
        params: PARAMS_LAMBERT_CONFORMAL_CONIC_ALT1,
    },
    EsriMethod {
        esri_name: "Lambert_Conformal_Conic",
        wkt2_name: "Lambert Conic Conformal (2SP)",
        epsg_code: 9802,
        params: PARAMS_LAMBERT_CONFORMAL_CONIC_ALT2,
    },
    EsriMethod {
        esri_name: "Lambert_Conformal_Conic",
        wkt2_name: "Lambert Conic Conformal (2SP)",
        epsg_code: 9802,
        params: PARAMS_LAMBERT_CONFORMAL_CONIC_ALT3,
    },
    EsriMethod {
        esri_name: "Lambert_Conformal_Conic",
        wkt2_name: "Lambert Conic Conformal (2SP Michigan)",
        epsg_code: 1051,
        params: PARAMS_LAMBERT_CONFORMAL_CONIC_ALT4,
    },
    EsriMethod {
        esri_name: "Polyconic",
        wkt2_name: "American Polyconic",
        epsg_code: 9818,
        params: PARAMS_POLYCONIC,
    },
    EsriMethod {
        esri_name: "Quartic_Authalic",
        wkt2_name: "Quartic Authalic",
        epsg_code: 0,
        params: PARAMS_QUARTIC_AUTHALIC,
    },
    EsriMethod {
        esri_name: "Loximuthal",
        wkt2_name: "Loximuthal",
        epsg_code: 0,
        params: PARAMS_LOXIMUTHAL,
    },
    EsriMethod {
        esri_name: "Bonne",
        wkt2_name: "Bonne",
        epsg_code: 9827,
        params: PARAMS_BONNE,
    },
    EsriMethod {
        esri_name: "Hotine_Oblique_Mercator_Two_Point_Natural_Origin",
        wkt2_name: "Hotine Oblique Mercator Two Point Natural Origin",
        epsg_code: 0,
        params: PARAMS_HOTINE_OBLIQUE_MERCATOR_TWO_POINT_NATURAL_ORIGIN,
    },
    EsriMethod {
        esri_name: "Stereographic",
        wkt2_name: "Stereographic",
        epsg_code: 0,
        params: PARAMS_STEREOGRAPHIC,
    },
    EsriMethod {
        esri_name: "Polar_Stereographic_Variant_A",
        wkt2_name: "Polar Stereographic (variant A)",
        epsg_code: 9810,
        params: PARAMS_POLAR_STEREOGRAPHIC_VARIANT_A,
    },
    EsriMethod {
        esri_name: "Equidistant_Conic",
        wkt2_name: "Equidistant Conic",
        epsg_code: 1119,
        params: PARAMS_EQUIDISTANT_CONIC,
    },
    EsriMethod {
        esri_name: "Cassini",
        wkt2_name: "Cassini-Soldner",
        epsg_code: 9806,
        params: PARAMS_CASSINI,
    },
    EsriMethod {
        esri_name: "Van_der_Grinten_I",
        wkt2_name: "Van Der Grinten",
        epsg_code: 0,
        params: PARAMS_VAN_DER_GRINTEN_I,
    },
    EsriMethod {
        esri_name: "Robinson",
        wkt2_name: "Robinson",
        epsg_code: 0,
        params: PARAMS_ROBINSON,
    },
    EsriMethod {
        esri_name: "Two_Point_Equidistant",
        wkt2_name: "Two Point Equidistant",
        epsg_code: 0,
        params: PARAMS_TWO_POINT_EQUIDISTANT,
    },
    EsriMethod {
        esri_name: "Azimuthal_Equidistant",
        wkt2_name: "Azimuthal Equidistant",
        epsg_code: 1125,
        params: PARAMS_AZIMUTHAL_EQUIDISTANT,
    },
    EsriMethod {
        esri_name: "Lambert_Azimuthal_Equal_Area",
        wkt2_name: "Lambert Azimuthal Equal Area",
        epsg_code: 9820,
        params: PARAMS_LAMBERT_AZIMUTHAL_EQUAL_AREA,
    },
    EsriMethod {
        esri_name: "Cylindrical_Equal_Area",
        wkt2_name: "Lambert Cylindrical Equal Area",
        epsg_code: 9835,
        params: PARAMS_CYLINDRICAL_EQUAL_AREA,
    },
    EsriMethod {
        esri_name: "Behrmann",
        wkt2_name: "Lambert Cylindrical Equal Area",
        epsg_code: 9835,
        params: PARAMS_BEHRMANN,
    },
    EsriMethod {
        esri_name: "Hotine_Oblique_Mercator_Two_Point_Center",
        wkt2_name: "Hotine Oblique Mercator Two Point Natural Origin",
        epsg_code: 0,
        params: PARAMS_HOTINE_OBLIQUE_MERCATOR_TWO_POINT_CENTER,
    },
    EsriMethod {
        esri_name: "Hotine_Oblique_Mercator_Azimuth_Natural_Origin",
        wkt2_name: "Hotine Oblique Mercator (variant A)",
        epsg_code: 9812,
        params: PARAMS_HOTINE_OBLIQUE_MERCATOR_AZIMUTH_NATURAL_ORIGIN,
    },
    EsriMethod {
        esri_name: "Hotine_Oblique_Mercator_Azimuth_Center",
        wkt2_name: "Hotine Oblique Mercator (variant B)",
        epsg_code: 9815,
        params: PARAMS_HOTINE_OBLIQUE_MERCATOR_AZIMUTH_CENTER,
    },
    EsriMethod {
        esri_name: "Double_Stereographic",
        wkt2_name: "Oblique Stereographic",
        epsg_code: 9809,
        params: PARAMS_DOUBLE_STEREOGRAPHIC,
    },
    EsriMethod {
        esri_name: "Krovak",
        wkt2_name: "Krovak",
        epsg_code: 9819,
        params: PARAMS_KROVAK_ALT1,
    },
    EsriMethod {
        esri_name: "Krovak",
        wkt2_name: "Krovak (North Orientated)",
        epsg_code: 1041,
        params: PARAMS_KROVAK_ALT2,
    },
    EsriMethod {
        esri_name: "New_Zealand_Map_Grid",
        wkt2_name: "New Zealand Map Grid",
        epsg_code: 9811,
        params: PARAMS_NEW_ZEALAND_MAP_GRID,
    },
    EsriMethod {
        esri_name: "Orthographic",
        wkt2_name: "Orthographic (Spherical)",
        epsg_code: 0,
        params: PARAMS_ORTHOGRAPHIC,
    },
    EsriMethod {
        esri_name: "Local",
        wkt2_name: "Local Orthographic",
        epsg_code: 1130,
        params: PARAMS_LOCAL,
    },
    EsriMethod {
        esri_name: "Winkel_Tripel",
        wkt2_name: "Winkel Tripel",
        epsg_code: 0,
        params: PARAMS_WINKEL_TRIPEL,
    },
    EsriMethod {
        esri_name: "Aitoff",
        wkt2_name: "Aitoff",
        epsg_code: 0,
        params: PARAMS_AITOFF,
    },
    EsriMethod {
        esri_name: "Flat_Polar_Quartic",
        wkt2_name: "Flat Polar Quartic",
        epsg_code: 0,
        params: PARAMS_FLAT_POLAR_QUARTIC,
    },
    EsriMethod {
        esri_name: "Craster_Parabolic",
        wkt2_name: "Craster Parabolic",
        epsg_code: 0,
        params: PARAMS_CRASTER_PARABOLIC,
    },
    EsriMethod {
        esri_name: "Gnomonic",
        wkt2_name: "Gnomonic",
        epsg_code: 0,
        params: PARAMS_GNOMONIC,
    },
    EsriMethod {
        esri_name: "Times",
        wkt2_name: "Times",
        epsg_code: 0,
        params: PARAMS_TIMES,
    },
    EsriMethod {
        esri_name: "Vertical_Near_Side_Perspective",
        wkt2_name: "Vertical Perspective",
        epsg_code: 9838,
        params: PARAMS_VERTICAL_NEAR_SIDE_PERSPECTIVE,
    },
    EsriMethod {
        esri_name: "Stereographic_North_Pole",
        wkt2_name: "Polar Stereographic (variant B)",
        epsg_code: 9829,
        params: PARAMS_STEREOGRAPHIC_NORTH_POLE,
    },
    EsriMethod {
        esri_name: "Stereographic_South_Pole",
        wkt2_name: "Polar Stereographic (variant B)",
        epsg_code: 9829,
        params: PARAMS_STEREOGRAPHIC_SOUTH_POLE,
    },
    EsriMethod {
        esri_name: "Rectified_Skew_Orthomorphic_Natural_Origin",
        wkt2_name: "Hotine Oblique Mercator (variant A)",
        epsg_code: 9812,
        params: PARAMS_RECTIFIED_SKEW_ORTHOMORPHIC_NATURAL_ORIGIN,
    },
    EsriMethod {
        esri_name: "Rectified_Skew_Orthomorphic_Center",
        wkt2_name: "Hotine Oblique Mercator (variant B)",
        epsg_code: 9815,
        params: PARAMS_RECTIFIED_SKEW_ORTHOMORPHIC_CENTER,
    },
    EsriMethod {
        esri_name: "Goode_Homolosine",
        wkt2_name: "Goode Homolosine",
        epsg_code: 0,
        params: PARAMS_GOODE_HOMOLOSINE_ALT1,
    },
    EsriMethod {
        esri_name: "Goode_Homolosine",
        wkt2_name: "Interrupted Goode Homolosine",
        epsg_code: 0,
        params: PARAMS_GOODE_HOMOLOSINE_ALT2,
    },
    EsriMethod {
        esri_name: "Goode_Homolosine",
        wkt2_name: "Interrupted Goode Homolosine Ocean",
        epsg_code: 0,
        params: PARAMS_GOODE_HOMOLOSINE_ALT3,
    },
    EsriMethod {
        esri_name: "Equidistant_Cylindrical_Ellipsoidal",
        wkt2_name: "Equidistant Cylindrical",
        epsg_code: 1028,
        params: PARAMS_EQUIDISTANT_CYLINDRICAL_ELLIPSOIDAL,
    },
    EsriMethod {
        esri_name: "Laborde_Oblique_Mercator",
        wkt2_name: "Laborde Oblique Mercator",
        epsg_code: 9813,
        params: PARAMS_LABORDE_OBLIQUE_MERCATOR,
    },
    EsriMethod {
        esri_name: "Gnomonic_Ellipsoidal",
        wkt2_name: "Gnomonic",
        epsg_code: 0,
        params: PARAMS_GNOMONIC_ELLIPSOIDAL,
    },
    EsriMethod {
        esri_name: "Wagner_IV",
        wkt2_name: "Wagner IV",
        epsg_code: 0,
        params: PARAMS_WAGNER_IV,
    },
    EsriMethod {
        esri_name: "Wagner_V",
        wkt2_name: "Wagner V",
        epsg_code: 0,
        params: PARAMS_WAGNER_V,
    },
    EsriMethod {
        esri_name: "Wagner_VII",
        wkt2_name: "Wagner VII",
        epsg_code: 0,
        params: PARAMS_WAGNER_VII,
    },
    EsriMethod {
        esri_name: "Natural_Earth",
        wkt2_name: "Natural Earth",
        epsg_code: 0,
        params: PARAMS_NATURAL_EARTH,
    },
    EsriMethod {
        esri_name: "Natural_Earth_II",
        wkt2_name: "Natural Earth II",
        epsg_code: 0,
        params: PARAMS_NATURAL_EARTH_II,
    },
    EsriMethod {
        esri_name: "Patterson",
        wkt2_name: "Patterson",
        epsg_code: 0,
        params: PARAMS_PATTERSON,
    },
    EsriMethod {
        esri_name: "Compact_Miller",
        wkt2_name: "Compact Miller",
        epsg_code: 0,
        params: PARAMS_COMPACT_MILLER,
    },
    EsriMethod {
        esri_name: "Geostationary_Satellite",
        wkt2_name: "Geostationary Satellite (Sweep Y)",
        epsg_code: 0,
        params: PARAMS_GEOSTATIONARY_SATELLITE,
    },
    EsriMethod {
        esri_name: "Mercator_Auxiliary_Sphere",
        wkt2_name: "Popular Visualisation Pseudo Mercator",
        epsg_code: 1024,
        params: PARAMS_MERCATOR_AUXILIARY_SPHERE,
    },
    EsriMethod {
        esri_name: "Mercator_Variant_A",
        wkt2_name: "Mercator (variant A)",
        epsg_code: 9804,
        params: PARAMS_MERCATOR_VARIANT_A,
    },
    EsriMethod {
        esri_name: "Mercator_Variant_C",
        wkt2_name: "Mercator (variant B)",
        epsg_code: 9805,
        params: PARAMS_MERCATOR_VARIANT_C,
    },
    EsriMethod {
        esri_name: "Transverse_Cylindrical_Equal_Area",
        wkt2_name: "Transverse Cylindrical Equal Area",
        epsg_code: 0,
        params: PARAMS_TRANSVERSE_CYLINDRICAL_EQUAL_AREA,
    },
    EsriMethod {
        esri_name: "IGAC_Plano_Cartesiano",
        wkt2_name: "Colombia Urban",
        epsg_code: 1052,
        params: PARAMS_IGAC_PLANO_CARTESIANO,
    },
    EsriMethod {
        esri_name: "Equal_Earth",
        wkt2_name: "Equal Earth",
        epsg_code: 1078,
        params: PARAMS_EQUAL_EARTH,
    },
    EsriMethod {
        esri_name: "Peirce_Quincuncial",
        wkt2_name: "Peirce Quincuncial (Square)",
        epsg_code: 0,
        params: PARAMS_PEIRCE_QUINCUNCIAL_ALT1,
    },
    EsriMethod {
        esri_name: "Peirce_Quincuncial",
        wkt2_name: "Peirce Quincuncial (Diamond)",
        epsg_code: 0,
        params: PARAMS_PEIRCE_QUINCUNCIAL_ALT2,
    },
];

/// PROJ `getESRIMapping`: the first row with this EPSG code or WKT2 name.
fn first_mapping(name: &str, epsg_code: i32) -> Option<&'static EsriMethod> {
    ESRI_METHODS.iter().find(|method| {
        (epsg_code != 0 && method.epsg_code == epsg_code)
            || method.wkt2_name.eq_ignore_ascii_case(name)
    })
}

/// PROJ `getESRIMethodNameAndParams`: the ESRI name and parameters of a conversion
/// with method `name` and EPSG code `epsg_code`, exported for the CRS named
/// `target_name`. `None` when the method has no ESRI name.
pub(crate) fn esri_method(
    conversion: &Conversion,
    name: &str,
    epsg_code: i32,
    target_name: Option<&str>,
) -> Option<EsriChoice> {
    let mapping = first_mapping(name, epsg_code)?;
    let target_has = |text: &str| target_name.is_some_and(|target| contains_ci(target, text));
    let choice = match mapping.epsg_code {
        1028 | 1029 => {
            if target_has("Plate Carree") && si_value(conversion, 8801) == 0.0 {
                EsriChoice {
                    name: "Plate_Carree",
                    params: PARAMS_PLATE_CARREE,
                }
            } else {
                EsriChoice {
                    name: "Equidistant_Cylindrical",
                    params: PARAMS_EQUIDISTANT_CYLINDRICAL,
                }
            }
        }
        9807 => {
            if contains_ci(conversion.name(), "Gauss Kruger")
                || target_has("Gauss")
                || target_has("GK_")
            {
                EsriChoice {
                    name: "Gauss_Kruger",
                    params: PARAMS_GAUSS_KRUGER,
                }
            } else {
                EsriChoice {
                    name: "Transverse_Mercator",
                    params: PARAMS_TRANSVERSE_MERCATOR,
                }
            }
        }
        9812 => {
            if rectified_angle_is_azimuth(conversion) {
                EsriChoice {
                    name: "Hotine_Oblique_Mercator_Azimuth_Natural_Origin",
                    params: PARAMS_HOTINE_OBLIQUE_MERCATOR_AZIMUTH_NATURAL_ORIGIN,
                }
            } else {
                EsriChoice {
                    name: "Rectified_Skew_Orthomorphic_Natural_Origin",
                    params: PARAMS_RECTIFIED_SKEW_ORTHOMORPHIC_NATURAL_ORIGIN,
                }
            }
        }
        9815 => {
            if rectified_angle_is_azimuth(conversion) {
                EsriChoice {
                    name: "Hotine_Oblique_Mercator_Azimuth_Center",
                    params: PARAMS_HOTINE_OBLIQUE_MERCATOR_AZIMUTH_CENTER,
                }
            } else {
                EsriChoice {
                    name: "Rectified_Skew_Orthomorphic_Center",
                    params: PARAMS_RECTIFIED_SKEW_ORTHOMORPHIC_CENTER,
                }
            }
        }
        9810 => {
            let ups = target_name.is_some_and(|target| {
                target == "WGS 84 / UPS North (E,N)" || target == "WGS 84 / UPS South (E,N)"
            });
            EsriChoice {
                name: if ups {
                    "Polar_Stereographic_Variant_A"
                } else {
                    "Stereographic"
                },
                params: mapping.params,
            }
        }
        9829 => EsriChoice {
            name: if si_value(conversion, 8832) > 0.0 {
                "Stereographic_North_Pole"
            } else {
                "Stereographic_South_Pole"
            },
            params: mapping.params,
        },
        9835 => EsriChoice {
            name: if (numeric(conversion, 8823, &UnitOfMeasure::degree()) - 30.0).abs() < 1e-10 {
                "Behrmann"
            } else {
                "Cylindrical_Equal_Area"
            },
            params: mapping.params,
        },
        _ => EsriChoice {
            name: mapping.esri_name,
            params: mapping.params,
        },
    };
    Some(choice)
}

/// Hotine Oblique Mercator, where the azimuth is the rectified grid angle.
fn rectified_angle_is_azimuth(conversion: &Conversion) -> bool {
    (si_value(conversion, 8813) - si_value(conversion, 8814)).abs() < 1e-15
}

/// PROJ `SingleOperation::parameterValue`: the parameter with EPSG code
/// `epsg_code`, else the one named `wkt2_name`.
pub(crate) fn find_parameter<'a>(
    conversion: &'a Conversion,
    wkt2_name: &str,
    epsg_code: i32,
) -> Option<&'a Parameter> {
    let by_code = conversion
        .parameters
        .iter()
        .find(|parameter| epsg_code != 0 && parameter.code.parse::<i32>().ok() == Some(epsg_code));
    by_code.or_else(|| {
        conversion
            .parameters
            .iter()
            .find(|parameter| names_match(&parameter.name, wkt2_name))
    })
}

fn measure_by_code(conversion: &Conversion, code: i32) -> Option<&Measure> {
    let parameter = conversion
        .parameters
        .iter()
        .find(|parameter| parameter.code.parse::<i32>().ok() == Some(code))?;
    match &parameter.value {
        ParameterValue::Measure(measure) => Some(measure),
        _ => None,
    }
}

/// PROJ `parameterValueNumericAsSI`: 0 when the parameter is missing.
fn si_value(conversion: &Conversion, code: i32) -> f64 {
    measure_by_code(conversion, code).map_or(0.0, Measure::si_value)
}

/// PROJ `parameterValueNumeric`: 0 when the parameter is missing.
fn numeric(conversion: &Conversion, code: i32, unit: &UnitOfMeasure) -> f64 {
    measure_by_code(conversion, code).map_or(0.0, |measure| measure.convert_to(unit))
}

fn contains_ci(text: &str, needle: &str) -> bool {
    text.to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

/// PROJ `Ellipsoid::squaredEccentricity`.
pub(crate) fn squared_eccentricity(ellipsoid: &Ellipsoid) -> f64 {
    let inverse_flattening = ellipsoid.inverse_flattening();
    let flattening = if inverse_flattening != 0.0 {
        1.0 / inverse_flattening
    } else {
        0.0
    };
    flattening * (2.0 - flattening)
}

/// PROJ `Conversion::convertToOtherMethod` from Mercator variant A (1SP) to
/// variant B (2SP), which the WKT1 ESRI export uses. `None` when PROJ does not
/// convert it: no base ellipsoid, a non-zero latitude of origin, or a scale
/// factor outside (0, 1].
pub(crate) fn mercator_variant_b(
    conversion: &Conversion,
    squared_eccentricity: Option<f64>,
) -> Result<Option<Conversion>, FactoryError> {
    if conversion.method.code != "9804" {
        return Ok(None);
    }
    let Some(e2) = squared_eccentricity else {
        return Ok(None);
    };
    if si_value(conversion, 8801) != 0.0 {
        return Ok(None);
    }
    let k0 = si_value(conversion, 8805);
    if !(k0 > 0.0 && k0 <= 1.0 + 1e-10) {
        return Ok(None);
    }
    let phi = if k0 >= 1.0 {
        0.0
    } else {
        ((1.0 - e2) / (1.0 / (k0 * k0) - e2)).sqrt().acos()
    };
    let latitude = phi / UnitOfMeasure::degree().factor;
    Conversion::create_mercator_variant_b(
        &OpProps::new(),
        latitude,
        numeric(conversion, 8802, &UnitOfMeasure::degree()),
        si_value(conversion, 8806),
        si_value(conversion, 8807),
    )
    .map(Some)
}
