//! ISO 19111 object graph the unit tests construct and export.

use std::fmt;

/// Opening `proj.db`, an authority code, or a WKT export failed.
#[derive(Debug)]
pub enum FactoryError {
    /// No row for this authority and code.
    NotFound {
        /// Kind of object that was requested.
        object: &'static str,
        /// Authority name, such as `EPSG`.
        authority: String,
        /// Authority code.
        code: String,
    },
    /// The row exists but cannot be turned into an object.
    Factory {
        /// Kind of object that was requested.
        object: &'static str,
        /// Authority name.
        authority: String,
        /// Authority code.
        code: String,
        /// Cause, without a coordinate substituted for the failure.
        detail: String,
    },
    /// The database rejected a statement.
    Sql(String),
    /// A WKT convention cannot represent this object.
    Format(String),
    /// A constructor rejected its arguments.
    Invalid(String),
}

impl fmt::Display for FactoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound {
                object,
                authority,
                code,
            } => write!(f, "{object} not found ({authority}:{code})"),
            Self::Factory {
                object,
                authority,
                code,
                detail,
            } => write!(f, "{object} {authority}:{code}: {detail}"),
            Self::Sql(detail) | Self::Format(detail) | Self::Invalid(detail) => f.write_str(detail),
        }
    }
}

impl std::error::Error for FactoryError {}

impl FactoryError {
    pub(crate) fn missing(object: &'static str, authority: &str, code: &str) -> Self {
        Self::NotFound {
            object,
            authority: authority.to_owned(),
            code: code.to_owned(),
        }
    }

    pub(crate) fn wrap(
        object: &'static str,
        authority: &str,
        code: &str,
        detail: impl Into<String>,
    ) -> Self {
        Self::Factory {
            object,
            authority: authority.to_owned(),
            code: code.to_owned(),
            detail: detail.into(),
        }
    }
}

/// Authority code, for example EPSG:4326.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identifier {
    pub(crate) code_space: String,
    pub(crate) code: String,
}

impl Identifier {
    /// Code space such as `EPSG`.
    #[must_use]
    pub fn code_space(&self) -> &str {
        &self.code_space
    }

    /// Code such as `4326`.
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    pub(crate) fn new(code_space: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            code_space: code_space.into(),
            code: code.into(),
        }
    }
}

/// Scope and area recorded for one usage of an object.
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectDomain {
    pub(crate) scope: Option<String>,
    pub(crate) area: Option<String>,
    pub(crate) south: Option<f64>,
    pub(crate) west: Option<f64>,
    pub(crate) north: Option<f64>,
    pub(crate) east: Option<f64>,
    pub(crate) vertical_min: Option<f64>,
    pub(crate) vertical_max: Option<f64>,
    pub(crate) vertical_unit: Option<UnitOfMeasure>,
    pub(crate) temporal_start: Option<String>,
    pub(crate) temporal_stop: Option<String>,
}

impl ObjectDomain {
    /// Scope text, when the database has one.
    #[must_use]
    pub fn scope(&self) -> Option<&str> {
        self.scope.as_deref()
    }

    /// Area description, when the database has one.
    #[must_use]
    pub fn area(&self) -> Option<&str> {
        self.area.as_deref()
    }

    /// Extent description. This is [`Self::area`].
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.area()
    }

    /// West bound in degrees, when the extent has a box.
    #[must_use]
    pub fn west_bound_longitude(&self) -> Option<f64> {
        self.west
    }

    /// East bound in degrees, when the extent has a box.
    #[must_use]
    pub fn east_bound_longitude(&self) -> Option<f64> {
        self.east
    }

    /// North bound in degrees, when the extent has a box.
    #[must_use]
    pub fn north_bound_latitude(&self) -> Option<f64> {
        self.north
    }

    /// South bound in degrees, when the extent has a box.
    #[must_use]
    pub fn south_bound_latitude(&self) -> Option<f64> {
        self.south
    }

    /// Lower bound of the vertical extent, in [`Self::vertical_unit`].
    #[must_use]
    pub fn vertical_minimum(&self) -> Option<f64> {
        self.vertical_min
    }

    /// Upper bound of the vertical extent, in [`Self::vertical_unit`].
    #[must_use]
    pub fn vertical_maximum(&self) -> Option<f64> {
        self.vertical_max
    }

    /// Unit of the vertical extent. PROJ uses the metre when WKT omits one.
    #[must_use]
    pub fn vertical_unit(&self) -> Option<&UnitOfMeasure> {
        self.vertical_unit.as_ref()
    }

    /// Start of the temporal extent.
    #[must_use]
    pub fn temporal_start(&self) -> Option<&str> {
        self.temporal_start.as_deref()
    }

    /// End of the temporal extent.
    #[must_use]
    pub fn temporal_stop(&self) -> Option<&str> {
        self.temporal_stop.as_deref()
    }

    /// Same description and bounding box. Scope is not part of an extent.
    #[must_use]
    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        self.area == other.area
            && same_optional(self.south, other.south)
            && same_optional(self.west, other.west)
            && same_optional(self.north, other.north)
            && same_optional(self.east, other.east)
            && same_optional(self.vertical_min, other.vertical_min)
            && same_optional(self.vertical_max, other.vertical_max)
            && self.temporal_start == other.temporal_start
            && self.temporal_stop == other.temporal_stop
    }
}

fn same_optional(left: Option<f64>, right: Option<f64>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left == right,
        (None, None) => true,
        _ => false,
    }
}

/// Name, identifiers, remarks, and usages shared by identified objects.
#[derive(Clone, Debug)]
pub(crate) struct Identified {
    pub(crate) name: String,
    pub(crate) identifiers: Vec<Identifier>,
    pub(crate) remarks: String,
    pub(crate) deprecated: bool,
    pub(crate) domains: Vec<ObjectDomain>,
    pub(crate) anchor: Option<String>,
    pub(crate) anchor_epoch: Option<f64>,
    pub(crate) publication_date: Option<String>,
}

impl Identified {
    pub(crate) fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            identifiers: Vec::new(),
            remarks: String::new(),
            deprecated: false,
            domains: Vec::new(),
            anchor: None,
            anchor_epoch: None,
            publication_date: None,
        }
    }

    pub(crate) fn with_id(mut self, code_space: &str, code: &str) -> Self {
        if !code_space.is_empty() && !code.is_empty() {
            self.identifiers.push(Identifier::new(code_space, code));
        }
        self
    }
}

impl Default for Identified {
    fn default() -> Self {
        Self::named(String::new())
    }
}

/// Linear, angular, scale, or time unit, with its factor to the SI unit.
#[derive(Clone, Debug, PartialEq)]
pub struct UnitOfMeasure {
    pub(crate) name: String,
    pub(crate) factor: f64,
    pub(crate) kind: UnitKind,
    pub(crate) code_space: String,
    pub(crate) code: String,
}

/// What quantity a [`UnitOfMeasure`] converts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitKind {
    /// Length. The SI unit is the metre.
    Linear,
    /// Angle. The SI unit is the radian.
    Angular,
    /// Scale, including unity and parts per million.
    Scale,
    /// Time. The SI unit is the second.
    Time,
    /// No conversion, used by an ordinal axis.
    None,
    /// A database type this crate does not classify.
    Unknown,
}

impl UnitOfMeasure {
    /// Unit named `name`, converting to the SI unit of `kind` by `factor`.
    #[must_use]
    pub fn new(name: impl Into<String>, factor: f64, kind: UnitKind) -> Self {
        Self {
            name: name.into(),
            factor,
            kind,
            code_space: String::new(),
            code: String::new(),
        }
    }

    /// SI metre, EPSG:9001.
    #[must_use]
    pub fn metre() -> Self {
        Self {
            name: "metre".to_owned(),
            factor: 1.0,
            kind: UnitKind::Linear,
            code_space: "EPSG".to_owned(),
            code: "9001".to_owned(),
        }
    }

    /// Degree, `π/180` radians. EPSG:9122.
    #[must_use]
    pub fn degree() -> Self {
        Self {
            name: "degree".to_owned(),
            factor: std::f64::consts::PI / 180.0,
            kind: UnitKind::Angular,
            code_space: "EPSG".to_owned(),
            code: "9122".to_owned(),
        }
    }

    /// Grad, `π/200` radians. EPSG:9105.
    #[must_use]
    pub fn grad() -> Self {
        Self {
            name: "grad".to_owned(),
            factor: std::f64::consts::PI / 200.0,
            kind: UnitKind::Angular,
            code_space: "EPSG".to_owned(),
            code: "9105".to_owned(),
        }
    }

    /// International foot, 0.3048 metre. EPSG:9002.
    #[must_use]
    pub fn foot() -> Self {
        Self {
            name: "foot".to_owned(),
            factor: 0.3048,
            kind: UnitKind::Linear,
            code_space: "EPSG".to_owned(),
            code: "9002".to_owned(),
        }
    }

    /// Scale of unity. It has no authority code, so WKT omits an id on it.
    #[must_use]
    pub fn unity() -> Self {
        Self {
            name: "unity".to_owned(),
            factor: 1.0,
            kind: UnitKind::Scale,
            code_space: String::new(),
            code: String::new(),
        }
    }

    /// Unit name. PROJ compares units by this string.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Multiplier that converts a value in this unit to the SI unit.
    #[must_use]
    pub fn conversion_to_si(&self) -> f64 {
        self.factor
    }

    /// Quantity this unit measures.
    #[must_use]
    pub fn kind(&self) -> UnitKind {
        self.kind
    }

    /// Authority code space, or empty when the unit is anonymous.
    #[must_use]
    pub fn code_space(&self) -> &str {
        &self.code_space
    }

