//! `AuthorityFactory` and `CoordinateOperationFactory` over the embedded `proj.db`.
//!
//! The statements are the ones in PROJ `src/iso19111/factory.cpp`. A missing
//! code is an error. A `text_definition` is not turned into a coordinate.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use rusqlite::{types::ValueRef, Connection};

use super::buildop::{concatenation_interpolation, concatenation_interpolation_of_steps, OpProps};
use super::compound::{flatten_steps, get_accuracy, get_accuracy_of_steps, stored_accuracy};
use super::coordinate_metadata::CoordinateMetadata;
use super::equiv::{
    equivalent_name, equivalent_name_bigger_differences, geodetic_frame, same_geodetic_datum,
    same_geodetic_datum_with_database, same_geographic_base, Equivalence,
};
use super::json::create_from_user_input;
use super::model::{
    ensemble_name_as_datum, Axis, BoundingBox, CartesianCS, CompoundCRS, ConcatenatedOperation,
    ConcatenatedStep, Conversion, CoordinateOperation, CoordinateSystem, Crs, Datum, DatumEnsemble,
    DerivedProjectedCRS, Ellipsoid, EllipsoidalCS, EngineeringCRS, EngineeringDatum, FactoryError,
    GeocentricCRS, GeodeticCRS, GeodeticReferenceFrame, GeographicCRS, Identified, Identifier,
    Measure, Meridian, ObjectDomain, OperationMethod, OrdinalCS, Parameter, ParameterValue,
    PrimeMeridian, ProjectedCRS, SphericalCS, Transformation, UnitKind, UnitOfMeasure, VerticalCRS,
    VerticalCS, VerticalReferenceFrame,
};
use super::parammap::{MethodMap, OTHER_METHODS, PROJECTION_METHODS};
use super::pipeline::ProjLines;
use super::projstring::look_for_proj_well_known_ellps;
use super::wkt::format_g;
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

/// Whether [`AuthorityFactory::create_from_coordinate_reference_system_codes_in`] keeps an
/// operation that a listed operation supersedes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SupersededOperations {
    /// Keep every operation.
    Keep,
    /// Drop an operation whose replacement is listed.
    Discard,
}

/// Whether a coordinate-operation search may chain through a named pivot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntermediateCrsUse {
    /// Use pivots from [`CoordinateOperationContext::set_intermediate_crs`] when no
    /// registered operation is found directly (`IF_NO_DIRECT_TRANSFORMATION`).
    Allow,
    /// Return only a direct operation or a deriving conversion.
    Never,
    /// Use the pivots even when a registered operation is found directly (`ALWAYS`).
    Always,
}

/// Whether operations with a ballpark transformation are candidates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BallparkUse {
    /// Ballpark operations are kept, as PROJ does by default.
    Allow,
    /// Ballpark operations are dropped (`--hide-ballpark`).
    Forbid,
}

/// `inCreateOperationsGeogToVertWithIntermediateVert`.
pub(super) const GUARD_VERTICAL_INTERMEDIATE: u8 = 1;
/// `inCreateOperationsGeogToVertWithAlternativeGeog`.
pub(super) const GUARD_ALTERNATIVE_GEOG: u8 = 2;
/// `inCreateOperationsWithIntermediate`.
pub(super) const GUARD_DATUM_INTERMEDIATE: u8 = 4;
/// `inCreateOperationsWithDatumPivotAntiRecursion`.
pub(super) const GUARD_DATUM_PIVOT: u8 = 8;
/// `inCreateOperationsVertToVertWithIntermediateVert`.
pub(super) const GUARD_VERTICAL_PIVOT: u8 = 16;
/// `inCreateOperationsGeogToGeogWithAlternativeGeog`.
pub(super) const GUARD_GEOG_ALTERNATIVE: u8 = 32;
/// `skipHorizontalTransformation`: not a guard, but a flag of the same `Private::Context`, set
/// while `createOperationsCompoundToGeog` builds its vertical transformations.
pub(super) const GUARD_SKIP_HORIZONTAL: u8 = 64;

/// How the availability of an operation's grids enters the ordering.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GridAvailabilityUse {
    /// An operation whose grids are not on disk sorts after one whose grids are.
    #[default]
    UseForSorting,
    /// Grids are taken as available, as `IGNORE_GRID_AVAILABILITY`.
    IgnoreGridAvailability,
    /// A grid the database knows counts as available, as `KNOWN_AVAILABLE`
    /// (`considerKnownGridsAsAvailable`). A registry operation with a grid that is neither on
    /// disk nor known is discarded.
    KnownAvailable,
    /// `DISCARD_OPERATION_IF_MISSING_GRID`: a registry operation with a grid that is not on disk
    /// is discarded, and a grid the database knows is not counted as available. The search then
    /// falls through to the operations that need no such grid.
    DiscardIfMissing,
}

/// Whether the registered operations of the database are searched.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum RegistryUse {
    /// Registered operations, pivots and datum-based chains are candidates.
    #[default]
    Database,
    /// Only the operations that need no database: `createOperation(source, target)`.
    None,
}

/// How an operation extent is compared with the area of interest.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpatialCriterion {
    /// The operation extent must contain the area. PROJ's default.
    #[default]
    StrictContainment,
    /// The operation extent must intersect the area.
    PartialIntersection,
}

/// Which CRS extents bound the candidate operations, as PROJ's `--crs-extent-use`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CrsExtentUse {
    /// The CRS extents are not used.
    Ignore,
    /// The operation extent must contain both CRS extents.
    Both,
    /// The area of interest is the intersection of the two CRS extents.
    Intersection,
    /// The area of interest is the smaller of the two CRS extents. PROJ's default.
    #[default]
    Smallest,
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

    pub(super) fn query(
        &self,
        sql: &str,
        params: &[String],
    ) -> Result<Vec<Vec<Cell>>, FactoryError> {
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

    /// Versioned code spaces of `authority`, such as `IAU_2015` for `IAU`.
    pub(crate) fn versioned_authorities_from_name(&self, authority: &str) -> Vec<String> {
        self.query(
            "SELECT versioned_auth_name FROM versioned_auth_name_mapping WHERE auth_name = ?",
            &[authority.to_owned()],
        )
        .map(|rows| rows.into_iter().map(|row| row[0].text()).collect())
        .unwrap_or_default()
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

    /// `grid_alternatives.proj_grid_name` for an official grid name, when the database renames it for PROJ.
    pub(crate) fn proj_grid_name(&self, original: &str) -> Result<Option<String>, FactoryError> {
        Ok(self.proj_grid_alternative(original)?.map(|(name, _)| name))
    }

    /// `lookForGridAlternative`: the PROJ grid name of an official grid name, and whether
    /// PROJ reads that grid in the inverse direction (`inverse_direction`).
    pub(crate) fn proj_grid_alternative(
        &self,
        original: &str,
    ) -> Result<Option<(String, bool)>, FactoryError> {
        let rows = self.query(
            "SELECT proj_grid_name, inverse_direction FROM grid_alternatives \
             WHERE original_grid_name = ? AND proj_grid_name <> ''",
            &[original.to_owned()],
        )?;
        Ok(rows
            .first()
            .map(|row| (row[0].text(), row[1].text() == "1")))
    }

    /// `lookForGridInfo` with `considerKnownGridsAsAvailable`: a grid is known when
    /// a grid package names it, when a download URL goes with an open licence, or
    /// when the file is already on disk. `null` is always known.
    pub(crate) fn grid_is_known(&self, name: &str) -> Result<bool, FactoryError> {
        if name == "null" {
            return Ok(true);
        }
        let local = crate::gridpath::resolve(name).is_some();
        let rows = self.query(
            "SELECT grid_packages.package_name, grid_alternatives.url, \
             grid_packages.url, grid_alternatives.open_license, \
             grid_packages.open_license \
             FROM grid_alternatives \
             LEFT JOIN grid_packages ON \
             grid_alternatives.package_name = grid_packages.package_name \
             WHERE proj_grid_name = ? OR old_proj_grid_name = ?",
            &[name.to_owned(), name.to_owned()],
        )?;
        let Some(row) = rows.first() else {
            return Ok(local);
        };
        let package = row[0].text();
        let url = if row[1].empty() {
            row[2].text()
        } else {
            row[1].text()
        };
        let open_license = if row[3].empty() {
            row[4].text()
        } else {
            row[3].text()
        } == "1";
        Ok(!package.is_empty() || (!url.is_empty() && open_license) || local)
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

    /// ESRI vertical CRS whose datum is `from_geogdatum_{code_space}_{code}`.
    ///
    /// `exportAsESRIWktCompoundCRSWithEllipsoidalHeight` uses that one CRS as the `VERTCS`.
    pub(crate) fn esri_vertical_code_for_geodetic(
        &self,
        code_space: &str,
        code: &str,
    ) -> Option<String> {
        let datum_code = format!("from_geogdatum_{code_space}_{code}");
        let rows = self
            .query(
                "SELECT code FROM vertical_crs \
                 WHERE auth_name = 'ESRI' AND datum_auth_name = 'ESRI' AND datum_code = ?",
                &[datum_code],
            )
            .ok()?;
        if rows.len() == 1 {
            Some(rows[0][0].text())
        } else {
            None
        }
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
            | "vertical_datum" | "vertical_crs" => table,
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

    /// Repairs a TOWGS84 clause whose rotation signs are reversed.
    ///
    /// `params` holds `tx, ty, tz, rx, ry, rz, scale_difference`. When these
    /// values match an EPSG Coordinate Frame rotation (9607) and no Position
    /// Vector (9606) row, the rotation terms are negated and `true` is returned.
    /// The values are left alone otherwise.
    ///
    /// # Errors
    ///
    /// The database rejects the statement.
    pub fn to_wgs84_autocorrect_wrong_values(
        &self,
        params: &mut [f64; 7],
    ) -> Result<bool, FactoryError> {
        if params[3..6].iter().all(|rotation| *rotation == 0.0) {
            return Ok(false);
        }
        let methods = self.towgs84_methods(params)?;
        if !methods.iter().any(|code| code == "9607") || methods.iter().any(|code| code == "9606") {
            return Ok(false);
        }
        let mut reversed = *params;
        for rotation in &mut reversed[3..6] {
            *rotation = -*rotation;
        }
        if !self.towgs84_methods(&reversed)?.is_empty() {
            return Ok(false);
        }
        params[3..6].copy_from_slice(&reversed[3..6]);
        Ok(true)
    }

    /// Method codes of the EPSG Helmert rows that match `params`.
    fn towgs84_methods(&self, params: &[f64; 7]) -> Result<Vec<String>, FactoryError> {
        let guard = self.lock();
        let mut statement = guard
            .prepare(
                "SELECT DISTINCT CAST(method_code AS TEXT) \
                 FROM helmert_transformation_table WHERE \
                 abs(tx - ?) <= 1e-8 * abs(tx) AND \
                 abs(ty - ?) <= 1e-8 * abs(ty) AND \
                 abs(tz - ?) <= 1e-8 * abs(tz) AND \
                 abs(rx - ?) <= 1e-8 * abs(rx) AND \
                 abs(ry - ?) <= 1e-8 * abs(ry) AND \
                 abs(rz - ?) <= 1e-8 * abs(rz) AND \
                 abs(scale_difference - ?) <= 1e-8 * abs(scale_difference) AND \
                 method_auth_name = 'EPSG' AND \
                 method_code IN (9606, 9607) AND \
                 translation_uom_auth_name = 'EPSG' AND \
                 translation_uom_code = 9001 AND \
                 rotation_uom_auth_name = 'EPSG' AND \
                 rotation_uom_code = 9104 AND \
                 scale_difference_uom_auth_name = 'EPSG' AND \
                 scale_difference_uom_code = 9202 AND \
                 deprecated = 0",
            )
            .map_err(|error| FactoryError::Sql(error.to_string()))?;
        let rows = statement
            .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                row.get::<_, String>(0)
            })
            .map_err(|error| FactoryError::Sql(error.to_string()))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| FactoryError::Sql(error.to_string()))
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
        // Realizations share the ensemble name as an alias. Prefer the ensemble
        // (`European Terrestrial Reference System 1989 ensemble`) and return the
        // name `datum()` synthesizes, not an arbitrary member such as EPSG:1132.
        let ensemble_key = format!("{key}ensemble");
        let ensemble = self.query(
            "SELECT name FROM geodetic_datum \
             WHERE REPLACE(REPLACE(LOWER(name), '_', ''), ' ', '') = ? \
             ORDER BY deprecated LIMIT 1",
            std::slice::from_ref(&ensemble_key),
        );
        if let Ok(rows) = ensemble {
            if let Some(row) = rows.first() {
                let name = row[0].text();
                if let Some(stripped) = name.strip_suffix(" ensemble") {
                    return Some((stripped.to_owned(), String::new(), String::new()));
                }
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
        if let Ok(rows) = by_alias {
            if let Some(row) = rows.first() {
                return Some((row[0].text(), row[1].text(), row[2].text()));
            }
        }
        self.geodetic_datum_by_folded_name(name)
    }

    /// Alias match that also folds accents, skipping `EPSG_OLD` spellings.
    ///
    /// The SQL above only drops spaces and underscores, so
    /// `System_Jednotne_Trigonometricke_Site_Katastralni_Ferro` misses the
    /// accented EPSG alias of datum 6818.
    fn geodetic_datum_by_folded_name(&self, name: &str) -> Option<(String, String, String)> {
        let rows = self
            .query(
                "SELECT g.name, g.auth_name, g.code, a.alt_name, IFNULL(a.source, ''), \
                 g.deprecated \
                 FROM alias_name a \
                 JOIN geodetic_datum g ON g.auth_name = a.auth_name AND g.code = a.code \
                 WHERE a.table_name = 'geodetic_datum'",
                &[],
            )
            .ok()?;
        let mut matches: Vec<(bool, bool, String, String, String)> = rows
            .into_iter()
            .filter(|row| {
                let source = row[4].text();
                let official = equivalent_name(&row[0].text(), name);
                let alias = source != "EPSG_OLD" && equivalent_name(&row[3].text(), name);
                official || alias
            })
            .map(|row| {
                (
                    row[5].text() != "0",
                    row[1].text() != "EPSG",
                    row[0].text(),
                    row[1].text(),
                    row[2].text(),
                )
            })
            .collect();
        matches.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
        matches
            .first()
            .map(|(_, _, official, auth, code)| (official.clone(), auth.clone(), code.clone()))
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

    /// `getNonDeprecated`: replacement authority and code. A `PROJ` source
    /// wins; otherwise every row is returned.
    ///
    /// # Errors
    ///
    /// The database rejects the statement.
    pub fn non_deprecated_codes(
        &self,
        table: &str,
        authority: &str,
        code: &str,
    ) -> Result<Vec<(String, String)>, FactoryError> {
        let rows = self.query(
            "SELECT replacement_auth_name, replacement_code, source FROM deprecation \
             WHERE table_name = ? AND deprecated_auth_name = ? AND deprecated_code = ?",
            &[table.to_owned(), authority.to_owned(), code.to_owned()],
        )?;
        let from_proj: Vec<_> = rows
            .iter()
            .filter(|row| row[2].text() == "PROJ")
            .map(|row| (row[0].text(), row[1].text()))
            .collect();
        if !from_proj.is_empty() {
            return Ok(from_proj);
        }
        Ok(rows
            .iter()
            .map(|row| (row[0].text(), row[1].text()))
            .collect())
    }

    /// `lookForGridInfo` with known grids not treated as available.
    /// `null` is the implicit grid: no file, an open licence, and available.
    ///
    /// # Errors
    ///
    /// The database rejects the statement.
    pub fn grid_info(&self, name: &str) -> Result<Option<GridInfo>, FactoryError> {
        self.look_for_grid_info(name, GridAvailabilityUse::UseForSorting)
    }

    /// `lookForGridInfo`: the record of `name`, or `None` when the database has no row for it.
    /// A grid the database knows is available under [`GridAvailabilityUse::KnownAvailable`].
    pub(crate) fn look_for_grid_info(
        &self,
        name: &str,
        known: GridAvailabilityUse,
    ) -> Result<Option<GridInfo>, FactoryError> {
        if name == "null" {
            return Ok(Some(GridInfo {
                full_name: String::new(),
                package_name: String::new(),
                url: String::new(),
                direct_download: false,
                open_license: true,
                available: true,
            }));
        }
        let rows = self.query(
            "SELECT grid_packages.package_name, grid_alternatives.url, \
             grid_packages.url, grid_alternatives.open_license, \
             grid_packages.open_license, grid_alternatives.direct_download, \
             grid_packages.direct_download, grid_alternatives.proj_grid_name, \
             grid_alternatives.old_proj_grid_name \
             FROM grid_alternatives \
             LEFT JOIN grid_packages ON \
             grid_alternatives.package_name = grid_packages.package_name \
             WHERE proj_grid_name = ? OR old_proj_grid_name = ?",
            &[name.to_owned(), name.to_owned()],
        )?;
        let Some(row) = rows.first() else {
            // An official grid name is looked up under the PROJ name that replaces it.
            if let Some((alternative, _)) = self.proj_grid_alternative(name)? {
                if alternative != name {
                    return self.look_for_grid_info(&alternative, known);
                }
            }
            return Ok(None);
        };
        let package_name = row[0].text();
        let url = if row[1].empty() {
            row[2].text()
        } else {
            row[1].text()
        };
        let open_license = license_flag(&row[3], &row[4]);
        let direct_download = license_flag(&row[5], &row[6]);
        let proj_grid_name = row[7].text();
        let old_name = row[8].text();
        let mut full_name = String::new();
        let mut available = false;
        if proj_grid_name != old_name && old_name == name {
            if let Some(path) = crate::gridpath::resolve(&proj_grid_name) {
                available = true;
                full_name = path.to_string_lossy().into_owned();
            }
        }
        let known_grid = !package_name.is_empty() || (!url.is_empty() && open_license);
        if !available && known == GridAvailabilityUse::KnownAvailable && known_grid {
            available = true;
            if let Some(path) = crate::gridpath::resolve(name) {
                full_name = path.to_string_lossy().into_owned();
            }
        }
        if !available {
            if let Some(path) = crate::gridpath::resolve(name) {
                available = true;
                full_name = path.to_string_lossy().into_owned();
            }
        }
        Ok(Some(GridInfo {
            full_name,
            package_name,
            url,
            direct_download,
            open_license,
            available,
        }))
    }

    /// Geographic CRS built the way `proj_create_geographic_crs` builds one.
    ///
    /// A name ending in ` (deprecated)` is stored without that suffix and
    /// marked deprecated. A datum name `WGS_1984`, or a name containing `_`,
    /// is replaced by the official geodetic-datum name when the database has
    /// an equivalent spelling or alias.
    #[must_use]
    pub fn create_geographic_crs(
        &self,
        parts: &GeodeticComponents<'_>,
        coordinate_system: EllipsoidalCS,
    ) -> GeographicCRS {
        let (name, deprecated) = object_name(parts.crs_name);
        let mut identified = Identified::named(name);
        identified.deprecated = deprecated;
        GeographicCRS {
            identified,
            datum: Some(self.geodetic_reference_frame(parts)),
            ensemble: None,
            coordinate_system,
            implicit_cs: false,
        }
    }

    /// Geocentric CRS built the way `proj_create_geocentric_crs` builds one.
    #[must_use]
    pub fn create_geocentric_crs(
        &self,
        parts: &GeodeticComponents<'_>,
        linear_unit_name: Option<&str>,
        linear_unit_to_metre: f64,
    ) -> GeodeticCRS {
        let (name, deprecated) = object_name(parts.crs_name);
        let mut identified = Identified::named(name);
        identified.deprecated = deprecated;
        GeodeticCRS::Geocentric(GeocentricCRS {
            identified,
            datum: Some(self.geodetic_reference_frame(parts)),
            ensemble: None,
            coordinate_system: CartesianCS::create_geocentric(linear_unit(
                linear_unit_name,
                linear_unit_to_metre,
            )),
        })
    }

    fn geodetic_reference_frame(&self, parts: &GeodeticComponents<'_>) -> GeodeticReferenceFrame {
        let angular = angular_unit(parts.angular_unit_name, parts.angular_unit_to_radian);
        let ellipsoid_name = parts.ellipsoid_name.unwrap_or("unnamed");
        let ellipsoid = if parts.inverse_flattening == 0.0 {
            Ellipsoid::create_two_axis(
                ellipsoid_name,
                parts.semi_major_metre,
                parts.semi_major_metre,
            )
        } else {
            Ellipsoid::create_flattened_sphere(
                ellipsoid_name,
                parts.semi_major_metre,
                parts.inverse_flattening,
            )
        };
        let meridian_name = match parts.prime_meridian_name {
            Some(name) => name.to_owned(),
            None if parts.prime_meridian_offset == 0.0 => "Greenwich".to_owned(),
            None => "unnamed".to_owned(),
        };
        GeodeticReferenceFrame::create(
            self.wkt1_datum_name(parts.datum_name),
            ellipsoid,
            PrimeMeridian {
                identified: Identified::named(meridian_name),
                longitude: Measure {
                    value: parts.prime_meridian_offset,
                    unit: angular,
                },
            },
        )
    }

    /// `createGeodeticReferenceFrame`: `WGS_1984` is EPSG:6326, and a WKT1
    /// datum name with `_` is the official name when an equivalent datum or
    /// alias exists.
    fn wkt1_datum_name(&self, datum_name: Option<&str>) -> String {
        let datum_name = datum_name.unwrap_or("unnamed");
        if datum_name == "WGS_1984" {
            return "World Geodetic System 1984".to_owned();
        }
        if !datum_name.contains('_') {
            return datum_name.to_owned();
        }
        let factory = AuthorityFactory::create(self, "");
        if let Ok(found) = factory.create_objects_from_name(
            datum_name,
            &[ObjectType::GeodeticReferenceFrame],
            NameMatch::Approximate,
            1,
        ) {
            if let Some(AuthorityObject::Datum(Datum::Geodetic(frame))) = found.first() {
                if equivalent_name(datum_name, frame.name()) {
                    return frame.name().to_owned();
                }
                if let Some(identifier) = frame.identifiers().first() {
                    let aliases = self.get_aliases(
                        identifier.code_space(),
                        identifier.code(),
                        frame.name(),
                        "geodetic_datum",
                        "",
                    );
                    if aliases
                        .iter()
                        .any(|alias| equivalent_name(datum_name, alias))
                    {
                        return frame.name().to_owned();
                    }
                }
            }
        }
        // Approximate search does not fold accents. `getOfficialNameFromAlias`
        // does, which is how the Ferro WKT1 spelling is found.
        if let Ok(Some(official)) = factory.official_name_from_alias(
            datum_name,
            Some("geodetic_datum"),
            None,
            NameSpelling::Equivalent,
        ) {
            return official.name;
        }
        datum_name.to_owned()
    }
}

/// Ellipsoid, datum, and prime meridian of `proj_create_geographic_crs`.
///
/// A missing name is the C API's `NULL`.
#[derive(Clone, Copy, Debug)]
pub struct GeodeticComponents<'a> {
    /// CRS name.
    pub crs_name: Option<&'a str>,
    /// Geodetic datum name.
    pub datum_name: Option<&'a str>,
    /// Ellipsoid name.
    pub ellipsoid_name: Option<&'a str>,
    /// Semi-major axis, metres.
    pub semi_major_metre: f64,
    /// Inverse flattening. `0` is a sphere.
    pub inverse_flattening: f64,
    /// Prime meridian name.
    pub prime_meridian_name: Option<&'a str>,
    /// Prime meridian longitude, in `angular_unit_name`.
    pub prime_meridian_offset: f64,
    /// Angular unit of the prime meridian. Missing means degree.
    pub angular_unit_name: Option<&'a str>,
    /// Factor from that angular unit to radians. Ignored for degree and grad.
    pub angular_unit_to_radian: f64,
}

fn object_name(name: Option<&str>) -> (String, bool) {
    let name = name.unwrap_or("unnamed");
    match name.strip_suffix(" (deprecated)") {
        Some(stripped) => (stripped.to_owned(), true),
        None => (name.to_owned(), false),
    }
}

fn angular_unit(name: Option<&str>, factor: f64) -> UnitOfMeasure {
    match name {
        None => UnitOfMeasure::degree(),
        Some(name) if name.eq_ignore_ascii_case("degree") => UnitOfMeasure::degree(),
        Some(name) if name.eq_ignore_ascii_case("grad") => UnitOfMeasure::grad(),
        Some(name) => UnitOfMeasure::new(name, factor, UnitKind::Angular),
    }
}

fn linear_unit(name: Option<&str>, factor: f64) -> UnitOfMeasure {
    match name {
        None => UnitOfMeasure::metre(),
        Some(name) if name.eq_ignore_ascii_case("metre") || name.eq_ignore_ascii_case("meter") => {
            UnitOfMeasure::metre()
        }
        Some(name) => UnitOfMeasure::new(name, factor, UnitKind::Linear),
    }
}

fn license_flag(primary: &Cell, fallback: &Cell) -> bool {
    if primary.empty() {
        fallback.text() == "1"
    } else {
        primary.text() == "1"
    }
}

/// File, package, and download record of one grid in `proj.db`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridInfo {
    /// Path of the file on the search path. Empty when the file is absent.
    pub full_name: String,
    /// Package that ships the file. Empty when `grid_packages` has no row.
    pub package_name: String,
    /// Download URL. Empty when the database has none.
    pub url: String,
    /// The file can be downloaded directly.
    pub direct_download: bool,
    /// The file is under an open licence.
    pub open_license: bool,
    /// The file is on the local search path.
    pub available: bool,
}

/// One grid an operation needs, as PROJ's `GridDescription` from `gridsNeeded`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridDescription {
    /// The grid name the operation gives, as written in its file parameter.
    pub short_name: String,
    /// The file, package, and download record of the grid.
    pub info: GridInfo,
}

