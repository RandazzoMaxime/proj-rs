//! NKG coordinate operations stored in the embedded PROJ database.
//!
//! PROJ 9.9.0 reads them in `src/iso19111/factory.cpp`
//! (`AuthorityFactory::createCoordinateOperation`) and emits a Helmert pipeline
//! from `src/iso19111/operation/singleoperation.cpp`. The numeric step is the
//! linearized time-dependent Helmert in `src/transformations/helmert.cpp`
//! (`update_parameters`, approximate `build_rot_matrix`). The registry rows are
//! `data/sql/nkg.sql`.
//!
//! A PROJ-string step, including `+proj=deformation` and `+proj=xyzgridshift`,
//! is built when its grid file is on disk or fetchable with the network. A
//! missing grid is `Unsupported("grid")`, never a coordinate.

use crate::engine::{Coord, CreateError, Frame, IoKind, Operation, Params, Step, TransformError};
use crate::helmert::{Helmert, HelmertConvention, HelmertParams};
use crate::projdb::{with_db, DbFail};
use crate::Ellipsoid;
use rusqlite::types::ValueRef;
use rusqlite::{params, Connection, Row};

/// `+code` is the NKG operation code. No other parameter is accepted.
pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "nkg" {
        Some(&["code"])
    } else {
        None
    }
}

/// Build `+proj=nkg +code=…`. `None` when `name` is not `nkg`.
///
/// # Errors
///
/// [`CreateError::MissingArgument`] without `+code`, [`CreateError::UnknownParameter`]
/// for any other key, or the same errors as [`from_definition`] when the code
/// needs a grid or is not a Helmert this crate builds.
pub(crate) fn build(
    name: &str,
    params: &Params,
    _frame: &Frame,
) -> Result<Option<Box<dyn Step>>, CreateError> {
    if name != "nkg" {
        return Ok(None);
    }
    let allowed = extra_keys(name).unwrap_or(&[]);
    for (key, _) in params.entries() {
        if key != "proj" && !allowed.contains(&key) {
            return Err(CreateError::UnknownParameter(key.to_owned()));
        }
    }
    match params.text("code") {
        Some(code) if !code.is_empty() => from_code(code).map(Some),
        Some(_) => Err(CreateError::IllegalValue("code")),
        None => Err(CreateError::MissingArgument("code")),
    }
}

/// Resolve an NKG coordinate-operation URN, or `+proj=nkg`.
///
/// `Ok(None)` when `definition` is neither. A grid step is
/// [`CreateError::Unsupported`] with `"grid"`.
///
/// # Errors
///
/// See [`build`]. An unknown NKG code is [`CreateError::Unsupported`].
pub(crate) fn from_definition(definition: &str) -> Result<Option<Box<dyn Step>>, CreateError> {
    if let Some(code) = urn_code(definition) {
        return from_code(&code).map(Some);
    }
    if !is_proj_nkg(definition) {
        return Ok(None);
    }
    let params = Params::parse(definition)?;
    if params.text("proj") != Some("nkg") {
        return Ok(None);
    }
    // The frame is unused: NKG Helmert steps are already geocentric.
    build("nkg", &params, &unused_frame())
}

fn unused_frame() -> Frame {
    Frame {
        earth: Ellipsoid::grs80(),
        lam0: 0.0,
        phi0: 0.0,
        x0: 0.0,
        y0: 0.0,
        z0: 0.0,
        k0: 1.0,
        to_meter: 1.0,
        fr_meter: 1.0,
        vto_meter: 1.0,
        vfr_meter: 1.0,
        over: false,
        geoc: false,
        from_greenwich: 0.0,
        lon_wrap: None,
    }
}

fn from_code(code: &str) -> Result<Box<dyn Step>, CreateError> {
    if code.is_empty() {
        return Err(CreateError::IllegalValue("code"));
    }
    match load(code)? {
        Found::Missing => Err(CreateError::Unsupported("authority string")),
        Found::Grid => Err(CreateError::Unsupported("grid")),
        Found::Unsupported(reason) => Err(CreateError::Unsupported(reason)),
        Found::Ready(links) => chain_from(links),
    }
}

fn urn_code(definition: &str) -> Option<String> {
    let text = definition.trim();
    let rest = strip_urn_prefix(text)?;
    let mut parts = rest.split(':');
    let object = parts.next()?;
    if !object.eq_ignore_ascii_case("coordinateOperation") {
        return None;
    }
    let authority = parts.next()?;
    if !authority.eq_ignore_ascii_case("NKG") {
        return None;
    }
    let version_or_code = parts.next()?;
    let code = match parts.next() {
        Some(code) => code,
        None => version_or_code,
    };
    if code.is_empty() || parts.next().is_some() {
        return None;
    }
    Some(code.to_owned())
}

