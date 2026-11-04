//! CRS edits from `crs.cpp`: promote and demote, linear-unit changes, and
//! `extractGeographicCRS` / `extractVerticalCRS`.

use super::bound_import::bind_towgs84;
use super::factory::DatabaseContext;
use super::model::{
    Axis, BoundCRS, CartesianCS, CompoundCRS, Conversion, CoordinateOperation, CoordinateSystem,
    Crs, DatumEnsemble, DerivedProjectedCRS, FactoryError, GeodeticCRS, GeodeticReferenceFrame,
    GeographicCRS, Identified, Parameter, ParameterValue, ProjectedCRS, UnitKind, UnitOfMeasure,
    VerticalCRS,
};
use super::niche::{DerivedCRS, DerivedKind};

/// How [`Conversion::alter_parameters_linear_unit`] applies a linear unit.
///
/// `Relabel` keeps the number and changes the unit. `Convert` divides the
/// number by the new unit's factor to the metre.
#[derive(Clone, Debug)]
pub enum ParameterLinearUnit {
    /// Keep the numeric value and attach this unit.
    Relabel(UnitOfMeasure),
    /// Convert the numeric value into this unit.
    Convert(UnitOfMeasure),
}

/// Whether `isDynamic` treats the WGS 84 datum and ensemble as dynamic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wgs84Dynamic {
    /// WGS 84 is static. This is `isDynamic(false)`.
    Static,
    /// A datum or ensemble named World Geodetic System 1984 is dynamic.
    TreatAsDynamic,
}

impl Conversion {
    /// `Conversion::alterParametersLinearUnit`.
    ///
    /// Linear measures take `change`. A conversion with no such measure is cloned.
    #[must_use]
    pub fn alter_parameters_linear_unit(&self, change: ParameterLinearUnit) -> Self {
        let mut parameters = Vec::with_capacity(self.parameters.len());
        let mut changed = false;
        for parameter in &self.parameters {
            if let Some(updated) = relabel_linear(parameter, &change) {
                parameters.push(updated);
                changed = true;
            } else {
                parameters.push(parameter.clone());
            }
        }
        if !changed {
            return self.clone();
        }
        let mut conversion = self.clone();
        conversion.parameters = parameters;
        conversion.identified = Identified::named("unknown");
        conversion
    }
}

impl ProjectedCRS {
    /// `ProjectedCRS::alterParametersLinearUnit`.
    #[must_use]
    pub fn alter_parameters_linear_unit(&self, change: ParameterLinearUnit) -> Self {
        let mut crs = self.clone();
        crs.conversion = self.conversion.alter_parameters_linear_unit(change);
        crs.identified.identifiers.clear();
        crs
    }
}

impl DerivedProjectedCRS {
    /// `DerivedProjectedCRS::demoteTo2D`.
    ///
    /// Drops a third cartesian axis and demotes the projected base. A CRS that
    /// is not a 3D cartesian system is returned unchanged.
    #[must_use]
    pub fn demote_to_2d(&self, db: Option<&DatabaseContext>) -> Self {
        let CoordinateSystem::Cartesian(cs) = &self.coordinate_system else {
            return self.clone();
        };
        if cs.axes.len() != 3 {
            return self.clone();
        }
        let mut coordinate_system = cs.clone();
        coordinate_system.axes.truncate(2);
        Self {
            identified: Identified::named(self.name()),
            base: self.base.demote_to_2d(db),
            conversion: self.conversion.clone(),
            coordinate_system: CoordinateSystem::Cartesian(coordinate_system),
        }
    }
}

impl BoundCRS {
    /// `BoundCRS::createFromTOWGS84`.
    ///
    /// Three values are geocentric translations. Seven are a position vector
    /// transformation. The hub is WGS 84.
    ///
    /// # Errors
    ///
    /// `values` has neither 3 nor 7 elements, or `source` has no geodetic CRS.
    pub fn from_towgs84(source: Crs, values: &[f64]) -> Result<Self, FactoryError> {
        match bind_towgs84(source, values)? {
            Crs::Bound(bound) => Ok(bound),
            _ => Err(FactoryError::Invalid(
                "TOWGS84 did not build a bound CRS".to_owned(),
            )),
        }
    }

