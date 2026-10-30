//! `AuthorityFactory` and `CoordinateOperationFactory` over the embedded `proj.db`.
//!
//! The statements are the ones in PROJ `src/iso19111/factory.cpp`. A missing
//! code is an error. A `text_definition` is not turned into a coordinate.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use rusqlite::{types::ValueRef, Connection};

use super::model::{
    ensemble_name_as_datum, Axis, CartesianCS, CompoundCRS, ConcatenatedOperation,
    ConcatenatedStep, Conversion, CoordinateOperation, CoordinateSystem, Crs, Datum, DatumEnsemble,
    DerivedProjectedCRS, Ellipsoid, EllipsoidalCS, EngineeringCRS, EngineeringDatum, FactoryError,
    GeocentricCRS, GeodeticCRS, GeodeticReferenceFrame, GeographicCRS, Identified, Identifier,
    Measure, Meridian, ObjectDomain, OperationMethod, OrdinalCS, Parameter, ParameterValue,
    PrimeMeridian, ProjectedCRS, SphericalCS, Transformation, UnitKind, UnitOfMeasure, VerticalCRS,
    VerticalCS, VerticalReferenceFrame,
};
use crate::projdb;

/// Whether [`AuthorityFactory::get_authority_codes`] returns deprecated rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeprecatedCodes {
    /// Include deprecated objects.
    Included,
    /// Leave deprecated objects out.
    Excluded,
}

/// Grid file name written on a transformation loaded from `proj.db`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlternativeGridNames {
    /// The grid name stored by the authority.
    Official,
    /// `grid_alternatives.proj_grid_name` when a row exists.
    Proj,
}