    /// Authority code, or empty when the unit is anonymous.
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    pub(crate) fn same_name(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

/// A scalar and the unit it is expressed in.
#[derive(Clone, Debug)]
pub struct Measure {
    pub(crate) value: f64,
    pub(crate) unit: UnitOfMeasure,
}

impl Measure {
    /// Numeric value, in [`Self::unit`].
    #[must_use]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Unit of [`Self::value`].
    #[must_use]
    pub fn unit(&self) -> &UnitOfMeasure {
        &self.unit
    }

    /// Value converted to the SI unit of the same kind.
    #[must_use]
    pub fn si_value(&self) -> f64 {
        self.value * self.unit.factor
    }

    pub(crate) fn convert_to(&self, target: &UnitOfMeasure) -> f64 {
        if target.factor == 0.0 {
            return self.value;
        }
        self.si_value() / target.factor
    }

    /// Angle in degrees.
    #[must_use]
    pub fn angle(value: f64) -> Self {
        Self {
            value,
            unit: UnitOfMeasure::degree(),
        }
    }

    /// Length in metres.
    #[must_use]
    pub fn length(value: f64) -> Self {
        Self {
            value,
            unit: UnitOfMeasure::metre(),
        }
    }

    /// Scale factor, unity.
    #[must_use]
    pub fn scale(value: f64) -> Self {
        Self {
            value,
            unit: UnitOfMeasure::unity(),
        }
    }
}

/// Reference ellipsoid.
#[derive(Clone, Debug)]
pub struct Ellipsoid {
    pub(crate) identified: Identified,
    pub(crate) semi_major: Measure,
    pub(crate) inverse_flattening: Option<f64>,
    pub(crate) semi_minor: Option<Measure>,
    pub(crate) body: String,
}

impl Ellipsoid {
    /// Name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.identified.name
    }

    /// Authority identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[Identifier] {
        &self.identified.identifiers
    }

    /// Semi-major axis, in its stated unit.
    #[must_use]
    pub fn semi_major_axis(&self) -> &Measure {
        &self.semi_major
    }

    /// Inverse flattening, or `0` for a sphere.
    ///
    /// A two-axis ellipsoid uses `a / (a - b)`.
    #[must_use]
    pub fn inverse_flattening(&self) -> f64 {
        if let Some(value) = self.inverse_flattening {
            return value;
        }
        if let Some(minor) = &self.semi_minor {
            let major = self.semi_major.si_value();
            let minor = minor.si_value();
            if major == minor {
                return 0.0;
            }
            return major / (major - minor);
        }
        0.0
    }

    /// Celestial body name from the database.
    #[must_use]
    pub fn celestial_body(&self) -> &str {
        &self.body
    }

    /// Semi-minor axis, when the database stored `b` instead of inverse flattening.
    #[must_use]
    pub fn semi_minor_axis(&self) -> Option<&Measure> {
        self.semi_minor.as_ref()
    }

    /// Inverse flattening as stored, not computed from `b`.
    #[must_use]
    pub fn declared_inverse_flattening(&self) -> Option<f64> {
        self.inverse_flattening
    }

    /// Sphere: stored `b` equals `a`, or inverse flattening is zero.
    #[must_use]
    pub fn is_sphere(&self) -> bool {
        if let Some(minor) = &self.semi_minor {
            let major = self.semi_major.si_value();
            return (minor.si_value() - major).abs() <= 1e-10 * major.abs();
        }
        self.inverse_flattening == Some(0.0)
    }

    /// Flattened sphere with no authority code.
    ///
    /// This is `Ellipsoid::createFlattenedSphere` for a name that is not in
    /// `proj.db`. An empty name stays empty.
    #[must_use]
    pub fn create_flattened_sphere(
        name: impl Into<String>,
        semi_major_metre: f64,
        inverse_flattening: f64,
    ) -> Self {
        Self {
            identified: Identified::named(name),
            semi_major: Measure {
                value: semi_major_metre,
                unit: UnitOfMeasure::metre(),
            },
            inverse_flattening: Some(inverse_flattening),
            semi_minor: None,
            body: "Earth".to_owned(),
        }
    }

    /// WGS 84, EPSG:7030.
    #[must_use]
    pub fn wgs84() -> Self {
        Self::flattened("WGS 84", "7030", 6_378_137.0, 298.257_223_563)
    }

    /// GRS 1980, EPSG:7019.
    #[must_use]
    pub fn grs80() -> Self {
        Self::flattened("GRS 1980", "7019", 6_378_137.0, 298.257_222_101)
    }

    /// Clarke 1866, EPSG:7008, given by semi-minor axis.
    #[must_use]
    pub fn clarke_1866() -> Self {
        Self::two_axis("Clarke 1866", "7008", 6_378_206.4, 6_356_583.8)
    }

    /// Clarke 1880 (IGN), EPSG:7011, with the flattening used by EPSG:4807.
    #[must_use]
    pub fn clarke_1880_ign() -> Self {
        Self::flattened(
            "Clarke 1880 (IGN)",
            "7011",
            6_378_249.2,
            293.466_021_293_626_9,
        )
    }

    fn flattened(name: &str, code: &str, major: f64, inverse_flattening: f64) -> Self {
        Self {
            identified: Identified::named(name).with_id("EPSG", code),
            semi_major: Measure {
                value: major,
                unit: UnitOfMeasure::metre(),
            },
            inverse_flattening: Some(inverse_flattening),
            semi_minor: None,
            body: "Earth".to_owned(),
        }
    }

    fn two_axis(name: &str, code: &str, major: f64, minor: f64) -> Self {
        Self {
            identified: Identified::named(name).with_id("EPSG", code),
            semi_major: Measure {
                value: major,
                unit: UnitOfMeasure::metre(),
            },
            inverse_flattening: None,
            semi_minor: Some(Measure {
                value: minor,
                unit: UnitOfMeasure::metre(),
            }),
            body: "Earth".to_owned(),
        }
    }
}

/// Prime meridian.
#[derive(Clone, Debug)]
pub struct PrimeMeridian {
    pub(crate) identified: Identified,
    pub(crate) longitude: Measure,
}

impl PrimeMeridian {
    /// Name. An empty name exports as `Greenwich`.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.identified.name
    }

    /// Authority identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[Identifier] {
        &self.identified.identifiers
    }

    /// Longitude of the meridian, in its stated unit.
    #[must_use]
    pub fn longitude(&self) -> &Measure {
        &self.longitude
    }

    /// Greenwich, EPSG:8901.
    #[must_use]
    pub fn greenwich() -> Self {
        Self {
            identified: Identified::named("Greenwich").with_id("EPSG", "8901"),
            longitude: Measure {
                value: 0.0,
                unit: UnitOfMeasure::degree(),
            },
        }
    }

    /// Paris, EPSG:8903, longitude in grad.
    #[must_use]
    pub fn paris() -> Self {
        Self {
            identified: Identified::named("Paris").with_id("EPSG", "8903"),
            longitude: Measure {
                value: 2.596_921_3,
                unit: UnitOfMeasure::grad(),
            },
        }
    }
}

/// Geodetic datum, including a dynamic frame when an epoch is set.
#[derive(Clone, Debug)]
pub struct GeodeticReferenceFrame {
    pub(crate) identified: Identified,
    pub(crate) ellipsoid: Ellipsoid,
    pub(crate) prime_meridian: PrimeMeridian,
    pub(crate) frame_epoch: Option<f64>,
    pub(crate) deformation_model: Option<String>,
}

impl GeodeticReferenceFrame {
    /// Name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.identified.name
    }

    /// Authority identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[Identifier] {
        &self.identified.identifiers
    }

    /// Ellipsoid of the frame.
    #[must_use]
    pub fn ellipsoid(&self) -> &Ellipsoid {
        &self.ellipsoid
    }

    /// Prime meridian of the frame.
    #[must_use]
    pub fn prime_meridian(&self) -> &PrimeMeridian {
        &self.prime_meridian
    }

    /// Frame reference epoch in years, for a dynamic datum.
    #[must_use]
    pub fn frame_reference_epoch(&self) -> Option<f64> {
        self.frame_epoch
    }

    /// Anchor definition.
    #[must_use]
    pub fn anchor(&self) -> Option<&str> {
        self.identified.anchor.as_deref()
    }

    /// Anchor epoch in years.
    #[must_use]
    pub fn anchor_epoch(&self) -> Option<f64> {
        self.identified.anchor_epoch
    }

    /// Deformation model or velocity grid name of a dynamic datum.
    #[must_use]
    pub fn deformation_model(&self) -> Option<&str> {
        self.deformation_model.as_deref()
    }

    /// Usages recorded for this datum.
    #[must_use]
    pub fn domains(&self) -> &[ObjectDomain] {
        &self.identified.domains
    }

    /// Publication date `YYYY-MM-DD`, when the database has one.
    #[must_use]
    pub fn publication_date(&self) -> Option<&str> {
        self.identified.publication_date.as_deref()
    }

    /// World Geodetic System 1984, EPSG:6326. This is the realization, not the ensemble.
    #[must_use]
    pub fn epsg_6326() -> Self {
        Self::epsg(
            "World Geodetic System 1984",
            "6326",
            Ellipsoid::wgs84(),
            PrimeMeridian::greenwich(),
        )
    }

    /// North American Datum 1927, EPSG:6267.
    #[must_use]
    pub fn epsg_6267() -> Self {
        Self::epsg(
            "North American Datum 1927",
            "6267",
            Ellipsoid::clarke_1866(),
            PrimeMeridian::greenwich(),
        )
    }

    /// Geodetic datum with no anchor and no epoch.
    ///
    /// This is `GeodeticReferenceFrame::create` when the property map only
    /// carries a name. An empty name stays empty.
    #[must_use]
    pub fn create(
        name: impl Into<String>,
        ellipsoid: Ellipsoid,
        prime_meridian: PrimeMeridian,
    ) -> Self {
        Self {
            identified: Identified::named(name),
            ellipsoid,
            prime_meridian,
            frame_epoch: None,
            deformation_model: None,
        }
    }

    /// Datum with an empty name, the WGS 84 ellipsoid, and Greenwich.
    ///
    /// This is `GeodeticReferenceFrame::create(PropertyMap(), Ellipsoid::WGS84, Greenwich)`.
    #[must_use]
    pub fn anonymous_wgs84() -> Self {
        Self {
            identified: Identified::named(""),
            ellipsoid: Ellipsoid::wgs84(),
            prime_meridian: PrimeMeridian::greenwich(),
            frame_epoch: None,
            deformation_model: None,
        }
    }

    /// North American Datum 1983, EPSG:6269.
    #[must_use]
    pub fn epsg_6269() -> Self {
        Self::epsg(
            "North American Datum 1983",
            "6269",
            Ellipsoid::grs80(),
            PrimeMeridian::greenwich(),
        )
    }

    fn epsg(name: &str, code: &str, ellipsoid: Ellipsoid, prime_meridian: PrimeMeridian) -> Self {
        Self {
            identified: Identified::named(name).with_id("EPSG", code),
            ellipsoid,
            prime_meridian,
            frame_epoch: None,
            deformation_model: None,
        }
    }
}