    /// `BoundCRS::baseCRSWithCanonicalBoundCRS`: a copy of the base CRS that remembers this
    /// bound CRS, so that `create_operations` starts from it.
    #[must_use]
    pub fn base_crs_with_canonical_bound_crs(&self) -> Crs {
        self.source
            .as_ref()
            .clone()
            .with_canonical_bound(self.clone())
    }
}

impl Crs {
    /// `CRS::isDynamic` with the WGS 84 flag.
    #[must_use]
    pub fn is_dynamic_with(&self, wgs84: Wgs84Dynamic) -> bool {
        if wgs84 == Wgs84Dynamic::TreatAsDynamic && named_wgs84(self) {
            return true;
        }
        self.is_dynamic()
    }

    /// `CRS::extractGeographicCRS`.
    #[must_use]
    pub fn extract_geographic_crs(&self) -> Option<GeographicCRS> {
        match self {
            Self::Geographic(crs) | Self::Geodetic(GeodeticCRS::Geographic(crs)) => {
                Some(crs.clone())
            }
            Self::Projected(crs) => match crs.base_crs() {
                GeodeticCRS::Geographic(crs) => Some(crs.clone()),
                _ => None,
            },
            Self::DerivedProjected(crs) => {
                Self::Projected(crs.base_crs().clone()).extract_geographic_crs()
            }
            Self::Derived(crs) if crs.kind() == DerivedKind::Geographic => {
                crs.base_crs().extract_geographic_crs()
            }
            Self::Compound(crs) => crs
                .components()
                .iter()
                .find_map(Self::extract_geographic_crs),
            Self::Bound(crs) => crs.source_crs().extract_geographic_crs(),
            _ => None,
        }
    }

    /// `CRS::extractVerticalCRS`.
    #[must_use]
    pub fn extract_vertical_crs(&self) -> Option<VerticalCRS> {
        match self {
            Self::Vertical(crs) => Some(crs.clone()),
            Self::Derived(crs) if crs.kind() == DerivedKind::Vertical => {
                crs.base_crs().extract_vertical_crs()
            }
            Self::Compound(crs) => crs.components().iter().find_map(Self::extract_vertical_crs),
            Self::Bound(crs) => crs.source_crs().extract_vertical_crs(),
            _ => None,
        }
    }

    /// `CRS::stripVerticalComponent`, which is `demoteTo2D` with no database.
    #[must_use]
    pub fn strip_vertical_component(&self) -> Self {
        self.demote_to_2d(None)
    }

    /// `CRS::alterGeodeticCRS`.
    ///
    /// A geodetic CRS is replaced. A projected CRS keeps its conversion and
    /// axes and takes `new_geodetic` as its base.
    #[must_use]
    pub fn alter_geodetic_crs(&self, new_geodetic: &GeodeticCRS) -> Self {
        match self {
            Self::Geographic(_) | Self::Geodetic(_) => Self::from(new_geodetic.clone()),
            Self::Projected(crs) => {
                let mut projected = crs.clone();
                projected.base = new_geodetic.clone();
                projected.identified.identifiers.clear();
                Self::Projected(projected)
            }
            Self::DerivedProjected(crs) => {
                let base = Self::Projected(crs.base.clone()).alter_geodetic_crs(new_geodetic);
                let Self::Projected(base) = base else {
                    return self.clone();
                };
                let mut derived = crs.clone();
                derived.base = base;
                derived.identified.identifiers.clear();
                Self::DerivedProjected(derived)
            }
            Self::Compound(crs) => {
                let components = crs
                    .components()
                    .iter()
                    .map(|component| component.alter_geodetic_crs(new_geodetic))
                    .collect();
                let Ok(mut created) = CompoundCRS::create(crs.name(), components) else {
                    return self.clone();
                };
                created.identified.deprecated = crs.identified.deprecated;
                Self::Compound(created)
            }
            _ => self.clone(),
        }
    }

    /// `CRS::alterCSAngularUnit`.
    ///
    /// Angular axes of a geographic CRS take `unit`, and its identifiers are cleared. Other
    /// kinds are returned unchanged.
    #[must_use]
    pub fn alter_cs_angular_unit(&self, unit: &UnitOfMeasure) -> Self {
        match self {
            Self::Geographic(crs) => Self::Geographic(geographic_angular_unit(crs, unit)),
            Self::Geodetic(GeodeticCRS::Geographic(crs)) => {
                Self::Geographic(geographic_angular_unit(crs, unit))
            }
            _ => self.clone(),
        }
    }