/// Whether a coordinate-operation search may chain through a named pivot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntermediateCrsUse {
    /// Use pivots from [`CoordinateOperationContext::set_intermediate_crs`].
    Allow,
    /// Return only a direct operation or a deriving conversion.
    Never,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GeodeticFilter {
    Any,
    GeographicOnly,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EnsembleHandling {
    Keep,
    AsDatum,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ConcatenatedSteps {
    Allow,
    Forbid,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PointMotion {
    Yes,
    No,
}

const DEGREE_SI: f64 = std::f64::consts::PI / 180.0;
const ARC_SECOND_SI: f64 = DEGREE_SI / 3600.0;
const FACTOR_EPS: f64 = 1e-10;

/// Embedded PROJ 9.9 database.
///
/// `DatabaseContext::create` checks `DATABASE.LAYOUT.VERSION` the way PROJ
/// does: major must be 1 and minor must be at least 7.
#[derive(Clone)]
pub struct DatabaseContext {
    conn: Arc<Mutex<Connection>>,
}

impl std::fmt::Debug for DatabaseContext {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("DatabaseContext { proj.db }")
    }
}

impl DatabaseContext {
    /// Open `data/proj.db` from the crate bytes.
    ///
    /// # Errors
    ///
    /// The bytes cannot be deserialized, or the layout version is not the one
    /// this factory reads.
    pub fn create() -> Result<Self, FactoryError> {
        let conn =
            projdb::open_connection().map_err(|error| FactoryError::Sql(error.to_string()))?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.check_layout()?;
        Ok(db)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    fn check_layout(&self) -> Result<(), FactoryError> {
        let rows = self.query(
            "SELECT key, value FROM metadata WHERE key IN \
             ('DATABASE.LAYOUT.VERSION.MAJOR', 'DATABASE.LAYOUT.VERSION.MINOR')",
            &[],
        )?;
        if rows.len() != 2 {
            return Err(FactoryError::Sql(
                "proj.db lacks DATABASE.LAYOUT.VERSION.MAJOR / MINOR".to_owned(),
            ));
        }
        let mut major = String::new();
        let mut minor = String::new();
        for row in &rows {
            if row[0].text() == "DATABASE.LAYOUT.VERSION.MAJOR" {
                major = row[1].text();
            } else {
                minor = row[1].text();
            }
        }
        if major != "1" || minor.parse::<i64>().unwrap_or(0) < 7 {
            return Err(FactoryError::Sql(format!(
                "proj.db layout is {major}.{minor}; 1.7 or newer is required"
            )));
        }
        Ok(())
    }

    /// CRS whose name, or alias, equals `name` ignoring case, as PROJ's
    /// exact pass of `createObjectsFromName` finds them. Deprecated rows are
    /// found only when the name ends with ` (deprecated)`.
    ///
    /// Returns `(authority, code)` pairs in PROJ's order: deprecated last,
    /// direct names before aliases, then shorter names first.
    pub(crate) fn crs_codes_named(
        &self,
        name: &str,
    ) -> Result<Vec<(String, String)>, FactoryError> {
        const TABLES: [&str; 6] = [
            "geodetic_crs",
            "projected_crs",
            "derived_projected_crs",
            "vertical_crs",
            "compound_crs",
            "engineering_crs",
        ];
        let (name, deprecated) = match name.strip_suffix(" (deprecated)") {
            Some(stripped) => (stripped, true),
            None => (name, false),
        };
        let deprecated = i64::from(deprecated);
        let mut parts = Vec::new();
        let mut params = Vec::new();
        for table in TABLES {
            parts.push(format!(
                "SELECT auth_name, code, name, deprecated, 0 AS is_alias FROM {table} \
                 WHERE name = ? COLLATE NOCASE AND deprecated = ?"
            ));
            params.push(name.to_owned());
            params.push(deprecated.to_string());
            parts.push(format!(
                "SELECT ov.auth_name, ov.code, a.alt_name, ov.deprecated, 1 AS is_alias \
                 FROM {table} ov JOIN alias_name a \
                 ON ov.auth_name = a.auth_name AND ov.code = a.code \
                 WHERE a.source != 'EPSG_OLD' AND a.table_name = '{table}' \
                 AND a.alt_name = ? COLLATE NOCASE AND ov.deprecated = ?"
            ));
            params.push(name.to_owned());
            params.push(deprecated.to_string());
        }
        let sql = format!(
            "SELECT auth_name, code FROM ({}) ORDER BY deprecated, is_alias, length(name), name LIMIT 10",
            parts.join(" UNION ALL ")
        );
        let rows = self.query(&sql, &params)?;
        Ok(rows
            .iter()
            .map(|row| (row[0].text(), row[1].text()))
            .collect())
    }

    /// The rows of `sql` as text, with `params` bound in order.
    pub(crate) fn text_rows(
        &self,
        sql: &str,
        params: &[String],
    ) -> Result<Vec<Vec<String>>, FactoryError> {
        Ok(self
            .query(sql, params)?
            .iter()
            .map(|row| row.iter().map(Cell::text).collect())
            .collect())
    }

    fn query(&self, sql: &str, params: &[String]) -> Result<Vec<Vec<Cell>>, FactoryError> {
        let guard = self.lock();
        let mut statement = guard
            .prepare(sql)
            .map_err(|error| FactoryError::Sql(error.to_string()))?;
        let mut rows = statement
            .query(rusqlite::params_from_iter(params.iter()))
            .map_err(|error| FactoryError::Sql(error.to_string()))?;
        let mut out = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|error| FactoryError::Sql(error.to_string()))?
        {
            let width = row.as_ref().column_count();
            let mut cells = Vec::with_capacity(width);
            for index in 0..width {
                cells.push(Cell::read(row, index)?);
            }
            out.push(cells);
        }
        Ok(out)
    }

    /// `IAU_2015` for authority `IAU` and version `2015`.
    pub(crate) fn versioned_authority(&self, authority: &str, version: &str) -> Option<String> {
        let rows = self
            .query(
                "SELECT versioned_auth_name FROM versioned_auth_name_mapping \
                 WHERE auth_name = ? AND version = ?",
                &[authority.to_owned(), version.to_owned()],
            )
            .ok()?;
        rows.first().map(|row| row[0].text())
    }

    /// `(auth_name, version)` for a code space such as `IAU_2015`.
    pub(crate) fn authority_and_version(&self, versioned: &str) -> Option<(String, String)> {
        let rows = self
            .query(
                "SELECT auth_name, version FROM versioned_auth_name_mapping \
                 WHERE versioned_auth_name = ?",
                &[versioned.to_owned()],
            )
            .ok()?;
        let row = rows.first()?;
        Some((row[0].text(), row[1].text()))
    }

    pub(crate) fn aliases(
        &self,
        authority: &str,
        code: &str,
        table: &str,
        source: &str,
    ) -> Result<Vec<String>, FactoryError> {
        let table = genuine_table(table);
        let rows = self.query(
            "SELECT alt_name FROM alias_name WHERE table_name = ? AND auth_name = ? \
             AND code = ? AND source = ?",
            &[
                table.to_owned(),
                authority.to_owned(),
                code.to_owned(),
                source.to_owned(),
            ],
        )?;
        Ok(rows.into_iter().map(|row| row[0].text()).collect())
    }

    /// `text_definition` for one authority object, when the column is set.
    pub(crate) fn text_definition(
        &self,
        table: &str,
        authority: &str,
        code: &str,
    ) -> Option<String> {
        let table = match table {
            "projected_crs" | "geodetic_crs" | "conversion" => table,
            _ => return None,
        };
        let sql =
            format!("SELECT text_definition FROM \"{table}\" WHERE auth_name = ? AND code = ?");
        let Ok(rows) = self.query(&sql, &[authority.to_owned(), code.to_owned()]) else {
            return None;
        };
        let text = rows.first()?.first()?.text();
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }

    pub(crate) fn alias_from_official_name(
        &self,
        official_name: &str,
        table: &str,
        source: &str,
    ) -> Result<String, FactoryError> {
        let genuine = genuine_table(table);
        let mut sql = format!("SELECT auth_name, code FROM \"{genuine}\" WHERE name = ?");
        if table == "geodetic_crs" || table == "geographic_2D_crs" {
            sql.push_str(" AND type = 'geographic 2D'");
        } else if table == "geographic_3D_crs" {
            sql.push_str(" AND type = 'geographic 3D'");
        }
        sql.push_str(" ORDER BY deprecated");
        let mut rows = self.query(&sql, &[official_name.to_owned()])?;
        if rows.is_empty() && !(official_name == "NAD83" && table == "geographic_3D_crs") {
            rows = self.query(
                "SELECT auth_name, code FROM alias_name WHERE table_name = ? AND alt_name = ? \
                 AND source IN ('EPSG', 'PROJ')",
                &[genuine.to_owned(), official_name.to_owned()],
            )?;
            if rows.len() != 1 {
                return Ok(String::new());
            }
        }
        for row in &rows {
            let aliases = self.aliases(&row[0].text(), &row[1].text(), genuine, source)?;
            if aliases.len() == 1 {
                return Ok(aliases.into_iter().next().unwrap_or_default());
            }
            if !aliases.is_empty() {
                return Ok(aliases[0].clone());
            }
        }
        Ok(String::new())
    }

    /// Official name of one object, or empty when the row is missing.
    pub(crate) fn object_name(&self, table: &str, authority: &str, code: &str) -> String {
        let table = genuine_table(table);
        let Ok(rows) = self.query(
            &format!("SELECT name FROM \"{table}\" WHERE auth_name = ? AND code = ?"),
            &[authority.to_owned(), code.to_owned()],
        ) else {
            return String::new();
        };
        rows.first().map(|row| row[0].text()).unwrap_or_default()
    }

    /// Alias names from `alias_name`.
    ///
    /// An empty authority or code resolves the object by `official_name` the way
    /// `DatabaseContext::getAliases` does: the first non-deprecated official row,
    /// or the single EPSG/PROJ alias when the official name is absent.
    pub(crate) fn get_aliases(
        &self,
        authority: &str,
        code: &str,
        official_name: &str,
        table: &str,
        source: &str,
    ) -> Vec<String> {
        let genuine = genuine_table(table);
        let (authority, code) = if authority.is_empty() || code.is_empty() {
            let mut sql = format!("SELECT auth_name, code FROM \"{genuine}\" WHERE name = ?");
            if table == "geodetic_crs" || table == "geographic_2D_crs" {
                sql.push_str(" AND type = 'geographic 2D'");
            } else if table == "geographic_3D_crs" {
                sql.push_str(" AND type = 'geographic 3D'");
            }
            sql.push_str(" ORDER BY deprecated");
            let Ok(mut rows) = self.query(&sql, &[official_name.to_owned()]) else {
                return Vec::new();
            };
            if rows.is_empty() {
                let Ok(alias_rows) = self.query(
                    "SELECT auth_name, code FROM alias_name WHERE table_name = ? AND alt_name = ? \
                     AND source IN ('EPSG', 'PROJ')",
                    &[genuine.to_owned(), official_name.to_owned()],
                ) else {
                    return Vec::new();
                };
                if alias_rows.len() != 1 {
                    return Vec::new();
                }
                rows = alias_rows;
            }
            let Some(row) = rows.first() else {
                return Vec::new();
            };
            (row[0].text(), row[1].text())
        } else {
            (authority.to_owned(), code.to_owned())
        };
        let mut sql =
            "SELECT alt_name FROM alias_name WHERE table_name = ? AND auth_name = ? AND code = ?"
                .to_owned();
        let mut params = vec![genuine.to_owned(), authority, code];
        if source == "not EPSG_OLD" {
            sql.push_str(" AND source != 'EPSG_OLD'");
        } else if !source.is_empty() {
            sql.push_str(" AND source = ?");
            params.push(source.to_owned());
        }
        self.query(&sql, &params)
            .map(|rows| rows.into_iter().map(|row| row[0].text()).collect())
            .unwrap_or_default()
    }

    /// True when `name` is the official name of exactly one EPSG geodetic datum.
    ///
    /// WKT1 GDAL then morphs that name (spaces and parentheses) the way
    /// `importFromEPSG` did, even when the WKT datum carries no identifier.
    pub(crate) fn single_epsg_geodetic_datum(&self, name: &str) -> bool {
        let Ok(rows) = self.query(
            "SELECT auth_name FROM geodetic_datum WHERE name = ? COLLATE NOCASE",
            &[name.to_owned()],
        ) else {
            return false;
        };
        rows.len() == 1 && rows[0][0].text() == "EPSG"
    }

    /// True when `name` is the official name of exactly one ESRI object in `table`.
    ///
    /// WKT1 Esri then keeps that name, including a `_(Sphere)` suffix, instead of
    /// morphing it and prefixing `GCS_` or `D_`.
    pub(crate) fn has_single_esri_name(&self, table: &str, name: &str) -> bool {
        let table = match table {
            "geographic_2D_crs" | "geographic_3D_crs" | "geodetic_crs" => "geodetic_crs",
            "geodetic_datum" | "ellipsoid" | "prime_meridian" | "projected_crs"
            | "vertical_datum" => table,
            _ => return false,
        };
        let sql = format!("SELECT COUNT(*) FROM \"{table}\" WHERE name = ? AND auth_name = 'ESRI'");
        let Ok(rows) = self.query(&sql, &[name.to_owned()]) else {
            return false;
        };
        rows.first().is_some_and(|row| row[0].text() == "1")
    }

    /// Official name, authority, and code for an alias row.
    ///
    /// This is `AuthorityFactory::getOfficialNameFromAlias` with exact spelling.
    /// ESRI also matches `ESRI_OLD`.
    pub(crate) fn official_from_alias(
        &self,
        alias: &str,
        table: &str,
        source: &str,
    ) -> Option<(String, String, String)> {
        let mut sql =
            String::from("SELECT table_name, auth_name, code FROM alias_name WHERE alt_name = ?");
        let mut params = vec![alias.to_owned()];
        if !table.is_empty() {
            sql.push_str(" AND table_name = ?");
            params.push(table.to_owned());
        }
        if source == "ESRI" {
            sql.push_str(" AND source IN ('ESRI', 'ESRI_OLD')");
        } else if !source.is_empty() {
            sql.push_str(" AND source = ?");
            params.push(source.to_owned());
        }
        let Ok(rows) = self.query(&sql, &params) else {
            return None;
        };
        if rows.is_empty() {
            return None;
        }
        let mut union = String::new();
        let mut union_params = Vec::new();
        for (index, row) in rows.iter().enumerate() {
            if index > 0 {
                union.push_str(" UNION ALL ");
            }
            let table_name = row[0].text().replace('"', "\"\"");
            union.push_str(&format!(
                "SELECT name, ? AS table_name, auth_name, code, deprecated FROM \"{table_name}\" \
                 WHERE auth_name = ? AND code = ?"
            ));
            union_params.push(row[0].text());
            union_params.push(row[1].text());
            union_params.push(row[2].text());
        }
        let sql = format!(
            "SELECT name, table_name, auth_name, code FROM ({union}) x ORDER BY deprecated LIMIT 1"
        );
        let Ok(found) = self.query(&sql, &union_params) else {
            return None;
        };
        let row = found.first()?;
        Some((row[0].text(), row[2].text(), row[3].text()))
    }

    /// `(authority, code)` of a datum ensemble with this exact name.
    pub(crate) fn ensemble_by_name(&self, name: &str, table: &str) -> Option<(String, String)> {
        let sql = format!(
            "SELECT auth_name, code FROM \"{table}\" WHERE name = ? AND ensemble_accuracy IS NOT NULL \
             ORDER BY deprecated LIMIT 1"
        );
        let Ok(rows) = self.query(&sql, &[name.to_owned()]) else {
            return None;
        };
        rows.first().map(|row| (row[0].text(), row[1].text()))
    }

    /// True when `name` is a row in `geodetic_datum`.
    pub(crate) fn known_geodetic_datum_name(&self, name: &str) -> bool {
        let Ok(rows) = self.query(
            "SELECT 1 FROM geodetic_datum WHERE name = ? LIMIT 1",
            &[name.to_owned()],
        ) else {
            return false;
        };
        !rows.is_empty()
    }

    /// Authority names registered in `authority_list`, in sorted order.
    ///
    /// # Errors
    ///
    /// The database rejects the statement.
    pub fn authorities(&self) -> Result<Vec<String>, FactoryError> {
        let rows = self.query(
            "SELECT auth_name FROM authority_list ORDER BY auth_name",
            &[],
        )?;
        Ok(rows.into_iter().map(|row| row[0].text()).collect())
    }

    /// Metadata value for `key`, or `None` when the key is absent.
    ///
    /// # Errors
    ///
    /// The database rejects the statement.
    pub fn metadata(&self, key: &str) -> Result<Option<String>, FactoryError> {
        let rows = self.query(
            "SELECT value FROM metadata WHERE key = ?",
            &[key.to_owned()],
        )?;
        Ok(rows.into_iter().next().map(|row| row[0].text()))
    }

    /// The `CREATE` statements of the tables, then the views, then the
    /// triggers, each ending in `;`, followed by the layout version rows.
    ///
    /// # Errors
    ///
    /// The database rejects the statement.
    pub fn database_structure(&self) -> Result<Vec<String>, FactoryError> {
        let mut statements = Vec::new();
        for kind in [
            "'table' AND name NOT LIKE 'sqlite_stat%'",
            "'view'",
            "'trigger'",
        ] {
            let rows = self.query(
                &format!("SELECT sql||';' FROM sqlite_master WHERE type = {kind} ORDER BY rowid"),
                &[],
            )?;
            statements.extend(rows.into_iter().map(|row| row[0].text()));
        }
        let major = self.layout_version("DATABASE.LAYOUT.VERSION.MAJOR")?;
        if major > 0 {
            let minor = self.layout_version("DATABASE.LAYOUT.VERSION.MINOR")?;
            statements.push(format!(
                "INSERT INTO metadata VALUES('DATABASE.LAYOUT.VERSION.MAJOR',{major});"
            ));
            statements.push(format!(
                "INSERT INTO metadata VALUES('DATABASE.LAYOUT.VERSION.MINOR',{minor});"
            ));
        }
        Ok(statements)
    }

    fn layout_version(&self, key: &str) -> Result<i64, FactoryError> {
        Ok(self
            .metadata(key)?
            .and_then(|value| value.parse().ok())
            .unwrap_or(0))
    }

    /// Geodetic datum whose name or alias matches `name` ignoring case, spaces, and underscores.
    pub(crate) fn geodetic_datum_equivalent(&self, name: &str) -> Option<(String, String, String)> {
        let key = canonical_datum_key(name);
        if key.is_empty() {
            return None;
        }
        let by_name = self.query(
            "SELECT name, auth_name, code FROM geodetic_datum \
             WHERE REPLACE(REPLACE(LOWER(name), '_', ''), ' ', '') = ? \
             ORDER BY deprecated, CASE WHEN auth_name = 'EPSG' THEN 0 ELSE 1 END \
             LIMIT 1",
            std::slice::from_ref(&key),
        );
        if let Ok(rows) = by_name {
            if let Some(row) = rows.first() {
                return Some((row[0].text(), row[1].text(), row[2].text()));
            }
        }
        let by_alias = self.query(
            "SELECT g.name, g.auth_name, g.code FROM alias_name a \
             JOIN geodetic_datum g ON g.auth_name = a.auth_name AND g.code = a.code \
             WHERE a.table_name = 'geodetic_datum' \
             AND REPLACE(REPLACE(LOWER(a.alt_name), '_', ''), ' ', '') = ? \
             ORDER BY g.deprecated LIMIT 1",
            std::slice::from_ref(&key),
        );
        let Ok(rows) = by_alias else {
            return None;
        };
        rows.first()
            .map(|row| (row[0].text(), row[1].text(), row[2].text()))
    }

    /// Celestial bodies of ellipsoids whose name is `name` or starts with `name`.
    ///
    /// `Ellipsoid::guessBodyName` resolves a short name such as `Ananke` to
    /// `Ananke (2015) - Sphere` before comparing the semi-major axis.
    pub(crate) fn ellipsoid_bodies_named(&self, name: &str) -> Vec<(String, f64)> {
        let like_space = format!("{name} %");
        let like_paren = format!("{name} (%");
        let Ok(rows) = self.query(
            "SELECT celestial_body.name, ellipsoid.semi_major_axis FROM ellipsoid \
             JOIN celestial_body ON ellipsoid.celestial_body_auth_name = celestial_body.auth_name \
             AND ellipsoid.celestial_body_code = celestial_body.code \
             WHERE ellipsoid.name = ? OR ellipsoid.name LIKE ? OR ellipsoid.name LIKE ? \
             ORDER BY CASE WHEN ellipsoid.name = ? THEN 0 ELSE 1 END, ellipsoid.deprecated",
            &[name.to_owned(), like_space, like_paren, name.to_owned()],
        ) else {
            return Vec::new();
        };
        rows.into_iter()
            .filter_map(|row| row[1].f64().ok().map(|axis| (row[0].text(), axis)))
            .collect()
    }
}

fn canonical_datum_key(name: &str) -> String {
    name.chars()
        .filter(|ch| !matches!(ch, ' ' | '_' | '-'))
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

fn genuine_table(table: &str) -> &str {
    if table == "geographic_2D_crs" || table == "geographic_3D_crs" {
        "geodetic_crs"
    } else {
        table
    }
}

/// How [`AuthorityFactory::create_geodetic_crs_by_name`] compares a name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameMatch {
    /// Official name or alias, case-insensitive.
    Exact,
    /// The searched text is a substring of the name or of its canonical form.
    Approximate,
}

struct NameRow {
    table: String,
    authority: String,
    code: String,
    matched_name: String,
    deprecated: bool,
}

struct BuiltObject<T> {
    matched_name: String,
    object: T,
}

trait NamedAuthorityObject: Sized {
    fn name(&self) -> &str;
    fn identifiers(&self) -> &[Identifier];
}

impl NamedAuthorityObject for GeodeticCRS {
    fn name(&self) -> &str {
        GeodeticCRS::name(self)
    }

    fn identifiers(&self) -> &[Identifier] {
        GeodeticCRS::identifiers(self)
    }
}

impl NamedAuthorityObject for GeodeticReferenceFrame {
    fn name(&self) -> &str {
        GeodeticReferenceFrame::name(self)
    }

    fn identifiers(&self) -> &[Identifier] {
        GeodeticReferenceFrame::identifiers(self)
    }
}

impl AuthorityObject {
    /// The object's name, or `""` for a kind that has none in this port.
    #[must_use]
    pub fn name(&self) -> &str {
        NamedAuthorityObject::name(self)
    }

    /// Whether the object is deprecated. Every other kind is not.
    #[must_use]
    pub fn is_deprecated(&self) -> bool {
        match self {
            Self::Crs(crs) => crs.is_deprecated(),
            Self::Ellipsoid(object) => object.identified.deprecated,
            Self::Datum(Datum::Geodetic(object)) => object.identified.deprecated,
            Self::Datum(Datum::Vertical(object)) => object.identified.deprecated,
            Self::Datum(Datum::Engineering(object)) => object.identified.deprecated,
            Self::DatumEnsemble(object) => object.identified.deprecated,
            Self::Extent(_) | Self::Unit(_) | Self::PrimeMeridian(_) | Self::Operation(_) => false,
        }
    }
}

impl NamedAuthorityObject for AuthorityObject {
    fn name(&self) -> &str {
        match self {
            Self::Ellipsoid(object) => object.name(),
            Self::Datum(Datum::Geodetic(object)) => object.name(),
            Self::Datum(Datum::Vertical(object)) => object.name(),
            Self::Datum(Datum::Engineering(object)) => object.name(),
            Self::DatumEnsemble(object) => object.name(),
            Self::Crs(object) => object.name(),
            Self::Extent(_) | Self::Unit(_) | Self::PrimeMeridian(_) | Self::Operation(_) => "",
        }
    }

    fn identifiers(&self) -> &[Identifier] {
        match self {
            Self::Ellipsoid(object) => object.identifiers(),
            Self::Datum(Datum::Geodetic(object)) => object.identifiers(),
            Self::Datum(Datum::Vertical(object)) => object.identifiers(),
            Self::Datum(Datum::Engineering(object)) => object.identifiers(),
            Self::DatumEnsemble(object) => object.identifiers(),
            Self::Crs(object) => object.identifiers(),
            Self::Extent(_) | Self::Unit(_) | Self::PrimeMeridian(_) | Self::Operation(_) => &[],
        }
    }
}

/// The tables a name search reads, as `table` or `table:filter`. `filter` is
/// `ensemble`, `frame`, or a CRS type, as `createObjectsFromNameEx` picks them.
fn name_keys(types: &[ObjectType], name: &str) -> Vec<&'static str> {
    let hide_vertical = name.starts_with("D_");
    if types.is_empty() {
        let mut keys = vec![
            "ellipsoid",
            "geodetic_datum",
            "engineering_datum",
            "geodetic_crs",
            "projected_crs",
            "derived_projected_crs",
            "vertical_crs",
            "compound_crs",
            "engineering_crs",
        ];
        if !hide_vertical {
            keys.push("vertical_datum");
        }
        return keys;
    }
    let mut keys: Vec<&'static str> = Vec::new();
    let mut want = |kind: ObjectType, found: &[&'static str]| {
        if types.contains(&kind) {
            for &key in found {
                if !keys.contains(&key) {
                    keys.push(key);
                }
            }
        }
    };
    want(ObjectType::Ellipsoid, &["ellipsoid"]);
    want(
        ObjectType::Datum,
        if hide_vertical {
            &["geodetic_datum"]
        } else {
            &["geodetic_datum", "vertical_datum", "engineering_datum"]
        },
    );
    want(ObjectType::GeodeticReferenceFrame, &["geodetic_datum"]);
    want(
        ObjectType::DynamicGeodeticReferenceFrame,
        &["geodetic_datum:frame"],
    );
    want(ObjectType::VerticalReferenceFrame, &["vertical_datum"]);
    want(
        ObjectType::DynamicVerticalReferenceFrame,
        &["vertical_datum:frame"],
    );
    want(ObjectType::EngineeringDatum, &["engineering_datum"]);
    want(
        ObjectType::DatumEnsemble,
        &["geodetic_datum:ensemble", "vertical_datum:ensemble"],
    );
    want(
        ObjectType::Crs,
        &[
            "geodetic_crs",
            "projected_crs",
            "vertical_crs",
            "compound_crs",
            "engineering_crs",
            "derived_projected_crs",
        ],
    );
    want(ObjectType::GeodeticCrs, &["geodetic_crs"]);
    want(ObjectType::GeocentricCrs, &["geodetic_crs:geocentric"]);
    want(
        ObjectType::GeographicCrs,
        &["geodetic_crs:geographic 2D", "geodetic_crs:geographic 3D"],
    );
    want(ObjectType::Geographic2dCrs, &["geodetic_crs:geographic 2D"]);
    want(ObjectType::Geographic3dCrs, &["geodetic_crs:geographic 3D"]);
    want(ObjectType::ProjectedCrs, &["projected_crs"]);
    want(ObjectType::DerivedProjectedCrs, &["derived_projected_crs"]);
    want(ObjectType::VerticalCrs, &["vertical_crs"]);
    want(ObjectType::CompoundCrs, &["compound_crs"]);
    want(ObjectType::EngineeringCrs, &["engineering_crs"]);
    keys
}

fn name_filter_sql(prefix: &str, filter: &str) -> String {
    match filter {
        "" => String::new(),
        "ensemble" => format!("AND {prefix}ensemble_accuracy IS NOT NULL "),
        "frame" => format!("AND {prefix}frame_reference_epoch IS NOT NULL "),
        crs_type => format!("AND {prefix}type = '{crs_type}' "),
    }
}

fn load_named(
    factory: &AuthorityFactory,
    row: &NameRow,
    types: &[ObjectType],
) -> Result<AuthorityObject, FactoryError> {
    let (table, filter) = row
        .table
        .split_once(':')
        .unwrap_or((row.table.as_str(), ""));
    let code = row.code.as_str();
    let ensemble_allowed = types.is_empty() || types.contains(&ObjectType::DatumEnsemble);
    Ok(match (table, filter) {
        ("ellipsoid", _) => AuthorityObject::Ellipsoid(factory.create_ellipsoid(code)?),
        ("geodetic_datum", "ensemble") | ("vertical_datum", "ensemble") => {
            AuthorityObject::DatumEnsemble(factory.create_datum_ensemble(code)?)
        }
        ("geodetic_datum", "") if ensemble_allowed => match factory.create_datum_ensemble(code) {
            Ok(ensemble) => AuthorityObject::DatumEnsemble(ensemble),
            Err(_) => AuthorityObject::Datum(Datum::Geodetic(factory.create_geodetic_datum(code)?)),
        },
        ("geodetic_datum", _) => {
            AuthorityObject::Datum(Datum::Geodetic(factory.create_geodetic_datum(code)?))
        }
        ("vertical_datum", _) => {
            AuthorityObject::Datum(Datum::Vertical(factory.create_vertical_datum(code)?))
        }
        ("engineering_datum", _) => {
            AuthorityObject::Datum(Datum::Engineering(factory.create_engineering_datum(code)?))
        }
        (
            "geodetic_crs"
            | "projected_crs"
            | "vertical_crs"
            | "compound_crs"
            | "engineering_crs"
            | "derived_projected_crs",
            _,
        ) => AuthorityObject::Crs(factory.create_coordinate_reference_system(code)?),
        _ => return Err(FactoryError::missing("object", &row.authority, code)),
    })
}

fn canonical_object_name(name: &str) -> String {
    name.chars()
        .filter(|ch| !matches!(ch, ' ' | '_' | '-' | '/' | '(' | ')' | '.' | '&' | ','))
        .flat_map(char::to_lowercase)
        .collect()
}

fn equivalent_object_name(left: &str, right: &str) -> bool {
    canonical_object_name(left) == canonical_object_name(right)
}

fn approximate_name(candidate: &str, searched: &str, canonical_searched: &str) -> bool {
    candidate
        .to_ascii_lowercase()
        .contains(&searched.to_ascii_lowercase())
        || canonical_object_name(candidate).contains(canonical_searched)
}

fn semi_minor_si(ellipsoid: &Ellipsoid) -> f64 {
    if let Some(minor) = ellipsoid.semi_minor_axis() {
        return minor.si_value();
    }
    let major = ellipsoid.semi_major_axis().si_value();
    let flattening = ellipsoid.inverse_flattening();
    if flattening == 0.0 {
        major
    } else {
        major * (1.0 - 1.0 / flattening)
    }
}

#[derive(Clone, Debug)]
enum Cell {
    Empty,
    Text(String),
    Real(f64),
}

impl Cell {
    fn read(row: &rusqlite::Row<'_>, index: usize) -> Result<Self, FactoryError> {
        let value = row
            .get_ref(index)
            .map_err(|error| FactoryError::Sql(error.to_string()))?;
        Ok(match value {
            ValueRef::Null => Self::Empty,
            ValueRef::Integer(value) => Self::Text(value.to_string()),
            ValueRef::Real(value) => Self::Real(value),
            ValueRef::Text(value) => Self::Text(String::from_utf8_lossy(value).into_owned()),
            ValueRef::Blob(_) => Self::Empty,
        })
    }

    fn text(&self) -> String {
        match self {
            Self::Empty => String::new(),
            Self::Text(value) => value.clone(),
            Self::Real(value) => format!("{value}"),
        }
    }

    fn empty(&self) -> bool {
        matches!(self, Self::Empty) || matches!(self, Self::Text(value) if value.is_empty())
    }

    fn f64(&self) -> Result<f64, FactoryError> {
        match self {
            Self::Real(value) => Ok(*value),
            Self::Text(value) => value
                .parse::<f64>()
                .map_err(|_| FactoryError::Invalid(format!("expected a number, got {value}"))),
            Self::Empty => Err(FactoryError::Invalid("expected a number".to_owned())),
        }
    }

    fn flag(&self) -> bool {
        self.text() == "1"
    }
}

/// Builds ISO 19111 objects from one authority in [`DatabaseContext`].
#[derive(Clone, Debug)]
pub struct AuthorityFactory {
    db: DatabaseContext,
    authority: String,
}

impl AuthorityFactory {
    /// Factory for `authority`. An empty authority does not filter operation searches.
    #[must_use]
    pub fn create(db: &DatabaseContext, authority: impl Into<String>) -> Self {
        Self {
            db: db.clone(),
            authority: authority.into(),
        }
    }

    /// Authority name passed to [`Self::create`].
    #[must_use]
    pub fn authority(&self) -> &str {
        &self.authority
    }

    /// Database this factory reads.
    #[must_use]
    pub fn database_context(&self) -> &DatabaseContext {
        &self.db
    }

    fn child(&self, authority: &str) -> Self {
        if authority.is_empty() || authority == self.authority {
            self.clone()
        } else {
            Self {
                db: self.db.clone(),
                authority: authority.to_owned(),
            }
        }
    }

    fn one(&self, sql: &str, code: &str) -> Result<Vec<Cell>, FactoryError> {
        let rows = self
            .db
            .query(sql, &[self.authority.clone(), code.to_owned()])?;
        rows.into_iter()
            .next()
            .ok_or_else(|| FactoryError::missing("object", &self.authority, code))
    }

    /// Any single object whose code is unambiguous in `object_view`.
    ///
    /// # Errors
    ///
    /// The code is missing, or more than one table has that code.
    pub fn create_object(&self, code: &str) -> Result<AuthorityObject, FactoryError> {
        let rows = self.db.query(
            "SELECT table_name, type FROM object_view WHERE auth_name = ? AND code = ?",
            &[self.authority.clone(), code.to_owned()],
        )?;
        if rows.is_empty() {
            return Err(FactoryError::missing("object", &self.authority, code));
        }
        if rows.len() != 1 {
            return Err(FactoryError::wrap(
                "object",
                &self.authority,
                code,
                "more than one object matches this code",
            ));
        }
        let table = rows[0][0].text();
        let kind = rows[0][1].text();
        let object = match table.as_str() {
            "extent" => AuthorityObject::Extent(self.create_extent(code)?),
            "unit_of_measure" => AuthorityObject::Unit(self.create_unit_of_measure(code)?),
            "prime_meridian" => AuthorityObject::PrimeMeridian(self.create_prime_meridian(code)?),
            "ellipsoid" => AuthorityObject::Ellipsoid(self.create_ellipsoid(code)?),
            "geodetic_datum" if kind == "ensemble" => {
                AuthorityObject::DatumEnsemble(self.create_datum_ensemble(code)?)
            }
            "geodetic_datum" => {
                AuthorityObject::Datum(Datum::Geodetic(self.create_geodetic_datum(code)?))
            }
            "vertical_datum" if kind == "ensemble" => {
                AuthorityObject::DatumEnsemble(self.create_datum_ensemble(code)?)
            }
            "vertical_datum" => {
                AuthorityObject::Datum(Datum::Vertical(self.create_vertical_datum(code)?))
            }
            "engineering_datum" => {
                AuthorityObject::Datum(Datum::Engineering(self.create_engineering_datum(code)?))
            }
            "geodetic_crs" => AuthorityObject::Crs(Crs::from(self.create_geodetic_crs(code)?)),
            "vertical_crs" => AuthorityObject::Crs(Crs::Vertical(self.create_vertical_crs(code)?)),
            "projected_crs" => {
                AuthorityObject::Crs(Crs::Projected(self.create_projected_crs(code)?))
            }
            "compound_crs" => AuthorityObject::Crs(Crs::Compound(self.create_compound_crs(code)?)),
            "engineering_crs" => {
                AuthorityObject::Crs(Crs::Engineering(self.create_engineering_crs(code)?))
            }
            "derived_projected_crs" => AuthorityObject::Crs(Crs::DerivedProjected(
                self.create_derived_projected_crs(code)?,
            )),
            "conversion" => AuthorityObject::Operation(CoordinateOperation::Conversion(
                self.create_conversion(code)?,
            )),
            "helmert_transformation"
            | "grid_transformation"
            | "other_transformation"
            | "concatenated_operation" => AuthorityObject::Operation(
                self.create_coordinate_operation(code, AlternativeGridNames::Proj)?,
            ),
            other => {
                return Err(FactoryError::wrap(
                    "object",
                    &self.authority,
                    code,
                    format!("unhandled table {other}"),
                ));
            }
        };
        Ok(object)
    }

    /// Linear, angular, scale, or time unit.
    ///
    /// # Errors
    ///
    /// No unit has this code. Codes 9107 and 9108 use the degree factor, as in PROJ.
    pub fn create_unit_of_measure(&self, code: &str) -> Result<UnitOfMeasure, FactoryError> {
        let row = self.db.query(
            "SELECT name, conv_factor, type FROM unit_of_measure WHERE auth_name = ? AND code = ?",
            &[self.authority.clone(), code.to_owned()],
        )?;
        let Some(row) = row.into_iter().next() else {
            return Err(FactoryError::missing(
                "unit of measure",
                &self.authority,
                code,
            ));
        };
        let mut name = row[0].text();
        if name == "degree (supplier to define representation)" {
            name = "degree".to_owned();
        }
        let mut factor = if code == "9107" || code == "9108" {
            DEGREE_SI
        } else {
            row[1].f64().map_err(|error| {
                FactoryError::wrap("unit of measure", &self.authority, code, error.to_string())
            })?
        };
        if (factor - DEGREE_SI).abs() < FACTOR_EPS * DEGREE_SI {
            factor = DEGREE_SI;
        } else if (factor - ARC_SECOND_SI).abs() < FACTOR_EPS * ARC_SECOND_SI {
            factor = ARC_SECOND_SI;
        }
        let kind = match row[2].text().as_str() {
            "length" => UnitKind::Linear,
            "angle" => UnitKind::Angular,
            "scale" => UnitKind::Scale,
            "time" => UnitKind::Time,
            _ => UnitKind::Unknown,
        };
        Ok(UnitOfMeasure {
            name,
            factor,
            kind,
            code_space: self.authority.clone(),
            code: code.to_owned(),
        })
    }

    /// Prime meridian.
    ///
    /// # Errors
    ///
    /// The code is missing, or the longitude unit cannot be read.
    /// Sexagesimal DMS (EPSG:9110) is normalized to degrees, as in PROJ.
    pub fn create_prime_meridian(&self, code: &str) -> Result<PrimeMeridian, FactoryError> {
        let row = self
            .one(
                "SELECT name, longitude, uom_auth_name, uom_code, deprecated FROM prime_meridian \
             WHERE auth_name = ? AND code = ?",
                code,
            )
            .map_err(|_| FactoryError::missing("prime meridian", &self.authority, code))?;
        let (value, unit_code) = normalize_measure(&row[3].text(), &row[1].text())?;
        let unit = self
            .child(&row[2].text())
            .create_unit_of_measure(&unit_code)
            .map_err(|error| {
                FactoryError::wrap("prime meridian", &self.authority, code, error.to_string())
            })?;
        Ok(PrimeMeridian {
            identified: self.properties(code, &row[0].text(), &row[4], "", "prime_meridian")?,
            longitude: Measure { value, unit },
        })
    }

    /// Celestial body whose semi-major axis matches `semi_major_axis` within `tolerance`.
    ///
    /// # Errors
    ///
    /// No body matches, or more than one body matches.
    pub fn identify_body_from_semi_major_axis(
        &self,
        semi_major_axis: f64,
        tolerance: f64,
    ) -> Result<String, FactoryError> {
        let rows = self.db.query(
            "SELECT DISTINCT name, (ABS(semi_major_axis - ?) / semi_major_axis) AS rel_error \
             FROM celestial_body WHERE rel_error <= ? ORDER BY rel_error, name",
            &[semi_major_axis.to_string(), tolerance.to_string()],
        )?;
        if rows.is_empty() {
            return Err(FactoryError::Invalid(
                "no celestial body matches".to_owned(),
            ));
        }
        if rows.len() > 1 {
            let first = rows[0][1].text();
            let second = rows[1][1].text();
            if !(first == "0" && second != "0")
                && rows.iter().any(|row| row[0].text() != rows[0][0].text())
            {
                return Err(FactoryError::Invalid(
                    "more than one celestial body matches".to_owned(),
                ));
            }
        }
        Ok(rows[0][0].text())
    }

    /// Ellipsoid.
    ///
    /// # Errors
    ///
    /// The code is missing.
    pub fn create_ellipsoid(&self, code: &str) -> Result<Ellipsoid, FactoryError> {
        let row = self
            .one(
                "SELECT ellipsoid.name, ellipsoid.semi_major_axis, ellipsoid.uom_auth_name, \
             ellipsoid.uom_code, ellipsoid.inv_flattening, ellipsoid.semi_minor_axis, \
             celestial_body.name, ellipsoid.deprecated FROM ellipsoid \
             JOIN celestial_body ON ellipsoid.celestial_body_auth_name = celestial_body.auth_name \
             AND ellipsoid.celestial_body_code = celestial_body.code \
             WHERE ellipsoid.auth_name = ? AND ellipsoid.code = ?",
                code,
            )
            .map_err(|_| FactoryError::missing("ellipsoid", &self.authority, code))?;
        let major = row[1].f64().map_err(|error| {
            FactoryError::wrap("ellipsoid", &self.authority, code, error.to_string())
        })?;
        let unit = self
            .child(&row[2].text())
            .create_unit_of_measure(&row[3].text())
            .map_err(|error| {
                FactoryError::wrap("ellipsoid", &self.authority, code, error.to_string())
            })?;
        let inverse = if row[4].empty() {
            None
        } else {
            Some(row[4].f64().map_err(|error| {
                FactoryError::wrap("ellipsoid", &self.authority, code, error.to_string())
            })?)
        };
        let minor = if row[5].empty() {
            None
        } else {
            let value = row[5].f64().map_err(|error| {
                FactoryError::wrap("ellipsoid", &self.authority, code, error.to_string())
            })?;
            Some(Measure {
                value,
                unit: unit.clone(),
            })
        };
        Ok(Ellipsoid {
            identified: self.properties(code, &row[0].text(), &row[7], "", "ellipsoid")?,
            semi_major: Measure { value: major, unit },
            inverse_flattening: inverse,
            semi_minor: minor,
            body: row[6].text(),
        })
    }

    /// Extent description and bounding box.
    ///
    /// # Errors
    ///
    /// The code is missing.
    pub fn create_extent(&self, code: &str) -> Result<ObjectDomain, FactoryError> {
        let row = self
            .one(
                "SELECT description, south_lat, north_lat, west_lon, east_lon FROM extent \
             WHERE auth_name = ? AND code = ?",
                code,
            )
            .map_err(|_| FactoryError::missing("extent", &self.authority, code))?;
        domain_from_extent(&row[0].text(), &row[1], &row[2], &row[3], &row[4], None)
    }

    /// Geodetic datum. An ensemble code is returned as one datum with the non-ensemble name.
    ///
    /// # Errors
    ///
    /// The code is missing.
    pub fn create_geodetic_datum(
        &self,
        code: &str,
    ) -> Result<GeodeticReferenceFrame, FactoryError> {
        match self.geodetic_datum(code, EnsembleHandling::AsDatum)? {
            DatumOrEnsemble::Datum(datum) => Ok(datum),
            DatumOrEnsemble::Ensemble(_) => Err(FactoryError::wrap(
                "geodetic datum",
                &self.authority,
                code,
                "ensemble was not turned into a datum",
            )),
        }
    }

    /// Vertical datum. An ensemble code is returned as one vertical datum.
    ///
    /// # Errors
    ///
    /// The code is missing.
    pub fn create_vertical_datum(
        &self,
        code: &str,
    ) -> Result<VerticalReferenceFrame, FactoryError> {
        match self.vertical_datum(code, EnsembleHandling::AsDatum)? {
            VerticalOrEnsemble::Datum(datum) => Ok(datum),
            VerticalOrEnsemble::Ensemble(_) => Err(FactoryError::wrap(
                "vertical datum",
                &self.authority,
                code,
                "ensemble was not turned into a datum",
            )),
        }
    }

    /// Engineering datum.
    ///
    /// # Errors
    ///
    /// The code is missing.
    pub fn create_engineering_datum(&self, code: &str) -> Result<EngineeringDatum, FactoryError> {
        let row = self
            .one(
                "SELECT name, publication_date, anchor, anchor_epoch, deprecated \
             FROM engineering_datum WHERE auth_name = ? AND code = ?",
                code,
            )
            .map_err(|_| FactoryError::missing("engineering datum", &self.authority, code))?;
        let mut identified =
            self.properties(code, &row[0].text(), &row[4], "", "engineering_datum")?;
        if !row[1].empty() {
            identified.publication_date = Some(row[1].text());
        }
        if !row[2].empty() {
            identified.anchor = Some(row[2].text());
        }
        if !row[3].empty() {
            identified.anchor_epoch = Some(row[3].f64()?);
        }
        Ok(EngineeringDatum { identified })
    }

    /// Geodetic, vertical, or engineering datum.
    ///
    /// # Errors
    ///
    /// The code is missing from all three datum tables.
    pub fn create_datum(&self, code: &str) -> Result<Datum, FactoryError> {
        let rows = self.db.query(
            "SELECT 'geodetic_datum' FROM geodetic_datum WHERE auth_name = ? AND code = ? \
             UNION ALL SELECT 'vertical_datum' FROM vertical_datum WHERE auth_name = ? AND code = ? \
             UNION ALL SELECT 'engineering_datum' FROM engineering_datum \
             WHERE auth_name = ? AND code = ?",
            &[
                self.authority.clone(),
                code.to_owned(),
                self.authority.clone(),
                code.to_owned(),
                self.authority.clone(),
                code.to_owned(),
            ],
        )?;
        let Some(row) = rows.first() else {
            return Err(FactoryError::missing("datum", &self.authority, code));
        };
        match row[0].text().as_str() {
            "geodetic_datum" => Ok(Datum::Geodetic(self.create_geodetic_datum(code)?)),
            "vertical_datum" => Ok(Datum::Vertical(self.create_vertical_datum(code)?)),
            _ => Ok(Datum::Engineering(self.create_engineering_datum(code)?)),
        }
    }

    /// Geodetic or vertical datum ensemble.
    ///
    /// # Errors
    ///
    /// The code is not an ensemble.
    pub fn create_datum_ensemble(&self, code: &str) -> Result<DatumEnsemble, FactoryError> {
        let rows = self.db.query(
            "SELECT 'geodetic_datum' FROM geodetic_datum WHERE auth_name = ? AND code = ? \
             AND ensemble_accuracy IS NOT NULL \
             UNION ALL SELECT 'vertical_datum' FROM vertical_datum \
             WHERE auth_name = ? AND code = ? AND ensemble_accuracy IS NOT NULL",
            &[
                self.authority.clone(),
                code.to_owned(),
                self.authority.clone(),
                code.to_owned(),
            ],
        )?;
        let Some(row) = rows.first() else {
            return Err(FactoryError::missing(
                "datum ensemble",
                &self.authority,
                code,
            ));
        };
        match row[0].text().as_str() {
            "geodetic_datum" => match self.geodetic_datum(code, EnsembleHandling::Keep)? {
                DatumOrEnsemble::Ensemble(ensemble) => Ok(ensemble),
                DatumOrEnsemble::Datum(_) => Err(FactoryError::missing(
                    "datum ensemble",
                    &self.authority,
                    code,
                )),
            },
            _ => match self.vertical_datum(code, EnsembleHandling::Keep)? {
                VerticalOrEnsemble::Ensemble(ensemble) => Ok(ensemble),
                VerticalOrEnsemble::Datum(_) => Err(FactoryError::missing(
                    "datum ensemble",
                    &self.authority,
                    code,
                )),
            },
        }
    }

    /// Datum ensemble stored in `table` (`geodetic_datum` or `vertical_datum`).
    ///
    /// # Errors
    ///
    /// The code is not an ensemble in that table.
    pub fn create_datum_ensemble_in(
        &self,
        code: &str,
        table: &str,
    ) -> Result<DatumEnsemble, FactoryError> {
        match table {
            "geodetic_datum" => match self.geodetic_datum(code, EnsembleHandling::Keep)? {
                DatumOrEnsemble::Ensemble(ensemble) => Ok(ensemble),
                DatumOrEnsemble::Datum(_) => Err(FactoryError::missing(
                    "datum ensemble",
                    &self.authority,
                    code,
                )),
            },
            "vertical_datum" => match self.vertical_datum(code, EnsembleHandling::Keep)? {
                VerticalOrEnsemble::Ensemble(ensemble) => Ok(ensemble),
                VerticalOrEnsemble::Datum(_) => Err(FactoryError::missing(
                    "datum ensemble",
                    &self.authority,
                    code,
                )),
            },
            _ => Err(FactoryError::missing(
                "datum ensemble",
                &self.authority,
                code,
            )),
        }
    }

    /// Coordinate system and its axes, ordered by `coordinate_system_order`.
    ///
    /// # Errors
    ///
    /// The code is missing, or the axis count does not match the system type.
    pub fn create_coordinate_system(&self, code: &str) -> Result<CoordinateSystem, FactoryError> {
        let rows = self.db.query(
            "SELECT axis.name, abbrev, orientation, uom_auth_name, uom_code, cs.type FROM axis \
             LEFT JOIN coordinate_system cs ON axis.coordinate_system_auth_name = cs.auth_name \
             AND axis.coordinate_system_code = cs.code \
             WHERE coordinate_system_auth_name = ? AND coordinate_system_code = ? \
             ORDER BY coordinate_system_order",
            &[self.authority.clone(), code.to_owned()],
        )?;
        if rows.is_empty() {
            return Err(FactoryError::missing(
                "coordinate system",
                &self.authority,
                code,
            ));
        }
        let cs_type = rows[0][5].text();
        let mut axes = Vec::new();
        for row in &rows {
            if row[3].empty() && cs_type != "ordinal" {
                return Err(FactoryError::wrap(
                    "coordinate system",
                    &self.authority,
                    code,
                    "an axis has no unit",
                ));
            }
            let unit = if row[3].empty() {
                UnitOfMeasure {
                    name: String::new(),
                    factor: 0.0,
                    kind: UnitKind::None,
                    code_space: String::new(),
                    code: String::new(),
                }
            } else {
                self.child(&row[3].text())
                    .create_unit_of_measure(&row[4].text())?
            };
            let (direction, meridian) = axis_direction(&row[2].text()).map_err(|error| {
                FactoryError::wrap("coordinate system", &self.authority, code, error)
            })?;
            let mut axis = Axis::new(row[0].text(), row[1].text(), direction, unit);
            if let Some(value) = meridian {
                axis.meridian = Some(Meridian {
                    longitude: Measure {
                        value,
                        unit: UnitOfMeasure::degree(),
                    },
                });
            }
            axes.push(axis);
        }
        let identified = Identified::default().with_id(&self.authority, code);
        match cs_type.as_str() {
            "ellipsoidal" if axes.len() == 2 || axes.len() == 3 => {
                Ok(CoordinateSystem::Ellipsoidal(EllipsoidalCS {
                    identified,
                    axes,
                }))
            }
            "Cartesian" if axes.len() == 2 || axes.len() == 3 => {
                Ok(CoordinateSystem::Cartesian(CartesianCS {
                    identified,
                    axes,
                }))
            }
            "spherical" if axes.len() == 2 || axes.len() == 3 => {
                Ok(CoordinateSystem::Spherical(SphericalCS {
                    identified,
                    axes,
                }))
            }
            "vertical" if axes.len() == 1 => Ok(CoordinateSystem::Vertical(VerticalCS {
                identified,
                axis: axes.remove(0),
            })),
            "ordinal" => Ok(CoordinateSystem::Ordinal(OrdinalCS { identified, axes })),
            other => Err(FactoryError::wrap(
                "coordinate system",
                &self.authority,
                code,
                format!("unhandled coordinate system type {other}"),
            )),
        }
    }

    /// Geographic or geocentric CRS.
    ///
    /// # Errors
    ///
    /// The code is missing, or `text_definition` is set. That column is parsed
    /// by PROJ's WKT parser; this factory does not invent a CRS from it.
    pub fn create_geodetic_crs(&self, code: &str) -> Result<GeodeticCRS, FactoryError> {
        self.geodetic_crs(code, GeodeticFilter::Any)
    }

    /// Geographic 2D or 3D CRS.
    ///
    /// # Errors
    ///
    /// The code is not a geographic CRS.
    pub fn create_geographic_crs(&self, code: &str) -> Result<GeographicCRS, FactoryError> {
        match self.geodetic_crs(code, GeodeticFilter::GeographicOnly)? {
            GeodeticCRS::Geographic(crs) => Ok(crs),
            _ => Err(FactoryError::missing(
                "geographic CRS",
                &self.authority,
                code,
            )),
        }
    }

    /// First geographic 2D CRS whose official name or alias is `name`.
    ///
    /// The order is `createObjectsFromName`: non-deprecated official names,
    /// then aliases. A CRS this factory cannot build is skipped.
    ///
    /// # Errors
    ///
    /// The database rejects the statement.
    pub fn create_geographic_2d_crs_from_name(
        &self,
        name: &str,
    ) -> Result<Option<GeographicCRS>, FactoryError> {
        self.geographic_crs_from_name(name, "geographic 2D")
    }

    /// First geographic 3D CRS whose official name or alias is `name`.
    ///
    /// # Errors
    ///
    /// The database rejects the statement.
    pub fn create_geographic_3d_crs_from_name(
        &self,
        name: &str,
    ) -> Result<Option<GeographicCRS>, FactoryError> {
        self.geographic_crs_from_name(name, "geographic 3D")
    }

    fn geographic_crs_from_name(
        &self,
        name: &str,
        crs_type: &str,
    ) -> Result<Option<GeographicCRS>, FactoryError> {
        let sql = format!(
            "SELECT code FROM (\
                SELECT code, name, deprecated, 0 AS is_alias FROM geodetic_crs \
                WHERE type = '{crs_type}' AND name = ? COLLATE NOCASE AND auth_name = ? \
                UNION \
                SELECT ov.code, a.alt_name AS name, ov.deprecated, 1 AS is_alias \
                FROM geodetic_crs ov \
                JOIN alias_name a ON ov.auth_name = a.auth_name AND ov.code = a.code \
                WHERE a.source != 'EPSG_OLD' AND a.table_name = 'geodetic_crs' \
                AND ov.type = '{crs_type}' AND a.alt_name = ? COLLATE NOCASE \
                AND ov.auth_name = ?\
             ) ORDER BY deprecated, is_alias, length(name), name, code"
        );
        let rows = self.db.query(
            &sql,
            &[
                name.to_owned(),
                self.authority.clone(),
                name.to_owned(),
                self.authority.clone(),
            ],
        )?;
        for row in rows {
            if let Ok(crs) = self.create_geographic_crs(&row[0].text()) {
                return Ok(Some(crs));
            }
        }
        Ok(None)
    }

    /// Non-deprecated geodetic CRS that use this datum.
    ///
    /// `crs_type` is `geographic 2D`, `geographic 3D`, or `geocentric`. An empty
    /// type does not filter `geodetic_crs.type`.
    ///
    /// # Errors
    ///
    /// The database rejects the statement. A code this factory cannot build is
    /// skipped.
    pub fn create_geodetic_crs_from_datum(
        &self,
        datum_auth: &str,
        datum_code: &str,
        crs_type: &str,
    ) -> Result<Vec<GeodeticCRS>, FactoryError> {
        let mut sql = "SELECT auth_name, code FROM geodetic_crs WHERE \
             datum_auth_name = ? AND datum_code = ? AND deprecated = 0"
            .to_owned();
        let mut params = vec![datum_auth.to_owned(), datum_code.to_owned()];
        if !self.authority.is_empty() {
            sql.push_str(" AND auth_name = ?");
            params.push(self.authority.clone());
        }
        if !crs_type.is_empty() {
            sql.push_str(" AND type = ?");
            params.push(crs_type.to_owned());
        }
        sql.push_str(" ORDER BY auth_name, code");
        self.collect_geodetic_crs(&sql, &params)
    }

    /// Non-deprecated geodetic CRS whose datum uses this ellipsoid.
    ///
    /// # Errors
    ///
    /// The database rejects the statement. A code this factory cannot build is
    /// skipped.
    pub fn create_geodetic_crs_from_ellipsoid(
        &self,
        ellipsoid_auth: &str,
        ellipsoid_code: &str,
        crs_type: &str,
    ) -> Result<Vec<GeodeticCRS>, FactoryError> {
        let mut sql = "SELECT geodetic_crs.auth_name, geodetic_crs.code FROM geodetic_crs \
             JOIN geodetic_datum ON geodetic_crs.datum_auth_name = geodetic_datum.auth_name \
             AND geodetic_crs.datum_code = geodetic_datum.code WHERE \
             geodetic_datum.ellipsoid_auth_name = ? AND geodetic_datum.ellipsoid_code = ? \
             AND geodetic_datum.deprecated = 0 AND geodetic_crs.deprecated = 0"
            .to_owned();
        let mut params = vec![ellipsoid_auth.to_owned(), ellipsoid_code.to_owned()];
        if !self.authority.is_empty() {
            sql.push_str(" AND geodetic_crs.auth_name = ?");
            params.push(self.authority.clone());
        }
        if !crs_type.is_empty() {
            sql.push_str(" AND geodetic_crs.type = ?");
            params.push(crs_type.to_owned());
        }
        self.collect_geodetic_crs(&sql, &params)
    }

    /// Ellipsoids with the same semi-major axis and inverse flattening.
    ///
    /// This is `createEllipsoidFromExisting`. The match is relative `1e-10` on
    /// `a` and on either `b` or `1/f`.
    ///
    /// # Errors
    ///
    /// The database rejects the statement.
    pub fn create_ellipsoid_from_existing(
        &self,
        ellipsoid: &Ellipsoid,
    ) -> Result<Vec<Ellipsoid>, FactoryError> {
        let major = ellipsoid.semi_major_axis().si_value();
        let minor = semi_minor_si(ellipsoid);
        let flattening = ellipsoid.inverse_flattening();
        let rows = self.db.query(
            "SELECT auth_name, code FROM ellipsoid WHERE \
             abs(semi_major_axis - ?) < 1e-10 * abs(semi_major_axis) AND \
             ((semi_minor_axis IS NOT NULL AND abs(semi_minor_axis - ?) < 1e-10 * abs(semi_minor_axis)) OR \
             (inv_flattening IS NOT NULL AND abs(inv_flattening - ?) < 1e-10 * abs(inv_flattening)))",
            &[major.to_string(), minor.to_string(), flattening.to_string()],
        )?;
        let mut found = Vec::new();
        for row in rows {
            if let Ok(ellipsoid) = self.child(&row[0].text()).create_ellipsoid(&row[1].text()) {
                found.push(ellipsoid);
            }
        }
        Ok(found)
    }

    /// Geodetic CRS whose official name or alias matches `name`.
    ///
    /// Exact mode is `name = ? COLLATE NOCASE` on the official name and on
    /// aliases other than `EPSG_OLD`. Approximate mode keeps a name when the
    /// searched text, or its canonical form, is a substring. Deprecated rows are
    /// dropped once a non-deprecated row has been seen. When an exact official
    /// name matched, only those official-name matches are returned.
    ///
    /// # Errors
    ///
    /// The database rejects the statement.
    pub(crate) fn create_geodetic_crs_by_name(
        &self,
        name: &str,
        mode: NameMatch,
    ) -> Result<Vec<GeodeticCRS>, FactoryError> {
        self.objects_from_name(name, &["geodetic_crs"], mode, 0, |factory, row| {
            factory.create_geodetic_crs(&row.code)
        })
    }

    /// Geodetic datums whose official name or alias matches `name`.
    ///
    /// An ensemble code is returned as the datum `createGeodeticDatum` builds,
    /// with the non-ensemble name. The match is exact.
    ///
    /// # Errors
    ///
    /// The database rejects the statement.
    pub(crate) fn create_geodetic_datums_by_name(
        &self,
        name: &str,
    ) -> Result<Vec<GeodeticReferenceFrame>, FactoryError> {
        self.objects_from_name(
            name,
            &["geodetic_datum"],
            NameMatch::Exact,
            0,
            |factory, row| factory.create_geodetic_datum(&row.code),
        )
    }

    /// Objects whose official name or alias matches `name`, as PROJ's
    /// `createObjectsFromName` returns them. An empty `types` searches every
    /// table. `limit` caps an exact search; `0` means no cap.
    ///
    /// Ellipsoids, datums, datum ensembles, and CRSs are loaded. Prime
    /// meridians and coordinate operations are not: their rows are skipped.
    ///
    /// # Errors
    ///
    /// The database rejects the statement.
    pub fn create_objects_from_name(
        &self,
        name: &str,
        types: &[ObjectType],
        mode: NameMatch,
        limit: usize,
    ) -> Result<Vec<AuthorityObject>, FactoryError> {
        let keys = name_keys(types, name);
        self.objects_from_name(name, &keys, mode, limit, |factory, row| {
            load_named(factory, row, types)
        })
    }

    /// Extents whose name matches `name`, as `(authority, code)` pairs.
    /// An approximate match is a substring match.
    ///
    /// # Errors
    ///
    /// The database rejects the statement.
    pub fn list_area_of_use_from_name(
        &self,
        name: &str,
        mode: NameMatch,
    ) -> Result<Vec<(String, String)>, FactoryError> {
        let mut sql = String::from("SELECT auth_name, code FROM extent WHERE deprecated = 0 AND ");
        let mut params = Vec::new();
        if !self.authority.is_empty() {
            sql.push_str("auth_name = ? AND ");
            params.push(self.authority.clone());
        }
        sql.push_str("name LIKE ?");
        params.push(match mode {
            NameMatch::Exact => name.to_owned(),
            NameMatch::Approximate => format!("%{name}%"),
        });
        let rows = self.db.query(&sql, &params)?;
        Ok(rows
            .into_iter()
            .map(|row| (row[0].text(), row[1].text()))
            .collect())
    }

    fn objects_from_name<T>(
        &self,
        name: &str,
        tables: &[&str],
        mode: NameMatch,
        limit: usize,
        load: impl Fn(&Self, &NameRow) -> Result<T, FactoryError>,
    ) -> Result<Vec<T>, FactoryError>
    where
        T: NamedAuthorityObject,
    {
        let canonical = canonical_object_name(name);
        if canonical.chars().count() <= 1 {
            return Ok(Vec::new());
        }
        let rows = self.name_rows(tables, name, mode, limit)?;
        let mut seen = BTreeSet::new();
        let mut built: Vec<BuiltObject<T>> = Vec::new();
        let mut first = true;
        let mut first_deprecated = false;
        let mut exact_official = 0usize;
        let mut exact_alias = 0usize;
        for row in rows {
            if mode == NameMatch::Approximate
                && !approximate_name(&row.matched_name, name, &canonical)
            {
                continue;
            }
            let key = format!("{}\u{0}{}", row.authority, row.code);
            if !seen.insert(key) {
                continue;
            }
            if first {
                first_deprecated = row.deprecated;
                first = false;
            }
            if row.deprecated && !built.is_empty() && !first_deprecated {
                break;
            }
            let Ok(object) = load(&self.child(&row.authority), &row) else {
                continue;
            };
            if equivalent_object_name(object.name(), name) {
                exact_official += 1;
            } else if equivalent_object_name(&row.matched_name, name) {
                exact_alias += 1;
            }
            built.push(BuiltObject {
                matched_name: row.matched_name,
                object,
            });
        }
        if mode == NameMatch::Exact && exact_official + exact_alias >= 1 {
            let alias_only = exact_official == 0;
            built.retain(|item| {
                equivalent_object_name(item.object.name(), name)
                    || (alias_only && equivalent_object_name(&item.matched_name, name))
            });
        }
        built.sort_by(|left, right| {
            left.object
                .name()
                .len()
                .cmp(&right.object.name().len())
                .then_with(|| {
                    left.object
                        .identifiers()
                        .len()
                        .cmp(&right.object.identifiers().len())
                })
                .then_with(|| {
                    left.object
                        .identifiers()
                        .iter()
                        .zip(right.object.identifiers())
                        .find_map(|(left_id, right_id)| {
                            let space = left_id.code_space().cmp(right_id.code_space());
                            if space != std::cmp::Ordering::Equal {
                                return Some(space);
                            }
                            let code = left_id.code().cmp(right_id.code());
                            if code != std::cmp::Ordering::Equal {
                                return Some(code);
                            }
                            None
                        })
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });
        Ok(built.into_iter().map(|item| item.object).collect())
    }

    fn name_rows(
        &self,
        keys: &[&str],
        name: &str,
        mode: NameMatch,
        limit: usize,
    ) -> Result<Vec<NameRow>, FactoryError> {
        let mut sql =
            String::from("SELECT table_name, auth_name, code, name, deprecated, is_alias FROM (");
        let mut params = Vec::new();
        for (index, key) in keys.iter().enumerate() {
            let (table, filter) = key.split_once(':').unwrap_or((key, ""));
            if index > 0 {
                sql.push_str("UNION ");
            }
            sql.push_str(&format!(
                "SELECT '{key}' AS table_name, auth_name, code, name, deprecated, 0 AS is_alias \
                 FROM \"{table}\" WHERE 1 = 1 {}",
                name_filter_sql("", filter)
            ));
            if mode == NameMatch::Exact {
                sql.push_str("AND name = ? COLLATE NOCASE ");
                params.push(name.to_owned());
            }
            if !self.authority.is_empty() {
                sql.push_str("AND auth_name = ? ");
                params.push(self.authority.clone());
            }
            sql.push_str(&format!(
                "UNION SELECT '{key}' AS table_name, ov.auth_name, ov.code, a.alt_name AS name, \
                 ov.deprecated, 1 AS is_alias FROM \"{table}\" ov \
                 JOIN alias_name a ON ov.auth_name = a.auth_name AND ov.code = a.code \
                 WHERE a.source != 'EPSG_OLD' AND a.table_name = '{table}' {}",
                name_filter_sql("ov.", filter)
            ));
            if mode == NameMatch::Exact {
                sql.push_str("AND a.alt_name = ? COLLATE NOCASE ");
                params.push(name.to_owned());
            }
            if !self.authority.is_empty() {
                sql.push_str("AND ov.auth_name = ? ");
                params.push(self.authority.clone());
            }
        }
        sql.push_str(") ORDER BY deprecated, is_alias, length(name), name");
        if limit > 0 && mode == NameMatch::Exact {
            sql.push_str(&format!(" LIMIT {limit}"));
        }
        let rows = self.db.query(&sql, &params)?;
        Ok(rows
            .into_iter()
            .map(|row| NameRow {
                table: row[0].text(),
                authority: row[1].text(),
                code: row[2].text(),
                matched_name: row[3].text(),
                deprecated: row[4].text() == "1",
            })
            .collect())
    }

    fn collect_geodetic_crs(
        &self,
        sql: &str,
        params: &[String],
    ) -> Result<Vec<GeodeticCRS>, FactoryError> {
        let rows = self.db.query(sql, params)?;
        let mut found = Vec::new();
        for row in rows {
            if let Ok(crs) = self
                .child(&row[0].text())
                .create_geodetic_crs(&row[1].text())
            {
                found.push(crs);
            }
        }
        Ok(found)
    }

    /// Vertical CRS.
    ///
    /// # Errors
    ///
    /// The code is missing.
    pub fn create_vertical_crs(&self, code: &str) -> Result<VerticalCRS, FactoryError> {
        let row = self
            .one(
                "SELECT name, coordinate_system_auth_name, coordinate_system_code, \
             datum_auth_name, datum_code, deprecated FROM vertical_crs \
             WHERE auth_name = ? AND code = ?",
                code,
            )
            .map_err(|_| FactoryError::missing("vertical CRS", &self.authority, code))?;
        let cs = self
            .child(&row[1].text())
            .create_coordinate_system(&row[2].text())?;
        let CoordinateSystem::Vertical(coordinate_system) = cs else {
            return Err(FactoryError::wrap(
                "vertical CRS",
                &self.authority,
                code,
                "the coordinate system is not vertical",
            ));
        };
        let (datum, ensemble) = match self
            .child(&row[3].text())
            .vertical_datum(&row[4].text(), EnsembleHandling::Keep)?
        {
            VerticalOrEnsemble::Datum(datum) => (Some(datum), None),
            VerticalOrEnsemble::Ensemble(ensemble) => (None, Some(ensemble)),
        };
        Ok(VerticalCRS {
            identified: self.properties(code, &row[0].text(), &row[5], "", "vertical_crs")?,
            datum,
            ensemble,
            coordinate_system,
            geoid_models: Vec::new(),
        })
    }

    /// Engineering CRS.
    ///
    /// # Errors
    ///
    /// The code is missing.
    pub fn create_engineering_crs(&self, code: &str) -> Result<EngineeringCRS, FactoryError> {
        let row = self
            .one(
                "SELECT name, coordinate_system_auth_name, coordinate_system_code, \
             datum_auth_name, datum_code, deprecated FROM engineering_crs \
             WHERE auth_name = ? AND code = ?",
                code,
            )
            .map_err(|_| FactoryError::missing("engineering CRS", &self.authority, code))?;
        let coordinate_system = self
            .child(&row[1].text())
            .create_coordinate_system(&row[2].text())?;
        let datum = self
            .child(&row[3].text())
            .create_engineering_datum(&row[4].text())?;
        Ok(EngineeringCRS {
            identified: self.properties(code, &row[0].text(), &row[5], "", "engineering_crs")?,
            datum,
            coordinate_system,
        })
    }

    /// Conversion, including a map projection.
    ///
    /// # Errors
    ///
    /// The code is missing from `conversion` and from `other_transformation`.
    pub fn create_conversion(&self, code: &str) -> Result<Conversion, FactoryError> {
        let rows = self.db.query(
            "SELECT name, description, method_auth_name, method_code, method_name, \
             param1_auth_name, param1_code, param1_name, param1_value, param1_uom_auth_name, param1_uom_code, \
             param2_auth_name, param2_code, param2_name, param2_value, param2_uom_auth_name, param2_uom_code, \
             param3_auth_name, param3_code, param3_name, param3_value, param3_uom_auth_name, param3_uom_code, \
             param4_auth_name, param4_code, param4_name, param4_value, param4_uom_auth_name, param4_uom_code, \
             param5_auth_name, param5_code, param5_name, param5_value, param5_uom_auth_name, param5_uom_code, \
             param6_auth_name, param6_code, param6_name, param6_value, param6_uom_auth_name, param6_uom_code, \
             param7_auth_name, param7_code, param7_name, param7_value, param7_uom_auth_name, param7_uom_code, \
             deprecated FROM conversion WHERE auth_name = ? AND code = ?",
            &[self.authority.clone(), code.to_owned()],
        )?;
        let Some(row) = rows.into_iter().next() else {
            if let Ok(CoordinateOperation::Conversion(conversion)) = self
                .create_coordinate_operation_typed(
                    code,
                    ConcatenatedSteps::Forbid,
                    AlternativeGridNames::Official,
                    "other_transformation",
                )
            {
                return Ok(conversion);
            }
            return Err(FactoryError::missing("conversion", &self.authority, code));
        };
        let parameters = self.parameter_block(&row, 5, 7)?;
        let deprecated = &row[5 + 7 * 6];
        let mut identified = self.properties(
            code,
            &row[0].text(),
            deprecated,
            &row[1].text(),
            "conversion",
        )?;
        if identified.name.is_empty() {
            identified.name = "unnamed".to_owned();
        }
        Ok(Conversion {
            identified,
            method: OperationMethod {
                name: row[4].text(),
                code_space: row[2].text(),
                code: row[3].text(),
                parameters: Vec::new(),
            },
            parameters,
            version: None,
        })
    }

    /// Projected CRS built from the relational columns.
    ///
    /// A row whose only definition is `text_definition` is expanded by
    /// [`Self::create_projected_crs`].
    ///
    /// # Errors
    ///
    /// The code is missing, the coordinate system is not cartesian, or
    /// `text_definition` is set without a coordinate system and conversion.
    pub(crate) fn create_projected_crs_from_columns(
        &self,
        code: &str,
    ) -> Result<ProjectedCRS, FactoryError> {
        let row = self
            .one(
                "SELECT name, coordinate_system_auth_name, coordinate_system_code, \
             geodetic_crs_auth_name, geodetic_crs_code, conversion_auth_name, conversion_code, \
             text_definition, deprecated FROM projected_crs WHERE auth_name = ? AND code = ?",
                code,
            )
            .map_err(|_| FactoryError::missing("projected CRS", &self.authority, code))?;
        if !row[7].empty() {
            return Err(FactoryError::wrap(
                "projected CRS",
                &self.authority,
                code,
                "text_definition is not expanded",
            ));
        }
        let cs = self
            .child(&row[1].text())
            .create_coordinate_system(&row[2].text())?;
        let CoordinateSystem::Cartesian(coordinate_system) = cs else {
            return Err(FactoryError::wrap(
                "projected CRS",
                &self.authority,
                code,
                "the coordinate system is not cartesian",
            ));
        };
        let base = self
            .child(&row[3].text())
            .create_geodetic_crs(&row[4].text())?;
        let mut conversion = self
            .child(&row[5].text())
            .create_conversion(&row[6].text())?;
        if conversion.name() == "unnamed" {
            conversion.identified.name = row[0].text();
        }
        Ok(ProjectedCRS {
            identified: self.properties(code, &row[0].text(), &row[8], "", "projected_crs")?,
            base,
            conversion,
            coordinate_system,
        })
    }

    /// Derived projected CRS.
    ///
    /// # Errors
    ///
    /// The code is missing or `text_definition` is set.
    pub fn create_derived_projected_crs(
        &self,
        code: &str,
    ) -> Result<DerivedProjectedCRS, FactoryError> {
        let row = self
            .one(
                "SELECT name, coordinate_system_auth_name, coordinate_system_code, \
             base_crs_auth_name, base_crs_code, conversion_auth_name, conversion_code, \
             text_definition, deprecated FROM derived_projected_crs \
             WHERE auth_name = ? AND code = ?",
                code,
            )
            .map_err(|_| FactoryError::missing("derived projected CRS", &self.authority, code))?;
        if !row[7].empty() {
            return Err(FactoryError::wrap(
                "derived projected CRS",
                &self.authority,
                code,
                "text_definition is not expanded",
            ));
        }
        let coordinate_system = self
            .child(&row[1].text())
            .create_coordinate_system(&row[2].text())?;
        let base = self
            .child(&row[3].text())
            .create_projected_crs(&row[4].text())?;
        let conversion = self
            .child(&row[5].text())
            .create_conversion(&row[6].text())?;
        Ok(DerivedProjectedCRS {
            identified: self.properties(
                code,
                &row[0].text(),
                &row[8],
                "",
                "derived_projected_crs",
            )?,
            base,
            conversion,
            coordinate_system,
        })
    }

    /// Compound CRS.
    ///
    /// # Errors
    ///
    /// The code is missing, or the components are not a valid compound.
    pub fn create_compound_crs(&self, code: &str) -> Result<CompoundCRS, FactoryError> {
        let row = self
            .one(
                "SELECT name, horiz_crs_auth_name, horiz_crs_code, vertical_crs_auth_name, \
             vertical_crs_code, deprecated FROM compound_crs WHERE auth_name = ? AND code = ?",
                code,
            )
            .map_err(|_| FactoryError::missing("compound CRS", &self.authority, code))?;
        let horizontal = self
            .child(&row[1].text())
            .create_coordinate_reference_system(&row[2].text())?;
        let vertical = Crs::Vertical(
            self.child(&row[3].text())
                .create_vertical_crs(&row[4].text())?,
        );
        let mut crs = CompoundCRS::create(row[0].text(), vec![horizontal, vertical])?;
        crs.identified = self.properties(code, &row[0].text(), &row[5], "", "compound_crs")?;
        Ok(crs)
    }

    /// CRS of whatever type `crs_view` records.
    ///
    /// # Errors
    ///
    /// The code is missing or the type is not handled.
    pub fn create_coordinate_reference_system(&self, code: &str) -> Result<Crs, FactoryError> {
        let row = self
            .one(
                "SELECT type FROM crs_view WHERE auth_name = ? AND code = ?",
                code,
            )
            .map_err(|_| FactoryError::missing("CRS", &self.authority, code))?;
        match row[0].text().as_str() {
            "geographic 2D" | "geographic 3D" | "geocentric" | "other" => {
                Ok(Crs::from(self.create_geodetic_crs(code)?))
            }
            "vertical" => Ok(Crs::Vertical(self.create_vertical_crs(code)?)),
            "projected" => Ok(Crs::Projected(self.create_projected_crs(code)?)),
            "derived projected" => Ok(Crs::DerivedProjected(
                self.create_derived_projected_crs(code)?,
            )),
            "engineering" => Ok(Crs::Engineering(self.create_engineering_crs(code)?)),
            "compound" => Ok(Crs::Compound(self.create_compound_crs(code)?)),
            other => Err(FactoryError::wrap(
                "CRS",
                &self.authority,
                code,
                format!("unhandled CRS type {other}"),
            )),
        }
    }

    /// Coordinate operation, using PROJ alternative grid names when `grids` is set.
    ///
    /// # Errors
    ///
    /// The code is missing, or a step cannot be built.
    pub fn create_coordinate_operation(
        &self,
        code: &str,
        grids: AlternativeGridNames,
    ) -> Result<CoordinateOperation, FactoryError> {
        self.create_coordinate_operation_typed(code, ConcatenatedSteps::Allow, grids, "")
    }

    /// Operations registered from `source` to `target` in `coordinate_operation_view`.
    ///
    /// Deprecated rows are left out. The order is largest area of use, then
    /// smallest accuracy, which is the `ORDER BY` in
    /// `AuthorityFactory::createFromCoordinateReferenceSystemCodes`.
    ///
    /// # Errors
    ///
    /// A selected operation cannot be instantiated.
    pub fn create_from_coordinate_reference_system_codes(
        &self,
        source_code: &str,
        target_code: &str,
    ) -> Result<Vec<CoordinateOperation>, FactoryError> {
        operations_between(
            &self.db,
            &self.authority,
            &self.authority,
            source_code,
            &self.authority,
            target_code,
            AlternativeGridNames::Official,
        )
    }

    /// Authority codes of `kind`.
    ///
    /// # Errors
    ///
    /// The database rejects the statement.
    pub fn get_authority_codes(
        &self,
        kind: ObjectType,
        deprecated: DeprecatedCodes,
    ) -> Result<BTreeSet<String>, FactoryError> {
        let mut sql = kind.sql().to_owned();
        sql.push_str("auth_name = ?");
        if deprecated == DeprecatedCodes::Excluded {
            sql.push_str(" AND deprecated = 0");
        }
        let rows = self.db.query(&sql, std::slice::from_ref(&self.authority))?;
        Ok(rows.into_iter().map(|row| row[0].text()).collect())
    }

    /// Official name of `code`. A CRS is preferred when several objects share the code.
    ///
    /// # Errors
    ///
    /// Nothing has this code.
    pub fn get_description_text(&self, code: &str) -> Result<String, FactoryError> {
        let rows = self.db.query(
            "SELECT name, table_name FROM object_view WHERE auth_name = ? AND code = ? \
             ORDER BY table_name",
            &[self.authority.clone(), code.to_owned()],
        )?;
        if rows.is_empty() {
            return Err(FactoryError::missing("object", &self.authority, code));
        }
        let mut fallback = String::new();
        for row in &rows {
            let table = row[1].text();
            if matches!(
                table.as_str(),
                "geodetic_crs"
                    | "projected_crs"
                    | "vertical_crs"
                    | "compound_crs"
                    | "engineering_crs"
                    | "derived_projected_crs"
            ) {
                return Ok(row[0].text());
            }
            if fallback.is_empty() {
                fallback = row[0].text();
            }
        }
        Ok(fallback)
    }

    /// CRS summary rows for this authority.
    ///
    /// The query is `AuthorityFactory::getCRSInfoList`: each CRS table, its
    /// usage extent, and the celestial body reached through the datum.
    ///
    /// # Errors
    ///
    /// The database rejects the statement.
    pub fn get_crs_info_list(&self) -> Result<Vec<CrsInfo>, FactoryError> {
        let auth = self.authority.clone();
        let sql = "SELECT * FROM (\
            SELECT c.auth_name, c.code, c.name, c.type, c.deprecated, \
            a.west_lon, a.south_lat, a.east_lon, a.north_lat, a.description, NULL, cb.name \
            FROM geodetic_crs c \
            LEFT JOIN usage u ON u.object_table_name = 'geodetic_crs' AND u.object_auth_name = c.auth_name \
            AND u.object_code = c.code \
            LEFT JOIN extent a ON a.auth_name = u.extent_auth_name AND a.code = u.extent_code \
            LEFT JOIN geodetic_datum gd ON gd.auth_name = c.datum_auth_name AND gd.code = c.datum_code \
            LEFT JOIN ellipsoid e ON e.auth_name = gd.ellipsoid_auth_name AND e.code = gd.ellipsoid_code \
            LEFT JOIN celestial_body cb ON cb.auth_name = e.celestial_body_auth_name AND cb.code = e.celestial_body_code \
            WHERE c.auth_name = ? \
            UNION ALL SELECT c.auth_name, c.code, c.name, 'projected', c.deprecated, \
            a.west_lon, a.south_lat, a.east_lon, a.north_lat, a.description, cm.name, cb.name \
            FROM projected_crs c \
            LEFT JOIN conversion_table conv ON c.conversion_auth_name = conv.auth_name AND c.conversion_code = conv.code \
            LEFT JOIN conversion_method cm ON conv.method_auth_name = cm.auth_name AND conv.method_code = cm.code \
            LEFT JOIN geodetic_crs gcrs ON gcrs.auth_name = c.geodetic_crs_auth_name AND gcrs.code = c.geodetic_crs_code \
            LEFT JOIN usage u ON u.object_table_name = 'projected_crs' AND u.object_auth_name = c.auth_name \
            AND u.object_code = c.code \
            LEFT JOIN extent a ON a.auth_name = u.extent_auth_name AND a.code = u.extent_code \
            LEFT JOIN geodetic_datum gd ON gd.auth_name = gcrs.datum_auth_name AND gd.code = gcrs.datum_code \
            LEFT JOIN ellipsoid e ON e.auth_name = gd.ellipsoid_auth_name AND e.code = gd.ellipsoid_code \
            LEFT JOIN celestial_body cb ON cb.auth_name = e.celestial_body_auth_name AND cb.code = e.celestial_body_code \
            WHERE c.auth_name = ? \
            UNION ALL SELECT c.auth_name, c.code, c.name, 'vertical', c.deprecated, \
            a.west_lon, a.south_lat, a.east_lon, a.north_lat, a.description, NULL, 'Earth' \
            FROM vertical_crs c \
            LEFT JOIN usage u ON u.object_table_name = 'vertical_crs' AND u.object_auth_name = c.auth_name \
            AND u.object_code = c.code \
            LEFT JOIN extent a ON a.auth_name = u.extent_auth_name AND a.code = u.extent_code \
            WHERE c.auth_name = ? \
            UNION ALL SELECT c.auth_name, c.code, c.name, 'compound', c.deprecated, \
            a.west_lon, a.south_lat, a.east_lon, a.north_lat, a.description, NULL, 'Earth' \
            FROM compound_crs c \
            LEFT JOIN usage u ON u.object_table_name = 'compound_crs' AND u.object_auth_name = c.auth_name \
            AND u.object_code = c.code \
            LEFT JOIN extent a ON a.auth_name = u.extent_auth_name AND a.code = u.extent_code \
            WHERE c.auth_name = ? \
            UNION ALL SELECT c.auth_name, c.code, c.name, 'engineering', c.deprecated, \
            a.west_lon, a.south_lat, a.east_lon, a.north_lat, a.description, NULL, 'Earth' \
            FROM engineering_crs c \
            LEFT JOIN usage u ON u.object_table_name = 'engineering_crs' AND u.object_auth_name = c.auth_name \
            AND u.object_code = c.code \
            LEFT JOIN extent a ON a.auth_name = u.extent_auth_name AND a.code = u.extent_code \
            WHERE c.auth_name = ? \
            UNION ALL SELECT c.auth_name, c.code, c.name, 'derived projected', c.deprecated, \
            a.west_lon, a.south_lat, a.east_lon, a.north_lat, a.description, NULL, 'Earth' \
            FROM derived_projected_crs c \
            LEFT JOIN usage u ON u.object_table_name = 'derived_projected_crs' AND u.object_auth_name = c.auth_name \
            AND u.object_code = c.code \
            LEFT JOIN extent a ON a.auth_name = u.extent_auth_name AND a.code = u.extent_code \
            WHERE c.auth_name = ?\
            ) r ORDER BY auth_name, code";
        let rows = self.db.query(
            sql,
            &[
                auth.clone(),
                auth.clone(),
                auth.clone(),
                auth.clone(),
                auth.clone(),
                auth,
            ],
        )?;
        let mut list = Vec::with_capacity(rows.len());
        for row in rows {
            let bbox = if row[5].empty() {
                None
            } else {
                Some((row[5].f64()?, row[6].f64()?, row[7].f64()?, row[8].f64()?))
            };
            list.push(CrsInfo {
                authority: row[0].text(),
                code: row[1].text(),
                name: row[2].text(),
                type_name: row[3].text(),
                deprecated: row[4].flag(),
                west_lon: bbox.map(|value| value.0),
                south_lat: bbox.map(|value| value.1),
                east_lon: bbox.map(|value| value.2),
                north_lat: bbox.map(|value| value.3),
                area_name: row[9].text(),
                projection_method_name: row[10].text(),
                celestial_body_name: row[11].text(),
            });
        }
        Ok(list)
    }

    fn geodetic_crs(
        &self,
        code: &str,
        filter: GeodeticFilter,
    ) -> Result<GeodeticCRS, FactoryError> {
        let mut sql = "SELECT name, type, coordinate_system_auth_name, coordinate_system_code, \
             datum_auth_name, datum_code, text_definition, deprecated, description \
             FROM geodetic_crs WHERE auth_name = ? AND code = ?"
            .to_owned();
        if filter == GeodeticFilter::GeographicOnly {
            sql.push_str(" AND type IN ('geographic 2D', 'geographic 3D')");
        }
        let row = self
            .one(&sql, code)
            .map_err(|_| FactoryError::missing("geodetic CRS", &self.authority, code))?;
        if !row[6].empty() {
            return Err(FactoryError::wrap(
                "geodetic CRS",
                &self.authority,
                code,
                "text_definition is not expanded",
            ));
        }
        let cs = self
            .child(&row[2].text())
            .create_coordinate_system(&row[3].text())?;
        let (datum, ensemble) = match self
            .child(&row[4].text())
            .geodetic_datum(&row[5].text(), EnsembleHandling::Keep)?
        {
            DatumOrEnsemble::Datum(datum) => (Some(datum), None),
            DatumOrEnsemble::Ensemble(ensemble) => (None, Some(ensemble)),
        };
        let identified = self.properties(
            code,
            &row[0].text(),
            &row[7],
            &row[8].text(),
            "geodetic_crs",
        )?;
        let crs = match (row[1].text().as_str(), cs) {
            (
                "geographic 2D" | "geographic 3D",
                CoordinateSystem::Ellipsoidal(coordinate_system),
            ) => GeodeticCRS::Geographic(GeographicCRS {
                identified,
                datum,
                ensemble,
                coordinate_system,
                implicit_cs: false,
            }),
            ("geocentric", CoordinateSystem::Cartesian(coordinate_system)) => {
                GeodeticCRS::Geocentric(GeocentricCRS {
                    identified,
                    datum,
                    ensemble,
                    coordinate_system,
                })
            }
            ("other", CoordinateSystem::Spherical(coordinate_system)) => {
                GeodeticCRS::Spherical(super::model::SphericalGeodeticCRS {
                    identified,
                    datum,
                    ensemble,
                    coordinate_system,
                })
            }
            (kind, cs) => {
                return Err(FactoryError::wrap(
                    "geodetic CRS",
                    &self.authority,
                    code,
                    format!("unsupported type {kind} with {}", cs.wkt2_type()),
                ));
            }
        };
        Ok(crs)
    }

    fn geodetic_datum(
        &self,
        code: &str,
        handling: EnsembleHandling,
    ) -> Result<DatumOrEnsemble, FactoryError> {
        let row = self
            .one(
                "SELECT name, ellipsoid_auth_name, ellipsoid_code, prime_meridian_auth_name, \
             prime_meridian_code, publication_date, frame_reference_epoch, ensemble_accuracy, \
             anchor, anchor_epoch, deprecated, CAST(ensemble_accuracy AS TEXT) FROM geodetic_datum \
             WHERE auth_name = ? AND code = ?",
                code,
            )
            .map_err(|_| FactoryError::missing("geodetic datum", &self.authority, code))?;
        let mut name = row[0].text();
        if handling == EnsembleHandling::AsDatum {
            if let Some(renamed) = ensemble_name_as_datum(&name) {
                name = renamed.to_owned();
            }
        }
        if handling == EnsembleHandling::Keep && !row[7].empty() {
            let members = self.ensemble_members(
                "SELECT member_auth_name, member_code FROM geodetic_datum_ensemble_member \
                 WHERE ensemble_auth_name = ? AND ensemble_code = ? ORDER BY sequence",
                code,
            )?;
            let identified = self.properties(code, &name, &row[10], "", "geodetic_datum")?;
            return Ok(DatumOrEnsemble::Ensemble(DatumEnsemble {
                identified,
                members,
                accuracy: row[7].f64()?,
                accuracy_text: Some(row[11].text()),
            }));
        }
        let ellipsoid = self
            .child(&row[1].text())
            .create_ellipsoid(&row[2].text())?;
        let prime_meridian = self
            .child(&row[3].text())
            .create_prime_meridian(&row[4].text())?;
        let mut identified = self.properties(code, &name, &row[10], "", "geodetic_datum")?;
        if !row[5].empty() {
            identified.publication_date = Some(row[5].text());
        }
        if !row[8].empty() {
            identified.anchor = Some(row[8].text());
        }
        if !row[9].empty() {
            identified.anchor_epoch = Some(row[9].f64()?);
        }
        let frame_epoch = if row[6].empty() {
            None
        } else {
            Some(row[6].f64()?)
        };
        Ok(DatumOrEnsemble::Datum(GeodeticReferenceFrame {
            identified,
            ellipsoid,
            prime_meridian,
            frame_epoch,
            deformation_model: None,
        }))
    }

    fn vertical_datum(
        &self,
        code: &str,
        handling: EnsembleHandling,
    ) -> Result<VerticalOrEnsemble, FactoryError> {
        let row = self
            .one(
                "SELECT name, publication_date, frame_reference_epoch, ensemble_accuracy, \
             anchor, anchor_epoch, deprecated, CAST(ensemble_accuracy AS TEXT) FROM vertical_datum \
             WHERE auth_name = ? AND code = ?",
                code,
            )
            .map_err(|_| FactoryError::missing("vertical datum", &self.authority, code))?;
        if handling == EnsembleHandling::Keep && !row[3].empty() {
            let members = self.ensemble_members(
                "SELECT member_auth_name, member_code FROM vertical_datum_ensemble_member \
                 WHERE ensemble_auth_name = ? AND ensemble_code = ? ORDER BY sequence",
                code,
            )?;
            return Ok(VerticalOrEnsemble::Ensemble(DatumEnsemble {
                identified: self.properties(code, &row[0].text(), &row[6], "", "vertical_datum")?,
                members,
                accuracy: row[3].f64()?,
                accuracy_text: Some(row[7].text()),
            }));
        }
        let mut identified =
            self.properties(code, &row[0].text(), &row[6], "", "vertical_datum")?;
        if !row[1].empty() {
            identified.publication_date = Some(row[1].text());
        }
        if !row[4].empty() {
            identified.anchor = Some(row[4].text());
        }
        if !row[5].empty() {
            identified.anchor_epoch = Some(row[5].f64()?);
        }
        let wkt1_type = if self.authority == "ESRI" && code.starts_with("from_geogdatum_") {
            "2002"
        } else {
            "2005"
        };
        Ok(VerticalOrEnsemble::Datum(VerticalReferenceFrame {
            identified,
            wkt1_type: wkt1_type.to_owned(),
            frame_epoch: if row[2].empty() {
                None
            } else {
                Some(row[2].f64()?)
            },
            deformation_model: None,
        }))
    }

    fn ensemble_members(&self, sql: &str, code: &str) -> Result<Vec<Datum>, FactoryError> {
        let rows = self
            .db
            .query(sql, &[self.authority.clone(), code.to_owned()])?;
        let mut members = Vec::with_capacity(rows.len());
        for row in rows {
            members.push(self.child(&row[0].text()).create_datum(&row[1].text())?);
        }
        Ok(members)
    }

    fn properties(
        &self,
        code: &str,
        name: &str,
        deprecated: &Cell,
        remarks: &str,
        table: &str,
    ) -> Result<Identified, FactoryError> {
        let mut identified = Identified::named(name.to_owned()).with_id(&self.authority, code);
        identified.deprecated = deprecated.flag();
        identified.remarks = remarks.to_owned();
        identified.domains = self.usages(table, code)?;
        Ok(identified)
    }

    fn usages(&self, table: &str, code: &str) -> Result<Vec<ObjectDomain>, FactoryError> {
        let rows = if table == "geodetic_crs" && code == "4326" && self.authority == "EPSG" {
            self.db.query(
                "SELECT extent.description, extent.south_lat, extent.north_lat, extent.west_lon, \
                 extent.east_lon, scope.scope FROM extent, scope \
                 WHERE extent.code = '1262' AND scope.code = '1183'",
                &[],
            )?
        } else {
            let mut rows = self.db.query(
                "SELECT extent.description, extent.south_lat, extent.north_lat, extent.west_lon, \
                 extent.east_lon, scope.scope, \
                 (CASE WHEN scope.scope LIKE '%large scale%' THEN 0 ELSE 1 END), \
                 usage.auth_name, usage.code \
                 FROM usage \
                 JOIN extent ON usage.extent_auth_name = extent.auth_name AND usage.extent_code = extent.code \
                 JOIN scope ON usage.scope_auth_name = scope.auth_name AND usage.scope_code = scope.code \
                 WHERE object_table_name = ? AND object_auth_name = ? AND object_code = ? \
                 AND NOT (usage.extent_auth_name = 'PROJ' AND usage.extent_code = 'EXTENT_UNKNOWN') \
                 AND NOT (usage.scope_auth_name = 'PROJ' AND usage.scope_code = 'SCOPE_UNKNOWN')",
                &[table.to_owned(), self.authority.clone(), code.to_owned()],
            )?;
            rows.sort_by(|left, right| {
                let score_left = left[6].text();
                let score_right = right[6].text();
                score_left
                    .cmp(&score_right)
                    .then_with(|| left[7].text().cmp(&right[7].text()))
                    .then_with(|| left[8].text().cmp(&right[8].text()))
            });
            rows
        };
        let mut domains = Vec::new();
        for row in rows {
            if let Ok(domain) = domain_from_extent(
                &row[0].text(),
                &row[1],
                &row[2],
                &row[3],
                &row[4],
                Some(&row[5].text()),
            ) {
                domains.push(domain);
            }
        }
        Ok(domains)
    }

    fn parameter_block(
        &self,
        row: &[Cell],
        start: usize,
        count: usize,
    ) -> Result<Vec<Parameter>, FactoryError> {
        let mut parameters = Vec::new();
        for index in 0..count {
            let base = start + index * 6;
            if row[base].empty() {
                break;
            }
            let (value, unit_code) =
                normalize_measure(&row[base + 5].text(), &row[base + 3].text())?;
            let unit = self
                .child(&row[base + 4].text())
                .create_unit_of_measure(&unit_code)?;
            parameters.push(Parameter {
                name: row[base + 2].text(),
                code_space: row[base].text(),
                code: row[base + 1].text(),
                value: ParameterValue::Measure(Measure { value, unit }),
            });
        }
        Ok(parameters)
    }

    fn create_coordinate_operation_typed(
        &self,
        code: &str,
        concatenated: ConcatenatedSteps,
        grids: AlternativeGridNames,
        type_hint: &str,
    ) -> Result<CoordinateOperation, FactoryError> {
        let mut kind = type_hint.to_owned();
        if kind.is_empty() {
            let row = self
                .one(
                    "SELECT type FROM coordinate_operation_with_conversion_view \
                 WHERE auth_name = ? AND code = ?",
                    code,
                )
                .map_err(|_| {
                    FactoryError::missing("coordinate operation", &self.authority, code)
                })?;
            kind = row[0].text();
        }
        match kind.as_str() {
            "conversion" => Ok(CoordinateOperation::Conversion(
                self.create_conversion(code)?,
            )),
            "helmert_transformation" => self.helmert(code).map(CoordinateOperation::Transformation),
            "grid_transformation" => self
                .grid_transformation(code, grids)
                .map(CoordinateOperation::Transformation),
            "other_transformation" => self.other_transformation(code, grids),
            "concatenated_operation" if concatenated == ConcatenatedSteps::Allow => self
                .concatenated(code, grids)
                .map(CoordinateOperation::Concatenated),
            other => Err(FactoryError::wrap(
                "coordinate operation",
                &self.authority,
                code,
                format!("unhandled coordinate operation type {other}"),
            )),
        }
    }

    fn helmert(&self, code: &str) -> Result<Transformation, FactoryError> {
        let row = self.one(
            "SELECT name, description, method_auth_name, method_code, method_name, \
             source_crs_auth_name, source_crs_code, target_crs_auth_name, target_crs_code, \
             accuracy, tx, ty, tz, translation_uom_auth_name, translation_uom_code, \
             rx, ry, rz, rotation_uom_auth_name, rotation_uom_code, \
             scale_difference, scale_difference_uom_auth_name, scale_difference_uom_code, \
             rate_tx, rate_ty, rate_tz, rate_translation_uom_auth_name, rate_translation_uom_code, \
             rate_rx, rate_ry, rate_rz, rate_rotation_uom_auth_name, rate_rotation_uom_code, \
             rate_scale_difference, rate_scale_difference_uom_auth_name, rate_scale_difference_uom_code, \
             epoch, epoch_uom_auth_name, epoch_uom_code, \
             px, py, pz, pivot_uom_auth_name, pivot_uom_code, \
             operation_version, deprecated FROM helmert_transformation \
             WHERE auth_name = ? AND code = ?",
            code,
        )?;
        let translation = self
            .child(&row[13].text())
            .create_unit_of_measure(&row[14].text())?;
        let mut parameters = vec![
            helmert_parameter("X-axis translation", "8605", &row[10], &translation)?,
            helmert_parameter("Y-axis translation", "8606", &row[11], &translation)?,
            helmert_parameter("Z-axis translation", "8607", &row[12], &translation)?,
        ];
        if !row[15].empty() {
            let rotation = self
                .child(&row[18].text())
                .create_unit_of_measure(&row[19].text())?;
            parameters.push(helmert_parameter(
                "X-axis rotation",
                "8608",
                &row[15],
                &rotation,
            )?);
            parameters.push(helmert_parameter(
                "Y-axis rotation",
                "8609",
                &row[16],
                &rotation,
            )?);
            parameters.push(helmert_parameter(
                "Z-axis rotation",
                "8610",
                &row[17],
                &rotation,
            )?);
            let scale_unit = if row[21].empty() {
                UnitOfMeasure {
                    name: String::new(),
                    factor: 0.0,
                    kind: UnitKind::None,
                    code_space: String::new(),
                    code: String::new(),
                }
            } else {
                self.child(&row[21].text())
                    .create_unit_of_measure(&row[22].text())?
            };
            parameters.push(helmert_parameter(
                "Scale difference",
                "8611",
                &row[20],
                &scale_unit,
            )?);
        }
        let epoch_unit = if row[37].empty() {
            None
        } else {
            Some(
                self.child(&row[37].text())
                    .create_unit_of_measure(&row[38].text())?,
            )
        };
        let has_rates = !row[23].empty()
            || !row[24].empty()
            || !row[25].empty()
            || !row[28].empty()
            || !row[29].empty()
            || !row[30].empty()
            || !row[33].empty();
        if has_rates {
            let rate_translation = self
                .child(&row[26].text())
                .create_unit_of_measure(&row[27].text())?;
            let rate_rotation = self
                .child(&row[31].text())
                .create_unit_of_measure(&row[32].text())?;
            let rate_scale = self
                .child(&row[34].text())
                .create_unit_of_measure(&row[35].text())?;
            parameters.push(helmert_parameter(
                "Rate of change of X-axis translation",
                "1040",
                &row[23],
                &rate_translation,
            )?);
            parameters.push(helmert_parameter(
                "Rate of change of Y-axis translation",
                "1041",
                &row[24],
                &rate_translation,
            )?);
            parameters.push(helmert_parameter(
                "Rate of change of Z-axis translation",
                "1042",
                &row[25],
                &rate_translation,
            )?);
            parameters.push(helmert_parameter(
                "Rate of change of X-axis rotation",
                "1043",
                &row[28],
                &rate_rotation,
            )?);
            parameters.push(helmert_parameter(
                "Rate of change of Y-axis rotation",
                "1044",
                &row[29],
                &rate_rotation,
            )?);
            parameters.push(helmert_parameter(
                "Rate of change of Z-axis rotation",
                "1045",
                &row[30],
                &rate_rotation,
            )?);
            parameters.push(helmert_parameter(
                "Rate of change of Scale difference",
                "1046",
                &row[33],
                &rate_scale,
            )?);
            if let Some(unit) = epoch_unit {
                parameters.push(helmert_parameter(
                    "Parameter reference epoch",
                    "1047",
                    &row[36],
                    &unit,
                )?);
            }
        } else if let Some(unit) = epoch_unit {
            parameters.push(helmert_parameter(
                "Transformation reference epoch",
                "1049",
                &row[36],
                &unit,
            )?);
        } else if !row[39].empty() {
            let pivot = self
                .child(&row[42].text())
                .create_unit_of_measure(&row[43].text())?;
            parameters.push(helmert_parameter(
                "Ordinate 1 of evaluation point",
                "8617",
                &row[39],
                &pivot,
            )?);
            parameters.push(helmert_parameter(
                "Ordinate 2 of evaluation point",
                "8618",
                &row[40],
                &pivot,
            )?);
            parameters.push(helmert_parameter(
                "Ordinate 3 of evaluation point",
                "8667",
                &row[41],
                &pivot,
            )?);
        }
        self.finish_transformation(
            code,
            "helmert_transformation",
            &row[0].text(),
            &row[1].text(),
            &row[2].text(),
            &row[3].text(),
            &row[4].text(),
            &row[5].text(),
            &row[6].text(),
            &row[7].text(),
            &row[8].text(),
            &row[9],
            &row[44].text(),
            &row[45],
            parameters,
            None,
            PointMotion::No,
        )
    }

    fn grid_transformation(
        &self,
        code: &str,
        grids: AlternativeGridNames,
    ) -> Result<Transformation, FactoryError> {
        let row = self.one(
            "SELECT name, description, method_auth_name, method_code, method_name, \
             source_crs_auth_name, source_crs_code, target_crs_auth_name, target_crs_code, \
             accuracy, grid_param_auth_name, grid_param_code, grid_param_name, grid_name, \
             grid2_param_auth_name, grid2_param_code, grid2_param_name, grid2_name, \
             interpolation_crs_auth_name, interpolation_crs_code, operation_version, deprecated \
             FROM grid_transformation WHERE auth_name = ? AND code = ?",
            code,
        )?;
        let mut parameters = vec![self.grid_parameter(
            &row[10].text(),
            &row[11].text(),
            &row[12].text(),
            &row[13].text(),
            grids,
        )?];
        if !row[17].empty() {
            parameters.push(self.grid_parameter(
                &row[14].text(),
                &row[15].text(),
                &row[16].text(),
                &row[17].text(),
                grids,
            )?);
        }
        let interpolation = if row[18].empty() {
            None
        } else {
            Some(Box::new(
                self.child(&row[18].text())
                    .create_coordinate_reference_system(&row[19].text())?,
            ))
        };
        self.finish_transformation(
            code,
            "grid_transformation",
            &row[0].text(),
            &row[1].text(),
            &row[2].text(),
            &row[3].text(),
            &row[4].text(),
            &row[5].text(),
            &row[6].text(),
            &row[7].text(),
            &row[8].text(),
            &row[9],
            &row[20].text(),
            &row[21],
            parameters,
            interpolation,
            if row[4].text().starts_with("Point motion") {
                PointMotion::Yes
            } else {
                PointMotion::No
            },
        )
    }

    fn other_transformation(
        &self,
        code: &str,
        grids: AlternativeGridNames,
    ) -> Result<CoordinateOperation, FactoryError> {
        let sql = "SELECT name, description, method_auth_name, method_code, method_name, \
             source_crs_auth_name, source_crs_code, target_crs_auth_name, target_crs_code, \
             grid_param_auth_name, grid_param_code, grid_param_name, grid_name, \
             interpolation_crs_auth_name, interpolation_crs_code, operation_version, accuracy, deprecated, \
             param1_auth_name, param1_code, param1_name, param1_value, param1_uom_auth_name, param1_uom_code, \
             param2_auth_name, param2_code, param2_name, param2_value, param2_uom_auth_name, param2_uom_code, \
             param3_auth_name, param3_code, param3_name, param3_value, param3_uom_auth_name, param3_uom_code, \
             param4_auth_name, param4_code, param4_name, param4_value, param4_uom_auth_name, param4_uom_code, \
             param5_auth_name, param5_code, param5_name, param5_value, param5_uom_auth_name, param5_uom_code, \
             param6_auth_name, param6_code, param6_name, param6_value, param6_uom_auth_name, param6_uom_code, \
             param7_auth_name, param7_code, param7_name, param7_value, param7_uom_auth_name, param7_uom_code, \
             param8_auth_name, param8_code, param8_name, param8_value, param8_uom_auth_name, param8_uom_code, \
             param9_auth_name, param9_code, param9_name, param9_value, param9_uom_auth_name, param9_uom_code \
             FROM other_transformation WHERE auth_name = ? AND code = ?";
        let row = self.one(sql, code)?;
        let mut parameters = Vec::new();
        if !row[12].empty() {
            parameters.push(self.grid_parameter(
                &row[9].text(),
                &row[10].text(),
                &row[11].text(),
                &row[12].text(),
                grids,
            )?);
        }
        parameters.extend(self.parameter_block(&row, 18, 9)?);
        let interpolation = if row[13].empty() {
            None
        } else {
            Some(Box::new(
                self.child(&row[13].text())
                    .create_coordinate_reference_system(&row[14].text())?,
            ))
        };
        let transformation = self.finish_transformation(
            code,
            "other_transformation",
            &row[0].text(),
            &row[1].text(),
            &row[2].text(),
            &row[3].text(),
            &row[4].text(),
            &row[5].text(),
            &row[6].text(),
            &row[7].text(),
            &row[8].text(),
            &row[16],
            &row[15].text(),
            &row[17],
            parameters,
            interpolation,
            PointMotion::No,
        )?;
        if transformation.method.code_space == "EPSG"
            && matches!(
                transformation.method.code.as_str(),
                "9843" | "9844" | "1068" | "1069" | "1104"
            )
        {
            return Ok(CoordinateOperation::Conversion(Conversion {
                identified: transformation.identified,
                method: transformation.method,
                parameters: transformation.parameters,
                version: transformation.version,
            }));
        }
        Ok(CoordinateOperation::Transformation(transformation))
    }

    fn concatenated(
        &self,
        code: &str,
        grids: AlternativeGridNames,
    ) -> Result<ConcatenatedOperation, FactoryError> {
        let row = self.one(
            "SELECT name, description, source_crs_auth_name, source_crs_code, \
             target_crs_auth_name, target_crs_code, accuracy, operation_version, deprecated \
             FROM concatenated_operation WHERE auth_name = ? AND code = ?",
            code,
        )?;
        let steps_rows = self.db.query(
            "SELECT step_auth_name, step_code, step_direction FROM concatenated_operation_step \
             WHERE operation_auth_name = ? AND operation_code = ? ORDER BY step_number",
            &[self.authority.clone(), code.to_owned()],
        )?;
        let mut steps = Vec::new();
        for step in steps_rows {
            let operation = self
                .child(&step[0].text())
                .create_coordinate_operation(&step[1].text(), grids)?;
            steps.push(ConcatenatedStep {
                operation,
                reverse: step[2].text() == "reverse",
            });
        }
        let source = self
            .child(&row[2].text())
            .create_coordinate_reference_system(&row[3].text())?;
        let target = self
            .child(&row[4].text())
            .create_coordinate_reference_system(&row[5].text())?;
        let accuracy = accuracy_of(&row[6]);
        Ok(ConcatenatedOperation {
            identified: self.properties(
                code,
                &row[0].text(),
                &row[8],
                &row[1].text(),
                "concatenated_operation",
            )?,
            steps,
            source: Some(Box::new(source)),
            target: Some(Box::new(target)),
            accuracy,
            version: optional_text(&row[7].text()),
        })
    }

    fn finish_transformation(
        &self,
        code: &str,
        table: &str,
        name: &str,
        description: &str,
        method_auth: &str,
        method_code: &str,
        method_name: &str,
        source_auth: &str,
        source_code: &str,
        target_auth: &str,
        target_code: &str,
        accuracy: &Cell,
        version: &str,
        deprecated: &Cell,
        parameters: Vec<Parameter>,
        interpolation: Option<Box<Crs>>,
        motion: PointMotion,
    ) -> Result<Transformation, FactoryError> {
        let source = if source_auth.is_empty() {
            None
        } else {
            Some(Box::new(
                self.child(source_auth)
                    .create_coordinate_reference_system(source_code)?,
            ))
        };
        let target = if target_auth.is_empty() {
            None
        } else {
            Some(Box::new(
                self.child(target_auth)
                    .create_coordinate_reference_system(target_code)?,
            ))
        };
        let identified = self.properties(code, name, deprecated, description, table)?;
        Ok(Transformation {
            identified,
            method: OperationMethod {
                name: method_name.to_owned(),
                code_space: method_auth.to_owned(),
                code: method_code.to_owned(),
                parameters: Vec::new(),
            },
            parameters,
            source,
            target,
            interpolation,
            accuracy: accuracy_of(accuracy),
            version: optional_text(version),
            point_motion: motion == PointMotion::Yes,
            inverted: false,
        })
    }

    fn grid_parameter(
        &self,
        auth: &str,
        code: &str,
        name: &str,
        grid: &str,
        grids: AlternativeGridNames,
    ) -> Result<Parameter, FactoryError> {
        let filename = if grids == AlternativeGridNames::Proj {
            let rows = self.db.query(
                "SELECT proj_grid_name FROM grid_alternatives WHERE original_grid_name = ?",
                &[grid.to_owned()],
            )?;
            rows.into_iter()
                .find_map(|row| {
                    let name = row[0].text();
                    if name.is_empty() {
                        None
                    } else {
                        Some(name)
                    }
                })
                .unwrap_or_else(|| grid.to_owned())
        } else {
            grid.to_owned()
        };
        Ok(Parameter {
            name: name.to_owned(),
            code_space: auth.to_owned(),
            code: code.to_owned(),
            value: ParameterValue::Filename(filename),
        })
    }
}

#[expect(
    clippy::large_enum_variant,
    reason = "a datum and an ensemble are alternative results, not a hot discriminant"
)]
enum DatumOrEnsemble {
    Datum(GeodeticReferenceFrame),
    Ensemble(DatumEnsemble),
}

enum VerticalOrEnsemble {
    Datum(VerticalReferenceFrame),
    Ensemble(DatumEnsemble),
}

/// Object returned by [`AuthorityFactory::create_object`].
#[derive(Clone, Debug)]
#[expect(
    clippy::large_enum_variant,
    reason = "authority objects are returned whole; the CRS variant owns the graph"
)]
pub enum AuthorityObject {
    /// Extent.
    Extent(ObjectDomain),
    /// Unit of measure.
    Unit(UnitOfMeasure),
    /// Prime meridian.
    PrimeMeridian(PrimeMeridian),
    /// Ellipsoid.
    Ellipsoid(Ellipsoid),
    /// Single datum.
    Datum(Datum),
    /// Datum ensemble.
    DatumEnsemble(DatumEnsemble),
    /// Coordinate reference system.
    Crs(Crs),
    /// Coordinate operation.
    Operation(CoordinateOperation),
}

