//! `isEquivalentTo` for the objects the CRS and factory tests compare.
//!
//! `Strict` is PROJ's default criterion: the name must match. `Equivalent`
//! does not require the CRS name to match. `ExceptAxisOrder` also allows a
//! geographic latitude/longitude swap.

use super::model::{
    Axis, CompoundCRS, Conversion, CoordinateSystem, Crs, Datum, DatumEnsemble, Ellipsoid,
    EngineeringCRS, EngineeringDatum, GeodeticCRS, GeodeticReferenceFrame, GeographicCRS,
    Parameter, ParameterValue, PrimeMeridian, ProjectedCRS, VerticalCRS, VerticalReferenceFrame,
};

/// How much two ISO 19111 objects may differ and still compare equal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Equivalence {
    /// Names match, and so does geographic axis order.
    Strict,
    /// Structure matches. The CRS name is not required to match.
    Equivalent,
    /// [`Self::Equivalent`], and latitude/longitude may be swapped.
    ExceptAxisOrder,
}

impl GeographicCRS {
    /// `isEquivalentTo` with the strict criterion.
    #[must_use]
    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        self.is_equivalent_to_with(other, Equivalence::Strict)
    }

    /// `isEquivalentTo` with `criterion`.
    #[must_use]
    pub fn is_equivalent_to_with(&self, other: &Self, criterion: Equivalence) -> bool {
        geographic_equivalent(self, other, criterion)
    }

    /// `other` is `None` when the value is not a geographic CRS.
    #[must_use]
    pub fn is_equivalent_to_geographic(
        &self,
        other: Option<&Self>,
        criterion: Equivalence,
    ) -> bool {
        other.is_some_and(|other| self.is_equivalent_to_with(other, criterion))
    }

    /// True when this 2D CRS is the horizontal part of `other`.
    #[must_use]
    pub fn is_2d_part_of_3d(&self, other: &Self) -> bool {
        let left = self.coordinate_system().axes();
        let right = other.coordinate_system().axes();
        if left.len() != 2 || right.len() != 3 {
            return false;
        }
        axis_equivalent(&left[0], &right[0])
            && axis_equivalent(&left[1], &right[1])
            && match (self.datum(), other.datum()) {
                (Some(left), Some(right)) => frame_equivalent(left, right),
                _ => false,
            }
    }
}

impl GeodeticReferenceFrame {
    /// Strict datum equivalence: name, epoch, ellipsoid, and prime meridian.
    #[must_use]
    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        frame_equivalent(self, other)
    }

    /// Equivalence against a [`Datum`], false when the kind differs.
    #[must_use]
    pub fn is_equivalent_to_datum(&self, other: &Datum) -> bool {
        matches!(other, Datum::Geodetic(datum) if frame_equivalent(self, datum))
    }
}

impl VerticalReferenceFrame {
    /// Strict vertical-datum equivalence.
    #[must_use]
    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        vertical_equivalent(self, other)
    }

    /// Equivalence against a [`Datum`], false when the kind differs.
    #[must_use]
    pub fn is_equivalent_to_datum(&self, other: &Datum) -> bool {
        matches!(other, Datum::Vertical(datum) if vertical_equivalent(self, datum))
    }
}

impl EngineeringDatum {
    /// Strict engineering-datum equivalence.
    #[must_use]
    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        same_name(self.name(), other.name())
    }

    /// Equivalence against a [`Datum`], false when the kind differs.
    #[must_use]
    pub fn is_equivalent_to_datum(&self, other: &Datum) -> bool {
        matches!(other, Datum::Engineering(datum) if self.is_equivalent_to(datum))
    }
}

impl Datum {
    /// Strict equivalence of two datums of the same kind.
    #[must_use]
    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        datum_equivalent(self, other)
    }
}

impl DatumEnsemble {
    /// Strict ensemble equivalence.
    #[must_use]
    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        ensemble_equivalent(self, other)
    }
}

impl Ellipsoid {
    /// Strict ellipsoid equivalence.
    #[must_use]
    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        ellipsoid_equivalent(self, other)
    }
}

impl PrimeMeridian {
    /// Strict prime-meridian equivalence.
    #[must_use]
    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        meridian_equivalent(self, other)
    }
}

impl CoordinateSystem {
    /// Strict coordinate-system equivalence.
    #[must_use]
    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        coordinate_system_equivalent(self, other)
    }
}

impl Conversion {
    /// Strict conversion equivalence.
    #[must_use]
    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        conversion_equivalent(self, other)
    }
}

impl GeodeticCRS {
    /// `isEquivalentTo` with `criterion`.
    #[must_use]
    pub fn is_equivalent_to_with(&self, other: &Self, criterion: Equivalence) -> bool {
        geodetic_equivalent(self, other, criterion)
    }