    /// `CRS::alterCSLinearUnit`.
    ///
    /// Cartesian axes, a geographic height axis, and a vertical axis take `unit`
    /// when they are linear. A parametric CRS is returned unchanged.
    #[must_use]
    pub fn alter_cs_linear_unit(&self, unit: &UnitOfMeasure) -> Self {
        match self {
            Self::Projected(crs) => {
                let mut projected = crs.clone();
                projected.coordinate_system =
                    cartesian_with_unit(&projected.coordinate_system, unit);
                projected.identified.identifiers.clear();
                Self::Projected(projected)
            }
            Self::Geodetic(GeodeticCRS::Geocentric(crs)) => {
                let mut geocentric = crs.clone();
                geocentric.coordinate_system =
                    cartesian_with_unit(&geocentric.coordinate_system, unit);
                geocentric.identified.identifiers.clear();
                Self::Geodetic(GeodeticCRS::Geocentric(geocentric))
            }
            Self::Geographic(crs) if crs.coordinate_system.axes.len() == 3 => {
                Self::Geographic(geographic_linear_unit(crs, unit))
            }
            Self::Geodetic(GeodeticCRS::Geographic(crs))
                if crs.coordinate_system.axes.len() == 3 =>
            {
                Self::Geographic(geographic_linear_unit(crs, unit))
            }
            Self::Vertical(crs) => {
                let mut vertical = crs.clone();
                replace_linear_axis(&mut vertical.coordinate_system.axis, unit);
                vertical.identified.identifiers.clear();
                Self::Vertical(vertical)
            }
            Self::Engineering(crs) => engineering_linear_unit(crs, unit),
            Self::DerivedProjected(crs) => {
                let mut derived = crs.clone();
                if let CoordinateSystem::Cartesian(cs) = &mut derived.coordinate_system {
                    *cs = cartesian_with_unit(cs, unit);
                }
                derived.identified.identifiers.clear();
                Self::DerivedProjected(derived)
            }
            Self::Compound(crs) => {
                let components = crs
                    .components()
                    .iter()
                    .map(|component| component.alter_cs_linear_unit(unit))
                    .collect();
                let Ok(mut created) = CompoundCRS::create(crs.name(), components) else {
                    return self.clone();
                };
                created.identified.deprecated = crs.identified.deprecated;
                Self::Compound(created)
            }
            Self::Bound(crs) => {
                let mut bound = crs.clone();
                bound.source = Box::new(crs.source.alter_cs_linear_unit(unit));
                Self::Bound(bound)
            }
            _ => self.clone(),
        }
    }

    /// `CRS::promoteTo3D`. An empty name in PROJ is this CRS's own name.
    #[must_use]
    pub fn promote_to_3d(&self, db: Option<&DatabaseContext>) -> Self {
        match self {
            Self::Geographic(crs) => Self::Geographic(crs.promote_to_3d_with_db(db)),
            Self::Geodetic(GeodeticCRS::Geographic(crs)) => {
                Self::Geographic(crs.promote_to_3d_with_db(db))
            }
            Self::Projected(crs) => Self::Projected(crs.promote_to_3d(db)),
            Self::DerivedProjected(crs) => Self::DerivedProjected(crs.promote_to_3d(db)),
            Self::Derived(crs) if crs.kind() == DerivedKind::Geographic => {
                promote_derived_geographic(crs, db)
            }
            Self::Bound(crs) => promote_bound(crs, db),
            _ => self.clone(),
        }
    }

    /// `CRS::demoteTo2D`. A compound CRS returns its first component.
    #[must_use]
    pub fn demote_to_2d(&self, db: Option<&DatabaseContext>) -> Self {
        match self {
            Self::Geographic(crs) => Self::Geographic(crs.demote_to_2d(db)),
            Self::Geodetic(GeodeticCRS::Geographic(crs)) => Self::Geographic(crs.demote_to_2d(db)),
            Self::Projected(crs) => Self::Projected(crs.demote_to_2d(db)),
            Self::DerivedProjected(crs) => Self::DerivedProjected(crs.demote_to_2d(db)),
            Self::Derived(crs) if crs.kind() == DerivedKind::Geographic => {
                demote_derived_geographic(crs, db)
            }
            Self::Bound(crs) => demote_bound(crs, db),
            Self::Compound(crs) => crs
                .components()
                .first()
                .cloned()
                .unwrap_or_else(|| self.clone()),
            _ => self.clone(),
        }
    }
}