/// One row of [`AuthorityFactory::get_crs_info_list`].
#[derive(Clone, Debug)]
pub struct CrsInfo {
    /// Authority name.
    pub authority: String,
    /// Code.
    pub code: String,
    /// Name.
    pub name: String,
    /// `geographic 2D`, `projected`, `vertical`, or another `crs_view` type.
    pub type_name: String,
    /// Deprecated flag.
    pub deprecated: bool,
    /// West longitude in degrees, when the extent has a box.
    pub west_lon: Option<f64>,
    /// South latitude in degrees.
    pub south_lat: Option<f64>,
    /// East longitude in degrees.
    pub east_lon: Option<f64>,
    /// North latitude in degrees.
    pub north_lat: Option<f64>,
    /// Area of use.
    pub area_name: String,
    /// Projection method, for a projected CRS.
    pub projection_method_name: String,
    /// Celestial body.
    pub celestial_body_name: String,
}

/// Which codes [`AuthorityFactory::get_authority_codes`] lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectType {
    /// Prime meridian.
    PrimeMeridian,
    /// Ellipsoid.
    Ellipsoid,
    /// Any datum table.
    Datum,
    /// Geodetic datum.
    GeodeticReferenceFrame,
    /// Dynamic geodetic datum.
    DynamicGeodeticReferenceFrame,
    /// Vertical datum.
    VerticalReferenceFrame,
    /// Dynamic vertical datum.
    DynamicVerticalReferenceFrame,
    /// Engineering datum.
    EngineeringDatum,
    /// Any CRS.
    Crs,
    /// Geodetic CRS.
    GeodeticCrs,
    /// Geocentric CRS.
    GeocentricCrs,
    /// Geographic 2D or 3D CRS.
    GeographicCrs,
    /// Geographic 2D CRS.
    Geographic2dCrs,
    /// Geographic 3D CRS.
    Geographic3dCrs,
    /// Projected CRS.
    ProjectedCrs,
    /// Vertical CRS.
    VerticalCrs,
    /// Compound CRS.
    CompoundCrs,
    /// Engineering CRS.
    EngineeringCrs,
    /// Derived projected CRS.
    DerivedProjectedCrs,
    /// Any coordinate operation, including conversions.
    CoordinateOperation,
    /// Conversion.
    Conversion,
    /// Transformation that is not concatenated.
    Transformation,
    /// Concatenated operation.
    ConcatenatedOperation,
    /// Datum ensemble.
    DatumEnsemble,
}

