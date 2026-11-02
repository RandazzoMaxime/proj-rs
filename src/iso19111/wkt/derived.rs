//! WKT export of the temporal, parametric and derived CRS (`TemporalCRS::_exportToWKT`,
//! `ParametricCRS::_exportToWKT`, `DerivedCRS::baseExportToWKT` and the derived projected CRS).

use super::{
    base_angular_unit, export_base_datum, export_cs, export_engineering_datum,
    export_prime_meridian, export_vertical, export_vertical_datum, export_vertical_ensemble,
    export_wkt2_conversion, node_from_ids, top_has_id, NodeId, OutputAxisRule, WktFormatter,
};
use crate::iso19111::derived_crs::{DerivedCRS, DerivedKind, ParametricCRS, TemporalCRS};
use crate::iso19111::model::{
    Axis, Conversion, CoordinateSystem, Crs, DerivedProjectedCRS, FactoryError, GeodeticCRS,
    Identified, Identifier, UnitOfMeasure, VerticalCRS, VerticalCS,
};

fn format_error(message: &str) -> FactoryError {
    FactoryError::Format(message.to_owned())
}

/// The CS of a derived CRS is always written with its axes.
fn export_axes(cs: &CoordinateSystem, formatter: &mut WktFormatter) {
    let previous = formatter.output_axis;
    formatter.output_axis = OutputAxisRule::Yes;
    export_cs(cs, formatter);
    formatter.output_axis = previous;
}

fn export_temporal_datum(crs: &TemporalCRS, formatter: &mut WktFormatter) {
    let datum = &crs.datum;
    formatter.start_node("TDATUM", node_from_ids(&datum.identified.identifiers));
    formatter.add_quoted(datum.name());
    if formatter.keywords_2019 {
        formatter.start_node("CALENDAR", NodeId::Absent);
        formatter.add_quoted(datum.calendar());
        formatter.end_node();
    }
    let origin = datum.temporal_origin();
    if !origin.is_empty() {
        formatter.start_node("TIMEORIGIN", NodeId::Absent);
        if origin.starts_with(|c: char| c.is_ascii_digit()) {
            formatter.add_raw(origin);
        } else {
            formatter.add_quoted(origin);
        }
        formatter.end_node();
    }
    formatter.end_node();
}

fn export_parametric_datum(crs: &ParametricCRS, formatter: &mut WktFormatter) {
    let datum = &crs.datum;
    formatter.start_node("PDATUM", node_from_ids(&datum.identified.identifiers));
    formatter.add_quoted(datum.name());
    formatter.end_node();
}

pub(super) fn export_temporal(
    crs: &TemporalCRS,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    if !formatter.wkt2 {
        return Err(format_error("TemporalCRS can only be exported to WKT2"));
    }
    formatter.start_node("TIMECRS", node_from_ids(&crs.identified.identifiers));
    formatter.add_quoted(crs.name());
    export_temporal_datum(crs, formatter);
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
    if !formatter.wkt2 {
        return Err(format_error("ParametricCRS can only be exported to WKT2"));
    }
    formatter.start_node("PARAMETRICCRS", node_from_ids(&crs.identified.identifiers));
    formatter.add_quoted(crs.name());
    export_parametric_datum(crs, formatter);
    export_axes(
        &CoordinateSystem::Parametric(crs.coordinate_system.clone()),
        formatter,
    );
    formatter.base_usage(&crs.identified);
    formatter.end_node();
    Ok(())
}

/// `DERIVINGCONVERSION`, then the CS and the usage of the derived CRS.
fn export_derived_tail(
    conversion: &Conversion,
    cs: &CoordinateSystem,
    identified: &Identified,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    export_wkt2_conversion(conversion, formatter, "DERIVINGCONVERSION")?;
    export_axes(cs, formatter);
    formatter.base_usage(identified);
    formatter.end_node();
    Ok(())
}

/// The base CRS identifier, written under the 2019 keywords unless a parent has one.
fn export_base_id(identifiers: &[Identifier], formatter: &mut WktFormatter) {
    if formatter.keywords_2019 && !(formatter.id_on_top_only && top_has_id(formatter)) {
        formatter.output_id.push(true);
        formatter.format_ids(identifiers);
        formatter.output_id.pop();
    }
}