fn strip_urn_prefix(text: &str) -> Option<&str> {
    const PREFIX: &str = "urn:ogc:def:";
    if text.len() < PREFIX.len() {
        return None;
    }
    if text[..PREFIX.len()].eq_ignore_ascii_case(PREFIX) {
        Some(&text[PREFIX.len()..])
    } else {
        None
    }
}

fn is_proj_nkg(definition: &str) -> bool {
    definition.split_whitespace().any(|token| {
        let token = token.trim_start_matches('+');
        token == "proj=nkg"
    })
}

fn load(code: &str) -> Result<Found, CreateError> {
    match with_db(|connection| describe(connection, "NKG", code, &mut Vec::new())) {
        Ok(found) => Ok(found),
        Err(DbFail::Open(_) | DbFail::Sql(_)) => Err(CreateError::Unsupported("proj.db")),
    }
}

enum Found {
    Missing,
    Grid,
    Unsupported(&'static str),
    Ready(Vec<Link>),
}

#[derive(Clone, Copy)]
enum Sense {
    Forward,
    Reverse,
}

enum Link {
    Helmert(Sense, Shift),
    Proj(Sense, String),
}

#[derive(Clone, Copy)]
struct Shift {
    tx: f64,
    ty: f64,
    tz: f64,
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
    convention: HelmertConvention,
}

impl Shift {
    fn rated(self) -> bool {
        self.dx != 0.0
            || self.dy != 0.0
            || self.dz != 0.0
            || self.drx != 0.0
            || self.dry != 0.0
            || self.drz != 0.0
            || self.ds != 0.0
    }
}

struct Targets {
    metre: f64,
    arcsec: f64,
    ppm: f64,
    metre_per_year: f64,
    arcsec_per_year: f64,
    ppm_per_year: f64,
    year: f64,
}

fn describe(
    connection: &Connection,
    auth: &str,
    code: &str,
    stack: &mut Vec<(String, String)>,
) -> Result<Found, rusqlite::Error> {
    if stack
        .iter()
        .any(|(seen_auth, seen_code)| seen_auth == auth && seen_code == code)
    {
        return Ok(Found::Unsupported("authority string"));
    }
    stack.push((auth.to_owned(), code.to_owned()));
    let found = describe_one(connection, auth, code, stack)?;
    stack.pop();
    Ok(found)
}

fn describe_one(
    connection: &Connection,
    auth: &str,
    code: &str,
    stack: &mut Vec<(String, String)>,
) -> Result<Found, rusqlite::Error> {
    if table_has(
        connection,
        "SELECT 1 FROM concatenated_operation WHERE auth_name=?1 AND code=?2",
        auth,
        code,
    )? {
        return describe_concat(connection, auth, code, stack);
    }
    if table_has(
        connection,
        "SELECT 1 FROM helmert_transformation WHERE auth_name=?1 AND code=?2",
        auth,
        code,
    )? {
        let targets = targets(connection)?;
        return read_helmert(connection, auth, code, &targets);
    }
    if table_has(
        connection,
        "SELECT 1 FROM other_transformation WHERE auth_name=?1 AND code=?2",
        auth,
        code,
    )? {
        return read_other(connection, auth, code);
    }
    if table_has(
        connection,
        "SELECT 1 FROM grid_transformation WHERE auth_name=?1 AND code=?2",
        auth,
        code,
    )? {
        return Ok(Found::Grid);
    }
    Ok(Found::Missing)
}

fn describe_concat(
    connection: &Connection,
    auth: &str,
    code: &str,
    stack: &mut Vec<(String, String)>,
) -> Result<Found, rusqlite::Error> {
    let mut statement = connection.prepare(
        "SELECT step_auth_name, step_code, step_direction
         FROM concatenated_operation_step
         WHERE operation_auth_name=?1 AND operation_code=?2
         ORDER BY step_number",
    )?;
    let rows = statement.query_map(params![auth, code], |row| {
        Ok((cell_text(row, 0)?, cell_text(row, 1)?, cell_text(row, 2)?))
    })?;
    let mut links = Vec::new();
    let mut unsupported = None;
    let mut saw_step = false;
    for row in rows {
        let (step_auth, step_code, direction) = row?;
        let (Some(step_auth), Some(step_code)) = (step_auth, step_code) else {
            return Ok(Found::Unsupported("authority string"));
        };
        saw_step = true;
        let sense = sense_of(direction.as_deref());
        match describe(connection, &step_auth, &step_code, stack)? {
            Found::Grid => return Ok(Found::Grid),
            Found::Missing => return Ok(Found::Unsupported("authority string")),
            Found::Unsupported(reason) => unsupported = Some(reason),
            Found::Ready(more) => {
                if unsupported.is_none() {
                    links.extend(more.into_iter().map(|link| link.with_sense(sense)));
                }
            }
        }
    }
    if !saw_step {
        return Ok(Found::Unsupported("authority string"));
    }
    if let Some(reason) = unsupported {
        return Ok(Found::Unsupported(reason));
    }
    Ok(Found::Ready(links))
}

fn sense_of(direction: Option<&str>) -> Sense {
    match direction {
        Some(text) if text.eq_ignore_ascii_case("reverse") => Sense::Reverse,
        _ => Sense::Forward,
    }
}

fn table_has(
    connection: &Connection,
    sql: &str,
    auth: &str,
    code: &str,
) -> Result<bool, rusqlite::Error> {
    match connection.query_row(sql, params![auth, code], |_| Ok(())) {
        Ok(()) => Ok(true),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
        Err(error) => Err(error),
    }
}

fn targets(connection: &Connection) -> Result<Targets, rusqlite::Error> {
    Ok(Targets {
        metre: required_conv(connection, "9001")?,
        arcsec: required_conv(connection, "9104")?,
        ppm: required_conv(connection, "9202")?,
        metre_per_year: required_conv(connection, "1042")?,
        arcsec_per_year: required_conv(connection, "1043")?,
        ppm_per_year: required_conv(connection, "1041")?,
        year: required_conv(connection, "1029")?,
    })
}

fn required_conv(connection: &Connection, code: &str) -> Result<f64, rusqlite::Error> {
    conv(connection, "EPSG", code)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

fn conv(connection: &Connection, auth: &str, code: &str) -> Result<Option<f64>, rusqlite::Error> {
    match connection.query_row(
        "SELECT conv_factor FROM unit_of_measure WHERE auth_name=?1 AND code=?2",
        params![auth, code],
        |row| cell_f64(row, 0),
    ) {
        Ok(value) => Ok(value),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(error),
    }
}

fn read_helmert(
    connection: &Connection,
    auth: &str,
    code: &str,
    targets: &Targets,
) -> Result<Found, rusqlite::Error> {
    let row = connection.query_row(
        "SELECT method_code, tx, ty, tz, translation_uom_auth_name, translation_uom_code,
                rx, ry, rz, rotation_uom_auth_name, rotation_uom_code,
                scale_difference, scale_difference_uom_auth_name, scale_difference_uom_code,
                rate_tx, rate_ty, rate_tz, rate_translation_uom_auth_name, rate_translation_uom_code,
                rate_rx, rate_ry, rate_rz, rate_rotation_uom_auth_name, rate_rotation_uom_code,
                rate_scale_difference, rate_scale_difference_uom_auth_name,
                rate_scale_difference_uom_code,
                epoch, epoch_uom_auth_name, epoch_uom_code, px, py, pz
         FROM helmert_transformation WHERE auth_name=?1 AND code=?2",
        params![auth, code],
        |row| {
            Ok((
                cell_i64(row, 0)?,
                cell_f64(row, 1)?,
                cell_f64(row, 2)?,
                cell_f64(row, 3)?,
                cell_text(row, 4)?,
                cell_text(row, 5)?,
                cell_f64(row, 6)?,
                cell_f64(row, 7)?,
                cell_f64(row, 8)?,
                cell_text(row, 9)?,
                cell_text(row, 10)?,
                cell_f64(row, 11)?,
                cell_text(row, 12)?,
                cell_text(row, 13)?,
                cell_f64(row, 14)?,
                cell_f64(row, 15)?,
                cell_f64(row, 16)?,
                cell_text(row, 17)?,
                cell_text(row, 18)?,
                cell_f64(row, 19)?,
                cell_f64(row, 20)?,
                cell_f64(row, 21)?,
                cell_text(row, 22)?,
                cell_text(row, 23)?,
                cell_f64(row, 24)?,
                cell_text(row, 25)?,
                cell_text(row, 26)?,
                cell_f64(row, 27)?,
                cell_text(row, 28)?,
                cell_text(row, 29)?,
                cell_f64(row, 30)?,
                cell_f64(row, 31)?,
                cell_f64(row, 32)?,
            ))
        },
    );
    let (
        method,
        tx,
        ty,
        tz,
        translation_auth,
        translation_code,
        rx,
        ry,
        rz,
        rotation_auth,
        rotation_code,
        scale,
        scale_auth,
        scale_code,
        dx,
        dy,
        dz,
        rate_translation_auth,
        rate_translation_code,
        drx,
        dry,
        drz,
        rate_rotation_auth,
        rate_rotation_code,
        ds,
        rate_scale_auth,
        rate_scale_code,
        epoch,
        epoch_auth,
        epoch_code,
        px,
        py,
        pz,
    ) = match row {
        Ok(values) => values,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(Found::Missing),
        Err(error) => return Err(error),
    };
    if pivot_set(px, py, pz) {
        return Ok(Found::Unsupported("molodensky"));
    }
    let Some(method) = method else {
        return Ok(Found::Unsupported("helmert method"));
    };
    let convention = match geocentric_convention(method) {
        Ok(convention) => convention,
        Err(reason) => return Ok(Found::Unsupported(reason)),
    };
    let translation_unit = unit_factor(connection, translation_auth, translation_code)?;
    let rotation_unit = unit_factor(connection, rotation_auth, rotation_code)?;
    let scale_unit = unit_factor(connection, scale_auth, scale_code)?;
    let rate_translation_unit =
        unit_factor(connection, rate_translation_auth, rate_translation_code)?;
    let rate_rotation_unit = unit_factor(connection, rate_rotation_auth, rate_rotation_code)?;
    let rate_scale_unit = unit_factor(connection, rate_scale_auth, rate_scale_code)?;
    let epoch_unit = unit_factor(connection, epoch_auth, epoch_code)?;
    let shift = match scaled_shift(ShiftParts {
        targets,
        convention,
        tx: tx.unwrap_or(0.0),
        ty: ty.unwrap_or(0.0),
        tz: tz.unwrap_or(0.0),
        translation_unit,
        rx: rx.unwrap_or(0.0),
        ry: ry.unwrap_or(0.0),
        rz: rz.unwrap_or(0.0),
        rotation_unit,
        scale: scale.unwrap_or(0.0),
        scale_unit,
        dx: dx.unwrap_or(0.0),
        dy: dy.unwrap_or(0.0),
        dz: dz.unwrap_or(0.0),
        rate_translation_unit,
        drx: drx.unwrap_or(0.0),
        dry: dry.unwrap_or(0.0),
        drz: drz.unwrap_or(0.0),
        rate_rotation_unit,
        ds: ds.unwrap_or(0.0),
        rate_scale_unit,
        epoch: epoch.unwrap_or(0.0),
        epoch_unit,
    }) {
        Ok(shift) => shift,
        Err(reason) => return Ok(Found::Unsupported(reason)),
    };
    Ok(Found::Ready(vec![Link::Helmert(Sense::Forward, shift)]))
}

struct ShiftParts<'a> {
    targets: &'a Targets,
    convention: HelmertConvention,
    tx: f64,
    ty: f64,
    tz: f64,
    translation_unit: Option<f64>,
    rx: f64,
    ry: f64,
    rz: f64,
    rotation_unit: Option<f64>,
    scale: f64,
    scale_unit: Option<f64>,
    dx: f64,
    dy: f64,
    dz: f64,
    rate_translation_unit: Option<f64>,
    drx: f64,
    dry: f64,
    drz: f64,
    rate_rotation_unit: Option<f64>,
    ds: f64,
    rate_scale_unit: Option<f64>,
    epoch: f64,
    epoch_unit: Option<f64>,
}

fn pivot_set(px: Option<f64>, py: Option<f64>, pz: Option<f64>) -> bool {
    px.unwrap_or(0.0) != 0.0 || py.unwrap_or(0.0) != 0.0 || pz.unwrap_or(0.0) != 0.0
}

fn geocentric_convention(method: i64) -> Result<HelmertConvention, &'static str> {
    match method {
        // Geocentric translations, position vector, and its time-dependent forms.
        1031 | 1033 | 1053 | 1065 => Ok(HelmertConvention::PositionVector),
        // Coordinate-frame rotations, including the time-dependent forms.
        1032 | 1056 | 1066 => Ok(HelmertConvention::CoordinateFrame),
        _ => Err("helmert method"),
    }
}

