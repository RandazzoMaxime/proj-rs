//! Named CRS used by [`crate::Proj::new_known_crs`].
//!
//! EPSG codes are read from the embedded PROJ 9.9.0 `proj.db`. Axis order on
//! the public boundary is longitude, latitude and easting, northing. A
//! conversion or datum step is built only when every operation is one this
//! crate already accepts. A grid is applied when its file is on the search
//! path. A missing grid, a deformation model, or any other method returns
//! [`ProjCreateError`] and no coordinate. [`epsg_pipeline`] is the EPSG-axis
//! form of that rule, including a time-dependent Helmert when the registered
//! operation is one the Helmert kernel already accepts.

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::engine::{CreateError, Operation};
use crate::proj::ProjCreateError;
use crate::projdb::{with_db, DbFail};
use crate::Xyzt;

/// How a CRS presents coordinates to the georust facade.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Kind {
    /// Longitude and latitude, degrees on the public boundary.
    Geographic,
    /// Easting and northing, metres (or the CRS linear unit).
    Projected,
    /// Geocentric X, Y, Z, metres.
    Geocentric,
    /// Topocentric east, north, up, metres, about a geodetic or geocentric origin.
    Topocentric,
}

/// One coordinate reference system the facade can build.
#[derive(Clone, Debug)]
pub(crate) struct Crs {
    kind: Kind,
    op: Operation,
    datum_auth: String,
    datum_code: String,
    /// Semi-major axis, metres, for a datum Helmert.
    a: f64,
    /// Inverse flattening. Absent when the ellipsoid is given by `b`.
    rf: Option<f64>,
    /// Semi-minor axis, metres, when inverse flattening is absent.
    b: Option<f64>,
}

impl Crs {
    /// Parse an `AUTHORITY:CODE` or a PROJ string that names one CRS.
    ///
    /// # Errors
    ///
    /// [`ProjCreateError`] when the code is absent, needs a method this crate
    /// does not build, or the PROJ string does not build.
    pub(crate) fn parse(text: &str) -> Result<Self, ProjCreateError> {
        let text = text.trim();
        if let Some(code) = authority_code(text) {
            return from_epsg(code);
        }
        if text.contains("proj=") {
            return from_proj(text);
        }
        Err(ProjCreateError::ProjError(format!("unknown CRS `{text}`")))
    }

    /// Geographic or projected CRS described by WKT or PROJJSON.
    ///
    /// # Errors
    ///
    /// [`ProjCreateError`] when the text is not one of those CRS kinds, or the
    /// conversion method is not built.
    pub(crate) fn from_well_known(text: &str) -> Result<Self, ProjCreateError> {
        let definition = crate::crs_text::to_proj(text).map_err(ProjCreateError::ProjError)?;
        from_proj(&definition)
    }

    /// Pipeline from geodetic radians on `self` to geodetic radians on `target`.
    ///
    /// `None` means the longitude and latitude are already on the target
    /// datum: the datums match, or the registered Helmert is all zeros.
    ///
    /// # Errors
    ///
    /// [`ProjCreateError`] when the datums differ and every registered
    /// operation needs a grid, a deformation model, or a method this crate
    /// does not build.
    pub(crate) fn datum_bridge(&self, target: &Self) -> Result<Option<Operation>, ProjCreateError> {
        if self.datum_auth.is_empty()
            || self.datum_code.is_empty()
            || target.datum_auth.is_empty()
            || target.datum_code.is_empty()
        {
            return Ok(None);
        }
        if self.datum_auth == target.datum_auth && self.datum_code == target.datum_code {
            return Ok(None);
        }
        let helmert = with_db(|connection| {
            select_helmert(
                connection,
                &self.datum_auth,
                &self.datum_code,
                &target.datum_auth,
                &target.datum_code,
            )
        })
        .map_err(db_fail)?;
        match helmert {
            HelmertPick::Identity => Ok(None),
            HelmertPick::Shift(shift) => {
                let definition = helmert_pipeline(self, target, &shift);
                Operation::create(&definition).map(Some).map_err(create_error)
            }
            HelmertPick::Unsupported => Err(ProjCreateError::ProjError(format!(
                "EPSG datum {} to {} needs a grid, a deformation model, or a method this crate does not build",
                self.datum_code, target.datum_code
            ))),
        }
    }

    pub(crate) fn is_geographic(&self) -> bool {
        self.kind == Kind::Geographic
    }

    pub(crate) fn is_geocentric(&self) -> bool {
        self.kind == Kind::Geocentric
    }

    /// Geodetic radians on this CRS's ellipsoid, height in `z`.
    ///
    /// `z` is the third public axis: height, geocentric Z, or topocentric Up.
    pub(crate) fn to_geodetic(
        &self,
        x: f64,
        y: f64,
        z: f64,
    ) -> Result<Xyzt, crate::engine::TransformError> {
        match self.kind {
            Kind::Geographic => Ok(Xyzt {
                x: x.to_radians(),
                y: y.to_radians(),
                z,
                t: 0.0,
            }),
            Kind::Projected | Kind::Geocentric | Kind::Topocentric => {
                self.op.inverse(Xyzt { x, y, z, t: 0.0 })
            }
        }
    }

    /// Public coordinates of this CRS from geodetic radians.
    pub(crate) fn geodetic_out(&self, coord: Xyzt) -> Result<Xyzt, crate::engine::TransformError> {
        match self.kind {
            Kind::Geographic => Ok(Xyzt {
                x: coord.x.to_degrees(),
                y: coord.y.to_degrees(),
                z: coord.z,
                t: 0.0,
            }),
            Kind::Projected | Kind::Geocentric | Kind::Topocentric => self.op.forward(coord),
        }
    }
}

fn authority_code(text: &str) -> Option<u32> {
    let (authority, code) = text.split_once(':')?;
    if !authority.eq_ignore_ascii_case("EPSG") {
        return None;
    }
    code.parse().ok()
}

fn from_epsg(code: u32) -> Result<Crs, ProjCreateError> {
    let loaded = with_db(|connection| load_crs(connection, code)).map_err(db_fail)?;
    let op = Operation::create(&loaded.definition).map_err(create_error)?;
    let kind = match loaded.kind {
        LoadedKind::Geographic => Kind::Geographic,
        LoadedKind::Geocentric => Kind::Geocentric,
        LoadedKind::Projected => {
            if op.angular_input(false) && !op.angular_output(false) {
                Kind::Projected
            } else {
                return Err(ProjCreateError::ProjError(format!(
                    "EPSG:{code} is not a projected CRS"
                )));
            }
        }
    };
    Ok(Crs {
        kind,
        op,
        datum_auth: loaded.datum_auth,
        datum_code: loaded.datum_code,
        a: loaded.a,
        rf: loaded.rf,
        b: loaded.b,
    })
}

fn from_proj(definition: &str) -> Result<Crs, ProjCreateError> {
    if definition.contains("proj=pipeline") {
        return Err(ProjCreateError::ProjError(
            "a pipeline is not a CRS".to_owned(),
        ));
    }
    if is_topocentric_crs(definition) {
        return from_topocentric(definition);
    }
    let op = Operation::create(definition).map_err(create_error)?;
    let kind = if definition.contains("proj=cart") || definition.contains("proj=geocent") {
        Kind::Geocentric
    } else if definition.contains("proj=longlat")
        || definition.contains("proj=latlong")
        || definition.contains("proj=latlon")
        || definition.contains("proj=lonlat")
    {
        Kind::Geographic
    } else if op.angular_input(false) && !op.angular_output(false) {
        Kind::Projected
    } else {
        return Err(ProjCreateError::ProjError(format!(
            "`{definition}` is not a geographic, projected, or geocentric CRS"
        )));
    };
    Ok(Crs {
        kind,
        op,
        datum_auth: String::new(),
        datum_code: String::new(),
        a: 0.0,
        rf: None,
        b: None,
    })
}

fn is_topocentric_crs(definition: &str) -> bool {
    let tokens: Vec<&str> = definition
        .split_whitespace()
        .map(|token| token.trim_start_matches('+'))
        .collect();
    tokens.contains(&"proj=topocentric") && tokens.contains(&"type=crs")
}

/// A topocentric CRS is three-dimensional. Its operation runs the geocentric
/// step first and the topocentric step second, so the forward direction goes
/// from geodetic to east, north, up, as PROJ's `buildProjectedCRS` does with a
/// geocentric base. The ellipsoid is set on the pipeline header, which every
/// step inherits; the origin stays on the topocentric step alone.
fn from_topocentric(definition: &str) -> Result<Crs, ProjCreateError> {
    let (ellipsoid, origin): (Vec<&str>, Vec<&str>) = definition
        .split_whitespace()
        .filter(|token| {
            let bare = token.trim_start_matches('+');
            !bare.starts_with("proj=") && bare != "type=crs"
        })
        .partition(|token| is_ellipsoid_key(token.trim_start_matches('+')));
    let pipeline = format!(
        "+proj=pipeline {} +step +proj=cart +step +proj=topocentric {}",
        ellipsoid.join(" "),
        origin.join(" ")
    );
    let op = Operation::create(&pipeline).map_err(create_error)?;
    Ok(Crs {
        kind: Kind::Topocentric,
        op,
        datum_auth: String::new(),
        datum_code: String::new(),
        a: 0.0,
        rf: None,
        b: None,
    })
}

fn is_ellipsoid_key(bare: &str) -> bool {
    let key = bare.split('=').next().unwrap_or_default();
    matches!(
        key,
        "ellps" | "datum" | "a" | "b" | "rf" | "f" | "es" | "e" | "R"
    )
}

fn create_error(error: CreateError) -> ProjCreateError {
    ProjCreateError::ProjError(error.to_string())
}

fn db_fail(error: DbFail) -> ProjCreateError {
    let message = match error {
        DbFail::Open(message) => message,
        DbFail::Sql(error) => error.to_string(),
    };
    ProjCreateError::ProjError(message)
}

struct Loaded {
    kind: LoadedKind,
    definition: String,
    datum_auth: String,
    datum_code: String,
    a: f64,
    rf: Option<f64>,
    b: Option<f64>,
}

enum LoadedKind {
    Geographic,
    Geocentric,
    Projected,
}

struct Earth {
    a: f64,
    rf: Option<f64>,
    b: Option<f64>,
    datum_auth: String,
    datum_code: String,
}

fn load_crs(connection: &Connection, code: u32) -> Result<Loaded, rusqlite::Error> {
    let kind_name: Option<(String, String)> = connection
        .query_row(
            "SELECT table_name, type FROM crs_view WHERE auth_name = 'EPSG' AND code = ?1",
            params![i64::from(code)],
            |row| Ok((column_text(row, 0)?, column_text(row, 1)?)),
        )
        .optional()?;
    let Some((table, crs_type)) = kind_name else {
        return Err(missing(code));
    };
    match table.as_str() {
        "geodetic_crs" => load_geodetic(connection, code, &crs_type),
        "projected_crs" => load_projected(connection, code),
        _ => Err(unsupported(format!(
            "EPSG:{code} ({crs_type}) is not built"
        ))),
    }
}

fn missing(code: u32) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("EPSG:{code} is not a CRS in proj.db"),
    )))
}

fn unsupported(message: String) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(message)))
}

fn load_geodetic(
    connection: &Connection,
    code: u32,
    crs_type: &str,
) -> Result<Loaded, rusqlite::Error> {
    let proj = match crs_type {
        "geographic 2D" | "geographic 3D" => "longlat",
        "geocentric" => "cart",
        _ => {
            return Err(unsupported(format!(
                "EPSG:{code} ({crs_type}) is not built"
            )))
        }
    };
    let kind = if proj == "cart" {
        LoadedKind::Geocentric
    } else {
        LoadedKind::Geographic
    };
    let earth = earth_of(connection, code)?;
    Ok(Loaded {
        definition: format!("+proj={proj} {} +no_defs", ellipsoid_clause(&earth)),
        kind,
        datum_auth: earth.datum_auth,
        datum_code: earth.datum_code,
        a: earth.a,
        rf: earth.rf,
        b: earth.b,
    })
}

