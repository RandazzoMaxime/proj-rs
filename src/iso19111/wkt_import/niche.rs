//! WKT import of temporal, parametric and derived CRS, after `buildTemporalCRS`,
//! `buildParametricCRS` and the `buildDerived*CRS` functions of PROJ `io.cpp`.

use super::{
    ci_eq, easting_northing, geocentric_cs, invalid, strip_quotes, EngineeringDatum, WktNode,
    WktObject, WktParser,
};
use crate::iso19111::factory::{AuthorityFactory, AuthorityObject, NameMatch, ObjectType};
use crate::iso19111::model::{
    Conversion, CoordinateSystem, Crs, DerivedProjectedCRS, FactoryError, GeocentricCRS,
    GeodeticCRS, Parameter, ParameterValue, UnitKind, UnitOfMeasure,
};
use crate::iso19111::niche::{
    DerivedCRS, ParametricCRS, ParametricCS, ParametricDatum, TemporalCRS, TemporalCS,
    TemporalDatum, PROLEPTIC_GREGORIAN,
};

fn missing(keyword: &str) -> FactoryError {
    invalid(format!("missing {keyword} node"))
}

impl WktParser {
    pub(super) fn build_temporal_datum(
        &mut self,
        node: &WktNode,
    ) -> Result<TemporalDatum, FactoryError> {
        let calendar = match node.child(&["CALENDAR"]) {
            Some(calendar) if calendar.children.len() == 1 => strip_quotes(&calendar.children[0]),
            _ => PROLEPTIC_GREGORIAN.to_owned(),
        };
        let origin = match node.child(&["TIMEORIGIN"]) {
            Some(origin) if origin.children.len() == 1 => strip_quotes(&origin.children[0]),
            _ => String::new(),
        };
        let props = self.build_properties(node)?;
        Ok(TemporalDatum {
            identified: props.identified(),
            calendar,
            origin,
        })
    }

    pub(super) fn build_parametric_datum(
        &mut self,
        node: &WktNode,
    ) -> Result<ParametricDatum, FactoryError> {
        let props = self.build_properties(node)?;
        let mut identified = props.identified();
        identified.anchor = self.anchor_of(node)?;
        Ok(ParametricDatum { identified })
    }

    pub(super) fn build_engineering_datum(
        &mut self,
        node: &WktNode,
    ) -> Result<EngineeringDatum, FactoryError> {
        let props = self.build_properties(node)?;
        let mut identified = props.identified();
        identified.anchor = self.anchor_of(node)?;
        Ok(EngineeringDatum { identified })
    }

    fn build_temporal_cs(&mut self, parent: &WktNode) -> Result<TemporalCS, FactoryError> {
        let cs_node = parent.child(&["CS"]);
        if cs_node.is_none() && !ci_eq(&parent.value, "BASETIMECRS") {
            return Err(missing("CS"));
        }
        match self.build_cs(cs_node, parent, UnitOfMeasure::new("", 1.0, UnitKind::None))? {
            CoordinateSystem::Temporal(cs) => Ok(cs),
            other => Err(invalid(format!(
                "buildCS: temporal CS expected, but found {}",
                other.wkt2_type()
            ))),
        }
    }

    fn build_parametric_cs(&mut self, parent: &WktNode) -> Result<ParametricCS, FactoryError> {
        let cs_node = parent.child(&["CS"]);
        if cs_node.is_none() && !ci_eq(&parent.value, "BASEPARAMCRS") {
            return Err(missing("CS"));
        }
        match self.build_cs(cs_node, parent, UnitOfMeasure::new("", 1.0, UnitKind::None))? {
            CoordinateSystem::Parametric(cs) => Ok(cs),
            other => Err(invalid(format!(
                "buildCS: parametric CS expected, but found {}",
                other.wkt2_type()
            ))),
        }
    }