fn unit_factor(
    connection: &Connection,
    auth: Option<String>,
    code: Option<String>,
) -> Result<Option<f64>, rusqlite::Error> {
    match (auth, code) {
        (Some(auth), Some(code)) => conv(connection, &auth, &code),
        _ => Ok(None),
    }
}

fn scaled_shift(parts: ShiftParts<'_>) -> Result<Shift, &'static str> {
    let targets = parts.targets;
    Ok(Shift {
        tx: scale_to(parts.tx, parts.translation_unit, targets.metre)?,
        ty: scale_to(parts.ty, parts.translation_unit, targets.metre)?,
        tz: scale_to(parts.tz, parts.translation_unit, targets.metre)?,
        rx: scale_to(parts.rx, parts.rotation_unit, targets.arcsec)?,
        ry: scale_to(parts.ry, parts.rotation_unit, targets.arcsec)?,
        rz: scale_to(parts.rz, parts.rotation_unit, targets.arcsec)?,
        scale: scale_to(parts.scale, parts.scale_unit, targets.ppm)?,
        dx: scale_to(
            parts.dx,
            parts.rate_translation_unit,
            targets.metre_per_year,
        )?,
        dy: scale_to(
            parts.dy,
            parts.rate_translation_unit,
            targets.metre_per_year,
        )?,
        dz: scale_to(
            parts.dz,
            parts.rate_translation_unit,
            targets.metre_per_year,
        )?,
        drx: scale_to(parts.drx, parts.rate_rotation_unit, targets.arcsec_per_year)?,
        dry: scale_to(parts.dry, parts.rate_rotation_unit, targets.arcsec_per_year)?,
        drz: scale_to(parts.drz, parts.rate_rotation_unit, targets.arcsec_per_year)?,
        ds: scale_to(parts.ds, parts.rate_scale_unit, targets.ppm_per_year)?,
        epoch: scale_to(parts.epoch, parts.epoch_unit, targets.year)?,
        convention: parts.convention,
    })
}

fn scale_to(value: f64, source: Option<f64>, target: f64) -> Result<f64, &'static str> {
    if value == 0.0 {
        return Ok(0.0);
    }
    let source = source.ok_or("helmert unit")?;
    if target == 0.0 || !source.is_finite() || !target.is_finite() {
        return Err("helmert unit");
    }
    let scaled = value * source / target;
    if scaled.is_finite() {
        Ok(scaled)
    } else {
        Err("helmert unit")
    }
}