impl Crs {
    /// CRS objects that replace this deprecated CRS, as `CRS::getNonDeprecated`.
    ///
    /// # Errors
    ///
    /// The database rejects the lookup, or a replacement code cannot be built.
    pub fn non_deprecated(&self, db: &DatabaseContext) -> Result<Vec<Self>, FactoryError> {
        let table = match self {
            Self::Geographic(_) | Self::Geodetic(_) => "geodetic_crs",
            Self::Projected(_) => "projected_crs",
            Self::Vertical(_) => "vertical_crs",
            Self::Compound(_) => "compound_crs",
            Self::Bound(_)
            | Self::Engineering(_)
            | Self::DerivedProjected(_)
            | Self::Temporal(_)
            | Self::Parametric(_)
            | Self::Derived(_) => {
                return Ok(Vec::new());
            }
        };
        let Some(identifier) = self.identifiers().first() else {
            return Ok(Vec::new());
        };
        let pairs = db.non_deprecated_codes(table, identifier.code_space(), identifier.code())?;
        let mut replacements = Vec::with_capacity(pairs.len());
        for (authority, code) in pairs {
            let factory = AuthorityFactory::create(db, authority);
            replacements.push(factory.create_coordinate_reference_system(&code)?);
        }
        Ok(replacements)
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

/// How [`AuthorityFactory::official_name_from_alias`] compares an alias with `alt_name`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameSpelling {
    /// The alias is the stored `alt_name`, character for character.
    Exact,
    /// Case, spaces, and underscores are ignored, as [`equivalent_name`] does.
    Equivalent,
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

/// Whether a name search keeps only deprecated rows, as a ` (deprecated)` suffix asks.
#[derive(Clone, Copy, PartialEq, Eq)]
enum DeprecatedFilter {
    Any,
    Only,
}

trait NamedAuthorityObject: Sized {
    fn name(&self) -> &str;
    fn identifiers(&self) -> &[Identifier];
    /// The concrete PROJ class of the object, as `typeid` tells it apart.
    fn class(&self) -> u16;
}

impl NamedAuthorityObject for GeodeticCRS {
    fn name(&self) -> &str {
        GeodeticCRS::name(self)
    }

    fn identifiers(&self) -> &[Identifier] {
        GeodeticCRS::identifiers(self)
    }

    fn class(&self) -> u16 {
        geodetic_class(self)
    }
}

/// The class of a geodetic CRS: a geographic CRS and a geocentric one differ.
fn geodetic_class(crs: &GeodeticCRS) -> u16 {
    match crs {
        GeodeticCRS::Geographic(_) => 10,
        GeodeticCRS::Geocentric(_) => 11,
        GeodeticCRS::Spherical(_) => 12,
    }
}

impl NamedAuthorityObject for ProjectedCRS {
    fn name(&self) -> &str {
        ProjectedCRS::name(self)
    }

    fn identifiers(&self) -> &[Identifier] {
        ProjectedCRS::identifiers(self)
    }

    fn class(&self) -> u16 {
        13
    }
}

impl NamedAuthorityObject for VerticalCRS {
    fn name(&self) -> &str {
        VerticalCRS::name(self)
    }

    fn identifiers(&self) -> &[Identifier] {
        VerticalCRS::identifiers(self)
    }

    fn class(&self) -> u16 {
        14
    }
}

impl NamedAuthorityObject for CompoundCRS {
    fn name(&self) -> &str {
        CompoundCRS::name(self)
    }

    fn identifiers(&self) -> &[Identifier] {
        CompoundCRS::identifiers(self)
    }

    fn class(&self) -> u16 {
        15
    }
}

impl NamedAuthorityObject for GeodeticReferenceFrame {
    fn name(&self) -> &str {
        GeodeticReferenceFrame::name(self)
    }

    fn identifiers(&self) -> &[Identifier] {
        GeodeticReferenceFrame::identifiers(self)
    }

    fn class(&self) -> u16 {
        2
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
            Self::PrimeMeridian(object) => object.name(),
            Self::Operation(object) => object.name(),
            Self::Extent(_) | Self::Unit(_) => "",
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
            Self::PrimeMeridian(object) => object.identifiers(),
            Self::Operation(object) => object.identifiers(),
            Self::Extent(_) | Self::Unit(_) => &[],
        }
    }

    fn class(&self) -> u16 {
        match self {
            Self::Ellipsoid(_) => 1,
            Self::Datum(Datum::Geodetic(_)) => 2,
            Self::Datum(Datum::Vertical(_)) => 3,
            Self::Datum(Datum::Engineering(_)) => 4,
            Self::DatumEnsemble(_) => 5,
            Self::PrimeMeridian(_) => 6,
            Self::Crs(Crs::Geographic(_)) => 10,
            Self::Crs(Crs::Geodetic(crs)) => geodetic_class(crs),
            Self::Crs(Crs::Projected(_)) => 13,
            Self::Crs(Crs::Vertical(_)) => 14,
            Self::Crs(Crs::Compound(_)) => 15,
            Self::Crs(Crs::Bound(_)) => 16,
            Self::Crs(Crs::Engineering(_)) => 17,
            Self::Crs(Crs::DerivedProjected(_)) => 18,
            Self::Crs(Crs::Temporal(_)) => 19,
            Self::Crs(Crs::Parametric(_)) => 23,
            Self::Crs(Crs::Derived(_)) => 24,
            Self::Operation(CoordinateOperation::Conversion(_)) => 20,
            Self::Operation(CoordinateOperation::Transformation(_)) => 21,
            Self::Operation(CoordinateOperation::Concatenated(_)) => 22,
            Self::Extent(_) => 30,
            Self::Unit(_) => 31,
        }
    }
}

/// The tables a name search reads, as `table` or `table:filter`. `filter` is
/// `ensemble`, `frame`, or a CRS type, as `createObjectsFromNameEx` picks them.
fn name_keys(types: &[ObjectType], name: &str) -> Vec<&'static str> {
    let hide_vertical = name.starts_with("D_");
    if types.is_empty() {
        let mut keys = vec![
            "prime_meridian",
            "ellipsoid",
            "geodetic_datum",
            "engineering_datum",
            "geodetic_crs",
            "projected_crs",
            "derived_projected_crs",
            "vertical_crs",
            "compound_crs",
            "engineering_crs",
            "conversion",
            "helmert_transformation",
            "grid_transformation",
            "other_transformation",
            "concatenated_operation",
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
    want(ObjectType::PrimeMeridian, &["prime_meridian"]);
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
    want(
        ObjectType::CoordinateOperation,
        &[
            "conversion",
            "helmert_transformation",
            "grid_transformation",
            "other_transformation",
            "concatenated_operation",
        ],
    );
    want(ObjectType::Conversion, &["conversion"]);
    want(
        ObjectType::Transformation,
        &[
            "helmert_transformation",
            "grid_transformation",
            "other_transformation",
        ],
    );
    want(
        ObjectType::ConcatenatedOperation,
        &["concatenated_operation"],
    );
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
        ("prime_meridian", _) => {
            AuthorityObject::PrimeMeridian(factory.create_prime_meridian(code)?)
        }
        ("conversion", _) => AuthorityObject::Operation(CoordinateOperation::Conversion(
            factory.create_conversion(code)?,
        )),
        (
            "helmert_transformation"
            | "grid_transformation"
            | "other_transformation"
            | "concatenated_operation",
            _,
        ) => AuthorityObject::Operation(
            factory.create_coordinate_operation(code, AlternativeGridNames::Proj)?,
        ),
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

/// `Identifier::canonicalizeName` as the approximate name match compares it:
/// lower case, separators dropped, ` + ` dropped, and `19dd` read as `dd`, so
/// `ITRF1993` matches `ITRF93`.
fn canonical_search_name(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let at = |index: usize| chars.get(index).copied().unwrap_or('\0');
    let word_at = |index: usize, word: &str| {
        word.chars()
            .enumerate()
            .all(|(offset, expected)| at(index + offset).to_ascii_lowercase() == expected)
    };
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i].to_ascii_lowercase();
        if ch == ' ' && at(i + 1) == '+' && at(i + 2) == ' ' {
            i += 3;
            continue;
        }
        // "19dd" is canonicalized as "dd".
        if ch == '1'
            && out
                .chars()
                .last()
                .is_some_and(|last| !last.is_ascii_digit())
            && at(i + 1) == '9'
            && at(i + 2).is_ascii_digit()
            && at(i + 3).is_ascii_digit()
        {
            i += 2;
            continue;
        }
        // The bigger differences of `Identifier::canonicalizeName`: "zone" and "height" after a
        // separator are dropped; "north" and "south" after a separator or a digit are shortened
        // to their first letter.
        let after_separator = i > 0 && is_ignored_search_char(chars[i - 1]);
        if after_separator && (word_at(i, "zone") || word_at(i, "height")) {
            i += if word_at(i, "zone") { 4 } else { 6 };
            continue;
        }
        let after_separator_or_digit = after_separator || (i > 0 && chars[i - 1].is_ascii_digit());
        if after_separator_or_digit && (word_at(i, "north") || word_at(i, "south")) {
            out.push(ch);
            i += 5;
            continue;
        }
        if word_at(i, "_intlfeet") && at(i + 9) == '\0' {
            out.push_str("feet");
            break;
        }
        if !is_ignored_search_char(ch) {
            out.push(ch);
        }
        i += 1;
    }
    out
}

/// `isIgnoredChar` of `Identifier::canonicalizeName`.
fn is_ignored_search_char(ch: char) -> bool {
    matches!(ch, ' ' | '_' | '-' | '/' | '(' | ')' | '.' | '&' | ',')
}

fn approximate_name(candidate: &str, searched: &str, canonical_searched: &str) -> bool {
    candidate
        .to_ascii_lowercase()
        .contains(&searched.to_ascii_lowercase())
        || canonical_search_name(candidate).contains(canonical_searched)
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
pub(super) enum Cell {
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

    pub(super) fn text(&self) -> String {
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
            "parametric" => UnitKind::Parametric,
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
        if authority_restricts(&self.authority) {
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
        if authority_restricts(&self.authority) {
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

    /// Projected CRS whose official name or alias matches `name`, each with the matched name
    /// (`createObjectsFromNameEx` for `PROJECTED_CRS`).
    pub(crate) fn create_projected_crs_by_name_matched(
        &self,
        name: &str,
        mode: NameMatch,
    ) -> Result<Vec<(ProjectedCRS, String)>, FactoryError> {
        self.objects_from_name_matched(name, &["projected_crs"], mode, 0, |factory, row| {
            factory.create_projected_crs(&row.code)
        })
    }

    pub(crate) fn create_vertical_crs_by_name(
        &self,
        name: &str,
        mode: NameMatch,
    ) -> Result<Vec<VerticalCRS>, FactoryError> {
        self.objects_from_name(name, &["vertical_crs"], mode, 0, |factory, row| {
            factory.create_vertical_crs(&row.code)
        })
    }

    /// Compound CRS whose official name or alias matches `name`, as the
    /// `createObjectsFromName` call of `CompoundCRS::identify` returns them.
    pub(crate) fn create_compound_crs_by_name(
        &self,
        name: &str,
        mode: NameMatch,
    ) -> Result<Vec<CompoundCRS>, FactoryError> {
        self.objects_from_name(name, &["compound_crs"], mode, 0, |factory, row| {
            factory.create_compound_crs(&row.code)
        })
    }

    /// `createProjectedCRSFromExisting` (factory.cpp:10149): the projected CRS of this
    /// factory that `crs` describes.
    ///
    /// The candidates have the same base CRS (identified by this factory, with the other axis
    /// order too for a geographic base), the EPSG method of `crs`, and parameter values within
    /// one unit of those of `crs`. Among them, the ones whose name is equivalent to the name of
    /// `crs` are returned. Without one, the projected CRS without a conversion whose WKT text
    /// names the method, the semi-major axis, and the parameters of `crs` are returned, each
    /// set of rows only when it holds 200 rows at most.
    pub(crate) fn create_projected_crs_from_existing(
        &self,
        crs: &ProjectedCRS,
    ) -> Vec<ProjectedCRS> {
        let conversion = crs.deriving_conversion();
        let method = conversion.method();
        let method_code = epsg_code_of_identifiers(method.identifiers());
        if method_code == 0 {
            return Vec::new();
        }
        let base = crs.base_crs();
        let mut candidates = Crs::Geodetic(base.clone()).identify_with(Some(self));
        if let GeodeticCRS::Geographic(geographic) = base {
            if let Some(other) = geographic_with_other_axis_order(geographic) {
                candidates.extend(Crs::Geographic(other).identify_with(Some(self)));
            }
        }
        let base_axis_unit = base.coordinate_system().axes()[0].unit().clone();
        let Some(base_ellipsoid) = base.ellipsoid() else {
            return Vec::new();
        };
        let semi_major = base_ellipsoid.semi_major_axis().value();

        let mut sql = "SELECT projected_crs.auth_name, projected_crs.code, \
             projected_crs.name FROM projected_crs \
             JOIN conversion_table conv ON \
             projected_crs.conversion_auth_name = conv.auth_name AND \
             projected_crs.conversion_code = conv.code \
             JOIN geodetic_crs gcrs ON \
             gcrs.auth_name = projected_crs.geodetic_crs_auth_name AND \
             gcrs.code = projected_crs.geodetic_crs_code \
             JOIN geodetic_datum datum ON \
             datum.auth_name = gcrs.datum_auth_name AND \
             datum.code = gcrs.datum_code \
             JOIN ellipsoid ellps ON \
             ellps.auth_name = datum.ellipsoid_auth_name AND \
             ellps.code = datum.ellipsoid_code \
             WHERE \
             abs(ellps.semi_major_axis - ?) <= 1e-4 * ellps.semi_major_axis AND \
             projected_crs.deprecated = 0 AND "
            .to_owned();
        let mut params = vec![format_g(semi_major, 15)];
        if !candidates.is_empty() {
            sql.push_str(&auth_name_code_in(
                &candidates,
                &mut params,
                "projected_crs.geodetic_crs_",
            ));
            sql.push_str(" AND ");
        }
        sql.push_str("conv.method_auth_name = 'EPSG' AND conv.method_code = ?");
        params.push(method_code.to_string());
        if authority_restricts(&self.authority) {
            sql.push_str(" AND projected_crs.auth_name = ?");
            params.push(self.authority.clone());
        }

        // The parameter values, read in order. The loop stops at the first value that is not
        // an EPSG-coded measure, as PROJ does.
        let mut has_lat_1st = false;
        let mut lat_1st = 0.0;
        let mut param_lat_1st = 0;
        let mut has_lat_2nd = false;
        let mut lat_2nd = 0.0;
        let mut param_lat_2nd = 0;
        for (index, parameter) in conversion.parameters().iter().enumerate() {
            let i_param = index + 1;
            let param_code = epsg_code_of_parameter(parameter);
            let ParameterValue::Measure(measure) = parameter.value() else {
                break;
            };
            if param_code <= 0 {
                break;
            }
            if same_unit(measure.unit(), &UnitOfMeasure::degree())
                && same_unit(measure.unit(), &base_axis_unit)
            {
                if method_code == EPSG_METHOD_LCC_2SP {
                    if param_code == EPSG_PARAMETER_LAT_1ST_STD_PARALLEL {
                        has_lat_1st = true;
                        lat_1st = measure.value();
                        param_lat_1st = i_param;
                        continue;
                    } else if param_code == EPSG_PARAMETER_LAT_2ND_STD_PARALLEL {
                        has_lat_2nd = true;
                        lat_2nd = measure.value();
                        param_lat_2nd = i_param;
                        continue;
                    }
                }
                sql.push_str(&format!(
                    " AND conv.param{i_param}_code = ? AND conv.param{i_param}_auth_name = 'EPSG' \
                     AND conv.param{i_param}_value BETWEEN ? AND ?"
                ));
                // As angles might be expressed with the odd unit EPSG:9110 "sexagesimal DMS",
                // the range is broad.
                params.push(param_code.to_string());
                params.push((measure.value() - 1.0).to_string());
                params.push((measure.value() + 1.0).to_string());
            }
        }
        // The two standard parallels of LCC 2SP may be switched.
        if method_code == EPSG_METHOD_LCC_2SP && has_lat_1st && has_lat_2nd {
            sql.push_str(&format!(
                " AND conv.param{param_lat_1st}_code = ? AND conv.param{param_lat_1st}_auth_name = 'EPSG' \
                 AND conv.param{param_lat_2nd}_code = ? AND conv.param{param_lat_2nd}_auth_name = 'EPSG' AND (("
            ));
            params.push(EPSG_PARAMETER_LAT_1ST_STD_PARALLEL.to_string());
            params.push(EPSG_PARAMETER_LAT_2ND_STD_PARALLEL.to_string());
            let (mut value_1, mut value_2) = (lat_1st, lat_2nd);
            for alternative in 0..2 {
                if alternative == 1 {
                    sql.push_str(") OR (");
                    std::mem::swap(&mut value_1, &mut value_2);
                }
                sql.push_str(&format!(
                    "conv.param{param_lat_1st}_value BETWEEN ? AND ? AND \
                     conv.param{param_lat_2nd}_value BETWEEN ? AND ?"
                ));
                params.push((value_1 - 1.0).to_string());
                params.push((value_1 + 1.0).to_string());
                params.push((value_2 - 1.0).to_string());
                params.push((value_2 + 1.0).to_string());
            }
            sql.push_str("))");
        }
        let mut res = Vec::new();
        let Ok(rows) = self.db.text_rows(&sql, &params) else {
            return res;
        };
        for row in &rows {
            if equivalent_name_bigger_differences(crs.name(), &row[2]) {
                if let Ok(found) = self.child(&row[0]).create_projected_crs(&row[1]) {
                    res.push(found);
                }
            }
        }
        if !res.is_empty() {
            return res;
        }

        // No candidate has the name: the projected CRS without a conversion row whose WKT
        // text names the method, the semi-major axis, and the parameters.
        let mut params = Vec::new();
        let mut sql = "SELECT auth_name, code FROM projected_crs WHERE \
             deprecated = 0 AND conversion_auth_name IS NULL AND "
            .to_owned();
        if !candidates.is_empty() {
            sql.push_str(&auth_name_code_in(
                &candidates,
                &mut params,
                "geodetic_crs_",
            ));
            sql.push_str(" AND ");
        }
        let semi_major_text = format_g(base_ellipsoid.semi_major_axis().si_value(), 10);
        sql.push_str("(text_definition LIKE ? ESCAPE '\\'");
        // WKT2 definition.
        params.push(format!(
            "%,{semi_major_text}%{}%",
            escape_like(method.name())
        ));
        let mapping = projection_mapping(method_code, method.name());
        if let Some(proj_name) = mapping.and_then(|mapping| mapping.proj_name) {
            sql.push_str(" OR (text_definition LIKE ? AND (text_definition LIKE ?");
            params.push(format!("%proj={proj_name}%"));
            params.push(format!("%={semi_major_text}%"));
            if let Some(proj_ellps) = look_for_proj_well_known_ellps(base_ellipsoid) {
                sql.push_str(" OR text_definition LIKE ?");
                params.push(format!("%={proj_ellps}%"));
            }
            sql.push_str("))");
        }
        // WKT1 GDAL definition.
        if let Some(gdal_name) = wkt1_gdal_method_name(method_code, method.name()) {
            sql.push_str(" OR text_definition LIKE ? ESCAPE '\\'");
            params.push(format!("%,{semi_major_text}%{}%", escape_like(gdal_name)));
        }
        // WKT1 ESRI definition.
        if let Some(esri) = esri_method(conversion, method.name(), method_code) {
            sql.push_str(" OR text_definition LIKE ? ESCAPE '\\'");
            let mut pattern = format!("%,{semi_major_text}%{}%", escape_like(esri.name));
            let false_easting = conversion
                .measure_of_code(EPSG_PARAMETER_FALSE_EASTING)
                .or_else(|| conversion.measure_of_code(EPSG_PARAMETER_EASTING_FALSE_ORIGIN));
            if let Some(false_easting) = false_easting {
                pattern.push_str(&format!(
                    "PARAMETER[\"False\\_Easting\",{}%",
                    format_g(
                        false_easting.convert_to(crs.coordinate_system().axes()[0].unit()),
                        10
                    )
                ));
            }
            let latitude = conversion_measure_by_name(conversion, "Latitude of natural origin")
                .or_else(|| conversion_measure_by_name(conversion, "Latitude of false origin"));
            if let Some(latitude) = latitude {
                let angular = if matches!(base, GeodeticCRS::Geographic(_)) {
                    base_axis_unit
                } else {
                    UnitOfMeasure::degree()
                };
                pattern.push_str(&format!(
                    "PARAMETER[\"Latitude\\_Of\\_Origin\",{}%",
                    format_g(latitude.convert_to(&angular), 10)
                ));
            }
            params.push(pattern);
        }
        sql.push(')');
        if authority_restricts(&self.authority) {
            sql.push_str(" AND auth_name = ?");
            params.push(self.authority.clone());
        }
        let rows_without_conversion = self.db.text_rows(&sql, &params).unwrap_or_default();

        if rows.len() <= 200 {
            for row in &rows {
                if let Ok(found) = self.child(&row[0]).create_projected_crs(&row[1]) {
                    res.push(found);
                }
            }
        }
        if rows_without_conversion.len() <= 200 {
            for row in &rows_without_conversion {
                if let Ok(found) = self.child(&row[0]).create_projected_crs(&row[1]) {
                    res.push(found);
                }
            }
        }
        res
    }

    /// `createCompoundCRSFromExisting`: the compound CRS of this factory whose horizontal
    /// and vertical components are identified, one by one, as the components of `crs`.
    /// A compound CRS with other than two components has none.
    pub(crate) fn create_compound_crs_from_existing(&self, crs: &CompoundCRS) -> Vec<CompoundCRS> {
        let components = crs.components();
        if components.len() != 2 {
            return Vec::new();
        }
        let horizontal = components[0].identify_with(Some(self));
        let vertical = components[1].identify_with(Some(self));
        if horizontal.is_empty() && vertical.is_empty() {
            return Vec::new();
        }
        let mut sql =
            "SELECT auth_name, code FROM compound_crs WHERE deprecated = 0 AND ".to_owned();
        let mut params = Vec::new();
        let mut add_and = false;
        if !horizontal.is_empty() {
            sql.push_str(&auth_name_code_in(&horizontal, &mut params, "horiz_crs_"));
            add_and = true;
        }
        if !vertical.is_empty() {
            if add_and {
                sql.push_str(" AND ");
            }
            sql.push_str(&auth_name_code_in(&vertical, &mut params, "vertical_crs_"));
            add_and = true;
        }
        if authority_restricts(&self.authority) {
            if add_and {
                sql.push_str(" AND ");
            }
            sql.push_str("auth_name = ?");
            params.push(self.authority.clone());
        }
        let Ok(rows) = self.db.text_rows(&sql, &params) else {
            return Vec::new();
        };
        rows.into_iter()
            .filter_map(|row| self.child(&row[0]).create_compound_crs(&row[1]).ok())
            .collect()
    }

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
        if authority_restricts(&self.authority) {
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
        Ok(self
            .objects_from_name_matched(name, tables, mode, limit, load)?
            .into_iter()
            .map(|(object, _)| object)
            .collect())
    }

    /// [`Self::objects_from_name`] with the name each object matched, as the second element of
    /// PROJ's `createObjectsFromNameEx` pair: its official name, or the alias that matched.
    fn objects_from_name_matched<T>(
        &self,
        name: &str,
        tables: &[&str],
        mode: NameMatch,
        limit: usize,
        load: impl Fn(&Self, &NameRow) -> Result<T, FactoryError>,
    ) -> Result<Vec<(T, String)>, FactoryError>
    where
        T: NamedAuthorityObject,
    {
        let (name, deprecated) = match name.strip_suffix(" (deprecated)") {
            Some(stripped) => (stripped, DeprecatedFilter::Only),
            None => (name, DeprecatedFilter::Any),
        };
        let canonical = canonical_search_name(name);
        if canonical.chars().count() <= 1 {
            return Ok(Vec::new());
        }
        let rows = self.name_rows(tables, name, mode, limit, deprecated)?;
        let mut seen = BTreeSet::new();
        let mut built: Vec<BuiltObject<T>> = Vec::new();
        let mut first = true;
        let mut first_deprecated = false;
        let mut exact_official = 0usize;
        let mut exact_alias = 0usize;
        let mut first_class: Option<u16> = None;
        let mut one_class = true;
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
            let class = object.class();
            if *first_class.get_or_insert(class) != class {
                one_class = false;
            }
            built.push(BuiltObject {
                matched_name: row.matched_name,
                object,
            });
            if limit > 0 && built.len() == limit {
                break;
            }
        }
        if mode == NameMatch::Exact && exact_official + exact_alias >= 1 && one_class {
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
        Ok(built
            .into_iter()
            .map(|item| (item.object, item.matched_name))
            .collect())
    }

    fn name_rows(
        &self,
        keys: &[&str],
        name: &str,
        mode: NameMatch,
        limit: usize,
        deprecated: DeprecatedFilter,
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
            if deprecated == DeprecatedFilter::Only {
                sql.push_str("AND deprecated = 1 ");
            }
            if mode == NameMatch::Exact {
                sql.push_str("AND name = ? COLLATE NOCASE ");
                params.push(name.to_owned());
            }
            if authority_restricts(&self.authority) {
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
            if deprecated == DeprecatedFilter::Only {
                sql.push_str("AND ov.deprecated = 1 ");
            }
            if mode == NameMatch::Exact {
                sql.push_str("AND a.alt_name = ? COLLATE NOCASE ");
                params.push(name.to_owned());
            }
            if authority_restricts(&self.authority) {
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
            source: None,
            target: None,
            version: None,
            inverted: false,
            interpolation: None,
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
            implicit_cs: false,
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
        if self.authority == "OGC" && code == "84" {
            return self.create_coordinate_reference_system("CRS84");
        }
        if self.authority == "OGC" {
            if let Some(temporal) = ogc_temporal_crs(code) {
                return Ok(Crs::Temporal(temporal));
            }
        }
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

    /// `createCoordinateMetadata(code)`: the CRS of the `coordinate_metadata` row, with its epoch.
    ///
    /// # Errors
    ///
    /// The code is missing, or its CRS or epoch cannot be built.
    pub fn create_coordinate_metadata(
        &self,
        code: &str,
    ) -> Result<CoordinateMetadata, FactoryError> {
        let row = self
            .one(
                "SELECT crs_auth_name, crs_code, crs_text_definition, coordinate_epoch \
                 FROM coordinate_metadata WHERE auth_name = ? AND code = ?",
                code,
            )
            .map_err(|_| FactoryError::missing("CoordinateMetadata", &self.authority, code))?;
        let build = || -> Result<CoordinateMetadata, FactoryError> {
            let crs = if row[0].empty() {
                create_from_user_input(&row[2].text(), &self.db)?
            } else {
                AuthorityFactory::create(&self.db, row[0].text())
                    .create_coordinate_reference_system(&row[1].text())?
            };
            if row[3].empty() {
                CoordinateMetadata::create_without_epoch(&crs)
            } else {
                CoordinateMetadata::create(&crs, row[3].f64()?, &self.db)
            }
        };
        build().map_err(|error| {
            FactoryError::wrap(
                "CoordinateMetadata",
                &self.authority,
                code,
                error.to_string(),
            )
        })
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
        self.create_from_coordinate_reference_system_codes_in(
            (self.authority.as_str(), source_code),
            (self.authority.as_str(), target_code),
            SupersededOperations::Keep,
        )
    }

    /// Operations registered from the CRS `source` to the CRS `target`, each given as an
    /// authority name and a code. The factory authority, when it is not empty, restricts the
    /// operations. `superseded` chooses whether an operation replaced by a listed one is kept.
    ///
    /// # Errors
    ///
    /// A selected operation cannot be instantiated.
    pub fn create_from_coordinate_reference_system_codes_in(
        &self,
        source: (&str, &str),
        target: (&str, &str),
        superseded: SupersededOperations,
    ) -> Result<Vec<CoordinateOperation>, FactoryError> {
        if let Some(conversion) = self.projected_deriving_conversion(source, target) {
            return Ok(vec![CoordinateOperation::Conversion(conversion)]);
        }
        operations_between(
            &self.db,
            &self.authority,
            source,
            target,
            (None, None),
            match superseded {
                SupersededOperations::Keep => RegistrySearch::Forward,
                SupersededOperations::Discard => RegistrySearch::ForwardWithoutSuperseded,
            },
            AlternativeGridNames::Official,
        )
    }

    /// The point motion operations registered for the datum of `crs`, in database order.
    ///
    /// A point motion operation has the same CRS as source and target.
    ///
    /// # Errors
    ///
    /// The database rejects a statement or an operation cannot be instantiated.
    pub fn get_point_motion_operations_for(
        &self,
        crs: &GeodeticCRS,
        grids: AlternativeGridNames,
    ) -> Result<Vec<Transformation>, FactoryError> {
        let datum_id = crs
            .datum()
            .and_then(|datum| datum.identifiers().first())
            .or_else(|| {
                crs.datum_ensemble()
                    .and_then(|ensemble| ensemble.identifiers().first())
            });
        let by_name = match (datum_id, crs.datum()) {
            (None, Some(frame)) => Self::datum_by_name(&self.db, frame.name()),
            _ => None,
        };
        let datum_id = match (datum_id, &by_name) {
            (Some(id), _) => (id.code_space().to_owned(), id.code().to_owned()),
            (None, Some(found)) => found.clone(),
            (None, None) => return Ok(Vec::new()),
        };
        let candidates = AuthorityFactory::create(&self.db, "").create_geodetic_crs_from_datum(
            &datum_id.0,
            &datum_id.1,
            "",
        )?;
        let mut clauses = Vec::new();
        let mut params = Vec::new();
        for candidate in &candidates {
            let Some(id) = candidate.identifiers().first() else {
                continue;
            };
            clauses.push("(source_crs_auth_name = ? AND source_crs_code = ?)");
            params.push(id.code_space().to_owned());
            params.push(id.code().to_owned());
        }
        if clauses.is_empty() {
            return Ok(Vec::new());
        }
        let mut sql = format!(
            "SELECT auth_name, code FROM coordinate_operation_view \
             WHERE source_crs_auth_name = target_crs_auth_name AND \
             source_crs_code = target_crs_code AND deprecated = 0 AND ({})",
            clauses.join(" OR ")
        );
        if authority_restricts(&self.authority) {
            sql.push_str(" AND auth_name = ?");
            params.push(self.authority.clone());
        }
        let mut operations = Vec::new();
        for row in self.db.query(&sql, &params)? {
            let factory = AuthorityFactory::create(&self.db, row[0].text());
            if let CoordinateOperation::Transformation(transformation) =
                factory.create_coordinate_operation(&row[1].text(), grids)?
            {
                if transformation.is_point_motion() {
                    operations.push(transformation);
                }
            }
        }
        Ok(operations)
    }

    /// The identifier of the one registered geodetic datum named `name`, for a datum that has no
    /// identifier of its own (`createGeodeticCRSFromDatum` searches by name).
    fn datum_by_name(db: &DatabaseContext, name: &str) -> Option<(String, String)> {
        if name == "unknown" || name == "unnamed" || name.is_empty() {
            return None;
        }
        let matches = AuthorityFactory::create(db, "")
            .create_objects_from_name(
                name,
                &[ObjectType::GeodeticReferenceFrame],
                NameMatch::Exact,
                2,
            )
            .ok()?;
        let [AuthorityObject::Datum(Datum::Geodetic(found))] = matches.as_slice() else {
            return None;
        };
        found
            .identifiers()
            .first()
            .map(|id| (id.code_space().to_owned(), id.code().to_owned()))
    }

    /// The conversion of the projected CRS `target` when `source` is its base CRS, with the
    /// CRS pair set: `createFromCoordinateReferenceSystemCodes` answers with it before any search.
    fn projected_deriving_conversion(
        &self,
        source: (&str, &str),
        target: (&str, &str),
    ) -> Option<Conversion> {
        if target.0.is_empty() {
            return None;
        }
        let projected = AuthorityFactory::create(&self.db, target.0)
            .create_projected_crs(target.1)
            .ok()?;
        let base = projected.base_crs();
        let is_source = source.0.is_empty()
            || base
                .identifiers()
                .first()
                .is_some_and(|id| id.code_space() == source.0 && id.code() == source.1);
        if !is_source {
            return None;
        }
        let mut conversion = projected.deriving_conversion().clone();
        if authority_restricts(&self.authority)
            && conversion
                .identifiers()
                .first()
                .is_some_and(|id| id.code_space() != self.authority)
        {
            return None;
        }
        conversion.source = Some(Box::new(Crs::Geodetic(base.clone())));
        conversion.target = Some(Box::new(Crs::Projected(projected)));
        Some(conversion)
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
            WHERE (?1 = '' OR c.auth_name = ?1) \
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
            WHERE (?1 = '' OR c.auth_name = ?1) \
            UNION ALL SELECT c.auth_name, c.code, c.name, 'vertical', c.deprecated, \
            a.west_lon, a.south_lat, a.east_lon, a.north_lat, a.description, NULL, 'Earth' \
            FROM vertical_crs c \
            LEFT JOIN usage u ON u.object_table_name = 'vertical_crs' AND u.object_auth_name = c.auth_name \
            AND u.object_code = c.code \
            LEFT JOIN extent a ON a.auth_name = u.extent_auth_name AND a.code = u.extent_code \
            WHERE (?1 = '' OR c.auth_name = ?1) \
            UNION ALL SELECT c.auth_name, c.code, c.name, 'compound', c.deprecated, \
            a.west_lon, a.south_lat, a.east_lon, a.north_lat, a.description, NULL, 'Earth' \
            FROM compound_crs c \
            LEFT JOIN usage u ON u.object_table_name = 'compound_crs' AND u.object_auth_name = c.auth_name \
            AND u.object_code = c.code \
            LEFT JOIN extent a ON a.auth_name = u.extent_auth_name AND a.code = u.extent_code \
            WHERE (?1 = '' OR c.auth_name = ?1) \
            UNION ALL SELECT c.auth_name, c.code, c.name, 'engineering', c.deprecated, \
            a.west_lon, a.south_lat, a.east_lon, a.north_lat, a.description, NULL, 'Earth' \
            FROM engineering_crs c \
            LEFT JOIN usage u ON u.object_table_name = 'engineering_crs' AND u.object_auth_name = c.auth_name \
            AND u.object_code = c.code \
            LEFT JOIN extent a ON a.auth_name = u.extent_auth_name AND a.code = u.extent_code \
            WHERE (?1 = '' OR c.auth_name = ?1) \
            UNION ALL SELECT c.auth_name, c.code, c.name, 'derived projected', c.deprecated, \
            a.west_lon, a.south_lat, a.east_lon, a.north_lat, a.description, NULL, 'Earth' \
            FROM derived_projected_crs c \
            LEFT JOIN usage u ON u.object_table_name = 'derived_projected_crs' AND u.object_auth_name = c.auth_name \
            AND u.object_code = c.code \
            LEFT JOIN extent a ON a.auth_name = u.extent_auth_name AND a.code = u.extent_code \
            WHERE (?1 = '' OR c.auth_name = ?1)\
            ) r ORDER BY auth_name, code";
        let rows = self.db.query(sql, &[auth])?;
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

    /// Geoid models that apply to the vertical CRS `code`, by name, sorted.
    ///
    /// # Errors
    ///
    /// The database rejects the statement.
    pub fn get_geoid_models(&self, code: &str) -> Result<Vec<String>, FactoryError> {
        let mut sql = "SELECT DISTINCT GM0.name \
             FROM geoid_model GM0 \
             INNER JOIN grid_transformation GT0 \
             ON GT0.code = GM0.operation_code \
             AND GT0.auth_name = GM0.operation_auth_name \
             AND GT0.deprecated = 0 \
             INNER JOIN vertical_crs VC0 \
             ON VC0.code = GT0.target_crs_code \
             AND VC0.auth_name = GT0.target_crs_auth_name \
             INNER JOIN vertical_crs VC1 \
             ON VC1.datum_code = VC0.datum_code \
             AND VC1.datum_auth_name = VC0.datum_auth_name \
             AND VC1.code = ?"
            .to_owned();
        let mut params = vec![code.to_owned()];
        if authority_restricts(&self.authority) {
            sql.push_str(" AND GT0.target_crs_auth_name = ?");
            params.push(self.authority.clone());
        }
        sql.push_str(" ORDER BY 1");
        let rows = self.db.query(&sql, &params)?;
        Ok(rows.into_iter().map(|row| row[0].text()).collect())
    }

    /// Units of measure, in authority then code order.
    ///
    /// `category` is the PROJ category: `linear`, `angular`, or `scale`,
    /// with `_per_time` when the name says so. Other types are kept as stored.
    ///
    /// # Errors
    ///
    /// The database rejects the statement.
    pub fn get_unit_list(&self) -> Result<Vec<UnitInfo>, FactoryError> {
        let mut sql = String::from(
            "SELECT auth_name, code, name, type, conv_factor, proj_short_name, deprecated \
             FROM unit_of_measure",
        );
        let mut params = Vec::new();
        if authority_restricts(&self.authority) {
            sql.push_str(" WHERE auth_name = ?");
            params.push(self.authority.clone());
        }
        sql.push_str(" ORDER BY auth_name, code");
        let rows = self.db.query(&sql, &params)?;
        let mut list = Vec::with_capacity(rows.len());
        for row in rows {
            let name = row[2].text();
            let per_time = name.contains(" per ");
            let category = match row[3].text().as_str() {
                "length" if per_time => "linear_per_time".to_owned(),
                "length" => "linear".to_owned(),
                "angle" if per_time => "angular_per_time".to_owned(),
                "angle" => "angular".to_owned(),
                "scale" if name.contains(" per year") || name.contains(" per second") => {
                    "scale_per_time".to_owned()
                }
                "scale" => "scale".to_owned(),
                other => other.to_owned(),
            };
            list.push(UnitInfo {
                authority: row[0].text(),
                code: row[1].text(),
                name,
                category,
                conv_factor: if row[4].empty() { 0.0 } else { row[4].f64()? },
                proj_short_name: row[5].text(),
                deprecated: row[6].flag(),
            });
        }
        Ok(list)
    }

    /// Celestial bodies, in authority then name order.
    ///
    /// # Errors
    ///
    /// The database rejects the statement.
    pub fn get_celestial_body_list(&self) -> Result<Vec<CelestialBodyInfo>, FactoryError> {
        let mut sql = String::from("SELECT auth_name, name FROM celestial_body");
        let mut params = Vec::new();
        if authority_restricts(&self.authority) {
            sql.push_str(" WHERE auth_name = ?");
            params.push(self.authority.clone());
        }
        sql.push_str(" ORDER BY auth_name, name");
        let rows = self.db.query(&sql, &params)?;
        Ok(rows
            .into_iter()
            .map(|row| CelestialBodyInfo {
                authority: row[0].text(),
                name: row[1].text(),
            })
            .collect())
    }

    /// Official name of the object that `alias` names, or `None` when no alias matches.
    ///
    /// `table` and `source` narrow the search when non-empty. A source of
    /// `ESRI` also matches `ESRI_OLD`. The official name comes from the row
    /// with the smallest `deprecated` flag.
    ///
    /// # Errors
    ///
    /// The database rejects a statement.
    pub fn official_name_from_alias(
        &self,
        alias: &str,
        table: Option<&str>,
        source: Option<&str>,
        spelling: NameSpelling,
    ) -> Result<Option<OfficialName>, FactoryError> {
        let table = table.filter(|table| !table.is_empty());
        let source = source.filter(|source| !source.is_empty());
        match spelling {
            NameSpelling::Equivalent => {
                let mut sql =
                    String::from("SELECT table_name, auth_name, code, alt_name FROM alias_name");
                let mut params = Vec::new();
                match (table, source) {
                    (Some(table), Some(source)) => {
                        sql.push_str(" WHERE table_name = ? AND source = ?");
                        params.extend([table.to_owned(), source.to_owned()]);
                    }
                    (Some(table), None) => {
                        sql.push_str(" WHERE table_name = ?");
                        params.push(table.to_owned());
                    }
                    (None, Some(source)) => {
                        sql.push_str(" WHERE source = ?");
                        params.push(source.to_owned());
                    }
                    (None, None) => {}
                }
                let rows = self.db.query(&sql, &params)?;
                for row in rows {
                    if equivalent_name(&row[3].text(), alias) {
                        return self.official_for_alias_row(
                            &row[0].text(),
                            &row[1].text(),
                            &row[2].text(),
                        );
                    }
                }
                Ok(None)
            }
            NameSpelling::Exact => {
                let mut sql = String::from(
                    "SELECT table_name, auth_name, code FROM alias_name WHERE alt_name = ?",
                );
                let mut params = vec![alias.to_owned()];
                if let Some(table) = table {
                    sql.push_str(" AND table_name = ?");
                    params.push(table.to_owned());
                }
                match source {
                    Some("ESRI") => sql.push_str(" AND source IN ('ESRI', 'ESRI_OLD')"),
                    Some(source) => {
                        sql.push_str(" AND source = ?");
                        params.push(source.to_owned());
                    }
                    None => {}
                }
                let rows = self.db.query(&sql, &params)?;
                let mut best: Option<(bool, OfficialName)> = None;
                for row in rows {
                    let found =
                        self.official_rows(&row[0].text(), &row[1].text(), &row[2].text())?;
                    for (name, deprecated) in found {
                        match &best {
                            Some((rank, _)) if *rank <= deprecated => {}
                            _ => best = Some((deprecated, name)),
                        }
                    }
                }
                Ok(best.map(|(_, name)| name))
            }
        }
    }

    fn official_for_alias_row(
        &self,
        table: &str,
        auth_name: &str,
        code: &str,
    ) -> Result<Option<OfficialName>, FactoryError> {
        Ok(self
            .official_rows(table, auth_name, code)?
            .into_iter()
            .next()
            .map(|(name, _)| name))
    }

    /// Name row of `table` for `auth_name`:`code`, with its deprecated flag.
    fn official_rows(
        &self,
        table: &str,
        auth_name: &str,
        code: &str,
    ) -> Result<Vec<(OfficialName, bool)>, FactoryError> {
        let sql = format!(
            "SELECT name, deprecated FROM \"{}\" WHERE auth_name = ? AND code = ?",
            table.replace('"', "\"\"")
        );
        let rows = self
            .db
            .query(&sql, &[auth_name.to_owned(), code.to_owned()])?;
        Ok(rows
            .into_iter()
            .map(|row| {
                (
                    OfficialName {
                        name: row[0].text(),
                        table_name: table.to_owned(),
                        auth_name: auth_name.to_owned(),
                        code: code.to_owned(),
                    },
                    row[1].flag(),
                )
            })
            .collect())
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
            realization_method: None,
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

    /// `createCoordinateOperation` with `usePROJAlternativeGridNames`: a transformation built
    /// with the official grid names is replaced by `substitutePROJAlternativeGridNames` when
    /// that substitutes it (`proj_grid_substitution`). Otherwise it is built as the file rename
    /// of [`AlternativeGridNames::Proj`] gives it, as before.
    fn with_proj_grid_names(
        &self,
        grids: AlternativeGridNames,
        build: impl Fn(AlternativeGridNames) -> Result<CoordinateOperation, FactoryError>,
    ) -> Result<CoordinateOperation, FactoryError> {
        if grids != AlternativeGridNames::Proj {
            return build(grids);
        }
        let official = build(AlternativeGridNames::Official)?;
        let CoordinateOperation::Transformation(transformation) = &official else {
            return build(AlternativeGridNames::Proj);
        };
        match super::proj_grid_substitution::substitute_proj_alternative_grid_names(
            transformation,
            &self.db,
        )? {
            Some(replacement) => Ok(CoordinateOperation::Transformation(replacement)),
            None => build(AlternativeGridNames::Proj),
        }
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
            "grid_transformation" => self.with_proj_grid_names(grids, |names| {
                self.grid_transformation(code, names)
                    .map(CoordinateOperation::Transformation)
            }),
            "other_transformation" => {
                self.with_proj_grid_names(grids, |names| self.other_transformation(code, names))
            }
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
             interpolation_crs_auth_name, interpolation_crs_code, operation_version, deprecated, \
             param1_auth_name, param1_code, param1_name, param1_value, param1_uom_auth_name, param1_uom_code, \
             param2_auth_name, param2_code, param2_name, param2_value, param2_uom_auth_name, param2_uom_code \
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
        parameters.extend(self.parameter_block(&row, 22, 2)?);
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
                source: None,
                target: None,
                version: transformation.version,
                inverted: false,
                interpolation: None,
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
        let mut explicit_direction = 0_usize;
        for step in steps_rows {
            let direction = step[2].text();
            if direction == "forward" || direction == "reverse" {
                explicit_direction += 1;
            }
            let operation = self
                .child(&step[0].text())
                .create_coordinate_operation(&step[1].text(), grids)?;
            steps.push(ConcatenatedStep {
                operation,
                reverse: direction == "reverse",
            });
        }
        if explicit_direction > 0 && explicit_direction != steps.len() {
            return Err(FactoryError::Invalid(format!(
                "not all steps have a defined direction for concatenated operation {code}"
            )));
        }
        let source = self
            .child(&row[2].text())
            .create_coordinate_reference_system(&row[3].text())?;
        let target = self
            .child(&row[4].text())
            .create_coordinate_reference_system(&row[5].text())?;
        let accuracy = accuracy_of(&row[6]);
        if explicit_direction == 0 {
            fix_unspecified_step_directions(&mut steps, &source, &target);
            for index in 0..steps.len() {
                super::wkt_import::fix_transformation_direction(&source, &mut steps, index);
            }
        } else {
            assign_missing_conversion_crs(&mut steps, &source, &target);
        }
        append_geographic_geocentric_tail(&mut steps, &target);
        super::wkt_import::check_steps_chaining(&steps, &source, &target)?;
        let interpolation = concatenation_interpolation_of_steps(&steps);
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
            computed_name: false,
            source_epoch: None,
            target_epoch: None,
            proj_based: None,
            interpolation,
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
            accuracy_text: accuracy_of(accuracy).and_then(|_| accuracy_text(accuracy)),
            version: optional_text(version),
            point_motion: motion == PointMotion::Yes,
            ballpark: false,
            source_epoch: None,
            target_epoch: None,
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

impl CrsInfo {
    /// The object type PROJ reports as `info.type`. A geodetic CRS that is
    /// neither geographic nor geocentric (`other` in the database) is `GeodeticCrs`.
    #[must_use]
    pub fn object_type(&self) -> ObjectType {
        match self.type_name.as_str() {
            "geographic 2D" => ObjectType::Geographic2dCrs,
            "geographic 3D" => ObjectType::Geographic3dCrs,
            "geocentric" => ObjectType::GeocentricCrs,
            "projected" => ObjectType::ProjectedCrs,
            "vertical" => ObjectType::VerticalCrs,
            "compound" => ObjectType::CompoundCrs,
            "engineering" => ObjectType::EngineeringCrs,
            "derived projected" => ObjectType::DerivedProjectedCrs,
            _ => ObjectType::GeodeticCrs,
        }
    }
}

/// One row of `unit_of_measure`, as `AuthorityFactory::getUnitList` reports it.
#[derive(Clone, Debug, PartialEq)]
pub struct UnitInfo {
    /// Authority name.
    pub authority: String,
    /// Code.
    pub code: String,
    /// Name.
    pub name: String,
    /// `linear`, `linear_per_time`, `angular`, `angular_per_time`, `scale`,
    /// or `scale_per_time`; other stored types are kept as they are.
    pub category: String,
    /// Conversion factor to the SI unit, or 0 when the database has none.
    pub conv_factor: f64,
    /// Short name used by PROJ.
    pub proj_short_name: String,
    /// Deprecated flag.
    pub deprecated: bool,
}

/// One row of `celestial_body`, as `AuthorityFactory::getCelestialBodyList` reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CelestialBodyInfo {
    /// Authority name.
    pub authority: String,
    /// Name.
    pub name: String,
}

/// An official name found from an alias, with the object it belongs to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfficialName {
    /// Official name.
    pub name: String,
    /// Table holding the object, such as `geodetic_crs`.
    pub table_name: String,
    /// Authority name of the object.
    pub auth_name: String,
    /// Code of the object.
    pub code: String,
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
    intermediate_use: IntermediateCrsUse,
    area_of_interest: Option<ObjectDomain>,
    spatial_criterion: SpatialCriterion,
    crs_extent_use: CrsExtentUse,
    desired_accuracy: f64,
    ballpark: BallparkUse,
    pub(super) grid_availability: GridAvailabilityUse,
    superseded: SupersededOperations,
    registry: RegistryUse,
    /// The CRS extents of the outermost call, which a sub-call of a compound CRS reuses
    /// instead of its own components' extents (PROJ's `Private::Context` extents).
    inherited_extents: Option<(Option<ObjectDomain>, Option<ObjectDomain>)>,
    /// The anti-recursion guards of PROJ's `Private::Context`, as `GUARD_*` bits.
    pub(super) guards: u8,
    /// `usePROJAlternativeGridNames` (`usePROJNames_`, true by default): the grid names of the
    /// registered operations, as the database has them (`Official`) or as PROJ reads them
    /// (`Proj`). The candidates are sorted on the same names (`gridsNeeded`, `SortFunction`).
    grid_names: AlternativeGridNames,
}

impl CoordinateOperationContext {
    /// Context that reads `db`. An empty authority considers every authority.
    ///
    /// The name is the one `proj_create_operation_factory_context` gives its
    /// `AuthorityFactory`: `EPSG`, `ESRI` and `PROJ` in any case are spelled as PROJ spells them
    /// (`factory.cpp:4400-4417`), and the context keeps the spelling.
    #[must_use]
    pub fn create(db: &DatabaseContext, authority: impl Into<String>) -> Self {
        Self {
            db: db.clone(),
            authority: canonical_authority(authority.into()),
            intermediates: Vec::new(),
            intermediate_use: IntermediateCrsUse::Allow,
            area_of_interest: None,
            spatial_criterion: SpatialCriterion::default(),
            crs_extent_use: CrsExtentUse::default(),
            desired_accuracy: 0.0,
            ballpark: BallparkUse::Allow,
            grid_availability: GridAvailabilityUse::default(),
            superseded: SupersededOperations::Discard,
            registry: RegistryUse::default(),
            inherited_extents: None,
            guards: 0,
            grid_names: AlternativeGridNames::Proj,
        }
    }

    /// The grid names the registered operations are read with, and the candidates are sorted on
    /// (`usePROJAlternativeGridNames`, see [`Self::set_alternative_grid_names`]).
    fn alternative_grid_names(&self) -> AlternativeGridNames {
        self.grid_names
    }

    /// This context, with the CRS extents of `source` and `target` for every sub-call, unless
    /// the outermost call already set them.
    pub(crate) fn with_inherited_extents(&self, source: &Crs, target: &Crs) -> Self {
        let mut context = self.clone();
        if context.inherited_extents.is_none() {
            context.inherited_extents = Some((crs_extent(source), crs_extent(target)));
        }
        context
    }

    /// Whether the guard `guard` (`GUARD_*`) is set.
    pub(super) fn has_guard(&self, guard: u8) -> bool {
        self.guards & guard != 0
    }

    /// `disallowEmptyIntersection`: an operation whose extents do not meet is not built,
    /// unless the CRS extents are not used at all.
    pub(super) fn disallows_empty_intersection(&self) -> bool {
        !matches!(self.crs_extent_use, CrsExtentUse::Ignore)
    }

    /// The database the operations are read from.
    pub(super) fn database(&self) -> &DatabaseContext {
        &self.db
    }

    /// Pivot CRS codes, as `(authority, code)` pairs.
    pub fn set_intermediate_crs(&mut self, intermediates: Vec<(String, String)>) -> &mut Self {
        self.intermediates = intermediates;
        self
    }

    /// Whether pivots are used: [`IntermediateCrsUse::Never`] returns only a direct registered
    /// operation or a deriving conversion.
    pub fn set_intermediate_crs_use(&mut self, policy: IntermediateCrsUse) -> &mut Self {
        self.intermediate_use = policy;
        self
    }

    /// Area the operations must cover. `None` derives it from the CRS extents.
    pub fn set_area_of_interest(&mut self, area: Option<ObjectDomain>) -> &mut Self {
        self.area_of_interest = area;
        self
    }

    /// How operation extents are compared with the area of interest.
    pub fn set_spatial_criterion(&mut self, criterion: SpatialCriterion) -> &mut Self {
        self.spatial_criterion = criterion;
        self
    }

    /// Which CRS extents bound the operations.
    pub fn set_crs_extent_use(&mut self, use_: CrsExtentUse) -> &mut Self {
        self.crs_extent_use = use_;
        self
    }

    /// Accuracy in metres an operation must meet. Zero keeps every operation.
    pub fn set_desired_accuracy(&mut self, accuracy: f64) -> &mut Self {
        self.desired_accuracy = accuracy;
        self
    }

    /// Whether the availability of grids on disk orders the operations.
    pub fn set_grid_availability_use(&mut self, policy: GridAvailabilityUse) -> &mut Self {
        self.grid_availability = policy;
        self
    }

    /// `setUsePROJAlternativeGridNames`: the grids of the registered operations are read as PROJ
    /// names them. `Proj` substitutes the `grid_alternatives` of `proj.db` as
    /// `substitutePROJAlternativeGridNames` does (`proj_grid_substitution`). The default is
    /// `Proj`, as PROJ's `usePROJNames_` is true, and `Official` keeps the names the database
    /// stores.
    pub fn set_alternative_grid_names(&mut self, names: AlternativeGridNames) -> &mut Self {
        self.grid_names = names;
        self
    }

    /// Whether a registry operation with a grid that is not available is discarded, under
    /// `KNOWN_AVAILABLE` or `DISCARD_OPERATION_IF_MISSING_GRID`.
    pub(super) fn discards_missing_grids(&self) -> bool {
        matches!(
            self.grid_availability,
            GridAvailabilityUse::KnownAvailable | GridAvailabilityUse::DiscardIfMissing
        )
    }

    /// Whether an operation that a registered operation supersedes is dropped. PROJ's
    /// `setDiscardSuperseded`; the default is [`SupersededOperations::Discard`].
    pub fn set_superseded_operations(&mut self, policy: SupersededOperations) -> &mut Self {
        self.superseded = policy;
        self
    }

    /// Whether operations with a ballpark transformation are candidates.
    pub fn set_ballpark_use(&mut self, policy: BallparkUse) -> &mut Self {
        self.ballpark = policy;
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

    /// `createOperation(source, target)`: the first operation between the two CRS when no
    /// authority database is consulted, or `None` when there is none.
    ///
    /// # Errors
    ///
    /// An operation cannot be built from the CRS.
    pub fn create_operation(
        &self,
        source: &Crs,
        target: &Crs,
    ) -> Result<Option<CoordinateOperation>, FactoryError> {
        let db = DatabaseContext::create()?;
        let mut context = CoordinateOperationContext::create(&db, "");
        context.registry = RegistryUse::None;
        Ok(self
            .create_operations(source, target, &context)?
            .into_iter()
            .next())
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
        self.create_operations_between_epochs(source, target, Epochs::default(), context)
    }

    /// PROJ's public `createOperations` with the coordinate epochs `epochs`: the candidates
    /// (`candidates_at_epochs`), filtered and sorted, then the ballpark change of a vertical
    /// source, as PROJ does in its private `createOperations`.
    fn create_operations_between_epochs(
        &self,
        source: &Crs,
        target: &Crs,
        epochs: Epochs,
        context: &CoordinateOperationContext,
    ) -> Result<Vec<CoordinateOperation>, FactoryError> {
        // `createOperations` starts from the canonical bound CRS of an input that has one.
        let source = canonical_or_self(source);
        let target = canonical_or_self(target);
        check_same_celestial_body(&source, &target)?;
        let (source, source_extent, source_changed) = resolve_crs(&source, context);
        let (target, target_extent, target_changed) = resolve_crs(&target, context);
        let mut resolved_context = context.clone();
        if resolved_context.inherited_extents.is_none() && (source_changed || target_changed) {
            resolved_context.inherited_extents = Some((source_extent, target_extent));
        }
        let context = &resolved_context;
        if context.crs_extent_use == CrsExtentUse::Intersection {
            if let (Some(left), Some(right)) = (crs_extent(&source), crs_extent(&target)) {
                if !extent_intersects(&left, &right) {
                    return Ok(Vec::new());
                }
            }
        }
        let mut found = self.candidates_at_epochs(&source, epochs, &target, context)?;
        // The dispatch of `createOperations` for a vertical source and a geographic target appends
        // the ballpark change of unit to what the database gave (coordinateoperationfactory.cpp:
        // 3748-3751, 6107-6108). A geographic source and a vertical target take the reversed
        // search instead, which returns what it finds without that change (`geogSrc && vertDst`,
        // :4259-4276): the change comes only when it finds nothing. The geoid route has added it
        // already (`create_candidates`), so it is not added twice.
        if (matches!(source, Crs::Vertical(_)) || found.is_empty())
            && !found.iter().any(super::compound::is_synthesized_ballpark)
        {
            found.extend(super::compound::vertical_geog_synthesized(&source, &target));
        }
        Ok(select_candidates(found, &source, &target, context))
    }

    /// `createOperations(sourceCoordinateMetadata, targetCRS, context)`: as
    /// [`Self::create_operations`], each operation carrying the epoch of `source`.
    ///
    /// # Errors
    ///
    /// As [`Self::create_operations`].
    pub fn create_operations_from_metadata(
        &self,
        source: &CoordinateMetadata,
        target: &Crs,
        context: &CoordinateOperationContext,
    ) -> Result<Vec<CoordinateOperation>, FactoryError> {
        self.create_operations_at_epochs(
            source.crs(),
            source.coordinate_epoch(),
            target,
            None,
            context,
        )
    }

    /// `createOperations(sourceCRS, targetCoordinateMetadata, context)`: as
    /// [`Self::create_operations`], each operation carrying the epoch of `target`.
    ///
    /// # Errors
    ///
    /// As [`Self::create_operations`].
    pub fn create_operations_to_metadata(
        &self,
        source: &Crs,
        target: &CoordinateMetadata,
        context: &CoordinateOperationContext,
    ) -> Result<Vec<CoordinateOperation>, FactoryError> {
        self.create_operations_at_epochs(
            source,
            None,
            target.crs(),
            target.coordinate_epoch(),
            context,
        )
    }

    /// `createOperations(sourceCoordinateMetadata, targetCoordinateMetadata, context)`: as
    /// [`Self::create_operations`], each operation carrying both epochs.
    ///
    /// # Errors
    ///
    /// As [`Self::create_operations`].
    pub fn create_operations_between_metadata(
        &self,
        source: &CoordinateMetadata,
        target: &CoordinateMetadata,
        context: &CoordinateOperationContext,
    ) -> Result<Vec<CoordinateOperation>, FactoryError> {
        self.create_operations_at_epochs(
            source.crs(),
            source.coordinate_epoch(),
            target.crs(),
            target.coordinate_epoch(),
            context,
        )
    }

    /// The operations of [`Self::create_operations`] with the epochs set on each of them.
    ///
    /// The point motion branch needs both epochs. The epoch change of
    /// `createOperationsGeogToGeog` (coordinateoperationfactory.cpp:3226-3257) is not ported.
    fn create_operations_at_epochs(
        &self,
        source: &Crs,
        source_epoch: Option<f64>,
        target: &Crs,
        target_epoch: Option<f64>,
        context: &CoordinateOperationContext,
    ) -> Result<Vec<CoordinateOperation>, FactoryError> {
        let epochs = Epochs {
            source: source_epoch,
            target: target_epoch,
        };
        let operations = self.create_operations_between_epochs(source, target, epochs, context)?;
        Ok(operations
            .into_iter()
            .map(|operation| with_epochs(operation, source_epoch, target_epoch))
            .collect())
    }

    /// `createOperationsFromDatabase`, the point motion branch
    /// (coordinateoperationfactory.cpp:4278-4345), for two geodetic CRS at two epochs.
    ///
    /// The point motion operations of the source datum are paired with those of the target
    /// datum; when every pair is equivalent, each one gives the operation from the source to its
    /// own CRS, the point motion with both epochs, and the operation from its CRS to the target,
    /// concatenated. The forced ballpark of `createOperationsGeodToGeod` is appended. `None`
    /// when the branch does not apply (the epochs are equal, a CRS is not geodetic, the lists
    /// differ in length or in a pair), and the registry search runs.
    fn point_motion_from_database(
        &self,
        source: &Crs,
        source_epoch: f64,
        target: &Crs,
        target_epoch: f64,
        context: &CoordinateOperationContext,
    ) -> Result<Option<Vec<CoordinateOperation>>, FactoryError> {
        if source_epoch == target_epoch {
            return Ok(None);
        }
        let (Some(source_geodetic), Some(target_geodetic)) =
            (geodetic_of_crs(source), geodetic_of_crs(target))
        else {
            return Ok(None);
        };
        let factory = AuthorityFactory::create(&context.db, context.authority.as_str());
        let source_list = factory
            .get_point_motion_operations_for(&source_geodetic, AlternativeGridNames::Proj)?;
        if source_list.is_empty() {
            return Ok(None);
        }
        let target_list = factory
            .get_point_motion_operations_for(&target_geodetic, AlternativeGridNames::Proj)?;
        if target_list.len() != source_list.len() {
            return Ok(None);
        }
        let mut res = Vec::new();
        for (from, to) in source_list.iter().zip(&target_list) {
            if !from.is_equivalent_to_with(to, Equivalence::Equivalent) {
                return Ok(None);
            }
            // The point motion has one CRS as source and target (`pmo->sourceCRS() ==
            // pmo->targetCRS()`).
            let Some(point_motion_crs) = from.source_crs().cloned() else {
                return Ok(None);
            };
            let point_motion = point_motion_with_epochs(from.clone(), source_epoch, target_epoch);
            let mut ops = Vec::new();
            if !point_motion_crs.is_equivalent_to_with(source, Equivalence::Equivalent) {
                let first = self.create_candidates(source, &point_motion_crs, context)?;
                let Some(first) = first.into_iter().next() else {
                    return Ok(None);
                };
                ops.push(first);
            }
            ops.push(CoordinateOperation::Transformation(point_motion));
            if !point_motion_crs.is_equivalent_to_with(target, Equivalence::Equivalent) {
                let last = self.create_candidates(&point_motion_crs, target, context)?;
                let Some(last) = last.into_iter().next() else {
                    return Ok(None);
                };
                ops.push(last);
            }
            if let Some(operation) = super::compound::compute_metadata(ops, context) {
                res.push(operation);
            }
        }
        res.extend(self.forced_geodetic_operations(source, target, context)?);
        Ok(Some(res))
    }

    /// `createOperationsGeodToGeod` with `forceBallpark` (coordinateoperationfactory.cpp:
    /// 5236-5523), for two geodetic CRS. Two geographic CRS: [`forced_ballpark_operations`]. A
    /// geographic and a geocentric CRS of the same datum: the flagged PROJ-based conversion. On
    /// different datums, the geographic intermediate of the geocentric CRS, then the operations
    /// of the full `createOperations` from it to the other CRS, each chained (5314-5331); the
    /// reverse pair is the inverse. Two geocentric CRS: the ballpark translation.
    fn forced_geodetic_operations(
        &self,
        source: &Crs,
        target: &Crs,
        context: &CoordinateOperationContext,
    ) -> Result<Vec<CoordinateOperation>, FactoryError> {
        let source_geocentric = matches!(source, Crs::Geodetic(GeodeticCRS::Geocentric(_)));
        let target_geocentric = matches!(target, Crs::Geodetic(GeodeticCRS::Geocentric(_)));
        let (source_geographic, target_geographic) = (
            geographic_of(source).is_some(),
            geographic_of(target).is_some(),
        );
        if source_geographic && target_geographic {
            return Ok(forced_ballpark_operations(source, target));
        }
        if !((source_geocentric && target_geographic) || (source_geographic && target_geocentric)) {
            return Ok(forced_ballpark_operations(source, target));
        }
        if same_geodetic_datum(source, target) {
            return Ok(vec![super::geodetic_ops::forced_geographic_geocentric(
                source, target,
            )?]);
        }
        if !source_geocentric {
            return Ok(self
                .forced_geodetic_operations(target, source, context)?
                .iter()
                .map(CoordinateOperation::inverse)
                .collect());
        }
        let intermediate = super::geodetic_ops::geocentric_intermediate(source)?;
        let first = CoordinateOperation::Transformation(
            super::geodetic_ops::geographic_geocentric_conversion(source, &intermediate)?,
        );
        let mut found = Vec::new();
        for second in self.create_candidates(&intermediate, target, context)? {
            if let Some(operation) =
                super::compound::compute_metadata(vec![first.clone(), second], context)
            {
                found.push(operation);
            }
        }
        Ok(found)
    }

    /// `proj_get_suggested_operation` (forward) on `operations`, the list from `source`
    /// to a target: the index of the operation whose area of use holds `coord`. `coord` is
    /// in the coordinates of `source`: longitude and latitude in its axis order for a
    /// geographic source, the projected coordinates for a projected one. An operation with no
    /// area of use covers the world. See [`CoordinateOperationFactory::suggested_operation_xyz`]
    /// for a geocentric source.
    ///
    /// # Errors
    ///
    /// `source` is a geographic CRS whose axis order is not latitude or longitude, or the
    /// default database cannot be opened for a projected or geocentric source.
    pub fn suggested_operation(
        &self,
        operations: &[CoordinateOperation],
        source: &Crs,
        coord: (f64, f64),
    ) -> Result<Option<usize>, FactoryError> {
        self.suggested_operation_xyz(operations, source, (coord.0, coord.1, 0.0))
    }

    /// [`CoordinateOperationFactory::suggested_operation`] for a point with a height, as a
    /// geocentric source needs (`proj_get_suggested_operation` takes a `PJ_COORD`). The
    /// height is ignored by the other kinds of source.
    ///
    /// # Errors
    ///
    /// As [`CoordinateOperationFactory::suggested_operation`].
    pub fn suggested_operation_xyz(
        &self,
        operations: &[CoordinateOperation],
        source: &Crs,
        coord: (f64, f64, f64),
    ) -> Result<Option<usize>, FactoryError> {
        super::suggest::suggested_forward(operations, source, coord)
    }

    /// The registry operations of a pair, and nothing else: PROJ's `createOperationsFromDatabase`
    /// when it is called directly, as `createOperationsCompoundToCompound` does for the horizontal
    /// parts (`coordinateoperationfactory.cpp:7790-7794`). No derived or datum branch runs.
    pub(super) fn registry_operations(
        &self,
        source: &Crs,
        target: &Crs,
        context: &CoordinateOperationContext,
    ) -> Result<Vec<CoordinateOperation>, FactoryError> {
        if context.registry != RegistryUse::Database {
            return Ok(Vec::new());
        }
        let (Some(source_id), Some(target_id)) = (
            registry_identity(source, context),
            registry_identity(target, context),
        ) else {
            return Ok(Vec::new());
        };
        let (source_extent, target_extent) = crs_extents_of(context, source, target);
        let boxes = (
            source_extent.as_ref().and_then(ObjectDomain::bounding_box),
            target_extent.as_ref().and_then(ObjectDomain::bounding_box),
        );
        direct_registry_operations(
            context,
            (source_id.0.as_str(), source_id.1.as_str()),
            (target_id.0.as_str(), target_id.1.as_str()),
            boxes,
        )
    }

    /// PROJ's private `createOperations`: the candidates before the area, spatial, and
    /// accuracy filters. Only [`Self::create_operations`] applies them, once, so that the
    /// operations a derived CRS chains are not filtered twice.
    pub(super) fn create_candidates(
        &self,
        source: &Crs,
        target: &Crs,
        context: &CoordinateOperationContext,
    ) -> Result<Vec<CoordinateOperation>, FactoryError> {
        self.create_candidates_at(source, target, Epochs::default(), context)
    }

    /// PROJ's private `createOperations` with the coordinate epochs `epochs`: the point motion
    /// branch of `createOperationsFromDatabase` first, for two geodetic CRS at two different
    /// epochs (coordinateoperationfactory.cpp:4278-4345). When that branch does not apply, the
    /// candidates of [`Self::create_candidates_at`]. Nothing is filtered, as in PROJ.
    pub(super) fn candidates_at_epochs(
        &self,
        source: &Crs,
        epochs: Epochs,
        target: &Crs,
        context: &CoordinateOperationContext,
    ) -> Result<Vec<CoordinateOperation>, FactoryError> {
        if let Some((source_epoch, target_epoch)) = epochs.differing() {
            if let Some(found) = self.point_motion_from_database(
                source,
                source_epoch,
                target,
                target_epoch,
                context,
            )? {
                return Ok(found);
            }
        }
        self.create_candidates_at(source, target, epochs, context)
    }

    /// The candidates of [`Self::create_candidates`] at the coordinate epochs `epochs`: the epochs
    /// reach the datum pivot, whose epoch change is a point motion of a candidate.
    fn create_candidates_at(
        &self,
        source: &Crs,
        target: &Crs,
        epochs: Epochs,
        context: &CoordinateOperationContext,
    ) -> Result<Vec<CoordinateOperation>, FactoryError> {
        if let Some(operation) = super::projstring::operation_from_proj_extensions(source, target)?
        {
            return Ok(vec![operation]);
        }
        if super::geodetic_ops::is_spherical_planetocentric(source) {
            return super::geodetic_ops::spherical_planetocentric_operations(
                self, source, target, context,
            );
        }
        if super::geodetic_ops::is_spherical_planetocentric(target) {
            return Ok(self
                .create_candidates(target, source, context)?
                .iter()
                .map(CoordinateOperation::inverse)
                .collect());
        }
        if let Some(bound) = super::bound_ops::bound_candidates(self, source, target, context)? {
            return Ok(bound);
        }
        // `createOperationsFromDatabase` reads the geoid models of a 3D geographic CRS and a
        // vertical CRS before the registry: a result ends the search, with the ballpark change of
        // unit that PROJ appends to it (coordinateoperationfactory.cpp:4260-4271).
        if context.registry == RegistryUse::Database {
            let geoid = super::compound::geoid_vertical(source, target, context)?;
            if !geoid.is_empty() {
                let mut operations = geoid;
                operations.extend(super::compound::vertical_geog_synthesized(source, target));
                return Ok(operations);
            }
        }
        let mut found = Vec::new();
        let mut direct_empty = true;
        let mut direct_unfiltered_empty = true;
        if let Crs::Projected(projected) = target {
            if projection_uses_source(source, projected, context.registry) {
                // The caller's own source, not the base: its axis order is the one the
                // pipeline must read (a PROJ-string base may be longitude first).
                found.push(CoordinateOperation::Conversion(
                    projected
                        .deriving_conversion()
                        .clone()
                        .with_endpoints(source.clone(), target.clone()),
                ));
            }
        }
        let (source_extent, target_extent) = crs_extents_of(context, source, target);
        let boxes = (
            source_extent.as_ref().and_then(ObjectDomain::bounding_box),
            target_extent.as_ref().and_then(ObjectDomain::bounding_box),
        );
        let registry = context.registry == RegistryUse::Database;
        let source_identity = if registry {
            registry_identity(source, context)
        } else {
            None
        };
        let target_identity = if registry {
            registry_identity(target, context)
        } else {
            None
        };
        if let (true, Some(source_id), Some(target_id)) =
            (registry, source_identity, target_identity)
        {
            found.extend(direct_registry_operations(
                context,
                (source_id.0.as_str(), source_id.1.as_str()),
                (target_id.0.as_str(), target_id.1.as_str()),
                boxes,
            )?);
            direct_unfiltered_empty = found.is_empty();
            direct_empty = filter_and_sort(
                found.clone(),
                context,
                &sort_extents(context, source, target),
            )
            .operations
            .is_empty();
            let pivots_used = match context.intermediate_use {
                IntermediateCrsUse::Never => false,
                IntermediateCrsUse::Allow => direct_empty,
                IntermediateCrsUse::Always => true,
            };
            if pivots_used {
                for (pivot_auth, pivot_code) in &context.intermediates {
                    let first = operations_between(
                        &context.db,
                        &context.authority,
                        (source_id.0.as_str(), source_id.1.as_str()),
                        (pivot_auth, pivot_code),
                        boxes,
                        RegistrySearch::Both,
                        context.alternative_grid_names(),
                    )?;
                    let second = operations_between(
                        &context.db,
                        &context.authority,
                        (pivot_auth, pivot_code),
                        (target_id.0.as_str(), target_id.1.as_str()),
                        boxes,
                        RegistrySearch::Both,
                        context.alternative_grid_names(),
                    )?;
                    if let (Some(left), Some(right)) = (first.first(), second.first()) {
                        let interpolation =
                            concatenation_interpolation(&[left.clone(), right.clone()])
                                .map(Box::new);
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
                            computed_name: true,
                            source_epoch: None,
                            target_epoch: None,
                            proj_based: None,
                            interpolation,
                        }));
                    }
                }
            }
        }
        // `createOperations` (coordinateoperationfactory.cpp:3652-3656, 3694-3701): with a derived
        // source (a projected CRS), the registry result is kept and `createOperationsDerivedTo`
        // (5523) is appended, unless that result has a perfect accuracy (4351, 4577). The
        // candidates appended here are then sorted and filtered with the grid policy, as in PROJ.
        if super::derived::is_derived(source)
            && !has_perfect_accuracy_result(
                &found,
                context,
                source_extent.as_ref(),
                target_extent.as_ref(),
            )
        {
            if let Some(derived) =
                super::derived::derived_candidates(self, source, target, context)?
            {
                found.extend(derived);
                return Ok(found);
            }
        }
        if found.is_empty() {
            if let Some(derived) =
                super::derived::derived_candidates(self, source, target, context)?
            {
                return Ok(derived);
            }
        }
        // `NEVER` bars the datum pivot between identified CRS. A CRS without identifier
        // keeps it: PROJ finds its operations through the candidates of its datum, which
        // this port reads here (not ported: that candidate search).
        let pivot_barred = context.intermediate_use == IntermediateCrsUse::Never
            && !source.identifiers().is_empty()
            && !target.identifiers().is_empty();
        if registry
            && !pivot_barred
            && context.intermediates.is_empty()
            && is_geodetic(source)
            && is_geodetic(target)
        {
            // The datum pivot is tried only when the direct registry search gave nothing
            // (coordinateoperationfactory.cpp:4404).
            // `sameGeodeticDatum` of createOperationsFromDatabase (coordinateoperationfactory.cpp:
            // 4402), which compares the datums with the database.
            let same_datum = same_geodetic_datum_with_database(source, target, &context.db);
            let mut pivot = if direct_empty && !context.has_guard(GUARD_DATUM_PIVOT) {
                datum_pivot_operations(source, target, epochs, context)
            } else {
                Vec::new()
            };
            // `findsOpsInRegistryWithIntermediate(..., false)` (coordinateoperationfactory.cpp:
            // 4449-4457): `ALWAYS` reads the registered CRS between the two whatever the direct
            // and pivot results are; `IF_NO_DIRECT` only when there is no direct operation and no
            // datum pivot. The result is filtered as the direct one is.
            let crs_codes_needed = context.intermediate_use != IntermediateCrsUse::Never
                && (context.intermediate_use == IntermediateCrsUse::Always
                    || (direct_unfiltered_empty && pivot.is_empty()))
                && context.intermediates.is_empty()
                && !context.has_guard(GUARD_DATUM_INTERMEDIATE)
                && !same_datum;
            if crs_codes_needed {
                let codes = crs_codes_intermediate_operations(source, target, context)?;
                pivot.extend(select_candidates(codes, source, target, context));
            }
            // `!inCreateOperationsWithDatumPivotAntiRecursion && !inCreateOperationsWithIntermediate`
            // (coordinateoperationfactory.cpp:4467-4468): the intermediate search is off inside a
            // pivot or inside another intermediate search, such as the ensemble step's members.
            // The block after the datum pivot (coordinateoperationfactory.cpp:4467-4515): `res` is
            // the registry result, the datum pivot and the CRS codes; the intermediate search runs
            // when `res` is empty, or when its operations are replaced, too small for the area of
            // interest, all time-dependent between static CRS, or ensemble null operations.
            let res: Vec<CoordinateOperation> = found.iter().chain(pivot.iter()).cloned().collect();
            let intermediate_tried = !context.has_guard(GUARD_DATUM_PIVOT)
                && !context.has_guard(GUARD_DATUM_INTERMEDIATE)
                && context.intermediate_use != IntermediateCrsUse::Never
                && !same_datum;
            let replaced = !direct_unfiltered_empty && use_only_replaced_operations(&res);
            if intermediate_tried && (direct_unfiltered_empty || replaced) {
                let too_small = has_too_small_areas(&res, source, target, context);
                let all_time_dependent = res
                    .iter()
                    .all(CoordinateOperation::requires_per_coordinate_input_time);
                if res.is_empty()
                    || replaced
                    || too_small
                    || (all_time_dependent && !source.is_dynamic() && !target.is_dynamic())
                    || only_ensemble_null_operations(&res)
                {
                    let with_intermediate =
                        datum_based_intermediate_operations(source, target, context)?;
                    if too_small && !res.is_empty() {
                        // Only the operations whose names are not already there.
                        let names: BTreeSet<&str> =
                            res.iter().map(CoordinateOperation::name).collect();
                        let new_ones: Vec<CoordinateOperation> = with_intermediate
                            .into_iter()
                            .filter(|operation| !names.contains(operation.name()))
                            .collect();
                        pivot.extend(new_ones);
                    } else {
                        pivot.extend(with_intermediate);
                    }
                }
            }
            found.extend(pivot);
        }
        // The point motion block of `createOperationsCompoundToCompound` (coordinateoperationfactory.
        // cpp:7228-7290), which runs after the registry for two compound CRS. Its result is the
        // whole of that branch: PROJ returns from it, so nothing after it is added. The registry
        // operations stay in `found`, as PROJ keeps them in `res`.
        if matches!((source, target), (Crs::Compound(_), Crs::Compound(_))) {
            let point_motion =
                super::compound::compound_point_motion(self, source, target, epochs, context)?;
            if !point_motion.is_empty() {
                found.extend(point_motion);
                return Ok(found);
            }
        }
        // `createOperations`, the ensemble step (coordinateoperationfactory.cpp:4524-4572): one CRS
        // defined by a datum ensemble and the other by a single datum. The operations go through
        // each member of the ensemble. Not ported: `bTriedThroughIntermediateCRS`, so the step runs
        // even where PROJ has already tried the registered intermediates.
        if registry
            && is_geodetic(source)
            && is_geodetic(target)
            && ensemble_step_applies(source, target, context)
        {
            if ensemble_of(source).is_some() && !target.domains().is_empty() {
                found.extend(ensemble_to_other_geodetic(source, target, context)?);
            } else if ensemble_of(target).is_some() && !source.domains().is_empty() {
                for operation in ensemble_to_other_geodetic(target, source, context)? {
                    found.push(operation.inverse());
                }
            }
        }
        if registry && found.is_empty() {
            found.extend(promoted_registry_vertical(source, target, boxes, context)?);
        }
        if registry && found.is_empty() {
            found.extend(super::compound::vertical_geog_fallback(
                self, source, target, context,
            )?);
        }
        if registry && found.is_empty() {
            found.extend(super::compound::vertical_pivot(
                self, source, target, context,
            )?);
        }
        // `findsOpsInRegistryWithIntermediate` for a vertical pair: it runs after the vertical
        // pivots of `createOperationsFromDatabaseWithVertCRS`, with no direct result before the
        // filter (coordinateoperationfactory.cpp:4449-4463, the call is not gated on geodetic CRS).
        let vertical_pair = matches!((source, target), (Crs::Vertical(_), Crs::Vertical(_)));
        if registry
            && vertical_pair
            && found.is_empty()
            && direct_unfiltered_empty
            && context.intermediate_use != IntermediateCrsUse::Never
            && context.intermediates.is_empty()
            && !context.has_guard(GUARD_DATUM_INTERMEDIATE)
        {
            let codes = crs_codes_intermediate_operations(source, target, context)?;
            found.extend(select_candidates(codes, source, target, context));
        }
        // `createOperationsVertToVert` also runs when the registry gave no perfect operation.
        if found.is_empty()
            || (vertical_pair
                && !has_perfect_accuracy_result(
                    &found,
                    context,
                    source_extent.as_ref(),
                    target_extent.as_ref(),
                ))
        {
            found.extend(super::compound::vertical_to_vertical(source, target));
        }
        if found.is_empty() {
            found.extend(super::compound::geographic_height_change(source, target));
        }
        found.extend(super::compound::compound_to_geog3d(
            self, source, target, context,
        )?);
        // `createOperationsToGeod` for a compound CRS to a geocentric CRS.
        if let (Crs::Compound(_), Crs::Geodetic(geodetic)) = (source, target) {
            if geographic_of(target).is_none() {
                found.extend(super::bound_ops::bound_to_geod(
                    self, source, geodetic, target, context,
                )?);
            }
        }
        // A geocentric CRS to a compound CRS is the inverse of the compound CRS to it.
        if let (Crs::Geodetic(_), Crs::Compound(_)) = (source, target) {
            if geographic_of(source).is_none() && !context.has_guard(GUARD_DATUM_INTERMEDIATE) {
                let mut inner = context.clone();
                inner.guards |= GUARD_DATUM_INTERMEDIATE;
                found.extend(
                    self.create_candidates(target, source, &inner)?
                        .iter()
                        .map(CoordinateOperation::inverse),
                );
            }
        }
        found.extend(super::compound::geog_to_compound(
            self, source, target, context,
        )?);
        found.extend(super::compound::compound_to_compound(
            self, source, target, epochs, context,
        )?);
        // `boundDst && compoundSrc` (coordinateoperationfactory.cpp, "reverse of previous case"):
        // the inverse of the bound CRS to the compound CRS.
        if let (Crs::Compound(_), Crs::Bound(_)) = (source, target) {
            found.extend(
                self.create_candidates(target, source, context)?
                    .iter()
                    .map(CoordinateOperation::inverse),
            );
        }
        if is_geodetic(source)
            && is_geodetic(target)
            && !has_perfect_accuracy_result(
                &found,
                context,
                source_extent.as_ref(),
                target_extent.as_ref(),
            )
        {
            // `sameDatum` of createOperationsGeogToGeog and createBallparkGeographicOffset
            // (coordinateoperationfactory.cpp:2961, 2306), with the database.
            let rule = super::geodetic_ops::BallparkRule::ByDatum {
                same_datum: same_geodetic_datum_with_database(source, target, &context.db),
            };
            // `createOperationsGeodToGeod` runs after a registry result without a perfect accuracy
            // (coordinateoperationfactory.cpp:3664-3668): for two CRS of the same datum it adds the
            // null offset, the axis order or the vertical unit change of
            // `createOperationsGeogToGeog`, as without a registry result. For example the point
            // motion operation `Canada velocity grid v7` of `NAD83(CSRS)v7` to itself comes with
            // `Null geographic offset from NAD83(CSRS)v7 to NAD83(CSRS)v7`.
            if registry && !found.is_empty() && !rule.same_datum() {
                found.extend(ballpark_operations(source, target, rule));
                found.extend(super::geodetic_ops::geographic_geocentric_ballpark(
                    source, target,
                ));
            } else {
                // No registered operation: the same geodetic conversions as
                // `createOperation`, including a same-datum geographic/geocentric pair.
                found.extend(super::geodetic_ops::geodetic_operations(
                    source, target, rule,
                )?);
            }
        }
        // `createOperations` (coordinateoperationfactory.cpp:3809-3833): an engineering CRS to an
        // equivalent one is the identity, a Cartesian grid offset of zero over the world.
        if let Crs::Engineering(_) = source {
            if source.is_equivalent_to(target) {
                let mut operation = Transformation::create_from_code(
                    &OpProps::new().name(format!(
                        "Identity transformation from {} to {}",
                        source.name(),
                        target.name()
                    )),
                    source.clone(),
                    target.clone(),
                    9656,
                    vec![ParameterValue::length(0.0); 2],
                    world_extent(),
                )?;
                operation.accuracy = Some(0.0);
                found.push(CoordinateOperation::Transformation(operation));
            }
        }
        Ok(found)
    }
}

/// The canonical bound CRS of `crs` when it has one, otherwise `crs` itself.
fn canonical_or_self(crs: &Crs) -> std::borrow::Cow<'_, Crs> {
    match crs.canonical_bound_crs() {
        Some(bound) => std::borrow::Cow::Owned(Crs::Bound(bound.clone())),
        None => std::borrow::Cow::Borrowed(crs),
    }
}

/// The registered operation `code` of `authority`, with the PROJ grid names.
pub(super) fn registered_operation(
    context: &CoordinateOperationContext,
    authority: &str,
    code: &str,
) -> Result<CoordinateOperation, FactoryError> {
    AuthorityFactory::create(&context.db, authority)
        .create_coordinate_operation(code, AlternativeGridNames::Proj)
}

/// `AuthorityFactory::getTransformationsForGeoid`: the operations registered for the geoid
/// model `name` (`geoid_model.name`), in the authority of `context`.
pub(super) fn geoid_model_operations(
    context: &CoordinateOperationContext,
    name: &str,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let rows = context.db.query(
        "SELECT operation_auth_name, operation_code FROM geoid_model WHERE name = ?",
        &[name.to_owned()],
    )?;
    rows.iter()
        .map(|row| {
            AuthorityFactory::create(&context.db, row[0].text())
                .create_coordinate_operation(&row[1].text(), AlternativeGridNames::Proj)
        })
        .collect()
}

/// `DatabaseContext::getTransformationsForGridName`: the operations registered on the grid
/// `grid`, or on the official name of a grid that PROJ names `grid`.
pub(super) fn grid_name_operations(
    context: &CoordinateOperationContext,
    grid: &str,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let rows = context.db.query(
        "SELECT auth_name, code FROM grid_transformation \
         WHERE grid_name = ? OR grid_name IN \
         (SELECT original_grid_name FROM grid_alternatives WHERE proj_grid_name = ?) \
         ORDER BY auth_name, code",
        &[grid.to_owned(), grid.to_owned()],
    )?;
    rows.iter()
        .map(|row| {
            AuthorityFactory::create(&context.db, row[0].text())
                .create_coordinate_operation(&row[1].text(), AlternativeGridNames::Proj)
        })
        .collect()
}

/// The public call's filters and order: area, spatial test, accuracy, ballpark use, then
/// the sort, and the last synthetic operation dropped when a sorted list has a better one.
/// `opClone->setSourceCoordinateEpoch` and `setTargetCoordinateEpoch`. Only a transformation
/// keeps epochs: PROJ writes none for the other kinds.
fn with_epochs(
    operation: CoordinateOperation,
    source_epoch: Option<f64>,
    target_epoch: Option<f64>,
) -> CoordinateOperation {
    match operation {
        CoordinateOperation::Transformation(mut transformation) => {
            transformation.source_epoch = source_epoch;
            transformation.target_epoch = target_epoch;
            CoordinateOperation::Transformation(transformation)
        }
        CoordinateOperation::Concatenated(mut concatenated) => {
            concatenated.source_epoch = source_epoch;
            concatenated.target_epoch = target_epoch;
            CoordinateOperation::Concatenated(concatenated)
        }
        other => other,
    }
}

fn select_candidates(
    found: Vec<CoordinateOperation>,
    source: &Crs,
    target: &Crs,
    context: &CoordinateOperationContext,
) -> Vec<CoordinateOperation> {
    let extents = sort_extents(context, source, target);
    let mut filtered = filter_and_sort(found, context, &extents);
    let sorted = super::select::sort_candidates(
        std::mem::take(&mut filtered.operations),
        &context.db,
        source,
        target,
        context.grid_availability,
        context.alternative_grid_names(),
        &extents,
    );
    let sorted = remove_synthetic_null_transforms(sorted, &filtered);
    let sorted = remove_uninteresting_ops(sorted);
    let sorted = remove_duplicate_ops(sorted, &context.db);
    remove_synthetic_null_transforms(sorted, &filtered)
}

/// `removeSyntheticNullTransforms`: with an operation that contains the area and needs no grid,
/// a last result that is a ballpark or null transformation is dropped, as the sorted list
/// starts with better ones.
fn remove_synthetic_null_transforms(
    mut operations: Vec<CoordinateOperation>,
    filtered: &Filtered,
) -> Vec<CoordinateOperation> {
    let synthetic_last = operations.last().is_some_and(|operation| {
        operation.has_ballpark_transformation()
            || super::select::is_null_transformation(operation.name())
    });
    if filtered.contains_area_without_grid && operations.len() > 1 && synthetic_last {
        operations.pop();
    }
    operations
}

/// `removeUninterestingOps`: for a given area, an operation with similar or worse accuracy and
/// more transformation steps than the one kept before it adds nothing.
fn remove_uninteresting_ops(operations: Vec<CoordinateOperation>) -> Vec<CoordinateOperation> {
    let mut kept = Vec::with_capacity(operations.len());
    let mut last: Option<(&CoordinateOperation, Option<ObjectDomain>, f64, usize)> = None;
    for operation in &operations {
        let accuracy = get_accuracy(operation);
        let extent = operation_extent(operation);
        let steps = if operation.identifiers().is_empty() {
            transformation_step_count(operation)
        } else {
            1
        };
        if let Some((last_operation, last_extent, last_accuracy, last_steps)) = &last {
            if last_operation.is_equivalent_to(operation) {
                continue;
            }
            let same_extent = match (&extent, last_extent) {
                (None, None) => true,
                (Some(current), Some(previous)) => {
                    extent_contains(current, previous) && extent_contains(previous, current)
                }
                _ => false,
            };
            if ((accuracy >= *last_accuracy && *last_accuracy >= 0.0)
                || (accuracy < 0.0 && *last_accuracy >= 0.0))
                && same_extent
                && steps > *last_steps
            {
                continue;
            }
        }
        kept.push(operation.clone());
        last = Some((operation, extent, accuracy, steps));
    }
    kept
}

/// `removeDuplicateOps`: an operation whose PROJ string and extent equal those of an earlier one
/// adds nothing. An operation whose PROJ string cannot be written is kept.
fn remove_duplicate_ops(
    operations: Vec<CoordinateOperation>,
    db: &DatabaseContext,
) -> Vec<CoordinateOperation> {
    if operations.len() <= 1 {
        return operations;
    }
    let mut seen: Vec<(String, Option<BoundingBox>)> = Vec::new();
    let mut kept = Vec::with_capacity(operations.len());
    for operation in operations {
        let Ok(text) = operation.export_to_proj_string_with_database(db, ProjLines::Single) else {
            kept.push(operation);
            continue;
        };
        let bbox = operation_extent(&operation).and_then(|extent| extent.bounding_box());
        let key = (text, bbox);
        if !seen.contains(&key) {
            seen.push(key);
            kept.push(operation);
        }
    }
    kept
}

/// `getTransformationStepCount`: the steps of a concatenated operation that are not
/// conversions, and 1 for any other operation.
fn transformation_step_count(operation: &CoordinateOperation) -> usize {
    match operation {
        CoordinateOperation::Concatenated(concatenated) => concatenated
            .steps()
            .iter()
            .filter(|step| !matches!(step.operation, CoordinateOperation::Conversion(_)))
            .count(),
        _ => 1,
    }
}

/// The extent of a CRS: its first usage, or for a compound CRS the intersection of its components.
pub(crate) fn crs_extent(crs: &Crs) -> Option<ObjectDomain> {
    if let Crs::Compound(compound) = crs {
        let mut extent: Option<ObjectDomain> = None;
        for component in compound.components() {
            let Some(part) = crs_extent(component) else {
                continue;
            };
            extent = Some(match extent {
                None => part,
                Some(sofar) => intersect_extents(sofar, part)?,
            });
        }
        return extent;
    }
    crs.domains().first().cloned()
}

/// Extent of an operation: its first usage, or the intersection of its steps for a concatenated one.
pub(crate) fn operation_extent(operation: &CoordinateOperation) -> Option<ObjectDomain> {
    if matches!(operation, CoordinateOperation::Conversion(_)) {
        return Some(world_extent());
    }
    if let Some(domain) = operation.domains().first() {
        return Some(domain.clone());
    }
    let CoordinateOperation::Concatenated(concatenated) = operation else {
        return None;
    };
    let mut extent: Option<ObjectDomain> = None;
    for step in &concatenated.steps {
        let Some(part) = operation_extent(&step.operation) else {
            continue;
        };
        extent = Some(match extent {
            None => part,
            Some(sofar) => intersect_extents(sofar, part)?,
        });
    }
    extent
}

/// `Extent::intersection`: an extent that contains the other is kept whole, with its
/// description; otherwise the common box, which has none. `None` when the boxes are disjoint.
/// An extent without a box keeps the other.
pub(super) fn intersect_extents(sofar: ObjectDomain, part: ObjectDomain) -> Option<ObjectDomain> {
    match (sofar.bounding_box(), part.bounding_box()) {
        (Some(left), Some(right)) => {
            if left.contains(&right) {
                return Some(part);
            }
            if right.contains(&left) {
                return Some(sofar);
            }
            Some(ObjectDomain::from_bounding_box(left.intersection(&right)?))
        }
        _ => Some(sofar),
    }
}

pub(super) fn world_extent() -> ObjectDomain {
    ObjectDomain {
        scope: None,
        area: Some("World".to_owned()),
        south: Some(-90.0),
        west: Some(-180.0),
        north: Some(90.0),
        east: Some(180.0),
    }
}

/// `Extent::contains` for two extents: an extent without a box contains anything.
fn extent_contains(extent: &ObjectDomain, other: &ObjectDomain) -> bool {
    match (extent.bounding_box(), other.bounding_box()) {
        (Some(left), Some(right)) => left.contains(&right),
        _ => true,
    }
}

/// `Extent::intersects` for two extents: an extent without a box intersects anything.
fn extent_intersects(extent: &ObjectDomain, other: &ObjectDomain) -> bool {
    match (extent.bounding_box(), other.bounding_box()) {
        (Some(left), Some(right)) => left.intersects(&right),
        _ => true,
    }
}

/// `getPseudoArea` of an extent: the integral of cos(latitude) over its box, in degrees squared.
fn extent_pseudo_area(extent: &ObjectDomain) -> f64 {
    extent.bounding_box().map_or(0.0, |bbox| bbox.pseudo_area())
}

/// `FilterResults` with `andSort`: keeps the operations that match the area of interest and accuracy.
/// The extents `FilterResults` measures against: the area of interest (the given one, or
/// the one `computeAreaOfInterest` derives from the CRS extents), and the two CRS extents.
pub(crate) struct SortExtents {
    pub(crate) area: Option<ObjectDomain>,
    pub(crate) source: Option<ObjectDomain>,
    pub(crate) target: Option<ObjectDomain>,
}

impl SortExtents {
    /// `computeAreaOfInterest`: a given area stands; otherwise the area is the intersection
    /// or the smaller of the two CRS extents, as the CRS extent use says.
    pub(crate) fn new(
        context: &CoordinateOperationContext,
        source: Option<ObjectDomain>,
        target: Option<ObjectDomain>,
    ) -> Self {
        let area = match (&context.area_of_interest, context.crs_extent_use) {
            (Some(area), _) => Some(area.clone()),
            (None, CrsExtentUse::Intersection) => match (&source, &target) {
                (Some(left), Some(right)) => match (left.bounding_box(), right.bounding_box()) {
                    (Some(a), Some(b)) => a.intersection(&b).map(ObjectDomain::from_bounding_box),
                    _ => None,
                },
                _ => None,
            },
            (None, CrsExtentUse::Smallest) => match (&source, &target) {
                (Some(left), Some(right)) => {
                    if extent_pseudo_area(left) < extent_pseudo_area(right) {
                        Some(left.clone())
                    } else {
                        Some(right.clone())
                    }
                }
                (Some(extent), None) | (None, Some(extent)) => Some(extent.clone()),
                (None, None) => None,
            },
            _ => None,
        };
        Self {
            area,
            source,
            target,
        }
    }
}

/// [`SortExtents`] of a pair: the CRS extents, unless the context ignores them.
pub(crate) fn sort_extents(
    context: &CoordinateOperationContext,
    source: &Crs,
    target: &Crs,
) -> SortExtents {
    let (source_extent, target_extent) = crs_extents_of(context, source, target);
    SortExtents::new(context, source_extent, target_extent)
}

/// The CRS extents of a pair: the inherited ones of a sub-call, else those of the CRS, and none
/// when the context ignores them.
fn crs_extents_of(
    context: &CoordinateOperationContext,
    source: &Crs,
    target: &Crs,
) -> (Option<ObjectDomain>, Option<ObjectDomain>) {
    match (&context.inherited_extents, context.crs_extent_use) {
        (_, CrsExtentUse::Ignore) => (None, None),
        (Some((source_extent, target_extent)), _) => (source_extent.clone(), target_extent.clone()),
        _ => (crs_extent(source), crs_extent(target)),
    }
}

/// Operations kept by [`filter_and_sort`], and whether a kept-or-not operation with no
/// grid and no ballpark contains the area: PROJ's `hasOpThatContainsAreaOfInterestAndNoGrid`.
struct Filtered {
    operations: Vec<CoordinateOperation>,
    contains_area_without_grid: bool,
}

fn filter_and_sort(
    operations: Vec<CoordinateOperation>,
    context: &CoordinateOperationContext,
    extents: &SortExtents,
) -> Filtered {
    let area = extents.area.as_ref();
    let (extent1, extent2) = (extents.source.as_ref(), extents.target.as_ref());
    let described = context.area_of_interest.as_ref().and_then(|area| {
        let text = area.description()?;
        operations
            .iter()
            .any(|operation| {
                operation_extent(operation).is_some_and(|extent| extent.description() == Some(text))
            })
            .then_some(text)
    });
    let mut result = Vec::new();
    let mut only_ballpark = true;
    let mut non_ballpark_with_extent = false;
    let mut non_ballpark_without_extent = false;
    let mut contains_area_without_grid = false;
    let no_grid = |operation: &CoordinateOperation| {
        !operation.has_ballpark_transformation() && super::select::grid_names(operation).is_empty()
    };
    for operation in &operations {
        if !accuracy_allowed(operation, context.desired_accuracy) {
            continue;
        }
        if context.ballpark == BallparkUse::Forbid && operation.has_ballpark_transformation() {
            continue;
        }
        let ballpark = operation.has_ballpark_transformation();
        if let Some(area) = area {
            let Some(extent) = operation_extent(operation) else {
                non_ballpark_without_extent |= !ballpark;
                continue;
            };
            if let Some(expected) = described {
                if extent.description() != Some(expected) {
                    continue;
                }
            }
            non_ballpark_with_extent |= !ballpark;
            let contains = extent_contains(&extent, area);
            if contains && no_grid(operation) {
                contains_area_without_grid = true;
            }
            match context.spatial_criterion {
                SpatialCriterion::StrictContainment if !contains => continue,
                SpatialCriterion::PartialIntersection if !extent_intersects(&extent, area) => {
                    continue
                }
                _ => {}
            }
        } else if context.crs_extent_use == CrsExtentUse::Both {
            let Some(extent) = operation_extent(operation) else {
                non_ballpark_without_extent |= !ballpark;
                continue;
            };
            non_ballpark_with_extent |= !ballpark;
            let contains_source = extent1.is_none_or(|source| extent_contains(&extent, source));
            let contains_target = extent2.is_none_or(|target| extent_contains(&extent, target));
            if contains_source && contains_target && no_grid(operation) {
                contains_area_without_grid = true;
            }
            match context.spatial_criterion {
                SpatialCriterion::StrictContainment if !(contains_source && contains_target) => {
                    continue
                }
                SpatialCriterion::PartialIntersection => {
                    let meets_source =
                        extent1.is_none_or(|source| extent_intersects(&extent, source));
                    let meets_target =
                        extent2.is_some_and(|target| extent_intersects(&extent, target));
                    if !(meets_source && meets_target) {
                        continue;
                    }
                }
                _ => {}
            }
        }
        only_ballpark &= ballpark;
        result.push(operation.clone());
    }
    if (result.is_empty() && !non_ballpark_with_extent)
        || (only_ballpark && non_ballpark_without_extent)
    {
        result.extend(
            operations
                .iter()
                .filter(|operation| accuracy_allowed(operation, context.desired_accuracy))
                .filter(|operation| {
                    context.ballpark == BallparkUse::Allow
                        || !operation.has_ballpark_transformation()
                })
                .cloned(),
        );
    }
    Filtered {
        operations: result,
        contains_area_without_grid,
    }
}

/// `hasPerfectAccuracyResult`: an operation of `operations` that the strict containment
/// filter keeps, with an accuracy of exactly 0.
fn has_perfect_accuracy_result(
    operations: &[CoordinateOperation],
    context: &CoordinateOperationContext,
    extent1: Option<&ObjectDomain>,
    extent2: Option<&ObjectDomain>,
) -> bool {
    let mut strict = context.clone();
    strict.spatial_criterion = SpatialCriterion::StrictContainment;
    let extents = SortExtents::new(&strict, extent1.cloned(), extent2.cloned());
    filter_and_sort(operations.to_vec(), &strict, &extents)
        .operations
        .iter()
        .any(|operation| get_accuracy(operation) == 0.0)
}

/// Geographic or geocentric CRS, not spherical: the ones `createOperationsGeodToGeod` handles.
fn is_geodetic(crs: &Crs) -> bool {
    matches!(
        crs,
        Crs::Geographic(_) | Crs::Geodetic(GeodeticCRS::Geographic(_) | GeodeticCRS::Geocentric(_))
    )
}

/// The geographic CRS, whichever of the two spellings the CRS was read with.
pub(super) fn geographic_of(crs: &Crs) -> Option<&GeographicCRS> {
    match crs {
        Crs::Geographic(crs) | Crs::Geodetic(GeodeticCRS::Geographic(crs)) => Some(crs),
        _ => None,
    }
}

/// The ballpark operation of `createOperationsGeodToGeod` between two CRS of different
/// datums: a geographic offset between geographic CRS, or a geocentric translation between
/// geocentric CRS. A null offset (same datum), a prime meridian shift, and a geographic to
/// geocentric pair are not ported, and give nothing here. A same-datum height-unit change
/// is [`super::compound::geographic_height_change`].
fn ballpark_operations(
    source: &Crs,
    target: &Crs,
    rule: super::geodetic_ops::BallparkRule,
) -> Vec<CoordinateOperation> {
    if rule.same_datum() {
        return Vec::new();
    }
    forced_ballpark_operations(source, target)
}

/// [`ballpark_operations`] without the same-datum exception: `createOperationsGeodToGeod` with
/// `forceBallpark`.
pub(super) fn forced_ballpark_operations(source: &Crs, target: &Crs) -> Vec<CoordinateOperation> {
    let (name, code, values, domain) = match (geographic_of(source), geographic_of(target)) {
        (Some(left), Some(right)) => {
            let meridian = |crs: &Crs| {
                geodetic_frame(crs).map(|frame| frame.prime_meridian().longitude().si_value())
            };
            if meridian(source) != meridian(target) {
                return Vec::new();
            }
            let three_d = left.coordinate_system().axes().len() == 3
                || right.coordinate_system().axes().len() == 3;
            let (code, values) = if three_d {
                (
                    9660,
                    vec![
                        ParameterValue::angle(0.0),
                        ParameterValue::angle(0.0),
                        ParameterValue::length(0.0),
                    ],
                )
            } else {
                (
                    9619,
                    vec![ParameterValue::angle(0.0), ParameterValue::angle(0.0)],
                )
            };
            let domain = match (crs_extent(source), crs_extent(target)) {
                (Some(left), Some(right)) if left == right => left,
                _ => world_extent(),
            };
            (
                format!(
                    "Ballpark geographic offset from {} to {}",
                    source.name(),
                    target.name()
                ),
                code,
                values,
                domain,
            )
        }
        _ => match (source, target) {
            (
                Crs::Geodetic(GeodeticCRS::Geocentric(_)),
                Crs::Geodetic(GeodeticCRS::Geocentric(_)),
            ) => (
                format!(
                    "Ballpark geocentric translation from {} to {}",
                    source.name(),
                    target.name()
                ),
                1031,
                vec![ParameterValue::length(0.0); 3],
                world_extent(),
            ),
            _ => return Vec::new(),
        },
    };
    Transformation::create_from_code(
        &OpProps::new().name(name),
        source.clone(),
        target.clone(),
        code,
        values,
        domain,
    )
    .map(|operation| vec![CoordinateOperation::Transformation(operation)])
    .unwrap_or_default()
}

/// The coordinate epochs of a `createOperations` call: PROJ's `sourceEpoch` and `targetEpoch`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Epochs {
    pub(super) source: Option<f64>,
    pub(super) target: Option<f64>,
}

impl Epochs {
    /// Both epochs, when both are set and differ (`!sourceEpoch->_isEquivalentTo(targetEpoch)`).
    pub(super) fn differing(self) -> Option<(f64, f64)> {
        match (self.source, self.target) {
            (Some(source), Some(target)) if source != target => Some((source, target)),
            _ => None,
        }
    }
}

/// The point motion operations of a geodetic CRS's datum, with PROJ's grid names
/// (`getPointMotionOperationsFor(crs, true)`). Empty for a CRS that is not geodetic.
pub(super) fn point_motion_candidates(
    crs: &Crs,
    context: &CoordinateOperationContext,
) -> Result<Vec<Transformation>, FactoryError> {
    let Some(geodetic) = geodetic_of_crs(crs) else {
        return Ok(Vec::new());
    };
    AuthorityFactory::create(&context.db, context.authority.as_str())
        .get_point_motion_operations_for(&geodetic, AlternativeGridNames::Proj)
}

/// The epoch change of `createTransformations` (coordinateoperationfactory.cpp:3214-3235): with
/// two different epochs, the operations of the source candidate at the two epochs when its datum
/// has a point motion, else the same for the target candidate. Returns the list after the first
/// leg and the list before the third one, which PROJ inserts around the second operation.
fn pivot_epoch_changes(
    factory: &CoordinateOperationFactory,
    left: &Crs,
    right: &Crs,
    epochs: Epochs,
    context: &CoordinateOperationContext,
) -> Result<(Vec<CoordinateOperation>, Vec<CoordinateOperation>), FactoryError> {
    if epochs.differing().is_none() {
        return Ok((Vec::new(), Vec::new()));
    }
    if !point_motion_candidates(left, context)?.is_empty() {
        let changes = factory.candidates_at_epochs(left, epochs, left, context)?;
        return Ok((changes, Vec::new()));
    }
    if !point_motion_candidates(right, context)?.is_empty() {
        let changes = factory.candidates_at_epochs(right, epochs, right, context)?;
        return Ok((Vec::new(), changes));
    }
    Ok((Vec::new(), Vec::new()))
}

/// The chains of `createTransformations` (coordinateoperationfactory.cpp:3262-3320): for each
/// index of the epoch changes, and for each second operation, the first leg, the source epoch
/// change, the second operation, the target epoch change and the third leg. `prepare` is applied
/// to each second operation first.
fn pivot_chains(
    second: &[CoordinateOperation],
    first: Option<&CoordinateOperation>,
    third: Option<&CoordinateOperation>,
    epoch_changes: (&[CoordinateOperation], &[CoordinateOperation]),
    prepare: impl Fn(CoordinateOperation) -> CoordinateOperation,
    source: &Crs,
    target: &Crs,
) -> Vec<CoordinateOperation> {
    let (changes_src, changes_dst) = epoch_changes;
    let iterations = changes_src.len().max(changes_dst.len()).max(1);
    let mut res = Vec::new();
    for index in 0..iterations {
        for operation in second {
            let mut steps: Vec<CoordinateOperation> = Vec::new();
            steps.extend(first.cloned());
            steps.extend(changes_src.get(index).cloned());
            steps.push(prepare(operation.clone()));
            steps.extend(changes_dst.get(index).cloned());
            steps.extend(third.cloned());
            pivot_endpoints(&mut steps, source, target);
            if let [only] = steps.as_slice() {
                res.push(only.clone());
                continue;
            }
            let domains = pivot_domain(&steps);
            let mut concatenated = pivot_concatenation(steps);
            concatenated.identified.domains = domains.into_iter().collect();
            res.push(CoordinateOperation::Concatenated(concatenated));
        }
    }
    res
}

/// The registered pass of `createOperationsWithDatumPivot`. Two geodetic CRS of different
/// datums with no registered operation between them are linked through a geodetic CRS of
/// each datum that has one. The first pair with a registered operation wins, the pair whose
/// names match the requested CRS first, as PROJ orders them. With two epochs, the epoch change
/// of the candidates is inserted between the legs (`pivot_epoch_changes`).
///
/// Not ported: the passes that recurse through a third CRS, and the preferred-authority order of
/// the candidates.
fn datum_pivot_operations(
    source: &Crs,
    target: &Crs,
    epochs: Epochs,
    context: &CoordinateOperationContext,
) -> Vec<CoordinateOperation> {
    let (Some(source_datum), Some(target_datum)) = (
        datum_identity(source, &context.db),
        datum_identity(target, &context.db),
    ) else {
        return Vec::new();
    };
    if same_geodetic_datum_with_database(source, target, &context.db) {
        return Vec::new();
    }
    let authority = AuthorityFactory::create(&context.db, context.authority.clone());
    let both_3d = axis_count(source) == 3 && axis_count(target) == 3;
    let candidates_source = candidate_geodetic_crs(&authority, &source_datum);
    let candidates_target = candidate_geodetic_crs(&authority, &target_datum);
    let factory = CoordinateOperationFactory::create();
    let mut pairs: Vec<(&Crs, &Crs)> = candidates_source
        .iter()
        .flat_map(|left| candidates_target.iter().map(move |right| (left, right)))
        .collect();
    pairs.sort_by_key(|(left, right)| {
        u8::from(!(left.name() == source.name() && right.name() == target.name()))
    });
    for (left, right) in pairs {
        let (left_leg, right_leg) = if both_3d {
            (
                promote_crs_to_3d(left, &context.db),
                promote_crs_to_3d(right, &context.db),
            )
        } else {
            (left.clone(), right.clone())
        };
        let (Some(first), Some(third)) =
            (pivot_leg(source, &left_leg), pivot_leg(&right_leg, target))
        else {
            continue;
        };
        let boxes = (
            crs_extent(left)
                .as_ref()
                .and_then(ObjectDomain::bounding_box),
            crs_extent(right)
                .as_ref()
                .and_then(ObjectDomain::bounding_box),
        );
        let Ok((changes_src, changes_dst)) =
            pivot_epoch_changes(&factory, left, right, epochs, context)
        else {
            continue;
        };
        let second =
            direct_registry_operations(context, identity_of(left), identity_of(right), boxes)
                .map(|operations| {
                    filter_and_sort(operations, context, &sort_extents(context, source, target))
                        .operations
                })
                .unwrap_or_default();
        let second: Vec<CoordinateOperation> = second
            .into_iter()
            .filter(|operation| {
                matches!(
                    operation,
                    CoordinateOperation::Transformation(_) | CoordinateOperation::Concatenated(_)
                )
            })
            .collect();
        // `createTransformations` (coordinateoperationfactory.cpp:3278-3330): a null first or
        // third leg is dropped, and the registry operation it leaves is marked `DERIVED_FROM`, as
        // `addModifiedIdentifier` does.
        let is_null_leg = first.is_none() || third.is_none();
        let linked = pivot_chains(
            &second,
            first.as_ref(),
            third.as_ref(),
            (&changes_src, &changes_dst),
            |operation| {
                let operation = if is_null_leg {
                    derived_from_identifier(operation)
                } else {
                    operation
                };
                if both_3d {
                    promote_pivot_operation_to_3d(operation, &context.db)
                } else {
                    operation
                }
            },
            source,
            target,
        );
        if !linked.is_empty() {
            return linked;
        }
    }
    // `createTransformations` without `useOnlyDirectRegistryOp` (iterations 1 and 2): the full
    // search between the candidates, first between two of the same kind, then of another. It
    // runs for identified CRS too: PROJ's ETRF2005 to ETRF2014 result (the ITRF chains, not the
    // ETRS89 pivot of the intermediate search) comes from here (test_projinfo.yaml#88).
    if context.has_guard(GUARD_DATUM_PIVOT) {
        return Vec::new();
    }
    let kind = |crs: &Crs| match geographic_of(crs) {
        Some(geographic) if geographic.coordinate_system().axes().len() == 3 => 1,
        Some(_) => 0,
        None => 2,
    };
    let mut inner = context.clone();
    inner.guards |= GUARD_DATUM_PIVOT;
    // `createTransformations` of one pair: the operations between the two candidates, each one
    // linked to the two legs of the pivot.
    let linked_pair = |left: &Crs, right: &Crs| -> Vec<CoordinateOperation> {
        let (left_leg, right_leg) = if both_3d {
            (
                promote_crs_to_3d(left, &context.db),
                promote_crs_to_3d(right, &context.db),
            )
        } else {
            (left.clone(), right.clone())
        };
        let (Some(first), Some(third)) =
            (pivot_leg(source, &left_leg), pivot_leg(&right_leg, target))
        else {
            return Vec::new();
        };
        let Ok((changes_src, changes_dst)) =
            pivot_epoch_changes(&factory, left, right, epochs, &inner)
        else {
            return Vec::new();
        };
        let Ok(second) = factory.create_candidates(left, right, &inner) else {
            return Vec::new();
        };
        let second: Vec<CoordinateOperation> = second
            .into_iter()
            .filter(|operation| !operation.has_ballpark_transformation())
            .collect();
        pivot_chains(
            &second,
            first.as_ref(),
            third.as_ref(),
            (&changes_src, &changes_dst),
            |operation| {
                if both_3d {
                    promote_pivot_operation_to_3d(operation, &context.db)
                } else {
                    operation
                }
            },
            source,
            target,
        )
    };
    for iteration in 1..=2 {
        // Named candidates, of the same kind in iteration 1 and of another kind in iteration 2.
        for left in &candidates_source {
            if left.name() != source.name() {
                continue;
            }
            for right in &candidates_target {
                if right.name() != target.name() {
                    continue;
                }
                if (iteration == 1) != (kind(left) == kind(right)) {
                    continue;
                }
                let linked = linked_pair(left, right);
                if !linked.is_empty() {
                    return linked;
                }
            }
        }
        // Every other pair, the candidates whose names differ from the request included. The
        // pair of the two names of the request is left to the first loop.
        for left in &candidates_source {
            let same_source_name = left.name() == source.name();
            for right in &candidates_target {
                if same_source_name && right.name() == target.name() {
                    continue;
                }
                let linked = linked_pair(left, right);
                if !linked.is_empty() {
                    return linked;
                }
            }
        }
    }
    Vec::new()
}

/// `createOperationsFromDatabase` (`coordinateoperationfactory.cpp:4359-4387`): a two-dimensional
/// geographic CRS and a vertical CRS with no registered operation between them read the registry
/// for the three-dimensional geographic CRS of the same datum. Each operation found keeps the CRS
/// pair of the request (`setCRSs`).
fn promoted_registry_vertical(
    source: &Crs,
    target: &Crs,
    boxes: (Option<BoundingBox>, Option<BoundingBox>),
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let is_geographic_2d = |crs: &Crs| {
        geographic_of(crs)
            .is_some_and(|geographic| geographic.coordinate_system().axes().len() == 2)
    };
    let (promoted_source, promoted_target) =
        if is_geographic_2d(source) && matches!(target, Crs::Vertical(_)) {
            (promote_crs_to_3d(source, &context.db), target.clone())
        } else if is_geographic_2d(target) && matches!(source, Crs::Vertical(_)) {
            (source.clone(), promote_crs_to_3d(target, &context.db))
        } else {
            return Ok(Vec::new());
        };
    let (Some(source_id), Some(target_id)) = (
        registry_identity(&promoted_source, context),
        registry_identity(&promoted_target, context),
    ) else {
        return Ok(Vec::new());
    };
    let found = direct_registry_operations(
        context,
        (source_id.0.as_str(), source_id.1.as_str()),
        (target_id.0.as_str(), target_id.1.as_str()),
        boxes,
    )?;
    Ok(found
        .into_iter()
        .map(|mut operation| {
            super::bound_ops::set_endpoints(&mut operation, source, target);
            operation
        })
        .collect())
}

/// `findsOpsInRegistryWithIntermediate` for a source CRS the registry does not know: each
/// registered CRS of its datum, of the same kind and axis count (and of the authority a 3D
/// promotion comes from), stands for it, and the operations from it to the target are
/// preceded by the operations from the source to it.
fn intermediate_through_candidates(
    source: &Crs,
    target: &Crs,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let Some(frame) = geodetic_frame(source) else {
        return Ok(Vec::new());
    };
    if frame.identifiers().is_empty()
        && (frame.name().eq_ignore_ascii_case("unknown")
            || frame.name().starts_with("Unknown based on "))
    {
        return Ok(Vec::new());
    }
    let Some(datum) = datum_identity(source, &context.db) else {
        return Ok(Vec::new());
    };
    let originating = source
        .remarks()
        .strip_prefix("Promoted to 3D from ")
        .and_then(|rest| rest.split(':').next())
        .unwrap_or("");
    let authority = AuthorityFactory::create(&context.db, context.authority.clone());
    let source_axes = axis_count(source);
    let mut inner = context.clone();
    inner.guards |= GUARD_DATUM_INTERMEDIATE;
    for candidate in candidate_geodetic_crs(&authority, &datum) {
        let ids = candidate.identifiers();
        let same_authority = originating.is_empty()
            || (ids.len() == 1 && ids[0].code_space() == originating)
            || ids.len() > 1;
        if !same_authority
            || axis_count(&candidate) != source_axes
            || geographic_of(source).is_some() != geographic_of(&candidate).is_some()
        {
            continue;
        }
        if same_geodetic_datum(&candidate, target) {
            continue;
        }
        let with_intermediate = datum_based_intermediate_operations(&candidate, target, &inner)?;
        if with_intermediate.is_empty() {
            continue;
        }
        let mut res = Vec::new();
        for first in
            CoordinateOperationFactory::create().create_candidates(source, &candidate, &inner)?
        {
            for second in &with_intermediate {
                res.push(CoordinateOperation::Concatenated(pivot_concatenation(
                    vec![first.clone(), second.clone()],
                )));
            }
        }
        if !res.is_empty() {
            return Ok(res);
        }
    }
    Ok(Vec::new())
}

/// `createBetweenGeodeticCRSWithDatumBasedIntermediates`: two geodetic CRS of different
/// datums with no registered operation between them are linked through an intermediate
/// datum. The operations read are those between a CRS of the source datum and a CRS of
/// the intermediate datum, and the same for the target datum, in the order PROJ gives.
///
/// Not ported: `discardSuperseded` (off by default), `usePROJAlternativeGridNames`, and the
/// grid discard of `KNOWN_AVAILABLE`.
fn datum_based_intermediate_operations(
    source: &Crs,
    target: &Crs,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    if !context.has_guard(GUARD_DATUM_INTERMEDIATE) {
        if registry_identity(source, context).is_none() {
            return intermediate_through_candidates(source, target, context);
        }
        if registry_identity(target, context).is_none() {
            return Ok(
                datum_based_intermediate_operations(target, source, context)?
                    .iter()
                    .map(CoordinateOperation::inverse)
                    .collect(),
            );
        }
    }
    let source_id = identity_of(source);
    let target_id = identity_of(target);
    if source_id.0.is_empty() || target_id.0.is_empty() || source_id == target_id {
        return Ok(Vec::new());
    }
    if same_geodetic_datum(source, target) {
        return Ok(Vec::new());
    }
    let db = &context.db;
    let source_list = crs_with_same_datum(db, source)?;
    let target_list = crs_with_same_datum(db, target)?;
    if source_list.is_empty() || target_list.is_empty() {
        return Ok(Vec::new());
    }
    let nad83_csrs =
        source.name().starts_with("NAD83(CSRS)") && target.name().starts_with("NAD83(CSRS)");
    // `findsOpsInRegistryWithIntermediate` (coordinateoperationfactory.cpp:2073-2298): with no
    // authority named by the context, the authorities of the pair restrict the intermediate
    // operations when there are several (`getCandidateAuthorities`, factory.cpp:7819-7839 and
    // 8397-8408). EPSG to EPSG reads PROJ, EPSG and NKG, not ESRI.
    let allowed_authorities = pivot_authorities(context, source_id.0, target_id.0)?;

    let mut sql = String::new();
    let mut params: Vec<String> = Vec::new();
    for (is_source, select_on_target) in
        [(true, true), (false, true), (true, false), (false, false)]
    {
        if !sql.is_empty() {
            sql.push_str("UNION ALL ");
        }
        let situation = format!(
            "{}_is_{}",
            if is_source { "src" } else { "tgt" },
            if select_on_target { "tgt" } else { "src" }
        );
        let (prefix1, prefix2) = if select_on_target {
            ("source", "target")
        } else {
            ("target", "source")
        };
        let list = if is_source {
            &source_list
        } else {
            &target_list
        };
        sql.push_str(&format!(
            "SELECT '{situation}' as situation, v.table_name, v.auth_name, v.code, v.name, \
             gcrs.datum_auth_name, gcrs.datum_code, a.west_lon, a.south_lat, a.east_lon, a.north_lat \
             FROM coordinate_operation_view v \
             JOIN geodetic_crs gcrs ON gcrs.auth_name = {prefix1}_crs_auth_name \
             AND gcrs.code = {prefix1}_crs_code \
             LEFT JOIN usage u ON u.object_table_name = v.table_name \
             AND u.object_auth_name = v.auth_name AND u.object_code = v.code \
             LEFT JOIN extent a ON a.auth_name = u.extent_auth_name AND a.code = u.extent_code \
             WHERE v.deprecated = 0 AND ("
        ));
        let condition = list
            .iter()
            .map(|_| format!("({prefix2}_crs_auth_name = ? AND {prefix2}_crs_code = ?)"))
            .collect::<Vec<_>>()
            .join(" OR ");
        for (auth, code) in list {
            params.push(auth.clone());
            params.push(code.clone());
        }
        sql.push_str(&condition);
        sql.push_str(") ");
        if !allowed_authorities.is_empty() {
            let marks = vec!["?"; allowed_authorities.len()].join(",");
            sql.push_str(&format!("AND v.auth_name IN ({marks}) "));
            params.extend(allowed_authorities.iter().cloned());
        } else if authority_restricts(&context.authority) {
            sql.push_str("AND v.auth_name = ? ");
            params.push(context.authority.clone());
        }
        if nad83_csrs {
            sql.push_str("AND gcrs.name NOT IN ('NAD83', 'WGS 84') ");
        }
    }

    let mut from_source: BTreeMap<String, Vec<DatumTrfm>> = BTreeMap::new();
    let mut from_target: BTreeMap<String, Vec<DatumTrfm>> = BTreeMap::new();
    for row in db.query(&sql, &params)? {
        let Some(bbox) = row_box(&row[8], &row[7], &row[10], &row[9]) else {
            continue;
        };
        let situation = row[0].text();
        let trfm = DatumTrfm {
            situation: situation.clone(),
            table: row[1].text(),
            auth: row[2].text(),
            code: row[3].text(),
            bbox,
        };
        let key = format!("{}:{}", row[5].text(), row[6].text());
        if situation == "src_is_tgt" || situation == "src_is_src" {
            from_source.entry(key).or_default().push(trfm);
        } else {
            from_target.entry(key).or_default().push(trfm);
        }
    }

    let extra: Vec<BoundingBox> = match context.crs_extent_use {
        CrsExtentUse::Ignore => Vec::new(),
        _ => [source, target]
            .into_iter()
            .filter_map(|crs| crs_extent(crs).and_then(|extent| extent.bounding_box()))
            .collect(),
    };
    let skip_intermediate_intersection = context.crs_extent_use == CrsExtentUse::Ignore;

    let mut candidates = Vec::new();
    for (key, sources) in &from_source {
        let Some(targets) = from_target.get(key) else {
            continue;
        };
        for first in sources {
            if !extra.iter().all(|area| area.intersects(&first.bbox)) {
                continue;
            }
            for second in targets {
                if !extra.iter().all(|area| area.intersects(&second.bbox)) {
                    continue;
                }
                if !skip_intermediate_intersection && !first.bbox.intersects(&second.bbox) {
                    continue;
                }
                candidates.push((first, second));
            }
        }
    }

    let mut operations: BTreeMap<String, Option<CoordinateOperation>> = BTreeMap::new();
    let mut operation_of = |trfm: &DatumTrfm| -> Option<CoordinateOperation> {
        let key = format!("{}_{}_{}", trfm.table, trfm.auth, trfm.code);
        operations
            .entry(key)
            .or_insert_with(|| {
                let operation = AuthorityFactory::create(db, trfm.auth.clone())
                    .create_coordinate_operation_typed(
                        &trfm.code,
                        ConcatenatedSteps::Allow,
                        AlternativeGridNames::Official,
                        &trfm.table,
                    )
                    .ok()?;
                let irrelevant = useless_pivot(&operation, source_id, target_id);
                (!irrelevant).then_some(operation)
            })
            .clone()
    };

    let mut pair_operations = Vec::new();
    for (first, second) in candidates {
        let (Some(mut op1), Some(mut op2)) = (operation_of(first), operation_of(second)) else {
            continue;
        };
        if first.situation == "src_is_tgt" {
            op1 = op1.inverse();
        }
        if second.situation == "tgt_is_src" {
            op2 = op2.inverse();
        }
        pair_operations.push((op1, op2));
    }

    // `createBetweenGeodeticCRSWithDatumBasedIntermediates`: a pivot that goes through a datum
    // published before the 1900 cut-off is skipped when the source or target datum is more
    // recent (ED50 to WGS 84 must not go through NTF).
    let publication_of = |crs: &Crs| {
        geodetic_frame(crs).and_then(|frame| frame.publication_date().map(str::to_owned))
    };
    let source_pub = publication_of(source);
    let target_pub = publication_of(target);
    let most_ancient = match (&target_pub, &source_pub) {
        (Some(target_date), Some(source_date)) if target_date < source_date => target_pub.clone(),
        (Some(_), None) => target_pub.clone(),
        _ => source_pub.clone(),
    }
    .unwrap_or_default();
    const CUT_OFF_DATE: &str = "1900-01-01";
    let is_older = |crs: &Crs| {
        most_ancient.as_str() > CUT_OFF_DATE
            && publication_of(crs).is_some_and(|date| date.as_str() < CUT_OFF_DATE)
    };

    let mut result = Vec::new();
    for (op1, op2) in pair_operations {
        let (Some(op1_source), Some(op1_target), Some(op2_source), Some(op2_target)) = (
            op1.source_crs(),
            op1.target_crs(),
            op2.source_crs(),
            op2.target_crs(),
        ) else {
            continue;
        };
        if [op1_source, op1_target, op2_source, op2_target]
            .into_iter()
            .any(is_older)
        {
            continue;
        }
        let factory = CoordinateOperationFactory::create();
        let mut steps = Vec::new();
        if !source.is_equivalent_to(op1_source) {
            let Some(step) = factory.create_operation(source, op1_source)? else {
                continue;
            };
            steps.push(step);
        }
        steps.push(op1.clone());
        if !op1_target.is_equivalent_to(op2_source) {
            let Some(step) = factory.create_operation(op1_target, op2_source)? else {
                continue;
            };
            steps.push(step);
        }
        steps.push(op2.clone());
        if !op2_target.is_equivalent_to(target) {
            let Some(step) = factory.create_operation(op2_target, target)? else {
                continue;
            };
            steps.push(step);
        }
        result.push(CoordinateOperation::Concatenated(pivot_concatenation(
            steps,
        )));
    }
    // The grid rejection that ends `createBetweenGeodeticCRSWithDatumBasedIntermediates`
    // (factory.cpp:8774-8780): a pivot whose grid is not available is dropped.
    without_missing_grids(result, context)
}

/// `discardIfMissingGrid` (`rejectOpDueToMissingGrid`, factory.cpp:8232-8238 and 8774-8780): with
/// the grid policy that discards an operation whose grid is not available, the operations whose
/// grids are all available are kept.
fn without_missing_grids(
    operations: Vec<CoordinateOperation>,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    if !context.discards_missing_grids() {
        return Ok(operations);
    }
    let mut kept = Vec::with_capacity(operations.len());
    for operation in operations {
        let grids = operation.grids_needed(&context.db, context.grid_availability)?;
        if grids.iter().all(|grid| grid.info.available) {
            kept.push(operation);
        }
    }
    Ok(kept)
}

/// The authorities an intermediate leg of a pair may use (`allowedAuthorities`,
/// `findsOpsInRegistryWithIntermediate`, coordinateoperationfactory.cpp:2073-2298): with no
/// authority named by the context, the authorities of the pair restrict the legs when there are
/// several (`getCandidateAuthorities`). A named authority, `any` included, gives no list: PROJ
/// then reads the legs with `""` (coordinateoperationfactory.cpp:2209-2213).
fn pivot_authorities(
    context: &CoordinateOperationContext,
    source_authority: &str,
    target_authority: &str,
) -> Result<Vec<String>, FactoryError> {
    if !context.authority.is_empty() {
        return Ok(Vec::new());
    }
    let list = candidate_authorities(&context.db, "", source_authority, target_authority)?;
    Ok(if list.len() > 1 { list } else { Vec::new() })
}

/// `checkPivot` (factory.cpp:7955-7987): an ETRF to ETRF pivot does not go through an ITRF
/// outside the two dates, and a NAD83(CSRS) to NAD83(CSRS) pivot is neither NAD83 nor WGS 84.
fn pivot_allowed(source: &Crs, target: &Crs, pivot: &Crs) -> bool {
    let (source, target, pivot) = (source.name(), target.name(), pivot.name());
    if source.starts_with("ETRF") && target.starts_with("ETRF") && pivot.starts_with("ITRF") {
        let year = |text: &str| {
            let value: i64 = text
                .chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse()
                .unwrap_or(0);
            if (80..=99).contains(&value) {
                value + 1900
            } else {
                value
            }
        };
        let (source_year, target_year, pivot_year) =
            (year(&source[4..]), year(&target[4..]), year(&pivot[4..]));
        if source_year > 0
            && target_year > 0
            && pivot_year > 0
            && (pivot_year < source_year.min(target_year)
                || pivot_year > source_year.max(target_year))
        {
            return false;
        }
    }
    if source.starts_with("NAD83(CSRS)")
        && target.starts_with("NAD83(CSRS)")
        && (pivot == "NAD83" || pivot == "WGS 84")
    {
        return false;
    }
    true
}

/// `createFromCRSCodesWithIntermediates` (factory.cpp:7630-8241): the operations from `source` to
/// `target` through a registered CRS X, read by the four joins PROJ runs in turn. X is the target
/// of the first leg and the source of the second (`source -> X -> target`), or the same in
/// another orientation (`source -> X <- target`, `X -> source -> ...`). Each row of a join is one
/// candidate, in the order of its `ORDER BY` (accuracy sum, nulls last). A row is kept when both
/// legs intersect each other and the extents of the CRS, when its authorities are the allowed
/// ones, and when its pivot is not an irrelevant one. The grid rejection of PROJ applies to the
/// concatenation.
///
/// Not ported: the explicit intermediate CRS list (`getIntermediateCRS`), the `EXPLAIN`-free
/// `intersects_bbox` (the extents are compared in Rust, with `BoundingBox::intersects`), and the
/// error of an invalid row, which is skipped as the datum-based search does.
fn crs_codes_intermediate_operations(
    source: &Crs,
    target: &Crs,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let (Some(source_id), Some(target_id)) = (
        registry_identity(source, context),
        registry_identity(target, context),
    ) else {
        return Ok(Vec::new());
    };
    if source_id == target_id {
        return Ok(Vec::new());
    }
    let db = &context.db;
    let source_key = (source_id.0.as_str(), source_id.1.as_str());
    let target_key = (target_id.0.as_str(), target_id.1.as_str());
    let allowed = pivot_authorities(context, &source_id.0, &target_id.0)?;
    let both_geographic = geographic_of(source).is_some() && geographic_of(target).is_some();
    // `allowedIntermediateObjectType == GEOGRAPHIC_CRS`: a geographic pair with no intermediate
    // CRS given takes geographic intermediates only (factory.cpp is called at 4436 with that type).
    let geographic_intermediates = both_geographic && context.intermediates.is_empty();
    let skip_intersection = context.crs_extent_use == CrsExtentUse::Ignore;
    let extra: Vec<BoundingBox> = match context.crs_extent_use {
        CrsExtentUse::Ignore => Vec::new(),
        _ => [source, target]
            .into_iter()
            .filter_map(|crs| crs_extent(crs).and_then(|extent| extent.bounding_box()))
            .collect(),
    };
    let date_of = |crs: &Crs| {
        geodetic_frame(crs).and_then(|frame| frame.publication_date().map(str::to_owned))
    };
    let min_date = match (date_of(source), date_of(target)) {
        (Some(source_date), Some(target_date)) => Some(source_date.min(target_date)),
        _ => None,
    };
    // `criterionOnIntermediateCRS`: the intermediate CRS is geographic, and its datum is not older
    // than the pair's. `role` is the column of `v1` that holds the intermediate CRS.
    let criterion = |role: &str| -> String {
        if !geographic_intermediates {
            return String::new();
        }
        match &min_date {
            Some(date) => format!(
                "AND EXISTS(SELECT 1 FROM geodetic_crs x JOIN geodetic_datum y \
                 ON y.auth_name = x.datum_auth_name AND y.code = x.datum_code \
                 WHERE x.auth_name = v1.{role}_crs_auth_name AND x.code = v1.{role}_crs_code \
                 AND x.type IN ('geographic 2D', 'geographic 3D') AND \
                 (y.publication_date IS NULL OR y.publication_date >= '{date}')) "
            ),
            None => format!(
                "AND EXISTS(SELECT 1 FROM geodetic_crs x WHERE \
                 x.auth_name = v1.{role}_crs_auth_name AND x.code = v1.{role}_crs_code \
                 AND x.type IN ('geographic 2D', 'geographic 3D')) "
            ),
        }
    };
    let discard_superseded = context.superseded == SupersededOperations::Discard;
    // The four joins of `createFromCRSCodesWithIntermediates`: the join condition, the role of X
    // in the criterion, the operation of each leg (`inverse` or not), and the leg order.
    let joins = [
        (
            "v1.target_crs_auth_name = v2.source_crs_auth_name AND v1.target_crs_code = v2.source_crs_code",
            "v1.source_crs_auth_name = ? AND v1.source_crs_code = ? AND v2.target_crs_auth_name = ? AND v2.target_crs_code = ?",
            "target",
            (false, false),
        ),
        (
            "v1.target_crs_auth_name = v2.target_crs_auth_name AND v1.target_crs_code = v2.target_crs_code",
            "v1.source_crs_auth_name = ? AND v1.source_crs_code = ? AND v2.source_crs_auth_name = ? AND v2.source_crs_code = ?",
            "target",
            (false, true),
        ),
        (
            "v1.source_crs_auth_name = v2.source_crs_auth_name AND v1.source_crs_code = v2.source_crs_code",
            "v1.target_crs_auth_name = ? AND v1.target_crs_code = ? AND v2.target_crs_auth_name = ? AND v2.target_crs_code = ?",
            "source",
            (true, false),
        ),
        (
            "v1.source_crs_auth_name = v2.target_crs_auth_name AND v1.source_crs_code = v2.target_crs_code",
            "v1.target_crs_auth_name = ? AND v1.target_crs_code = ? AND v2.source_crs_auth_name = ? AND v2.source_crs_code = ?",
            "source",
            (true, true),
        ),
    ];
    let mut operations = Vec::new();
    for (on, wher, role, (invert_first, invert_second)) in joins {
        let mut sql = String::from(
            "SELECT v1.table_name, v1.auth_name, v1.code, v1.accuracy, \
             v2.table_name, v2.auth_name, v2.code, v2.accuracy, \
             a1.south_lat, a1.west_lon, a1.north_lat, a1.east_lon, \
             a2.south_lat, a2.west_lon, a2.north_lat, a2.east_lon, ",
        );
        if discard_superseded {
            sql.push_str(
                "ss1.replacement_auth_name, ss1.replacement_code, \
                 ss2.replacement_auth_name, ss2.replacement_code \
                 FROM coordinate_operation_view v1 JOIN coordinate_operation_view v2 ",
            );
        } else {
            sql.push_str(
                "NULL, NULL, NULL, NULL FROM coordinate_operation_view v1 \
                 JOIN coordinate_operation_view v2 ",
            );
        }
        sql.push_str(&format!("ON {on} "));
        if discard_superseded {
            for (alias, view) in [("ss1", "v1"), ("ss2", "v2")] {
                sql.push_str(&format!(
                    "LEFT JOIN supersession {alias} ON {alias}.superseded_table_name = {view}.table_name \
                     AND {alias}.superseded_auth_name = {view}.auth_name \
                     AND {alias}.superseded_code = {view}.code \
                     AND {alias}.superseded_table_name = {alias}.replacement_table_name \
                     AND {alias}.same_source_target_crs = 1 "
                ));
            }
        }
        sql.push_str(
            "JOIN usage u1 ON u1.object_table_name = v1.table_name \
             AND u1.object_auth_name = v1.auth_name AND u1.object_code = v1.code \
             JOIN extent a1 ON a1.auth_name = u1.extent_auth_name AND a1.code = u1.extent_code \
             JOIN usage u2 ON u2.object_table_name = v2.table_name \
             AND u2.object_auth_name = v2.auth_name AND u2.object_code = v2.code \
             JOIN extent a2 ON a2.auth_name = u2.extent_auth_name AND a2.code = u2.extent_code \
             WHERE ",
        );
        sql.push_str(wher);
        sql.push(' ');
        let mut params = vec![
            source_id.0.clone(),
            source_id.1.clone(),
            target_id.0.clone(),
            target_id.1.clone(),
        ];
        sql.push_str("AND v1.deprecated = 0 AND v2.deprecated = 0 ");
        sql.push_str(&criterion(role));
        if !allowed.is_empty() {
            let marks = vec!["?"; allowed.len()].join(",");
            sql.push_str(&format!(
                "AND v1.auth_name IN ({marks}) AND v2.auth_name IN ({marks}) "
            ));
            params.extend(allowed.iter().cloned());
            params.extend(allowed.iter().cloned());
        }
        if authority_restricts(&context.authority) {
            sql.push_str("AND v1.auth_name = ? AND v2.auth_name = ? ");
            params.extend([context.authority.clone(), context.authority.clone()]);
        }
        sql.push_str(
            "ORDER BY (CASE WHEN v1.accuracy IS NULL THEN 1 ELSE 0 END) + \
             (CASE WHEN v2.accuracy IS NULL THEN 1 ELSE 0 END), v1.accuracy + v2.accuracy",
        );
        let rows = db.query(&sql, &params)?;
        // The extents of the legs: each leg meets the extents of the CRS, and the two legs meet.
        let mut kept = Vec::new();
        for row in rows {
            let (Some(first_box), Some(second_box)) = (
                row_box(&row[8], &row[9], &row[10], &row[11]),
                row_box(&row[12], &row[13], &row[14], &row[15]),
            ) else {
                continue;
            };
            if !extra
                .iter()
                .all(|area| area.intersects(&first_box) && area.intersects(&second_box))
            {
                continue;
            }
            if !skip_intersection && !first_box.intersects(&second_box) {
                continue;
            }
            kept.push(row);
        }
        // `filterOutSuperseded`: a leg whose replacement is another row of the same result set is
        // dropped.
        if discard_superseded {
            let first_set: BTreeSet<(String, String)> = kept
                .iter()
                .map(|row| (row[1].text(), row[2].text()))
                .collect();
            let second_set: BTreeSet<(String, String)> = kept
                .iter()
                .map(|row| (row[5].text(), row[6].text()))
                .collect();
            kept.retain(|row| {
                let replaced_first =
                    !row[16].empty() && first_set.contains(&(row[16].text(), row[17].text()));
                let replaced_second =
                    !row[18].empty() && second_set.contains(&(row[18].text(), row[19].text()));
                !replaced_first && !replaced_second
            });
        }
        for row in kept {
            let (Some(first), Some(second)) = (
                AuthorityFactory::create(db, row[1].text())
                    .create_coordinate_operation_typed(
                        &row[2].text(),
                        ConcatenatedSteps::Allow,
                        AlternativeGridNames::Official,
                        &row[0].text(),
                    )
                    .ok(),
                AuthorityFactory::create(db, row[5].text())
                    .create_coordinate_operation_typed(
                        &row[6].text(),
                        ConcatenatedSteps::Allow,
                        AlternativeGridNames::Official,
                        &row[4].text(),
                    )
                    .ok(),
            ) else {
                continue;
            };
            if useless_pivot(&first, source_key, target_key)
                || useless_pivot(&second, source_key, target_key)
            {
                continue;
            }
            let pivot_crs = if role == "target" {
                first.target_crs()
            } else {
                first.source_crs()
            };
            if pivot_crs.is_some_and(|pivot| !pivot_allowed(source, target, pivot)) {
                continue;
            }
            let first = if invert_first { first.inverse() } else { first };
            let second = if invert_second {
                second.inverse()
            } else {
                second
            };
            operations.push(CoordinateOperation::Concatenated(pivot_concatenation(
                vec![first, second],
            )));
        }
    }
    without_missing_grids(operations, context)
}

/// `useOnlyReplacedOperations` (coordinateoperationfactory.cpp:4031-4040): every operation is
/// replaced by another one.
fn use_only_replaced_operations(operations: &[CoordinateOperation]) -> bool {
    operations
        .iter()
        .all(|operation| operation.remarks().contains("Replaced by"))
}

/// `hasTooSmallAreas` (coordinateoperationfactory.cpp:3897-3959): fewer than ten operations, and
/// each one covers at most half the area of interest, the CRS extents or their intersection.
fn has_too_small_areas(
    operations: &[CoordinateOperation],
    source: &Crs,
    target: &Crs,
    context: &CoordinateOperationContext,
) -> bool {
    if operations.is_empty() || operations.len() >= 10 {
        return false;
    }
    let pseudo_area = |extent: Option<ObjectDomain>| extent.map_or(0.0, |e| extent_pseudo_area(&e));
    let intersection =
        |left: &ObjectDomain, right: &ObjectDomain| intersect_extents(left.clone(), right.clone());
    let area_of_interest = context.area_of_interest.clone();
    let (extent1, extent2) = crs_extents_of(context, source, target);
    let target_area = if let Some(area) = &area_of_interest {
        extent_pseudo_area(area)
    } else if let Some(first) = &extent1 {
        match &extent2 {
            Some(second) => pseudo_area(intersection(first, second)),
            None => extent_pseudo_area(first),
        }
    } else {
        extent2.as_ref().map_or(0.0, extent_pseudo_area)
    };
    if target_area <= 0.0 {
        return false;
    }
    for operation in operations {
        let Some(extent_op) = operation_extent(operation) else {
            continue;
        };
        let area = if let Some(area) = &area_of_interest {
            pseudo_area(intersection(&extent_op, area))
        } else if let (Some(first), Some(second)) = (&extent1, &extent2) {
            let x = intersection(&extent_op, first);
            let y = intersection(&extent_op, second);
            let both = match (&x, &y) {
                (Some(x), Some(y)) => pseudo_area(intersection(x, y)),
                _ => 0.0,
            };
            pseudo_area(x.clone()) + pseudo_area(y.clone()) - both
        } else if let Some(first) = &extent1 {
            pseudo_area(intersection(&extent_op, first))
        } else if let Some(second) = &extent2 {
            pseudo_area(intersection(&extent_op, second))
        } else {
            extent_pseudo_area(&extent_op)
        };
        if area > 0.5 * target_area {
            return false;
        }
    }
    true
}

/// `useOnlyIntermediateCRSThroughDatumEnsembleNullOp`: every operation has a PROJ step that
/// is a null transformation between a datum ensemble and one of its members. Such a pivot
/// does not connect the two datums, so the intermediate datums are searched too.
fn only_ensemble_null_operations(operations: &[CoordinateOperation]) -> bool {
    !operations.is_empty()
        && operations.iter().all(|operation| {
            let CoordinateOperation::Concatenated(concatenated) = operation else {
                return false;
            };
            concatenated.steps.iter().any(|step| {
                let operation = &step.operation;
                let is_proj = match operation {
                    CoordinateOperation::Conversion(conversion) => conversion.identifiers(),
                    CoordinateOperation::Transformation(transformation) => {
                        transformation.identifiers()
                    }
                    CoordinateOperation::Concatenated(_) => &[],
                }
                .iter()
                .any(|id| id.code_space().contains("PROJ"));
                if !is_proj {
                    return false;
                }
                let (Some(source), Some(target)) = (operation.source_crs(), operation.target_crs())
                else {
                    return false;
                };
                member_of_ensemble(source, target) || member_of_ensemble(target, source)
            })
        })
}

/// The guard and name conditions of the ensemble step (coordinateoperationfactory.cpp:4524-4545):
/// exactly one of the two CRS has a datum ensemble, neither datum is a member of the other's
/// ensemble, the datums differ, and the GDA94/GDA2020 to WGS 84 pairs keep their own operations.
fn ensemble_step_applies(source: &Crs, target: &Crs, context: &CoordinateOperationContext) -> bool {
    let source_ensemble = ensemble_of(source).is_some();
    let target_ensemble = ensemble_of(target).is_some();
    let gda = |name: &str| name == "GDA94" || name == "GDA2020";
    !context.has_guard(GUARD_GEOG_ALTERNATIVE | GUARD_DATUM_INTERMEDIATE | GUARD_DATUM_PIVOT)
        && source_ensemble != target_ensemble
        && !same_geodetic_datum(source, target)
        && !(source_ensemble && member_of_ensemble(target, source))
        && !(target_ensemble && member_of_ensemble(source, target))
        && !(gda(source.name()) && target.name() == "WGS 84")
        && !(source.name() == "WGS 84" && gda(target.name()))
}

/// The kinds `getNatureOf` tells apart (coordinateoperationfactory.cpp:4059-4068).
#[derive(Clone, Copy, PartialEq, Eq)]
enum GeodeticNature {
    Geographic2D,
    Geographic3D,
    Geocentric,
}

impl GeodeticNature {
    /// The `type` of the `geodetic_crs` rows, as `createGeodeticCRSFromDatum` filters them.
    fn sql_type(self) -> &'static str {
        match self {
            Self::Geographic2D => "geographic 2D",
            Self::Geographic3D => "geographic 3D",
            Self::Geocentric => "geocentric",
        }
    }
}

/// `getNatureOf`: a geographic CRS by its axis count, any other geodetic CRS as geocentric.
fn geodetic_nature(crs: &Crs) -> GeodeticNature {
    match geographic_of(crs) {
        Some(geographic) if geographic.coordinate_system().axes().len() == 2 => {
            GeodeticNature::Geographic2D
        }
        Some(_) => GeodeticNature::Geographic3D,
        None => GeodeticNature::Geocentric,
    }
}

/// `ConcatenatedOperation::createComputeMetadata(steps, disallowEmptyIntersection)`: the steps,
/// whose source is the source of the first one and whose target is the target of the last one.
/// `None` when the extents of the steps do not meet and the context disallows that.
fn concatenate_steps(
    steps: Vec<CoordinateOperation>,
    context: &CoordinateOperationContext,
) -> Option<CoordinateOperation> {
    if context.disallows_empty_intersection() && super::derived::joint_extent(&steps).is_none() {
        return None;
    }
    let source = steps.first().and_then(|step| step.source_crs().cloned());
    let target = steps.last().and_then(|step| step.target_crs().cloned());
    let mut concatenated = pivot_concatenation(steps);
    concatenated.source = source.map(Box::new);
    concatenated.target = target.map(Box::new);
    Some(CoordinateOperation::Concatenated(concatenated))
}

/// The `(authority, code)` of a geodetic reference frame, when it has an identifier.
fn frame_identity(frame: &GeodeticReferenceFrame) -> Option<(String, String)> {
    frame.identifiers().first().map(|identifier| {
        (
            identifier.code_space().to_owned(),
            identifier.code().to_owned(),
        )
    })
}

/// `createOperationsEnsembleCRSToOtherGeodCRS` (coordinateoperationfactory.cpp:4047-4242):
/// the operations from a CRS defined by a datum ensemble to a geodetic CRS of another datum,
/// through each member of the ensemble that the area of the target covers. The member's own
/// operation to `target` is the one that runs, joined to the null step from the ensemble to
/// the member when the member is not the source datum (`ITRF2014 to ETRS89 (EPSG:4936)` is
/// read through `ETRS89-DNK`, whose operation to ITRF2014 is inverted by the caller).
///
/// The interpolation CRS of a member operation equal to the member CRS is replaced by the
/// ensemble CRS (coordinateoperationfactory.cpp:4161-4170). Not ported: the members that carry no
/// identifier, which `createGeodeticCRSFromDatum` cannot read.
fn ensemble_to_other_geodetic(
    source: &Crs,
    target: &Crs,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let Some(source_ensemble) = ensemble_of(source) else {
        return Ok(Vec::new());
    };
    let Some(target_extent) = target
        .domains()
        .first()
        .and_then(ObjectDomain::bounding_box)
    else {
        return Ok(Vec::new());
    };
    // `AntiRecursionGuard`: the three guards are set for the whole function.
    let mut guarded = context.clone();
    guarded.guards |= GUARD_GEOG_ALTERNATIVE | GUARD_DATUM_INTERMEDIATE | GUARD_DATUM_PIVOT;
    let context = &guarded;
    let authority = AuthorityFactory::create(&context.db, context.authority.clone());
    let factory = CoordinateOperationFactory::create();
    let is_3d = axis_count(source) == 3 && axis_count(target) == 3;
    let source_frame = geodetic_frame(source);
    let target_frame = geodetic_frame(target);
    let mut new_ops = Vec::new();
    for iteration in 0..if is_3d { 2 } else { 1 } {
        let intermediate = if !is_3d {
            GeodeticNature::Geographic2D
        } else if iteration == 0 {
            GeodeticNature::Geographic3D
        } else {
            GeodeticNature::Geocentric
        };
        for member in source_ensemble.members() {
            let Datum::Geodetic(member_frame) = member else {
                continue;
            };
            let Some(member_extent) = member_frame
                .domains()
                .first()
                .and_then(ObjectDomain::bounding_box)
            else {
                continue;
            };
            if !member_extent.intersects(&target_extent) {
                continue;
            }
            if let Some(area) = context
                .area_of_interest
                .as_ref()
                .and_then(ObjectDomain::bounding_box)
            {
                if !member_extent.intersects(&area) {
                    continue;
                }
            }
            let Some((member_auth, member_code)) = frame_identity(member_frame) else {
                continue;
            };
            let member_crs = authority.create_geodetic_crs_from_datum(
                &member_auth,
                &member_code,
                intermediate.sql_type(),
            )?;
            for member_geog in member_crs.into_iter().map(Crs::from) {
                let mut ops: Vec<CoordinateOperation> = Vec::new();
                let mut ops_last_conversion: Vec<CoordinateOperation> = Vec::new();
                // The target datum's geocentric CRS, when a 3D geographic target needs one.
                if geodetic_nature(&member_geog) == GeodeticNature::Geocentric
                    && geodetic_nature(target) == GeodeticNature::Geographic3D
                {
                    if let Some((target_auth, target_code)) =
                        target_frame.as_ref().and_then(frame_identity)
                    {
                        let geocentric = authority.create_geodetic_crs_from_datum(
                            &target_auth,
                            &target_code,
                            GeodeticNature::Geocentric.sql_type(),
                        )?;
                        if let Some(first) = geocentric.into_iter().next() {
                            let first = Crs::from(first);
                            ops = factory.create_operations(&member_geog, &first, context)?;
                            ops_last_conversion =
                                factory.create_operations(&first, target, context)?;
                        }
                    }
                }
                if ops.is_empty() {
                    ops_last_conversion.clear();
                    ops = factory.create_operations(&member_geog, target, context)?;
                }
                if geodetic_nature(&member_geog) == geodetic_nature(source)
                    && same_coordinate_system(&member_geog, source)
                {
                    for op in ops.iter().filter(|op| !op.has_ballpark_transformation()) {
                        let mut new_op = op.clone();
                        // An interpolation CRS that is the member CRS becomes the ensemble CRS
                        // (coordinateoperationfactory.cpp:4164-4170, `setInterpolationCRS`).
                        if interpolation_of(&new_op).is_some_and(|interpolation| {
                            interpolation
                                .is_equivalent_to_with(&member_geog, Equivalence::Equivalent)
                        }) {
                            set_interpolation_of(&mut new_op, source.clone());
                        }
                        if let Some(last) = ops_last_conversion.first() {
                            let Some(joined) =
                                concatenate_steps(vec![new_op, last.clone()], context)
                            else {
                                continue;
                            };
                            new_op = joined;
                        }
                        new_ops.push(with_endpoints_of(new_op, source, target));
                    }
                } else {
                    // A null transformation between an ensemble and one of its members: the
                    // operation of the source to the member is the first one.
                    let mut ops_first_conversion: Vec<CoordinateOperation> = Vec::new();
                    let mut ops_first: Vec<CoordinateOperation> = Vec::new();
                    if geodetic_nature(&member_geog) == GeodeticNature::Geocentric
                        && geodetic_nature(source) == GeodeticNature::Geographic3D
                    {
                        if let Some((source_auth, source_code)) =
                            source_frame.as_ref().and_then(frame_identity)
                        {
                            let geocentric = authority.create_geodetic_crs_from_datum(
                                &source_auth,
                                &source_code,
                                GeodeticNature::Geocentric.sql_type(),
                            )?;
                            if let Some(first) = geocentric.into_iter().next() {
                                let first = Crs::from(first);
                                ops_first_conversion =
                                    factory.create_operations(source, &first, context)?;
                                let mut without_pivot = context.clone();
                                without_pivot.guards &= !GUARD_DATUM_PIVOT;
                                ops_first = factory.create_operations(
                                    &first,
                                    &member_geog,
                                    &without_pivot,
                                )?;
                            }
                        }
                    }
                    if ops_first.is_empty() {
                        ops_first_conversion.clear();
                        ops_first = factory.create_operations(source, &member_geog, context)?;
                    }
                    if let Some(op_first) = ops_first
                        .first()
                        .filter(|op| !op.has_ballpark_transformation())
                    {
                        for op in ops.iter().filter(|op| !op.has_ballpark_transformation()) {
                            let mut steps = Vec::new();
                            steps.extend(ops_first_conversion.first().cloned());
                            steps.push(op_first.clone());
                            steps.push(op.clone());
                            steps.extend(ops_last_conversion.first().cloned());
                            if let Some(joined) = concatenate_steps(steps, context) {
                                new_ops.push(joined);
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(new_ops)
}

/// `CoordinateOperation::interpolationCRS`: the interpolation CRS the operation stores, if any.
fn interpolation_of(operation: &CoordinateOperation) -> Option<&Crs> {
    match operation {
        CoordinateOperation::Conversion(conversion) => conversion.interpolation_crs(),
        CoordinateOperation::Transformation(transformation) => transformation.interpolation_crs(),
        CoordinateOperation::Concatenated(concatenated) => concatenated.interpolation_crs(),
    }
}

/// `CoordinateOperation::setInterpolationCRS`: stores `crs` as the interpolation CRS.
fn set_interpolation_of(operation: &mut CoordinateOperation, crs: Crs) {
    let crs = Some(Box::new(crs));
    match operation {
        CoordinateOperation::Conversion(conversion) => conversion.interpolation = crs,
        CoordinateOperation::Transformation(transformation) => transformation.interpolation = crs,
        CoordinateOperation::Concatenated(concatenated) => concatenated.interpolation = crs,
    }
}

/// `setCRSs` on a copy: the source and target of the operation, as stored.
fn with_endpoints_of(
    mut operation: CoordinateOperation,
    source: &Crs,
    target: &Crs,
) -> CoordinateOperation {
    match &mut operation {
        CoordinateOperation::Conversion(conversion) => {
            conversion.source = Some(Box::new(source.clone()));
            conversion.target = Some(Box::new(target.clone()));
        }
        CoordinateOperation::Transformation(transformation) => {
            transformation.source = Some(Box::new(source.clone()));
            transformation.target = Some(Box::new(target.clone()));
        }
        CoordinateOperation::Concatenated(concatenated) => {
            concatenated.source = Some(Box::new(source.clone()));
            concatenated.target = Some(Box::new(target.clone()));
        }
    }
    operation
}

/// `coordinateSystem()->_isEquivalentTo(...)` between two geodetic CRS, on the direction and
/// the unit of each axis, in order.
fn same_coordinate_system(left: &Crs, right: &Crs) -> bool {
    let axes = |crs: &Crs| -> Vec<(String, f64, String)> {
        geographic_of(crs).map_or_else(Vec::new, |geographic| {
            geographic
                .coordinate_system()
                .axes()
                .iter()
                .map(|axis| {
                    (
                        axis.direction().to_owned(),
                        axis.unit().conversion_to_si(),
                        format!("{:?}", axis.unit().kind()),
                    )
                })
                .collect()
        })
    };
    let (left_axes, right_axes) = (axes(left), axes(right));
    left_axes.len() == right_axes.len()
        && left_axes
            .iter()
            .zip(&right_axes)
            .all(|(a, b)| a.0 == b.0 && a.2 == b.2 && (a.1 - b.1).abs() <= 1e-10 * a.1.abs())
}

/// `isDatumInEnsemble`: the datum of `member` is a member of the ensemble of `ensemble`.
fn member_of_ensemble(member: &Crs, ensemble: &Crs) -> bool {
    let (Some(datum), Some(ensemble)) = (geodetic_frame(member), ensemble_of(ensemble)) else {
        return false;
    };
    ensemble
        .members()
        .iter()
        .any(|candidate| datum.is_equivalent_to_datum(candidate))
}

/// The datum ensemble of a geodetic CRS, when it has one.
fn ensemble_of(crs: &Crs) -> Option<&DatumEnsemble> {
    match crs {
        Crs::Geographic(geographic) => geographic.datum_ensemble(),
        Crs::Geodetic(geodetic) => geodetic.datum_ensemble(),
        _ => None,
    }
}

/// A registered operation, by its `(table, auth, code)` row, read for a datum-based pivot.
struct DatumTrfm {
    situation: String,
    table: String,
    auth: String,
    code: String,
    bbox: BoundingBox,
}

/// `useIrrelevantPivot`: a concatenated operation whose inner pivot is one of the two CRS
/// being converted is not a pivot of the pair.
fn useless_pivot(
    operation: &CoordinateOperation,
    source: (&str, &str),
    target: (&str, &str),
) -> bool {
    let CoordinateOperation::Concatenated(concatenated) = operation else {
        return false;
    };
    let steps = &concatenated.steps;
    steps
        .iter()
        .take(steps.len().saturating_sub(1))
        .any(|step| {
            step.operation.target_crs().is_some_and(|crs| {
                let ids = crs.identifiers();
                ids.len() == 1
                    && ((ids[0].code_space() == source.0 && ids[0].code() == source.1)
                        || (ids[0].code_space() == target.0 && ids[0].code() == target.1))
            })
        })
}

/// `GetListCRSWithSameDatum`: the geodetic CRS of the datum of `crs`, or `crs` itself when
/// the database has none for that datum.
fn crs_with_same_datum(
    db: &DatabaseContext,
    crs: &Crs,
) -> Result<Vec<(String, String)>, FactoryError> {
    let Some((datum_auth, datum_code)) = datum_identity(crs, db) else {
        return Ok(Vec::new());
    };
    let mut list: Vec<(String, String)> = db
        .query(
            "SELECT auth_name, code FROM geodetic_crs WHERE \
             datum_auth_name = ? AND datum_code = ? AND deprecated = 0",
            &[datum_auth, datum_code],
        )?
        .iter()
        .map(|row| (row[0].text(), row[1].text()))
        .collect();
    if list.is_empty() {
        let (auth, code) = identity_of(crs);
        list.push((auth.to_owned(), code.to_owned()));
    }
    Ok(list)
}

/// Axis count of a geographic or geocentric CRS, zero for any other.
fn axis_count(crs: &Crs) -> usize {
    match crs {
        Crs::Geodetic(GeodeticCRS::Geocentric(_)) => 3,
        other => {
            geographic_of(other).map_or(0, |geographic| geographic.coordinate_system().axes().len())
        }
    }
}

fn promote_crs_to_3d(crs: &Crs, db: &DatabaseContext) -> Crs {
    match crs {
        Crs::Geographic(geographic) => Crs::Geographic(geographic.promote_to_3d_with_db(Some(db))),
        Crs::Geodetic(GeodeticCRS::Geographic(geographic)) => Crs::Geodetic(
            GeodeticCRS::Geographic(geographic.promote_to_3d_with_db(Some(db))),
        ),
        other => other.clone(),
    }
}

/// `createTransformations` with `sourceAndTargetAre3D` (coordinateoperationfactory.cpp:
/// 3318-3349): a concatenated operation is rebuilt by `createComputeMetadata` from its steps, each
/// with its endpoints promoted to 3D, so it loses its name, its identifiers and its registered
/// accuracy for the computed ones; another operation has its endpoints promoted.
fn promote_pivot_operation_to_3d(
    operation: CoordinateOperation,
    db: &DatabaseContext,
) -> CoordinateOperation {
    match operation {
        CoordinateOperation::Concatenated(concatenated) if concatenated.proj_based.is_none() => {
            let steps = concatenated
                .steps
                .iter()
                .map(|step| promote_operation_to_3d(step.oriented_operation(), db))
                .collect();
            CoordinateOperation::Concatenated(pivot_concatenation(steps))
        }
        other => promote_operation_to_3d(other, db),
    }
}

/// `promoteTo3D` on the endpoints of a registered operation, so that a Helmert advertised
/// for the 2D domain is written without the `push` / `pop` of the vertical coordinate.
fn promote_operation_to_3d(
    operation: CoordinateOperation,
    db: &DatabaseContext,
) -> CoordinateOperation {
    let promote = |crs: &Option<Box<Crs>>| -> Option<Box<Crs>> {
        crs.as_deref()
            .map(|crs| Box::new(promote_crs_to_3d(crs, db)))
    };
    match operation {
        CoordinateOperation::Transformation(mut transformation) => {
            transformation.source = promote(&transformation.source);
            transformation.target = promote(&transformation.target);
            CoordinateOperation::Transformation(transformation)
        }
        CoordinateOperation::Concatenated(mut concatenated) => {
            concatenated.steps = concatenated
                .steps
                .into_iter()
                .map(|step| ConcatenatedStep {
                    operation: promote_operation_to_3d(step.operation, db),
                    reverse: step.reverse,
                })
                .collect();
            CoordinateOperation::Concatenated(concatenated)
        }
        other @ CoordinateOperation::Conversion(_) => other,
    }
}

/// Authority code of the datum of a geodetic CRS, the key its candidates are read with.
///
/// An ensemble CRS has no datum code of its own. The ensemble code is the datum code
/// `geodetic_crs.datum_code` stores, such as EPSG:6326 for WGS 84.
fn datum_identity(crs: &Crs, db: &DatabaseContext) -> Option<(String, String)> {
    let frame = geodetic_frame(crs)?;
    if let Some(identifier) = frame.identifiers().first() {
        return Some((
            identifier.code_space().to_owned(),
            identifier.code().to_owned(),
        ));
    }
    // `findCandidateGeodCRSForDatum`: a datum without identifier takes the one of the single
    // registered datum of that name it is equivalent to.
    let name = frame.name();
    if name != "unknown" && name != "unnamed" && !name.is_empty() {
        if let Ok(matches) = AuthorityFactory::create(db, "").create_objects_from_name(
            name,
            &[ObjectType::GeodeticReferenceFrame],
            NameMatch::Exact,
            2,
        ) {
            if let [AuthorityObject::Datum(Datum::Geodetic(found))] = matches.as_slice() {
                if frame.is_equivalent_to(found) {
                    if let Some(identifier) = found.identifiers().first() {
                        return Some((
                            identifier.code_space().to_owned(),
                            identifier.code().to_owned(),
                        ));
                    }
                }
            }
        }
    }
    let ensemble = match crs {
        Crs::Geographic(geographic) => geographic.datum_ensemble(),
        Crs::Geodetic(geodetic) => geodetic.datum_ensemble(),
        _ => None,
    }?;
    let identifier = ensemble.identifiers().first()?;
    Some((
        identifier.code_space().to_owned(),
        identifier.code().to_owned(),
    ))
}

/// `CRS::getResolvedCRS`: a geographic, projected, or compound CRS read from the database by its
/// identifier, or by its name when it has none, when that object is equivalent to it. The extent
/// is the one of the CRS, or the registered one when the CRS has none (or only a synthesized one).
fn resolve_crs(
    crs: &Crs,
    context: &CoordinateOperationContext,
) -> (Crs, Option<ObjectDomain>, bool) {
    let mut extent = crs_extent(crs);
    if context.registry != RegistryUse::Database {
        return (crs.clone(), extent, false);
    }
    let original_has_extent = extent.is_some();
    let approximate = extent.is_none() || matches!(crs, Crs::Compound(_));
    let object_type = match crs {
        Crs::Geographic(geographic) | Crs::Geodetic(GeodeticCRS::Geographic(geographic)) => {
            if geographic.coordinate_system().axes().len() == 2 {
                ObjectType::Geographic2dCrs
            } else {
                ObjectType::Geographic3dCrs
            }
        }
        Crs::Projected(_) => ObjectType::ProjectedCrs,
        Crs::Compound(_) => ObjectType::CompoundCrs,
        _ => return (crs.clone(), extent, false),
    };
    let axis_directions = |crs: &Crs| -> Vec<String> {
        geographic_of(crs)
            .map(|geographic| {
                geographic
                    .coordinate_system()
                    .axes()
                    .iter()
                    .map(|axis| axis.direction().to_owned())
                    .collect()
            })
            .unwrap_or_default()
    };
    let equivalent = |left: &Crs, right: &Crs| {
        left.is_equivalent_to_with(right, Equivalence::Equivalent)
            || (left.is_equivalent_to_db(right, &context.db)
                && axis_directions(left) == axis_directions(right))
    };
    let has_geoid = matches!(
        crs,
        Crs::Compound(compound) if matches!(
            compound.components(),
            [_, Crs::Vertical(vertical)] if !vertical.geoid_models().is_empty()
        )
    );
    if let Some(id) = crs.identifiers().first() {
        let factory = AuthorityFactory::create(&context.db, id.code_space());
        if let Ok(resolved) = factory.create_coordinate_reference_system(id.code()) {
            let mut changed = false;
            if approximate {
                if let Some(registered) = crs_extent(&resolved) {
                    extent = Some(registered);
                    changed = !original_has_extent;
                }
            }
            if equivalent(&resolved, crs) && !has_geoid {
                return (resolved, extent, true);
            }
            return (crs.clone(), extent, changed);
        }
        return (crs.clone(), extent, false);
    }
    let name = crs.name();
    if name != "unknown" && name != "unnamed" {
        let authority = if context.authority == "any" {
            ""
        } else {
            context.authority.as_str()
        };
        if let Ok(matches) = AuthorityFactory::create(&context.db, authority)
            .create_objects_from_name(name, &[object_type], NameMatch::Exact, 2)
        {
            if let [AuthorityObject::Crs(found)] = matches.as_slice() {
                let mut changed = false;
                if approximate {
                    if let Some(registered) = crs_extent(found) {
                        extent = Some(registered);
                        changed = !original_has_extent;
                    }
                }
                if equivalent(found, crs) && !has_geoid {
                    return (found.clone(), extent, true);
                }
                if !matches!(crs, Crs::Compound(_)) {
                    return (crs.clone(), extent, changed);
                }
            }
        }
    }
    // A compound CRS without identifier and without its own extent: the intersection of the
    // extents of its components, each resolved (coordinateoperationfactory.cpp:8342-8356).
    if let Crs::Compound(compound) = crs {
        if approximate {
            let mut resolved: Option<ObjectDomain> = None;
            for component in compound.components() {
                let (_, component_extent, _) = resolve_crs(component, context);
                resolved = match (resolved, component_extent) {
                    (Some(sofar), Some(part)) => intersect_extents(sofar, part),
                    (None, Some(part)) => Some(part),
                    (sofar, None) => sofar,
                };
            }
            let changed = resolved != extent;
            return (crs.clone(), resolved, changed);
        }
    }
    (crs.clone(), extent, false)
}

/// `buildCRSIds`: the identifier of the CRS, or when it has none the identifier of the one
/// registered object of that type and name that is equivalent to it.
fn registry_identity(crs: &Crs, context: &CoordinateOperationContext) -> Option<(String, String)> {
    if let Some(id) = crs
        .authority_code("EPSG")
        .or_else(|| crs.identifiers().first())
    {
        return Some((id.code_space().to_owned(), id.code().to_owned()));
    }
    let name = crs.name();
    if name.eq_ignore_ascii_case("unknown") {
        return None;
    }
    let object_type = match crs {
        Crs::Geographic(geographic) | Crs::Geodetic(GeodeticCRS::Geographic(geographic)) => {
            if geodetic_frame(crs).is_some_and(|frame| frame.name() == "unknown") {
                return None;
            }
            if geographic.coordinate_system().axes().len() == 2 {
                ObjectType::Geographic2dCrs
            } else {
                ObjectType::Geographic3dCrs
            }
        }
        Crs::Projected(_) => ObjectType::ProjectedCrs,
        Crs::Vertical(_) => ObjectType::VerticalCrs,
        _ => return None,
    };
    let authority = if context.authority == "any" {
        ""
    } else {
        context.authority.as_str()
    };
    let matches = AuthorityFactory::create(&context.db, authority)
        .create_objects_from_name(name, &[object_type], NameMatch::Exact, 2)
        .ok()?;
    let [AuthorityObject::Crs(found)] = matches.as_slice() else {
        return None;
    };
    if !crs.is_equivalent_to_with(found, Equivalence::Equivalent)
        && !crs.is_equivalent_to_db(found, &context.db)
    {
        return None;
    }
    found
        .identifiers()
        .first()
        .map(|id| (id.code_space().to_owned(), id.code().to_owned()))
}

/// Authority code of a CRS, as `(codeSpace, code)`.
fn identity_of(crs: &Crs) -> (&str, &str) {
    crs.identifiers().first().map_or(("", ""), |identifier| {
        (identifier.code_space(), identifier.code())
    })
}

/// Non-deprecated geodetic CRS of the datum, in the database order.
fn candidate_geodetic_crs(authority: &AuthorityFactory, datum: &(String, String)) -> Vec<Crs> {
    authority
        .create_geodetic_crs_from_datum(&datum.0, &datum.1, "")
        .unwrap_or_default()
        .into_iter()
        .map(Crs::from)
        .collect()
}

/// One leg of the datum pivot, from `from` to `to` on the same datum. `Some(None)`: the two
/// CRS are both geographic or both geocentric, so the leg is the identity. `None`: the pair
/// cannot be linked, and the candidate pair is skipped.
fn pivot_leg(from: &Crs, to: &Crs) -> Option<Option<CoordinateOperation>> {
    match (geographic_of(from), geographic_of(to)) {
        // Two geographic CRS of one datum differ at most by their axis order. PROJ takes the
        // first operation of `createOperations(source, candidate)` as this leg, and keeps it
        // (`coordinateoperationfactory.cpp:3480-3486`, 3351-3352). For such a pair that
        // operation is the axis order change of `createOperationsGeogToGeog` (2967-2993, 3019-3020).
        (Some(left), Some(right)) => {
            if super::geodetic_ops::axis_reversal_between(left, right) {
                super::geodetic_ops::axis_order_change(from, to, left, right)
                    .ok()
                    .map(Some)
            } else {
                Some(None)
            }
        }
        (None, None) => Some(None),
        (Some(_), None) => geographic_geocentric(from, to)
            .map(|operation| Some(CoordinateOperation::Transformation(operation))),
        (None, Some(_)) => geographic_geocentric(to, from)
            .map(|operation| Some(CoordinateOperation::Transformation(operation.inverse()))),
    }
}

/// `buildOpName`: equal CRS names gain `(geog2D)`, `(geog3D)`, or `(geocentric)`.
fn operation_crs_name(kind: &str, source: &Crs, target: &Crs) -> String {
    let mut source_qualifier = "";
    let mut target_qualifier = "";
    if source.name() == target.name() {
        source_qualifier = crs_qualifier(source);
        target_qualifier = crs_qualifier(target);
        if source_qualifier == target_qualifier {
            source_qualifier = "";
            target_qualifier = "";
        }
    }
    format!(
        "{kind} from {}{source_qualifier} to {}{target_qualifier}",
        source.name(),
        target.name()
    )
}

fn crs_qualifier(crs: &Crs) -> &'static str {
    if is_geocentric_crs(crs) {
        " (geocentric)"
    } else if let Some(geographic) = geographic_of(crs) {
        if geographic.coordinate_system().axes().len() == 2 {
            " (geog2D)"
        } else {
            " (geog3D)"
        }
    } else {
        ""
    }
}

/// The geographic/geocentric conversion, EPSG:9602, as a transformation that keeps both CRS,
/// so the PROJ string export can read the ellipsoid of each side. Its domain is the world, as
/// `getExtent` gives any `Conversion` (`oputils.cpp`), not the extent of the geographic CRS: a
/// concatenated operation through it takes the area of its other steps.
fn geographic_geocentric(geographic: &Crs, geocentric: &Crs) -> Option<Transformation> {
    let name = operation_crs_name("Conversion", geographic, geocentric);
    let mut conversion = Transformation::create_from_code(
        &OpProps::new().name(name),
        geographic.clone(),
        geocentric.clone(),
        9602,
        Vec::new(),
        world_extent(),
    )
    .ok()?;
    conversion.accuracy = Some(0.0);
    Some(conversion)
}

/// The area of use of a concatenated operation of pivot legs: the intersection of the areas of
/// its steps, a step with none being skipped (`getExtent` of `oputils.cpp:529`, called by
/// `ConcatenatedOperation::createComputeMetadata`). `None` when no step has an area, or when the
/// areas do not meet.
fn pivot_domain(steps: &[CoordinateOperation]) -> Option<ObjectDomain> {
    let mut extent: Option<ObjectDomain> = None;
    for step in steps {
        let Some(part) = operation_extent(step) else {
            continue;
        };
        extent = Some(match extent {
            None => part,
            Some(sofar) => intersect_extents(sofar, part)?,
        });
    }
    extent
}

/// `Private::setCRSs` on the first step (its source becomes `source`) and on the last one (its
/// target becomes `target`), as `createTransformations` does on its subOps
/// (coordinateoperationfactory.cpp:3365-3372). `Private::setCRSs` is `setCRSsUpdateInverse`
/// (coordinateoperationfactory.cpp:3127-3132), which also reaches the first and last steps of a
/// concatenated subOp. A step keeps the other end it stores.
fn pivot_endpoints(steps: &mut [CoordinateOperation], source: &Crs, target: &Crs) {
    if let Some(first) = steps.first_mut() {
        if let Some(first_target) = super::bound_ops::operation_target(first).cloned() {
            super::bound_ops::set_endpoints_update_inverse(first, source, &first_target);
        }
    }
    if let Some(last) = steps.last_mut() {
        if let Some(last_source) = super::bound_ops::operation_source(last).cloned() {
            super::bound_ops::set_endpoints_update_inverse(last, &last_source, target);
        }
    }
}

/// `addModifiedIdentifier` with `derivedFrom` (coordinateoperationfactory.cpp:3297-3305): an
/// operation with one identifier, not already `DERIVED_FROM`, gets `DERIVED_FROM(<authority>)`,
/// inside `INVERSE(...)` when the identifier is an inverse one (`INVERSE(EPSG)`, 9090).
fn derived_from_identifier(mut operation: CoordinateOperation) -> CoordinateOperation {
    let identifiers = match &mut operation {
        CoordinateOperation::Conversion(conversion) => &mut conversion.identified.identifiers,
        CoordinateOperation::Transformation(transformation) => {
            &mut transformation.identified.identifiers
        }
        CoordinateOperation::Concatenated(concatenated) => &mut concatenated.identified.identifiers,
    };
    if let [identifier] = identifiers.as_mut_slice() {
        if !identifier.code_space().contains("DERIVED_FROM") {
            let code_space = identifier.code_space().to_owned();
            let derived = match code_space
                .strip_prefix("INVERSE(")
                .and_then(|rest| rest.strip_suffix(')'))
            {
                Some(authority) => format!("INVERSE(DERIVED_FROM({authority}))"),
                None => format!("DERIVED_FROM({code_space})"),
            };
            *identifier = Identifier::new(derived, identifier.code());
        }
    }
    operation
}

/// A concatenated operation of the pivot legs. Its accuracy is `getAccuracy` of the flattened
/// steps, as `ConcatenatedOperation::createComputeMetadata` takes it (`concatenatedoperation.cpp:
/// 754-756`): the sum of the steps, except that two 0.1 m legs through ETRF2000 between two
/// `ETRS89-XXX` CRS, one of them a no-op, make 0.1 m (EPSG:11108 to EPSG:11128). A step with no
/// accuracy leaves it unknown. Its domain is the intersection of the steps' extents, as
/// `ConcatenatedOperation::createComputeMetadata` gives it (`joint_extent`).
pub(super) fn pivot_concatenation(steps: Vec<CoordinateOperation>) -> ConcatenatedOperation {
    // `ConcatenatedOperation::createComputeMetadata` takes the steps of a concatenated step in
    // place of that step (concatenatedoperation.cpp:686-697): a registered concatenated operation
    // in the pivot is named by its own steps (test_projinfo.yaml#90).
    let steps = flatten_steps(steps);
    let name = steps
        .iter()
        .map(CoordinateOperation::name)
        .collect::<Vec<_>>()
        .join(" + ");
    let accuracy = get_accuracy_of_steps(&flatten_steps(steps.clone()));
    let accuracy = (accuracy >= 0.0).then(|| stored_accuracy(accuracy));
    let mut identified = Identified::named(name);
    identified.domains = super::derived::joint_extent(&steps)
        .flatten()
        .into_iter()
        .collect();
    let interpolation = concatenation_interpolation(&steps).map(Box::new);
    ConcatenatedOperation {
        identified,
        steps: steps
            .into_iter()
            .map(|operation| ConcatenatedStep {
                operation,
                reverse: false,
            })
            .collect(),
        source: None,
        target: None,
        accuracy,
        version: None,
        computed_name: true,
        source_epoch: None,
        target_epoch: None,
        proj_based: None,
        interpolation,
    }
}

/// `getAccuracy` against `desired`: unknown accuracy is never allowed when a target is set.
fn accuracy_allowed(operation: &CoordinateOperation, desired: f64) -> bool {
    if desired == 0.0 {
        return true;
    }
    (0.0..=desired).contains(&get_accuracy(operation))
}

/// The deriving conversion of `projected` is the operation from `source`.
///
/// An authority match is the database path. Without a registry, the same datum and
/// angular unit are enough, and only when the dimension matches, so a 3D geographic
/// source still goes through the derived-CRS chain.
fn projection_uses_source(source: &Crs, projected: &ProjectedCRS, registry: RegistryUse) -> bool {
    if same_authority(source, projected.base_crs()) {
        return true;
    }
    if registry != RegistryUse::None {
        return false;
    }
    let (Some(geographic), GeodeticCRS::Geographic(base)) =
        (geographic_of(source), projected.base_crs())
    else {
        return false;
    };
    geographic.coordinate_system().axes().len() == base.coordinate_system().axes().len()
        && same_geographic_base(geographic, base)
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
    source: (String, String),
    auth: String,
    code: String,
    table: String,
    bbox: Option<BoundingBox>,
    area: f64,
    accuracy_missing: bool,
    accuracy: f64,
    replacement: Option<(String, String)>,
    replacement_is_unknown_grid: bool,
}

/// Which registered rows [`operations_between`] reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RegistrySearch {
    /// Only `source` to `target`, every row kept. The public
    /// `AuthorityFactory::create_from_coordinate_reference_system_codes`.
    Forward,
    /// As [`Self::Forward`], with the superseded rows dropped.
    ForwardWithoutSuperseded,
    /// Both directions, with inverses of the reverse rows, superseded rows
    /// dropped, and rows restricted to the CRS extents when some intersect.
    Both,
    /// As [`Self::Both`], with the superseded rows kept (`setDiscardSuperseded(false)`).
    BothKeepingSuperseded,
}

/// Registered operations between two CRS read in the authorities of `findOpsInRegistryDirect`.
/// The authorities are tried in order, and the search stops at the first one with operations,
/// except `PROJ` whose operations are kept while the next authority is read too.
fn direct_registry_operations(
    context: &CoordinateOperationContext,
    source: (&str, &str),
    target: (&str, &str),
    crs_boxes: (Option<BoundingBox>, Option<BoundingBox>),
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let mut found = Vec::new();
    for authority in candidate_authorities(&context.db, &context.authority, source.0, target.0)? {
        let search = match context.superseded {
            SupersededOperations::Discard => RegistrySearch::Both,
            SupersededOperations::Keep => RegistrySearch::BothKeepingSuperseded,
        };
        let mut operations = operations_between(
            &context.db,
            &authority,
            source,
            target,
            crs_boxes,
            search,
            context.alternative_grid_names(),
        )?;
        if context.discards_missing_grids() {
            // `rejectOpDueToMissingGrid`: an operation with a grid that is not returned when the
            // grid is neither on disk nor known to the database (`KNOWN_AVAILABLE`), or not on
            // disk (`DISCARD_OPERATION_IF_MISSING_GRID`).
            let mut kept = Vec::with_capacity(operations.len());
            for operation in operations {
                let grids = operation.grids_needed(&context.db, context.grid_availability)?;
                if grids.iter().all(|grid| grid.info.available) {
                    kept.push(operation);
                }
            }
            operations = kept;
        }
        found.extend(operations);
        if authority != "PROJ" && !found.is_empty() {
            break;
        }
    }
    Ok(found)
}

/// `hasAuthorityRestriction` (`factory.cpp:4138-4140`): the SQL of a lookup is restricted to one
/// authority. An empty name and `any` do not restrict it.
fn authority_restricts(name: &str) -> bool {
    !name.is_empty() && name != "any"
}

/// `buildSqlLookForAuthNameCode` (`factory.cpp:10098`): the SQL that matches the rows whose
/// `prefix`-named authority and code are the first identifier of one of `list`, grouped by
/// authority in sorted order. A bound CRS is looked up through its base CRS.
fn auth_name_code_in(list: &[(Crs, i32)], params: &mut Vec<String>, prefix: &str) -> String {
    let first_identifier = |crs: &Crs| -> Option<(String, String)> {
        let identifier = match crs {
            Crs::Bound(bound) => bound.source_crs().identifiers().first(),
            other => other.identifiers().first(),
        }?;
        Some((
            identifier.code_space().to_owned(),
            identifier.code().to_owned(),
        ))
    };
    let ids: Vec<(String, String)> = list
        .iter()
        .filter_map(|(crs, _)| first_identifier(crs))
        .collect();
    let authorities: BTreeSet<&str> = ids.iter().map(|(space, _)| space.as_str()).collect();
    let mut sql = String::from("(");
    for (index, authority) in authorities.iter().enumerate() {
        if index > 0 {
            sql.push_str(" OR ");
        }
        sql.push_str(&format!("( {prefix}auth_name = ? AND {prefix}code IN ("));
        params.push((*authority).to_owned());
        let mut first = true;
        for (space, code) in &ids {
            if space == authority {
                if !first {
                    sql.push(',');
                }
                first = false;
                sql.push('?');
                params.push(code.clone());
            }
        }
        sql.push_str("))");
    }
    sql.push(')');
    sql
}

/// The spelling of an authority name that `AuthorityFactory::create` (`factory.cpp:4400-4417`)
/// gives the known ones: `EPSG`, `ESRI` and `PROJ`, compared without case. Any other name is kept.
fn canonical_authority(name: String) -> String {
    for known in ["EPSG", "ESRI", "PROJ"] {
        if name.eq_ignore_ascii_case(known) {
            return known.to_owned();
        }
    }
    name
}

/// `getCandidateAuthorities`: the authorities a direct search reads, in order. An authority
/// the context names is the only one. Otherwise the table `authority_to_authority_preference`
/// gives the list for the pair of CRS authorities, so that `ESRI` operations are not read for
/// an `EPSG` pair. `""` stands for every authority.
fn candidate_authorities(
    db: &DatabaseContext,
    authority: &str,
    source: &str,
    target: &str,
) -> Result<Vec<String>, FactoryError> {
    let unrestricted = |name: &str| {
        if name == "any" {
            String::new()
        } else {
            name.to_owned()
        }
    };
    if !authority.is_empty() {
        return Ok(vec![unrestricted(authority)]);
    }
    let lookups = [
        (
            "SELECT allowed_authorities FROM authority_to_authority_preference \
             WHERE source_auth_name = ? AND target_auth_name = ?",
            vec![source.to_owned(), target.to_owned()],
        ),
        (
            "SELECT allowed_authorities FROM authority_to_authority_preference \
             WHERE source_auth_name = ? AND target_auth_name = 'any'",
            vec![source.to_owned()],
        ),
        (
            "SELECT allowed_authorities FROM authority_to_authority_preference \
             WHERE source_auth_name = 'any' AND target_auth_name = ?",
            vec![target.to_owned()],
        ),
        (
            "SELECT allowed_authorities FROM authority_to_authority_preference \
             WHERE source_auth_name = 'any' AND target_auth_name = 'any'",
            Vec::new(),
        ),
    ];
    for (sql, params) in lookups {
        if let Some(row) = db.query(sql, &params)?.first() {
            return Ok(row[0].text().split(',').map(unrestricted).collect());
        }
    }
    Ok(vec![String::new()])
}

/// Registered operations between two CRS, as `createFromCoordinateReferenceSystemCodes`.
///
/// Rows are ordered by pseudo area then accuracy. `crs_boxes` are the bounding
/// boxes of the source and target CRS extents.
fn operations_between(
    db: &DatabaseContext,
    authority: &str,
    source: (&str, &str),
    target: (&str, &str),
    crs_boxes: (Option<BoundingBox>, Option<BoundingBox>),
    search: RegistrySearch,
    grids: AlternativeGridNames,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let mut sql = "SELECT cov.source_crs_auth_name, cov.source_crs_code, \
         cov.target_crs_auth_name, cov.target_crs_code, cov.auth_name, cov.code, cov.table_name, \
         extent.south_lat, extent.west_lon, extent.north_lat, extent.east_lon, cov.accuracy, \
         ss.replacement_auth_name, ss.replacement_code, \
         (gt.auth_name IS NOT NULL) AS replacement_is_grid_transform, \
         (ga.proj_grid_name IS NOT NULL) AS replacement_is_known_grid \
         FROM coordinate_operation_view cov \
         JOIN usage ON usage.object_table_name = cov.table_name \
         AND usage.object_auth_name = cov.auth_name AND usage.object_code = cov.code \
         JOIN extent ON extent.auth_name = usage.extent_auth_name AND extent.code = usage.extent_code \
         LEFT JOIN supersession ss ON ss.superseded_table_name = cov.table_name \
         AND ss.superseded_auth_name = cov.auth_name AND ss.superseded_code = cov.code \
         AND ss.superseded_table_name = ss.replacement_table_name AND ss.same_source_target_crs = 1 \
         LEFT JOIN grid_transformation gt ON gt.auth_name = ss.replacement_auth_name \
         AND gt.code = ss.replacement_code \
         LEFT JOIN grid_alternatives ga ON ga.original_grid_name = gt.grid_name \
         WHERE "
        .to_owned();
    let forward = "cov.source_crs_auth_name = ? AND cov.source_crs_code = ? \
         AND cov.target_crs_auth_name = ? AND cov.target_crs_code = ?";
    let reverse = "cov.source_crs_auth_name = ? AND cov.source_crs_code = ? \
         AND cov.target_crs_auth_name = ? AND cov.target_crs_code = ?";
    let mut params = vec![
        source.0.to_owned(),
        source.1.to_owned(),
        target.0.to_owned(),
        target.1.to_owned(),
    ];
    if matches!(
        search,
        RegistrySearch::Both | RegistrySearch::BothKeepingSuperseded
    ) {
        sql.push_str(&format!("(({forward}) OR ({reverse}))"));
        params.extend([
            target.0.to_owned(),
            target.1.to_owned(),
            source.0.to_owned(),
            source.1.to_owned(),
        ]);
    } else {
        sql.push_str(&format!("({forward})"));
    }
    sql.push_str(" AND cov.deprecated = 0");
    if authority_restricts(authority) {
        sql.push_str(" AND cov.auth_name = ?");
        params.push(authority.to_owned());
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
        let replacement = (!row[12].empty()).then(|| (row[12].text(), row[13].text()));
        parsed.push(OperationRow {
            source: (row[0].text(), row[1].text()),
            auth: row[4].text(),
            code: row[5].text(),
            table: row[6].text(),
            bbox: row_box(&row[7], &row[8], &row[9], &row[10]),
            area,
            accuracy_missing,
            accuracy,
            replacement,
            replacement_is_unknown_grid: row[14].text() == "1" && row[15].text() != "1",
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
    let listed: BTreeSet<(String, String)> = parsed
        .iter()
        .map(|row| (row.auth.clone(), row.code.clone()))
        .collect();
    let superseded = |row: &OperationRow| {
        !matches!(
            search,
            RegistrySearch::Forward | RegistrySearch::BothKeepingSuperseded
        ) && row
            .replacement
            .as_ref()
            .is_some_and(|key| !row.replacement_is_unknown_grid && listed.contains(key))
    };
    let kept: Vec<&OperationRow> = parsed.iter().filter(|row| !superseded(row)).collect();
    let intersects_crs = |bbox: &Option<BoundingBox>| {
        [crs_boxes.0, crs_boxes.1]
            .iter()
            .flatten()
            .all(|extent| bbox.as_ref().is_none_or(|bbox| bbox.intersects(extent)))
    };
    let intersecting = kept.iter().any(|row| intersects_crs(&row.bbox));
    let mut seen = BTreeSet::new();
    let mut operations = Vec::new();
    for row in kept {
        if intersecting && !intersects_crs(&row.bbox) {
            continue;
        }
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
        if row.source.0 == source.0 && row.source.1 == source.1 {
            operations.push(operation);
        } else {
            operations.push(operation.inverse());
        }
    }
    Ok(operations)
}

/// Bounding box from the four extent columns, when all are present.
fn row_box(south: &Cell, west: &Cell, north: &Cell, east: &Cell) -> Option<BoundingBox> {
    Some(BoundingBox::new(
        west.f64().ok()?,
        south.f64().ok()?,
        east.f64().ok()?,
        north.f64().ok()?,
    ))
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

/// `ConcatenatedOperation::fixSteps` when no step stores `forward` or `reverse`:
/// a step that arrives at the concatenated source, or leaves from the concatenated
/// target, runs backwards.
fn fix_unspecified_step_directions(steps: &mut Vec<ConcatenatedStep>, source: &Crs, target: &Crs) {
    if steps.len() < 2 {
        return;
    }
    let reverse_first = matches!(
        step_endpoints(&steps[0]),
        (Some(step_source), Some(step_target))
            if !same_registered_crs(step_source, source) && same_registered_crs(step_target, source)
    );
    if reverse_first {
        steps[0].operation = steps[0].operation.inverse();
    }
    let last = steps.len() - 1;
    let reverse_last = matches!(
        step_endpoints(&steps[last]),
        (Some(step_source), Some(step_target))
            if !same_registered_crs(step_target, target) && same_registered_crs(step_source, target)
    );
    if reverse_last {
        steps[last].operation = steps[last].operation.inverse();
    }
    link_geographic_geocentric(steps, source);
    assign_missing_conversion_crs(steps, source, target);
}

/// Intermediate transformations whose CRS name matches the previous target, but
/// one side is geographic and the other geocentric, gain an explicit
/// geographic/geocentric conversion (`EPSG:9103`). A transformation that
/// arrives from the previous target runs backwards.
fn link_geographic_geocentric(steps: &mut Vec<ConcatenatedStep>, source: &Crs) {
    let mut index = 0;
    while index < steps.len() {
        if matches!(steps[index].operation(), CoordinateOperation::Conversion(_)) {
            index += 1;
            continue;
        }
        let Some((step_source, step_target)) = owned_endpoints(&steps[index]) else {
            index += 1;
            continue;
        };
        let Some(previous) = (if index == 0 {
            Some(source.clone())
        } else {
            step_endpoints(&steps[index - 1]).1.cloned()
        }) else {
            index += 1;
            continue;
        };
        if same_registered_crs(&step_source, &previous) {
            index += 1;
            continue;
        }
        if same_registered_crs(&step_target, &previous) {
            steps[index].operation = steps[index].operation.inverse();
            index += 1;
            continue;
        }
        let bridge = geographic_geocentric_link(&previous, &step_source)
            .or_else(|| geographic_geocentric_link(&previous, &step_target));
        if let Some(bridge) = bridge {
            steps.insert(
                index,
                ConcatenatedStep {
                    operation: CoordinateOperation::Transformation(bridge),
                    reverse: false,
                },
            );
            index += 1;
            continue;
        }
        index += 1;
    }
}

fn owned_endpoints(step: &ConcatenatedStep) -> Option<(Crs, Crs)> {
    let (source, target) = step_endpoints(step);
    Some((source?.clone(), target?.clone()))
}

/// `createGeographicGeocentric` when the two CRS share a name and differ only
/// by geographic versus geocentric. The conversion runs from `from` to `to`.
fn geographic_geocentric_link(from: &Crs, to: &Crs) -> Option<Transformation> {
    if from.name() != to.name() {
        return None;
    }
    let from_geographic = geographic_of(from).is_some();
    let to_geographic = geographic_of(to).is_some();
    if from_geographic && is_geocentric_crs(to) {
        geographic_geocentric(from, to)
    } else if is_geocentric_crs(from) && to_geographic {
        geographic_geocentric(to, from).map(|operation| operation.inverse())
    } else {
        None
    }
}

fn is_geocentric_crs(crs: &Crs) -> bool {
    matches!(crs, Crs::Geodetic(GeodeticCRS::Geocentric(_)))
}

/// Conversions in a concatenated operation have no CRS in `proj.db`. PROJ fills
/// them from the neighbouring steps, as `ConcatenatedOperation::fixSteps` does
/// (`src/iso19111/operation/concatenatedoperation.cpp`, the `i == 0` branch at
/// 382-419 and the intermediate and last branches at 420-460 and 461-544). This
/// runs whether or not the steps store their directions: PROJ gates only the
/// direction fixes on `fixDirectionAllowed`, not the CRS assignment. A
/// height-to-depth reversal whose next step has no source gets a depth CRS
/// derived from the previous height (`EPSG:7987`).
fn assign_missing_conversion_crs(
    steps: &mut [ConcatenatedStep],
    concatenated_source: &Crs,
    concatenated_target: &Crs,
) {
    let len = steps.len();
    if len >= 2 {
        let first_is_bare_conversion = matches!(
            steps[0].operation(),
            CoordinateOperation::Conversion(conversion)
                if conversion.source_crs().is_none() && conversion.target_crs().is_none()
        );
        if first_is_bare_conversion {
            if let Some(next) = effective_endpoints(&steps[1]).0.cloned() {
                set_effective_endpoints(&mut steps[0], concatenated_source.clone(), next);
            }
        }
    }
    for index in 1..len {
        let method = match steps[index].operation() {
            CoordinateOperation::Conversion(conversion)
                if conversion.source_crs().is_none() && conversion.target_crs().is_none() =>
            {
                conversion.method().code().to_owned()
            }
            _ => continue,
        };
        let previous = effective_endpoints(&steps[index - 1]).1.cloned();
        let next = (index + 1 < len)
            .then(|| effective_endpoints(&steps[index + 1]).0.cloned())
            .flatten();
        if index + 1 < len {
            if let (Some(source), Some(target)) = (previous.clone(), next) {
                set_effective_endpoints(&mut steps[index], source, target);
                continue;
            }
            if method == "1068" {
                if let Some(Crs::Vertical(vertical)) = previous.as_ref() {
                    if vertical.name().ends_with(" height")
                        && vertical.coordinate_system().axis().direction() == "up"
                    {
                        let depth = depth_crs_from_height(vertical);
                        if let Some(source) = previous {
                            set_effective_endpoints(&mut steps[index], source, depth);
                        }
                    }
                }
            }
        } else if let Some(source) = previous {
            set_effective_endpoints(&mut steps[index], source, concatenated_target.clone());
        }
    }
}

/// The source and target of a step in the direction the concatenated operation runs:
/// a reversed step (`ConcatenatedStep::reverse`) swaps the operation's own pair.
fn effective_endpoints(step: &ConcatenatedStep) -> (Option<&Crs>, Option<&Crs>) {
    let (source, target) = step_endpoints(step);
    if step.reverse {
        (target, source)
    } else {
        (source, target)
    }
}

/// Sets the pair of a bare conversion step in the direction the concatenated operation
/// runs. A reversed step stores the pair the other way round.
fn set_effective_endpoints(step: &mut ConcatenatedStep, source: Crs, target: Crs) {
    if step.reverse {
        set_conversion_endpoints(step, target, source);
    } else {
        set_conversion_endpoints(step, source, target);
    }
}

fn set_conversion_endpoints(step: &mut ConcatenatedStep, source: Crs, target: Crs) {
    if let CoordinateOperation::Conversion(conversion) = &mut step.operation {
        conversion.source = Some(Box::new(source));
        conversion.target = Some(Box::new(target));
    }
}

fn depth_crs_from_height(vertical: &VerticalCRS) -> Crs {
    let stem = vertical
        .name()
        .strip_suffix(" height")
        .unwrap_or(vertical.name());
    let unit = vertical.coordinate_system().axis().unit().clone();
    Crs::Vertical(VerticalCRS {
        identified: Identified::named(format!("{stem} depth")),
        datum: vertical.datum().cloned(),
        ensemble: vertical.datum_ensemble().cloned(),
        coordinate_system: VerticalCS {
            identified: Identified::named(""),
            axis: Axis::new("Gravity-related depth", "D", "down", unit),
        },
        geoid_models: Vec::new(),
    })
}

fn step_endpoints(step: &ConcatenatedStep) -> (Option<&Crs>, Option<&Crs>) {
    match step.operation() {
        CoordinateOperation::Transformation(operation) => {
            (operation.source_crs(), operation.target_crs())
        }
        CoordinateOperation::Conversion(operation) => {
            (operation.source_crs(), operation.target_crs())
        }
        CoordinateOperation::Concatenated(operation) => {
            (operation.source_crs(), operation.target_crs())
        }
    }
}

fn same_registered_crs(left: &Crs, right: &Crs) -> bool {
    match (left.identifiers().first(), right.identifiers().first()) {
        (Some(left_id), Some(right_id))
            if left_id
                .code_space()
                .eq_ignore_ascii_case(right_id.code_space())
                && left_id.code() == right_id.code() =>
        {
            true
        }
        _ => left.is_equivalent_to(right),
    }
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

/// The accuracy as SQLite writes it: a REAL `1.0` is the text `1.0`, not `1`.
fn accuracy_text(cell: &Cell) -> Option<String> {
    match cell {
        Cell::Real(value) => {
            let text = format!("{value}");
            Some(if value.is_finite() && !text.contains('.') {
                format!("{text}.0")
            } else {
                text
            })
        }
        other => optional_text(&other.text()),
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

/// The ellipsoid of the geodetic CRS a CRS is based on.
fn base_ellipsoid(crs: &Crs) -> Option<&Ellipsoid> {
    match crs {
        Crs::Geographic(crs) => crs.ellipsoid(),
        Crs::Geodetic(crs) => crs.ellipsoid(),
        Crs::Projected(crs) => crs.base_crs().ellipsoid(),
        Crs::DerivedProjected(crs) => crs.base_crs().base_crs().ellipsoid(),
        Crs::Bound(crs) => base_ellipsoid(crs.source_crs()),
        _ => None,
    }
}

/// `createOperationsGeodToGeod`: the two ellipsoids must be on the same celestial body, or
/// both on an unnamed body with close semi-major axes. `PROJ_IGNORE_CELESTIAL_BODY` set to a
/// value other than `NO`, `FALSE` or `OFF` skips the check.
fn check_same_celestial_body(source: &Crs, target: &Crs) -> Result<(), FactoryError> {
    const NON_EARTH_BODY: &str = "Non-Earth body";
    let (Some(from), Some(to)) = (base_ellipsoid(source), base_ellipsoid(target)) else {
        return Ok(());
    };
    let (from_body, to_body) = (from.celestial_body(), to.celestial_body());
    if from_body == to_body && from_body != NON_EARTH_BODY {
        return Ok(());
    }
    let named_apart =
        from_body != to_body && from_body != NON_EARTH_BODY && to_body != NON_EARTH_BODY;
    let to_axis = to.semi_major_axis().si_value();
    let far_apart = (from.semi_major_axis().si_value() - to_axis).abs()
        > super::wkt_import::BODY_RELATIVE * to_axis;
    if !named_apart && !far_apart {
        return Ok(());
    }
    let ignore = std::env::var("PROJ_IGNORE_CELESTIAL_BODY");
    if let Ok(value) = &ignore {
        if !["NO", "FALSE", "OFF"]
            .iter()
            .any(|off| value.eq_ignore_ascii_case(off))
        {
            return Ok(());
        }
    }
    let mut message = format!(
        "Source and target ellipsoid do not belong to the same celestial body ({from_body} vs {to_body})."
    );
    if ignore.is_err() {
        message.push_str(
            " You may override this check by setting the PROJ_IGNORE_CELESTIAL_BODY environment variable to YES.",
        );
    }
    Err(FactoryError::Unsupported(message))
}

/// The OGC temporal CRS `AnsiDate`, `JulianDate` or `UnixTime`, which the factory builds
/// without a database row, as `AuthorityFactory::createCoordinateReferenceSystem` does.
fn ogc_temporal_crs(code: &str) -> Option<super::niche::TemporalCRS> {
    use super::niche::{TemporalCRS, TemporalCS, TemporalCsKind, TemporalDatum};
    let day = || UnitOfMeasure::new("day", 86_400.0, UnitKind::Time);
    let (name, datum_name, origin, unit) = match code {
        "AnsiDate" => (
            "Ansi Date",
            "Epoch time for the ANSI date (1-Jan-1601, 00h00 UTC) as day 1.",
            "1600-12-31T00:00:00Z",
            day(),
        ),
        "JulianDate" => (
            "Julian Date",
            "The beginning of the Julian period.",
            "-4714-11-24T12:00:00Z",
            day(),
        ),
        "UnixTime" => (
            "Unix Time",
            "Unix epoch",
            "1970-01-01T00:00:00Z",
            UnitOfMeasure::new("second", 1.0, UnitKind::Time),
        ),
        _ => return None,
    };
    let datum = TemporalDatum::create(datum_name, origin, super::niche::PROLEPTIC_GREGORIAN);
    let axis = Axis::new("Time", "T", "future", unit);
    let coordinate_system = TemporalCS::create(TemporalCsKind::Count, axis);
    let mut crs = TemporalCRS::create(name, datum, coordinate_system);
    crs.identified = Identified::named(name).with_id("OGC", code);
    Some(crs)
}

/// The end of `fixSteps`: a last step that ends on the geocentric or geographic twin of the
/// target CRS gets the `createGeographicGeocentric` conversion to the target.
fn append_geographic_geocentric_tail(steps: &mut Vec<ConcatenatedStep>, target: &Crs) {
    let Some(last) = steps.last() else {
        return;
    };
    let end = if last.reverse {
        last.operation.source_crs()
    } else {
        last.operation.target_crs()
    };
    let Some(end) = end.cloned() else {
        return;
    };
    if let Some(bridge) = geographic_geocentric_link(&end, target) {
        steps.push(ConcatenatedStep {
            operation: CoordinateOperation::Transformation(bridge),
            reverse: false,
        });
    }
}

/// The geodetic CRS of a geodetic or geographic CRS, the `dynamic_pointer_cast` of
/// `createOperationsFromDatabase` (coordinateoperationfactory.cpp:4286-4294).
fn geodetic_of_crs(crs: &Crs) -> Option<GeodeticCRS> {
    match crs {
        Crs::Geographic(geographic) => Some(GeodeticCRS::Geographic(geographic.clone())),
        Crs::Geodetic(geodetic) => Some(geodetic.clone()),
        _ => None,
    }
}

/// `PointMotionOperation::cloneWithEpochs` (singleoperation.cpp:5676-5703): the point motion with
/// both epochs, and the name that says them, `<name> from epoch <s> to epoch <t>`.
fn point_motion_with_epochs(
    mut transformation: Transformation,
    source_epoch: f64,
    target_epoch: f64,
) -> Transformation {
    transformation.source_epoch = Some(source_epoch);
    transformation.target_epoch = Some(target_epoch);
    transformation.identified.name = format!(
        "{} from epoch {} to epoch {}",
        transformation.identified.name,
        super::json::rounded_epoch_in_decimal_year(source_epoch),
        super::json::rounded_epoch_in_decimal_year(target_epoch)
    );
    transformation
}

/// The same unit: the same kind and the same size in SI.
pub(crate) fn same_unit(left: &UnitOfMeasure, right: &UnitOfMeasure) -> bool {
    left.kind() == right.kind()
        && (left.conversion_to_si() - right.conversion_to_si()).abs()
            <= 1e-15 * right.conversion_to_si().abs()
}

/// EPSG code of the identifier of `identifiers` in the EPSG code space, `0` when none.
fn epsg_code_of_identifiers(identifiers: Vec<Identifier>) -> i32 {
    identifiers
        .iter()
        .find(|identifier| identifier.code_space() == "EPSG")
        .and_then(|identifier| identifier.code().parse().ok())
        .unwrap_or(0)
}

/// `getEPSGCode()` of a conversion parameter, `0` when it has none.
fn epsg_code_of_parameter(parameter: &Parameter) -> i32 {
    if parameter.code_space() == "EPSG" {
        parameter.code().parse().unwrap_or(0)
    } else {
        0
    }
}

/// `Conversion::getESRIMethodName` with the parameter list of the ESRI mapping. The CRS target
/// of the conversion is not set on a projected CRS's deriving conversion, so the target-name
/// choices are not taken.
fn esri_method(
    conversion: &Conversion,
    name: &str,
    method_code: i32,
) -> Option<super::esri_parammap::EsriChoice> {
    super::esri_parammap::esri_method(conversion, name, method_code, None)
}

/// `Conversion::getWKT1GDALMethodName`: `Mercator_1SP` for the pseudo Mercator method, else the
/// GDAL name of the mapping of the method.
fn wkt1_gdal_method_name(method_code: i32, name: &str) -> Option<&'static str> {
    if method_code == EPSG_METHOD_POPULAR_VISUALISATION_PSEUDO_MERCATOR {
        return Some("Mercator_1SP");
    }
    projection_mapping(method_code, name).and_then(|mapping| mapping.wkt1_name)
}

/// `getMapping(const OperationMethod *)` (parammappings.cpp:1882): the first mapping whose EPSG
/// code is the method code, or whose WKT2 name is equivalent to the method name.
fn projection_mapping(method_code: i32, name: &str) -> Option<&'static MethodMap> {
    PROJECTION_METHODS
        .iter()
        .chain(OTHER_METHODS)
        .find(|mapping| {
            (method_code != 0 && mapping.epsg_code == method_code)
                || equivalent_name_bigger_differences(mapping.wkt2_name, name)
        })
}

/// `parameterValueMeasure(name)` of a conversion: the measure of the first parameter whose name
/// is equivalent to `name`, when that value is a measure.
fn conversion_measure_by_name(conversion: &Conversion, name: &str) -> Option<Measure> {
    let parameter = conversion
        .parameters()
        .iter()
        .find(|parameter| equivalent_name_bigger_differences(name, parameter.name()))?;
    match parameter.value() {
        ParameterValue::Measure(measure) => Some(measure.clone()),
        _ => None,
    }
}

/// The LIKE pattern text of `text`, with `\`, `_` and `%` escaped (`escapeLikeStr`).
fn escape_like(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('_', "\\_")
        .replace('%', "\\%")
}

/// The geographic CRS with the other order of its two first axes, in two dimensions:
/// `LONG_EAST_LAT_NORTH` becomes `LAT_NORTH_LONG_EAST` and the reverse, as
/// `createProjectedCRSFromExisting` builds it.
fn geographic_with_other_axis_order(crs: &GeographicCRS) -> Option<GeographicCRS> {
    let axes = crs.coordinate_system().axes();
    if axes.len() < 2 {
        return None;
    }
    if !matches!(
        (axes[0].direction(), axes[1].direction()),
        ("east", "north") | ("north", "east")
    ) {
        return None;
    }
    let mut swapped = crs.clone();
    swapped.coordinate_system.axes.swap(0, 1);
    swapped.coordinate_system.axes.truncate(2);
    Some(swapped)
}

const EPSG_METHOD_LCC_2SP: i32 = 9802;
const EPSG_METHOD_POPULAR_VISUALISATION_PSEUDO_MERCATOR: i32 = 1024;
const EPSG_PARAMETER_LAT_1ST_STD_PARALLEL: i32 = 8823;
const EPSG_PARAMETER_LAT_2ND_STD_PARALLEL: i32 = 8824;
const EPSG_PARAMETER_FALSE_EASTING: i32 = 8806;
const EPSG_PARAMETER_EASTING_FALSE_ORIGIN: i32 = 8826;