    /// Strict geodetic equivalence.
    #[must_use]
    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        self.is_equivalent_to_with(other, Equivalence::Strict)
    }
}

impl ProjectedCRS {
    /// Strict projected equivalence.
    #[must_use]
    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        same_name(self.name(), other.name())
            && geodetic_equivalent(self.base_crs(), other.base_crs(), Equivalence::Strict)
            && conversion_equivalent(self.deriving_conversion(), other.deriving_conversion())
            && cs_axes_match(
                self.coordinate_system().axes(),
                other.coordinate_system().axes(),
            )
    }
}

impl VerticalCRS {
    /// Strict vertical-CRS equivalence.
    #[must_use]
    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        same_name(self.name(), other.name())
            && vertical_anchor(self, other)
            && cs_axes_match(
                std::slice::from_ref(self.coordinate_system().axis()),
                std::slice::from_ref(other.coordinate_system().axis()),
            )
    }
}

impl EngineeringCRS {
    /// Strict engineering-CRS equivalence.
    #[must_use]
    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        same_name(self.name(), other.name())
            && self.datum().is_equivalent_to(other.datum())
            && coordinate_system_equivalent(self.coordinate_system(), other.coordinate_system())
    }
}

impl CompoundCRS {
    /// Strict compound equivalence.
    #[must_use]
    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        same_name(self.name(), other.name())
            && self.components().len() == other.components().len()
            && self
                .components()
                .iter()
                .zip(other.components())
                .all(|(left, right)| left.is_equivalent_to(right))
    }
}

impl Crs {
    /// Strict equivalence. A different CRS kind is not equivalent.
    #[must_use]
    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Geographic(left), Self::Geographic(right)) => left.is_equivalent_to(right),
            (Self::Geodetic(left), Self::Geodetic(right)) => left.is_equivalent_to(right),
            (Self::Geodetic(GeodeticCRS::Geographic(left)), Self::Geographic(right))
            | (Self::Geographic(left), Self::Geodetic(GeodeticCRS::Geographic(right))) => {
                left.is_equivalent_to(right)
            }
            (Self::Projected(left), Self::Projected(right)) => left.is_equivalent_to(right),
            (Self::Vertical(left), Self::Vertical(right)) => left.is_equivalent_to(right),
            (Self::Engineering(left), Self::Engineering(right)) => left.is_equivalent_to(right),
            (Self::Compound(left), Self::Compound(right)) => left.is_equivalent_to(right),
            _ => false,
        }
    }
}

fn geographic_equivalent(
    left: &GeographicCRS,
    right: &GeographicCRS,
    criterion: Equivalence,
) -> bool {
    if criterion == Equivalence::Strict && !same_name(left.name(), right.name()) {
        return false;
    }
    if geographic_anchor(left, right)
        && cs_axes_match(
            left.coordinate_system().axes(),
            right.coordinate_system().axes(),
        )
    {
        return true;
    }
    if criterion != Equivalence::ExceptAxisOrder {
        return false;
    }
    let mut swapped: Vec<Axis> = right.coordinate_system().axes().to_vec();
    if swapped.len() == 2 {
        swapped.reverse();
    } else if swapped.len() == 3 {
        swapped.swap(0, 1);
    } else {
        return false;
    }
    geographic_anchor(left, right) && cs_axes_match(left.coordinate_system().axes(), &swapped)
}

fn geographic_anchor(left: &GeographicCRS, right: &GeographicCRS) -> bool {
    match (
        left.datum(),
        right.datum(),
        left.datum_ensemble(),
        right.datum_ensemble(),
    ) {
        (Some(left), Some(right), _, _) => frame_equivalent(left, right),
        (None, None, Some(left), Some(right)) => ensemble_equivalent(left, right),
        _ => false,
    }
}

fn geodetic_equivalent(left: &GeodeticCRS, right: &GeodeticCRS, criterion: Equivalence) -> bool {
    match (left, right) {
        (GeodeticCRS::Geographic(left), GeodeticCRS::Geographic(right)) => {
            geographic_equivalent(left, right, criterion)
        }
        (GeodeticCRS::Geocentric(left), GeodeticCRS::Geocentric(right)) => {
            (criterion != Equivalence::Strict || same_name(left.name(), right.name()))
                && match (
                    left.datum(),
                    right.datum(),
                    left.ensemble.as_ref(),
                    right.ensemble.as_ref(),
                ) {
                    (Some(left), Some(right), _, _) => frame_equivalent(left, right),
                    (None, None, Some(left), Some(right)) => ensemble_equivalent(left, right),
                    _ => false,
                }
                && cs_axes_match(
                    left.coordinate_system().axes(),
                    right.coordinate_system().axes(),
                )
        }
        _ => false,
    }
}