fn read_other(connection: &Connection, auth: &str, code: &str) -> Result<Found, rusqlite::Error> {
    let (method_auth, method_code, method_name, grid_name) = connection.query_row(
        "SELECT method_auth_name, method_code, method_name, grid_name
         FROM other_transformation WHERE auth_name=?1 AND code=?2",
        params![auth, code],
        |row| {
            Ok((
                cell_text(row, 0)?,
                cell_text(row, 1)?,
                cell_text(row, 2)?,
                cell_text(row, 3)?,
            ))
        },
    )?;
    let Some(method_name) = method_name else {
        return Ok(if grid_name.is_some() {
            Found::Grid
        } else {
            Found::Unsupported("authority string")
        });
    };
    if method_code.as_deref() == Some("PROJString") && method_auth.as_deref() == Some("PROJ") {
        if !grids_available(&method_name) {
            return Ok(Found::Grid);
        }
        return Ok(Found::Ready(vec![Link::Proj(Sense::Forward, method_name)]));
    }
    if grid_name.is_some() || needs_grid(&method_name) {
        return Ok(Found::Grid);
    }
    Ok(Found::Unsupported("authority string"))
}

/// `false` when a required grid named by `+grids=`, `+xy_grids=`, or `+z_grids=`
/// is neither on disk nor fetchable. Optional (`@`) grids never block.
fn grids_available(definition: &str) -> bool {
    definition
        .split_whitespace()
        .filter_map(|token| {
            let (key, value) = token.trim_start_matches('+').split_once('=')?;
            matches!(key, "grids" | "xy_grids" | "z_grids").then_some(value)
        })
        .flat_map(|value| value.split(','))
        .filter(|name| !name.is_empty() && !name.starts_with('@'))
        .all(crate::gridpath::available)
}

