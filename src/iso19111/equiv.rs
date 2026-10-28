//! `isEquivalentTo` for the objects the CRS and factory tests compare.
//!
//! `Strict` is PROJ's default criterion: the name must match. `Equivalent`
//! does not require the CRS name to match. `ExceptAxisOrder` also allows a
//! geographic latitude/longitude swap.

use super::model::{
    ensemble_name_as_datum, Axis, CompoundCRS, Conversion, CoordinateSystem, Crs, Datum,
    DatumEnsemble, Ellipsoid, EngineeringCRS, EngineeringDatum, GeodeticCRS,
    GeodeticReferenceFrame, GeographicCRS, Identified, Parameter, ParameterValue, PrimeMeridian,
    ProjectedCRS, VerticalCRS, VerticalReferenceFrame,
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
    ///
    /// Axes match on direction and unit, which is PROJ's `EQUIVALENT` criterion.
    /// An ensemble is compared as the datum `asDatum` would build.
    #[must_use]
    pub fn is_2d_part_of_3d(&self, other: &Self) -> bool {
        let left = self.coordinate_system().axes();
        let right = other.coordinate_system().axes();
        if left.len() != 2 || right.len() != 3 {
            return false;
        }
        axis_direction_unit(&left[0], &right[0])
            && axis_direction_unit(&left[1], &right[1])
            && match (datum_frame(self), datum_frame(other)) {
                (Some(left), Some(right)) => frame_equivalent(&left, &right),
                _ => false,
            }
    }
}

/// Datum or ensemble-as-datum matches, and the angular unit matches.
///
/// This is the base check of `createOperation` from a geographic CRS to a
/// projected CRS of that datum. An ensemble is compared as `asDatum`.
pub(crate) fn same_geographic_base(left: &GeographicCRS, right: &GeographicCRS) -> bool {
    let (Some(left_frame), Some(right_frame)) = (datum_frame(left), datum_frame(right)) else {
        return false;
    };
    if !frame_equivalent(&left_frame, &right_frame) {
        return false;
    }
    match (
        left.coordinate_system().axes().first().map(Axis::unit),
        right.coordinate_system().axes().first().map(Axis::unit),
    ) {
        (Some(left_unit), Some(right_unit)) => {
            left_unit.name() == right_unit.name() && left_unit.factor == right_unit.factor
        }
        _ => false,
    }
}

fn axis_direction_unit(left: &Axis, right: &Axis) -> bool {
    left.direction() == right.direction()
        && left.unit().name() == right.unit().name()
        && left.unit().factor == right.unit().factor
        && left.unit().kind() == right.unit().kind()
        && meridian_value(left) == meridian_value(right)
}