impl ObjectType {
    fn sql(self) -> &'static str {
        match self {
            Self::PrimeMeridian => "SELECT code FROM prime_meridian WHERE ",
            Self::Ellipsoid => "SELECT code FROM ellipsoid WHERE ",
            Self::Datum => {
                "SELECT code FROM object_view WHERE table_name IN \
                 ('geodetic_datum', 'vertical_datum', 'engineering_datum') AND "
            }
            Self::GeodeticReferenceFrame => "SELECT code FROM geodetic_datum WHERE ",
            Self::DynamicGeodeticReferenceFrame => {
                "SELECT code FROM geodetic_datum WHERE frame_reference_epoch IS NOT NULL AND "
            }
            Self::VerticalReferenceFrame => "SELECT code FROM vertical_datum WHERE ",
            Self::DynamicVerticalReferenceFrame => {
                "SELECT code FROM vertical_datum WHERE frame_reference_epoch IS NOT NULL AND "
            }
            Self::EngineeringDatum => "SELECT code FROM engineering_datum WHERE ",
            Self::Crs => "SELECT code FROM crs_view WHERE ",
            Self::GeodeticCrs => "SELECT code FROM geodetic_crs WHERE ",
            Self::GeocentricCrs => "SELECT code FROM geodetic_crs WHERE type = 'geocentric' AND ",
            Self::GeographicCrs => {
                "SELECT code FROM geodetic_crs WHERE type IN ('geographic 2D', 'geographic 3D') AND "
            }
            Self::Geographic2dCrs => {
                "SELECT code FROM geodetic_crs WHERE type = 'geographic 2D' AND "
            }
            Self::Geographic3dCrs => {
                "SELECT code FROM geodetic_crs WHERE type = 'geographic 3D' AND "
            }
            Self::ProjectedCrs => "SELECT code FROM projected_crs WHERE ",
            Self::VerticalCrs => "SELECT code FROM vertical_crs WHERE ",
            Self::CompoundCrs => "SELECT code FROM compound_crs WHERE ",
            Self::EngineeringCrs => "SELECT code FROM engineering_crs WHERE ",
            Self::DerivedProjectedCrs => "SELECT code FROM derived_projected_crs WHERE ",
            Self::CoordinateOperation => {
                "SELECT code FROM coordinate_operation_with_conversion_view WHERE "
            }
            Self::Conversion => "SELECT code FROM conversion WHERE ",
            Self::Transformation => {
                "SELECT code FROM coordinate_operation_view WHERE table_name != \
                 'concatenated_operation' AND "
            }
            Self::ConcatenatedOperation => "SELECT code FROM concatenated_operation WHERE ",
            Self::DatumEnsemble => {
                "SELECT code FROM object_view WHERE table_name IN \
                 ('geodetic_datum', 'vertical_datum') AND type = 'ensemble' AND "
            }
        }
    }
}