fn needs_grid(definition: &str) -> bool {
    let folded = definition.to_ascii_lowercase();
    if folded.contains("grids=") || folded.contains("xy_grids=") || folded.contains("z_grids=") {
        return true;
    }
    const NAMES: &[&str] = &[
        "deformation",
        "gridshift",
        "hgridshift",
        "vgridshift",
        "xyzgridshift",
        "tinshift",
        "horner",
        "defmodel",
    ];
    folded.split_whitespace().any(|token| {
        let token = token.trim_start_matches('+');
        token
            .strip_prefix("proj=")
            .is_some_and(|name| NAMES.contains(&name))
    })
}

impl Link {
    fn with_sense(self, outer: Sense) -> Self {
        match outer {
            Sense::Forward => self,
            Sense::Reverse => match self {
                Self::Helmert(sense, shift) => Self::Helmert(flip(sense), shift),
                Self::Proj(sense, text) => Self::Proj(flip(sense), text),
            },
        }
    }
}

fn flip(sense: Sense) -> Sense {
    match sense {
        Sense::Forward => Sense::Reverse,
        Sense::Reverse => Sense::Forward,
    }
}

#[derive(Clone, Copy)]
enum Direction {
    Forward,
    Inverse,
}

fn apply_shift(shift: Shift, coord: Coord, direction: Direction) -> Result<Coord, TransformError> {
    let helmert = helmert_at(shift, coord.t)?;
    let (x, y, z) = match direction {
        Direction::Forward => helmert.apply(coord.x, coord.y, coord.z),
        Direction::Inverse => helmert.apply_inverse(coord.x, coord.y, coord.z),
    };
    Ok(Coord {
        x,
        y,
        z,
        t: coord.t,
    })
}