/// How a vertical datum is realized. ISO 19111 `RealizationMethod`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RealizationMethod {
    /// Levelling.
    Levelling,
    /// Geoid model.
    Geoid,
    /// Tidal surface.
    Tidal,
}

/// Vertical datum.
#[derive(Clone, Debug)]
pub struct VerticalReferenceFrame {
    pub(crate) identified: Identified,
    /// WKT1 datum type code. EPSG-style vertical datums use `2005`.
    pub(crate) wkt1_type: String,
    pub(crate) frame_epoch: Option<f64>,
    pub(crate) deformation_model: Option<String>,
    pub(crate) realization_method: Option<RealizationMethod>,
}

impl VerticalReferenceFrame {
    /// Vertical datum with `name` and no anchor.
    #[must_use]
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            identified: Identified::named(name),
            wkt1_type: "2005".to_owned(),
            frame_epoch: None,
            deformation_model: None,
            realization_method: None,
        }
    }

    /// Name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.identified.name
    }

    /// Authority identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[Identifier] {
        &self.identified.identifiers
    }

    /// Frame reference epoch in years, when the vertical datum is dynamic.
    #[must_use]
    pub fn frame_reference_epoch(&self) -> Option<f64> {
        self.frame_epoch
    }

    /// Usages recorded for this datum.
    #[must_use]
    pub fn domains(&self) -> &[ObjectDomain] {
        &self.identified.domains
    }

    /// Publication date `YYYY-MM-DD`, when the database has one.
    #[must_use]
    pub fn publication_date(&self) -> Option<&str> {
        self.identified.publication_date.as_deref()
    }

    /// Anchor definition.
    #[must_use]
    pub fn anchor(&self) -> Option<&str> {
        self.identified.anchor.as_deref()
    }

    /// Anchor epoch in years.
    #[must_use]
    pub fn anchor_epoch(&self) -> Option<f64> {
        self.identified.anchor_epoch
    }

    /// Attach one authority identifier.
    #[must_use]
    pub fn with_identifier(mut self, code_space: &str, code: &str) -> Self {
        self.identified
            .identifiers
            .push(Identifier::new(code_space, code));
        self
    }

    /// Anchor definition, the WKT `ANCHOR` node.
    #[must_use]
    pub fn with_anchor(mut self, anchor: impl Into<String>) -> Self {
        self.identified.anchor = Some(anchor.into());
        self
    }

    /// Realization method. It is not written in WKT.
    #[must_use]
    pub fn with_realization(mut self, method: RealizationMethod) -> Self {
        self.realization_method = Some(method);
        self
    }

    /// Realization method, when one was set.
    #[must_use]
    pub fn realization_method(&self) -> Option<RealizationMethod> {
        self.realization_method
    }

    /// Deformation model name of a dynamic vertical datum.
    #[must_use]
    pub fn deformation_model(&self) -> Option<&str> {
        self.deformation_model.as_deref()
    }
}

/// Engineering datum.
#[derive(Clone, Debug)]
pub struct EngineeringDatum {
    pub(crate) identified: Identified,
}

impl EngineeringDatum {
    /// Name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.identified.name
    }

    /// Authority identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[Identifier] {
        &self.identified.identifiers
    }

    /// Usages recorded for this datum.
    #[must_use]
    pub fn domains(&self) -> &[ObjectDomain] {
        &self.identified.domains
    }

    /// Publication date `YYYY-MM-DD`, when the database has one.
    #[must_use]
    pub fn publication_date(&self) -> Option<&str> {
        self.identified.publication_date.as_deref()
    }

    /// Anchor epoch in years.
    #[must_use]
    pub fn anchor_epoch(&self) -> Option<f64> {
        self.identified.anchor_epoch
    }
}

/// One datum, whichever kind the database row describes.
#[derive(Clone, Debug)]
#[expect(
    clippy::large_enum_variant,
    reason = "geodetic frames carry an ellipsoid; the other datum kinds do not"
)]
pub enum Datum {
    /// Geodetic reference frame.
    Geodetic(GeodeticReferenceFrame),
    /// Vertical reference frame.
    Vertical(VerticalReferenceFrame),
    /// Engineering datum.
    Engineering(EngineeringDatum),
}

impl Datum {
    /// Name of the datum.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Geodetic(datum) => datum.name(),
            Self::Vertical(datum) => datum.name(),
            Self::Engineering(datum) => datum.name(),
        }
    }

    /// Authority identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[Identifier] {
        match self {
            Self::Geodetic(datum) => datum.identifiers(),
            Self::Vertical(datum) => datum.identifiers(),
            Self::Engineering(datum) => datum.identifiers(),
        }
    }
}

/// Ensemble of realizations of one datum, with a stated accuracy in metres.
#[derive(Clone, Debug)]
pub struct DatumEnsemble {
    pub(crate) identified: Identified,
    pub(crate) members: Vec<Datum>,
    pub(crate) accuracy: f64,
}

impl DatumEnsemble {
    /// Build an ensemble.
    ///
    /// # Errors
    ///
    /// The member list is empty, or the members are not all the same datum kind.
    pub fn create(
        name: impl Into<String>,
        members: Vec<Datum>,
        accuracy: f64,
    ) -> Result<Self, FactoryError> {
        if members.is_empty() {
            return Err(FactoryError::Invalid(
                "a datum ensemble needs at least one member".to_owned(),
            ));
        }
        let kind = std::mem::discriminant(&members[0]);
        if members
            .iter()
            .any(|member| std::mem::discriminant(member) != kind)
        {
            return Err(FactoryError::Invalid(
                "datum ensemble members must share one datum type".to_owned(),
            ));
        }
        Ok(Self {
            identified: Identified::named(name),
            members,
            accuracy,
        })
    }

    /// Name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.identified.name
    }

    /// Authority identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[Identifier] {
        &self.identified.identifiers
    }

    /// Realizations, in database order.
    #[must_use]
    pub fn members(&self) -> &[Datum] {
        &self.members
    }

    /// Ensemble accuracy in metres.
    #[must_use]
    pub fn positional_accuracy(&self) -> f64 {
        self.accuracy
    }

    /// Usages recorded for this ensemble.
    #[must_use]
    pub fn domains(&self) -> &[ObjectDomain] {
        &self.identified.domains
    }

    /// Datum synthesized from the first member.
    ///
    /// A geodetic ensemble is renamed the way `DatumEnsemble::asDatum` renames it
    /// (`World Geodetic System 1984 ensemble` becomes `World Geodetic System 1984`).
    /// The identifier is the ensemble's. The frame epoch is not copied.
    #[must_use]
    pub fn as_datum(&self) -> Option<Datum> {
        let member = self.members.first()?;
        let mut identified = self.identified.clone();
        identified.anchor = None;
        identified.anchor_epoch = None;
        match member {
            Datum::Geodetic(frame) => {
                if let Some(name) = ensemble_name_as_datum(&identified.name) {
                    identified.name = name.to_owned();
                }
                Some(Datum::Geodetic(GeodeticReferenceFrame {
                    identified,
                    ellipsoid: frame.ellipsoid.clone(),
                    prime_meridian: frame.prime_meridian.clone(),
                    frame_epoch: None,
                    deformation_model: None,
                }))
            }
            Datum::Vertical(_) => Some(Datum::Vertical(VerticalReferenceFrame {
                identified,
                wkt1_type: "2005".to_owned(),
                frame_epoch: None,
                deformation_model: None,
                realization_method: None,
            })),
            Datum::Engineering(_) => None,
        }
    }
}

/// One axis of a coordinate system.
#[derive(Clone, Debug)]
pub struct Axis {
    pub(crate) name: String,
    pub(crate) abbreviation: String,
    pub(crate) direction: String,
    pub(crate) unit: UnitOfMeasure,
    pub(crate) meridian: Option<Meridian>,
    pub(crate) minimum: Option<f64>,
    pub(crate) maximum: Option<f64>,
    /// `exact` or `wraparound`, the WKT2 `RANGEMEANING` token.
    pub(crate) range_meaning: Option<String>,
}

/// Meridian of a polar axis, such as `North along 90°E`.
#[derive(Clone, Debug)]
pub struct Meridian {
    pub(crate) longitude: Measure,
}

impl Meridian {
    /// Meridian whose longitude is `degrees` east of Greenwich.
    #[must_use]
    pub fn create(degrees: f64) -> Self {
        Self {
            longitude: Measure::angle(degrees),
        }
    }

    /// Longitude of the meridian.
    #[must_use]
    pub fn longitude(&self) -> &Measure {
        &self.longitude
    }
}

impl Axis {
    /// Axis name as stored, for example `Geodetic latitude`.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Abbreviation, for example `Lat`.
    #[must_use]
    pub fn abbreviation(&self) -> &str {
        &self.abbreviation
    }

    /// Direction token, for example `north` or `geocentricX`.
    #[must_use]
    pub fn direction(&self) -> &str {
        &self.direction
    }

    /// Unit of this axis.
    #[must_use]
    pub fn unit(&self) -> &UnitOfMeasure {
        &self.unit
    }

    /// Meridian, when the direction is `North along` or `South along`.
    #[must_use]
    pub fn meridian(&self) -> Option<&Meridian> {
        self.meridian.as_ref()
    }

    /// `AXISMINVALUE`, when the axis declares one.
    #[must_use]
    pub fn minimum_value(&self) -> Option<f64> {
        self.minimum
    }

    /// `AXISMAXVALUE`, when the axis declares one.
    #[must_use]
    pub fn maximum_value(&self) -> Option<f64> {
        self.maximum
    }