fn load_projected(connection: &Connection, code: u32) -> Result<Loaded, rusqlite::Error> {
    let row = connection
        .query_row(
            "SELECT geodetic_crs_code, conversion_code, text_definition
             FROM projected_crs WHERE auth_name = 'EPSG' AND code = ?1",
            params![i64::from(code)],
            |row| {
                Ok((
                    column_text(row, 0)?,
                    column_text_opt(row, 1)?,
                    column_text_opt(row, 2)?,
                ))
            },
        )
        .optional()?;
    let Some((base_code, conversion, text)) = row else {
        return Err(missing(code));
    };
    if let Some(text) = text.filter(|item| !item.is_empty()) {
        if !text.contains("proj=") {
            return Err(unsupported(format!(
                "EPSG:{code} text definition is not a PROJ string"
            )));
        }
        let base = base_code.parse::<i64>().ok();
        let earth = if let Some(base) = base {
            earth_of(connection, u32_code(base)?)?
        } else {
            return Err(unsupported(format!("EPSG:{code} has no base geodetic CRS")));
        };
        return Ok(Loaded {
            kind: LoadedKind::Projected,
            definition: text,
            datum_auth: earth.datum_auth,
            datum_code: earth.datum_code,
            a: earth.a,
            rf: earth.rf,
            b: earth.b,
        });
    }
    let Some(conversion) = conversion else {
        return Err(unsupported(format!("EPSG:{code} has no conversion")));
    };
    let base = base_code
        .parse::<i64>()
        .map_err(|_| unsupported(format!("EPSG:{code} base CRS `{base_code}` is not numeric")))?;
    let earth = earth_of(connection, u32_code(base)?)?;
    let definition = conversion_proj(connection, &conversion, &earth, code)?;
    Ok(Loaded {
        kind: LoadedKind::Projected,
        definition,
        datum_auth: earth.datum_auth,
        datum_code: earth.datum_code,
        a: earth.a,
        rf: earth.rf,
        b: earth.b,
    })
}

fn u32_code(code: i64) -> Result<u32, rusqlite::Error> {
    u32::try_from(code).map_err(|_| unsupported(format!("CRS code {code} does not fit u32")))
}

fn earth_of(connection: &Connection, code: u32) -> Result<Earth, rusqlite::Error> {
    connection.query_row(
        "SELECT d.auth_name, d.code,
                e.semi_major_axis, e.inv_flattening, e.semi_minor_axis,
                u.conv_factor
         FROM geodetic_crs g
         JOIN geodetic_datum d
           ON d.auth_name = g.datum_auth_name AND d.code = g.datum_code
         JOIN ellipsoid e
           ON e.auth_name = d.ellipsoid_auth_name AND e.code = d.ellipsoid_code
         JOIN unit_of_measure u
           ON u.auth_name = e.uom_auth_name AND u.code = e.uom_code
         WHERE g.auth_name = 'EPSG' AND g.code = ?1",
        params![i64::from(code)],
        |row| {
            let factor = column_f64_opt(row, 5)?.unwrap_or(1.0);
            let a = column_f64(row, 2)? * factor;
            let rf = column_f64_opt(row, 3)?;
            let b = column_f64_opt(row, 4)?.map(|axis| axis * factor);
            Ok(Earth {
                a,
                rf,
                b,
                datum_auth: column_text(row, 0)?,
                datum_code: column_text(row, 1)?,
            })
        },
    )
}

struct RawParam {
    code: i64,
    value: f64,
    unit_type: String,
    conv: Option<f64>,
    unit_name: String,
}

fn conversion_proj(
    connection: &Connection,
    conversion: &str,
    earth: &Earth,
    crs: u32,
) -> Result<String, rusqlite::Error> {
    let (method, params) = conversion_row(connection, conversion)?;
    let Some(spec) = method_spec(method) else {
        return Err(unsupported(format!(
            "EPSG:{crs} method {method} is not built"
        )));
    };
    let mut parts = Vec::new();
    parts.push(format!("+proj={}", spec.proj));
    for flag in spec.flags {
        parts.push(format!("+{flag}"));
    }
    for param in params {
        match param_use(method, param.code) {
            ParamUse::Skip => {}
            ParamUse::Reject => {
                return Err(unsupported(format!(
                    "EPSG:{crs} parameter {} is not built",
                    param.code
                )));
            }
            ParamUse::Key(key) => {
                let number = param_number(&param, key).map_err(unsupported)?;
                if method == 9801 && param.code == 8801 {
                    parts.push(format!("+lat_0={}", format_f64(number)));
                    parts.push(format!("+lat_1={}", format_f64(number)));
                } else if method == 9804 && param.code == 8801 {
                    if number != 0.0 {
                        return Err(unsupported(format!(
                            "EPSG:{crs} Mercator latitude of origin {number} is not built"
                        )));
                    }
                } else {
                    parts.push(format!("+{key}={}", format_f64(number)));
                }
            }
        }
    }
    if spec.sphere_radius {
        parts.push(format!("+a={}", format_f64(earth.a)));
        parts.push(format!("+b={}", format_f64(earth.a)));
    } else {
        parts.push(ellipsoid_clause(earth));
    }
    parts.push("+units=m".to_owned());
    parts.push("+no_defs".to_owned());
    Ok(parts.join(" "))
}

fn conversion_row(
    connection: &Connection,
    code: &str,
) -> Result<(i64, Vec<RawParam>), rusqlite::Error> {
    let (method, slots) = connection.query_row(
        "SELECT method_code,
                param1_code, param1_value, param1_uom_auth_name, param1_uom_code,
                param2_code, param2_value, param2_uom_auth_name, param2_uom_code,
                param3_code, param3_value, param3_uom_auth_name, param3_uom_code,
                param4_code, param4_value, param4_uom_auth_name, param4_uom_code,
                param5_code, param5_value, param5_uom_auth_name, param5_uom_code,
                param6_code, param6_value, param6_uom_auth_name, param6_uom_code,
                param7_code, param7_value, param7_uom_auth_name, param7_uom_code
         FROM conversion WHERE auth_name = 'EPSG' AND code = ?1",
        params![code],
        |row| {
            let method = column_i64(row, 0)?;
            let mut slots = Vec::new();
            for slot in 0..7 {
                let base = 1 + slot * 4;
                let Some(param_code) = column_i64_opt(row, base)? else {
                    continue;
                };
                let Some(value) = column_f64_opt(row, base + 1)? else {
                    continue;
                };
                slots.push((
                    param_code,
                    value,
                    column_text_opt(row, base + 2)?,
                    column_text_opt(row, base + 3)?,
                ));
            }
            Ok((method, slots))
        },
    )?;
    let mut params = Vec::new();
    for (code, value, uom_auth, uom_code) in slots {
        let (unit_type, conv, unit_name) = match (uom_auth, uom_code) {
            (Some(auth), Some(unit)) => unit_of(connection, &auth, &unit)?,
            _ => (String::new(), None, String::new()),
        };
        params.push(RawParam {
            code,
            value,
            unit_type,
            conv,
            unit_name,
        });
    }
    Ok((method, params))
}

fn unit_of(
    connection: &Connection,
    auth: &str,
    code: &str,
) -> Result<(String, Option<f64>, String), rusqlite::Error> {
    connection.query_row(
        "SELECT type, conv_factor, name FROM unit_of_measure WHERE auth_name = ?1 AND code = ?2",
        params![auth, code],
        |row| {
            Ok((
                column_text(row, 0)?,
                column_f64_opt(row, 1)?,
                column_text(row, 2)?,
            ))
        },
    )
}

struct MethodSpec {
    proj: &'static str,
    flags: &'static [&'static str],
    /// Popular Visualisation Pseudo Mercator uses a sphere of radius `a`.
    sphere_radius: bool,
}

fn method_spec(method: i64) -> Option<MethodSpec> {
    let spec = |proj, flags: &'static [&'static str], sphere_radius| MethodSpec {
        proj,
        flags,
        sphere_radius,
    };
    Some(match method {
        9807 | 1111 => spec("tmerc", &[], false),
        9824 => spec("tmerczoned", &[], false),
        9822 => spec("aea", &[], false),
        9801 | 1102 | 9802 | 9803 | 1051 => spec("lcc", &[], false),
        1125 | 9832 => spec("aeqd", &[], false),
        9831 => spec("aeqd", &["guam"], false),
        9827 => spec("bonne", &[], false),
        9835 => spec("cea", &[], false),
        9834 => spec("cea", &["R_A"], false),
        9806 => spec("cass", &[], false),
        9833 => spec("cass", &["hyperbolic"], false),
        1119 => spec("eqdc", &[], false),
        1028 | 1029 => spec("eqc", &[], false),
        9812 => spec("omerc", &["no_uoff"], false),
        9815 => spec("omerc", &[], false),
        1041 => spec("krovak", &[], false),
        1043 => spec("mod_krovak", &[], false),
        9820 => spec("laea", &[], false),
        1027 => spec("laea", &["R_A"], false),
        9804 | 9805 => spec("merc", &[], false),
        1024 => spec("merc", &[], true),
        1026 => spec("merc", &["R_C"], false),
        9811 => spec("nzmg", &[], false),
        9809 => spec("sterea", &[], false),
        9840 | 1130 => spec("ortho", &[], false),
        9818 => spec("poly", &[], false),
        9810 | 9829 => spec("stere", &[], false),
        9813 => spec("labrd", &[], false),
        9838 => spec("nsper", &[], false),
        1052 => spec("col_urban", &[], false),
        9836 => spec("topocentric", &[], false),
        _ => return None,
    })
}

