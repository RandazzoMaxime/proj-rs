//! SQL that writes an object, and the objects it needs, into an authority of
//! `proj.db`: the insert path of `DatabaseContext::getInsertStatementsFor` in
//! `factory.cpp`.
//!
//! Covered: prime meridian, ellipsoid, geodetic datum, geographic and
//! geocentric CRS, and projected CRS with its conversion, with the units,
//! coordinate systems, and usages they need. A datum ensemble that is not
//! already in the database, a vertical or compound CRS, and a bound CRS are
//! errors. Each call starts from the database as it is; objects inserted by an
//! earlier call are not seen.

use super::equiv::directions_and_units;
use super::factory::{AuthorityFactory, AuthorityObject, DatabaseContext, NameMatch, ObjectType};
use super::model::{
    Axis, CoordinateSystem, Crs, Datum, Ellipsoid, FactoryError, GeodeticCRS,
    GeodeticReferenceFrame, Identifier, ObjectDomain, Parameter, ParameterValue, PrimeMeridian,
    ProjectedCRS, UnitKind, UnitOfMeasure,
};
use super::parammap::{method_by_code, method_by_name, MethodMap};
use super::wkt::format_g;

/// The SQL statements that write `object` into `auth_name` under `code`.
///
/// `allowed` lists the authorities an existing object may be reused from, as
/// `projinfo --authority` names them; `auth_name` is always one of them. The
/// result is empty when `object` is already known under that name.
///
/// # Errors
///
/// `object` has no insert in this build, a database lookup fails, or a part of
/// `object` has no authority code and no PROJ name to stand in for one.
pub fn insert_statements(
    db: &DatabaseContext,
    object: &AuthorityObject,
    auth_name: &str,
    code: &str,
    allowed: &[String],
) -> Result<Vec<String>, FactoryError> {
    let mut writer = Writer {
        db,
        allowed,
        statements: Vec::new(),
    };
    writer.object(object, auth_name, code)?;
    Ok(writer.statements)
}

struct Writer<'a> {
    db: &'a DatabaseContext,
    allowed: &'a [String],
    statements: Vec<String>,
}

/// `%q`: a text value between single quotes, with its quotes doubled.
fn quote(text: &str) -> String {
    text.replace('\'', "''")
}

/// `%f`, which `formatStatement` writes as `toString(double)`: 15 significant
/// digits, or 14 when the first form shows a run of nines.
fn number(value: f64) -> String {
    let text = format_g(value, 15);
    if text.contains("9999999999") {
        format_g(value, 14)
    } else {
        text
    }
}

fn unsupported(what: &str) -> FactoryError {
    FactoryError::Format(format!("cannot insert {what} into a database"))
}