/// Authority and pivot list used by [`CoordinateOperationFactory`].
#[derive(Clone, Debug)]
pub struct CoordinateOperationContext {
    db: DatabaseContext,
    authority: String,
    intermediates: Vec<(String, String)>,
    allow_intermediate: bool,
}

impl CoordinateOperationContext {
    /// Context that reads `db`. An empty authority considers every authority.
    #[must_use]
    pub fn create(db: &DatabaseContext, authority: impl Into<String>) -> Self {
        Self {
            db: db.clone(),
            authority: authority.into(),
            intermediates: Vec::new(),
            allow_intermediate: true,
        }
    }

    /// Pivot CRS codes, as `(authority, code)` pairs.
    pub fn set_intermediate_crs(&mut self, intermediates: Vec<(String, String)>) -> &mut Self {
        self.intermediates = intermediates;
        self
    }

    /// When false, only a direct registered operation or a deriving conversion is returned.
    pub fn set_intermediate_crs_use(&mut self, policy: IntermediateCrsUse) -> &mut Self {
        self.allow_intermediate = policy == IntermediateCrsUse::Allow;
        self
    }
}

/// Finds coordinate operations between two CRS.
///
/// The search is the registered half of PROJ's `CoordinateOperationFactory`:
/// the deriving conversion when the target is projected on the source, then
/// `coordinate_operation_view` ordered by area and accuracy. A pivot is used
/// only when [`CoordinateOperationContext::set_intermediate_crs`] names one and
/// both legs are in the database.
#[derive(Clone, Copy, Debug)]
pub struct CoordinateOperationFactory;