    pub(super) fn build_temporal_crs(
        &mut self,
        node: &WktNode,
    ) -> Result<TemporalCRS, FactoryError> {
        let Some(datum) = node.child(&["TDATUM", "TIMEDATUM"]) else {
            return Err(invalid("Missing TDATUM / TIMEDATUM node"));
        };
        let props = self.build_properties(node)?;
        Ok(TemporalCRS {
            identified: props.identified(),
            datum: self.build_temporal_datum(datum)?,
            coordinate_system: self.build_temporal_cs(node)?,
        })
    }

    pub(super) fn build_parametric_crs(
        &mut self,
        node: &WktNode,
    ) -> Result<ParametricCRS, FactoryError> {
        let Some(datum) = node.child(&["PDATUM", "PARAMETRICDATUM"]) else {
            return Err(invalid("Missing PDATUM / PARAMETRICDATUM node"));
        };
        let props = self.build_properties(node)?;
        Ok(ParametricCRS {
            identified: props.identified(),
            datum: self.build_parametric_datum(datum)?,
            coordinate_system: self.build_parametric_cs(node)?,
        })
    }

    /// A temporal CRS, derived or not.
    pub(super) fn build_temporal_object(
        &mut self,
        node: &WktNode,
    ) -> Result<WktObject, FactoryError> {
        let Some(base) = node.child(&["BASETIMECRS"]) else {
            return Ok(WktObject::Crs(Crs::Temporal(
                self.build_temporal_crs(node)?,
            )));
        };
        let base = self.build_temporal_crs(base)?;
        let conversion = self.deriving_conversion(node)?;
        let cs = self.build_temporal_cs(node)?;
        self.derived(
            node,
            Crs::Temporal(base),
            conversion,
            CoordinateSystem::Temporal(cs),
        )
    }

    /// A parametric CRS, derived or not.
    pub(super) fn build_parametric_object(
        &mut self,
        node: &WktNode,
    ) -> Result<WktObject, FactoryError> {
        let Some(base) = node.child(&["BASEPARAMCRS"]) else {
            return Ok(WktObject::Crs(Crs::Parametric(
                self.build_parametric_crs(node)?,
            )));
        };
        let base = self.build_parametric_crs(base)?;
        let conversion = self.deriving_conversion(node)?;
        let cs = self.build_parametric_cs(node)?;
        self.derived(
            node,
            Crs::Parametric(base),
            conversion,
            CoordinateSystem::Parametric(cs),
        )
    }

    /// An engineering CRS, derived or not.
    pub(super) fn build_engineering_object(
        &mut self,
        node: &WktNode,
    ) -> Result<WktObject, FactoryError> {
        let Some(base) = node.child(&["BASEENGCRS"]) else {
            return self.build_engineering(node);
        };
        let WktObject::Crs(base) = self.build_engineering(base)? else {
            return Err(invalid("base CRS is not engineering"));
        };
        let conversion = self.deriving_conversion(node)?;
        let cs_node = node.child(&["CS"]).ok_or_else(|| missing("CS"))?;
        let cs = self.build_cs(
            Some(cs_node),
            node,
            UnitOfMeasure::new("", 1.0, UnitKind::None),
        )?;
        self.derived(node, base, conversion, cs)
    }

    /// A vertical CRS with a `BASEVERTCRS`.
    pub(super) fn build_derived_vertical(
        &mut self,
        node: &WktNode,
    ) -> Result<WktObject, FactoryError> {
        let base_node = node
            .child(&["BASEVERTCRS"])
            .ok_or_else(|| missing("BASEVERTCRS"))?;
        let base = self.build_vertical(base_node)?;
        let conversion = self.deriving_conversion(node)?;
        let cs_node = node.child(&["CS"]).ok_or_else(|| missing("CS"))?;
        let cs = self.build_cs(
            Some(cs_node),
            node,
            UnitOfMeasure::new("", 1.0, UnitKind::None),
        )?;
        if !matches!(cs, CoordinateSystem::Vertical(_)) {
            return Err(invalid(format!(
                "vertical CS expected, but found {}",
                cs.wkt2_type()
            )));
        }
        self.derived(node, Crs::Vertical(base), conversion, cs)
    }