enum ParamUse {
    Key(&'static str),
    Skip,
    Reject,
}

fn param_use(method: i64, param: i64) -> ParamUse {
    if matches!(method, 1041 | 1043) && matches!(param, 1036 | 8818 | 8819) {
        return ParamUse::Skip;
    }
    if method == 9838 && param == 8836 {
        return ParamUse::Skip;
    }
    if method == 9827 && param == 8801 {
        return ParamUse::Key("lat_1");
    }
    if matches!(method, 9834 | 9835 | 1028 | 1029 | 9805) && param == 8823 {
        return ParamUse::Key("lat_ts");
    }
    if matches!(method, 9812 | 9815) && param == 8812 {
        return ParamUse::Key("lonc");
    }
    if method == 9813 && param == 8813 {
        return ParamUse::Key("azi");
    }
    if method == 1051 && param == 1038 {
        return ParamUse::Key("k_0");
    }
    if method == 9824 && param == 8830 {
        return ParamUse::Key("lon_0");
    }
    if method == 9824 && param == 8831 {
        return ParamUse::Key("width");
    }
    match param {
        8801 | 8821 | 8811 | 8834 => ParamUse::Key("lat_0"),
        8802 | 8822 | 8812 | 8833 | 8835 => ParamUse::Key("lon_0"),
        8805 | 8815 => ParamUse::Key("k_0"),
        8806 | 8826 | 8816 => ParamUse::Key("x_0"),
        8807 | 8827 | 8817 => ParamUse::Key("y_0"),
        8823 => ParamUse::Key("lat_1"),
        8824 => ParamUse::Key("lat_2"),
        8813 => ParamUse::Key("alpha"),
        8814 => ParamUse::Key("gamma"),
        8832 => ParamUse::Key("lat_ts"),
        8840 => ParamUse::Key("h"),
        1039 => ParamUse::Key("h_0"),
        8837 => ParamUse::Key("X_0"),
        8838 => ParamUse::Key("Y_0"),
        8839 => ParamUse::Key("Z_0"),
        _ => ParamUse::Reject,
    }
}

fn param_number(param: &RawParam, _key: &str) -> Result<f64, String> {
    match param.unit_type.as_str() {
        "angle" => to_degrees(param.value, param.conv, &param.unit_name),
        "length" => to_metres(param.value, param.conv),
        "scale" => to_scale(param.value, param.conv),
        "" => Ok(param.value),
        other => Err(format!("parameter unit type `{other}` is not built")),
    }
}

fn to_degrees(value: f64, conv: Option<f64>, name: &str) -> Result<f64, String> {
    if conv.is_none() || name.to_ascii_lowercase().contains("sexagesimal") {
        return sexagesimal_to_degrees(value);
    }
    let conv = conv.unwrap_or(1.0);
    // A degree unit's factor is π/180, but not bit-identical to `f64::to_radians`.
    // Multiplying and converting back turns -180 into -179.99999999999983, which
    // pushes the antimeridian into the wrong transverse-mercator zone.
    if (conv - std::f64::consts::PI / 180.0).abs() < 1e-12 {
        return Ok(value);
    }
    Ok((value * conv).to_degrees())
}

fn sexagesimal_to_degrees(value: f64) -> Result<f64, String> {
    // EPSG:9110 stores degrees, minutes and seconds in one real: DDD.MMSSsss.
    // `46.3` is 46°30'. The fractional part is not exact in binary, so the
    // minute field is floored with a small bias.
    let sign = if value.is_sign_negative() { -1.0 } else { 1.0 };
    let value = value.abs();
    let degrees = value.floor();
    let minute_scaled = (value - degrees) * 100.0;
    let minutes = (minute_scaled + 1e-8).floor();
    let mut seconds = (minute_scaled - minutes) * 100.0;
    if seconds < 0.0 {
        seconds = 0.0;
    }
    if minutes >= 60.0 || seconds >= 60.0 + 1e-4 {
        return Err(format!("sexagesimal value {value} is not a DMS angle"));
    }
    Ok(sign * (degrees + minutes / 60.0 + seconds / 3600.0))
}

fn to_metres(value: f64, conv: Option<f64>) -> Result<f64, String> {
    let factor = conv.ok_or_else(|| "length unit has no conversion factor".to_owned())?;
    Ok(value * factor)
}

fn to_scale(value: f64, conv: Option<f64>) -> Result<f64, String> {
    let factor = conv.unwrap_or(1.0);
    if (factor - 1.0).abs() < 1e-12 {
        Ok(value)
    } else {
        Ok(value * factor)
    }
}

fn ellipsoid_clause(earth: &Earth) -> String {
    if let Some(rf) = earth.rf.filter(|rf| *rf > 0.0) {
        format!("+a={} +rf={}", format_f64(earth.a), format_f64(rf))
    } else if let Some(b) = earth.b {
        format!("+a={} +b={}", format_f64(earth.a), format_f64(b))
    } else {
        format!("+a={} +b={}", format_f64(earth.a), format_f64(earth.a))
    }
}

fn format_f64(value: f64) -> String {
    let mut text = format!("{value:.15}");
    if let Some(dot) = text.find('.') {
        while text.ends_with('0') && text.len() > dot + 2 {
            text.pop();
        }
    }
    text
}

enum HelmertPick {
    Identity,
    Shift(Shift),
    Unsupported,
}

struct Shift {
    forward: bool,
    x: f64,
    y: f64,
    z: f64,
    rx: f64,
    ry: f64,
    rz: f64,
    scale: f64,
    convention: Option<&'static str>,
}

fn select_helmert(
    connection: &Connection,
    from_auth: &str,
    from_code: &str,
    to_auth: &str,
    to_code: &str,
) -> Result<HelmertPick, rusqlite::Error> {
    let from_datums = datum_family(connection, from_auth, from_code)?;
    let to_datums = datum_family(connection, to_auth, to_code)?;
    let mut binds = Vec::new();
    let from_sql = datum_predicate("s", &from_datums, &mut binds);
    let to_sql = datum_predicate("t", &to_datums, &mut binds);
    let from_rev = datum_predicate("t", &from_datums, &mut binds);
    let to_rev = datum_predicate("s", &to_datums, &mut binds);
    let sql = format!(
        "SELECT h.method_code, h.tx, h.ty, h.tz, h.rx, h.ry, h.rz, h.scale_difference,
                h.accuracy, h.epoch, h.px, h.py, h.pz,
                h.rate_tx, h.rate_ty, h.rate_tz, h.rate_rx, h.rate_ry, h.rate_rz,
                h.rate_scale_difference,
                h.translation_uom_auth_name, h.translation_uom_code,
                h.rotation_uom_auth_name, h.rotation_uom_code,
                h.scale_difference_uom_auth_name, h.scale_difference_uom_code,
                s.datum_auth_name, s.datum_code
         FROM helmert_transformation h
         JOIN geodetic_crs s
           ON s.auth_name = h.source_crs_auth_name AND s.code = h.source_crs_code
         JOIN geodetic_crs t
           ON t.auth_name = h.target_crs_auth_name AND t.code = h.target_crs_code
         WHERE h.deprecated = 0
           AND ((({from_sql}) AND ({to_sql})) OR (({from_rev}) AND ({to_rev})))"
    );
    let rows = {
        let mut statement = connection.prepare(&sql)?;
        let mapped =
            statement.query_map(rusqlite::params_from_iter(binds.iter()), read_raw_helmert)?;
        mapped.collect::<Result<Vec<_>, _>>()?
    };
    let Some(chosen) = rows
        .into_iter()
        .filter(|row| row.buildable)
        .min_by(|left, right| accuracy_key(left.accuracy).total_cmp(&accuracy_key(right.accuracy)))
    else {
        return Ok(HelmertPick::Unsupported);
    };
    if chosen.is_zero() {
        return Ok(HelmertPick::Identity);
    }
    let shift = chosen.shift(connection, &from_datums)?;
    Ok(HelmertPick::Shift(shift))
}

fn datum_predicate(alias: &str, datums: &[(String, String)], binds: &mut Vec<String>) -> String {
    let mut parts = Vec::new();
    for (auth, code) in datums {
        let auth_slot = binds.len() + 1;
        binds.push(auth.clone());
        let code_slot = binds.len() + 1;
        binds.push(code.clone());
        parts.push(format!(
            "({alias}.datum_auth_name = ?{auth_slot} AND {alias}.datum_code = ?{code_slot})"
        ));
    }
    if parts.is_empty() {
        return "0".to_owned();
    }
    parts.join(" OR ")
}

fn accuracy_key(accuracy: Option<f64>) -> f64 {
    accuracy.unwrap_or(f64::MAX)
}

struct RawHelmert {
    buildable: bool,
    accuracy: Option<f64>,
    tx: f64,
    ty: f64,
    tz: f64,
    rx: f64,
    ry: f64,
    rz: f64,
    scale: f64,
    convention: Option<&'static str>,
    source: (String, String),
    translation_auth: Option<String>,
    translation_code: Option<String>,
    rotation_auth: Option<String>,
    rotation_code: Option<String>,
    scale_auth: Option<String>,
    scale_code: Option<String>,
}

impl RawHelmert {
    fn is_zero(&self) -> bool {
        self.tx == 0.0
            && self.ty == 0.0
            && self.tz == 0.0
            && self.rx == 0.0
            && self.ry == 0.0
            && self.rz == 0.0
            && self.scale == 0.0
    }

    fn shift(
        &self,
        connection: &Connection,
        from_datums: &[(String, String)],
    ) -> Result<Shift, rusqlite::Error> {
        let translation = conv_of(connection, &self.translation_auth, &self.translation_code)?;
        let rotation = conv_of(connection, &self.rotation_auth, &self.rotation_code)?;
        let scale = conv_of(connection, &self.scale_auth, &self.scale_code)?;
        let x = apply_factor(self.tx, translation).map_err(unsupported)?;
        let y = apply_factor(self.ty, translation).map_err(unsupported)?;
        let z = apply_factor(self.tz, translation).map_err(unsupported)?;
        let rx = to_arcsec(self.rx, rotation).map_err(unsupported)?;
        let ry = to_arcsec(self.ry, rotation).map_err(unsupported)?;
        let rz = to_arcsec(self.rz, rotation).map_err(unsupported)?;
        let scale = to_ppm(self.scale, scale).map_err(unsupported)?;
        Ok(Shift {
            forward: from_datums.iter().any(|datum| datum == &self.source),
            x,
            y,
            z,
            rx,
            ry,
            rz,
            scale,
            convention: self.convention,
        })
    }
}

fn read_raw_helmert(row: &Row<'_>) -> Result<RawHelmert, rusqlite::Error> {
    let method = column_i64(row, 0)?;
    let epoch = column_f64_opt(row, 9)?;
    let px = column_f64_opt(row, 10)?.unwrap_or(0.0);
    let py = column_f64_opt(row, 11)?.unwrap_or(0.0);
    let pz = column_f64_opt(row, 12)?.unwrap_or(0.0);
    let mut rated = false;
    for index in 13..20 {
        if column_f64_opt(row, index)?.unwrap_or(0.0) != 0.0 {
            rated = true;
        }
    }
    let convention = helmert_convention(method);
    Ok(RawHelmert {
        buildable: convention.is_some()
            && epoch.is_none()
            && px == 0.0
            && py == 0.0
            && pz == 0.0
            && !rated,
        accuracy: column_f64_opt(row, 8)?,
        tx: column_f64_opt(row, 1)?.unwrap_or(0.0),
        ty: column_f64_opt(row, 2)?.unwrap_or(0.0),
        tz: column_f64_opt(row, 3)?.unwrap_or(0.0),
        rx: column_f64_opt(row, 4)?.unwrap_or(0.0),
        ry: column_f64_opt(row, 5)?.unwrap_or(0.0),
        rz: column_f64_opt(row, 6)?.unwrap_or(0.0),
        scale: column_f64_opt(row, 7)?.unwrap_or(0.0),
        convention,
        translation_auth: column_text_opt(row, 20)?,
        translation_code: column_text_opt(row, 21)?,
        rotation_auth: column_text_opt(row, 22)?,
        rotation_code: column_text_opt(row, 23)?,
        scale_auth: column_text_opt(row, 24)?,
        scale_code: column_text_opt(row, 25)?,
        source: (column_text(row, 26)?, column_text(row, 27)?),
    })
}

fn conv_of(
    connection: &Connection,
    auth: &Option<String>,
    code: &Option<String>,
) -> Result<Option<f64>, rusqlite::Error> {
    match (auth, code) {
        (Some(auth), Some(code)) => {
            let (_, conv, _) = unit_of(connection, auth, code)?;
            Ok(conv)
        }
        _ => Ok(None),
    }
}

fn helmert_convention(method: i64) -> Option<&'static str> {
    match method {
        1031 | 9603 | 1035 => Some(""),
        1032 | 9607 | 1038 => Some("coordinate_frame"),
        1033 | 9606 | 1037 => Some("position_vector"),
        _ => None,
    }
}

fn apply_factor(value: f64, conv: Option<f64>) -> Result<f64, String> {
    if value == 0.0 {
        return Ok(0.0);
    }
    to_metres(value, conv)
}

fn to_arcsec(value: f64, conv: Option<f64>) -> Result<f64, String> {
    if value == 0.0 {
        return Ok(0.0);
    }
    let factor = conv.ok_or_else(|| "rotation unit has no conversion factor".to_owned())?;
    Ok((value * factor).to_degrees() * 3600.0)
}

fn to_ppm(value: f64, conv: Option<f64>) -> Result<f64, String> {
    if value == 0.0 {
        return Ok(0.0);
    }
    let factor = conv.unwrap_or(1e-6);
    Ok(value * factor / 1e-6)
}