fn relabel_linear(parameter: &Parameter, change: &ParameterLinearUnit) -> Option<Parameter> {
    let ParameterValue::Measure(measure) = &parameter.value else {
        return None;
    };
    let unit = match change {
        ParameterLinearUnit::Relabel(unit) | ParameterLinearUnit::Convert(unit) => unit,
    };
    if measure.unit.kind != UnitKind::Linear || same_measure_unit(&measure.unit, unit) {
        return None;
    }
    let value = match change {
        ParameterLinearUnit::Convert(unit) => measure.convert_to(unit),
        ParameterLinearUnit::Relabel(_) => measure.value,
    };
    let mut updated = parameter.clone();
    updated.value = ParameterValue::measure(value, unit.clone());
    Some(updated)
}

fn same_measure_unit(left: &UnitOfMeasure, right: &UnitOfMeasure) -> bool {
    left.name() == right.name()
        && (left.conversion_to_si() - right.conversion_to_si()).abs() <= 1e-12
}

fn cartesian_with_unit(cs: &CartesianCS, unit: &UnitOfMeasure) -> CartesianCS {
    let mut cs = cs.clone();
    for axis in &mut cs.axes {
        replace_linear_axis(axis, unit);
    }
    cs
}

fn replace_linear_axis(axis: &mut Axis, unit: &UnitOfMeasure) {
    if axis.unit.kind == UnitKind::Linear {
        axis.unit = unit.clone();
    }
}

fn geographic_linear_unit(crs: &GeographicCRS, unit: &UnitOfMeasure) -> GeographicCRS {
    let mut crs = crs.clone();
    for axis in &mut crs.coordinate_system.axes {
        replace_linear_axis(axis, unit);
    }
    crs.identified.identifiers.clear();
    crs
}

fn geographic_angular_unit(crs: &GeographicCRS, unit: &UnitOfMeasure) -> GeographicCRS {
    let mut crs = crs.clone();
    for axis in &mut crs.coordinate_system.axes {
        if axis.unit.kind == UnitKind::Angular {
            axis.unit = unit.clone();
        }
    }
    crs.identified.identifiers.clear();
    crs
}

fn engineering_linear_unit(crs: &super::model::EngineeringCRS, unit: &UnitOfMeasure) -> Crs {
    let mut crs = crs.clone();
    match &mut crs.coordinate_system {
        CoordinateSystem::Cartesian(cs) => *cs = cartesian_with_unit(cs, unit),
        CoordinateSystem::Vertical(cs) => replace_linear_axis(&mut cs.axis, unit),
        _ => return Crs::Engineering(crs),
    }
    Crs::Engineering(crs)
}

fn named_wgs84(crs: &Crs) -> bool {
    let (datum, ensemble) = geodetic_anchor(crs);
    datum.is_some_and(|datum| datum.name() == "World Geodetic System 1984")
        || ensemble.is_some_and(|ensemble| ensemble.name() == "World Geodetic System 1984 ensemble")
}

fn geodetic_anchor(crs: &Crs) -> (Option<&GeodeticReferenceFrame>, Option<&DatumEnsemble>) {
    match crs {
        Crs::Geographic(crs) => (crs.datum(), crs.datum_ensemble()),
        Crs::Geodetic(crs) => (crs.datum(), crs.datum_ensemble()),
        Crs::Projected(crs) => anchor_of_geodetic(crs.base_crs()),
        Crs::DerivedProjected(crs) => anchor_of_geodetic(crs.base_crs().base_crs()),
        Crs::Derived(crs) => geodetic_anchor(crs.base_crs()),
        Crs::Compound(crs) => crs
            .components()
            .iter()
            .map(geodetic_anchor)
            .find(|(datum, ensemble)| datum.is_some() || ensemble.is_some())
            .unwrap_or((None, None)),
        Crs::Bound(crs) => geodetic_anchor(crs.source_crs()),
        _ => (None, None),
    }
}