fn helmert_at(shift: Shift, t: f64) -> Result<Helmert, TransformError> {
    let t_obs = if shift.rated() && t.is_finite() && t != 0.0 {
        t
    } else {
        shift.epoch
    };
    let dt = t_obs - shift.epoch;
    let scale = shift.scale + shift.ds * dt;
    let params = HelmertParams {
        translation_x: shift.tx + shift.dx * dt,
        translation_y: shift.ty + shift.dy * dt,
        translation_z: shift.tz + shift.dz * dt,
        rotation_x_arcsec: shift.rx + shift.drx * dt,
        rotation_y_arcsec: shift.ry + shift.dry * dt,
        rotation_z_arcsec: shift.rz + shift.drz * dt,
        scale_ppm: scale,
        convention: shift.convention,
    };
    if scale <= -1.0e6
        || !params.translation_x.is_finite()
        || !params.translation_y.is_finite()
        || !params.translation_z.is_finite()
        || !params.rotation_x_arcsec.is_finite()
        || !params.rotation_y_arcsec.is_finite()
        || !params.rotation_z_arcsec.is_finite()
        || !params.scale_ppm.is_finite()
    {
        return Err(TransformError::InvalidCoord);
    }
    Helmert::try_new(params).map_err(|_| TransformError::NonFinite)
}

fn chain_from(links: Vec<Link>) -> Result<Box<dyn Step>, CreateError> {
    if links.is_empty() {
        return Err(CreateError::IllegalValue("nkg"));
    }
    let mut built = Vec::with_capacity(links.len());
    for link in links {
        built.push(bake(link)?);
    }
    let earth = Ellipsoid::grs80();
    Ok(Box::new(Chain {
        links: built,
        a: earth.a(),
        es: earth.es(),
    }))
}

fn bake(link: Link) -> Result<Baked, CreateError> {
    match link {
        Link::Helmert(sense, shift) => Ok(Baked::Helmert(sense, shift)),
        Link::Proj(sense, text) => {
            // A nested NKG string would re-enter this loader. None of the
            // registry rows do that; reject it instead of recursing.
            if urn_code(&text).is_some() || is_proj_nkg(&text) {
                return Err(CreateError::Unsupported("authority string"));
            }
            let operation = Operation::create(&text)?;
            Ok(Baked::Other(sense, operation))
        }
    }
}

enum Baked {
    Helmert(Sense, Shift),
    Other(Sense, Operation),
}

impl Baked {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        match self {
            Self::Helmert(Sense::Forward, shift) => apply_shift(*shift, coord, Direction::Forward),
            Self::Helmert(Sense::Reverse, shift) => apply_shift(*shift, coord, Direction::Inverse),
            Self::Other(Sense::Forward, operation) => operation.forward(coord),
            Self::Other(Sense::Reverse, operation) => operation.inverse(coord),
        }
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        match self {
            Self::Helmert(Sense::Forward, shift) => apply_shift(*shift, coord, Direction::Inverse),
            Self::Helmert(Sense::Reverse, shift) => apply_shift(*shift, coord, Direction::Forward),
            Self::Other(Sense::Forward, operation) => operation.inverse(coord),
            Self::Other(Sense::Reverse, operation) => operation.forward(coord),
        }
    }
}

struct Chain {
    links: Vec<Baked>,
    a: f64,
    es: f64,
}

impl Step for Chain {
    fn forward(&self, mut coord: Coord) -> Result<Coord, TransformError> {
        for link in &self.links {
            coord = link.forward(coord)?;
        }
        Ok(coord)
    }

    fn inverse(&self, mut coord: Coord) -> Result<Coord, TransformError> {
        for link in self.links.iter().rev() {
            coord = link.inverse(coord)?;
        }
        Ok(coord)
    }

    fn input_kind(&self) -> IoKind {
        IoKind::Cartesian
    }