/// Geographic and geodetic bases are opened with an id whenever they have one, so the
/// datum and prime meridian stay anonymous in WKT2:2015. The other derived kinds do
/// that only under the 2019 keywords (`DerivedCRS::baseExportToWKT`).
fn base_node_id(kind: DerivedKind, identifiers: &[Identifier], formatter: &WktFormatter) -> NodeId {
    let suppress = !identifiers.is_empty()
        && (matches!(kind, DerivedKind::Geographic | DerivedKind::Geodetic)
            || formatter.keywords_2019);
    if suppress {
        NodeId::Present
    } else {
        NodeId::Absent
    }
}

pub(super) fn export_derived(
    crs: &DerivedCRS,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    let kind = crs.kind();
    if !formatter.wkt2 {
        if kind == DerivedKind::Vertical && simple_vertical_derivation(crs) {
            return export_vertical(&vertical_view(crs), formatter);
        }
        return Err(format_error(match kind {
            DerivedKind::Geographic => "DerivedGeographicCRS can only be exported to WKT2",
            DerivedKind::Geodetic => "DerivedGeodeticCRS can only be exported to WKT2",
            DerivedKind::Vertical => "DerivedVerticalCRS can only be exported to WKT2",
            DerivedKind::Engineering => "DerivedEngineeringCRS can only be exported to WKT2:2019",
            DerivedKind::Parametric => "DerivedParametricCRS can only be exported to WKT2",
            DerivedKind::Temporal => "DerivedTemporalCRS can only be exported to WKT2",
        }));
    }
    if kind == DerivedKind::Engineering && !formatter.keywords_2019 {
        return Err(format_error(
            "DerivedEngineeringCRS can only be exported to WKT2:2019",
        ));
    }
    let base = crs.base_crs();
    let keyword = match kind {
        DerivedKind::Geographic if formatter.keywords_2019 => "GEOGCRS",
        DerivedKind::Geographic | DerivedKind::Geodetic => "GEODCRS",
        DerivedKind::Vertical => "VERTCRS",
        DerivedKind::Engineering => "ENGCRS",
        DerivedKind::Parametric => "PARAMETRICCRS",
        DerivedKind::Temporal => "TIMECRS",
    };
    let base_keyword = match (kind, base) {
        (DerivedKind::Geographic | DerivedKind::Geodetic, Crs::Geographic(_))
            if formatter.keywords_2019 =>
        {
            "BASEGEOGCRS"
        }
        (DerivedKind::Geographic | DerivedKind::Geodetic, _) => "BASEGEODCRS",
        (DerivedKind::Vertical, _) => "BASEVERTCRS",
        (DerivedKind::Engineering, _) => "BASEENGCRS",
        (DerivedKind::Parametric, _) => "BASEPARAMCRS",
        (DerivedKind::Temporal, _) => "BASETIMECRS",
    };
    formatter.start_node(keyword, node_from_ids(&crs.identified.identifiers));
    formatter.add_quoted(crs.name());
    formatter.start_node(
        base_keyword,
        base_node_id(kind, base.identifiers(), formatter),
    );
    formatter.add_quoted(base.name());
    export_base_datum_of(base, formatter)?;
    export_base_id(base.identifiers(), formatter);
    formatter.end_node();
    export_derived_tail(
        &crs.conversion,
        &crs.coordinate_system,
        &crs.identified,
        formatter,
    )
}

/// `exportDatumOrDatumEnsembleToWkt` of the base CRS, and its prime meridian when geodetic.
fn export_base_datum_of(base: &Crs, formatter: &mut WktFormatter) -> Result<(), FactoryError> {
    match base {
        Crs::Geographic(crs) => {
            let geodetic = GeodeticCRS::Geographic(crs.clone());
            export_geodetic_base(&geodetic, formatter);
        }
        Crs::Geodetic(crs) => export_geodetic_base(crs, formatter),
        Crs::Vertical(crs) => {
            if let Some(datum) = &crs.datum {
                export_vertical_datum(datum, formatter);
            } else if let Some(ensemble) = &crs.ensemble {
                export_vertical_ensemble(ensemble, formatter);
            }
        }
        Crs::Engineering(crs) => export_engineering_datum(&crs.datum, formatter),
        Crs::Parametric(crs) => export_parametric_datum(crs, formatter),
        Crs::Temporal(crs) => export_temporal_datum(crs, formatter),
        _ => return Err(format_error("unsupported base CRS of a derived CRS")),
    }
    Ok(())
}

fn export_geodetic_base(base: &GeodeticCRS, formatter: &mut WktFormatter) {
    export_base_datum(base, formatter);
    if let Some(meridian) = base.prime_meridian() {
        formatter.axis_angular.push(base_angular_unit(base));
        export_prime_meridian(meridian, formatter);
        formatter.axis_angular.pop();
    }
}

