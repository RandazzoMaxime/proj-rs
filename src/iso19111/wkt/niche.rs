//! WKT2 export of temporal, parametric and derived CRS.

use super::{base_angular_unit, BaseShape, OutputAxisRule};
use super::{
    export_cs, export_engineering_datum, export_geodetic_anchor, export_prime_meridian,
    export_vertical_datum, export_vertical_ensemble, geodetic_meridian, node_from_ids, top_has_id,
    NodeId, WktFormatter,
};
use crate::iso19111::factory::{AuthorityFactory, NameMatch, ObjectType};
use crate::iso19111::model::{
    CoordinateSystem, Crs, DatumEnsemble, DerivedProjectedCRS, FactoryError, GeodeticCRS,
    GeodeticReferenceFrame, Identified, ProjectedCRS, UnitOfMeasure,
};
use crate::iso19111::niche::{
    DerivedCRS, ParametricCRS, ParametricDatum, TemporalCRS, TemporalDatum,
};

fn require_wkt2(formatter: &WktFormatter, what: &str) -> Result<(), FactoryError> {
    if formatter.wkt2 {
        Ok(())
    } else {
        Err(FactoryError::Format(format!(
            "{what} can only be exported to WKT2"
        )))
    }
}

pub(super) fn export_temporal_datum(datum: &TemporalDatum, formatter: &mut WktFormatter) {
    formatter.start_node("TDATUM", node_from_ids(&datum.identified.identifiers));
    formatter.add_quoted(&datum.identified.name);
    if formatter.keywords_2019 {
        formatter.start_node("CALENDAR", NodeId::Absent);
        formatter.add_quoted(&datum.calendar);
        formatter.end_node();
    }
    if !datum.origin.is_empty() {
        formatter.start_node("TIMEORIGIN", NodeId::Absent);
        if is_iso_8601(&datum.origin) {
            formatter.add_raw(&datum.origin);
        } else {
            formatter.add_quoted(&datum.origin);
        }
        formatter.end_node();
    }
    formatter.end_node();
}

/// `DateTime::isISO_8601`: starts with a digit, and is a date or date-time.
fn is_iso_8601(text: &str) -> bool {
    text.chars().next().is_some_and(|ch| ch.is_ascii_digit())
}

pub(super) fn export_parametric_datum(datum: &ParametricDatum, formatter: &mut WktFormatter) {
    formatter.start_node("PDATUM", node_from_ids(&datum.identified.identifiers));
    formatter.add_quoted(&datum.identified.name);
    export_anchor(&datum.identified, formatter);
    formatter.end_node();
}

fn export_anchor(identified: &Identified, formatter: &mut WktFormatter) {
    if let Some(anchor) = &identified.anchor {
        formatter.start_node("ANCHOR", NodeId::Absent);
        formatter.add_quoted(anchor);
        formatter.end_node();
    }
}

pub(super) fn export_temporal(
    crs: &TemporalCRS,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    require_wkt2(formatter, "TemporalCRS")?;
    formatter.start_node("TIMECRS", node_from_ids(&crs.identified.identifiers));
    formatter.add_quoted(&crs.identified.name);
    export_temporal_datum(&crs.datum, formatter);
    export_axes(
        &CoordinateSystem::Temporal(crs.coordinate_system.clone()),
        formatter,
    );
    formatter.base_usage(&crs.identified);
    formatter.end_node();
    Ok(())
}

pub(super) fn export_parametric(
    crs: &ParametricCRS,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    require_wkt2(formatter, "ParametricCRS")?;
    formatter.start_node("PARAMETRICCRS", node_from_ids(&crs.identified.identifiers));
    formatter.add_quoted(&crs.identified.name);
    export_parametric_datum(&crs.datum, formatter);
    export_axes(
        &CoordinateSystem::Parametric(crs.coordinate_system.clone()),
        formatter,
    );
    formatter.base_usage(&crs.identified);
    formatter.end_node();
    Ok(())
}

/// `coordinateSystem()->_exportToWKT`: the axes are always written.
fn export_axes(cs: &CoordinateSystem, formatter: &mut WktFormatter) {
    let previous = formatter.output_axis;
    formatter.output_axis = OutputAxisRule::Yes;
    export_cs(cs, formatter);
    formatter.output_axis = previous;
}

fn geodetic_parts(
    crs: &Crs,
) -> Option<(
    Option<&GeodeticReferenceFrame>,
    Option<&DatumEnsemble>,
    bool,
)> {
    match crs {
        Crs::Geographic(base) => Some((base.datum.as_ref(), base.ensemble.as_ref(), true)),
        Crs::Geodetic(base) => Some((
            base.datum(),
            base.datum_ensemble(),
            matches!(base, GeodeticCRS::Geographic(_)),
        )),
        _ => None,
    }
}

fn export_base_node(
    base: &Crs,
    keyword: &str,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    let base_has_id = formatter.keywords_2019 && !base.identifiers().is_empty();
    formatter.start_node(
        keyword,
        if base_has_id {
            NodeId::Present
        } else {
            NodeId::Absent
        },
    );
    formatter.add_quoted(base.name());
    match base {
        Crs::Geographic(_) | Crs::Geodetic(_) => {
            let Some((datum, ensemble, _)) = geodetic_parts(base) else {
                return Err(FactoryError::Format("unsupported geodetic base".to_owned()));
            };
            export_geodetic_anchor(datum, ensemble, formatter);
            if let Some(meridian) = geodetic_meridian(datum, ensemble) {
                export_prime_meridian(meridian, formatter);
            }
        }
        Crs::Vertical(vertical) => {
            if let Some(datum) = &vertical.datum {
                export_vertical_datum(datum, formatter);
            } else if let Some(ensemble) = &vertical.ensemble {
                export_vertical_ensemble(ensemble, formatter);
            }
        }
        Crs::Engineering(engineering) => export_engineering_datum(&engineering.datum, formatter),
        Crs::Parametric(parametric) => export_parametric_datum(&parametric.datum, formatter),
        Crs::Temporal(temporal) => export_temporal_datum(&temporal.datum, formatter),
        _ => {
            return Err(FactoryError::Format(format!(
                "{} cannot be the base of a derived CRS",
                base.name()
            )))
        }
    }
    if formatter.keywords_2019 && !(formatter.id_on_top_only && top_has_id(formatter)) {
        formatter.output_id.push(true);
        formatter.format_ids(base.identifiers());
        formatter.output_id.pop();
    }
    formatter.end_node();
    Ok(())
}