fn datum_family(
    connection: &Connection,
    auth: &str,
    code: &str,
) -> Result<Vec<(String, String)>, rusqlite::Error> {
    let mut family = vec![(auth.to_owned(), code.to_owned())];
    let mut statement = connection.prepare(
        "SELECT member_auth_name, member_code FROM geodetic_datum_ensemble_member
         WHERE ensemble_auth_name = ?1 AND ensemble_code = ?2
         UNION
         SELECT ensemble_auth_name, ensemble_code FROM geodetic_datum_ensemble_member
         WHERE member_auth_name = ?1 AND member_code = ?2",
    )?;
    let rows = statement.query_map(params![auth, code], |row| {
        Ok((column_text(row, 0)?, column_text(row, 1)?))
    })?;
    for row in rows {
        family.push(row?);
    }
    Ok(family)
}

fn helmert_pipeline(source: &Crs, target: &Crs, shift: &Shift) -> String {
    let mut helmert = format!(
        "+proj=helmert +x={} +y={} +z={}",
        format_f64(shift.x),
        format_f64(shift.y),
        format_f64(shift.z)
    );
    if shift.rx != 0.0 || shift.ry != 0.0 || shift.rz != 0.0 || shift.scale != 0.0 {
        helmert.push_str(&format!(
            " +rx={} +ry={} +rz={} +s={}",
            format_f64(shift.rx),
            format_f64(shift.ry),
            format_f64(shift.rz),
            format_f64(shift.scale)
        ));
    }
    if let Some(convention) = shift.convention.filter(|name| !name.is_empty()) {
        if shift.rx != 0.0 || shift.ry != 0.0 || shift.rz != 0.0 {
            helmert.push_str(" +convention=");
            helmert.push_str(convention);
        }
    }
    let inv = if shift.forward { "" } else { "+inv " };
    format!(
        "+proj=pipeline +step +proj=cart {} +step {inv}{helmert} +step +inv +proj=cart {}",
        ellipsoid_args(source),
        ellipsoid_args(target)
    )
}

fn ellipsoid_args(crs: &Crs) -> String {
    if let Some(rf) = crs.rf.filter(|rf| *rf > 0.0) {
        format!("+a={} +rf={}", format_f64(crs.a), format_f64(rf))
    } else if let Some(b) = crs.b {
        format!("+a={} +b={}", format_f64(crs.a), format_f64(b))
    } else {
        format!("+a={} +b={}", format_f64(crs.a), format_f64(crs.a))
    }
}

fn column_text(row: &Row<'_>, index: usize) -> Result<String, rusqlite::Error> {
    column_text_opt(row, index)?.ok_or_else(|| {
        rusqlite::Error::InvalidColumnType(index, "null".to_owned(), rusqlite::types::Type::Null)
    })
}

fn column_text_opt(row: &Row<'_>, index: usize) -> Result<Option<String>, rusqlite::Error> {
    match row.get_ref(index)? {
        rusqlite::types::ValueRef::Null => Ok(None),
        rusqlite::types::ValueRef::Integer(value) => Ok(Some(value.to_string())),
        rusqlite::types::ValueRef::Real(value) => Ok(Some(format_f64(value))),
        rusqlite::types::ValueRef::Text(bytes) => {
            Ok(Some(String::from_utf8_lossy(bytes).into_owned()))
        }
        rusqlite::types::ValueRef::Blob(_) => Ok(None),
    }
}

fn column_i64(row: &Row<'_>, index: usize) -> Result<i64, rusqlite::Error> {
    column_i64_opt(row, index)?.ok_or_else(|| {
        rusqlite::Error::InvalidColumnType(index, "null".to_owned(), rusqlite::types::Type::Null)
    })
}

fn column_i64_opt(row: &Row<'_>, index: usize) -> Result<Option<i64>, rusqlite::Error> {
    match row.get_ref(index)? {
        rusqlite::types::ValueRef::Null => Ok(None),
        rusqlite::types::ValueRef::Integer(value) => Ok(Some(value)),
        rusqlite::types::ValueRef::Text(bytes) => {
            let text = String::from_utf8_lossy(bytes);
            text.parse::<i64>()
                .map(Some)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
        }
        rusqlite::types::ValueRef::Real(value) => {
            if value.fract() != 0.0 {
                return Err(rusqlite::Error::InvalidColumnType(
                    index,
                    "real".to_owned(),
                    rusqlite::types::Type::Real,
                ));
            }
            Ok(Some(real_to_i64(value)))
        }
        rusqlite::types::ValueRef::Blob(_) => Ok(None),
    }
}

fn column_f64(row: &Row<'_>, index: usize) -> Result<f64, rusqlite::Error> {
    column_f64_opt(row, index)?.ok_or_else(|| {
        rusqlite::Error::InvalidColumnType(index, "null".to_owned(), rusqlite::types::Type::Null)
    })
}

fn column_f64_opt(row: &Row<'_>, index: usize) -> Result<Option<f64>, rusqlite::Error> {
    match row.get_ref(index)? {
        rusqlite::types::ValueRef::Null => Ok(None),
        rusqlite::types::ValueRef::Real(value) => Ok(Some(value)),
        rusqlite::types::ValueRef::Integer(value) => Ok(Some(i64_to_f64(value))),
        rusqlite::types::ValueRef::Text(bytes) => {
            let text = String::from_utf8_lossy(bytes);
            text.parse::<f64>()
                .map(Some)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
        }
        rusqlite::types::ValueRef::Blob(_) => Ok(None),
    }
}

#[expect(
    clippy::cast_precision_loss,
    clippy::as_conversions,
    reason = "EPSG parameter integers are well inside the exact f64 mantissa"
)]
fn i64_to_f64(value: i64) -> f64 {
    value as f64
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::as_conversions,
    reason = "the real was checked to be an integral CRS or parameter code"
)]
fn real_to_i64(value: f64) -> i64 {
    value as i64
}

/// PROJ pipeline from `source` to `target`, both `EPSG:` codes.
///
/// The boolean is true when the destination's first axis is latitude or
/// northing. A missing grid, or a method this crate does not build, is
/// [`CreateError::Unsupported`] and not a coordinate. A grid file that is
/// present is part of the pipeline.
/// Expand `+init=epsg:CODE` or the one NAD27 state-plane section the gie suite uses.
///
/// # Errors
///
/// [`CreateError::IllegalValue`] when the section is not a CRS this crate builds.
pub(crate) fn init_definition(spec: &str) -> Result<String, CreateError> {
    let spec = spec.trim();
    if let Some(zone) = spec.strip_prefix("nad27:") {
        if zone == "3901" {
            return Ok(
                "proj=lcc datum=NAD27 lon_0=-81 lat_1=34d58 lat_2=33d46 lat_0=33 x_0=609601.2192024384 y_0=0"
                    .to_owned(),
            );
        }
        return Err(CreateError::IllegalValue("init"));
    }
    let code = spec
        .strip_prefix("epsg:")
        .or_else(|| spec.strip_prefix("EPSG:"))
        .ok_or(CreateError::IllegalValue("init"))?;
    let code: u32 = code
        .parse()
        .map_err(|_| CreateError::IllegalValue("init"))?;
    match with_db(|connection| load_crs(connection, code)) {
        Ok(loaded) => Ok(loaded.definition),
        Err(DbFail::Open(_)) => Err(CreateError::Unsupported("proj.db")),
        Err(DbFail::Sql(_)) => Err(CreateError::IllegalValue("init")),
    }
}

pub(crate) fn epsg_pipeline(source: &str, target: &str) -> Result<(String, bool), CreateError> {
    let source =
        authority_code(source.trim()).ok_or(CreateError::Unsupported("authority string"))?;
    let target =
        authority_code(target.trim()).ok_or(CreateError::Unsupported("authority string"))?;
    with_db(|connection| pipeline_of(connection, source, target)).map_err(pipeline_error)
}

/// WKT or PROJJSON stored for an EPSG code, or the text the caller already passed.
pub(crate) fn stored_wkt(definition: &str) -> Option<String> {
    stored_markup(definition, Markup::Wkt)
}

pub(crate) fn stored_projjson(definition: &str) -> Option<String> {
    stored_markup(definition, Markup::ProjJson)
}

fn stored_markup(definition: &str, markup: Markup) -> Option<String> {
    let trimmed = definition.trim().trim_start_matches('\u{feff}');
    if markup.matches_inline(trimmed) {
        return Some(definition.to_owned());
    }
    let code = authority_code(trimmed)?;
    with_db(|connection| read_stored(connection, code, markup))
        .ok()
        .flatten()
}

#[derive(Clone, Copy)]
enum Markup {
    Wkt,
    ProjJson,
}

impl Markup {
    fn matches_inline(self, text: &str) -> bool {
        match self {
            Self::ProjJson => text.starts_with('{'),
            Self::Wkt => crate::crs_text::is_wkt(text),
        }
    }

    fn matches_stored(self, text: &str) -> bool {
        let trimmed = text.trim();
        match self {
            Self::ProjJson => trimmed.starts_with('{'),
            Self::Wkt => crate::crs_text::is_wkt(trimmed),
        }
    }

    fn columns(self) -> &'static [&'static str] {
        match self {
            Self::ProjJson => &["projjson", "proj_json", "json"],
            Self::Wkt => &["wkt", "wkt2", "wkt1", "wkt_2"],
        }
    }
}

fn pipeline_error(error: DbFail) -> CreateError {
    let text = match error {
        DbFail::Open(message) => message,
        DbFail::Sql(error) => error.to_string(),
    };
    if text.contains("grid step") {
        CreateError::Unsupported("grid")
    } else {
        CreateError::Unsupported("crs")
    }
}

fn pipeline_of(
    connection: &Connection,
    source: u32,
    target: u32,
) -> Result<(String, bool), rusqlite::Error> {
    let src = bound_of(connection, source)?;
    let dst = bound_of(connection, target)?;
    let datum = if same_datum(&src.earth, &dst.earth) {
        None
    } else {
        Some(require_bridge(connection, &src.earth, &dst.earth)?)
    };
    let vertical = vertical_step(connection, &src, &dst)?;
    let steps = assemble(&src, &dst, datum, vertical);
    let text = pipeline_string(&dst.earth, &steps);
    let lat_or_northing_first = dst.swap && !matches!(dst.horizontal, Horizontal::Geocentric);
    Ok((text, lat_or_northing_first))
}

struct Bound {
    horizontal: Horizontal,
    swap: bool,
    earth: Earth,
    height: Height,
    /// Prime meridian, degrees east of Greenwich. Zero leaves the pipeline unchanged.
    pm_deg: f64,
    /// Angular unit of a geographic CRS. Projected and geocentric bounds keep degrees.
    angle: AngleUnit,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AngleUnit {
    Degree,
    Grad,
}

impl AngleUnit {
    fn proj(self) -> &'static str {
        match self {
            Self::Degree => "deg",
            Self::Grad => "grad",
        }
    }
}

enum Height {
    None,
    Ellipsoidal,
    Vertical { auth: String, code: String },
}

enum Horizontal {
    Geographic,
    Projected(String),
    Geocentric,
}

#[derive(Clone, Copy)]
enum Plane {
    Geographic,
    Projected,
    Geocentric,
}

#[derive(Clone, Copy, Debug)]
enum AxisKind {
    Lat,
    Lon,
    East,
    North,
    Up,
    X,
    Y,
    Z,
}

fn bound_of(connection: &Connection, code: u32) -> Result<Bound, rusqlite::Error> {
    let found = connection
        .query_row(
            "SELECT table_name, type FROM crs_view WHERE auth_name = 'EPSG' AND code = ?1",
            params![i64::from(code)],
            |row| Ok((column_text(row, 0)?, column_text(row, 1)?)),
        )
        .optional()?;
    let Some((table, crs_type)) = found else {
        return Err(missing(code));
    };
    match table.as_str() {
        "geodetic_crs" => geodetic_bound(connection, code, &crs_type),
        "projected_crs" => projected_bound(connection, code),
        "compound_crs" => compound_bound(connection, code),
        _ => Err(unsupported(format!(
            "EPSG:{code} ({crs_type}) is not built"
        ))),
    }
}