impl CoordinateOperationFactory {
    /// Factory. It holds no database of its own; the context does.
    #[must_use]
    pub fn create() -> Self {
        Self
    }

    /// PROJ string of the operation from `source` to `target` when no database is required.
    ///
    /// A projected `target` whose base is `source` is the deriving conversion,
    /// wrapped as the pipeline PROJ emits for a north-east geographic source.
    ///
    /// # Errors
    ///
    /// The pair is not that projection, or a step cannot be written.
    pub fn create_operation_proj_string(
        &self,
        source: &Crs,
        target: &Crs,
    ) -> Result<String, FactoryError> {
        super::projstring::operation_proj_string(source, target)
    }

    /// Operations from `source` to `target`.
    ///
    /// # Errors
    ///
    /// A registered operation cannot be built. An unknown `+proj` step inside
    /// a `text_definition` is that error, not a coordinate.
    pub fn create_operations(
        &self,
        source: &Crs,
        target: &Crs,
        context: &CoordinateOperationContext,
    ) -> Result<Vec<CoordinateOperation>, FactoryError> {
        let mut found = Vec::new();
        if let Crs::Projected(projected) = target {
            if same_authority(source, projected.base_crs()) {
                found.push(CoordinateOperation::Conversion(
                    projected.deriving_conversion().clone(),
                ));
            }
        }
        let Some(source_id) = source
            .authority_code("EPSG")
            .or_else(|| source.identifiers().first())
        else {
            return Ok(found);
        };
        let Some(target_id) = target
            .authority_code("EPSG")
            .or_else(|| target.identifiers().first())
        else {
            return Ok(found);
        };
        found.extend(operations_between(
            &context.db,
            &context.authority,
            source_id.code_space(),
            source_id.code(),
            target_id.code_space(),
            target_id.code(),
            AlternativeGridNames::Official,
        )?);
        if context.allow_intermediate {
            for (pivot_auth, pivot_code) in &context.intermediates {
                let first = operations_between(
                    &context.db,
                    &context.authority,
                    source_id.code_space(),
                    source_id.code(),
                    pivot_auth,
                    pivot_code,
                    AlternativeGridNames::Official,
                )?;
                let second = operations_between(
                    &context.db,
                    &context.authority,
                    pivot_auth,
                    pivot_code,
                    target_id.code_space(),
                    target_id.code(),
                    AlternativeGridNames::Official,
                )?;
                if let (Some(left), Some(right)) = (first.first(), second.first()) {
                    found.push(CoordinateOperation::Concatenated(ConcatenatedOperation {
                        identified: Identified::named(format!(
                            "{} + {}",
                            left.name(),
                            right.name()
                        )),
                        steps: vec![
                            ConcatenatedStep {
                                operation: left.clone(),
                                reverse: false,
                            },
                            ConcatenatedStep {
                                operation: right.clone(),
                                reverse: false,
                            },
                        ],
                        source: None,
                        target: None,
                        accuracy: None,
                        version: None,
                    }));
                }
            }
        }
        Ok(found)
    }
}

