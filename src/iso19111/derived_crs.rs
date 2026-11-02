//! Temporal and parametric CRS, and the CRS derived from a base CRS by a conversion
//! (geographic, geodetic, vertical, engineering, parametric and temporal).
//!
//! The derived projected CRS is [`super::model::DerivedProjectedCRS`]; this module holds the
//! other kinds of PROJ's `DerivedCRS` family as one type, [`DerivedCRS`].

use super::model::{
    Axis, CartesianCS, Conversion, CoordinateSystem, Crs, EngineeringCRS, EngineeringDatum,
    FactoryError, GeodeticCRS, Identified, Identifier, ObjectDomain, UnitKind, UnitOfMeasure,
    VerticalCS,
};

/// `TemporalDatum::CALENDAR_PROLEPTIC_GREGORIAN`.
pub const CALENDAR_PROLEPTIC_GREGORIAN: &str = "proleptic Gregorian";

/// Kind of a temporal coordinate system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TemporalKind {
    /// `DateTimeTemporalCS`: the value is a date and time.
    DateTime,
    /// `TemporalCountCS`: the value is an integer count of the axis unit.
    Count,
    /// `TemporalMeasureCS`: the value is a real number of the axis unit.
    Measure,
}

/// Temporal coordinate system with one axis.
#[derive(Clone, Debug)]
pub struct TemporalCS {
    pub(crate) identified: Identified,
    pub(crate) kind: TemporalKind,
    pub(crate) axis: Axis,
}

impl TemporalCS {
    /// Temporal coordinate system of `kind` with the single `axis`.
    #[must_use]
    pub fn create(kind: TemporalKind, axis: Axis) -> Self {
        Self {
            identified: Identified::default(),
            kind,
            axis,
        }
    }

    /// Kind of value on the axis.
    #[must_use]
    pub fn kind(&self) -> TemporalKind {
        self.kind
    }

    /// The single axis.
    #[must_use]
    pub fn axis(&self) -> &Axis {
        &self.axis
    }
}

/// Parametric coordinate system with one axis.
#[derive(Clone, Debug)]
pub struct ParametricCS {
    pub(crate) identified: Identified,
    pub(crate) axis: Axis,
}

impl ParametricCS {
    /// Parametric coordinate system with the single `axis`.
    #[must_use]
    pub fn create(axis: Axis) -> Self {
        Self {
            identified: Identified::default(),
            axis,
        }
    }

    /// The single axis.
    #[must_use]
    pub fn axis(&self) -> &Axis {
        &self.axis
    }
}

/// Temporal datum: a calendar and the origin of the time axis.
#[derive(Clone, Debug)]
pub struct TemporalDatum {
    pub(crate) identified: Identified,
    pub(crate) origin: String,
    pub(crate) calendar: String,
}

impl TemporalDatum {
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

    /// Name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.identified.name
    }

    /// Text of the time origin.
    #[must_use]
    pub fn temporal_origin(&self) -> &str {
        &self.origin
    }

    /// Calendar name.
    #[must_use]
    pub fn calendar(&self) -> &str {
        &self.calendar
    }

    pub(crate) fn is_equivalent_to(&self, other: &Self) -> bool {
        self.name() == other.name()
            && self.origin == other.origin
            && self.calendar == other.calendar
    }
}

/// Parametric datum.
#[derive(Clone, Debug)]
pub struct ParametricDatum {
    pub(crate) identified: Identified,
}

impl ParametricDatum {
    /// Datum named `name`.
    #[must_use]
    pub fn create(name: impl Into<String>) -> Self {
        Self {
            identified: Identified::named(name),
        }
    }

    /// Name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.identified.name
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

    /// Name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.identified.name
    }

    /// Temporal datum.
    #[must_use]
    pub fn datum(&self) -> &TemporalDatum {
        &self.datum
    }

    /// Coordinate system.
    #[must_use]
    pub fn coordinate_system(&self) -> &TemporalCS {
        &self.coordinate_system
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

    /// Name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.identified.name
    }

    /// Parametric datum.
    #[must_use]
    pub fn datum(&self) -> &ParametricDatum {
        &self.datum
    }

    /// Coordinate system.
    #[must_use]
    pub fn coordinate_system(&self) -> &ParametricCS {
        &self.coordinate_system
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

/// A CRS defined by a deriving conversion from a base CRS of the same kind.
#[derive(Clone, Debug)]
pub struct DerivedCRS {
    pub(crate) identified: Identified,
    pub(crate) base: Box<Crs>,
    pub(crate) conversion: Conversion,
    pub(crate) coordinate_system: CoordinateSystem,
}

impl DerivedCRS {
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
        // `create` refuses a pair without a kind.
        self.derived_kind().unwrap_or(DerivedKind::Engineering)
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

    /// Usages recorded for this CRS.
    #[must_use]
    pub fn domains(&self) -> &[ObjectDomain] {
        &self.identified.domains
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

    pub(crate) fn is_equivalent_to(&self, other: &Self) -> bool {
        self.kind() == other.kind()
            && self.name() == other.name()
            && self.base.is_equivalent_to(&other.base)
            && self.conversion.is_equivalent_to(&other.conversion)
            && self
                .coordinate_system
                .is_equivalent_to(&other.coordinate_system)
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
}

impl EngineeringDatum {
    /// Engineering datum named `name`.
    #[must_use]
    pub fn create(name: impl Into<String>) -> Self {
        Self {
            identified: Identified::named(name),
        }
    }
}

impl EngineeringCRS {
    /// Engineering CRS named `name`.
    #[must_use]
    pub fn create(
        name: impl Into<String>,
        datum: EngineeringDatum,
        coordinate_system: CoordinateSystem,
    ) -> Self {
        Self {
            identified: Identified::named(name),
            datum,
            coordinate_system,
        }
    }
}

impl VerticalCS {
    /// `VerticalCS::createGravityRelatedHeight`.
    #[must_use]
    pub fn create_gravity_related_height(unit: UnitOfMeasure) -> Self {
        Self {
            identified: Identified::default(),
            axis: Axis::create("Gravity-related height", "H", "up", unit),
        }
    }
}

impl CartesianCS {
    /// `CartesianCS::createNorthingEasting`.
    #[must_use]
    pub fn create_northing_easting(unit: UnitOfMeasure) -> Self {
        Self {
            identified: Identified::default(),
            axes: vec![
                Axis::create("Northing", "N", "north", unit.clone()),
                Axis::create("Easting", "E", "east", unit),
            ],
        }
    }
}

/// The unit of a parametric axis, such as a hectopascal.
#[must_use]
pub fn parametric_unit(name: &str, factor: f64) -> UnitOfMeasure {
    UnitOfMeasure::new(name, factor, UnitKind::Parametric)
}