    /// A geodetic CRS with a `BASEGEODCRS` or `BASEGEOGCRS`.
    pub(super) fn build_derived_geodetic(
        &mut self,
        node: &WktNode,
    ) -> Result<WktObject, FactoryError> {
        let base_node = node
            .child(&["BASEGEODCRS", "BASEGEOGCRS"])
            .ok_or_else(|| missing("BASEGEODCRS"))?;
        let conversion = self.deriving_conversion(node)?;
        let cs_node = node.child(&["CS"]).ok_or_else(|| missing("CS"))?;
        let cs = self.build_cs(
            Some(cs_node),
            node,
            UnitOfMeasure::new("", 1.0, UnitKind::None),
        )?;
        let geocentric_method = matches!(
            conversion.method.code.as_str(),
            "1031" | "1032" | "1033" | "1053" | "1056" | "1132"
        );
        let force_geocentric = matches!(&cs, CoordinateSystem::Cartesian(_))
            && geocentric_method
            && ci_eq(&base_node.value, "BASEGEODCRS");
        let mut base = match self.build_geodetic_crs(base_node)? {
            WktObject::Crs(crs) => crs,
            _ => return Err(invalid("base CRS is not geodetic")),
        };
        if force_geocentric && base_node.child(&["CS"]).is_none() {
            if let Crs::Geographic(geographic) = &base {
                base = Crs::Geodetic(GeodeticCRS::Geocentric(GeocentricCRS {
                    identified: geographic.identified.clone(),
                    datum: geographic.datum.clone(),
                    ensemble: geographic.ensemble.clone(),
                    coordinate_system: geocentric_cs(UnitOfMeasure::metre()),
                }));
            }
        }
        match &cs {
            CoordinateSystem::Ellipsoidal(_) => {}
            _ if ci_eq(&node.value, "GEOGCRS") => {
                return Err(invalid(format!(
                    "ellipsoidal CS expected, but found {}",
                    cs.wkt2_type()
                )));
            }
            CoordinateSystem::Cartesian(cartesian) => {
                if cartesian.axes.len() != 3 {
                    return Err(invalid("Cartesian CS for a GeodeticCRS should have 3 axis"));
                }
            }
            CoordinateSystem::Spherical(_) => {}
            other => {
                return Err(invalid(format!("unhandled CS type: {}", other.wkt2_type())));
            }
        }
        self.derived(node, base, conversion, cs)
    }

    /// `DERIVEDPROJCRS`.
    pub(super) fn build_derived_projected(
        &mut self,
        node: &WktNode,
    ) -> Result<WktObject, FactoryError> {
        let base_node = node
            .child(&["BASEPROJCRS"])
            .ok_or_else(|| invalid("not enough children in BASEPROJCRS node"))?;
        let WktObject::Crs(Crs::Projected(base)) = self.build_projected(base_node)? else {
            return Err(invalid("base CRS is not projected"));
        };
        let conversion_node = node
            .child(&["DERIVINGCONVERSION"])
            .ok_or_else(|| invalid("not enough children in DERIVINGCONVERSION node"))?;
        let linear = self
            .unit_in(node, UnitKind::Unknown)?
            .unwrap_or_else(|| UnitOfMeasure::new("", 1.0, UnitKind::None));
        let angular = base
            .base
            .coordinate_system()
            .axes()
            .first()
            .map(|axis| axis.unit().clone())
            .unwrap_or_else(UnitOfMeasure::degree);
        let conversion = self.build_conversion(conversion_node, linear, angular)?;
        let cs_node = node.child(&["CS"]).ok_or_else(|| missing("CS"))?;
        let cs = self.build_cs(
            Some(cs_node),
            node,
            UnitOfMeasure::new("", 1.0, UnitKind::None),
        )?;
        let props = self.build_properties(node)?;
        Ok(WktObject::Crs(Crs::DerivedProjected(DerivedProjectedCRS {
            identified: props.identified(),
            base,
            conversion,
            coordinate_system: cs,
        })))
    }