fn geodetic_bound(
    connection: &Connection,
    code: u32,
    crs_type: &str,
) -> Result<Bound, rusqlite::Error> {
    let (cs_auth, cs_code) = connection.query_row(
        "SELECT coordinate_system_auth_name, coordinate_system_code
         FROM geodetic_crs WHERE auth_name = 'EPSG' AND code = ?1",
        params![i64::from(code)],
        |row| Ok((column_text(row, 0)?, column_text(row, 1)?)),
    )?;
    let earth = earth_of(connection, code)?;
    let pm_deg = prime_degrees(connection, &earth)?;
    let angle = angular_unit(connection, code)?;
    let axes = axes_of(connection, &cs_auth, &cs_code)?;
    let (horizontal, plane) = match crs_type {
        "geographic 2D" | "geographic 3D" => (Horizontal::Geographic, Plane::Geographic),
        "geocentric" => (Horizontal::Geocentric, Plane::Geocentric),
        _ => {
            return Err(unsupported(format!(
                "EPSG:{code} ({crs_type}) is not built"
            )))
        }
    };
    let swap = axis_swap(&axes, plane)?;
    let height = if crs_type == "geographic 3D" {
        Height::Ellipsoidal
    } else {
        Height::None
    };
    Ok(Bound {
        horizontal,
        swap,
        earth,
        height,
        pm_deg,
        angle,
    })
}

fn compound_bound(connection: &Connection, code: u32) -> Result<Bound, rusqlite::Error> {
    let (horiz_auth, horiz_code, vert_auth, vert_code) = connection.query_row(
        "SELECT horiz_crs_auth_name, horiz_crs_code, vertical_crs_auth_name, vertical_crs_code
         FROM compound_crs WHERE auth_name = 'EPSG' AND code = ?1",
        params![i64::from(code)],
        |row| {
            Ok((
                column_text(row, 0)?,
                column_text(row, 1)?,
                column_text(row, 2)?,
                column_text(row, 3)?,
            ))
        },
    )?;
    if horiz_auth != "EPSG" || vert_auth != "EPSG" {
        return Err(unsupported(format!(
            "EPSG:{code} compound authorities are not built"
        )));
    }
    let horiz = horiz_code
        .parse::<i64>()
        .map_err(|_| unsupported(format!("EPSG:{code} horizontal code is not numeric")))?;
    let mut bound = bound_of(connection, u32_code(horiz)?)?;
    if !matches!(bound.height, Height::None) {
        return Err(unsupported(format!(
            "EPSG:{code} horizontal CRS is not a 2D CRS"
        )));
    }
    require_up(connection, &vert_auth, &vert_code)?;
    bound.height = Height::Vertical {
        auth: vert_auth,
        code: vert_code,
    };
    Ok(bound)
}

fn require_up(connection: &Connection, auth: &str, code: &str) -> Result<(), rusqlite::Error> {
    let orientation = connection.query_row(
        "SELECT a.orientation FROM vertical_crs v
         JOIN axis a
           ON a.coordinate_system_auth_name = v.coordinate_system_auth_name
          AND a.coordinate_system_code = v.coordinate_system_code
         WHERE v.auth_name = ?1 AND v.code = ?2",
        params![auth, code],
        |row| column_text(row, 0),
    )?;
    if orientation.eq_ignore_ascii_case("up") {
        Ok(())
    } else {
        Err(unsupported(format!(
            "vertical axis `{orientation}` is not built"
        )))
    }
}

fn projected_bound(connection: &Connection, code: u32) -> Result<Bound, rusqlite::Error> {
    let loaded = load_projected(connection, code)?;
    let (cs_auth, cs_code) = connection.query_row(
        "SELECT coordinate_system_auth_name, coordinate_system_code
         FROM projected_crs WHERE auth_name = 'EPSG' AND code = ?1",
        params![i64::from(code)],
        |row| Ok((column_text(row, 0)?, column_text(row, 1)?)),
    )?;
    let axes = axes_of(connection, &cs_auth, &cs_code)?;
    let swap = axis_swap(&axes, Plane::Projected)?;
    let earth = Earth {
        a: loaded.a,
        rf: loaded.rf,
        b: loaded.b,
        datum_auth: loaded.datum_auth,
        datum_code: loaded.datum_code,
    };
    let pm_deg = prime_degrees(connection, &earth)?;
    let mut definition = loaded.definition;
    if pm_deg.abs() > 1e-12 {
        definition.push_str(" +pm=");
        definition.push_str(&format_f64(pm_deg));
    }
    Ok(Bound {
        horizontal: Horizontal::Projected(definition),
        swap,
        earth,
        height: Height::None,
        pm_deg,
        angle: AngleUnit::Degree,
    })
}

fn axes_of(
    connection: &Connection,
    auth: &str,
    code: &str,
) -> Result<Vec<AxisKind>, rusqlite::Error> {
    let mut statement = connection.prepare(
        "SELECT name, orientation FROM axis
         WHERE coordinate_system_auth_name = ?1 AND coordinate_system_code = ?2
         ORDER BY coordinate_system_order",
    )?;
    let rows = statement.query_map(params![auth, code], |row| {
        Ok((column_text(row, 0)?, column_text(row, 1)?))
    })?;
    let mut axes = Vec::new();
    for row in rows {
        let (name, orientation) = row?;
        let kind = classify_axis(&name, &orientation)
            .ok_or_else(|| unsupported(format!("axis `{name}` is not built")))?;
        axes.push(kind);
    }
    if axes.is_empty() {
        return Err(unsupported("CRS axis order is not built".to_owned()));
    }
    Ok(axes)
}

fn classify_axis(name: &str, orientation: &str) -> Option<AxisKind> {
    let name = name.to_ascii_lowercase();
    let orientation = orientation.to_ascii_lowercase();
    if name.contains("latitude") {
        Some(AxisKind::Lat)
    } else if name.contains("longitude") {
        Some(AxisKind::Lon)
    } else if name.contains("northing") || orientation == "north" {
        Some(AxisKind::North)
    } else if name.contains("easting") || orientation == "east" {
        Some(AxisKind::East)
    } else if name.contains("height") || orientation == "up" {
        Some(AxisKind::Up)
    } else if orientation == "geocentricx" || name.contains("geocentric x") {
        Some(AxisKind::X)
    } else if orientation == "geocentricy" || name.contains("geocentric y") {
        Some(AxisKind::Y)
    } else if orientation == "geocentricz" || name.contains("geocentric z") {
        Some(AxisKind::Z)
    } else {
        None
    }
}

fn axis_swap(axes: &[AxisKind], plane: Plane) -> Result<bool, rusqlite::Error> {
    let swap = match (plane, axes.first(), axes.get(1)) {
        (Plane::Geographic, Some(AxisKind::Lat), Some(AxisKind::Lon)) => true,
        (Plane::Geographic, Some(AxisKind::Lon), Some(AxisKind::Lat)) => false,
        (Plane::Projected, Some(AxisKind::North), Some(AxisKind::East)) => true,
        (Plane::Projected, Some(AxisKind::East), Some(AxisKind::North)) => false,
        (Plane::Geocentric, Some(AxisKind::X), Some(AxisKind::Y)) => false,
        (Plane::Geocentric, Some(AxisKind::Y), Some(AxisKind::X)) => true,
        _ => return Err(unsupported("CRS axis order is not built".to_owned())),
    };
    if let Some(third) = axes.get(2) {
        let legal = match plane {
            Plane::Geocentric => matches!(third, AxisKind::Z),
            Plane::Geographic | Plane::Projected => matches!(third, AxisKind::Up | AxisKind::Z),
        };
        if !legal {
            return Err(unsupported("CRS axis order is not built".to_owned()));
        }
    }
    if axes.len() > 3 {
        return Err(unsupported("CRS axis order is not built".to_owned()));
    }
    Ok(swap)
}

fn prime_degrees(connection: &Connection, earth: &Earth) -> Result<f64, rusqlite::Error> {
    let (longitude, conv, name) = connection.query_row(
        "SELECT pm.longitude, u.conv_factor, u.name
         FROM geodetic_datum d
         JOIN prime_meridian pm
           ON pm.auth_name = d.prime_meridian_auth_name AND pm.code = d.prime_meridian_code
         JOIN unit_of_measure u
           ON u.auth_name = pm.uom_auth_name AND u.code = pm.uom_code
         WHERE d.auth_name = ?1 AND d.code = ?2",
        params![earth.datum_auth, earth.datum_code],
        |row| {
            Ok((
                column_f64(row, 0)?,
                column_f64_opt(row, 1)?,
                column_text(row, 2)?,
            ))
        },
    )?;
    to_degrees(longitude, conv, &name).map_err(unsupported)
}

fn angular_unit(connection: &Connection, code: u32) -> Result<AngleUnit, rusqlite::Error> {
    let found = connection
        .query_row(
            "SELECT u.name, u.conv_factor
             FROM geodetic_crs g
             JOIN axis a
               ON a.coordinate_system_auth_name = g.coordinate_system_auth_name
              AND a.coordinate_system_code = g.coordinate_system_code
             JOIN unit_of_measure u
               ON u.auth_name = a.uom_auth_name AND u.code = a.uom_code
             WHERE g.auth_name = 'EPSG' AND g.code = ?1
             ORDER BY a.coordinate_system_order
             LIMIT 1",
            params![i64::from(code)],
            |row| Ok((column_text(row, 0)?, column_f64_opt(row, 1)?)),
        )
        .optional()?;
    let Some((name, conv)) = found else {
        return Ok(AngleUnit::Degree);
    };
    let folded = name.to_ascii_lowercase();
    if folded.contains("grad") || folded == "gon" {
        return Ok(AngleUnit::Grad);
    }
    if conv.is_some_and(|factor| (factor - std::f64::consts::PI / 200.0).abs() < 1e-12) {
        return Ok(AngleUnit::Grad);
    }
    Ok(AngleUnit::Degree)
}

fn same_datum(left: &Earth, right: &Earth) -> bool {
    !left.datum_code.is_empty()
        && left.datum_auth == right.datum_auth
        && left.datum_code == right.datum_code
}

enum DatumStep {
    Helmert(String),
    HorizontalGrid(String),
}

fn assemble(
    src: &Bound,
    dst: &Bound,
    datum: Option<DatumStep>,
    vertical: Option<String>,
) -> Vec<String> {
    let mut steps = Vec::new();
    let src_geocentric = matches!(src.horizontal, Horizontal::Geocentric);
    let dst_geocentric = matches!(dst.horizontal, Horizontal::Geocentric);
    if !src_geocentric {
        push_to_angular(&mut steps, src);
    }
    let helmert = match &datum {
        Some(DatumStep::Helmert(step)) => Some(step.clone()),
        _ => None,
    };
    if helmert.is_some() || dst_geocentric || src_geocentric {
        if !src_geocentric {
            steps.push(cart_step(CartSense::Forward, &src.earth));
        }
        if let Some(helmert) = helmert {
            steps.push(helmert);
        }
        if !dst_geocentric {
            steps.push(cart_step(CartSense::Inverse, &dst.earth));
            if let Some(vertical) = vertical {
                steps.push(vertical);
            }
            push_from_angular(&mut steps, dst);
        }
    } else {
        if let Some(DatumStep::HorizontalGrid(step)) = datum {
            steps.push(step);
        }
        if let Some(vertical) = vertical {
            steps.push(vertical);
        }
        push_from_angular(&mut steps, dst);
    }
    if steps.is_empty() {
        steps.push("+proj=noop".to_owned());
    }
    steps
}