impl Writer<'_> {
    fn push(&mut self, sql: String) {
        self.statements.push(sql);
    }

    /// Authorities whose existing objects may stand in for an object that
    /// belongs to `parent`: the allowed list, then `parent` itself.
    fn authorities(&self, parent: &str) -> Vec<String> {
        let mut authorities = self.allowed.to_vec();
        authorities.push(parent.to_owned());
        authorities
    }

    fn object(
        &mut self,
        object: &AuthorityObject,
        auth: &str,
        code: &str,
    ) -> Result<(), FactoryError> {
        match object {
            AuthorityObject::Ellipsoid(ellipsoid) => self.ellipsoid(ellipsoid, auth, code),
            AuthorityObject::PrimeMeridian(meridian) => self.prime_meridian(meridian, auth, code),
            AuthorityObject::Datum(Datum::Geodetic(datum)) => {
                self.geodetic_datum(datum, auth, code)
            }
            AuthorityObject::Crs(crs) => self.crs(crs, auth, code),
            _ => Err(unsupported("this kind of object")),
        }
    }

    /// `getInsertStatementsFor(CRS)`: nothing when a 100% identical CRS is
    /// already known under `auth:code`, else the CRS's parts and its row.
    fn crs(&mut self, crs: &Crs, auth: &str, code: &str) -> Result<(), FactoryError> {
        for authority in self.authorities(auth) {
            let factory = AuthorityFactory::create(self.db, authority.as_str());
            for (candidate, confidence) in crs.identify_with(Some(&factory)) {
                if confidence != 100 {
                    continue;
                }
                let known = candidate
                    .identifiers()
                    .iter()
                    .any(|id| id.code_space() == auth && id.code() == code);
                if known {
                    return Ok(());
                }
            }
        }
        match crs {
            Crs::Geographic(geographic) => {
                self.geodetic(&GeodeticCRS::Geographic(geographic.clone()), auth, code)
            }
            Crs::Geodetic(geodetic) => self.geodetic(geodetic, auth, code),
            Crs::Projected(projected) => self.projected(projected, auth, code),
            _ => Err(unsupported("this kind of CRS")),
        }
    }

    fn geodetic(&mut self, crs: &GeodeticCRS, auth: &str, code: &str) -> Result<(), FactoryError> {
        let (datum_auth, datum_code) = if let Some(ensemble) = crs.datum_ensemble() {
            let object = AuthorityObject::DatumEnsemble(ensemble.clone());
            self.existing(auth, &object)?
                .ok_or_else(|| unsupported("a datum ensemble that the database does not know"))?
        } else if let Some(datum) = crs.datum() {
            let object = AuthorityObject::Datum(Datum::Geodetic(datum.clone()));
            match self.existing(auth, &object)? {
                Some(known) => known,
                None => {
                    let datum_code = format!("GEODETIC_DATUM_{code}");
                    self.geodetic_datum(datum, auth, &datum_code)?;
                    (auth.to_owned(), datum_code)
                }
            }
        } else {
            return Err(unsupported("a geodetic CRS without a datum"));
        };
        let (cs_auth, cs_code) =
            self.coordinate_system(&crs.coordinate_system(), "GEODETIC_CRS", auth, code)?;
        let axes = crs.coordinate_system().axes().len();
        let kind = match crs {
            GeodeticCRS::Geographic(_) if axes == 3 => "geographic 3D",
            GeodeticCRS::Geocentric(_) if axes == 3 => "geocentric",
            GeodeticCRS::Geographic(_) => "geographic 2D",
            GeodeticCRS::Geocentric(_) | GeodeticCRS::Spherical(_) => {
                return Err(unsupported("this kind of geodetic CRS"))
            }
        };
        self.push(format!(
            "INSERT INTO geodetic_crs VALUES('{}','{}','{}','','{kind}','{}','{}','{}','{}',NULL,0);",
            quote(auth),
            quote(code),
            quote(crs.name()),
            quote(&cs_auth),
            quote(&cs_code),
            quote(&datum_auth),
            quote(&datum_code),
        ));
        self.usages(crs.domains(), "geodetic_crs", auth, code)
    }

    fn projected(
        &mut self,
        crs: &ProjectedCRS,
        auth: &str,
        code: &str,
    ) -> Result<(), FactoryError> {
        let base = crs.base_crs().clone();
        let mut known_base = None;
        'authorities: for authority in self.authorities(auth) {
            let factory = AuthorityFactory::create(self.db, authority.as_str());
            for (candidate, confidence) in base.identify_with(Some(&factory)) {
                if confidence != 100 {
                    continue;
                }
                if let Some(id) = candidate.identifiers().first() {
                    known_base = Some((id.code_space().to_owned(), id.code().to_owned()));
                    break 'authorities;
                }
            }
        }
        let (base_auth, base_code) = match known_base {
            Some(known) => known,
            None => {
                let base_code = format!("GEODETIC_CRS_{code}");
                self.crs(&Crs::Geodetic(base), auth, &base_code)?;
                (auth.to_owned(), base_code)
            }
        };
        let conversion_code = format!("CONVERSION_{code}");
        self.conversion(
            crs.deriving_conversion(),
            crs.domains(),
            auth,
            &conversion_code,
        )?;
        let (cs_auth, cs_code) = self.coordinate_system(
            &CoordinateSystem::Cartesian(crs.coordinate_system().clone()),
            "PROJECTED_CRS",
            auth,
            code,
        )?;
        self.push(format!(
            "INSERT INTO projected_crs VALUES('{}','{}','{}','','{}','{}','{}','{}','{}','{}',NULL,0);",
            quote(auth),
            quote(code),
            quote(crs.name()),
            quote(&cs_auth),
            quote(&cs_code),
            quote(&base_auth),
            quote(&base_code),
            quote(auth),
            quote(&conversion_code),
        ));
        self.usages(crs.domains(), "projected_crs", auth, code)
    }

    /// `getInsertStatementsFor(Conversion)` as the projected CRS path runs it:
    /// the method and its parameters, in the order the method's mapping gives.
    fn conversion(
        &mut self,
        conversion: &super::model::Conversion,
        domains: &[ObjectDomain],
        auth: &str,
        code: &str,
    ) -> Result<(), FactoryError> {
        const MAX_PARAMS: usize = 7;
        let method = conversion.method();
        let (method_auth, method_code) = match method.identifiers().first() {
            Some(id) => (id.code_space().to_owned(), id.code().to_owned()),
            None => match method_by_name(method.name()) {
                Some(map) if map.epsg_code > 0 => ("EPSG".to_owned(), map.epsg_code.to_string()),
                Some(MethodMap {
                    proj_name: Some(proj),
                    proj_aux,
                    ..
                }) => (
                    "PROJ".to_owned(),
                    match proj_aux {
                        Some(aux) => format!("{proj} {aux}"),
                        None => (*proj).to_owned(),
                    },
                ),
                _ => {
                    return Err(FactoryError::Format(
                        "Cannot insert projection with method without identifier".to_owned(),
                    ))
                }
            },
        };
        let mapping = if method_auth == "EPSG" {
            method_code.parse().ok().and_then(method_by_code)
        } else {
            method_by_name(method.name())
        };
        let sources = conversion.parameters();
        if sources.len() > MAX_PARAMS {
            return Err(FactoryError::Format(format!(
                "Cannot insert projection with more than {MAX_PARAMS} parameters"
            )));
        }
        let mut ordered: Vec<&Parameter> = Vec::new();
        if let Some(map) = mapping {
            for wanted in map.params {
                ordered.extend(
                    sources
                        .iter()
                        .filter(|parameter| parameter.name() == wanted.wkt2_name),
                );
            }
        }
        if ordered.len() != sources.len() {
            ordered = sources.iter().collect();
        }
        let mut sql = format!(
            "INSERT INTO conversion VALUES('{}','{}','{}','','{}','{}','{}'",
            quote(auth),
            quote(code),
            quote(conversion.name()),
            quote(&method_auth),
            quote(&method_code),
            quote(method.name()),
        );
        for parameter in &ordered {
            let (param_auth, param_code) = if parameter.code_space().is_empty() {
                let epsg = mapping
                    .and_then(|map| map.params.iter().find(|p| p.wkt2_name == parameter.name()))
                    .map_or(0, |param| param.epsg_code);
                if epsg == 0 {
                    return Err(FactoryError::Format(
                        "Cannot insert projection with method parameter without identifier"
                            .to_owned(),
                    ));
                }
                ("EPSG".to_owned(), epsg.to_string())
            } else {
                (
                    parameter.code_space().to_owned(),
                    parameter.code().to_owned(),
                )
            };
            let ParameterValue::Measure(measure) = parameter.value() else {
                return Err(FactoryError::Format(
                    "Cannot insert projection with non-numeric parameter value".to_owned(),
                ));
            };
            let (unit_auth, unit_code) = self.unit(measure.unit(), auth)?;
            sql.push_str(&format!(
                ",'{}','{}','{}',{},'{}','{}'",
                quote(&param_auth),
                quote(&param_code),
                quote(parameter.name()),
                number(measure.value()),
                quote(&unit_auth),
                quote(&unit_code),
            ));
        }
        for _ in ordered.len()..MAX_PARAMS {
            sql.push_str(",NULL,NULL,NULL,NULL,NULL,NULL");
        }
        sql.push_str(",0);");
        self.push(sql);
        self.usages(domains, "conversion", auth, code)
    }

    fn ellipsoid(
        &mut self,
        ellipsoid: &Ellipsoid,
        auth: &str,
        code: &str,
    ) -> Result<(), FactoryError> {
        let object = AuthorityObject::Ellipsoid(ellipsoid.clone());
        if self.existing(auth, &object)?.as_ref() == Some(&(auth.to_owned(), code.to_owned())) {
            return Ok(());
        }
        let major = ellipsoid.semi_major_axis();
        let body_code = format!("BODY_{code}");
        let (body_auth, body_code) = self.celestial_body(
            major.si_value(),
            &format!("Body of {}", ellipsoid.name()),
            auth,
            &body_code,
        )?;
        let (unit_auth, unit_code) = self.unit(major.unit(), auth)?;
        let (inverse_flattening, semi_minor) =
            if ellipsoid.is_sphere() || ellipsoid.semi_minor_axis().is_some() {
                let minor = match ellipsoid.semi_minor_axis() {
                    Some(minor) => minor.value(),
                    None if ellipsoid.is_sphere() => major.value(),
                    None => major.value() * (1.0 - 1.0 / ellipsoid.inverse_flattening()),
                };
                ("NULL".to_owned(), number(minor))
            } else {
                (number(ellipsoid.inverse_flattening()), "NULL".to_owned())
            };
        self.push(format!(
            "INSERT INTO ellipsoid VALUES('{}','{}','{}','','{}','{}',{},'{}','{}',{},{},0);",
            quote(auth),
            quote(code),
            quote(ellipsoid.name()),
            quote(&body_auth),
            quote(&body_code),
            number(major.value()),
            quote(&unit_auth),
            quote(&unit_code),
            inverse_flattening,
            semi_minor,
        ));
        Ok(())
    }

    /// The celestial body whose semi-major axis is within 0.5% of `semi_major`,
    /// else a new one under `auth`.
    fn celestial_body(
        &mut self,
        semi_major: f64,
        name: &str,
        auth: &str,
        code: &str,
    ) -> Result<(String, String), FactoryError> {
        const TOLERANCE: f64 = 0.005;
        let rows = self.db.text_rows(
            &format!(
                "SELECT auth_name, code, (ABS(semi_major_axis - ?) / semi_major_axis) AS rel_error \
                 FROM celestial_body WHERE rel_error <= {TOLERANCE}"
            ),
            &[format!("{semi_major}")],
        )?;
        if let Some(row) = rows.first() {
            if let [body_auth, body_code, ..] = row.as_slice() {
                return Ok((body_auth.clone(), body_code.clone()));
            }
        }
        self.push(format!(
            "INSERT INTO celestial_body VALUES('{}','{}','{}',{});",
            quote(auth),
            quote(code),
            quote(name),
            number(semi_major),
        ));
        Ok((auth.to_owned(), code.to_owned()))
    }

    fn prime_meridian(
        &mut self,
        meridian: &PrimeMeridian,
        auth: &str,
        code: &str,
    ) -> Result<(), FactoryError> {
        let object = AuthorityObject::PrimeMeridian(meridian.clone());
        if self.existing(auth, &object)?.as_ref() == Some(&(auth.to_owned(), code.to_owned())) {
            return Ok(());
        }
        let longitude = meridian.longitude();
        let (unit_auth, unit_code) = self.unit(longitude.unit(), auth)?;
        self.push(format!(
            "INSERT INTO prime_meridian VALUES('{}','{}','{}',{},'{}','{}',0);",
            quote(auth),
            quote(code),
            quote(meridian.name()),
            number(longitude.value()),
            quote(&unit_auth),
            quote(&unit_code),
        ));
        Ok(())
    }

    fn geodetic_datum(
        &mut self,
        datum: &GeodeticReferenceFrame,
        auth: &str,
        code: &str,
    ) -> Result<(), FactoryError> {
        let object = AuthorityObject::Datum(Datum::Geodetic(datum.clone()));
        if self.existing(auth, &object)?.as_ref() == Some(&(auth.to_owned(), code.to_owned())) {
            return Ok(());
        }
        let ellipsoid = datum.ellipsoid();
        let (ellipsoid_auth, ellipsoid_code) =
            match self.existing(auth, &AuthorityObject::Ellipsoid(ellipsoid.clone()))? {
                Some(known) => known,
                None => {
                    let ellipsoid_code = format!("ELLPS_{code}");
                    self.ellipsoid(ellipsoid, auth, &ellipsoid_code)?;
                    (auth.to_owned(), ellipsoid_code)
                }
            };
        let meridian = datum.prime_meridian();
        let (meridian_auth, meridian_code) =
            match self.existing(auth, &AuthorityObject::PrimeMeridian(meridian.clone()))? {
                Some(known) => known,
                None => {
                    let meridian_code = format!("PM_{code}");
                    self.prime_meridian(meridian, auth, &meridian_code)?;
                    (auth.to_owned(), meridian_code)
                }
            };
        let optional = |text: Option<&str>| match text {
            Some(text) => format!("'{}'", quote(text)),
            None => "NULL".to_owned(),
        };
        let frame_epoch = datum
            .frame_reference_epoch()
            .map_or_else(|| "NULL".to_owned(), number);
        let anchor = datum.anchor().filter(|text| !text.is_empty());
        let anchor_epoch = datum
            .anchor_epoch()
            .map_or_else(|| "NULL".to_owned(), |epoch| format!("{epoch:.3}"));
        self.push(format!(
            "INSERT INTO geodetic_datum VALUES('{}','{}','{}','','{}','{}','{}','{}',{},{frame_epoch},NULL,{},{anchor_epoch},0);",
            quote(auth),
            quote(code),
            quote(datum.name()),
            quote(&ellipsoid_auth),
            quote(&ellipsoid_code),
            quote(&meridian_auth),
            quote(&meridian_code),
            optional(datum.publication_date()),
            optional(anchor),
        ));
        self.usages(datum.domains(), "geodetic_datum", auth, code)
    }

    /// `identifyFromNameOrCode`: the authority and code of a known object
    /// equivalent to `object`. An identifier wins over a name match.
    fn existing(
        &self,
        parent: &str,
        object: &AuthorityObject,
    ) -> Result<Option<(String, String)>, FactoryError> {
        let authorities = self.authorities(parent);
        for id in identifiers_of(object) {
            if !authorities
                .iter()
                .any(|authority| authority == id.code_space())
            {
                continue;
            }
            let factory = AuthorityFactory::create(self.db, id.code_space());
            if let Ok(candidate) = same_kind(&factory, object, id.code()) {
                if equivalent(&candidate, object) {
                    return Ok(Some((id.code_space().to_owned(), id.code().to_owned())));
                }
            }
        }
        for authority in &authorities {
            let factory = AuthorityFactory::create(self.db, authority.as_str());
            let candidates = factory.create_objects_from_name(
                object.name(),
                &[object_type(object)],
                NameMatch::Exact,
                0,
            )?;
            for candidate in candidates {
                let Some(id) = identifiers_of(&candidate).first() else {
                    continue;
                };
                if equivalent(&candidate, object) {
                    return Ok(Some((id.code_space().to_owned(), id.code().to_owned())));
                }
            }
        }
        Ok(None)
    }

    /// `identifyOrInsert` for a unit: its own authority code, else a
    /// database unit with the same kind and factor, else a new row under
    /// `owner`.
    fn unit(
        &mut self,
        unit: &UnitOfMeasure,
        owner: &str,
    ) -> Result<(String, String), FactoryError> {
        if !unit.code_space().is_empty() {
            return Ok((unit.code_space().to_owned(), unit.code().to_owned()));
        }
        let factor = unit.conversion_to_si();
        let well_known = match unit.kind() {
            UnitKind::Linear if factor == 1.0 => Some("9001"),
            UnitKind::Angular
                if (factor - 0.017_453_292_519_943_278).abs()
                    <= 1e-10 * 0.017_453_292_519_943_278 =>
            {
                Some("9102")
            }
            UnitKind::Scale if factor == 1.0 => Some("9201"),
            _ => None,
        };
        if let Some(code) = well_known {
            return Ok(("EPSG".to_owned(), code.to_owned()));
        }
        let unit_type = unit_database_type(unit.kind());
        let mut sql = String::from(
            "SELECT auth_name, code FROM unit_of_measure \
             WHERE abs(conv_factor - ?) <= 1e-10 * conv_factor",
        );
        let mut params = vec![format!("{factor}")];
        if let Some(unit_type) = unit_type {
            sql.push_str(" AND type = ?");
            params.push(unit_type.to_owned());
        }
        sql.push_str(" ORDER BY auth_name, code");
        for row in self.db.text_rows(&sql, &params)? {
            let [row_auth, row_code, ..] = row.as_slice() else {
                continue;
            };
            let factory = AuthorityFactory::create(self.db, row_auth.as_str());
            if factory.create_unit_of_measure(row_code).is_ok() {
                return Ok((row_auth.clone(), row_code.clone()));
            }
        }
        let Some(unit_type) = unit_type else {
            return Err(FactoryError::Format(
                "Cannot insert this type of UnitOfMeasure".to_owned(),
            ));
        };
        let prototype = unit.name().to_uppercase().replace(' ', "_");
        let code = self.free_code("unit_of_measure", owner, &prototype)?;
        self.push(format!(
            "INSERT INTO unit_of_measure VALUES('{}','{}','{}','{}',{},NULL,0);",
            quote(owner),
            quote(&code),
            quote(unit.name()),
            unit_type,
            number(factor),
        ));
        Ok((owner.to_owned(), code))
    }

    /// `identifyOrInsert` for a coordinate system.
    fn coordinate_system(
        &mut self,
        system: &CoordinateSystem,
        owner_type: &str,
        owner: &str,
        owner_code: &str,
    ) -> Result<(String, String), FactoryError> {
        let axes = system.axes();
        if let [axis] = axes.as_slice() {
            let metre = axis.unit().kind() == UnitKind::Linear
                && (axis.unit().conversion_to_si() - 1.0).abs() < 1e-10;
            if metre
                && axis.direction() == "up"
                && matches!(axis.name(), "Up" | "Gravity-related height")
            {
                return Ok(("EPSG".to_owned(), "6499".to_owned()));
            }
        }
        let cs_type = match system {
            CoordinateSystem::Ellipsoidal(_) => Some("ellipsoidal"),
            CoordinateSystem::Cartesian(_) => Some("Cartesian"),
            CoordinateSystem::Vertical(_) => Some("vertical"),
            _ => None,
        };
        let mut sql =
            String::from("SELECT auth_name, code FROM coordinate_system WHERE dimension = ?");
        let mut params = vec![axes.len().to_string()];
        if let Some(cs_type) = cs_type {
            sql.push_str(" AND type = ?");
            params.push(cs_type.to_owned());
        }
        sql.push_str(" ORDER BY auth_name, code");
        let mut known = None;
        for row in self.db.text_rows(&sql, &params)? {
            let [row_auth, row_code, ..] = row.as_slice() else {
                continue;
            };
            let factory = AuthorityFactory::create(self.db, row_auth.as_str());
            let Ok(candidate) = factory.create_coordinate_system(row_code) else {
                continue;
            };
            if same_system(&candidate, system) {
                known = Some((row_auth.clone(), row_code.clone()));
                if row_auth == "EPSG" && matches!(row_code.as_str(), "4400" | "6422" | "6423") {
                    return Ok(known.unwrap_or_default());
                }
            }
        }
        if let Some(known) = known {
            return Ok(known);
        }
        let Some(cs_type) = cs_type else {
            return Err(FactoryError::Format(
                "Cannot insert this type of CoordinateSystem".to_owned(),
            ));
        };
        let code = self.free_code(
            "coordinate_system",
            owner,
            &format!("CS_{owner_type}_{owner_code}"),
        )?;
        self.push(format!(
            "INSERT INTO coordinate_system VALUES('{}','{}','{cs_type}',{});",
            quote(owner),
            quote(&code),
            axes.len(),
        ));
        for (index, axis) in axes.iter().enumerate() {
            let (unit_auth, unit_code) = self.unit(axis.unit(), owner)?;
            self.push(format!(
                "INSERT INTO axis VALUES('{}','{}','{}','{}','{}','{}','{}',{},'{}','{}');",
                quote(owner),
                quote(&format!("{code}_AXIS_{}", index + 1)),
                quote(axis.name()),
                quote(axis.abbreviation()),
                quote(axis.direction()),
                quote(owner),
                quote(&code),
                index + 1,
                quote(&unit_auth),
                quote(&unit_code),
            ));
        }
        Ok((owner.to_owned(), code))
    }

    /// `findFreeCode`: `prototype`, else `prototype_2` to `prototype_9`.
    fn free_code(&self, table: &str, auth: &str, prototype: &str) -> Result<String, FactoryError> {
        for counter in 1..10 {
            let candidate = if counter == 1 {
                prototype.to_owned()
            } else {
                format!("{prototype}_{counter}")
            };
            let taken = !self
                .db
                .text_rows(
                    &format!("SELECT 1 FROM {table} WHERE auth_name = ? AND code = ?"),
                    &[auth.to_owned(), candidate.clone()],
                )?
                .is_empty();
            if !taken {
                return Ok(candidate);
            }
        }
        Err(FactoryError::Format(format!(
            "Cannot insert {table}: too many similar codes"
        )))
    }

    /// `identifyOrInsertUsages`: one `usage` row per domain, with the scope
    /// and extent it names, from the database when one matches, else new.
    fn usages(
        &mut self,
        domains: &[ObjectDomain],
        table: &str,
        auth: &str,
        code: &str,
    ) -> Result<(), FactoryError> {
        let upper = table.to_uppercase();
        let mut usage_code = String::from("USAGE_");
        if !code.starts_with(&upper) {
            usage_code.push_str(&upper);
            usage_code.push('_');
        }
        usage_code.push_str(code);
        if domains.is_empty() {
            self.push(format!(
                "INSERT INTO usage VALUES('{}','{}','{table}','{}','{}','PROJ','EXTENT_UNKNOWN','PROJ','SCOPE_UNKNOWN');",
                quote(auth),
                quote(&usage_code),
                quote(auth),
                quote(code),
            ));
            return Ok(());
        }
        let mut lookup_authorities = vec![auth.to_owned()];
        lookup_authorities.extend(self.allowed.iter().cloned());
        for (position, domain) in domains.iter().enumerate() {
            let (scope_auth, scope_code) = match domain.scope() {
                Some(scope) => {
                    let mut params = vec![scope.to_owned()];
                    params.extend(lookup_authorities.iter().cloned());
                    let rows = self.db.text_rows(
                        &format!(
                            "SELECT auth_name, code, (CASE WHEN auth_name = 'EPSG' THEN 0 ELSE 1 END) AS order_idx \
                             FROM scope WHERE scope = ? AND deprecated = 0 AND {} \
                             ORDER BY order_idx, auth_name, code",
                            in_list(lookup_authorities.len())
                        ),
                        &params,
                    )?;
                    match rows.first().map(Vec::as_slice) {
                        Some([row_auth, row_code, ..]) => (row_auth.clone(), row_code.clone()),
                        _ => {
                            let scope_code = format!("SCOPE_{table}_{code}");
                            self.push(format!(
                                "INSERT INTO scope VALUES('{}','{}','{}',0);",
                                quote(auth),
                                quote(&scope_code),
                                quote(scope),
                            ));
                            (auth.to_owned(), scope_code)
                        }
                    }
                }
                None => ("PROJ".to_owned(), "SCOPE_UNKNOWN".to_owned()),
            };
            let (extent_auth, extent_code) = match (
                domain.south_bound_latitude(),
                domain.north_bound_latitude(),
                domain.west_bound_longitude(),
                domain.east_bound_longitude(),
            ) {
                (Some(south), Some(north), Some(west), Some(east)) => {
                    let mut params = vec![
                        format!("{south}"),
                        format!("{north}"),
                        format!("{west}"),
                        format!("{east}"),
                    ];
                    params.extend(lookup_authorities.iter().cloned());
                    let rows = self.db.text_rows(
                        &format!(
                            "SELECT auth_name, code, (CASE WHEN auth_name = 'EPSG' THEN 0 ELSE 1 END) AS order_idx \
                             FROM extent WHERE south_lat = ? AND north_lat = ? AND west_lon = ? \
                             AND east_lon = ? AND deprecated = 0 AND {} \
                             ORDER BY order_idx, auth_name, code",
                            in_list(lookup_authorities.len())
                        ),
                        &params,
                    )?;
                    match rows.first().map(Vec::as_slice) {
                        Some([row_auth, row_code, ..]) => (row_auth.clone(), row_code.clone()),
                        _ => {
                            let extent_code = format!("EXTENT_{table}_{code}");
                            let description = domain
                                .description()
                                .filter(|text| !text.is_empty())
                                .unwrap_or("unknown");
                            self.push(format!(
                                "INSERT INTO extent VALUES('{}','{}','{}','{}',{},{},{},{},0);",
                                quote(auth),
                                quote(&extent_code),
                                quote(description),
                                quote(description),
                                number(south),
                                number(north),
                                number(west),
                                number(east),
                            ));
                            (auth.to_owned(), extent_code)
                        }
                    }
                }
                _ => ("PROJ".to_owned(), "EXTENT_UNKNOWN".to_owned()),
            };
            if domains.len() > 1 {
                usage_code.push('_');
                usage_code.push_str(&(position + 1).to_string());
            }
            self.push(format!(
                "INSERT INTO usage VALUES('{}','{}','{table}','{}','{}','{}','{}','{}','{}');",
                quote(auth),
                quote(&usage_code),
                quote(auth),
                quote(code),
                quote(&extent_auth),
                quote(&extent_code),
                quote(&scope_auth),
                quote(&scope_code),
            ));
        }
        Ok(())
    }
}