fn export_deriving_conversion(
    conversion: &crate::iso19111::model::Conversion,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    formatter.use_deriving_conversion = true;
    let result = super::export_conversion(conversion, formatter, BaseShape::NotSphere, None);
    formatter.use_deriving_conversion = false;
    result
}

pub(super) fn export_derived(
    crs: &DerivedCRS,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    let (keyword, base_keyword) = match (&*crs.base, &crs.coordinate_system) {
        (Crs::Geographic(_) | Crs::Geodetic(_), cs) => {
            let geographic_base = geodetic_parts(&crs.base).is_some_and(|parts| parts.2);
            let base_keyword = if formatter.keywords_2019 && geographic_base {
                "BASEGEOGCRS"
            } else {
                "BASEGEODCRS"
            };
            let keyword =
                if formatter.keywords_2019 && matches!(cs, CoordinateSystem::Ellipsoidal(_)) {
                    "GEOGCRS"
                } else {
                    "GEODCRS"
                };
            require_wkt2(formatter, "DerivedGeodeticCRS")?;
            (keyword, base_keyword)
        }
        (Crs::Vertical(_), _) => ("VERTCRS", "BASEVERTCRS"),
        (Crs::Engineering(_), _) => ("ENGCRS", "BASEENGCRS"),
        (Crs::Parametric(_), _) => ("PARAMETRICCRS", "BASEPARAMCRS"),
        (Crs::Temporal(_), _) => ("TIMECRS", "BASETIMECRS"),
        _ => {
            return Err(FactoryError::Format(
                "unsupported base of a derived CRS".to_owned(),
            ))
        }
    };
    require_wkt2(formatter, "DerivedCRS")?;
    formatter.start_node(keyword, node_from_ids(&crs.identified.identifiers));
    formatter.add_quoted(&crs.identified.name);
    export_base_node(&crs.base, base_keyword, formatter)?;
    export_deriving_conversion(&crs.conversion, formatter)?;
    export_axes(&crs.coordinate_system, formatter);
    formatter.base_usage(&crs.identified);
    formatter.end_node();
    Ok(())
}

pub(super) fn export_derived_projected(
    crs: &DerivedProjectedCRS,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    if !formatter.wkt2 || !formatter.keywords_2019 {
        return Err(FactoryError::Format(
            "DerivedProjectedCRS can only be exported to WKT2:2019".to_owned(),
        ));
    }
    formatter.start_node("DERIVEDPROJCRS", node_from_ids(&crs.identified.identifiers));
    formatter.add_quoted(&crs.identified.name);
    let base = crs.base_crs();
    formatter.start_node("BASEPROJCRS", node_from_ids(base.identifiers()));
    formatter.add_quoted(base.name());
    let geodetic = base.base_crs();
    let geographic = matches!(geodetic, GeodeticCRS::Geographic(_));
    formatter.start_node(
        if geographic {
            "BASEGEOGCRS"
        } else {
            "BASEGEODCRS"
        },
        node_from_ids(geodetic.identifiers()),
    );
    formatter.add_quoted(geodetic.name());
    export_geodetic_anchor(geodetic.datum(), geodetic.datum_ensemble(), formatter);
    if formatter.omit_parameter_unit_if_axis {
        formatter.export_unit(&base_angular_unit(geodetic), None);
    }
    if let Some(meridian) = geodetic_meridian(geodetic.datum(), geodetic.datum_ensemble()) {
        export_prime_meridian(meridian, formatter);
    }
    formatter.end_node();
    export_projected_conversion(base, formatter)?;
    // WKT2 has no BASEPROJCRS.CS, but the CS is written when the reader could not infer it.
    let non_metre = base
        .coordinate_system
        .axes
        .first()
        .is_some_and(|axis| !axis.unit().same_name(&UnitOfMeasure::metre()));
    if non_metre && base.identifiers().is_empty() && !known_projected_name(base, formatter) {
        export_axes(
            &CoordinateSystem::Cartesian(base.coordinate_system.clone()),
            formatter,
        );
    }
    if crs.identified.identifiers.is_empty() && !base.identifiers().is_empty() {
        formatter.format_ids(base.identifiers());
    }
    formatter.end_node();
    export_deriving_conversion(&crs.conversion, formatter)?;
    export_axes(&crs.coordinate_system, formatter);
    formatter.base_usage(&crs.identified);
    formatter.end_node();
    Ok(())
}

/// Whether `proj.db` has exactly one projected CRS of that name.
fn known_projected_name(base: &ProjectedCRS, formatter: &WktFormatter) -> bool {
    let Some(db) = &formatter.db else {
        return false;
    };
    AuthorityFactory::create(db, "")
        .create_objects_from_name(
            base.name(),
            &[ObjectType::ProjectedCrs],
            NameMatch::Exact,
            2,
        )
        .is_ok_and(|found| found.len() == 1)
}

fn export_projected_conversion(
    base: &ProjectedCRS,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    super::export_conversion(&base.conversion, formatter, BaseShape::NotSphere, None)
}