    /// `RANGEMEANING` token, `exact` or `wraparound`.
    #[must_use]
    pub fn range_meaning(&self) -> Option<&str> {
        self.range_meaning.as_deref()
    }

    /// Axis with no meridian.
    #[must_use]
    pub fn create(
        name: impl Into<String>,
        abbreviation: impl Into<String>,
        direction: impl Into<String>,
        unit: UnitOfMeasure,
    ) -> Self {
        Self::new(name, abbreviation, direction, unit)
    }

    /// Attach the meridian used by a `North along` or `South along` axis.
    #[must_use]
    pub fn with_meridian(mut self, meridian: Meridian) -> Self {
        self.meridian = Some(meridian);
        self
    }

    pub(crate) fn new(
        name: impl Into<String>,
        abbreviation: impl Into<String>,
        direction: impl Into<String>,
        unit: UnitOfMeasure,
    ) -> Self {
        Self {
            name: name.into(),
            abbreviation: abbreviation.into(),
            direction: direction.into(),
            unit,
            meridian: None,
            minimum: None,
            maximum: None,
            range_meaning: None,
        }
    }
}

/// Ellipsoidal coordinate system.
#[derive(Clone, Debug)]
pub struct EllipsoidalCS {
    pub(crate) identified: Identified,
    pub(crate) axes: Vec<Axis>,
}

impl EllipsoidalCS {
    /// Latitude and longitude, in that order, in `unit`.
    #[must_use]
    pub fn create_latitude_longitude(unit: UnitOfMeasure) -> Self {
        Self {
            identified: Identified::default(),
            axes: vec![
                Axis::new("Latitude", "Lat", "north", unit.clone()),
                Axis::new("Longitude", "Lon", "east", unit),
            ],
        }
    }

    /// Axes, in coordinate order.
    #[must_use]
    pub fn axes(&self) -> &[Axis] {
        &self.axes
    }

    /// Authority identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[Identifier] {
        &self.identified.identifiers
    }

    /// Longitude then latitude, in `unit`.
    #[must_use]
    pub fn create_longitude_latitude(unit: UnitOfMeasure) -> Self {
        Self {
            identified: Identified::default(),
            axes: vec![
                Axis::new("Longitude", "Lon", "east", unit.clone()),
                Axis::new("Latitude", "Lat", "north", unit),
            ],
        }
    }

    /// Latitude, longitude, and ellipsoidal height.
    #[must_use]
    pub fn create_latitude_longitude_height(angular: UnitOfMeasure, linear: UnitOfMeasure) -> Self {
        Self {
            identified: Identified::default(),
            axes: vec![
                Axis::new("Latitude", "Lat", "north", angular.clone()),
                Axis::new("Longitude", "Lon", "east", angular),
                Axis::new("Ellipsoidal height", "h", "up", linear),
            ],
        }
    }
}

/// Cartesian coordinate system.
#[derive(Clone, Debug)]
pub struct CartesianCS {
    pub(crate) identified: Identified,
    pub(crate) axes: Vec<Axis>,
}

impl CartesianCS {
    /// Build a 2D or 3D cartesian system.
    ///
    /// # Errors
    ///
    /// The axis count is not 2 or 3, or the axis units differ by name.
    /// That is the check in `cs.CartesianCS`.
    pub fn create(axes: Vec<Axis>) -> Result<Self, FactoryError> {
        if axes.len() == 2 || axes.len() == 3 {
            let first = axes[0].unit.name();
            if axes.iter().any(|axis| axis.unit.name() != first) {
                return Err(FactoryError::Invalid(
                    "invalid coordinate system: cartesian axes use different units".to_owned(),
                ));
            }
        }
        Self::create_allowing_mixed_units(axes)
    }

    /// Geocentric X, Y, Z in `unit`.
    #[must_use]
    pub fn create_geocentric(unit: UnitOfMeasure) -> Self {
        Self {
            identified: Identified::default(),
            axes: vec![
                Axis::new("Geocentric X", "X", "geocentricX", unit.clone()),
                Axis::new("Geocentric Y", "Y", "geocentricY", unit.clone()),
                Axis::new("Geocentric Z", "Z", "geocentricZ", unit),
            ],
        }
    }

    /// Easting then northing, in `unit`.
    #[must_use]
    pub fn create_easting_northing(unit: UnitOfMeasure) -> Self {
        Self {
            identified: Identified::default(),
            axes: vec![
                Axis::new("Easting", "E", "east", unit.clone()),
                Axis::new("Northing", "N", "north", unit),
            ],
        }
    }

    /// Cartesian system whose axes may use different units.
    ///
    /// # Errors
    ///
    /// The axis count is not 2 or 3.
    pub fn create_allowing_mixed_units(axes: Vec<Axis>) -> Result<Self, FactoryError> {
        if axes.len() != 2 && axes.len() != 3 {
            return Err(FactoryError::Invalid(
                "a cartesian coordinate system has 2 or 3 axes".to_owned(),
            ));
        }
        Ok(Self {
            identified: Identified::default(),
            axes,
        })
    }

    /// Axes, in coordinate order.
    #[must_use]
    pub fn axes(&self) -> &[Axis] {
        &self.axes
    }

    /// Authority identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[Identifier] {
        &self.identified.identifiers
    }
}

/// Vertical coordinate system.
#[derive(Clone, Debug)]
pub struct VerticalCS {
    pub(crate) identified: Identified,
    pub(crate) axis: Axis,
}

impl VerticalCS {
    /// Gravity-related height, abbreviation `H`, direction `up`.
    #[must_use]
    pub fn create_gravity_related_height(unit: UnitOfMeasure) -> Self {
        Self {
            identified: Identified::default(),
            axis: Axis::new("Gravity-related height", "H", "up", unit),
        }
    }

    /// The single axis.
    #[must_use]
    pub fn axis(&self) -> &Axis {
        &self.axis
    }

    /// Authority identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[Identifier] {
        &self.identified.identifiers
    }
}

/// Spherical coordinate system.
#[derive(Clone, Debug)]
pub struct SphericalCS {
    pub(crate) identified: Identified,
    pub(crate) axes: Vec<Axis>,
}

impl SphericalCS {
    /// Axes, in coordinate order.
    #[must_use]
    pub fn axes(&self) -> &[Axis] {
        &self.axes
    }
}

/// Ordinal coordinate system.
#[derive(Clone, Debug)]
pub struct OrdinalCS {
    pub(crate) identified: Identified,
    pub(crate) axes: Vec<Axis>,
}

impl OrdinalCS {
    /// Axes, in coordinate order.
    #[must_use]
    pub fn axes(&self) -> &[Axis] {
        &self.axes
    }
}

/// Coordinate system, one of the kinds `proj.db` stores on a CRS.
#[derive(Clone, Debug)]
#[expect(
    clippy::large_enum_variant,
    reason = "a vertical system stores its axis inline; the others keep axes in a Vec"
)]
pub enum CoordinateSystem {
    /// Latitude and longitude, optionally with ellipsoidal height.
    Ellipsoidal(EllipsoidalCS),
    /// Easting/northing or geocentric X/Y/Z.
    Cartesian(CartesianCS),
    /// Gravity-related height.
    Vertical(VerticalCS),
    /// Spherical geodetic system.
    Spherical(SphericalCS),
    /// Ordinal axes, which may have no unit.
    Ordinal(OrdinalCS),
    /// Affine axes. WKT writes `CS[affine,N]`.
    Affine(CartesianCS),
}

impl CoordinateSystem {
    /// Axes, in coordinate order.
    #[must_use]
    pub fn axes(&self) -> Vec<&Axis> {
        match self {
            Self::Ellipsoidal(cs) => cs.axes.iter().collect(),
            Self::Cartesian(cs) => cs.axes.iter().collect(),
            Self::Vertical(cs) => vec![&cs.axis],
            Self::Spherical(cs) => cs.axes.iter().collect(),
            Self::Ordinal(cs) => cs.axes.iter().collect(),
            Self::Affine(cs) => cs.axes.iter().collect(),
        }
    }

    pub(crate) fn identified(&self) -> &Identified {
        match self {
            Self::Ellipsoidal(cs) => &cs.identified,
            Self::Cartesian(cs) => &cs.identified,
            Self::Vertical(cs) => &cs.identified,
            Self::Spherical(cs) => &cs.identified,
            Self::Ordinal(cs) => &cs.identified,
            Self::Affine(cs) => &cs.identified,
        }
    }

    /// Authority identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[Identifier] {
        &self.identified().identifiers
    }

    pub(crate) fn wkt2_type(&self) -> &'static str {
        match self {
            Self::Ellipsoidal(_) => "ellipsoidal",
            Self::Cartesian(_) => "Cartesian",
            Self::Vertical(_) => "vertical",
            Self::Spherical(_) => "spherical",
            Self::Ordinal(_) => "ordinal",
            Self::Affine(_) => "affine",
        }
    }
}

/// Definition of one operation parameter, without a value.
#[derive(Clone, Debug)]
pub struct OperationParameter {
    pub(crate) identified: Identified,
}

impl OperationParameter {
    /// Parameter named `name`.
    #[must_use]
    pub fn create(name: impl Into<String>) -> Self {
        Self {
            identified: Identified::named(name),
        }
    }

    /// Parameter name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.identified.name
    }
}

/// Operation method name, authority code, and parameter definitions.
#[derive(Clone, Debug)]
pub struct OperationMethod {
    pub(crate) name: String,
    pub(crate) code_space: String,
    pub(crate) code: String,
    pub(crate) parameters: Vec<OperationParameter>,
}

impl OperationMethod {
    /// Method name, for example `Transverse Mercator`.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Authority code space.
    #[must_use]
    pub fn code_space(&self) -> &str {
        &self.code_space
    }

    /// Authority code.
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    /// The one identifier, when a code is set.
    #[must_use]
    pub fn identifiers(&self) -> Vec<Identifier> {
        if self.code.is_empty() {
            Vec::new()
        } else {
            vec![Identifier::new(self.code_space.clone(), self.code.clone())]
        }
    }

    /// Method with `name` and parameter definitions. The name may be empty.
    #[must_use]
    pub fn create(name: impl Into<String>, parameters: Vec<OperationParameter>) -> Self {
        Self {
            name: name.into(),
            code_space: String::new(),
            code: String::new(),
            parameters,
        }
    }

