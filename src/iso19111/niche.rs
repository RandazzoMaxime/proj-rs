//! Temporal, parametric and derived CRS (ISO 19111 `TemporalCRS`, `ParametricCRS`,
//! `DerivedGeographicCRS`, `DerivedVerticalCRS` and the other derived kinds).

use super::model::{
    Axis, Conversion, CoordinateSystem, Crs, EngineeringDatum, Identified, Identifier, ObjectDomain,
};

/// Calendar of a [`TemporalDatum`] when WKT names none.
pub(crate) const PROLEPTIC_GREGORIAN: &str = "proleptic Gregorian";

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

    /// Authority identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[Identifier] {
        &self.identified.identifiers
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
}

impl EngineeringDatum {
    /// Anchor definition, when the WKT gives one.
    #[must_use]
    pub fn anchor_definition(&self) -> Option<&str> {
        self.identified.anchor.as_deref()
    }
}
