//! `isEquivalentTo` for the objects the CRS and factory tests compare.
//!
//! `Strict` is PROJ's default criterion: the name must match. `Equivalent`
//! does not require the CRS name to match. `ExceptAxisOrder` also allows a
//! geographic latitude/longitude swap.

use super::model::{
    ensemble_name_as_datum, Axis, CompoundCRS, Conversion, CoordinateOperation, CoordinateSystem,
    Crs, Datum, DatumEnsemble, Ellipsoid, EngineeringCRS, EngineeringDatum, GeodeticCRS,
    GeodeticReferenceFrame, GeographicCRS, Identified, Parameter, ParameterValue, PrimeMeridian,
    ProjectedCRS, Transformation, VerticalCRS, VerticalReferenceFrame,
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

    /// `isEquivalentTo` with a database context, as `proj_is_equivalent_to_with_ctx`.
    ///
    /// Beyond the structural check, a datum name that is an alias of the other datum's
    /// official name in `proj.db` counts as the same name (`GDA2020` and
    /// `Geocentric Datum of Australia 2020`). The strict criterion keeps names exact.
    #[must_use]
    pub fn is_equivalent_to_with_database(
        &self,
        other: &Self,
        criterion: Equivalence,
        db: &super::factory::DatabaseContext,
    ) -> bool {
        if self.is_equivalent_to_with(other, criterion) {
            return true;
        }
        if criterion == Equivalence::Strict {
            return false;
        }
        let (Some(left), Some(right)) = (self.datum(), other.datum()) else {
            return false;
        };
        let official = |name: &str| db.geodetic_datum_equivalent(name).map(|found| found.0);
        let same_official = match (official(left.name()), official(right.name())) {
            (Some(left_official), Some(right_official)) => left_official == right_official,
            _ => false,
        };
        if !(same_official || frame_names_equivalent_by_db(left, right, db)) {
            return false;
        }
        let mut renamed = other.clone();
        if let Some(datum) = renamed.datum.as_mut() {
            datum.identified.name = left.name().to_owned();
        }
        self.is_equivalent_to_with(&renamed, criterion)
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
/// Projection pipelines print the conversion without requiring this match.
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
    frame_of(crs.datum(), crs.datum_ensemble())
}

/// The datum of a geographic or geocentric CRS, or its ensemble as a datum.
pub(crate) fn geodetic_frame(crs: &Crs) -> Option<GeodeticReferenceFrame> {
    match crs {
        Crs::Geographic(crs) => datum_frame(crs),
        Crs::Geodetic(crs) => frame_of(crs.datum(), crs.datum_ensemble()),
        _ => None,
    }
}

/// `isEquivalentTo` of the datum of `crs` with `frame`, under the `EQUIVALENT` name rule.
pub(crate) fn datum_equivalent_to_frame(crs: &Crs, frame: &GeodeticReferenceFrame) -> bool {
    geodetic_frame(crs)
        .is_some_and(|left| frame_equivalent_by(&left, frame, Equivalence::Equivalent))
}

/// `isSameGeodeticDatum`: the datums of two geodetic CRS, or their ensembles as datums, are
/// equivalent. An `unknown` datum matches only another `unknown` datum.
pub(crate) fn same_geodetic_datum(left: &Crs, right: &Crs) -> bool {
    let (Some(left), Some(right)) = (geodetic_frame(left), geodetic_frame(right)) else {
        return false;
    };
    if (left.identified.name == "unknown") != (right.identified.name == "unknown") {
        return false;
    }
    frame_equivalent(&left, &right)
}

/// `isSameGeodeticDatum` with the database (coordinateoperationfactory.cpp:2056-2066): the
/// datums of two geodetic CRS are equivalent under the `EQUIVALENT` criterion, where a datum
/// name may be an alias of the official name of the datum identifier, or of the other datum's.
/// The names are compared as `IdentifiedObject::_isEquivalentTo` does, then the prime meridian and
/// the ellipsoid (`GeodeticReferenceFrame::isEquivalentToNoExactTypeCheck`, datum.cpp:1572).
pub(crate) fn same_geodetic_datum_with_database(
    left: &Crs,
    right: &Crs,
    db: &super::factory::DatabaseContext,
) -> bool {
    let (Some(left), Some(right)) = (geodetic_frame(left), geodetic_frame(right)) else {
        return false;
    };
    if (left.identified.name == "unknown") != (right.identified.name == "unknown") {
        return false;
    }
    let names = equivalent_name(left.name(), right.name())
        || frame_names_equivalent_by_db(&left, &right, db);
    names
        && ellipsoid_equivalent_tolerant(left.ellipsoid(), right.ellipsoid())
        && meridian_equivalent(left.prime_meridian(), right.prime_meridian())
}

fn frame_of(
    datum: Option<&GeodeticReferenceFrame>,
    ensemble: Option<&DatumEnsemble>,
) -> Option<GeodeticReferenceFrame> {
    if let Some(datum) = datum {
        return Some(datum.clone());
    }
    let ensemble = ensemble?;
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

/// `GeodeticReferenceFrame::hasEquivalentNameToUsingAlias` (datum.cpp:1602): the two datum names
/// are one name through the official name or an alias of the authority identifier of `frame`.
fn names_equivalent_by_identifier(
    frame: &GeodeticReferenceFrame,
    this_name: &str,
    other_name: &str,
    db: &super::factory::DatabaseContext,
) -> bool {
    let Some(identifier) = frame.identifiers().first() else {
        return false;
    };
    let official = db.object_name("geodetic_datum", identifier.code_space(), identifier.code());
    let aliases = db.get_aliases(
        identifier.code_space(),
        identifier.code(),
        this_name,
        "geodetic_datum",
        "",
    );
    let matches = |name: &str| {
        equivalent_name_bigger_differences(name, &official)
            || aliases
                .iter()
                .any(|alias| equivalent_name_bigger_differences(name, alias))
    };
    matches(this_name) && matches(other_name)
}

/// `GeodeticReferenceFrame::hasEquivalentNameToUsingAlias` (datum.cpp:1602-1700): the names of
/// two datums are one name through the official name or an alias of the identifier of either
/// datum, or through an alias without identifier. An ESRI `D_` prefix is dropped first.
fn frame_names_equivalent_by_db(
    this: &GeodeticReferenceFrame,
    other: &GeodeticReferenceFrame,
    db: &super::factory::DatabaseContext,
) -> bool {
    let compare = |this_name: &str, other_name: &str| {
        if this_name == other_name || this_name == "unknown" || other_name == "unknown" {
            return true;
        }
        if starts_with_ci(this_name, "Unknown based on ")
            || starts_with_ci(other_name, "Unknown based on ")
        {
            return false;
        }
        (!this.identifiers().is_empty()
            && names_equivalent_by_identifier(this, this_name, other_name, db))
            || (!other.identifiers().is_empty()
                && names_equivalent_by_identifier(other, other_name, this_name, db))
            || alias_names_equivalent(this_name, other_name, db)
            || alias_names_equivalent(other_name, this_name, db)
    };
    let this_prefixed = starts_with_ci(this.name(), "D_");
    let other_prefixed = starts_with_ci(other.name(), "D_");
    if this_prefixed && !other_prefixed {
        let stripped = &this.name()[2..];
        equivalent_name_bigger_differences(stripped, other.name())
            || compare(stripped, other.name())
    } else if !this_prefixed && other_prefixed {
        let stripped = &other.name()[2..];
        equivalent_name_bigger_differences(this.name(), stripped) || compare(this.name(), stripped)
    } else {
        compare(this.name(), other.name())
    }
}

/// `compareFromThisName` (datum.cpp:1673): `other_name` is an alias of the datum `this_name`.
fn alias_names_equivalent(
    this_name: &str,
    other_name: &str,
    db: &super::factory::DatabaseContext,
) -> bool {
    db.get_aliases("", "", this_name, "geodetic_datum", "")
        .iter()
        .any(|alias| equivalent_name_bigger_differences(other_name, alias))
}

/// ASCII case-insensitive `starts_with`, as `ci_starts_with`.
fn starts_with_ci(text: &str, prefix: &str) -> bool {
    text.len() >= prefix.len() && text[..prefix.len()].eq_ignore_ascii_case(prefix)
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

    /// `isEquivalentTo` with `criterion`. The name, the conversion name, and the
    /// axis names are compared for the strict criterion only. The base CRS and
    /// the conversion method and parameters follow `criterion`.
    #[must_use]
    pub fn is_equivalent_to_with(&self, other: &Self, criterion: Equivalence) -> bool {
        if criterion != Equivalence::Strict {
            if let (Some(left), Some(right)) = (
                self.identified.extension_proj4.as_deref(),
                other.identified.extension_proj4.as_deref(),
            ) {
                return proj_extensions_equivalent(left, right);
            }
        }
        (criterion != Equivalence::Strict || same_name(self.name(), other.name()))
            && geodetic_equivalent(self.base_crs(), other.base_crs(), criterion)
            && conversion_equivalent_for(
                self.deriving_conversion(),
                other.deriving_conversion(),
                criterion,
            )
            && cs_axes_match_for(
                self.coordinate_system().axes(),
                other.coordinate_system().axes(),
                criterion,
            )
    }
}

/// `VerticalCRS::_isEquivalentTo` with EQUIVALENT: the datum anchor and the axis, the name
/// not compared.
pub(crate) fn vertical_crs_equivalent(left: &VerticalCRS, right: &VerticalCRS) -> bool {
    vertical_anchor(left, right)
        && cs_axes_match(
            std::slice::from_ref(left.coordinate_system().axis()),
            std::slice::from_ref(right.coordinate_system().axis()),
        )
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

    /// `VerticalCRS::_isEquivalentTo` (crs.cpp, `SingleCRS::baseIsEquivalentTo`) under a criterion
    /// other than `Strict`: the names are compared as `Identifier::isEquivalentName` does, the
    /// datum (or ensemble) must match, and the axis is compared by its direction and unit only,
    /// as `CoordinateSystemAxis::_isEquivalentTo` does outside the strict criterion.
    #[must_use]
    pub fn is_equivalent_to_with(&self, other: &Self, criterion: Equivalence) -> bool {
        if criterion == Equivalence::Strict {
            return self.is_equivalent_to(other);
        }
        equivalent_name(self.name(), other.name())
            && vertical_anchor(self, other)
            && axes_direction_and_unit(
                &CoordinateSystem::Vertical(self.coordinate_system().clone()),
                &CoordinateSystem::Vertical(other.coordinate_system().clone()),
            )
    }
}

impl EngineeringCRS {
    /// Strict engineering-CRS equivalence.
    #[must_use]
    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        self.is_equivalent_to_with(other, Equivalence::Strict)
    }

    /// `isEquivalentTo` with `criterion`.
    ///
    /// `Equivalent` treats an empty datum name and `"Unknown engineering datum"` as the
    /// same datum. A Cartesian pair whose directions are all `unspecified` and whose
    /// names are `X`/`Y` matches easting/northing (`EngineeringCRS::_isEquivalentTo`).
    #[must_use]
    pub fn is_equivalent_to_with(&self, other: &Self, criterion: Equivalence) -> bool {
        if criterion == Equivalence::Strict {
            return same_name(self.name(), other.name())
                && self.datum().is_equivalent_to(other.datum())
                && coordinate_system_equivalent(
                    self.coordinate_system(),
                    other.coordinate_system(),
                );
        }
        engineering_datum_equivalent(self.datum(), other.datum())
            && (axes_direction_and_unit(self.coordinate_system(), other.coordinate_system())
                || cartesian_xy_versus_en(self.coordinate_system(), other.coordinate_system()))
    }
}

impl ProjectedCRS {
    /// `isEquivalentTo` with `criterion` and a database. The base CRS, when geographic, may
    /// name its datum by an alias of the other datum's official name in `proj.db`.
    #[must_use]
    pub fn is_equivalent_to_with_database(
        &self,
        other: &Self,
        criterion: Equivalence,
        db: &super::factory::DatabaseContext,
    ) -> bool {
        if self.is_equivalent_to_with(other, criterion) {
            return true;
        }
        if criterion == Equivalence::Strict {
            return false;
        }
        let (GeodeticCRS::Geographic(left), GeodeticCRS::Geographic(right)) =
            (self.base_crs(), other.base_crs())
        else {
            return false;
        };
        left.is_equivalent_to_with_database(right, criterion, db)
            && conversion_equivalent_for(
                self.deriving_conversion(),
                other.deriving_conversion(),
                criterion,
            )
            && cs_axes_match_for(
                self.coordinate_system().axes(),
                other.coordinate_system().axes(),
                criterion,
            )
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

    /// `isEquivalentTo` with `criterion`. Under a criterion other than `Strict` the CRS name
    /// is not compared: the components are compared with the same criterion, in order.
    #[must_use]
    pub fn is_equivalent_to_with(&self, other: &Self, criterion: Equivalence) -> bool {
        if criterion == Equivalence::Strict {
            return self.is_equivalent_to(other);
        }
        self.components().len() == other.components().len()
            && self
                .components()
                .iter()
                .zip(other.components())
                .all(|(left, right)| left.is_equivalent_to_with(right, criterion))
    }
}

impl Crs {
    /// `isEquivalentTo` with `criterion`.
    ///
    /// Engineering CRS use the `Equivalent` datum and axis exceptions. Every other kind
    /// uses the strict comparison.
    #[must_use]
    pub fn is_equivalent_to_with(&self, other: &Self, criterion: Equivalence) -> bool {
        match (self, other) {
            (Self::Engineering(left), Self::Engineering(right)) => {
                left.is_equivalent_to_with(right, criterion)
            }
            (Self::Projected(left), Self::Projected(right)) => {
                left.is_equivalent_to_with(right, criterion)
            }
            (Self::Vertical(left), Self::Vertical(right)) if criterion != Equivalence::Strict => {
                left.is_equivalent_to_with(right, criterion)
            }
            // `GeographicCRS::_isEquivalentTo` (crs.cpp:3239): under a criterion other than
            // `Strict`, the name of the CRS and the names of its axes are not compared, only the
            // datum (or ensemble) and the axes' directions and units.
            (Self::Geographic(left), Self::Geographic(right))
                if criterion != Equivalence::Strict =>
            {
                left.is_equivalent_to_with(right, criterion)
            }
            (Self::Geodetic(GeodeticCRS::Geographic(left)), Self::Geographic(right))
            | (Self::Geographic(left), Self::Geodetic(GeodeticCRS::Geographic(right)))
                if criterion != Equivalence::Strict =>
            {
                left.is_equivalent_to_with(right, criterion)
            }
            // Under a criterion other than `Strict`, the name of a compound CRS is not compared.
            (Self::Compound(left), Self::Compound(right)) if criterion != Equivalence::Strict => {
                left.is_equivalent_to_with(right, criterion)
            }
            _ => self.is_equivalent_to(other),
        }
    }

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
            (Self::DerivedProjected(left), Self::DerivedProjected(right)) => {
                same_name(left.name(), right.name())
                    && left.base_crs().is_equivalent_to(right.base_crs())
                    && conversion_equivalent(
                        left.deriving_conversion(),
                        right.deriving_conversion(),
                    )
                    && coordinate_system_equivalent(
                        left.coordinate_system(),
                        right.coordinate_system(),
                    )
            }
            (Self::Temporal(left), Self::Temporal(right)) => left.is_equivalent_to(right),
            (Self::Parametric(left), Self::Parametric(right)) => left.is_equivalent_to(right),
            (Self::Derived(left), Self::Derived(right)) => left.is_equivalent_to(right),
            (Self::Bound(left), Self::Bound(right)) => {
                same_name(left.name(), right.name())
                    && left.source_crs().is_equivalent_to(right.source_crs())
                    && left.target_crs().is_equivalent_to(right.target_crs())
                    && operations_equivalent(left.transformation(), right.transformation())
            }
            _ => false,
        }
    }
}