    /// Parameter definitions, in declaration order.
    #[must_use]
    pub fn parameters(&self) -> &[OperationParameter] {
        &self.parameters
    }

    pub(crate) fn named_code(name: String, code_space: String, code: String) -> Self {
        Self {
            name,
            code_space,
            code,
            parameters: Vec::new(),
        }
    }
}

/// Parameter value stored on a conversion or transformation.
#[derive(Clone, Debug)]
pub enum ParameterValue {
    /// Numeric measure.
    Measure(Measure),
    /// Text value.
    String(String),
    /// Integer value, including an EPSG code parameter.
    Integer(i64),
    /// Grid file name.
    Filename(String),
    /// Boolean flag.
    Boolean(bool),
}

impl ParameterValue {
    /// Text value.
    #[must_use]
    pub fn string(value: impl Into<String>) -> Self {
        Self::String(value.into())
    }

    /// File name.
    #[must_use]
    pub fn filename(value: impl Into<String>) -> Self {
        Self::Filename(value.into())
    }

    /// Integer value.
    #[must_use]
    pub fn integer(value: i64) -> Self {
        Self::Integer(value)
    }

    /// Boolean true.
    #[must_use]
    pub fn boolean_true() -> Self {
        Self::Boolean(true)
    }

    /// Boolean false.
    #[must_use]
    pub fn boolean_false() -> Self {
        Self::Boolean(false)
    }

    /// Angle in degrees.
    #[must_use]
    pub fn angle(value: f64) -> Self {
        Self::Measure(Measure::angle(value))
    }

    /// Length in metres.
    #[must_use]
    pub fn length(value: f64) -> Self {
        Self::Measure(Measure::length(value))
    }

    /// Scale factor.
    #[must_use]
    pub fn scale(value: f64) -> Self {
        Self::Measure(Measure::scale(value))
    }
}

/// A parameter definition paired with a value.
#[derive(Clone, Debug)]
pub struct OperationParameterValue {
    pub(crate) parameter: OperationParameter,
    pub(crate) value: ParameterValue,
}

impl OperationParameterValue {
    /// Pair `parameter` with `value`.
    #[must_use]
    pub fn create(parameter: OperationParameter, value: ParameterValue) -> Self {
        Self { parameter, value }
    }

    /// Parameter definition.
    #[must_use]
    pub fn parameter(&self) -> &OperationParameter {
        &self.parameter
    }

    /// Parameter value.
    #[must_use]
    pub fn value(&self) -> &ParameterValue {
        &self.value
    }
}

/// Named parameter of an operation.
#[derive(Clone, Debug)]
pub struct Parameter {
    pub(crate) name: String,
    pub(crate) code_space: String,
    pub(crate) code: String,
    pub(crate) value: ParameterValue,
}

impl Parameter {
    /// Parameter name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Authority code space.
    #[must_use]
    pub fn code_space(&self) -> &str {
        &self.code_space
    }

    /// Authority code.
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Parameter value.
    #[must_use]
    pub fn value(&self) -> &ParameterValue {
        &self.value
    }
}

/// Map projection or other conversion.
#[derive(Clone, Debug)]
pub struct Conversion {
    pub(crate) identified: Identified,
    pub(crate) method: OperationMethod,
    pub(crate) parameters: Vec<Parameter>,
    pub(crate) version: Option<String>,
}

impl Conversion {
    /// Name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.identified.name
    }

    /// Authority identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[Identifier] {
        &self.identified.identifiers
    }

    /// Method.
    #[must_use]
    pub fn method(&self) -> &OperationMethod {
        &self.method
    }

    /// Parameters, in database order.
    #[must_use]
    pub fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    /// Operation version, when the database has one.
    #[must_use]
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }
}

/// Coordinate transformation, including a grid or point-motion operation.
#[derive(Clone, Debug)]
pub struct Transformation {
    pub(crate) identified: Identified,
    pub(crate) method: OperationMethod,
    pub(crate) parameters: Vec<Parameter>,
    pub(crate) source: Option<Box<Crs>>,
    pub(crate) target: Option<Box<Crs>>,
    pub(crate) interpolation: Option<Box<Crs>>,
    pub(crate) accuracy: Option<f64>,
    pub(crate) version: Option<String>,
    pub(crate) point_motion: bool,
    pub(crate) inverted: bool,
}

impl Transformation {
    /// Name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.identified.name
    }

    /// Authority identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[Identifier] {
        &self.identified.identifiers
    }

    /// Method.
    #[must_use]
    pub fn method(&self) -> &OperationMethod {
        &self.method
    }

    /// Parameters, in the order PROJ emits them.
    #[must_use]
    pub fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    /// Source CRS, when the operation has one.
    #[must_use]
    pub fn source_crs(&self) -> Option<&Crs> {
        self.source.as_deref()
    }

    /// Target CRS, when the operation has one.
    #[must_use]
    pub fn target_crs(&self) -> Option<&Crs> {
        self.target.as_deref()
    }

    /// Stated accuracy in metres, when it is not the EPSG unknown value `999`.
    #[must_use]
    pub fn accuracy(&self) -> Option<f64> {
        self.accuracy
    }

    /// Interpolation CRS, when the operation names one.
    #[must_use]
    pub fn interpolation_crs(&self) -> Option<&Crs> {
        self.interpolation.as_deref()
    }
}

/// One step of a concatenated operation.
#[derive(Clone, Debug)]
pub struct ConcatenatedStep {
    pub(crate) operation: CoordinateOperation,
    pub(crate) reverse: bool,
}

impl ConcatenatedStep {
    /// Step operation.
    #[must_use]
    pub fn operation(&self) -> &CoordinateOperation {
        &self.operation
    }

    /// True when `concatenated_operation_step.step_direction` is `reverse`.
    #[must_use]
    pub fn is_reverse(&self) -> bool {
        self.reverse
    }
}

/// Concatenated coordinate operation.
#[derive(Clone, Debug)]
pub struct ConcatenatedOperation {
    pub(crate) identified: Identified,
    pub(crate) steps: Vec<ConcatenatedStep>,
    pub(crate) source: Option<Box<Crs>>,
    pub(crate) target: Option<Box<Crs>>,
    pub(crate) accuracy: Option<f64>,
    pub(crate) version: Option<String>,
}

impl ConcatenatedOperation {
    /// Name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.identified.name
    }

    /// Steps, in `step_number` order. The NKG chains start at 2; that number is kept.
    #[must_use]
    pub fn steps(&self) -> &[ConcatenatedStep] {
        &self.steps
    }

    /// Source CRS stored on the concatenated operation.
    #[must_use]
    pub fn source_crs(&self) -> Option<&Crs> {
        self.source.as_deref()
    }

    /// Target CRS stored on the concatenated operation.
    #[must_use]
    pub fn target_crs(&self) -> Option<&Crs> {
        self.target.as_deref()
    }

    /// Stated accuracy in metres.
    #[must_use]
    pub fn accuracy(&self) -> Option<f64> {
        self.accuracy
    }

    /// Operation version.
    #[must_use]
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }
}

/// Coordinate operation returned by the factories.
#[derive(Clone, Debug)]
pub enum CoordinateOperation {
    /// Conversion, including a map projection.
    Conversion(Conversion),
    /// Transformation, grid, Helmert, or point motion.
    Transformation(Transformation),
    /// Concatenated operation.
    Concatenated(ConcatenatedOperation),
}

impl CoordinateOperation {
    /// Name.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Conversion(operation) => operation.name(),
            Self::Transformation(operation) => operation.name(),
            Self::Concatenated(operation) => operation.name(),
        }
    }

    /// Authority identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[Identifier] {
        match self {
            Self::Conversion(operation) => operation.identifiers(),
            Self::Transformation(operation) => operation.identifiers(),
            Self::Concatenated(operation) => &operation.identified.identifiers,
        }
    }

    /// Stated accuracy in metres, when the database has one.
    #[must_use]
    pub fn accuracy(&self) -> Option<f64> {
        match self {
            Self::Conversion(_) => None,
            Self::Transformation(operation) => operation.accuracy(),
            Self::Concatenated(operation) => operation.accuracy(),
        }
    }

    /// Scope and area of use.
    #[must_use]
    pub fn domains(&self) -> &[ObjectDomain] {
        match self {
            Self::Conversion(operation) => &operation.identified.domains,
            Self::Transformation(operation) => &operation.identified.domains,
            Self::Concatenated(operation) => &operation.identified.domains,
        }
    }
}

/// Geographic CRS: ellipsoidal CS plus a geodetic datum or ensemble.
#[derive(Clone, Debug)]
pub struct GeographicCRS {
    pub(crate) identified: Identified,
    pub(crate) datum: Option<GeodeticReferenceFrame>,
    pub(crate) ensemble: Option<DatumEnsemble>,
    pub(crate) coordinate_system: EllipsoidalCS,
    /// WKT1 `GEOGCS` without `AXIS` nodes. Identification then ignores axis order.
    pub(crate) implicit_cs: bool,
}

impl GeographicCRS {
    /// Build a geographic CRS.
    ///
    /// # Errors
    ///
    /// Both the datum and the ensemble are missing, both are set, or an
    /// ensemble member is not a geodetic datum. Those are the failures in
    /// `GeographicCRS_ensemble_exception_in_create`.
    pub fn create(
        name: impl Into<String>,
        datum: Option<GeodeticReferenceFrame>,
        ensemble: Option<DatumEnsemble>,
        coordinate_system: EllipsoidalCS,
    ) -> Result<Self, FactoryError> {
        match (&datum, &ensemble) {
            (None, None) => {
                return Err(FactoryError::Invalid(
                    "a geographic CRS needs a datum or a datum ensemble".to_owned(),
                ));
            }
            (Some(_), Some(_)) => {
                return Err(FactoryError::Invalid(
                    "a geographic CRS cannot carry both a datum and a datum ensemble".to_owned(),
                ));
            }
            (None, Some(ensemble)) => {
                if ensemble
                    .members
                    .iter()
                    .any(|member| !matches!(member, Datum::Geodetic(_)))
                {
                    return Err(FactoryError::Invalid(
                        "a geographic CRS ensemble must contain geodetic datums".to_owned(),
                    ));
                }
            }
            (Some(_), None) => {}
        }
        Ok(Self {
            identified: Identified::named(name),
            datum,
            ensemble,
            coordinate_system,
            implicit_cs: false,
        })
    }

