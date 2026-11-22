//! `OperationParameter::getEPSGCode` (singleoperation.cpp:1210-1233): the EPSG code of a
//! parameter, read from its identifier, or else found by its name in `gParamNameCodes`
//! (operation/parammappings.cpp), in the order of that table, as PROJ 9.9.0 has it.

use super::equiv::equivalent_name_bigger_differences;
use super::model::Parameter;

/// `gParamNameCodes` of PROJ 9.9.0: the EPSG name and code of each parameter it maps.
const PARAM_NAME_CODES: &[(&str, i32)] = &[
    ("Co-latitude of cone axis", 1036),
    ("Latitude of natural origin", 8801),
    ("Longitude of natural origin", 8802),
    ("Scale factor at natural origin", 8805),
    ("False easting", 8806),
    ("False northing", 8807),
    ("Latitude of projection centre", 8811),
    ("Longitude of projection centre", 8812),
    ("Azimuth at projection centre", 8813),
    ("Angle from Rectified to Skew Grid", 8814),
    ("Scale factor at projection centre", 8815),
    ("Easting at projection centre", 8816),
    ("Northing at projection centre", 8817),
    ("Latitude of pseudo standard parallel", 8818),
    ("Scale factor on pseudo standard parallel", 8819),
    ("Latitude of false origin", 8821),
    ("Longitude of false origin", 8822),
    ("Latitude of 1st standard parallel", 8823),
    ("Latitude of 2nd standard parallel", 8824),
    ("Easting at false origin", 8826),
    ("Northing at false origin", 8827),
    ("Latitude of standard parallel", 8832),
    ("Longitude of origin", 8833),
    ("Ellipsoid scaling factor", 1038),
    ("Projection plane origin height", 1039),
    ("Geocentric X of topocentric origin", 8837),
    ("Geocentric Y of topocentric origin", 8838),
    ("Geocentric Z of topocentric origin", 8839),
    ("Semi-major axis length difference", 8654),
    ("Flattening difference", 8655),
    ("Latitude and longitude difference file", 8656),
    ("Geoid (height correction) model file", 8666),
    ("Vertical offset file", 8732),
    ("Geoid model difference file", 1063),
    ("Latitude difference file", 8657),
    ("Longitude difference file", 8658),
    ("Unit conversion scalar", 1051),
    ("Latitude offset", 8601),
    ("Longitude offset", 8602),
    ("Vertical Offset", 8603),
    ("Easting offset", 8728),
    ("Northing offset", 8729),
    ("Geoid height", 8604),
    ("Geoid undulation", 8604),
    ("A0", 8623),
    ("A1", 8624),
    ("A2", 8625),
    ("B0", 8639),
    ("B1", 8640),
    ("B2", 8641),
    ("X-axis translation", 8605),
    ("Y-axis translation", 8606),
    ("Z-axis translation", 8607),
    ("X-axis rotation", 8608),
    ("Y-axis rotation", 8609),
    ("Z-axis rotation", 8610),
    ("Scale difference", 8611),
    ("Rate of change of X-axis translation", 1040),
    ("Rate of change of Y-axis translation", 1041),
    ("Rate of change of Z-axis translation", 1042),
    ("Rate of change of X-axis rotation", 1043),
    ("Rate of change of Y-axis rotation", 1044),
    ("Rate of change of Z-axis rotation", 1045),
    ("Rate of change of Scale difference", 1046),
    ("Parameter reference epoch", 1047),
    ("Transformation reference epoch", 1049),
    ("Ordinate 1 of evaluation point", 8617),
    ("Ordinate 2 of evaluation point", 8618),
    ("Ordinate 3 of evaluation point", 8667),
    ("Bin grid origin I", 8733),
    ("Bin grid origin J", 8734),
    ("Bin grid origin Easting", 8735),
    ("Bin grid origin Northing", 8736),
    ("Scale factor of bin grid", 8737),
    ("Bin width on I-axis", 8738),
    ("Bin width on J-axis", 8739),
    ("Map grid bearing of bin grid J-axis", 8740),
    ("Bin node increment on I-axis", 8741),
    ("Bin node increment on J-axis", 8742),
    ("Geocentric translation file", 8727),
    ("Inclination in latitude", 8730),
    ("Inclination in longitude", 8731),
    ("EPSG code for Horizontal CRS", 1037),
    ("EPSG code for Interpolation CRS", 1048),
    ("Point motion velocity grid file", 1050),
    ("Point motion velocity north grid file", 1072),
    ("Source epoch", 1068),
    ("Target epoch", 1069),
    ("Initial longitude", 8830),
    ("Zone width", 8831),
];

impl Parameter {
    /// The EPSG code of this parameter, as `OperationParameter::getEPSGCode` gives it: the code
    /// of an `EPSG` identifier, or else the code of an equivalent name in `gParamNameCodes`, or
    /// of `Latitude of origin` and `Scale factor`. Zero when none matches.
    #[must_use]
    pub fn epsg_code(&self) -> i32 {
        if self.code_space().eq_ignore_ascii_case("EPSG") {
            if let Ok(code) = self.code().parse() {
                return code;
            }
        }
        let name = self.name();
        if let Some((_, code)) = PARAM_NAME_CODES
            .iter()
            .find(|(known, _)| equivalent_name_bigger_differences(name, known))
        {
            return *code;
        }
        if equivalent_name_bigger_differences(name, "Latitude of origin") {
            return 8801;
        }
        if equivalent_name_bigger_differences(name, "Scale factor") {
            return 8805;
        }
        0
    }
}