    /// The CS of a `BASEPROJCRS` that has none, which WKT2 does not allow: the CRS found by
    /// its identifier or its name, else a metric-or-not easting/northing from the linear unit
    /// of the conversion parameters.
    pub(super) fn base_projected_cs(
        &mut self,
        node: &WktNode,
    ) -> Result<Option<CoordinateSystem>, FactoryError> {
        let Some(conversion_node) = node.child(&["CONVERSION"]) else {
            return Ok(None);
        };
        if !ci_eq(&node.value, "BASEPROJCRS") || node.child(&["CS"]).is_some() {
            return Ok(None);
        }
        if let Some(db) = self.db.clone() {
            if let Some(id) = node.child(&["ID"]) {
                if let Ok(identifier) = self.build_id(id) {
                    let factory = AuthorityFactory::create(&db, identifier.code_space());
                    if let Ok(projected) = factory.create_projected_crs(identifier.code()) {
                        return Ok(Some(CoordinateSystem::Cartesian(
                            projected.coordinate_system,
                        )));
                    }
                }
            }
            let name = node.children.first().map(strip_quotes).unwrap_or_default();
            let found = AuthorityFactory::create(&db, "").create_objects_from_name(
                &name,
                &[ObjectType::ProjectedCrs],
                NameMatch::Exact,
                2,
            )?;
            if let [AuthorityObject::Crs(Crs::Projected(projected))] = found.as_slice() {
                return Ok(Some(CoordinateSystem::Cartesian(
                    projected.coordinate_system.clone(),
                )));
            }
        }
        let conversion = self.build_conversion(
            conversion_node,
            UnitOfMeasure::metre(),
            UnitOfMeasure::degree(),
        )?;
        let mut linear: Option<UnitOfMeasure> = None;
        for parameter in &conversion.parameters {
            let ParameterValue::Measure(measure) = &parameter.value else {
                continue;
            };
            if measure.unit.kind != UnitKind::Linear {
                continue;
            }
            match &linear {
                None => linear = Some(measure.unit.clone()),
                Some(unit) if !unit.same_name(&measure.unit) => return Ok(None),
                Some(_) => {}
            }
        }
        Ok(linear.map(|unit| CoordinateSystem::Cartesian(easting_northing(unit))))
    }

    /// `dealWithEPSGCodeForInterpolationCRSParameter`: the `EPSG code for ... CRS` parameter
    /// becomes the interpolation CRS when the database knows that geographic CRS.
    pub(super) fn take_interpolation_crs(
        &mut self,
        parameters: &mut Vec<Parameter>,
    ) -> Option<Box<Crs>> {
        let db = self.db.clone()?;
        for index in 0..parameters.len() {
            let parameter = &parameters[index];
            let is_code = parameter.name == "EPSG code for Interpolation CRS"
                || parameter.name == "EPSG code for Horizontal CRS"
                || (parameter.code_space == "EPSG"
                    && matches!(parameter.code.as_str(), "1048" | "1037"));
            let ParameterValue::Integer(code) = &parameter.value else {
                continue;
            };
            if !is_code {
                continue;
            }
            if let Ok(crs) =
                AuthorityFactory::create(&db, "EPSG").create_geographic_crs(&code.to_string())
            {
                parameters.remove(index);
                return Some(Box::new(Crs::Geographic(crs)));
            }
        }
        None
    }

    fn deriving_conversion(&mut self, node: &WktNode) -> Result<Conversion, FactoryError> {
        let conversion = node
            .child(&["DERIVINGCONVERSION"])
            .ok_or_else(|| invalid("not enough children in DERIVINGCONVERSION node"))?;
        let none = UnitOfMeasure::new("", 1.0, UnitKind::None);
        self.build_conversion(conversion, none.clone(), none)
    }

    fn derived(
        &mut self,
        node: &WktNode,
        base: Crs,
        conversion: Conversion,
        coordinate_system: CoordinateSystem,
    ) -> Result<WktObject, FactoryError> {
        let props = self.build_properties(node)?;
        Ok(WktObject::Crs(Crs::Derived(DerivedCRS {
            identified: props.identified(),
            base: Box::new(base),
            conversion,
            coordinate_system,
        })))
    }
}