    /// Name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.identified.name
    }

    /// Authority identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[Identifier] {
        &self.identified.identifiers
    }

    /// Geodetic datum, when the CRS is not an ensemble.
    #[must_use]
    pub fn datum(&self) -> Option<&GeodeticReferenceFrame> {
        self.datum.as_ref()
    }

    /// Datum ensemble, when the CRS is defined by one.
    #[must_use]
    pub fn datum_ensemble(&self) -> Option<&DatumEnsemble> {
        self.ensemble.as_ref()
    }

    /// Ellipsoidal coordinate system.
    #[must_use]
    pub fn coordinate_system(&self) -> &EllipsoidalCS {
        &self.coordinate_system
    }

    /// Add an ellipsoidal-height axis in metres.
    ///
    /// A CRS that is already 3D is returned unchanged. This is
    /// `GeographicCRS::promoteTo3D` when the database has no 3D CRS of the same name.
    #[must_use]
    pub fn promote_to_3d(&self) -> Self {
        if self.coordinate_system.axes.len() != 2 {
            return self.clone();
        }
        let mut crs = self.clone();
        crs.coordinate_system.axes.push(Axis::new(
            "Ellipsoidal height",
            "h",
            "up",
            UnitOfMeasure::metre(),
        ));
        crs
    }

    /// Ellipsoid of the datum, or of the first ensemble member.
    #[must_use]
    pub fn ellipsoid(&self) -> Option<&Ellipsoid> {
        if let Some(datum) = &self.datum {
            return Some(datum.ellipsoid());
        }
        self.ensemble.as_ref().and_then(|ensemble| {
            ensemble.members().iter().find_map(|member| match member {
                Datum::Geodetic(frame) => Some(frame.ellipsoid()),
                _ => None,
            })
        })
    }

    /// Usages recorded for this CRS.
    #[must_use]
    pub fn domains(&self) -> &[ObjectDomain] {
        &self.identified.domains
    }

    /// Attach an authority identifier. Used by the datum-ensemble WKT test.
    #[must_use]
    pub fn with_identifier(mut self, code_space: &str, code: &str) -> Self {
        self.identified
            .identifiers
            .push(Identifier::new(code_space, code));
        self
    }

    /// WGS 84 geographic 2D, EPSG:4326, on datum EPSG:6326 (not the ensemble).
    #[must_use]
    pub fn epsg_4326() -> Self {
        Self::from_datum(
            "WGS 84",
            "4326",
            GeodeticReferenceFrame::epsg_6326(),
            EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::degree()),
        )
    }

    /// WGS 84 geographic 3D, EPSG:4979.
    #[must_use]
    pub fn epsg_4979() -> Self {
        Self::from_datum(
            "WGS 84",
            "4979",
            GeodeticReferenceFrame::epsg_6326(),
            EllipsoidalCS::create_latitude_longitude_height(
                UnitOfMeasure::degree(),
                UnitOfMeasure::metre(),
            ),
        )
    }

    /// NAD27, EPSG:4267.
    #[must_use]
    pub fn epsg_4267() -> Self {
        Self::from_datum(
            "NAD27",
            "4267",
            GeodeticReferenceFrame::epsg_6267(),
            EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::degree()),
        )
    }

    /// NAD83, EPSG:4269.
    #[must_use]
    pub fn epsg_4269() -> Self {
        Self::from_datum(
            "NAD83",
            "4269",
            GeodeticReferenceFrame::epsg_6269(),
            EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::degree()),
        )
    }

    /// NTF (Paris), EPSG:4807.
    #[must_use]
    pub fn epsg_4807() -> Self {
        let datum = GeodeticReferenceFrame {
            identified: Identified::named("Nouvelle Triangulation Francaise (Paris)")
                .with_id("EPSG", "6807"),
            ellipsoid: Ellipsoid::clarke_1880_ign(),
            prime_meridian: PrimeMeridian::paris(),
            frame_epoch: None,
            deformation_model: None,
        };
        Self::from_datum(
            "NTF (Paris)",
            "4807",
            datum,
            EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::grad()),
        )
    }

    /// WGS 84 longitude/latitude, OGC:CRS84.
    #[must_use]
    pub fn ogc_crs84() -> Self {
        Self {
            identified: Identified::named("WGS 84 (CRS84)").with_id("OGC", "CRS84"),
            datum: Some(GeodeticReferenceFrame::epsg_6326()),
            ensemble: None,
            coordinate_system: EllipsoidalCS::create_longitude_latitude(UnitOfMeasure::degree()),
            implicit_cs: false,
        }
    }

    fn from_datum(
        name: &str,
        code: &str,
        datum: GeodeticReferenceFrame,
        coordinate_system: EllipsoidalCS,
    ) -> Self {
        Self {
            identified: Identified::named(name).with_id("EPSG", code),
            datum: Some(datum),
            ensemble: None,
            coordinate_system,
            implicit_cs: false,
        }
    }
}

/// Geocentric cartesian CRS.
#[derive(Clone, Debug)]
pub struct GeocentricCRS {
    pub(crate) identified: Identified,
    pub(crate) datum: Option<GeodeticReferenceFrame>,
    pub(crate) ensemble: Option<DatumEnsemble>,
    pub(crate) coordinate_system: CartesianCS,
}

impl GeocentricCRS {
    /// Name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.identified.name
    }

    /// Authority identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[Identifier] {
        &self.identified.identifiers
    }

    /// Cartesian coordinate system.
    #[must_use]
    pub fn coordinate_system(&self) -> &CartesianCS {
        &self.coordinate_system
    }

    /// Geodetic datum, when the CRS is not an ensemble.
    #[must_use]
    pub fn datum(&self) -> Option<&GeodeticReferenceFrame> {
        self.datum.as_ref()
    }

    /// Usages recorded for this CRS.
    #[must_use]
    pub fn domains(&self) -> &[ObjectDomain] {
        &self.identified.domains
    }
}

/// Spherical geodetic CRS (`geodetic_crs.type = other`).
#[derive(Clone, Debug)]
pub struct SphericalGeodeticCRS {
    pub(crate) identified: Identified,
    pub(crate) datum: Option<GeodeticReferenceFrame>,
    pub(crate) ensemble: Option<DatumEnsemble>,
    pub(crate) coordinate_system: SphericalCS,
}

/// Geodetic CRS that is geographic, geocentric, or spherical.
#[derive(Clone, Debug)]
pub enum GeodeticCRS {
    /// Geographic 2D or 3D.
    Geographic(GeographicCRS),
    /// Geocentric cartesian.
    Geocentric(GeocentricCRS),
    /// Spherical, stored as `type = other` in `geodetic_crs`.
    Spherical(SphericalGeodeticCRS),
}