fn push_to_angular(steps: &mut Vec<String>, bound: &Bound) {
    match &bound.horizontal {
        Horizontal::Geographic => {
            if bound.swap {
                steps.push("+proj=axisswap +order=2,1".to_owned());
            }
            steps.push(format!(
                "+proj=unitconvert +xy_in={} +xy_out=rad",
                bound.angle.proj()
            ));
            // Longitude in the CRS is relative to its prime meridian. The rest of
            // the pipeline speaks Greenwich radians.
            if bound.pm_deg.abs() > 1e-12 {
                steps.push(format!(
                    "+inv +proj=longlat +pm={}",
                    format_f64(bound.pm_deg)
                ));
            }
        }
        Horizontal::Projected(definition) => {
            if bound.swap {
                steps.push("+proj=axisswap +order=2,1".to_owned());
            }
            steps.push(format!("+inv {definition}"));
        }
        Horizontal::Geocentric => {}
    }
}

fn push_from_angular(steps: &mut Vec<String>, bound: &Bound) {
    match &bound.horizontal {
        Horizontal::Geographic => {
            if bound.pm_deg.abs() > 1e-12 {
                steps.push(format!("+proj=longlat +pm={}", format_f64(bound.pm_deg)));
            }
            steps.push(format!(
                "+proj=unitconvert +xy_in=rad +xy_out={}",
                bound.angle.proj()
            ));
            if bound.swap {
                steps.push("+proj=axisswap +order=2,1".to_owned());
            }
        }
        Horizontal::Projected(definition) => {
            steps.push(definition.clone());
            if bound.swap {
                steps.push("+proj=axisswap +order=2,1".to_owned());
            }
        }
        Horizontal::Geocentric => {}
    }
}

enum CartSense {
    Forward,
    Inverse,
}

fn cart_step(sense: CartSense, earth: &Earth) -> String {
    let inverse = match sense {
        CartSense::Forward => "",
        CartSense::Inverse => "+inv ",
    };
    format!("{inverse}+proj=cart {}", ellipsoid_clause(earth))
}

fn pipeline_string(earth: &Earth, steps: &[String]) -> String {
    let mut text = format!("+proj=pipeline {}", ellipsoid_clause(earth));
    for step in steps {
        text.push_str(" +step ");
        text.push_str(step);
    }
    text
}

struct HelmertCandidate {
    accuracy: f64,
    rank: u8,
    step: String,
}

struct Blocker {
    grid: bool,
    accuracy: f64,
}

struct UnitFactors {
    metre: f64,
    arcsec: f64,
    ppm: f64,
    metre_per_year: f64,
    arcsec_per_year: f64,
    ppm_per_year: f64,
    year: f64,
}

struct DatumChoice {
    accuracy: f64,
    rank: u8,
    step: DatumStep,
}

fn require_bridge(
    connection: &Connection,
    from: &Earth,
    to: &Earth,
) -> Result<DatumStep, rusqlite::Error> {
    let from_datums = datum_family(connection, &from.datum_auth, &from.datum_code)?;
    let to_datums = datum_family(connection, &to.datum_auth, &to.datum_code)?;
    let allowed = allowed_authorities(connection)?;
    let helmerts = helmert_candidates(connection, &from_datums, &to_datums, &allowed)?;
    let mut choices: Vec<DatumChoice> = helmerts
        .into_iter()
        .map(|item| DatumChoice {
            accuracy: item.accuracy,
            rank: item.rank,
            step: DatumStep::Helmert(item.step),
        })
        .collect();
    choices.extend(horizontal_grids(
        connection,
        &from_datums,
        &to_datums,
        &allowed,
    )?);
    let blockers = blockers_of(connection, &from_datums, &to_datums, &allowed)?;
    let choice = choices.into_iter().min_by(|left, right| {
        left.accuracy
            .total_cmp(&right.accuracy)
            .then(left.rank.cmp(&right.rank))
    });
    let blocker = blockers
        .into_iter()
        .min_by(|left, right| left.accuracy.total_cmp(&right.accuracy));
    let choice_accuracy = choice.as_ref().map(|item| item.accuracy);
    if let Some(blocker) = blocker {
        let wins = choice_accuracy.is_none_or(|accuracy| blocker.accuracy < accuracy);
        if wins {
            return Err(unsupported(if blocker.grid {
                "grid step".to_owned()
            } else {
                "method is not built".to_owned()
            }));
        }
    }
    choice.map(|item| item.step).ok_or_else(|| {
        unsupported(format!(
            "EPSG datum {} to {} needs a grid, a deformation model, or a method this crate does not build",
            from.datum_code, to.datum_code
        ))
    })
}

fn horizontal_grids(
    connection: &Connection,
    from_datums: &[(String, String)],
    to_datums: &[(String, String)],
    allowed: &[String],
) -> Result<Vec<DatumChoice>, rusqlite::Error> {
    let mut binds = Vec::new();
    let from_sql = datum_predicate("s", from_datums, &mut binds);
    let to_sql = datum_predicate("t", to_datums, &mut binds);
    let from_rev = datum_predicate("t", from_datums, &mut binds);
    let to_rev = datum_predicate("s", to_datums, &mut binds);
    let sql = format!(
        "SELECT g.auth_name, g.accuracy,
                s.datum_auth_name, s.datum_code,
                t.datum_auth_name, t.datum_code,
                a.proj_grid_name, a.inverse_direction
         FROM grid_transformation g
         JOIN geodetic_crs s
           ON s.auth_name = g.source_crs_auth_name AND s.code = g.source_crs_code
         JOIN geodetic_crs t
           ON t.auth_name = g.target_crs_auth_name AND t.code = g.target_crs_code
         JOIN grid_alternatives a ON a.original_grid_name = g.grid_name
         WHERE g.deprecated = 0
           AND a.proj_method = 'hgridshift'
           AND ((({from_sql}) AND ({to_sql})) OR (({from_rev}) AND ({to_rev})))"
    );
    let mut statement = connection.prepare(&sql)?;
    let mapped = statement.query_map(rusqlite::params_from_iter(binds.iter()), |row| {
        Ok((
            column_text(row, 0)?,
            column_f64_opt(row, 1)?.unwrap_or(f64::MAX),
            column_text(row, 2)?,
            column_text(row, 3)?,
            column_text(row, 4)?,
            column_text(row, 5)?,
            column_text(row, 6)?,
            column_i64(row, 7)? != 0,
        ))
    })?;
    let mut choices = Vec::new();
    for row in mapped {
        let (
            auth,
            accuracy,
            source_auth,
            source_code,
            target_auth,
            target_code,
            grid,
            inverse_file,
        ) = row?;
        let Some(rank) = authority_kept(&auth, allowed) else {
            continue;
        };
        if !crate::gridpath::available(&grid) {
            continue;
        }
        let forward = datum_listed(from_datums, &source_auth, &source_code)
            && datum_listed(to_datums, &target_auth, &target_code);
        let reverse = datum_listed(from_datums, &target_auth, &target_code)
            && datum_listed(to_datums, &source_auth, &source_code);
        if forward == reverse {
            continue;
        }
        let inverse = inverse_file ^ !forward;
        let step = if inverse {
            format!("+inv +proj=hgridshift +grids={grid}")
        } else {
            format!("+proj=hgridshift +grids={grid}")
        };
        choices.push(DatumChoice {
            accuracy,
            rank,
            step: DatumStep::HorizontalGrid(step),
        });
    }
    Ok(choices)
}

fn datum_listed(datums: &[(String, String)], auth: &str, code: &str) -> bool {
    datums
        .iter()
        .any(|(item_auth, item_code)| item_auth == auth && item_code == code)
}

fn vertical_step(
    connection: &Connection,
    src: &Bound,
    dst: &Bound,
) -> Result<Option<String>, rusqlite::Error> {
    let (earth, vertical_auth, vertical_code, ellip_to_vert) = match (&src.height, &dst.height) {
        (Height::Ellipsoidal, Height::Vertical { auth, code }) => (&src.earth, auth, code, true),
        (Height::Vertical { auth, code }, Height::Ellipsoidal) => (&dst.earth, auth, code, false),
        (
            Height::Vertical {
                auth: left_auth,
                code: left_code,
            },
            Height::Vertical {
                auth: right_auth,
                code: right_code,
            },
        ) if left_auth == right_auth && left_code == right_code => {
            return Ok(None);
        }
        (Height::Vertical { .. }, Height::Vertical { .. }) => {
            return Err(unsupported("grid step".to_owned()));
        }
        _ => return Ok(None),
    };
    let family = datum_family(connection, &earth.datum_auth, &earth.datum_code)?;
    let allowed = allowed_authorities(connection)?;
    let mut statement = connection.prepare(
        "SELECT g.auth_name, g.accuracy,
                g.source_crs_auth_name, g.source_crs_code,
                g.target_crs_auth_name, g.target_crs_code,
                s.datum_auth_name, s.datum_code,
                t.datum_auth_name, t.datum_code,
                a.proj_grid_name, a.inverse_direction, g.name
         FROM grid_transformation g
         JOIN grid_alternatives a ON a.original_grid_name = g.grid_name
         LEFT JOIN geodetic_crs s
           ON s.auth_name = g.source_crs_auth_name AND s.code = g.source_crs_code
         LEFT JOIN geodetic_crs t
           ON t.auth_name = g.target_crs_auth_name AND t.code = g.target_crs_code
         WHERE g.deprecated = 0
           AND a.proj_method = 'geoid_like'
           AND ((g.target_crs_auth_name = ?1 AND g.target_crs_code = ?2)
             OR (g.source_crs_auth_name = ?1 AND g.source_crs_code = ?2))",
    )?;
    let mapped = statement.query_map(params![vertical_auth, vertical_code], |row| {
        Ok((
            column_text(row, 0)?,
            column_f64_opt(row, 1)?.unwrap_or(f64::MAX),
            column_text(row, 2)?,
            column_text(row, 3)?,
            column_text(row, 4)?,
            column_text(row, 5)?,
            column_text_opt(row, 6)?,
            column_text_opt(row, 7)?,
            column_text_opt(row, 8)?,
            column_text_opt(row, 9)?,
            column_text(row, 10)?,
            column_i64(row, 11)? != 0,
            column_text(row, 12)?,
        ))
    })?;
    let mut best_available: Option<(f64, u8, String, String)> = None;
    let mut best_missing = f64::MAX;
    for row in mapped {
        let (
            auth,
            accuracy,
            source_auth,
            source_code,
            target_auth,
            target_code,
            source_datum_auth,
            source_datum_code,
            target_datum_auth,
            target_datum_code,
            grid,
            inverse_file,
            name,
        ) = row?;
        let Some(rank) = authority_kept(&auth, &allowed) else {
            continue;
        };
        let target_is_vertical = target_auth == *vertical_auth && target_code == *vertical_code;
        let source_is_vertical = source_auth == *vertical_auth && source_code == *vertical_code;
        let (datum_auth, datum_code, registered_ellip_to_vert) = if target_is_vertical {
            (source_datum_auth, source_datum_code, true)
        } else if source_is_vertical {
            (target_datum_auth, target_datum_code, false)
        } else {
            continue;
        };
        let (Some(datum_auth), Some(datum_code)) = (datum_auth, datum_code) else {
            continue;
        };
        if !datum_listed(&family, &datum_auth, &datum_code) {
            continue;
        }
        if !crate::gridpath::available(&grid) {
            if accuracy < best_missing {
                best_missing = accuracy;
            }
            continue;
        }
        // EPSG Geographic3D to gravity-related height is the inverse of
        // `+proj=vgridshift +multiplier=1` (that forward step adds the grid).
        let applying_forward = registered_ellip_to_vert == ellip_to_vert;
        let inverse = inverse_file ^ applying_forward;
        let step = if inverse {
            format!("+inv +proj=vgridshift +grids={grid} +multiplier=1")
        } else {
            format!("+proj=vgridshift +grids={grid} +multiplier=1")
        };
        let replace =
            best_available
                .as_ref()
                .is_none_or(|(best_accuracy, best_rank, best_name, _)| {
                    accuracy < *best_accuracy
                        || (accuracy == *best_accuracy
                            && (rank < *best_rank
                                || (rank == *best_rank && name_first(&name, best_name))))
                });
        if replace {
            best_available = Some((accuracy, rank, name, step));
        }
    }
    if let Some((accuracy, _, _, step)) = best_available {
        if best_missing < accuracy {
            return Err(unsupported("grid step".to_owned()));
        }
        return Ok(Some(step));
    }
    if best_missing < f64::MAX {
        return Err(unsupported("grid step".to_owned()));
    }
    Err(unsupported("grid step".to_owned()))
}