    fn output_kind(&self) -> IoKind {
        IoKind::Cartesian
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (self.a, self.es)
    }
}

fn cell_f64(row: &Row<'_>, index: usize) -> Result<Option<f64>, rusqlite::Error> {
    match row.get_ref(index)? {
        ValueRef::Null => Ok(None),
        ValueRef::Real(value) => Ok(Some(value)),
        ValueRef::Integer(value) => Ok(Some(i64_to_f64(value))),
        ValueRef::Text(bytes) => {
            let text = std::str::from_utf8(bytes)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            text.parse::<f64>()
                .map(Some)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
        }
        ValueRef::Blob(_) => Err(rusqlite::Error::InvalidColumnType(
            index,
            "blob".to_owned(),
            rusqlite::types::Type::Blob,
        )),
    }
}

fn cell_text(row: &Row<'_>, index: usize) -> Result<Option<String>, rusqlite::Error> {
    match row.get_ref(index)? {
        ValueRef::Null => Ok(None),
        ValueRef::Text(bytes) => {
            let text = std::str::from_utf8(bytes)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            Ok(Some(text.to_owned()))
        }
        ValueRef::Integer(value) => Ok(Some(value.to_string())),
        ValueRef::Real(value) => Ok(Some(whole_real(value).unwrap_or_else(|| value.to_string()))),
        ValueRef::Blob(_) => Err(rusqlite::Error::InvalidColumnType(
            index,
            "blob".to_owned(),
            rusqlite::types::Type::Blob,
        )),
    }
}

fn cell_i64(row: &Row<'_>, index: usize) -> Result<Option<i64>, rusqlite::Error> {
    match row.get_ref(index)? {
        ValueRef::Null => Ok(None),
        ValueRef::Integer(value) => Ok(Some(value)),
        ValueRef::Text(bytes) => {
            let text = std::str::from_utf8(bytes)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            text.parse::<i64>()
                .map(Some)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
        }
        ValueRef::Real(value) => whole_i64(value).map(Some).ok_or_else(|| {
            rusqlite::Error::InvalidColumnType(
                index,
                "real".to_owned(),
                rusqlite::types::Type::Real,
            )
        }),
        ValueRef::Blob(_) => Err(rusqlite::Error::InvalidColumnType(
            index,
            "blob".to_owned(),
            rusqlite::types::Type::Blob,
        )),
    }
}

fn whole_real(value: f64) -> Option<String> {
    whole_i64(value).map(|number| number.to_string())
}

fn whole_i64(value: f64) -> Option<i64> {
    if !value.is_finite() || value.fract() != 0.0 {
        return None;
    }
    let text = format!("{value:.0}");
    text.parse().ok()
}