fn same_authority(source: &Crs, base: &GeodeticCRS) -> bool {
    let base_ids = base.identifiers();
    let Some(source_id) = source.identifiers().first() else {
        return false;
    };
    base_ids.iter().any(|identifier| {
        identifier.code_space() == source_id.code_space() && identifier.code() == source_id.code()
    })
}

struct OperationRow {
    source_auth: String,
    source_code: String,
    target_auth: String,
    target_code: String,
    auth: String,
    code: String,
    table: String,
    area: f64,
    accuracy_missing: bool,
    accuracy: f64,
}

fn operations_between(
    db: &DatabaseContext,
    authority_filter: &str,
    source_auth: &str,
    source_code: &str,
    target_auth: &str,
    target_code: &str,
    grids: AlternativeGridNames,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let mut sql = "SELECT cov.source_crs_auth_name, cov.source_crs_code, \
         cov.target_crs_auth_name, cov.target_crs_code, cov.auth_name, cov.code, cov.table_name, \
         extent.south_lat, extent.west_lon, extent.north_lat, extent.east_lon, cov.accuracy \
         FROM coordinate_operation_view cov \
         JOIN usage ON usage.object_table_name = cov.table_name \
         AND usage.object_auth_name = cov.auth_name AND usage.object_code = cov.code \
         JOIN extent ON extent.auth_name = usage.extent_auth_name AND extent.code = usage.extent_code \
         WHERE cov.source_crs_auth_name = ? AND cov.source_crs_code = ? \
         AND cov.target_crs_auth_name = ? AND cov.target_crs_code = ? \
         AND cov.deprecated = 0"
        .to_owned();
    let mut params = vec![
        source_auth.to_owned(),
        source_code.to_owned(),
        target_auth.to_owned(),
        target_code.to_owned(),
    ];
    if !authority_filter.is_empty() {
        sql.push_str(" AND cov.auth_name = ?");
        params.push(authority_filter.to_owned());
    }
    let rows = db.query(&sql, &params)?;
    let mut parsed = Vec::new();
    for row in rows {
        let area = pseudo_area(&row[7], &row[8], &row[9], &row[10]);
        let accuracy_missing = row[11].empty();
        let accuracy = if accuracy_missing {
            f64::MAX
        } else {
            row[11].f64().unwrap_or(f64::MAX)
        };
        parsed.push(OperationRow {
            source_auth: row[0].text(),
            source_code: row[1].text(),
            target_auth: row[2].text(),
            target_code: row[3].text(),
            auth: row[4].text(),
            code: row[5].text(),
            table: row[6].text(),
            area,
            accuracy_missing,
            accuracy,
        });
    }
    parsed.sort_by(|left, right| {
        right
            .area
            .partial_cmp(&left.area)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.accuracy_missing.cmp(&right.accuracy_missing))
            .then_with(|| {
                left.accuracy
                    .partial_cmp(&right.accuracy)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    let mut seen = BTreeSet::new();
    let mut operations = Vec::new();
    for row in parsed {
        if !seen.insert((row.auth.clone(), row.code.clone())) {
            continue;
        }
        let factory = AuthorityFactory::create(db, &row.auth);
        let operation = factory.create_coordinate_operation_typed(
            &row.code,
            ConcatenatedSteps::Allow,
            grids,
            &row.table,
        )?;
        let _ = (
            &row.source_auth,
            &row.source_code,
            &row.target_auth,
            &row.target_code,
        );
        operations.push(operation);
    }
    Ok(operations)
}

fn pseudo_area(south: &Cell, west: &Cell, north: &Cell, east: &Cell) -> f64 {
    let Ok(south) = south.f64() else {
        return 0.0;
    };
    let Ok(west) = west.f64() else {
        return 0.0;
    };
    let Ok(north) = north.f64() else {
        return 0.0;
    };
    let mut east = east.f64().unwrap_or(west);
    if east < west {
        east += 360.0;
    }
    let south_rad = south * DEGREE_SI;
    let north_rad = north * DEGREE_SI;
    (east - west) * (north_rad.sin() - south_rad.sin())
}

fn domain_from_extent(
    description: &str,
    south: &Cell,
    north: &Cell,
    west: &Cell,
    east: &Cell,
    scope: Option<&str>,
) -> Result<ObjectDomain, FactoryError> {
    let bbox = if south.empty() {
        (None, None, None, None)
    } else {
        (
            Some(south.f64()?),
            Some(west.f64()?),
            Some(north.f64()?),
            Some(east.f64()?),
        )
    };
    Ok(ObjectDomain {
        scope: scope.filter(|text| !text.is_empty()).map(str::to_owned),
        area: if description.is_empty() {
            None
        } else {
            Some(description.to_owned())
        },
        south: bbox.0,
        west: bbox.1,
        north: bbox.2,
        east: bbox.3,
    })
}

fn normalize_measure(unit_code: &str, value: &str) -> Result<(f64, String), FactoryError> {
    if unit_code == "9110" {
        let number = value.parse::<f64>().map_err(|_| {
            FactoryError::Invalid(format!("sexagesimal value {value} is not numeric"))
        })?;
        let sign = if number.is_sign_negative() { -1.0 } else { 1.0 };
        let text = format!("{number:.12}");
        let digits = text.split('.').nth(1).unwrap_or("000000000000");
        let minutes: f64 = digits.get(..2).unwrap_or("0").parse().unwrap_or(0.0);
        let seconds_digits = digits.get(2..).unwrap_or("0");
        let seconds: f64 = seconds_digits.parse().unwrap_or(0.0);
        let scale = 10_f64.powi(i32::try_from(seconds_digits.len().saturating_sub(2)).unwrap_or(0));
        let degrees = number.abs().floor() + minutes / 60.0 + (seconds / scale) / 3600.0;
        return Ok((sign * degrees, "9102".to_owned()));
    }
    let number = if value.is_empty() {
        0.0
    } else {
        value
            .parse::<f64>()
            .map_err(|_| FactoryError::Invalid(format!("value {value} is not numeric")))?
    };
    Ok((number, unit_code.to_owned()))
}

fn helmert_parameter(
    name: &str,
    code: &str,
    value: &Cell,
    unit: &UnitOfMeasure,
) -> Result<Parameter, FactoryError> {
    Ok(Parameter {
        name: name.to_owned(),
        code_space: "EPSG".to_owned(),
        code: code.to_owned(),
        value: ParameterValue::Measure(Measure {
            value: if value.empty() { 0.0 } else { value.f64()? },
            unit: unit.clone(),
        }),
    })
}

fn accuracy_of(cell: &Cell) -> Option<f64> {
    if cell.empty() {
        return None;
    }
    let value = cell.f64().ok()?;
    if value == 999.0 {
        None
    } else {
        Some(value)
    }
}

fn optional_text(text: &str) -> Option<String> {
    if text.is_empty() {
        None
    } else {
        Some(text.to_owned())
    }
}

fn axis_direction(orientation: &str) -> Result<(String, Option<f64>), String> {
    let known = [
        "north",
        "northNorthEast",
        "northEast",
        "eastNorthEast",
        "east",
        "eastSouthEast",
        "southEast",
        "southSouthEast",
        "south",
        "southSouthWest",
        "southWest",
        "westSouthWest",
        "west",
        "westNorthWest",
        "northWest",
        "northNorthWest",
        "up",
        "down",
        "geocentricX",
        "geocentricY",
        "geocentricZ",
        "columnPositive",
        "columnNegative",
        "rowPositive",
        "rowNegative",
        "displayRight",
        "displayLeft",
        "displayUp",
        "displayDown",
        "forward",
        "aft",
        "port",
        "starboard",
        "clockwise",
        "counterClockwise",
        "towards",
        "awayFrom",
        "future",
        "past",
        "unspecified",
    ];
    if known.contains(&orientation) {
        return Ok((orientation.to_owned(), None));
    }
    if orientation.starts_with("Geocentre > equator/0") {
        return Ok(("geocentricX".to_owned(), None));
    }
    if orientation.starts_with("Geocentre > equator/90") {
        return Ok(("geocentricY".to_owned(), None));
    }
    if orientation == "Geocentre > north pole" {
        return Ok(("geocentricZ".to_owned(), None));
    }
    if let Some(rest) = orientation.strip_prefix("North along ") {
        return Ok(("north".to_owned(), meridian_degrees(rest)));
    }
    if let Some(rest) = orientation.strip_prefix("South along ") {
        return Ok(("south".to_owned(), meridian_degrees(rest)));
    }
    Err(format!("unknown axis direction: {orientation}"))
}

fn meridian_degrees(text: &str) -> Option<f64> {
    let text = text.trim();
    if let Some(prefix) = text.strip_suffix("°W") {
        return prefix.trim().parse::<f64>().ok().map(|value| -value);
    }
    if let Some(prefix) = text.strip_suffix("°E") {
        return prefix.trim().parse::<f64>().ok();
    }
    None
}

impl From<GeodeticCRS> for Crs {
    fn from(crs: GeodeticCRS) -> Self {
        match crs {
            GeodeticCRS::Geographic(crs) => Self::Geographic(crs),
            other => Self::Geodetic(other),
        }
    }
}