/// PROJ's last tie-break between operations: the shorter name, then the greater one.
fn name_first(name: &str, other: &str) -> bool {
    name.len() < other.len() || (name.len() == other.len() && name > other)
}

struct HelmertRaw {
    auth: String,
    method: i64,
    accuracy: Option<f64>,
    tx: f64,
    ty: f64,
    tz: f64,
    translation: (Option<String>, Option<String>),
    rx: f64,
    ry: f64,
    rz: f64,
    rotation: (Option<String>, Option<String>),
    scale: f64,
    scale_unit: (Option<String>, Option<String>),
    dx: f64,
    dy: f64,
    dz: f64,
    rate_translation: (Option<String>, Option<String>),
    drx: f64,
    dry: f64,
    drz: f64,
    rate_rotation: (Option<String>, Option<String>),
    ds: f64,
    rate_scale: (Option<String>, Option<String>),
    epoch: f64,
    epoch_unit: (Option<String>, Option<String>),
    px: f64,
    py: f64,
    pz: f64,
    source: (String, String),
}

fn allowed_authorities(connection: &Connection) -> Result<Vec<String>, rusqlite::Error> {
    let text = connection
        .query_row(
            "SELECT allowed_authorities FROM authority_to_authority_preference
             WHERE source_auth_name = 'EPSG' AND target_auth_name = 'EPSG'",
            [],
            |row| column_text(row, 0),
        )
        .optional()?;
    Ok(text
        .map(|item| {
            item.split(',')
                .map(|part| part.trim().to_owned())
                .filter(|part| !part.is_empty())
                .collect()
        })
        .unwrap_or_else(|| vec!["any".to_owned()]))
}

fn authority_kept(auth: &str, allowed: &[String]) -> Option<u8> {
    if allowed.iter().any(|item| item == "any") {
        return Some(auth_rank(auth));
    }
    let index = allowed.iter().position(|item| item == auth)?;
    u8::try_from(index).ok()
}

fn helmert_candidates(
    connection: &Connection,
    from_datums: &[(String, String)],
    to_datums: &[(String, String)],
    allowed: &[String],
) -> Result<Vec<HelmertCandidate>, rusqlite::Error> {
    let raws = {
        let mut binds = Vec::new();
        let from_sql = datum_predicate("s", from_datums, &mut binds);
        let to_sql = datum_predicate("t", to_datums, &mut binds);
        let from_rev = datum_predicate("t", from_datums, &mut binds);
        let to_rev = datum_predicate("s", to_datums, &mut binds);
        let sql = format!(
            "SELECT h.auth_name, h.method_code, h.accuracy,
                    h.tx, h.ty, h.tz, h.translation_uom_auth_name, h.translation_uom_code,
                    h.rx, h.ry, h.rz, h.rotation_uom_auth_name, h.rotation_uom_code,
                    h.scale_difference, h.scale_difference_uom_auth_name, h.scale_difference_uom_code,
                    h.rate_tx, h.rate_ty, h.rate_tz, h.rate_translation_uom_auth_name, h.rate_translation_uom_code,
                    h.rate_rx, h.rate_ry, h.rate_rz, h.rate_rotation_uom_auth_name, h.rate_rotation_uom_code,
                    h.rate_scale_difference, h.rate_scale_difference_uom_auth_name, h.rate_scale_difference_uom_code,
                    h.epoch, h.epoch_uom_auth_name, h.epoch_uom_code,
                    h.px, h.py, h.pz,
                    s.datum_auth_name, s.datum_code
             FROM helmert_transformation h
             JOIN geodetic_crs s
               ON s.auth_name = h.source_crs_auth_name AND s.code = h.source_crs_code
             JOIN geodetic_crs t
               ON t.auth_name = h.target_crs_auth_name AND t.code = h.target_crs_code
             WHERE h.deprecated = 0
               AND ((({from_sql}) AND ({to_sql})) OR (({from_rev}) AND ({to_rev})))"
        );
        let mut statement = connection.prepare(&sql)?;
        let mapped =
            statement.query_map(rusqlite::params_from_iter(binds.iter()), read_helmert_raw)?;
        mapped.collect::<Result<Vec<_>, _>>()?
    };
    let factors = unit_factors(connection)?;
    let mut candidates = Vec::new();
    for raw in raws {
        if let Some(candidate) =
            helmert_candidate(connection, &raw, from_datums, &factors, allowed)?
        {
            candidates.push(candidate);
        }
    }
    Ok(candidates)
}

fn pair_text(
    row: &Row<'_>,
    auth: usize,
    code: usize,
) -> Result<(Option<String>, Option<String>), rusqlite::Error> {
    Ok((column_text_opt(row, auth)?, column_text_opt(row, code)?))
}

fn read_helmert_raw(row: &Row<'_>) -> Result<HelmertRaw, rusqlite::Error> {
    Ok(HelmertRaw {
        auth: column_text(row, 0)?,
        method: column_i64(row, 1)?,
        accuracy: column_f64_opt(row, 2)?,
        tx: column_f64_opt(row, 3)?.unwrap_or(0.0),
        ty: column_f64_opt(row, 4)?.unwrap_or(0.0),
        tz: column_f64_opt(row, 5)?.unwrap_or(0.0),
        translation: pair_text(row, 6, 7)?,
        rx: column_f64_opt(row, 8)?.unwrap_or(0.0),
        ry: column_f64_opt(row, 9)?.unwrap_or(0.0),
        rz: column_f64_opt(row, 10)?.unwrap_or(0.0),
        rotation: pair_text(row, 11, 12)?,
        scale: column_f64_opt(row, 13)?.unwrap_or(0.0),
        scale_unit: pair_text(row, 14, 15)?,
        dx: column_f64_opt(row, 16)?.unwrap_or(0.0),
        dy: column_f64_opt(row, 17)?.unwrap_or(0.0),
        dz: column_f64_opt(row, 18)?.unwrap_or(0.0),
        rate_translation: pair_text(row, 19, 20)?,
        drx: column_f64_opt(row, 21)?.unwrap_or(0.0),
        dry: column_f64_opt(row, 22)?.unwrap_or(0.0),
        drz: column_f64_opt(row, 23)?.unwrap_or(0.0),
        rate_rotation: pair_text(row, 24, 25)?,
        ds: column_f64_opt(row, 26)?.unwrap_or(0.0),
        rate_scale: pair_text(row, 27, 28)?,
        epoch: column_f64_opt(row, 29)?.unwrap_or(0.0),
        epoch_unit: pair_text(row, 30, 31)?,
        px: column_f64_opt(row, 32)?.unwrap_or(0.0),
        py: column_f64_opt(row, 33)?.unwrap_or(0.0),
        pz: column_f64_opt(row, 34)?.unwrap_or(0.0),
        source: (column_text(row, 35)?, column_text(row, 36)?),
    })
}

fn helmert_candidate(
    connection: &Connection,
    raw: &HelmertRaw,
    from_datums: &[(String, String)],
    factors: &UnitFactors,
    allowed: &[String],
) -> Result<Option<HelmertCandidate>, rusqlite::Error> {
    let Some(rank) = authority_kept(&raw.auth, allowed) else {
        return Ok(None);
    };
    let Some(convention) = bridge_convention(raw.method) else {
        return Ok(None);
    };
    if raw.px != 0.0 || raw.py != 0.0 || raw.pz != 0.0 {
        return Ok(None);
    }
    let translation = conv_of(connection, &raw.translation.0, &raw.translation.1)?;
    let rotation = conv_of(connection, &raw.rotation.0, &raw.rotation.1)?;
    let scale = conv_of(connection, &raw.scale_unit.0, &raw.scale_unit.1)?;
    let rate_translation = conv_of(connection, &raw.rate_translation.0, &raw.rate_translation.1)?;
    let rate_rotation = conv_of(connection, &raw.rate_rotation.0, &raw.rate_rotation.1)?;
    let rate_scale = conv_of(connection, &raw.rate_scale.0, &raw.rate_scale.1)?;
    let epoch_unit = conv_of(connection, &raw.epoch_unit.0, &raw.epoch_unit.1)?;
    let dx = scale_unit(raw.dx, rate_translation, factors.metre_per_year)?;
    let dy = scale_unit(raw.dy, rate_translation, factors.metre_per_year)?;
    let dz = scale_unit(raw.dz, rate_translation, factors.metre_per_year)?;
    let drx = scale_unit(raw.drx, rate_rotation, factors.arcsec_per_year)?;
    let dry = scale_unit(raw.dry, rate_rotation, factors.arcsec_per_year)?;
    let drz = scale_unit(raw.drz, rate_rotation, factors.arcsec_per_year)?;
    let ds = scale_unit(raw.ds, rate_scale, factors.ppm_per_year)?;
    let rated =
        dx != 0.0 || dy != 0.0 || dz != 0.0 || drx != 0.0 || dry != 0.0 || drz != 0.0 || ds != 0.0;
    let epoch = if rated {
        scale_unit(raw.epoch, epoch_unit, factors.year)?
    } else {
        0.0
    };
    let shift = TimeShift {
        forward: from_datums.iter().any(|datum| datum == &raw.source),
        x: scale_unit(raw.tx, translation, factors.metre)?,
        y: scale_unit(raw.ty, translation, factors.metre)?,
        z: scale_unit(raw.tz, translation, factors.metre)?,
        rx: scale_unit(raw.rx, rotation, factors.arcsec)?,
        ry: scale_unit(raw.ry, rotation, factors.arcsec)?,
        rz: scale_unit(raw.rz, rotation, factors.arcsec)?,
        scale: scale_unit(raw.scale, scale, factors.ppm)?,
        dx,
        dy,
        dz,
        drx,
        dry,
        drz,
        ds,
        epoch,
        rated,
        convention,
    };
    Ok(Some(HelmertCandidate {
        accuracy: raw.accuracy.unwrap_or(f64::MAX),
        rank,
        step: shift.step(),
    }))
}

struct TimeShift {
    forward: bool,
    x: f64,
    y: f64,
    z: f64,
    rx: f64,
    ry: f64,
    rz: f64,
    scale: f64,
    dx: f64,
    dy: f64,
    dz: f64,
    drx: f64,
    dry: f64,
    drz: f64,
    ds: f64,
    epoch: f64,
    rated: bool,
    convention: &'static str,
}

impl TimeShift {
    fn step(&self) -> String {
        let mut parts = Vec::new();
        if !self.forward {
            parts.push("+inv".to_owned());
        }
        parts.push("+proj=helmert".to_owned());
        push_num(&mut parts, "x", self.x);
        push_num(&mut parts, "y", self.y);
        push_num(&mut parts, "z", self.z);
        push_num(&mut parts, "rx", self.rx);
        push_num(&mut parts, "ry", self.ry);
        push_num(&mut parts, "rz", self.rz);
        push_num(&mut parts, "s", self.scale);
        if self.rated {
            push_num(&mut parts, "dx", self.dx);
            push_num(&mut parts, "dy", self.dy);
            push_num(&mut parts, "dz", self.dz);
            push_num(&mut parts, "drx", self.drx);
            push_num(&mut parts, "dry", self.dry);
            push_num(&mut parts, "drz", self.drz);
            push_num(&mut parts, "ds", self.ds);
            parts.push(format!("+t_epoch={}", format_f64(self.epoch)));
        }
        let spin = self.rx != 0.0
            || self.ry != 0.0
            || self.rz != 0.0
            || self.drx != 0.0
            || self.dry != 0.0
            || self.drz != 0.0;
        if spin && !self.convention.is_empty() {
            parts.push(format!("+convention={}", self.convention));
        }
        parts.join(" ")
    }
}