#[expect(
    clippy::cast_precision_loss,
    clippy::as_conversions,
    reason = "SQLite integers used as Helmert parameters fit the f64 mantissa"
)]
fn i64_to_f64(value: i64) -> f64 {
    value as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point() -> Coord {
        Coord {
            x: 3_541_657.377_8,
            y: 948_984.234_3,
            z: 5_201_383.523_1,
            t: 2020.5,
        }
    }

    fn near(got: Coord, x: f64, y: f64, z: f64) {
        let limit = 1e-6;
        assert!((got.x - x).abs() < limit, "x {} vs {x}", got.x);
        assert!((got.y - y).abs() < limit, "y {} vs {y}", got.y);
        assert!((got.z - z).abs() < limit, "z {} vs {z}", got.z);
    }

    #[test]
    fn static_helmert_urn_matches_cct() {
        let operation = Operation::create("urn:ogc:def:coordinateOperation:NKG::P1_2008_DK")
            .expect("P1_2008_DK");
        let got = operation.forward(point()).expect("forward");
        near(
            got,
            3_541_657.384_127_005,
            948_984.217_393_669,
            5_201_383.529_415_069,
        );
        assert!((got.t - 2020.5).abs() < 1e-12);
        assert!(!operation.angular_input(false));
        let back = operation.inverse(got).expect("inverse");
        near(back, point().x, point().y, point().z);
    }

    #[test]
    fn proj_string_matches_the_urn() {
        let urn = Operation::create("urn:ogc:def:coordinateOperation:NKG:1.0:P1_2008_DK")
            .expect("versioned");
        let proj = Operation::create("+proj=nkg +code=P1_2008_DK").expect("proj");
        let left = urn.forward(point()).expect("urn");
        let right = proj.forward(point()).expect("proj");
        near(right, left.x, left.y, left.z);
    }

    #[test]
    fn lowercase_authority_is_accepted() {
        let operation =
            Operation::create("urn:ogc:def:coordinateOperation:nkg::P1_2008_DK").expect("lower");
        let got = operation.forward(point()).expect("forward");
        near(
            got,
            3_541_657.384_127_005,
            948_984.217_393_669,
            5_201_383.529_415_069,
        );
    }

    #[test]
    fn time_dependent_epsg_7941_matches_cct() {
        let found = load_authority("EPSG", "7941").expect("load");
        let Found::Ready(links) = found else {
            panic!("7941 should be a helmert");
        };
        let operation = chain_from(links).expect("chain");
        let got = operation.forward(point()).expect("2020.5");
        near(
            got,
            3_541_657.935_805_580,
            948_983.792_590_651,
            5_201_383.221_813_221,
        );
        let at_epoch = operation
            .forward(Coord {
                t: 2000.0,
                ..point()
            })
            .expect("epoch");
        near(
            at_epoch,
            3_541_657.607_801_949,
            948_984.113_242_767,
            5_201_383.386_650_648,
        );
        let unset = operation
            .forward(Coord { t: 0.0, ..point() })
            .expect("unset time uses t_epoch");
        near(unset, at_epoch.x, at_epoch.y, at_epoch.z);
    }

    #[test]
    fn urns_without_a_grid_build_and_grid_urns_are_refused() {
        let grid_free = [
            "P1_2008_DK",
            "P1_2008_EE",
            "P1_2008_FI",
            "P1_2008_LT",
            "P1_2008_LV",
            "P1_2008_NO",
            "P1_2008_SE",
            "PAR_2020_DK",
            "PAR_2020_EE",
            "PAR_2020_FI",
            "PAR_2020_LT",
            "PAR_2020_LV",
            "PAR_2020_NO",
            "PAR_2020_SE",
        ];
        for code in grid_free {
            let urn = format!("urn:ogc:def:coordinateOperation:NKG::{code}");
            Operation::create(&urn).unwrap_or_else(|error| panic!("{code}: {error}"));
        }
        let needs_grid = [
            "ITRF2000_TO_NKG_ETRF00",
            "ITRF2000_TO_DK",
            "ETRF00_TO_DK",
            "ITRF2014_TO_DK",
            "ITRF2014_TO_NKG_ETRF14",
            "ETRF14_TO_DK",
            "ITRF2000_TO_EE",
            "ETRF00_TO_EE",
            "ITRF2014_TO_EE",
            "ITRF2000_TO_FI",
            "ITRF2000_TO_FI_EUREF-FIN",
            "ETRF00_TO_FI",
            "ITRF2014_TO_FI",
            "ITRF2014_TO_FI_EUREF-FIN",
            "ITRF2000_TO_LV",
            "ETRF00_TO_LV",
            "ITRF2014_TO_LV",
            "ITRF2000_TO_LT",
            "ETRF00_TO_LT",
            "ITRF2014_TO_LT",
            "ITRF2000_TO_NO",
            "ETRF00_TO_NO",
            "ITRF2014_TO_NO",
            "ITRF2000_TO_SE",
            "ETRF00_TO_SE",
            "ITRF2014_TO_SE",
        ];
        for code in needs_grid {
            let urn = format!("urn:ogc:def:coordinateOperation:NKG::{code}");
            let error = Operation::create(&urn).err();
            assert_eq!(error, Some(CreateError::Unsupported("grid")), "{code}");
        }
    }

    #[test]
    fn unknown_nkg_code_is_an_error() {
        let error =
            Operation::create("urn:ogc:def:coordinateOperation:NKG::NO_SUCH").expect_err("missing");
        assert_eq!(error, CreateError::Unsupported("authority string"));
    }

    #[test]
    fn epsg_urn_is_left_to_the_caller() {
        assert!(
            from_definition("urn:ogc:def:coordinateOperation:EPSG::7941")
                .expect("parse")
                .is_none()
        );
    }

    #[test]
    fn nkg_rejects_unknown_parameters_and_a_missing_code() {
        let missing = Operation::create("+proj=nkg").expect_err("code");
        assert_eq!(missing, CreateError::MissingArgument("code"));
        let extra =
            Operation::create("+proj=nkg +code=P1_2008_DK +ellps=GRS80").expect_err("ellps");
        assert_eq!(extra, CreateError::UnknownParameter("ellps".to_owned()));
    }

    fn load_authority(auth: &str, code: &str) -> Result<Found, CreateError> {
        match with_db(|connection| describe(connection, auth, code, &mut Vec::new())) {
            Ok(found) => Ok(found),
            Err(DbFail::Open(_) | DbFail::Sql(_)) => Err(CreateError::Unsupported("proj.db")),
        }
    }
}