fn operations_equivalent(left: &CoordinateOperation, right: &CoordinateOperation) -> bool {
    match (left, right) {
        (CoordinateOperation::Transformation(left), CoordinateOperation::Transformation(right)) => {
            transformation_equivalent(left, right)
        }
        (CoordinateOperation::Conversion(left), CoordinateOperation::Conversion(right)) => {
            left.is_equivalent_to(right)
        }
        (CoordinateOperation::Concatenated(left), CoordinateOperation::Concatenated(right)) => {
            same_name(left.name(), right.name())
                && left.steps().len() == right.steps().len()
                && left
                    .steps()
                    .iter()
                    .zip(right.steps())
                    .all(|(left, right)| operations_equivalent(left.operation(), right.operation()))
        }
        _ => false,
    }
}

/// `SingleOperation::_isEquivalentTo` (singleoperation.cpp:1483-1750): the name, the method and
/// the parameter values are compared, the source and target CRS are not.
fn transformation_equivalent(left: &Transformation, right: &Transformation) -> bool {
    same_name(left.name(), right.name())
        && left.method().code() == right.method().code()
        && left.parameters().len() == right.parameters().len()
        && left
            .parameters()
            .iter()
            .zip(right.parameters())
            .all(|(left, right)| parameter_equivalent(left, right))
}

