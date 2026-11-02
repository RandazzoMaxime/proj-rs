//! Temporal, parametric and derived CRS (ISO 19111 `TemporalCRS`, `ParametricCRS`,
//! `DerivedGeographicCRS`, `DerivedVerticalCRS` and the other derived kinds).

use super::model::{
    Axis, Conversion, CoordinateSystem, Crs, EngineeringDatum, FactoryError, GeodeticCRS,
    Identified, Identifier, ObjectDomain, UnitKind, UnitOfMeasure,
};

/// Calendar of a [`TemporalDatum`] when WKT names none.
pub(crate) const PROLEPTIC_GREGORIAN: &str = "proleptic Gregorian";

/// `TemporalDatum::CALENDAR_PROLEPTIC_GREGORIAN`.
pub const CALENDAR_PROLEPTIC_GREGORIAN: &str = PROLEPTIC_GREGORIAN;

/// Kind of a temporal coordinate system. Same variants as [`TemporalCsKind`].
pub type TemporalKind = TemporalCsKind;

/// The unit of a parametric axis, such as a hectopascal.
#[must_use]
pub fn parametric_unit(name: &str, factor: f64) -> UnitOfMeasure {
    UnitOfMeasure::new(name, factor, UnitKind::Parametric)
}

/// Datum of a temporal CRS: a calendar and an origin.
#[derive(Clone, Debug)]
pub struct TemporalDatum {
    pub(crate) identified: Identified,
    pub(crate) calendar: String,
    pub(crate) origin: String,
}

impl TemporalDatum {
    /// Name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.identified.name
    }

    /// Calendar, such as `proleptic Gregorian`.
    #[must_use]
    pub fn calendar(&self) -> &str {
        &self.calendar
    }

    /// Time origin as written, such as `1980-01-01T00:00:00.0Z`.
    #[must_use]
    pub fn temporal_origin(&self) -> &str {
        &self.origin
    }

    /// Authority identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[Identifier] {
        &self.identified.identifiers
    }

    /// Datum named `name`. `origin` is the text of the `DateTime`, which may be empty.
    #[must_use]
    pub fn create(
        name: impl Into<String>,
        origin: impl Into<String>,
        calendar: impl Into<String>,
    ) -> Self {
        Self {
            identified: Identified::named(name),
            origin: origin.into(),
            calendar: calendar.into(),
        }
    }

    pub(crate) fn is_equivalent_to(&self, other: &Self) -> bool {
        self.name() == other.name()
            && self.origin == other.origin
            && self.calendar == other.calendar
    }
}

/// Datum of a parametric CRS.
#[derive(Clone, Debug)]
pub struct ParametricDatum {
    pub(crate) identified: Identified,
}

impl ParametricDatum {
    /// Name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.identified.name
    }

    /// Anchor definition, when the WKT gives one.
    #[must_use]
    pub fn anchor_definition(&self) -> Option<&str> {
        self.identified.anchor.as_deref()
    }

    /// Anchor definition. Same value as [`Self::anchor_definition`].
    #[must_use]
    pub fn anchor(&self) -> Option<&str> {
        self.anchor_definition()
    }

    /// Authority identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[Identifier] {
        &self.identified.identifiers
    }

    /// Datum named `name`.
    #[must_use]
    pub fn create(name: impl Into<String>) -> Self {
        Self {
            identified: Identified::named(name),
        }
    }
}

/// How a temporal axis is counted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TemporalCsKind {
    /// A date and time, `TemporalDateTime`. The axis has no unit.
    DateTime,
    /// A whole number of time units, `TemporalCount`.
    Count,
    /// A real number of time units, `TemporalMeasure`.
    Measure,
}

/// Temporal coordinate system: one axis.
#[derive(Clone, Debug)]
pub struct TemporalCS {
    pub(crate) identified: Identified,
    pub(crate) axis: Axis,
    pub(crate) kind: TemporalCsKind,
}

impl TemporalCS {
    /// The single axis.
    #[must_use]
    pub fn axis(&self) -> &Axis {
        &self.axis
    }

    /// Date-time, count or measure.
    #[must_use]
    pub fn kind(&self) -> TemporalCsKind {
        self.kind
    }

    /// Temporal coordinate system of `kind` with the single `axis`.
    #[must_use]
    pub fn create(kind: TemporalCsKind, axis: Axis) -> Self {
        Self {
            identified: Identified::default(),
            axis,
            kind,
        }
    }
}