fn datum_frame(crs: &GeographicCRS) -> Option<GeodeticReferenceFrame> {
    if let Some(datum) = crs.datum() {
        return Some(datum.clone());
    }
    let ensemble = crs.datum_ensemble()?;
    let member = ensemble.members().iter().find_map(|member| match member {
        Datum::Geodetic(frame) => Some(frame),
        _ => None,
    })?;
    let name = ensemble_name_as_datum(ensemble.name()).unwrap_or(ensemble.name());
    Some(GeodeticReferenceFrame {
        identified: Identified::named(name),
        ellipsoid: member.ellipsoid().clone(),
        prime_meridian: member.prime_meridian().clone(),
        frame_epoch: None,
        deformation_model: None,
    })
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
        self.is_equivalent_to_with(other, Equivalence::Strict)
    }

    /// `isEquivalentTo` with `criterion`.
    ///
    /// `Equivalent` does not require the projected CRS name. `ExceptAxisOrder`
    /// compares the base geographic CRS with a latitude/longitude swap allowed,
    /// and compares the projected axes by direction and unit.
    #[must_use]
    pub fn is_equivalent_to_with(&self, other: &Self, criterion: Equivalence) -> bool {
        let standard = standard_criterion(criterion);
        if criterion == Equivalence::Strict && !same_name(self.name(), other.name()) {
            return false;
        }
        geodetic_equivalent(self.base_crs(), other.base_crs(), criterion)
            && conversion_equivalent_with(
                self.deriving_conversion(),
                other.deriving_conversion(),
                standard,
            )
            && axes_for_criterion(
                self.coordinate_system().axes(),
                other.coordinate_system().axes(),
                standard,
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
    let anchored = if criterion == Equivalence::Strict {
        geographic_anchor(left, right)
    } else {
        // Non-strict comparison uses `datumNonNull`, so an ensemble matches
        // the datum `asDatum` would build.
        frames_as_datum(left, right)
    };
    if anchored
        && axes_for_criterion(
            left.coordinate_system().axes(),
            right.coordinate_system().axes(),
            criterion,
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
    anchored
        && axes_for_criterion(
            left.coordinate_system().axes(),
            &swapped,
            Equivalence::Equivalent,
        )
}

fn frames_as_datum(left: &GeographicCRS, right: &GeographicCRS) -> bool {
    match (anchor_frame(left), anchor_frame(right)) {
        (Some(left), Some(right)) => frame_equivalent(&left, &right),
        _ => false,
    }
}

fn anchor_frame(crs: &GeographicCRS) -> Option<GeodeticReferenceFrame> {
    if let Some(datum) = crs.datum() {
        return Some(datum.clone());
    }
    match crs.datum_ensemble()?.as_datum() {
        Some(Datum::Geodetic(frame)) => Some(frame),
        _ => None,
    }
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

pub(crate) fn meridian_equivalent(left: &PrimeMeridian, right: &PrimeMeridian) -> bool {
    same_name(left.name(), right.name())
        && left.longitude().value() == right.longitude().value()
        && same_name(left.longitude().unit().name(), right.longitude().unit().name())
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

fn standard_criterion(criterion: Equivalence) -> Equivalence {
    if criterion == Equivalence::ExceptAxisOrder {
        Equivalence::Equivalent
    } else {
        criterion
    }
}

fn conversion_equivalent(left: &Conversion, right: &Conversion) -> bool {
    conversion_equivalent_with(left, right, Equivalence::Strict)
}

fn conversion_equivalent_with(
    left: &Conversion,
    right: &Conversion,
    criterion: Equivalence,
) -> bool {
    if criterion == Equivalence::Strict {
        return same_name(left.name(), right.name())
            && left.method().code() == right.method().code()
            && left.parameters().len() == right.parameters().len()
            && left
                .parameters()
                .iter()
                .zip(right.parameters())
                .all(|(left, right)| parameter_equivalent(left, right));
    }
    if !methods_equivalent(left.method(), right.method()) {
        return false;
    }
    if left.parameters().len() != right.parameters().len() {
        return false;
    }
    let mut used = vec![false; right.parameters().len()];
    for parameter in left.parameters() {
        let mut found = false;
        for (index, other) in right.parameters().iter().enumerate() {
            if used[index] {
                continue;
            }
            if parameter_equivalent_with(parameter, other) {
                used[index] = true;
                found = true;
                break;
            }
        }
        if !found {
            return false;
        }
    }
    true
}

fn methods_equivalent(left: &super::model::OperationMethod, right: &super::model::OperationMethod) -> bool {
    let left_code = left.code();
    let right_code = right.code();
    (!left_code.is_empty() && left_code == right_code) || equivalent_name(left.name(), right.name())
}

fn parameter_equivalent_with(left: &Parameter, right: &Parameter) -> bool {
    let same_parameter = (!left.code().is_empty() && left.code() == right.code())
        || equivalent_name(left.name(), right.name());
    if !same_parameter {
        return false;
    }
    match (left.value(), right.value()) {
        (ParameterValue::Measure(left), ParameterValue::Measure(right)) => {
            measures_close(left, right)
        }
        (ParameterValue::String(left), ParameterValue::String(right)) => left == right,
        (ParameterValue::Integer(left), ParameterValue::Integer(right)) => left == right,
        (ParameterValue::Filename(left), ParameterValue::Filename(right)) => left == right,
        (ParameterValue::Boolean(left), ParameterValue::Boolean(right)) => left == right,
        _ => false,
    }
}

/// PROJ compares non-strict measures with a relative error of `2e-10` on the SI value.
fn measures_close(left: &super::model::Measure, right: &super::model::Measure) -> bool {
    let left_si = left.si_value();
    let right_si = right.si_value();
    if left_si.is_infinite() && right_si.is_infinite() {
        return left_si.is_sign_positive() == right_si.is_sign_positive();
    }
    (left_si - right_si).abs() <= 2e-10 * left_si.abs()
}

fn axes_for_criterion(left: &[Axis], right: &[Axis], criterion: Equivalence) -> bool {
    if criterion == Equivalence::Strict {
        return cs_axes_match(left, right);
    }
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            left.direction() == right.direction() && units_close(left.unit(), right.unit())
        })
}

fn units_close(left: &super::model::UnitOfMeasure, right: &super::model::UnitOfMeasure) -> bool {
    left.kind() == right.kind()
        && (left.conversion_to_si() - right.conversion_to_si()).abs()
            <= 1e-10 * left.conversion_to_si().abs()
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

/// `Identifier::isEquivalentName`: case-insensitive, ignoring space, `_`, `-`,
/// `/`, parentheses, `.`, `&`, and `,`.
pub(crate) fn equivalent_name(left: &str, right: &str) -> bool {
    folded_name(left) == folded_name(right)
}

fn folded_name(name: &str) -> String {
    name.chars()
        .filter(|ch| !matches!(ch, ' ' | '_' | '-' | '/' | '(' | ')' | '.' | '&' | ','))
        .flat_map(char::to_lowercase)
        .collect()
}

impl GeodeticCRS {
    /// `GeodeticCRS::identify` with no authority factory.
    ///
    /// The candidates are the hardcoded EPSG:4326, EPSG:4267, and EPSG:4269.
    /// A database search is `identify` with a factory, which is a later test.
    #[must_use]
    pub fn identify(&self) -> Vec<(Crs, i32)> {
        let name = self.name();
        let axis_order = match self {
            Self::Geographic(crs) if crs.implicit_cs => IdentifyAxisOrder::Implicit,
            _ => IdentifyAxisOrder::Exact,
        };
        let candidates = [
            Self::Geographic(GeographicCRS::epsg_4326()),
            Self::Geographic(GeographicCRS::epsg_4267()),
            Self::Geographic(GeographicCRS::epsg_4269()),
        ];
        for candidate in candidates {
            let candidate_name = candidate.name();
            let name_equal = name == candidate_name;
            let name_equivalent = equivalent_name(name, candidate_name);
            let equivalent = identify_equivalent(self, &candidate, axis_order);
            let confidence = if name_equivalent && equivalent {
                if name_equal { 100 } else { 90 }
            } else if name_equal && !equivalent {
                25
            } else if equivalent {
                70
            } else {
                continue;
            };
            return vec![(candidate_as_crs(candidate), confidence)];
        }
        Vec::new()
    }
}

impl Crs {
    /// `CRS::identify` with no authority factory.
    ///
    /// Geographic and geodetic CRS use [`GeodeticCRS::identify`]. A projected
    /// CRS uses the hardcoded UTM candidates. Other kinds have none.
    #[must_use]
    pub fn identify(&self) -> Vec<(Crs, i32)> {
        match self {
            Self::Geographic(crs) => Self::Geodetic(GeodeticCRS::Geographic(crs.clone())).identify(),
            Self::Geodetic(crs) => crs.identify(),
            Self::Projected(crs) => crs.identify_with(None),
            _ => Vec::new(),
        }
    }
}

fn candidate_as_crs(crs: GeodeticCRS) -> Crs {
    match crs {
        GeodeticCRS::Geographic(crs) => Crs::Geographic(crs),
        other => Crs::Geodetic(other),
    }
}

/// Whether identification may treat a latitude/longitude swap as the same CS.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum IdentifyAxisOrder {
    /// Axis directions must line up in order.
    Exact,
    /// A geographic latitude/longitude swap still matches.
    Implicit,
}

fn identify_equivalent(
    this: &GeodeticCRS,
    candidate: &GeodeticCRS,
    axis_order: IdentifyAxisOrder,
) -> bool {
    identify_anchors(this, candidate) && identify_axes_match(this, candidate, axis_order)
}

fn identify_anchors(this: &GeodeticCRS, candidate: &GeodeticCRS) -> bool {
    match (this, candidate) {
        (GeodeticCRS::Geographic(left), GeodeticCRS::Geographic(right)) => {
            geographic_anchor(left, right)
        }
        (GeodeticCRS::Geocentric(left), GeodeticCRS::Geocentric(right)) => {
            match (left.datum(), right.datum()) {
                (Some(left_datum), Some(right_datum)) => frame_equivalent(left_datum, right_datum),
                _ => false,
            }
        }
        _ => false,
    }
}

/// Axis direction and unit match for identification.
///
/// Datum equivalence is separate: the no-database path uses names, and the
/// database path also accepts aliases.
pub(crate) fn identify_axes_match(
    this: &GeodeticCRS,
    candidate: &GeodeticCRS,
    axis_order: IdentifyAxisOrder,
) -> bool {
    match (this, candidate) {
        (GeodeticCRS::Geographic(left), GeodeticCRS::Geographic(right)) => {
            geographic_axes_for_identify(left, right, axis_order)
        }
        (GeodeticCRS::Geocentric(left), GeodeticCRS::Geocentric(right)) => directions_and_units(
            left.coordinate_system().axes(),
            right.coordinate_system().axes(),
        ),
        _ => false,
    }
}

/// Ellipsoid size matches. Names are not compared.
///
/// This is `Ellipsoid::_isEquivalentTo` with `EQUIVALENT`: semi-major axis and
/// inverse flattening within a relative `1e-10`.
pub(crate) fn ellipsoid_parameters_match(left: &Ellipsoid, right: &Ellipsoid) -> bool {
    let major = left.semi_major_axis().si_value();
    let other_major = right.semi_major_axis().si_value();
    if (major - other_major).abs() > 1e-10 * major.abs() {
        return false;
    }
    let flattening = left.inverse_flattening();
    let other = right.inverse_flattening();
    let scale = flattening.abs().max(1.0);
    (flattening - other).abs() <= 1e-10 * scale
}

fn geographic_axes_for_identify(
    left: &GeographicCRS,
    right: &GeographicCRS,
    axis_order: IdentifyAxisOrder,
) -> bool {
    let left_axes = left.coordinate_system().axes();
    let right_axes = right.coordinate_system().axes();
    if directions_and_units(left_axes, right_axes) {
        return true;
    }
    axis_order == IdentifyAxisOrder::Implicit
        && left_axes.len() == 2
        && right_axes.len() == 2
        && direction_and_unit(&left_axes[0], &right_axes[1])
        && direction_and_unit(&left_axes[1], &right_axes[0])
}

pub(crate) fn directions_and_units(left: &[Axis], right: &[Axis]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| direction_and_unit(left, right))
}

fn direction_and_unit(left: &Axis, right: &Axis) -> bool {
    let scale = left.unit().conversion_to_si().abs();
    left.direction() == right.direction()
        && left.unit().kind() == right.unit().kind()
        && (left.unit().conversion_to_si() - right.unit().conversion_to_si()).abs()
            <= 1e-10 * scale
}

pub(crate) fn same_epoch(left: Option<f64>, right: Option<f64>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left == right,
        (None, None) => true,
        _ => false,
    }
}