fn geographic_equivalent(
    left: &GeographicCRS,
    right: &GeographicCRS,
    criterion: Equivalence,
) -> bool {
    if criterion == Equivalence::Strict && !same_name(left.name(), right.name()) {
        return false;
    }
    if geographic_anchor(left, right, criterion)
        && cs_axes_match_for(
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
    geographic_anchor(left, right, criterion)
        && cs_axes_match_for(left.coordinate_system().axes(), &swapped, criterion)
}

fn geographic_anchor(left: &GeographicCRS, right: &GeographicCRS, criterion: Equivalence) -> bool {
    if criterion != Equivalence::Strict {
        return match (datum_frame(left), datum_frame(right)) {
            (Some(left), Some(right)) => {
                frame_equivalent_by(&left, &right, Equivalence::Equivalent)
            }
            _ => false,
        };
    }
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
                    (Some(left), Some(right), _, _) => frame_equivalent_by(left, right, criterion),
                    (None, None, Some(left), Some(right)) => ensemble_equivalent(left, right),
                    _ => false,
                }
                && cs_axes_match_for(
                    left.coordinate_system().axes(),
                    right.coordinate_system().axes(),
                    criterion,
                )
        }
        (GeodeticCRS::Spherical(left), GeodeticCRS::Spherical(right)) => {
            (criterion != Equivalence::Strict
                || same_name(&left.identified.name, &right.identified.name))
                && match (
                    left.datum.as_ref(),
                    right.datum.as_ref(),
                    left.ensemble.as_ref(),
                    right.ensemble.as_ref(),
                ) {
                    (Some(left), Some(right), _, _) => frame_equivalent_by(left, right, criterion),
                    (None, None, Some(left), Some(right)) => ensemble_equivalent(left, right),
                    _ => false,
                }
                && cs_axes_match_for(
                    &left.coordinate_system.axes,
                    &right.coordinate_system.axes,
                    criterion,
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
    frame_equivalent_by(left, right, Equivalence::Strict)
}

/// Datum comparison for `criterion`. The strict criterion compares names ignoring case and
/// the ellipsoid exactly. The other criteria compare the names with `isEquivalentName` and the
/// ellipsoid to a relative `1e-10` (`Ellipsoid::_isEquivalentTo`).
fn frame_equivalent_by(
    left: &GeodeticReferenceFrame,
    right: &GeodeticReferenceFrame,
    criterion: Equivalence,
) -> bool {
    let strict = criterion == Equivalence::Strict;
    let names: fn(&str, &str) -> bool = if strict { same_name } else { equivalent_name };
    let ellipsoids_match = if strict {
        ellipsoid_equivalent_by(left.ellipsoid(), right.ellipsoid(), same_name)
    } else {
        ellipsoid_equivalent_tolerant(left.ellipsoid(), right.ellipsoid())
    };
    (names(left.name(), right.name()) || shares_epsg_code(left, right))
        && same_epoch(left.frame_reference_epoch(), right.frame_reference_epoch())
        && ellipsoids_match
        && meridian_equivalent(left.prime_meridian(), right.prime_meridian())
}

/// `Ellipsoid::_isEquivalentTo` with a non-strict criterion: the semi-major axis, the semi-minor
/// axis (computed when absent), and the inverse flattening when both have one, each to a
/// relative `1e-10`. The names are not compared.
fn ellipsoid_equivalent_tolerant(left: &Ellipsoid, right: &Ellipsoid) -> bool {
    let close = |a: f64, b: f64| (a - b).abs() <= 1e-10 * a.abs();
    let semi_minor = |ellipsoid: &Ellipsoid| {
        ellipsoid.semi_minor_axis().map_or_else(
            || {
                let flattening = ellipsoid.inverse_flattening();
                let major = ellipsoid.semi_major_axis().si_value();
                if flattening == 0.0 {
                    major
                } else {
                    major * (1.0 - 1.0 / flattening)
                }
            },
            |minor| minor.si_value(),
        )
    };
    close(
        left.semi_major_axis().si_value(),
        right.semi_major_axis().si_value(),
    ) && close(semi_minor(left), semi_minor(right))
        && match (
            left.declared_inverse_flattening(),
            right.declared_inverse_flattening(),
        ) {
            (Some(left), Some(right)) => close(left, right),
            _ => true,
        }
}

/// Two datums with the same EPSG code are the same datum, whatever their names.
fn shares_epsg_code(left: &GeodeticReferenceFrame, right: &GeodeticReferenceFrame) -> bool {
    left.identifiers().iter().any(|a| {
        a.code_space() == "EPSG"
            && right
                .identifiers()
                .iter()
                .any(|b| b.code_space() == "EPSG" && b.code() == a.code())
    })
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
    ellipsoid_equivalent_by(left, right, same_name)
}

/// Ellipsoid comparison with the `EQUIVALENT` name rule.
pub(crate) fn ellipsoid_equivalent_loose(left: &Ellipsoid, right: &Ellipsoid) -> bool {
    ellipsoid_equivalent_by(left, right, equivalent_name)
}

fn ellipsoid_equivalent_by(
    left: &Ellipsoid,
    right: &Ellipsoid,
    names: fn(&str, &str) -> bool,
) -> bool {
    names(left.name(), right.name())
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
        && (left.longitude().si_value() - right.longitude().si_value()).abs() <= 1e-12
}

/// Empty and `"Unknown engineering datum"` match (`EngineeringDatum::_isEquivalentTo`).
fn engineering_datum_equivalent(left: &EngineeringDatum, right: &EngineeringDatum) -> bool {
    let unknown = |name: &str| name.is_empty() || name == "Unknown engineering datum";
    if unknown(left.name()) && unknown(right.name()) {
        return true;
    }
    same_name(left.name(), right.name())
}

/// Directions and conversion factors match. Axis names do not.
fn axes_direction_and_unit(left: &CoordinateSystem, right: &CoordinateSystem) -> bool {
    let left_axes = left.axes();
    let right_axes = right.axes();
    std::mem::discriminant(left) == std::mem::discriminant(right)
        && left_axes.len() == right_axes.len()
        && left_axes.iter().zip(right_axes).all(|(left, right)| {
            let (left_si, right_si) = (
                left.unit().conversion_to_si(),
                right.unit().conversion_to_si(),
            );
            left.direction() == right.direction()
                && (left_si - right_si).abs() <= 1e-10 * left_si.abs()
        })
}

/// `X`/`Y` of unspecified direction against easting/northing, either way around.
fn cartesian_xy_versus_en(left: &CoordinateSystem, right: &CoordinateSystem) -> bool {
    let (CoordinateSystem::Cartesian(left), CoordinateSystem::Cartesian(right)) = (left, right)
    else {
        return false;
    };
    let left = left.axes();
    let right = right.axes();
    if left.len() != 2 || right.len() != 2 {
        return false;
    }
    let unspecified = |axes: &[Axis]| axes.iter().all(|axis| axis.direction() == "unspecified");
    if !unspecified(left) && !unspecified(right) {
        return false;
    }
    (left[0].name() == "X"
        && right[0].name() == "Easting"
        && left[1].name() == "Y"
        && right[1].name() == "Northing")
        || (right[0].name() == "X"
            && left[0].name() == "Easting"
            && right[1].name() == "Y"
            && left[1].name() == "Northing")
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

/// Axes match for `criterion`. A strict check also compares name, abbreviation,
/// and meridian. The other criteria compare direction and unit only, as
/// PROJ's `CoordinateSystemAxis::_isEquivalentTo` does.
fn cs_axes_match_for(left: &[Axis], right: &[Axis], criterion: Equivalence) -> bool {
    if criterion == Equivalence::Strict {
        return cs_axes_match(left, right);
    }
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            let (left_si, right_si) = (
                left.unit().conversion_to_si(),
                right.unit().conversion_to_si(),
            );
            left.direction() == right.direction()
                && (left_si - right_si).abs() <= 1e-10 * left_si.abs()
        })
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
    conversion_equivalent_for(left, right, Equivalence::Strict)
}