/// Parametric coordinate system: one axis.
#[derive(Clone, Debug)]
pub struct ParametricCS {
    pub(crate) identified: Identified,
    pub(crate) axis: Axis,
}

impl ParametricCS {
    /// The single axis.
    #[must_use]
    pub fn axis(&self) -> &Axis {
        &self.axis
    }

    /// Parametric coordinate system with the single `axis`.
    #[must_use]
    pub fn create(axis: Axis) -> Self {
        Self {
            identified: Identified::default(),
            axis,
        }
    }
}

/// Temporal CRS.
#[derive(Clone, Debug)]
pub struct TemporalCRS {
    pub(crate) identified: Identified,
    pub(crate) datum: TemporalDatum,
    pub(crate) coordinate_system: TemporalCS,
}

impl TemporalCRS {
    /// Name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.identified.name
    }

    /// Datum.
    #[must_use]
    pub fn datum(&self) -> &TemporalDatum {
        &self.datum
    }

    /// Coordinate system.
    #[must_use]
    pub fn coordinate_system(&self) -> &TemporalCS {
        &self.coordinate_system
    }

    /// Authority identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[Identifier] {
        &self.identified.identifiers
    }

    /// Usages recorded for this CRS.
    #[must_use]
    pub fn domains(&self) -> &[ObjectDomain] {
        &self.identified.domains
    }

    /// Temporal CRS named `name`.
    #[must_use]
    pub fn create(
        name: impl Into<String>,
        datum: TemporalDatum,
        coordinate_system: TemporalCS,
    ) -> Self {
        Self {
            identified: Identified::named(name),
            datum,
            coordinate_system,
        }
    }

    pub(crate) fn is_equivalent_to(&self, other: &Self) -> bool {
        self.name() == other.name()
            && self.datum.is_equivalent_to(&other.datum)
            && CoordinateSystem::Temporal(self.coordinate_system.clone())
                .is_equivalent_to(&CoordinateSystem::Temporal(other.coordinate_system.clone()))
    }
}

/// Parametric CRS.
#[derive(Clone, Debug)]
pub struct ParametricCRS {
    pub(crate) identified: Identified,
    pub(crate) datum: ParametricDatum,
    pub(crate) coordinate_system: ParametricCS,
}

impl ParametricCRS {
    /// Name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.identified.name
    }

    /// Datum.
    #[must_use]
    pub fn datum(&self) -> &ParametricDatum {
        &self.datum
    }

    /// Coordinate system.
    #[must_use]
    pub fn coordinate_system(&self) -> &ParametricCS {
        &self.coordinate_system
    }

    /// Authority identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[Identifier] {
        &self.identified.identifiers
    }

    /// Usages recorded for this CRS.
    #[must_use]
    pub fn domains(&self) -> &[ObjectDomain] {
        &self.identified.domains
    }

    /// Parametric CRS named `name`.
    #[must_use]
    pub fn create(
        name: impl Into<String>,
        datum: ParametricDatum,
        coordinate_system: ParametricCS,
    ) -> Self {
        Self {
            identified: Identified::named(name),
            datum,
            coordinate_system,
        }
    }

    pub(crate) fn is_equivalent_to(&self, other: &Self) -> bool {
        self.name() == other.name()
            && self.datum.name() == other.datum.name()
            && CoordinateSystem::Parametric(self.coordinate_system.clone()).is_equivalent_to(
                &CoordinateSystem::Parametric(other.coordinate_system.clone()),
            )
    }
}

/// What a [`DerivedCRS`] is derived into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DerivedKind {
    /// `DerivedGeographicCRS`: ellipsoidal coordinate system on a geodetic base.
    Geographic,
    /// `DerivedGeodeticCRS`: Cartesian or spherical coordinate system on a geodetic base.
    Geodetic,
    /// `DerivedVerticalCRS`.
    Vertical,
    /// `DerivedEngineeringCRS`.
    Engineering,
    /// `DerivedParametricCRS`.
    Parametric,
    /// `DerivedTemporalCRS`.
    Temporal,
}