/// Derived vertical CRS made by a unit change or a height/depth reversal only: WKT1 writes it
/// as a plain vertical CRS (`DerivedVerticalCRS::_exportToWKT`).
fn simple_vertical_derivation(crs: &DerivedCRS) -> bool {
    let mut current = crs;
    loop {
        let code = current.conversion.method.code.as_str();
        if !matches!(code, "1069" | "1104" | "1068") {
            return false;
        }
        match current.base_crs() {
            Crs::Derived(next) if next.kind() == DerivedKind::Vertical => current = next,
            _ => return true,
        }
    }
}

/// The vertical CRS PROJ exports for a derived vertical CRS in WKT1: the datum of the innermost
/// base, with the name and coordinate system of the derived CRS.
fn vertical_view(crs: &DerivedCRS) -> VerticalCRS {
    let mut innermost = crs.base_crs();
    while let Crs::Derived(next) = innermost {
        innermost = next.base_crs();
    }
    let (datum, ensemble) = match innermost {
        Crs::Vertical(base) => (base.datum.clone(), base.ensemble.clone()),
        _ => (None, None),
    };
    let axis = match &crs.coordinate_system {
        CoordinateSystem::Vertical(cs) => cs.axis.clone(),
        other => other.axes().first().map_or_else(
            || Axis::create("Gravity-related height", "H", "up", UnitOfMeasure::metre()),
            |axis| (*axis).clone(),
        ),
    };
    VerticalCRS {
        identified: crs.identified.clone(),
        datum,
        ensemble,
        coordinate_system: VerticalCS {
            identified: Identified::default(),
            axis,
        },
        geoid_models: Vec::new(),
    }
}

pub(super) fn export_derived_projected(
    crs: &DerivedProjectedCRS,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    if !formatter.wkt2 || !formatter.keywords_2019 {
        return Err(format_error(
            "DerivedProjectedCRS can only be exported to WKT2:2019",
        ));
    }
    formatter.start_node("DERIVEDPROJCRS", node_from_ids(&crs.identified.identifiers));
    formatter.add_quoted(crs.name());
    let base = crs.base_crs();
    formatter.start_node("BASEPROJCRS", node_from_ids(base.identifiers()));
    formatter.add_quoted(base.name());
    let geodetic = base.base_crs();
    let geodetic_keyword = if matches!(geodetic, GeodeticCRS::Geographic(_)) {
        "BASEGEOGCRS"
    } else {
        "BASEGEODCRS"
    };
    formatter.start_node(geodetic_keyword, node_from_ids(geodetic.identifiers()));
    formatter.add_quoted(geodetic.name());
    export_base_datum(geodetic, formatter);
    let angular = base_angular_unit(geodetic);
    if formatter.omit_parameter_unit_if_axis {
        formatter.export_unit(&angular, None);
    }
    if let Some(meridian) = geodetic.prime_meridian() {
        formatter.axis_angular.push(angular);
        export_prime_meridian(meridian, formatter);
        formatter.axis_angular.pop();
    }
    formatter.end_node();
    export_wkt2_conversion(&base.conversion, formatter, "CONVERSION")?;
    // A base CS is written only when its unit is not the metre and the CRS has no identifier.
    let base_cs = base.coordinate_system();
    let non_metre = base_cs
        .axes()
        .first()
        .is_some_and(|axis| !axis.unit().same_name(&UnitOfMeasure::metre()));
    if non_metre && base.identifiers().is_empty() {
        let known = formatter.db.as_ref().is_some_and(|db| {
            crate::iso19111::AuthorityFactory::create(db, "")
                .create_objects_from_name(
                    base.name(),
                    &[crate::iso19111::ObjectType::ProjectedCrs],
                    crate::iso19111::NameMatch::Exact,
                    2,
                )
                .is_ok_and(|found| found.len() == 1)
        });
        if !known {
            export_axes(&CoordinateSystem::Cartesian(base_cs.clone()), formatter);
        }
    }
    if crs.identified.identifiers.is_empty() && !base.identifiers().is_empty() {
        formatter.output_id.push(true);
        formatter.format_ids(base.identifiers());
        formatter.output_id.pop();
    }
    formatter.end_node();
    export_derived_tail(
        &crs.conversion,
        &crs.coordinate_system,
        &crs.identified,
        formatter,
    )
}