impl GeodeticCRS {
    /// Name.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Geographic(crs) => crs.name(),
            Self::Geocentric(crs) => crs.name(),
            Self::Spherical(crs) => &crs.identified.name,
        }
    }

    /// Authority identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[Identifier] {
        match self {
            Self::Geographic(crs) => crs.identifiers(),
            Self::Geocentric(crs) => crs.identifiers(),
            Self::Spherical(crs) => &crs.identified.identifiers,
        }
    }

    /// Geodetic datum, when present.
    #[must_use]
    pub fn datum(&self) -> Option<&GeodeticReferenceFrame> {
        match self {
            Self::Geographic(crs) => crs.datum(),
            Self::Geocentric(crs) => crs.datum(),
            Self::Spherical(crs) => crs.datum.as_ref(),
        }
    }

    /// Datum ensemble, when present.
    #[must_use]
    pub fn datum_ensemble(&self) -> Option<&DatumEnsemble> {
        match self {
            Self::Geographic(crs) => crs.datum_ensemble(),
            Self::Geocentric(crs) => crs.ensemble.as_ref(),
            Self::Spherical(crs) => crs.ensemble.as_ref(),
        }
    }

    /// Ellipsoid of the datum, or of the first ensemble member.
    #[must_use]
    pub fn ellipsoid(&self) -> Option<&Ellipsoid> {
        match self {
            Self::Geographic(crs) => crs.ellipsoid(),
            Self::Geocentric(crs) => {
                crs.datum()
                    .map(GeodeticReferenceFrame::ellipsoid)
                    .or_else(|| {
                        crs.ensemble.as_ref().and_then(|ensemble| {
                            ensemble.members().iter().find_map(|member| match member {
                                Datum::Geodetic(frame) => Some(frame.ellipsoid()),
                                _ => None,
                            })
                        })
                    })
            }
            Self::Spherical(crs) => crs.datum.as_ref().map(GeodeticReferenceFrame::ellipsoid),
        }
    }

    /// Geographic, geocentric, or spherical CRS.
    ///
    /// Exactly one of `datum` and `ensemble` is set. A cartesian system is
    /// geocentric. An ellipsoidal system is geographic.
    ///
    /// # Errors
    ///
    /// Both anchors are missing, both are set, the ensemble is not geodetic,
    /// or the coordinate system is not geographic, geocentric, or spherical.
    pub fn create(
        name: impl Into<String>,
        datum: Option<GeodeticReferenceFrame>,
        ensemble: Option<DatumEnsemble>,
        cs: CoordinateSystem,
    ) -> Result<Self, FactoryError> {
        match (&datum, &ensemble) {
            (None, None) => {
                return Err(FactoryError::Invalid(
                    "One of Datum or DatumEnsemble should be defined".to_owned(),
                ));
            }
            (Some(_), Some(_)) => {
                return Err(FactoryError::Invalid(
                    "Datum and DatumEnsemble should not be defined".to_owned(),
                ));
            }
            _ => {}
        }
        if let Some(ensemble) = &ensemble {
            if !matches!(ensemble.members().first(), Some(Datum::Geodetic(_))) {
                return Err(FactoryError::Invalid(
                    "Ensemble should contain GeodeticReferenceFrame".to_owned(),
                ));
            }
        }
        let identified = Identified::named(name);
        let crs = match cs {
            CoordinateSystem::Ellipsoidal(coordinate_system) => Self::Geographic(GeographicCRS {
                identified,
                datum,
                ensemble,
                coordinate_system,
                implicit_cs: false,
            }),
            CoordinateSystem::Cartesian(coordinate_system) => Self::Geocentric(GeocentricCRS {
                identified,
                datum,
                ensemble,
                coordinate_system,
            }),
            CoordinateSystem::Spherical(coordinate_system) => {
                Self::Spherical(SphericalGeodeticCRS {
                    identified,
                    datum,
                    ensemble,
                    coordinate_system,
                })
            }
            _ => {
                return Err(FactoryError::Invalid(
                    "a geodetic CRS uses an ellipsoidal, cartesian, or spherical coordinate system"
                        .to_owned(),
                ));
            }
        };
        Ok(crs)
    }

    /// Attach one authority identifier.
    #[must_use]
    pub fn with_identifier(mut self, code_space: &str, code: &str) -> Self {
        let identified = match &mut self {
            Self::Geographic(crs) => &mut crs.identified,
            Self::Geocentric(crs) => &mut crs.identified,
            Self::Spherical(crs) => &mut crs.identified,
        };
        identified
            .identifiers
            .push(Identifier::new(code_space, code));
        self
    }

    /// WGS 84 geocentric, EPSG:4978.
    #[must_use]
    pub fn epsg_4978() -> Self {
        Self::Geocentric(GeocentricCRS {
            identified: Identified::named("WGS 84").with_id("EPSG", "4978"),
            datum: Some(GeodeticReferenceFrame::epsg_6326()),
            ensemble: None,
            coordinate_system: CartesianCS::create_geocentric(UnitOfMeasure::metre()),
        })
    }

    /// Coordinate system of this geodetic CRS.
    #[must_use]
    pub fn coordinate_system(&self) -> CoordinateSystem {
        match self {
            Self::Geographic(crs) => CoordinateSystem::Ellipsoidal(crs.coordinate_system.clone()),
            Self::Geocentric(crs) => CoordinateSystem::Cartesian(crs.coordinate_system.clone()),
            Self::Spherical(crs) => CoordinateSystem::Spherical(crs.coordinate_system.clone()),
        }
    }

    /// Usages recorded for this CRS.
    #[must_use]
    pub fn domains(&self) -> &[ObjectDomain] {
        match self {
            Self::Geographic(crs) => crs.domains(),
            Self::Geocentric(crs) => crs.domains(),
            Self::Spherical(crs) => &crs.identified.domains,
        }
    }

    pub(crate) fn prime_meridian(&self) -> Option<&PrimeMeridian> {
        self.datum()
            .map(GeodeticReferenceFrame::prime_meridian)
            .or_else(|| {
                self.datum_ensemble().and_then(|ensemble| {
                    ensemble.members().iter().find_map(|member| match member {
                        Datum::Geodetic(frame) => Some(frame.prime_meridian()),
                        _ => None,
                    })
                })
            })
    }
}

/// One call of [`ProjectedCRS::deriving_operation`].
///
/// The conversion is a copy. `source` is the base CRS and `target` is the projected CRS.
#[derive(Debug)]
pub struct DerivedConversion<'a> {
    conversion: Conversion,
    source: &'a GeodeticCRS,
    target: &'a ProjectedCRS,
}

impl<'a> DerivedConversion<'a> {
    /// Base CRS of the projected CRS. PROJ's `sourceCRS()` on a deriving conversion.
    #[must_use]
    pub fn source_crs(&self) -> &'a GeodeticCRS {
        self.source
    }

    /// Projected CRS that owns this conversion.
    #[must_use]
    pub fn target_crs(&self) -> &'a ProjectedCRS {
        self.target
    }

    /// The copied conversion. A later call is a different object.
    #[must_use]
    pub fn conversion(&self) -> &Conversion {
        &self.conversion
    }
}

/// Projected CRS.
#[derive(Clone, Debug)]
pub struct ProjectedCRS {
    pub(crate) identified: Identified,
    pub(crate) base: GeodeticCRS,
    pub(crate) conversion: Conversion,
    pub(crate) coordinate_system: CartesianCS,
    /// WKT1 `PROJCS` or a PROJ string with no `AXIS` nodes.
    ///
    /// Identification then treats the projected axes as matching when the unit
    /// matches, even if the official axis order differs.
    pub(crate) implicit_cs: bool,
}

impl ProjectedCRS {
    /// Build a projected CRS from a base, a conversion, and a cartesian CS.
    #[must_use]
    pub fn create(
        name: impl Into<String>,
        base: GeodeticCRS,
        conversion: Conversion,
        coordinate_system: CartesianCS,
    ) -> Self {
        Self {
            identified: Identified::named(name),
            base,
            conversion,
            coordinate_system,
            implicit_cs: false,
        }
    }

    /// Name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.identified.name
    }

    /// Authority identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[Identifier] {
        &self.identified.identifiers
    }

    /// Base geodetic CRS.
    #[must_use]
    pub fn base_crs(&self) -> &GeodeticCRS {
        &self.base
    }

    /// Deriving conversion.
    #[must_use]
    pub fn deriving_conversion(&self) -> &Conversion {
        &self.conversion
    }

    /// Attach one authority identifier.
    #[must_use]
    pub fn with_identifier(mut self, code_space: &str, code: &str) -> Self {
        self.identified
            .identifiers
            .push(Identifier::new(code_space, code));
        self
    }

    /// Copy of the deriving conversion, with this CRS as its target.
    ///
    /// Each call returns a new conversion. The target of that copy is this CRS,
    /// which is what `derivingConversion()` does after `ProjectedCRS::create`.
    #[must_use]
    pub fn deriving_operation(&self) -> DerivedConversion<'_> {
        DerivedConversion {
            conversion: self.conversion.clone(),
            source: &self.base,
            target: self,
        }
    }

    /// Projected coordinate system.
    #[must_use]
    pub fn coordinate_system(&self) -> &CartesianCS {
        &self.coordinate_system
    }

    /// Usages recorded for this CRS.
    #[must_use]
    pub fn domains(&self) -> &[ObjectDomain] {
        &self.identified.domains
    }
}

/// Vertical CRS.
#[derive(Clone, Debug)]
pub struct VerticalCRS {
    pub(crate) identified: Identified,
    pub(crate) datum: Option<VerticalReferenceFrame>,
    pub(crate) ensemble: Option<DatumEnsemble>,
    pub(crate) coordinate_system: VerticalCS,
    pub(crate) geoid_models: Vec<GeoidModel>,
}

/// Geoid model named by a vertical CRS.
#[derive(Clone, Debug)]
pub struct GeoidModel {
    pub(crate) name: String,
    pub(crate) identifiers: Vec<Identifier>,
}

impl GeoidModel {
    /// Model name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Authority identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[Identifier] {
        &self.identifiers
    }
}

impl VerticalCRS {
    /// Name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.identified.name
    }

    /// Authority identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[Identifier] {
        &self.identified.identifiers
    }

    /// Vertical datum, when the CRS is not an ensemble.
    #[must_use]
    pub fn datum(&self) -> Option<&VerticalReferenceFrame> {
        self.datum.as_ref()
    }

    /// Datum ensemble, when the CRS is defined by one.
    #[must_use]
    pub fn datum_ensemble(&self) -> Option<&DatumEnsemble> {
        self.ensemble.as_ref()
    }

    /// Vertical coordinate system.
    #[must_use]
    pub fn coordinate_system(&self) -> &VerticalCS {
        &self.coordinate_system
    }

    /// Geoid models, in WKT order.
    #[must_use]
    pub fn geoid_models(&self) -> &[GeoidModel] {
        &self.geoid_models
    }

    /// Usages recorded for this CRS.
    #[must_use]
    pub fn domains(&self) -> &[ObjectDomain] {
        &self.identified.domains
    }

    /// Vertical CRS bound to one vertical datum.
    #[must_use]
    pub fn create(
        name: impl Into<String>,
        datum: VerticalReferenceFrame,
        coordinate_system: VerticalCS,
    ) -> Self {
        Self {
            identified: Identified::named(name),
            datum: Some(datum),
            ensemble: None,
            coordinate_system,
            geoid_models: Vec::new(),
        }
    }

    /// Vertical CRS bound to a vertical datum ensemble.
    ///
    /// # Errors
    ///
    /// A member is not a vertical datum.
    pub fn from_ensemble(
        name: impl Into<String>,
        ensemble: DatumEnsemble,
        coordinate_system: VerticalCS,
    ) -> Result<Self, FactoryError> {
        Self::create_with(name, None, Some(ensemble), coordinate_system)
    }

    /// One of `datum` or `ensemble` is set, and an ensemble is vertical.
    ///
    /// # Errors
    ///
    /// Both anchors are missing, both are set, or the ensemble is not vertical.
    pub fn create_with(
        name: impl Into<String>,
        datum: Option<VerticalReferenceFrame>,
        ensemble: Option<DatumEnsemble>,
        coordinate_system: VerticalCS,
    ) -> Result<Self, FactoryError> {
        match (&datum, &ensemble) {
            (None, None) => {
                return Err(FactoryError::Invalid(
                    "One of Datum or DatumEnsemble should be defined".to_owned(),
                ));
            }
            (Some(_), Some(_)) => {
                return Err(FactoryError::Invalid(
                    "Datum and DatumEnsemble should not be defined".to_owned(),
                ));
            }
            _ => {}
        }
        if let Some(ensemble) = &ensemble {
            let vertical = ensemble
                .members()
                .first()
                .is_some_and(|member| matches!(member, Datum::Vertical(_)));
            if !vertical {
                return Err(FactoryError::Invalid(
                    "Ensemble should contain VerticalReferenceFrame".to_owned(),
                ));
            }
        }
        Ok(Self {
            identified: Identified::named(name),
            datum,
            ensemble,
            coordinate_system,
            geoid_models: Vec::new(),
        })
    }

    /// Attach one authority identifier.
    #[must_use]
    pub fn with_identifier(mut self, code_space: &str, code: &str) -> Self {
        self.identified
            .identifiers
            .push(Identifier::new(code_space, code));
        self
    }
}