/// A CRS derived from a base CRS of any kind but projected (see `DerivedProjectedCRS`):
/// `DerivedGeographicCRS`, `DerivedGeodeticCRS`, `DerivedVerticalCRS`,
/// `DerivedEngineeringCRS`, `DerivedParametricCRS` and `DerivedTemporalCRS`.
#[derive(Clone, Debug)]
pub struct DerivedCRS {
    pub(crate) identified: Identified,
    pub(crate) base: Box<Crs>,
    pub(crate) conversion: Conversion,
    pub(crate) coordinate_system: CoordinateSystem,
}

impl DerivedCRS {
    /// Name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.identified.name
    }

    /// Base CRS.
    #[must_use]
    pub fn base_crs(&self) -> &Crs {
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

    /// Authority identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[Identifier] {
        &self.identified.identifiers
    }

    /// Usages recorded for this CRS.
    #[must_use]
    pub fn domains(&self) -> &[ObjectDomain] {
        &self.identified.domains
    }

    /// `DerivedGeographicCRS::create`, `DerivedGeodeticCRS::create`, `DerivedVerticalCRS::create`,
    /// `DerivedEngineeringCRS::create`, `DerivedParametricCRS::create` or
    /// `DerivedTemporalCRS::create`, chosen from `base` and `coordinate_system`.
    ///
    /// # Errors
    ///
    /// `base` is not a geodetic, vertical, engineering, parametric or temporal CRS, or the
    /// coordinate system does not suit it.
    pub fn create(
        name: impl Into<String>,
        base: Crs,
        conversion: Conversion,
        coordinate_system: CoordinateSystem,
    ) -> Result<Self, FactoryError> {
        let crs = Self {
            identified: Identified::named(name),
            base: Box::new(base),
            conversion,
            coordinate_system,
        };
        if crs.derived_kind().is_none() {
            return Err(FactoryError::Invalid(
                "the base CRS and the coordinate system do not make a derived CRS".to_owned(),
            ));
        }
        Ok(crs)
    }

    fn derived_kind(&self) -> Option<DerivedKind> {
        let cs = &self.coordinate_system;
        match (&*self.base, cs) {
            (Crs::Geographic(_) | Crs::Geodetic(_), CoordinateSystem::Ellipsoidal(_)) => {
                Some(DerivedKind::Geographic)
            }
            (
                Crs::Geographic(_) | Crs::Geodetic(_),
                CoordinateSystem::Cartesian(_) | CoordinateSystem::Spherical(_),
            ) => Some(DerivedKind::Geodetic),
            (Crs::Vertical(_), CoordinateSystem::Vertical(_)) => Some(DerivedKind::Vertical),
            (
                Crs::Engineering(_),
                CoordinateSystem::Cartesian(_)
                | CoordinateSystem::Affine(_)
                | CoordinateSystem::Ordinal(_)
                | CoordinateSystem::Spherical(_)
                | CoordinateSystem::Vertical(_)
                | CoordinateSystem::Ellipsoidal(_),
            ) => Some(DerivedKind::Engineering),
            (Crs::Parametric(_), CoordinateSystem::Parametric(_)) => Some(DerivedKind::Parametric),
            (Crs::Temporal(_), CoordinateSystem::Temporal(_)) => Some(DerivedKind::Temporal),
            _ => None,
        }
    }

    /// What this CRS is derived into.
    #[must_use]
    pub fn kind(&self) -> DerivedKind {
        self.derived_kind().unwrap_or(DerivedKind::Engineering)
    }

    /// The geodetic base CRS, when this CRS is derived from one.
    #[must_use]
    pub fn geodetic_base(&self) -> Option<GeodeticCRS> {
        match &*self.base {
            Crs::Geographic(crs) => Some(GeodeticCRS::Geographic(crs.clone())),
            Crs::Geodetic(crs) => Some(crs.clone()),
            _ => None,
        }
    }

    pub(crate) fn is_equivalent_to(&self, other: &Self) -> bool {
        self.kind() == other.kind()
            && self.name() == other.name()
            && self.base.is_equivalent_to(&other.base)
            && self.conversion.is_equivalent_to(&other.conversion)
            && self
                .coordinate_system
                .is_equivalent_to(&other.coordinate_system)
    }
}

impl EngineeringDatum {
    /// Anchor definition, when the WKT gives one.
    #[must_use]
    pub fn anchor_definition(&self) -> Option<&str> {
        self.identified.anchor.as_deref()
    }
}