fn vertical_anchor(left: &VerticalCRS, right: &VerticalCRS) -> bool {
    match (
        left.datum(),
        right.datum(),
        left.datum_ensemble(),
        right.datum_ensemble(),
    ) {
        (Some(left), Some(right), _, _) => vertical_equivalent(left, right),
        (None, None, Some(left), Some(right)) => ensemble_equivalent(left, right),
        _ => false,
    }
}

fn frame_equivalent(left: &GeodeticReferenceFrame, right: &GeodeticReferenceFrame) -> bool {
    same_name(left.name(), right.name())
        && same_epoch(left.frame_reference_epoch(), right.frame_reference_epoch())
        && ellipsoid_equivalent(left.ellipsoid(), right.ellipsoid())
        && meridian_equivalent(left.prime_meridian(), right.prime_meridian())
}

fn vertical_equivalent(left: &VerticalReferenceFrame, right: &VerticalReferenceFrame) -> bool {
    same_name(left.name(), right.name())
        && same_epoch(left.frame_reference_epoch(), right.frame_reference_epoch())
}

fn ensemble_equivalent(left: &DatumEnsemble, right: &DatumEnsemble) -> bool {
    same_name(left.name(), right.name())
        && left.positional_accuracy() == right.positional_accuracy()
        && left.members().len() == right.members().len()
        && left
            .members()
            .iter()
            .zip(right.members())
            .all(|(left, right)| datum_equivalent(left, right))
}

fn datum_equivalent(left: &Datum, right: &Datum) -> bool {
    match (left, right) {
        (Datum::Geodetic(left), Datum::Geodetic(right)) => frame_equivalent(left, right),
        (Datum::Vertical(left), Datum::Vertical(right)) => vertical_equivalent(left, right),
        (Datum::Engineering(left), Datum::Engineering(right)) => {
            same_name(left.name(), right.name())
        }
        _ => false,
    }
}

fn ellipsoid_equivalent(left: &Ellipsoid, right: &Ellipsoid) -> bool {
    same_name(left.name(), right.name())
        && left.semi_major_axis().si_value() == right.semi_major_axis().si_value()
        && left.inverse_flattening() == right.inverse_flattening()
        && match (left.semi_minor_axis(), right.semi_minor_axis()) {
            (Some(left), Some(right)) => left.si_value() == right.si_value(),
            (None, None) => true,
            _ => false,
        }
}

fn meridian_equivalent(left: &PrimeMeridian, right: &PrimeMeridian) -> bool {
    same_name(left.name(), right.name())
        && left.longitude().value() == right.longitude().value()
        && left.longitude().unit().name() == right.longitude().unit().name()
}

fn coordinate_system_equivalent(left: &CoordinateSystem, right: &CoordinateSystem) -> bool {
    std::mem::discriminant(left) == std::mem::discriminant(right)
        && cs_axes_match_refs(&left.axes(), &right.axes())
}

fn cs_axes_match(left: &[Axis], right: &[Axis]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| axis_equivalent(left, right))
}

fn cs_axes_match_refs(left: &[&Axis], right: &[&Axis]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| axis_equivalent(left, right))
}

fn axis_equivalent(left: &Axis, right: &Axis) -> bool {
    same_name(left.name(), right.name())
        && left.abbreviation() == right.abbreviation()
        && left.direction() == right.direction()
        && left.unit().name() == right.unit().name()
        && meridian_value(left) == meridian_value(right)
}

fn meridian_value(axis: &Axis) -> Option<f64> {
    axis.meridian().map(|meridian| meridian.longitude().value())
}

fn conversion_equivalent(left: &Conversion, right: &Conversion) -> bool {
    same_name(left.name(), right.name())
        && left.method().code() == right.method().code()
        && left.parameters().len() == right.parameters().len()
        && left
            .parameters()
            .iter()
            .zip(right.parameters())
            .all(|(left, right)| parameter_equivalent(left, right))
}

fn parameter_equivalent(left: &Parameter, right: &Parameter) -> bool {
    left.code() == right.code()
        && left.name() == right.name()
        && match (left.value(), right.value()) {
            (ParameterValue::Measure(left), ParameterValue::Measure(right)) => {
                left.value() == right.value() && left.unit().name() == right.unit().name()
            }
            (ParameterValue::String(left), ParameterValue::String(right)) => left == right,
            (ParameterValue::Integer(left), ParameterValue::Integer(right)) => left == right,
            (ParameterValue::Filename(left), ParameterValue::Filename(right)) => left == right,
            (ParameterValue::Boolean(left), ParameterValue::Boolean(right)) => left == right,
            _ => false,
        }
}

fn same_name(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

fn same_epoch(left: Option<f64>, right: Option<f64>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left == right,
        (None, None) => true,
        _ => false,
    }
}