fn push_num(parts: &mut Vec<String>, key: &str, value: f64) {
    if value != 0.0 {
        parts.push(format!("+{key}={}", format_f64(value)));
    }
}

fn bridge_convention(method: i64) -> Option<&'static str> {
    match method {
        1031 | 9603 | 1035 => Some(""),
        1032 | 9607 | 1038 | 1056 | 1057 | 1066 => Some("coordinate_frame"),
        1033 | 9606 | 1037 | 1053 | 1054 | 1065 => Some("position_vector"),
        _ => None,
    }
}

fn auth_rank(auth: &str) -> u8 {
    match auth {
        "EPSG" => 0,
        "PROJ" => 1,
        "ESRI" => 2,
        _ => 3,
    }
}

fn unit_factors(connection: &Connection) -> Result<UnitFactors, rusqlite::Error> {
    Ok(UnitFactors {
        metre: factor_of(connection, "9001")?,
        arcsec: factor_of(connection, "9104")?,
        ppm: factor_of(connection, "9202")?,
        metre_per_year: factor_of(connection, "1042")?,
        arcsec_per_year: factor_of(connection, "1043")?,
        ppm_per_year: factor_of(connection, "1041")?,
        year: factor_of(connection, "1029")?,
    })
}

fn factor_of(connection: &Connection, code: &str) -> Result<f64, rusqlite::Error> {
    let (_, conv, _) = unit_of(connection, "EPSG", code)?;
    conv.ok_or_else(|| unsupported(format!("EPSG unit {code} has no conversion factor")))
}

fn scale_unit(value: f64, factor: Option<f64>, target: f64) -> Result<f64, rusqlite::Error> {
    if value == 0.0 {
        return Ok(0.0);
    }
    let factor = factor.ok_or_else(|| unsupported("helmert unit is missing".to_owned()))?;
    if target == 0.0 || !factor.is_finite() || !target.is_finite() {
        return Err(unsupported("helmert unit is missing".to_owned()));
    }
    let scaled = value * factor / target;
    if scaled.is_finite() {
        Ok(scaled)
    } else {
        Err(unsupported("helmert unit is missing".to_owned()))
    }
}

fn blockers_of(
    connection: &Connection,
    from_datums: &[(String, String)],
    to_datums: &[(String, String)],
    allowed: &[String],
) -> Result<Vec<Blocker>, rusqlite::Error> {
    let mut blockers = grid_blockers(connection, from_datums, to_datums, allowed)?;
    blockers.extend(other_blockers(connection, from_datums, to_datums, allowed)?);
    Ok(blockers)
}

fn grid_blockers(
    connection: &Connection,
    from_datums: &[(String, String)],
    to_datums: &[(String, String)],
    allowed: &[String],
) -> Result<Vec<Blocker>, rusqlite::Error> {
    accuracy_rows(
        connection,
        from_datums,
        to_datums,
        allowed,
        "grid_transformation",
        "g",
    )
}

fn other_blockers(
    connection: &Connection,
    from_datums: &[(String, String)],
    to_datums: &[(String, String)],
    allowed: &[String],
) -> Result<Vec<Blocker>, rusqlite::Error> {
    let mut binds = Vec::new();
    let from_sql = datum_predicate("s", from_datums, &mut binds);
    let to_sql = datum_predicate("t", to_datums, &mut binds);
    let from_rev = datum_predicate("t", from_datums, &mut binds);
    let to_rev = datum_predicate("s", to_datums, &mut binds);
    let sql = format!(
        "SELECT o.auth_name, o.accuracy, o.grid_name, o.method_name
         FROM other_transformation o
         JOIN geodetic_crs s
           ON s.auth_name = o.source_crs_auth_name AND s.code = o.source_crs_code
         JOIN geodetic_crs t
           ON t.auth_name = o.target_crs_auth_name AND t.code = o.target_crs_code
         WHERE o.deprecated = 0
           AND ((({from_sql}) AND ({to_sql})) OR (({from_rev}) AND ({to_rev})))"
    );
    let mut statement = connection.prepare(&sql)?;
    let mapped = statement.query_map(rusqlite::params_from_iter(binds.iter()), |row| {
        let auth = column_text(row, 0)?;
        let accuracy = column_f64_opt(row, 1)?.unwrap_or(f64::MAX);
        let grid_name = column_text_opt(row, 2)?;
        let method = column_text_opt(row, 3)?.unwrap_or_default();
        Ok((auth, accuracy, grid_name, method))
    })?;
    let mut blockers = Vec::new();
    for row in mapped {
        let (auth, accuracy, grid_name, method) = row?;
        if authority_kept(&auth, allowed).is_none() {
            continue;
        }
        blockers.push(Blocker {
            grid: grid_name.as_ref().is_some_and(|text| !text.is_empty()) || grid_method(&method),
            accuracy,
        });
    }
    Ok(blockers)
}

fn grid_method(name: &str) -> bool {
    let folded = name.to_ascii_lowercase();
    folded.contains("grid") || folded.contains("deformation") || folded.contains("tinshift")
}

fn accuracy_rows(
    connection: &Connection,
    from_datums: &[(String, String)],
    to_datums: &[(String, String)],
    allowed: &[String],
    table: &str,
    alias: &str,
) -> Result<Vec<Blocker>, rusqlite::Error> {
    if !safe_ident(table) || !safe_ident(alias) {
        return Err(unsupported("CRS axis order is not built".to_owned()));
    }
    let mut binds = Vec::new();
    let from_sql = datum_predicate("s", from_datums, &mut binds);
    let to_sql = datum_predicate("t", to_datums, &mut binds);
    let from_rev = datum_predicate("t", from_datums, &mut binds);
    let to_rev = datum_predicate("s", to_datums, &mut binds);
    let sql = format!(
        "SELECT {alias}.auth_name, {alias}.accuracy
         FROM {table} {alias}
         JOIN geodetic_crs s
           ON s.auth_name = {alias}.source_crs_auth_name AND s.code = {alias}.source_crs_code
         JOIN geodetic_crs t
           ON t.auth_name = {alias}.target_crs_auth_name AND t.code = {alias}.target_crs_code
         WHERE {alias}.deprecated = 0
           AND ((({from_sql}) AND ({to_sql})) OR (({from_rev}) AND ({to_rev})))"
    );
    let mut statement = connection.prepare(&sql)?;
    let mapped = statement.query_map(rusqlite::params_from_iter(binds.iter()), |row| {
        Ok((
            column_text(row, 0)?,
            column_f64_opt(row, 1)?.unwrap_or(f64::MAX),
        ))
    })?;
    let mut blockers = Vec::new();
    for row in mapped {
        let (auth, accuracy) = row?;
        if authority_kept(&auth, allowed).is_none() {
            continue;
        }
        blockers.push(Blocker {
            grid: true,
            accuracy,
        });
    }
    Ok(blockers)
}

fn read_stored(
    connection: &Connection,
    code: u32,
    markup: Markup,
) -> Result<Option<String>, rusqlite::Error> {
    for table in ["geodetic_crs", "projected_crs", "derived_projected_crs"] {
        if let Some(text) = stored_in_table(connection, table, code, markup)? {
            return Ok(Some(text));
        }
    }
    Ok(None)
}

fn stored_in_table(
    connection: &Connection,
    table: &str,
    code: u32,
    markup: Markup,
) -> Result<Option<String>, rusqlite::Error> {
    let columns = column_names(connection, table)?;
    let mut names: Vec<&str> = Vec::new();
    for name in markup.columns() {
        if columns.iter().any(|column| column == name) {
            names.push(name);
        }
    }
    if columns.iter().any(|column| column == "text_definition") {
        names.push("text_definition");
    }
    for name in names {
        if let Some(text) = column_cell(connection, table, name, code)? {
            if markup.matches_stored(&text) {
                return Ok(Some(text));
            }
        }
    }
    Ok(None)
}

fn column_names(connection: &Connection, table: &str) -> Result<Vec<String>, rusqlite::Error> {
    if !safe_ident(table) {
        return Err(unsupported("CRS axis order is not built".to_owned()));
    }
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = statement.query_map([], |row| column_text(row, 1))?;
    rows.collect()
}

fn column_cell(
    connection: &Connection,
    table: &str,
    column: &str,
    code: u32,
) -> Result<Option<String>, rusqlite::Error> {
    if !safe_ident(table) || !safe_ident(column) {
        return Err(unsupported("CRS axis order is not built".to_owned()));
    }
    let sql = format!("SELECT {column} FROM {table} WHERE auth_name = 'EPSG' AND code = ?1");
    connection
        .query_row(&sql, params![i64::from(code)], |row| {
            column_text_opt(row, 0)
        })
        .optional()
        .map(Option::flatten)
}

fn safe_ident(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

#[cfg(test)]
mod epsg_pipeline_tests {
    use super::epsg_pipeline;
    use crate::engine::{Coord, CreateError, Operation};

    fn forward(source: &str, target: &str, coord: Coord) -> (String, Coord) {
        let (definition, _) = epsg_pipeline(source, target).unwrap_or_else(|error| {
            panic!("{source} -> {target}: {error}");
        });
        let operation = Operation::create(&definition).unwrap_or_else(|error| {
            panic!("{definition}: {error}");
        });
        let got = operation.forward(coord).unwrap_or_else(|error| {
            panic!("{definition}: {error}");
        });
        (definition, got)
    }

    fn near(got: f64, want: f64, tol: f64, label: &str) {
        assert!(
            (got - want).abs() <= tol,
            "{label}: {got} vs {want} (tol {tol})"
        );
    }

    #[test]
    fn no_grid_points_match_gie() {
        let (_, utm) = forward("EPSG:4258", "EPSG:25832", Coord::xy(55.0, 12.0));
        near(utm.x, 691_875.632_1, 1e-4, "utm east");
        near(utm.y, 6_098_907.825_0, 1e-4, "utm north");

        let (_, geocentric) = forward(
            "EPSG:4979",
            "EPSG:4978",
            Coord {
                x: 55.0,
                y: 12.0,
                z: 10.0,
                t: 0.0,
            },
        );
        near(geocentric.x, 3_586_475.267_2, 1e-4, "cart x");
        near(geocentric.y, 762_328.851_3, 1e-4, "cart y");
        near(geocentric.z, 5_201_391.714_7, 1e-4, "cart z");

        let (_, epoch) = forward(
            "EPSG:7843",
            "EPSG:7912",
            Coord {
                x: -33.8623,
                y: 151.2077,
                z: 0.0,
                t: 2020.0,
            },
        );
        near(epoch.x, -33.8623, 1e-9, "gda 2020 lat");
        near(epoch.y, 151.2077, 1e-9, "gda 2020 lon");
        near(epoch.z, 0.0, 1e-4, "gda 2020 h");

        let (_, later) = forward(
            "EPSG:7843",
            "EPSG:7912",
            Coord {
                x: -33.8623,
                y: 151.2077,
                z: 0.0,
                t: 2026.0,
            },
        );
        near(later.x, -33.862_297_056, 9e-10, "gda 2026 lat");
        near(later.y, 151.207_701_181, 9e-10, "gda 2026 lon");
        near(later.z, -0.001_01, 1e-4, "gda 2026 h");

        let (_, finland) = forward("EPSG:4123", "EPSG:2393", Coord::xy(60.1699, 24.9384));
        near(finland.x, 6_674_944.774_2, 1e-4, "ykj north");
        near(finland.y, 3_385_559.815_1, 1e-4, "ykj east");
    }

    #[test]
    fn nad_grid_is_not_a_coordinate() {
        let error = epsg_pipeline("EPSG:4267", "EPSG:4269").expect_err("nad");
        assert!(matches!(error, CreateError::Unsupported("grid")), "{error}");
    }
}