/// Engineering CRS.
#[derive(Clone, Debug)]
pub struct EngineeringCRS {
    pub(crate) identified: Identified,
    pub(crate) datum: EngineeringDatum,
    pub(crate) coordinate_system: CoordinateSystem,
}

impl EngineeringCRS {
    /// Name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.identified.name
    }

    /// Authority identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[Identifier] {
        &self.identified.identifiers
    }

    /// Engineering datum.
    #[must_use]
    pub fn datum(&self) -> &EngineeringDatum {
        &self.datum
    }

    /// Coordinate system.
    #[must_use]
    pub fn coordinate_system(&self) -> &CoordinateSystem {
        &self.coordinate_system
    }

    /// Usages recorded for this CRS.
    #[must_use]
    pub fn domains(&self) -> &[ObjectDomain] {
        &self.identified.domains
    }
}

/// Derived projected CRS.
#[derive(Clone, Debug)]
pub struct DerivedProjectedCRS {
    pub(crate) identified: Identified,
    pub(crate) base: ProjectedCRS,
    pub(crate) conversion: Conversion,
    pub(crate) coordinate_system: CoordinateSystem,
}

impl DerivedProjectedCRS {
    /// Derived projected CRS over `base`.
    #[must_use]
    pub fn create(
        name: impl Into<String>,
        base: ProjectedCRS,
        conversion: Conversion,
        coordinate_system: CoordinateSystem,
    ) -> Self {
        Self {
            identified: Identified::named(name),
            base,
            conversion,
            coordinate_system,
        }
    }

    /// Name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.identified.name
    }

    /// Base projected CRS.
    #[must_use]
    pub fn base_crs(&self) -> &ProjectedCRS {
        &self.base
    }

    /// Deriving conversion.
    #[must_use]
    pub fn deriving_conversion(&self) -> &Conversion {
        &self.conversion
    }

    /// Coordinate system of the derived CRS.
    #[must_use]
    pub fn coordinate_system(&self) -> &CoordinateSystem {
        &self.coordinate_system
    }
}

/// Compound CRS, normally horizontal plus vertical.
#[derive(Clone, Debug)]
pub struct CompoundCRS {
    pub(crate) identified: Identified,
    pub(crate) components: Vec<Crs>,
}

impl CompoundCRS {
    /// Build a compound CRS.
    ///
    /// # Errors
    ///
    /// Fewer than two components are given. PROJ also rejects two horizontal
    /// CRS; that check is `compoundCRS_invalid`.
    pub fn create(name: impl Into<String>, components: Vec<Crs>) -> Result<Self, FactoryError> {
        if components.len() < 2 {
            return Err(FactoryError::Invalid(
                "a compound CRS needs at least two components".to_owned(),
            ));
        }
        let horizontal = components.iter().filter(|crs| crs.is_horizontal()).count();
        if horizontal > 1 {
            return Err(FactoryError::Invalid(
                "a compound CRS cannot contain two horizontal CRS".to_owned(),
            ));
        }
        Ok(Self {
            identified: Identified::named(name),
            components,
        })
    }

    /// Name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.identified.name
    }

    /// Authority identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[Identifier] {
        &self.identified.identifiers
    }

    /// Component CRS, in order.
    #[must_use]
    pub fn components(&self) -> &[Crs] {
        &self.components
    }

    /// `CompoundCRS::createLax`: a geographic CRS plus an ellipsoidal-height
    /// vertical CRS (`VERT_DATUM` type `2002`) becomes one geographic 3D CRS.
    ///
    /// The promoted CRS keeps the horizontal name and drops authority codes.
    /// PROJ's other lax case, a second geographic CRS equivalent to the first,
    /// is not handled here.
    ///
    /// # Errors
    ///
    /// The components are not a valid compound CRS.
    pub fn create_lax(name: impl Into<String>, components: Vec<Crs>) -> Result<Crs, FactoryError> {
        if let [Crs::Geographic(horizontal), Crs::Vertical(vertical)] = components.as_slice() {
            if horizontal.coordinate_system.axes.len() == 2
                && vertical
                    .datum
                    .as_ref()
                    .is_some_and(|datum| datum.wkt1_type == "2002")
            {
                let axis = vertical.coordinate_system.axis();
                let mut promoted = horizontal.clone();
                promoted.identified.identifiers.clear();
                let metre_up = axis.direction() == "up"
                    && axis.unit().kind() == UnitKind::Linear
                    && (axis.unit().conversion_to_si() - 1.0).abs() < 1e-12;
                if !metre_up {
                    promoted.identified.name =
                        format!("{} ({})", horizontal.name(), vertical.name());
                }
                promoted.coordinate_system.axes.push(Axis::new(
                    "Ellipsoidal height",
                    "h",
                    axis.direction(),
                    axis.unit().clone(),
                ));
                return Ok(Crs::Geographic(promoted));
            }
        }
        Ok(Crs::Compound(Self::create(name, components)?))
    }

    /// Usages recorded for this CRS.
    #[must_use]
    pub fn domains(&self) -> &[ObjectDomain] {
        &self.identified.domains
    }
}

/// Bound CRS: a source CRS, a hub, and the transformation that binds them.
#[derive(Clone, Debug)]
pub struct BoundCRS {
    pub(crate) identified: Identified,
    pub(crate) source: Box<Crs>,
    pub(crate) target: Box<Crs>,
    pub(crate) transformation: CoordinateOperation,
}

impl BoundCRS {
    /// Build a bound CRS.
    #[must_use]
    pub fn create(source: Crs, target: Crs, transformation: CoordinateOperation) -> Self {
        let name = source.name().to_owned();
        Self {
            identified: Identified::named(name),
            source: Box::new(source),
            target: Box::new(target),
            transformation,
        }
    }

    /// Name, taken from the source CRS when built with [`Self::create`].
    #[must_use]
    pub fn name(&self) -> &str {
        &self.identified.name
    }

    /// Source CRS.
    #[must_use]
    pub fn source_crs(&self) -> &Crs {
        &self.source
    }

    /// Hub CRS.
    #[must_use]
    pub fn target_crs(&self) -> &Crs {
        &self.target
    }

    /// Binding transformation.
    #[must_use]
    pub fn transformation(&self) -> &CoordinateOperation {
        &self.transformation
    }
}

/// Coordinate reference system.
#[derive(Clone, Debug)]
#[expect(
    clippy::large_enum_variant,
    reason = "each CRS kind owns its datum and coordinate system"
)]
pub enum Crs {
    /// Geographic CRS.
    Geographic(GeographicCRS),
    /// Geocentric or spherical geodetic CRS.
    Geodetic(GeodeticCRS),
    /// Projected CRS.
    Projected(ProjectedCRS),
    /// Vertical CRS.
    Vertical(VerticalCRS),
    /// Compound CRS.
    Compound(CompoundCRS),
    /// Bound CRS.
    Bound(BoundCRS),
    /// Engineering CRS.
    Engineering(EngineeringCRS),
    /// Derived projected CRS.
    DerivedProjected(DerivedProjectedCRS),
}

impl Crs {
    /// Name.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Geographic(crs) => crs.name(),
            Self::Geodetic(crs) => crs.name(),
            Self::Projected(crs) => crs.name(),
            Self::Vertical(crs) => crs.name(),
            Self::Compound(crs) => crs.name(),
            Self::Bound(crs) => crs.name(),
            Self::Engineering(crs) => crs.name(),
            Self::DerivedProjected(crs) => crs.name(),
        }
    }

    /// Authority identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[Identifier] {
        match self {
            Self::Geographic(crs) => crs.identifiers(),
            Self::Geodetic(crs) => crs.identifiers(),
            Self::Projected(crs) => crs.identifiers(),
            Self::Vertical(crs) => crs.identifiers(),
            Self::Compound(crs) => crs.identifiers(),
            Self::Bound(crs) => &crs.identified.identifiers,
            Self::Engineering(crs) => &crs.identified.identifiers,
            Self::DerivedProjected(crs) => &crs.identified.identifiers,
        }
    }

    /// First authority code whose code space matches `authority`, ignoring case.
    #[must_use]
    pub fn authority_code(&self, authority: &str) -> Option<&Identifier> {
        self.identifiers()
            .iter()
            .find(|identifier| identifier.code_space.eq_ignore_ascii_case(authority))
    }

    /// Deprecated flag stored for this CRS.
    #[must_use]
    pub fn is_deprecated(&self) -> bool {
        match self {
            Self::Geographic(crs) => crs.identified.deprecated,
            Self::Geodetic(crs) => match crs {
                GeodeticCRS::Geographic(crs) => crs.identified.deprecated,
                GeodeticCRS::Geocentric(crs) => crs.identified.deprecated,
                GeodeticCRS::Spherical(crs) => crs.identified.deprecated,
            },
            Self::Projected(crs) => crs.identified.deprecated,
            Self::Vertical(crs) => crs.identified.deprecated,
            Self::Compound(crs) => crs.identified.deprecated,
            Self::Bound(crs) => crs.identified.deprecated,
            Self::Engineering(crs) => crs.identified.deprecated,
            Self::DerivedProjected(crs) => crs.identified.deprecated,
        }
    }

    pub(crate) fn is_horizontal(&self) -> bool {
        match self {
            Self::Vertical(_) => false,
            Self::Compound(crs) => crs.components.iter().any(Self::is_horizontal),
            Self::Bound(crs) => crs.source.is_horizontal(),
            _ => true,
        }
    }
}

/// Names PROJ rewrites when an ensemble is exported as a single datum.
pub(crate) fn ensemble_name_as_datum(name: &str) -> Option<&'static str> {
    match name {
        "World Geodetic System 1984 ensemble" => Some("World Geodetic System 1984"),
        "European Terrestrial Reference System 1989 ensemble" => {
            Some("European Terrestrial Reference System 1989")
        }
        "Greenland Reference 1996 ensemble" => Some("Greenland 1996"),
        _ => None,
    }
}