/// A conversion with the same method and parameters. Its name must match only
/// for the strict criterion.
fn conversion_equivalent_for(
    left: &Conversion,
    right: &Conversion,
    criterion: Equivalence,
) -> bool {
    if criterion != Equivalence::Strict
        && left.method().code() != right.method().code()
        && spherical_method_pair(left.method().code(), right.method().code())
    {
        return spherical_pair_parameters_match(left, right);
    }
    if criterion != Equivalence::Strict {
        return left.is_equivalent_to_with(right, criterion);
    }
    same_name(left.name(), right.name())
        && left.method().code() == right.method().code()
        && left.parameters().len() == right.parameters().len()
        && left
            .parameters()
            .iter()
            .zip(right.parameters())
            .all(|(left, right)| parameter_equivalent(left, right))
}

/// The EPSG methods PROJ treats as the same projection on a sphere: Lambert
/// azimuthal equal area (1027, 9820) and equidistant cylindrical (1029, 1028).
fn spherical_method_pair(left: &str, right: &str) -> bool {
    matches!(
        (left, right),
        ("1027", "9820") | ("9820", "1027") | ("1029", "1028") | ("1028", "1029")
    )
}

/// Parameters of a spherical/ellipsoidal method pair, matched by code. A
/// parameter the other method lacks must be zero.
fn spherical_pair_parameters_match(left: &Conversion, right: &Conversion) -> bool {
    let covered = |from: &Conversion, other: &Conversion| {
        from.parameters().iter().all(|parameter| {
            match other
                .parameters()
                .iter()
                .find(|candidate| candidate.code() == parameter.code())
            {
                Some(found) => parameter_value_equivalent(parameter, found),
                None => matches!(
                    parameter.value(),
                    ParameterValue::Measure(measure) if measure.value() == 0.0
                ),
            }
        })
    };
    covered(left, right) && covered(right, left)
}