/// `CoordinateSystem::_isEquivalentTo` with `EQUIVALENT`: the same kind, and
/// axes that match in direction and unit. Names are not compared, since the
/// database's coordinate systems have none.
fn same_system(candidate: &CoordinateSystem, system: &CoordinateSystem) -> bool {
    let owned =
        |system: &CoordinateSystem| -> Vec<Axis> { system.axes().into_iter().cloned().collect() };
    candidate.wkt2_type() == system.wkt2_type()
        && directions_and_units(&owned(candidate), &owned(system))
}

/// `?, ?, ...` for `count` authorities, as `auth_name IN (...)` takes them.
fn in_list(count: usize) -> String {
    format!("auth_name IN ({})", vec!["?"; count].join(","))
}

fn unit_database_type(kind: UnitKind) -> Option<&'static str> {
    match kind {
        UnitKind::Linear => Some("length"),
        UnitKind::Angular => Some("angle"),
        UnitKind::Scale => Some("scale"),
        UnitKind::Time => Some("time"),
        UnitKind::None | UnitKind::Unknown => None,
    }
}

fn object_type(object: &AuthorityObject) -> ObjectType {
    match object {
        AuthorityObject::Ellipsoid(_) => ObjectType::Ellipsoid,
        AuthorityObject::PrimeMeridian(_) => ObjectType::PrimeMeridian,
        AuthorityObject::DatumEnsemble(_) => ObjectType::DatumEnsemble,
        _ => ObjectType::GeodeticReferenceFrame,
    }
}