fn anchor_of_geodetic(
    crs: &GeodeticCRS,
) -> (Option<&GeodeticReferenceFrame>, Option<&DatumEnsemble>) {
    (crs.datum(), crs.datum_ensemble())
}

fn promote_derived_geographic(crs: &DerivedCRS, db: Option<&DatabaseContext>) -> Crs {
    let CoordinateSystem::Ellipsoidal(cs) = &crs.coordinate_system else {
        return Crs::Derived(crs.clone());
    };
    if cs.axes.len() != 2 {
        return Crs::Derived(crs.clone());
    }
    let mut coordinate_system = cs.clone();
    coordinate_system.axes.push(Axis::new(
        "Ellipsoidal height",
        "h",
        "up",
        UnitOfMeasure::metre(),
    ));
    Crs::Derived(DerivedCRS {
        identified: Identified::named(crs.name()),
        base: Box::new(crs.base_crs().promote_to_3d(db)),
        conversion: crs.conversion.clone(),
        coordinate_system: CoordinateSystem::Ellipsoidal(coordinate_system),
    })
}

fn demote_derived_geographic(crs: &DerivedCRS, db: Option<&DatabaseContext>) -> Crs {
    let CoordinateSystem::Ellipsoidal(cs) = &crs.coordinate_system else {
        return Crs::Derived(crs.clone());
    };
    if cs.axes.len() != 3 {
        return Crs::Derived(crs.clone());
    }
    let mut coordinate_system = cs.clone();
    coordinate_system.axes.truncate(2);
    Crs::Derived(DerivedCRS {
        identified: Identified::named(crs.name()),
        base: Box::new(crs.base_crs().demote_to_2d(db)),
        conversion: crs.conversion.clone(),
        coordinate_system: CoordinateSystem::Ellipsoidal(coordinate_system),
    })
}

fn promote_bound(crs: &BoundCRS, db: Option<&DatabaseContext>) -> Crs {
    let source = crs.source.promote_to_3d(db);
    if operation_has_towgs84(&crs.transformation) {
        Crs::Bound(BoundCRS::create(
            source,
            crs.target.promote_to_3d(db),
            promote_operation(&crs.transformation, db),
        ))
    } else {
        let mut bound = crs.clone();
        bound.source = Box::new(source);
        Crs::Bound(bound)
    }
}

fn demote_bound(crs: &BoundCRS, db: Option<&DatabaseContext>) -> Crs {
    let source = crs.source.demote_to_2d(db);
    if operation_has_towgs84(&crs.transformation) {
        Crs::Bound(BoundCRS::create(
            source,
            crs.target.demote_to_2d(db),
            demote_operation(&crs.transformation, db),
        ))
    } else {
        let mut bound = crs.clone();
        bound.source = Box::new(source);
        Crs::Bound(bound)
    }
}

fn operation_has_towgs84(operation: &CoordinateOperation) -> bool {
    match operation {
        CoordinateOperation::Transformation(transformation) => {
            transformation.towgs84_lenient().is_some()
        }
        _ => false,
    }
}

fn promote_operation(
    operation: &CoordinateOperation,
    db: Option<&DatabaseContext>,
) -> CoordinateOperation {
    let CoordinateOperation::Transformation(transformation) = operation else {
        return operation.clone();
    };
    let mut transformation = transformation.clone();
    if let Some(source) = transformation.source.take() {
        transformation.source = Some(Box::new(source.promote_to_3d(db)));
    }
    if let Some(target) = transformation.target.take() {
        transformation.target = Some(Box::new(target.promote_to_3d(db)));
    }
    CoordinateOperation::Transformation(transformation)
}

fn demote_operation(
    operation: &CoordinateOperation,
    db: Option<&DatabaseContext>,
) -> CoordinateOperation {
    let CoordinateOperation::Transformation(transformation) = operation else {
        return operation.clone();
    };
    let mut transformation = transformation.clone();
    if let Some(source) = transformation.source.take() {
        transformation.source = Some(Box::new(source.demote_to_2d(db)));
    }
    if let Some(target) = transformation.target.take() {
        transformation.target = Some(Box::new(target.demote_to_2d(db)));
    }
    CoordinateOperation::Transformation(transformation)
}