fn parameter_value_equivalent(left: &Parameter, right: &Parameter) -> bool {
    match (left.value(), right.value()) {
        (ParameterValue::Measure(left), ParameterValue::Measure(right)) => {
            left.value() == right.value() && left.unit().name() == right.unit().name()
        }
        _ => false,
    }
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
/// `/`, parentheses, `.`, `&`, and `,`, and writing accented letters as ASCII.
pub fn equivalent_name(left: &str, right: &str) -> bool {
    folded_name(left) == folded_name(right)
}

/// `areEquivalentParameters`: names PROJ treats as one parameter, such as
/// `latitude_of_origin` and `latitude_of_center`.
pub(crate) fn are_equivalent_parameters(left: &str, right: &str) -> bool {
    let left = folded_name(left);
    let right = folded_name(right);
    if left == right {
        return false;
    }
    const GROUPS: &[&[&str]] = &[
        &["latitude_of_point_1", "Latitude_Of_1st_Point"],
        &["longitude_of_point_1", "Longitude_Of_1st_Point"],
        &["latitude_of_point_2", "Latitude_Of_2nd_Point"],
        &["longitude_of_point_2", "Longitude_Of_2nd_Point"],
        &["satellite_height", "height"],
        &[
            "False easting",
            "Easting at false origin",
            "Easting at projection centre",
        ],
        &[
            "False northing",
            "Northing at false origin",
            "Northing at projection centre",
        ],
        &[
            "Scale factor at natural origin",
            "scale_factor",
            "Scale factor on initial line",
            "Scale factor at projection centre",
            "Scale factor on pseudo standard parallel",
        ],
        &[
            "latitude_of_origin",
            "latitude_of_center",
            "Latitude of natural origin",
            "Latitude of false origin",
            "Latitude of projection centre",
            "Central_Parallel",
        ],
        &[
            "central_meridian",
            "longitude_of_center",
            "Longitude of natural origin",
            "Longitude of false origin",
            "Longitude of projection centre",
            "Longitude of origin",
        ],
        &["Azimuth of initial line", "Azimuth at projection centre"],
        &["pseudo_standard_parallel_1", "standard_parallel_1"],
    ];
    GROUPS.iter().any(|group| {
        let in_group = |name: &str| group.iter().any(|entry| folded_name(entry) == name);
        in_group(&left) && in_group(&right)
    })
}

fn folded_name(name: &str) -> String {
    name.chars()
        .filter(|ch| !matches!(ch, ' ' | '_' | '-' | '/' | '(' | ')' | '.' | '&' | ','))
        .flat_map(char::to_lowercase)
        .map(|ch| accent_ascii(ch).unwrap_or(ch))
        .collect()
}

/// `Identifier::isEquivalentName(a, b)` with its default
/// `biggerDifferencesAllowed`: on top of [`equivalent_name`], the words
/// `zone` and `height` after a separator are ignored, `north` and `south`
/// after a separator or a digit are shortened to their first letter, and
/// `19dd` is read as `dd`. `EGM2008 height` is equivalent to `EGM2008`.
pub fn equivalent_name_bigger_differences(left: &str, right: &str) -> bool {
    let a: Vec<char> = left.chars().collect();
    let b: Vec<char> = right.chars().collect();
    let at = |text: &[char], idx: usize| text.get(idx).copied().unwrap_or('\0');
    let (mut i, mut j) = (0usize, 0usize);
    let (mut last_a, mut last_b) = ('\0', '\0');
    while i < a.len() || j < b.len() {
        let mut ca = lower_ascii(at(&a, i));
        let mut cb = lower_ascii(at(&b, j));
        if ca == ' ' && at(&a, i + 1) == '+' && at(&a, i + 2) == ' ' && at(&a, i + 3) != '\0' {
            i += 3;
            continue;
        }
        if cb == ' ' && at(&b, j + 1) == '+' && at(&b, j + 2) == ' ' && at(&b, j + 3) != '\0' {
            j += 3;
            continue;
        }
        if matches_at(&a, i, "_IntlFeet")
            && at(&a, i + 9) == '\0'
            && matches_at(&b, j, "_Feet")
            && at(&b, j + 5) == '\0'
        {
            return true;
        }
        if matches_at(&a, i, "_Feet")
            && at(&a, i + 5) == '\0'
            && matches_at(&b, j, "_IntlFeet")
            && at(&b, j + 9) == '\0'
        {
            return true;
        }
        if is_ignored_char(ca) {
            i += 1;
            continue;
        }
        if is_ignored_char(cb) {
            j += 1;
            continue;
        }
        if ca == '1'
            && !last_a.is_ascii_digit()
            && at(&a, i + 1) == '9'
            && is_digit_at(&a, i + 2)
            && is_digit_at(&a, i + 3)
        {
            i += 2;
            last_a = '9';
            continue;
        }
        if cb == '1'
            && !last_b.is_ascii_digit()
            && at(&b, j + 1) == '9'
            && is_digit_at(&b, j + 2)
            && is_digit_at(&b, j + 3)
        {
            j += 2;
            last_b = '9';
            continue;
        }
        let zone_a = skip_word(ca, &a, &mut i, "zone");
        let zone_b = skip_word(cb, &b, &mut j, "zone");
        if zone_a || zone_b {
            continue;
        }
        let height_a = skip_word(ca, &a, &mut i, "height");
        let height_b = skip_word(cb, &b, &mut j, "height");
        if height_a || height_b {
            continue;
        }
        if !shorten_word(ca, &a, &mut i, "north") {
            shorten_word(ca, &a, &mut i, "south");
        }
        if !shorten_word(cb, &b, &mut j, "north") {
            shorten_word(cb, &b, &mut j, "south");
        }
        if let Some(replacement) = accent_ascii(ca) {
            ca = replacement;
        }
        if let Some(replacement) = accent_ascii(cb) {
            cb = replacement;
        }
        if ca != cb {
            return false;
        }
        last_a = ca;
        last_b = cb;
        if ca != '\0' {
            i += 1;
        }
        if cb != '\0' {
            j += 1;
        }
    }
    true
}

fn lower_ascii(ch: char) -> char {
    ch.to_ascii_lowercase()
}

fn is_ignored_char(ch: char) -> bool {
    matches!(ch, ' ' | '_' | '-' | '/' | '(' | ')' | '.' | '&' | ',')
}

fn is_digit_at(text: &[char], idx: usize) -> bool {
    text.get(idx).is_some_and(char::is_ascii_digit)
}

/// Case-insensitive match of `needle` at `idx`.
fn matches_at(text: &[char], idx: usize, needle: &str) -> bool {
    needle.chars().enumerate().all(|(offset, ch)| {
        text.get(idx + offset)
            .is_some_and(|c| lower_ascii(*c) == lower_ascii(ch))
    })
}

/// `zone` or `height` after a separator: skip the word, returning true.
fn skip_word(ch: char, text: &[char], idx: &mut usize, word: &str) -> bool {
    let start = *idx;
    if ch == word.chars().next().unwrap_or('\0')
        && preceded_by(text, start, is_ignored_char)
        && matches_at(text, start, word)
    {
        *idx += word.len();
        return true;
    }
    false
}

fn preceded_by(text: &[char], idx: usize, test: fn(char) -> bool) -> bool {
    idx > 0 && text.get(idx - 1).is_some_and(|ch| test(*ch))
}

/// `north` or `south` after a separator or a digit: keep the first letter only.
fn shorten_word(ch: char, text: &[char], idx: &mut usize, word: &str) -> bool {
    let start = *idx;
    if ch == word.chars().next().unwrap_or('\0')
        && (preceded_by(text, start, is_ignored_char)
            || preceded_by(text, start, |c| c.is_ascii_digit()))
        && matches_at(text, start, word)
    {
        *idx += word.len() - 1;
        return true;
    }
    false
}

/// The ASCII letter that PROJ's `isEquivalentName` writes for an accented one.
fn accent_ascii(ch: char) -> Option<char> {
    Some(match ch {
        'á' | 'ä' => 'a',
        'ě' | 'è' | 'é' | 'ë' => 'e',
        'í' => 'i',
        'ô' | 'ö' => 'o',
        'ç' => 'c',
        _ => return None,
    })
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
                if name_equal {
                    100
                } else {
                    90
                }
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
    /// CRS uses the hardcoded UTM zones. Other kinds have no candidate.
    #[must_use]
    pub fn identify(&self) -> Vec<(Crs, i32)> {
        match self {
            Self::Geographic(crs) => {
                Self::Geodetic(GeodeticCRS::Geographic(crs.clone())).identify()
            }
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
            geographic_anchor(left, right, Equivalence::Strict)
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
    let flattening = left.inverse_flattening();
    let other = right.inverse_flattening();
    // `Ellipsoid::_isEquivalentTo` (datum.cpp:1112): "Clarke 1880 mod." is compared laxly.
    if left.name() == "Clarke 1880 mod." || right.name() == "Clarke 1880 mod." {
        return (major - other_major).abs() < 1e-8 * major.abs()
            && (flattening - other).abs() < 1e-5 * flattening.abs();
    }
    if (major - other_major).abs() > 1e-10 * major.abs() {
        return false;
    }
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
        && (left.unit().conversion_to_si() - right.unit().conversion_to_si()).abs() <= 1e-10 * scale
}

pub(crate) fn same_epoch(left: Option<f64>, right: Option<f64>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left == right,
        (None, None) => true,
        _ => false,
    }
}

/// Two PROJ strings of projected CRS are equivalent when they hold the same options, in any
/// order: the PROJ extension decides, as `ProjectedCRS::_isEquivalentTo` does for EQUIVALENT.
fn proj_extensions_equivalent(left: &str, right: &str) -> bool {
    let options = |text: &str| {
        let mut options: Vec<String> = text
            .split_whitespace()
            .map(|token| token.trim_start_matches('+').to_owned())
            .collect();
        options.sort();
        options
    };
    options(left) == options(right)
}

#[cfg(test)]
mod bigger_differences_tests {
    use super::equivalent_name_bigger_differences;

    #[test]
    fn height_and_zone_words_are_ignored() {
        assert!(equivalent_name_bigger_differences(
            "EGM2008 height",
            "EGM2008"
        ));
        assert!(equivalent_name_bigger_differences(
            "WGS 84 / UTM zone 31N",
            "WGS 84 / UTM 31N"
        ));
    }

    #[test]
    fn other_words_still_differ() {
        assert!(!equivalent_name_bigger_differences(
            "WGS 84 + EGM2008 height",
            "EGM2008"
        ));
        assert!(!equivalent_name_bigger_differences("EGM2008", "EGM2096"));
    }
}