fn identifiers_of(object: &AuthorityObject) -> &[Identifier] {
    match object {
        AuthorityObject::Ellipsoid(ellipsoid) => ellipsoid.identifiers(),
        AuthorityObject::PrimeMeridian(meridian) => meridian.identifiers(),
        AuthorityObject::Datum(datum) => datum.identifiers(),
        AuthorityObject::DatumEnsemble(ensemble) => ensemble.identifiers(),
        _ => &[],
    }
}

/// The object `code` names in `factory`, of the same kind as `object`.
fn same_kind(
    factory: &AuthorityFactory,
    object: &AuthorityObject,
    code: &str,
) -> Result<AuthorityObject, FactoryError> {
    Ok(match object {
        AuthorityObject::Ellipsoid(_) => {
            AuthorityObject::Ellipsoid(factory.create_ellipsoid(code)?)
        }
        AuthorityObject::PrimeMeridian(_) => {
            AuthorityObject::PrimeMeridian(factory.create_prime_meridian(code)?)
        }
        AuthorityObject::Datum(Datum::Geodetic(_)) => {
            AuthorityObject::Datum(Datum::Geodetic(factory.create_geodetic_datum(code)?))
        }
        AuthorityObject::DatumEnsemble(_) => {
            AuthorityObject::DatumEnsemble(factory.create_datum_ensemble(code)?)
        }
        _ => return Err(unsupported("this kind of object")),
    })
}

/// `isEquivalentTo` with the EQUIVALENT criterion, for the kinds `same_kind` makes.
fn equivalent(candidate: &AuthorityObject, object: &AuthorityObject) -> bool {
    match (candidate, object) {
        (AuthorityObject::Ellipsoid(known), AuthorityObject::Ellipsoid(other)) => {
            known.is_equivalent_to(other)
        }
        (AuthorityObject::PrimeMeridian(known), AuthorityObject::PrimeMeridian(other)) => {
            known.is_equivalent_to(other)
        }
        (
            AuthorityObject::Datum(Datum::Geodetic(known)),
            AuthorityObject::Datum(Datum::Geodetic(other)),
        ) => known.is_equivalent_to(other),
        (AuthorityObject::DatumEnsemble(known), AuthorityObject::DatumEnsemble(other)) => {
            known.is_equivalent_to(other)
        }
        _ => false,
    }
}
