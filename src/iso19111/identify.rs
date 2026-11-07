//! `GeodeticCRS::identify` when an authority factory is present.
//!
//! The no-factory path stays in `equiv`. With a factory, a hardcoded WGS 84 /
//! NAD27 / NAD83 candidate is returned only when the name is exactly the same
//! and the CRS is equivalent. Every other match comes from `proj.db`.

use std::collections::BTreeSet;

use super::bound::hub_is_wgs84;
use super::buildop::UtmHemisphere;
use super::equiv::{
    directions_and_units, ellipsoid_parameters_match, equivalent_name_bigger_differences,
    identify_axes_match, meridian_equivalent, same_epoch, vertical_crs_equivalent, Equivalence,
    IdentifyAxisOrder,
};
use super::factory::{
    same_unit, AuthorityFactory, CoordinateOperationContext, CoordinateOperationFactory,
    DatabaseContext, NameMatch, SpatialCriterion,
};
use super::model::{
    Axis, BoundCRS, CartesianCS, CompoundCRS, ConcatenatedStep, CoordinateOperation,
    CoordinateSystem, Crs, Datum, DatumEnsemble, DerivedProjectedCRS, GeodeticCRS,
    GeodeticReferenceFrame, GeographicCRS, Identified, Identifier, ObjectDomain, ProjectedCRS,
    VerticalCRS,
};
use super::pipeline::ProjLines;

const UNKNOWN_BASED_ON: &str = "Unknown based on ";
const BALLPARK_GEOCENTRIC_TRANSLATION: &str = "Ballpark geocentric translation";
const NULL_GEOGRAPHIC_OFFSET: &str = "Null geographic offset";

struct Hit {
    crs: GeodeticCRS,
    confidence: i32,
}

impl GeodeticCRS {
    /// `GeodeticCRS::identify`.
    ///
    /// `None` is the hardcoded WGS 84, NAD27, and NAD83 search. A factory
    /// looks the CRS up in that factory's authority, then by datum and ellipsoid.
    #[must_use]
    pub fn identify_with(&self, factory: Option<&AuthorityFactory>) -> Vec<(Crs, i32)> {
        let Some(factory) = factory else {
            return self.identify();
        };
        let db = factory.database_context();
        let order = axis_order(self);
        let mut hits = Vec::new();
        if factory.authority().is_empty() || factory.authority() == "EPSG" {
            if let Some(found) = hardcoded_match(self, order, db) {
                return vec![found];
            }
        }
        let crs_type = geodetic_type(self);
        let name = self.name();
        if insignificant(name) {
            search_by_datum_or_ellipsoid(self, factory, db, order, crs_type, &mut hits);
        } else if compatible_authority(self.identifiers(), factory) {
            if let Some(found) = identify_by_code(self, factory, db, order) {
                return vec![found];
            }
        } else {
            let mut above_25 = false;
            for mode in [NameMatch::Exact, NameMatch::Approximate] {
                let Ok(objects) = factory.create_geodetic_crs_by_name(name, mode) else {
                    break;
                };
                for crs in objects {
                    if db_equivalent(self, &crs, order, db) {
                        if crs.name() == name {
                            return vec![(to_crs(crs), 100)];
                        }
                        let confidence = if equivalent_name_bigger_differences(name, crs.name()) {
                            90
                        } else {
                            70
                        };
                        hits.push(Hit { crs, confidence });
                        above_25 = true;
                    } else {
                        hits.push(Hit {
                            crs,
                            confidence: 25,
                        });
                    }
                }
                if !hits.is_empty() {
                    break;
                }
            }
            if !above_25 {
                search_by_datum_or_ellipsoid(self, factory, db, order, crs_type, &mut hits);
            }
        }
        sort_hits(self, db, &mut hits);
        if hits.len() >= 2 && hits[0].confidence == 90 {
            let mut kept = Vec::new();
            for hit in hits {
                if hit.confidence == 90 {
                    kept.push(hit);
                } else {
                    break;
                }
            }
            hits = kept;
        }
        hits.into_iter()
            .map(|hit| (to_crs(hit.crs), hit.confidence))
            .collect()
    }
}

impl GeographicCRS {
    /// `GeographicCRS::identify`. See [`GeodeticCRS::identify_with`].
    #[must_use]
    pub fn identify_with(&self, factory: Option<&AuthorityFactory>) -> Vec<(Crs, i32)> {
        GeodeticCRS::Geographic(self.clone()).identify_with(factory)
    }
}

impl Crs {
    /// `CRS::identify`.
    ///
    /// Geographic and geodetic CRS use [`GeodeticCRS::identify_with`]. Other
    /// kinds have no match.
    #[must_use]
    pub fn identify_with(&self, factory: Option<&AuthorityFactory>) -> Vec<(Crs, i32)> {
        match self {
            Self::Geographic(crs) => crs.identify_with(factory),
            Self::Geodetic(crs) => crs.identify_with(factory),
            Self::Projected(crs) => crs.identify_with(factory),
            Self::Vertical(crs) => crs.identify_with(factory),
            Self::Compound(crs) => crs.identify_with(factory),
            Self::Bound(crs) => crs.identify_with(factory),
            _ => Vec::new(),
        }
    }

    /// `isEquivalentTo` with `EQUIVALENT` and a database for datum aliases.
    ///
    /// The CRS name is not required to match. Axis comparison is direction and
    /// unit. A geodetic datum name may match an alias, including an Esri `D_`
    /// prefix.
    #[must_use]
    pub fn is_equivalent_to_db(&self, other: &Self, db: &DatabaseContext) -> bool {
        let (Some(left), Some(right)) = (as_geodetic(self), as_geodetic(other)) else {
            return false;
        };
        db_equivalent(&left, &right, IdentifyAxisOrder::Exact, db)
    }
}

fn as_geodetic(crs: &Crs) -> Option<GeodeticCRS> {
    match crs {
        Crs::Geographic(crs) => Some(GeodeticCRS::Geographic(crs.clone())),
        Crs::Geodetic(crs) => Some(crs.clone()),
        _ => None,
    }
}

fn to_crs(crs: GeodeticCRS) -> Crs {
    match crs {
        GeodeticCRS::Geographic(crs) => Crs::Geographic(crs),
        other => Crs::Geodetic(other),
    }
}

fn axis_order(crs: &GeodeticCRS) -> IdentifyAxisOrder {
    match crs {
        GeodeticCRS::Geographic(crs) if crs.implicit_cs => IdentifyAxisOrder::Implicit,
        _ => IdentifyAxisOrder::Exact,
    }
}

fn geodetic_type(crs: &GeodeticCRS) -> &'static str {
    match crs {
        GeodeticCRS::Geocentric(_) => "geocentric",
        GeodeticCRS::Geographic(crs) if crs.coordinate_system().axes().len() == 2 => {
            "geographic 2D"
        }
        GeodeticCRS::Geographic(_) => "geographic 3D",
        GeodeticCRS::Spherical(_) => "",
    }
}

fn insignificant(name: &str) -> bool {
    name.is_empty() || name.eq_ignore_ascii_case("unknown") || name.eq_ignore_ascii_case("unnamed")
}

impl VerticalCRS {
    /// `VerticalCRS::identify`: the CRS of the factory's authority named by one of its
    /// identifiers, else the vertical CRS of the same name, with the confidence of PROJ.
    #[must_use]
    pub fn identify_with(&self, factory: Option<&AuthorityFactory>) -> Vec<(Crs, i32)> {
        let Some(factory) = factory else {
            return Vec::new();
        };
        let db = factory.database_context();
        if compatible_authority(self.identifiers(), factory) {
            for identifier in self.identifiers() {
                if !(factory.authority().is_empty()
                    || identifier.code_space() == factory.authority())
                {
                    continue;
                }
                let looked = AuthorityFactory::create(db, identifier.code_space())
                    .create_vertical_crs(identifier.code());
                if let Ok(found) = looked {
                    let confidence = if vertical_crs_equivalent(self, &found) {
                        100
                    } else {
                        25
                    };
                    return vec![(Crs::Vertical(found), confidence)];
                }
            }
            return Vec::new();
        }
        let name = self.name();
        if name.is_empty()
            || name.eq_ignore_ascii_case("unknown")
            || name.eq_ignore_ascii_case("unnamed")
        {
            return Vec::new();
        }
        for mode in [NameMatch::Exact, NameMatch::Approximate] {
            let Ok(objects) = factory.create_vertical_crs_by_name(name, mode) else {
                return Vec::new();
            };
            let mut hits: Vec<(VerticalCRS, i32)> = Vec::new();
            for found in objects {
                if vertical_crs_equivalent(self, &found) {
                    if found.name() == name {
                        return vec![(Crs::Vertical(found), 100)];
                    }
                    hits.push((found, 90));
                } else {
                    hits.push((found, 25));
                }
            }
            if !hits.is_empty() {
                hits.sort_by_key(|hit| std::cmp::Reverse(hit.1));
                return hits
                    .into_iter()
                    .map(|(crs, c)| (Crs::Vertical(crs), c))
                    .collect();
            }
        }
        Vec::new()
    }
}

impl CompoundCRS {
    /// `CompoundCRS::identify` (crs.cpp:5595).
    ///
    /// The compound CRS of the factory's authority named by one of its identifiers. Else
    /// the compound CRS of the same name, by equivalence of the components. Else the
    /// compound CRS of the factory that has the same components, when the name is
    /// unknown. Else, when the CRS has no identifier and two components, the compound of
    /// the horizontal and vertical components, each identified at 70 or more.
    #[must_use]
    pub fn identify_with(&self, factory: Option<&AuthorityFactory>) -> Vec<(Crs, i32)> {
        let Some(factory) = factory else {
            return Vec::new();
        };
        let db = factory.database_context();
        let this_name = self.name();
        let components = self.components();
        let criterion = if has_implicit_cs(&components[0])
            || matches!(&components[0], Crs::Projected(projected)
                if has_implicit_cs(&Crs::Geodetic(projected.base_crs().clone())))
        {
            Equivalence::ExceptAxisOrder
        } else {
            Equivalence::Equivalent
        };
        let insignificant_name = insignificant(this_name);
        let mut found_equivalent_name = false;
        let mut res: Vec<(CompoundCRS, i32)> = Vec::new();
        if compatible_authority(self.identifiers(), factory) {
            for identifier in self.identifiers() {
                if !(factory.authority().is_empty()
                    || identifier.code_space() == factory.authority())
                {
                    continue;
                }
                let looked = AuthorityFactory::create(db, identifier.code_space())
                    .create_compound_crs(identifier.code());
                if let Ok(found) = looked {
                    let matched = self.is_equivalent_to_with(&found, criterion);
                    return vec![(Crs::Compound(found), if matched { 100 } else { 25 })];
                }
            }
        } else if !insignificant_name {
            for approximate in [false, true] {
                let mode = if approximate {
                    NameMatch::Approximate
                } else {
                    NameMatch::Exact
                };
                let Ok(objects) = factory.create_compound_crs_by_name(this_name, mode) else {
                    break;
                };
                for found in objects {
                    let equivalent_name_match =
                        equivalent_name_bigger_differences(this_name, found.name());
                    found_equivalent_name |= equivalent_name_match;
                    if self.is_equivalent_to_with(&found, criterion) {
                        if found.name() == this_name {
                            return vec![(Crs::Compound(found), 100)];
                        }
                        let confidence = if equivalent_name_match { 90 } else { 70 };
                        res.push((found, confidence));
                    } else {
                        res.push((found, 25));
                    }
                }
                if !res.is_empty() {
                    break;
                }
            }
        }
        sort_compound_hits(this_name, &mut res);
        if self.identifiers().is_empty()
            && !found_equivalent_name
            && res.first().is_none_or(|(_, confidence)| *confidence < 50)
        {
            let mut already_known: BTreeSet<(String, String)> = BTreeSet::new();
            for (found, _) in &res {
                if let Some(identifier) = found.identifiers().first() {
                    already_known.insert((
                        identifier.code_space().to_owned(),
                        identifier.code().to_owned(),
                    ));
                }
            }
            for candidate in factory.create_compound_crs_from_existing(self) {
                if let Some(identifier) = candidate.identifiers().first() {
                    let key = (
                        identifier.code_space().to_owned(),
                        identifier.code().to_owned(),
                    );
                    if already_known.contains(&key) {
                        continue;
                    }
                }
                let confidence = if self.is_equivalent_to_with(&candidate, criterion) {
                    if insignificant_name {
                        90
                    } else {
                        70
                    }
                } else {
                    25
                };
                res.push((candidate, confidence));
            }
            sort_compound_hits(this_name, &mut res);
            // A single candidate at 90 with the same name is promoted to 100.
            if let [(found, confidence)] = res.as_mut_slice() {
                if *confidence == 90 && found.name() == this_name {
                    *confidence = 100;
                }
            }
        }
        // Without an identifier and with no match, a horizontal and a vertical CRS that
        // are themselves identified are combined into a compound CRS.
        if self.identifiers().is_empty() && res.is_empty() && components.len() == 2 {
            let horizontal = components[0].identify_with(Some(factory));
            let vertical = components[1].identify_with(Some(factory));
            if let (
                [(horizontal_crs, horizontal_confidence)],
                [(vertical_crs, vertical_confidence)],
            ) = (horizontal.as_slice(), vertical.as_slice())
            {
                if *horizontal_confidence >= 70 && *vertical_confidence >= 70 {
                    let name = format!("{} + {}", horizontal_crs.name(), vertical_crs.name());
                    if let Ok(new_crs) = CompoundCRS::create(
                        name.clone(),
                        vec![horizontal_crs.clone(), vertical_crs.clone()],
                    ) {
                        let base = if this_name == name {
                            100
                        } else if equivalent_name_bigger_differences(this_name, &name) {
                            90
                        } else {
                            70
                        };
                        res.push((
                            new_crs,
                            base.min(*horizontal_confidence.min(vertical_confidence)),
                        ));
                    }
                }
            }
        }
        // Keep only the results of the highest confidence.
        if res.len() >= 2 {
            let highest = res[0].1;
            res.retain(|(_, confidence)| *confidence == highest);
        }
        res.into_iter()
            .map(|(crs, confidence)| (Crs::Compound(crs), confidence))
            .collect()
    }
}

impl BoundCRS {
    /// `BoundCRS::_identify` (crs.cpp:6236).
    ///
    /// Only a bound CRS whose hub is EPSG:4326 is identified. Each identified base CRS is
    /// bound to that hub through the operation that the database has from it to EPSG:4326,
    /// when the operation has the transformation of this CRS. A match at confidence 70 or
    /// more keeps the transformation of the database, else the transformation of this CRS.
    #[must_use]
    pub fn identify_with(&self, factory: Option<&AuthorityFactory>) -> Vec<(Crs, i32)> {
        let Some(factory) = factory else {
            return Vec::new();
        };
        let db = factory.database_context();
        let hub = self.target_crs();
        let wgs84 = Crs::Geographic(GeographicCRS::epsg_4326());
        let mut res: Vec<(Crs, i32)> = Vec::new();
        let mut res_match_transf: Vec<(Crs, i32)> = Vec::new();
        if !hub.is_equivalent_to_with(&wgs84, Equivalence::Equivalent) {
            return res;
        }
        let transformation = self.transformation();
        let ref_transf = transformation
            .normalize_for_visualization()
            .unwrap_or_else(|| transformation.clone());
        let ref_transf_text = identify_proj_string(&ref_transf);
        let ref_is_null_transform = hub_is_wgs84(hub)
            && matches!(transformation, CoordinateOperation::Transformation(transf)
                if transf.towgs84_strict().ok().flatten() == Some([0.0; 7]));
        for (candidate, confidence) in self.source_crs().identify_with(Some(factory)) {
            let geodetic = match &candidate {
                Crs::Projected(projected) => Some(projected.base_crs().clone()),
                Crs::Geographic(crs) => Some(GeodeticCRS::Geographic(crs.clone())),
                Crs::Geodetic(crs) => Some(crs.clone()),
                _ => None,
            };
            let Some(geodetic) = geodetic else {
                continue;
            };
            let mut context = CoordinateOperationContext::create(db, factory.authority());
            context.set_spatial_criterion(SpatialCriterion::PartialIntersection);
            let ops = CoordinateOperationFactory::create()
                .create_operations(&to_crs(geodetic), &wgs84, &context)
                .unwrap_or_default();
            let mut found_op = false;
            for op in ops {
                let op_normalized = op
                    .normalize_for_visualization()
                    .unwrap_or_else(|| op.clone());
                let op_name = op.name();
                if op_name.starts_with(BALLPARK_GEOCENTRIC_TRANSLATION)
                    || op_name.starts_with(NULL_GEOGRAPHIC_OFFSET)
                {
                    if ref_is_null_transform {
                        res.push((to_bound(&candidate, hub, transformation), confidence));
                        found_op = true;
                        break;
                    }
                    continue;
                }
                let op_text = identify_proj_string(&op_normalized);
                let same_text = matches!(
                    (&ref_transf_text, &op_text),
                    (Some(reference), Some(text)) if reference == text
                );
                if same_text
                    || op_normalized.is_equivalent_to_with(&ref_transf, Equivalence::Equivalent)
                {
                    if let CoordinateOperation::Transformation(transf) = &op {
                        res_match_transf.push((
                            to_bound_with(
                                &candidate,
                                hub,
                                CoordinateOperation::Transformation(transf.clone()),
                            ),
                            confidence,
                        ));
                        found_op = true;
                        break;
                    }
                    if let CoordinateOperation::Concatenated(concatenated) = &op {
                        let steps = concatenated.steps();
                        if steps.len() == 2 {
                            let first = effective_operation(&steps[0]);
                            let first_is_longitude_rotation = matches!(
                                &first,
                                CoordinateOperation::Transformation(transf)
                                    if transf.method().code() == "9601"
                            );
                            let first_is_conversion =
                                matches!(&first, CoordinateOperation::Conversion(_));
                            let candidate_is_derived =
                                matches!(candidate, Crs::Projected(_) | Crs::DerivedProjected(_));
                            if first_is_longitude_rotation
                                || (candidate_is_derived && first_is_conversion)
                            {
                                if let CoordinateOperation::Transformation(transf) =
                                    effective_operation(&steps[1])
                                {
                                    if !transf.name().starts_with("Ballpark geo") {
                                        res_match_transf.push((
                                            to_bound_with(
                                                &candidate,
                                                hub,
                                                CoordinateOperation::Transformation(transf),
                                            ),
                                            confidence,
                                        ));
                                        found_op = true;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if !found_op {
                res.push((
                    to_bound(&candidate, hub, transformation),
                    confidence.min(70),
                ));
            }
        }
        if res_match_transf.is_empty() {
            res
        } else {
            res_match_transf
        }
    }
}

/// `BoundCRS::create(candidate, hub, transformation)` as `_identify` builds the results.
fn to_bound(source: &Crs, hub: &Crs, transformation: &CoordinateOperation) -> Crs {
    to_bound_with(source, hub, transformation.clone())
}

fn to_bound_with(source: &Crs, hub: &Crs, transformation: CoordinateOperation) -> Crs {
    Crs::Bound(BoundCRS::create(
        source.clone(),
        hub.clone(),
        transformation,
    ))
}

/// A step of a concatenated operation as PROJ holds it: the operation, inverted when the
/// step runs in reverse.
fn effective_operation(step: &ConcatenatedStep) -> CoordinateOperation {
    if step.is_reverse() {
        step.operation().inverse()
    } else {
        step.operation().clone()
    }
}

/// `exportToPROJString(PROJStringFormatter::create())` of an operation, with the zero
/// position-vector rotation removed as `BoundCRS::_identify` does. `None` when PROJ would
/// throw.
fn identify_proj_string(operation: &CoordinateOperation) -> Option<String> {
    let text = match operation {
        CoordinateOperation::Transformation(transf) => {
            transf.export_to_proj_string(ProjLines::Single).ok()?
        }
        CoordinateOperation::Concatenated(concatenated) => {
            concatenated.export_to_proj_string(ProjLines::Single).ok()?
        }
        CoordinateOperation::Conversion(_) => return None,
    };
    Some(text.replace(" +rx=0 +ry=0 +rz=0 +s=0 +convention=position_vector", ""))
}

/// `CRS::hasImplicitCS` for the CRS kinds that carry the flag in this port.
fn has_implicit_cs(crs: &Crs) -> bool {
    match crs {
        Crs::Geographic(crs) | Crs::Geodetic(GeodeticCRS::Geographic(crs)) => crs.implicit_cs,
        _ => false,
    }
}

/// `lambdaSort` of `CompoundCRS::identify`: confidence first, then the exact name of the
/// CRS, then the name in byte order.
fn sort_compound_hits(this_name: &str, hits: &mut [(CompoundCRS, i32)]) {
    hits.sort_by(|(left, left_confidence), (right, right_confidence)| {
        right_confidence
            .cmp(left_confidence)
            .then_with(|| prefer(left.name() == this_name, right.name() == this_name))
            .then_with(|| left.name().cmp(right.name()))
    });
}

fn compatible_authority(identifiers: &[Identifier], factory: &AuthorityFactory) -> bool {
    if identifiers.is_empty() {
        return false;
    }
    factory.authority().is_empty()
        || identifiers
            .iter()
            .any(|identifier| identifier.code_space() == factory.authority())
}

fn hardcoded_match(
    crs: &GeodeticCRS,
    order: IdentifyAxisOrder,
    db: &DatabaseContext,
) -> Option<(Crs, i32)> {
    let candidates = [
        GeodeticCRS::Geographic(GeographicCRS::epsg_4326()),
        GeodeticCRS::Geographic(GeographicCRS::epsg_4267()),
        GeodeticCRS::Geographic(GeographicCRS::epsg_4269()),
    ];
    for candidate in candidates {
        let name_equal = crs.name() == candidate.name();
        let equivalent = db_equivalent(crs, &candidate, order, db);
        if equivalent_name_bigger_differences(crs.name(), candidate.name())
            && equivalent
            && name_equal
        {
            return Some((to_crs(candidate), 100));
        }
    }
    None
}

fn identify_by_code(
    crs: &GeodeticCRS,
    factory: &AuthorityFactory,
    db: &DatabaseContext,
    order: IdentifyAxisOrder,
) -> Option<(Crs, i32)> {
    for identifier in crs.identifiers() {
        if !(factory.authority().is_empty() || identifier.code_space() == factory.authority()) {
            continue;
        }
        let looked = AuthorityFactory::create(db, identifier.code_space())
            .create_geodetic_crs(identifier.code());
        if let Ok(found) = looked {
            let confidence = if db_equivalent(crs, &found, order, db) {
                100
            } else {
                25
            };
            return Some((to_crs(found), confidence));
        }
    }
    None
}

fn search_by_datum_or_ellipsoid(
    crs: &GeodeticCRS,
    factory: &AuthorityFactory,
    db: &DatabaseContext,
    order: IdentifyAxisOrder,
    crs_type: &str,
    hits: &mut Vec<Hit>,
) {
    let Some(frame) = owning_frame(crs) else {
        return;
    };
    if !frame.identifiers().is_empty() {
        search_by_datum_code(crs, &frame, factory, db, order, crs_type, hits);
        return;
    }
    let mut modified = false;
    if let Ok(datums) = factory.create_geodetic_datums_by_name(frame.name()) {
        for datum in datums {
            if search_by_datum_code(crs, &datum, factory, db, order, crs_type, hits) {
                modified = true;
            }
        }
    }
    if !modified {
        search_by_ellipsoid(crs, &frame, factory, db, order, crs_type, hits);
    }
}

fn search_by_datum_code(
    crs: &GeodeticCRS,
    datum: &GeodeticReferenceFrame,
    factory: &AuthorityFactory,
    db: &DatabaseContext,
    order: IdentifyAxisOrder,
    crs_type: &str,
    hits: &mut Vec<Hit>,
) -> bool {
    let mut modified = false;
    for identifier in datum.identifiers() {
        let Ok(found) = factory.create_geodetic_crs_from_datum(
            identifier.code_space(),
            identifier.code(),
            crs_type,
        ) else {
            continue;
        };
        for candidate in found {
            if db_equivalent(crs, &candidate, order, db) {
                hits.push(Hit {
                    crs: candidate,
                    confidence: 70,
                });
                modified = true;
            }
        }
    }
    modified
}

fn search_by_ellipsoid(
    crs: &GeodeticCRS,
    frame: &GeodeticReferenceFrame,
    factory: &AuthorityFactory,
    _db: &DatabaseContext,
    order: IdentifyAxisOrder,
    crs_type: &str,
    hits: &mut Vec<Hit>,
) {
    let ellipsoids = if frame.ellipsoid().identifiers().is_empty() {
        factory
            .create_ellipsoid_from_existing(frame.ellipsoid())
            .unwrap_or_default()
    } else {
        vec![frame.ellipsoid().clone()]
    };
    let this_axes = axis_list(crs);
    for ellipsoid in &ellipsoids {
        for identifier in ellipsoid.identifiers() {
            let Ok(found) = factory.create_geodetic_crs_from_ellipsoid(
                identifier.code_space(),
                identifier.code(),
                crs_type,
            ) else {
                continue;
            };
            for candidate in found {
                let Some(candidate_frame) = owning_frame(&candidate) else {
                    continue;
                };
                let ellipsoid_matches =
                    ellipsoid_parameters_match(candidate_frame.ellipsoid(), ellipsoid);
                let meridian_matches =
                    meridian_equivalent(candidate_frame.prime_meridian(), frame.prime_meridian());
                let axes_match = order == IdentifyAxisOrder::Implicit
                    || directions_and_units(&this_axes, &axis_list(&candidate));
                if ellipsoid_matches && meridian_matches && axes_match {
                    hits.push(Hit {
                        crs: candidate,
                        confidence: 60,
                    });
                }
            }
        }
    }
}

fn sort_hits(crs: &GeodeticCRS, db: &DatabaseContext, hits: &mut [Hit]) {
    let frame = owning_frame(crs);
    let axes = axis_list(crs);
    let name = crs.name();
    hits.sort_by(|left, right| {
        right
            .confidence
            .cmp(&left.confidence)
            .then_with(|| prefer(left.crs.name() == name, right.crs.name() == name))
            .then_with(|| {
                prefer(
                    frame_matches(frame.as_ref(), &left.crs, db),
                    frame_matches(frame.as_ref(), &right.crs, db),
                )
            })
            .then_with(|| {
                prefer(
                    directions_and_units(&axes, &axis_list(&left.crs)),
                    directions_and_units(&axes, &axis_list(&right.crs)),
                )
            })
            .then_with(|| {
                prefer(
                    axis_list(&left.crs).len() == axes.len(),
                    axis_list(&right.crs).len() == axes.len(),
                )
            })
            .then_with(|| {
                prefer(
                    ellipsoid_name_is_crs_name(&left.crs),
                    ellipsoid_name_is_crs_name(&right.crs),
                )
            })
            .then_with(|| left.crs.name().cmp(right.crs.name()))
    });
}

#[expect(
    clippy::fn_params_excessive_bools,
    reason = "the two flags are the left and right sides of one comparison"
)]
fn prefer(left_wins: bool, right_wins: bool) -> std::cmp::Ordering {
    match (left_wins, right_wins) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => std::cmp::Ordering::Equal,
    }
}

fn frame_matches(
    frame: Option<&GeodeticReferenceFrame>,
    candidate: &GeodeticCRS,
    db: &DatabaseContext,
) -> bool {
    match (frame, owning_frame(candidate)) {
        (Some(frame), Some(other)) => frames_match(frame, &other, db),
        _ => false,
    }
}

fn ellipsoid_name_is_crs_name(crs: &GeodeticCRS) -> bool {
    owning_frame(crs).is_some_and(|frame| {
        equivalent_name_bigger_differences(frame.ellipsoid().name(), crs.name())
    })
}

fn db_equivalent(
    crs: &GeodeticCRS,
    candidate: &GeodeticCRS,
    order: IdentifyAxisOrder,
    db: &DatabaseContext,
) -> bool {
    match (owning_frame(crs), owning_frame(candidate)) {
        (Some(left), Some(right)) => {
            frames_match(&left, &right, db) && identify_axes_match(crs, candidate, order)
        }
        _ => false,
    }
}

fn frames_match(
    left: &GeodeticReferenceFrame,
    right: &GeodeticReferenceFrame,
    db: &DatabaseContext,
) -> bool {
    datum_names_match(left, right, db)
        && same_epoch(left.frame_reference_epoch(), right.frame_reference_epoch())
        && ellipsoid_parameters_match(left.ellipsoid(), right.ellipsoid())
        && meridian_equivalent(left.prime_meridian(), right.prime_meridian())
}

fn datum_names_match(
    left: &GeodeticReferenceFrame,
    right: &GeodeticReferenceFrame,
    db: &DatabaseContext,
) -> bool {
    let left_prefixed = has_d_prefix(left.name());
    let right_prefixed = has_d_prefix(right.name());
    if left_prefixed && !right_prefixed {
        let stripped = &left.name()[2..];
        return equivalent_name_bigger_differences(stripped, right.name())
            || names_alias(left, stripped, right, right.name(), db);
    }
    if right_prefixed && !left_prefixed {
        let stripped = &right.name()[2..];
        return equivalent_name_bigger_differences(left.name(), stripped)
            || names_alias(left, left.name(), right, stripped, db);
    }
    names_alias(left, left.name(), right, right.name(), db)
}

fn names_alias(
    left: &GeodeticReferenceFrame,
    left_name: &str,
    right: &GeodeticReferenceFrame,
    right_name: &str,
    db: &DatabaseContext,
) -> bool {
    if left_name == right_name || left_name == "unknown" || right_name == "unknown" {
        return true;
    }
    if starts_unknown_based(left_name) || starts_unknown_based(right_name) {
        return false;
    }
    if !left.identifiers().is_empty() && alias_from_identifier(left, left_name, right_name, db) {
        return true;
    }
    if !right.identifiers().is_empty() && alias_from_identifier(right, right_name, left_name, db) {
        return true;
    }
    alias_from_name(left_name, right_name, db) || alias_from_name(right_name, left_name, db)
}

fn alias_from_identifier(
    frame: &GeodeticReferenceFrame,
    this_name: &str,
    other_name: &str,
    db: &DatabaseContext,
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
        (!official.is_empty() && equivalent_name_bigger_differences(name, &official))
            || aliases
                .iter()
                .any(|alias| equivalent_name_bigger_differences(name, alias))
    };
    matches(this_name) && matches(other_name)
}

fn alias_from_name(this_name: &str, other_name: &str, db: &DatabaseContext) -> bool {
    db.get_aliases("", "", this_name, "geodetic_datum", "")
        .iter()
        .any(|alias| equivalent_name_bigger_differences(other_name, alias))
}

fn has_d_prefix(name: &str) -> bool {
    name.len() >= 2 && name[..2].eq_ignore_ascii_case("D_")
}

fn starts_unknown_based(name: &str) -> bool {
    name.len() >= UNKNOWN_BASED_ON.len()
        && name[..UNKNOWN_BASED_ON.len()].eq_ignore_ascii_case(UNKNOWN_BASED_ON)
}

fn owning_frame(crs: &GeodeticCRS) -> Option<GeodeticReferenceFrame> {
    if let Some(datum) = crs.datum() {
        return Some(datum.clone());
    }
    match crs.datum_ensemble().and_then(DatumEnsemble::as_datum) {
        Some(Datum::Geodetic(frame)) => Some(frame),
        _ => None,
    }
}

fn axis_list(crs: &GeodeticCRS) -> Vec<Axis> {
    crs.coordinate_system()
        .axes()
        .into_iter()
        .cloned()
        .collect()
}

impl GeographicCRS {
    /// `GeographicCRS::promoteTo3D`.
    ///
    /// With a database and one identifier, the same-name geographic 3D CRS is
    /// returned when its height axis is up in metres and this CRS is its 2D part.
    /// Otherwise an ellipsoidal-height axis is added and the identifier is cleared.
    #[must_use]
    pub fn promote_to_3d_with_db(&self, db: Option<&DatabaseContext>) -> Self {
        if self.coordinate_system.axes.len() != 2 {
            return self.clone();
        }
        if let Some(db) = db {
            if self.identifiers().len() == 1 {
                let identifier = &self.identifiers()[0];
                if let Ok(Some(found)) = AuthorityFactory::create(db, identifier.code_space())
                    .create_geographic_3d_crs_from_name(self.name())
                {
                    let height_matches =
                        found.coordinate_system().axes().get(2).is_some_and(|axis| {
                            axis.direction() == "up"
                                && axis.unit().kind() == super::model::UnitKind::Linear
                                && (axis.unit().conversion_to_si() - 1.0).abs() <= 1e-10
                        });
                    if height_matches && self.is_2d_part_of_3d(&found) {
                        return found;
                    }
                }
            }
        }
        let mut crs = self.promote_to_3d();
        crs.identified = promoted_properties(&self.identified);
        crs
    }

    /// `CRS::promoteTo3D(newName, dbContext, verticalAxisIfNotAlreadyPresent)` (crs.cpp:
    /// 1341-1459) with the height axis `axis`, or the ellipsoidal height in metres when `None`.
    ///
    /// With a database and one identifier, the same-name geographic 3D CRS is returned when its
    /// height axis is equivalent to `axis` and this CRS is its 2D part. Otherwise `axis` is added
    /// and the identifier is cleared.
    #[must_use]
    pub fn promote_to_3d_with_axis(
        &self,
        db: Option<&DatabaseContext>,
        axis: Option<&Axis>,
    ) -> Self {
        let Some(axis) = axis else {
            return self.promote_to_3d_with_db(db);
        };
        if self.coordinate_system.axes.len() != 2 {
            return self.clone();
        }
        if let Some(db) = db {
            if self.identifiers().len() == 1 {
                let identifier = &self.identifiers()[0];
                if let Ok(Some(found)) = AuthorityFactory::create(db, identifier.code_space())
                    .create_geographic_3d_crs_from_name(self.name())
                {
                    let height_matches =
                        found
                            .coordinate_system()
                            .axes()
                            .get(2)
                            .is_some_and(|height| {
                                height.direction().eq_ignore_ascii_case(axis.direction())
                                    && height.unit().kind() == axis.unit().kind()
                                    && (height.unit().conversion_to_si()
                                        - axis.unit().conversion_to_si())
                                    .abs()
                                        <= 1e-10
                            });
                    if height_matches && self.is_2d_part_of_3d(&found) {
                        return found;
                    }
                }
            }
        }
        let mut crs = self.promote_to_3d();
        if let Some(height) = crs.coordinate_system.axes.get_mut(2) {
            *height = axis.clone();
        }
        crs.identified = promoted_properties(&self.identified);
        crs
    }

    /// `GeographicCRS::demoteTo2D`.
    ///
    /// A database with one identifier returns the same-name geographic 2D CRS
    /// when it is the horizontal part of this CRS.
    #[must_use]
    pub fn demote_to_2d(&self, db: Option<&DatabaseContext>) -> Self {
        if self.coordinate_system.axes.len() != 3 {
            return self.clone();
        }
        if let Some(db) = db {
            if self.identifiers().len() == 1 {
                let identifier = &self.identifiers()[0];
                if let Ok(Some(found)) = AuthorityFactory::create(db, identifier.code_space())
                    .create_geographic_2d_crs_from_name(self.name())
                {
                    if found.is_2d_part_of_3d(self) {
                        return found;
                    }
                }
            }
        }
        let mut crs = self.clone();
        crs.coordinate_system.axes.truncate(2);
        crs.identified.identifiers.clear();
        crs
    }
}

impl DerivedProjectedCRS {
    /// `DerivedProjectedCRS::promoteTo3D`.
    ///
    /// Adds an ellipsoidal-height axis in the unit of the first axis and promotes the
    /// projected base. A CRS that is not a 2D Cartesian system is returned unchanged.
    #[must_use]
    pub fn promote_to_3d(&self, db: Option<&DatabaseContext>) -> Self {
        let CoordinateSystem::Cartesian(cs) = &self.coordinate_system else {
            return self.clone();
        };
        if cs.axes.len() != 2 {
            return self.clone();
        }
        let unit = cs.axes[0].unit().clone();
        let mut coordinate_system = cs.clone();
        coordinate_system
            .axes
            .push(Axis::new("Ellipsoidal height", "h", "up", unit));
        Self {
            identified: promoted_properties(&self.identified),
            base: self.base.promote_to_3d(db),
            conversion: self.conversion.clone(),
            coordinate_system: CoordinateSystem::Cartesian(coordinate_system),
        }
    }
}

impl ProjectedCRS {
    /// `ProjectedCRS::promoteTo3D`.
    ///
    /// The new axis is ellipsoidal height, upwards, in the horizontal unit.
    /// The base geographic CRS is promoted with `db`. Identifiers are not kept.
    #[must_use]
    pub fn promote_to_3d(&self, db: Option<&DatabaseContext>) -> Self {
        let base = match &self.base {
            GeodeticCRS::Geographic(crs) => GeodeticCRS::Geographic(crs.promote_to_3d_with_db(db)),
            other => other.clone(),
        };
        self.promote_with_base(base)
    }

    /// `ProjectedCRS::promoteTo3D` with an explicit geographic 3D base CRS.
    #[must_use]
    pub fn promote_to_3d_on(&self, base: &GeographicCRS) -> Self {
        self.promote_with_base(GeodeticCRS::Geographic(base.clone()))
    }

    fn promote_with_base(&self, base: GeodeticCRS) -> Self {
        if self.coordinate_system.axes.len() != 2 {
            return self.clone();
        }
        let unit = self.coordinate_system.axes[0].unit().clone();
        let mut coordinate_system = self.coordinate_system.clone();
        coordinate_system
            .axes
            .push(Axis::new("Ellipsoidal height", "h", "up", unit));
        Self {
            identified: promoted_properties(&self.identified),
            base,
            conversion: self.conversion.clone(),
            coordinate_system,
            implicit_cs: self.implicit_cs,
        }
    }

    /// `ProjectedCRS::demoteTo2D`.
    ///
    /// Drops the third axis and demotes a geographic base. The result has the
    /// same name and no identifier.
    #[must_use]
    pub fn demote_to_2d(&self, db: Option<&DatabaseContext>) -> Self {
        if self.coordinate_system.axes.len() != 3 {
            return self.clone();
        }
        let base = match &self.base {
            GeodeticCRS::Geographic(crs) => GeodeticCRS::Geographic(crs.demote_to_2d(db)),
            other => other.clone(),
        };
        let mut coordinate_system = self.coordinate_system.clone();
        coordinate_system.axes.truncate(2);
        Self {
            identified: Identified::named(self.name()),
            base,
            conversion: self.conversion.clone(),
            coordinate_system,
            implicit_cs: self.implicit_cs,
        }
    }
}

impl ProjectedCRS {
    /// `ProjectedCRS::identify` (crs.cpp:4858).
    ///
    /// A UTM zone on WGS 84, NAD27, or NAD83 is named by its hardcoded zone CRS. Otherwise,
    /// with a factory, the CRS of the factory's authority named by one of its identifiers is
    /// taken when equivalent; without one, the projected CRS of the same name, then the
    /// projected CRS that the base CRS, the conversion, and the axes describe
    /// (`createProjectedCRSFromExisting`). The results of the highest confidence are kept.
    #[must_use]
    pub fn identify_with(&self, factory: Option<&AuthorityFactory>) -> Vec<(Crs, i32)> {
        let base = identify_base(self.base_crs(), factory);
        if let Some(found) = identify_utm_zone(self, &base, factory) {
            return found;
        }
        identify_projected_general(self, factory)
            .into_iter()
            .map(|(crs, confidence)| (Crs::Projected(crs), confidence))
            .collect()
    }
}

/// The base CRS of `ProjectedCRS::identify` identified (crs.cpp:4858-4871). A geographic base
/// in longitude/latitude order is identified in latitude/longitude order.
fn identify_base(base: &GeodeticCRS, factory: Option<&AuthorityFactory>) -> Vec<(Crs, i32)> {
    let swapped = match base {
        GeodeticCRS::Geographic(crs) => latitude_longitude_base(crs),
        _ => None,
    };
    match swapped {
        Some(crs) => Crs::Geographic(crs).identify_with(factory),
        None => Crs::Geodetic(base.clone()).identify_with(factory),
    }
}

/// The UTM branch of `ProjectedCRS::identify` (crs.cpp:4882-4935): a zone on WGS 84, NAD27, or
/// NAD83 is the hardcoded CRS of that zone, with the conversion and axes of `source`. `None`
/// when the branch does not apply, and the general search runs.
fn identify_utm_zone(
    source: &ProjectedCRS,
    base: &[(Crs, i32)],
    factory: Option<&AuthorityFactory>,
) -> Option<Vec<(Crs, i32)>> {
    if !factory
        .is_none_or(|factory| factory.authority().is_empty() || factory.authority() == "EPSG")
    {
        return None;
    }
    let (zone, hemisphere) = source.conversion.utm_zone()?;
    let [(Crs::Geographic(base_crs), confidence)] = base else {
        return None;
    };
    if *confidence < 70 || !easting_northing_metre(&source.coordinate_system) {
        return None;
    }
    let base_crs = GeodeticCRS::Geographic(base_crs.clone());
    let north = hemisphere == UtmHemisphere::North;
    let equivalent = |other: GeographicCRS| {
        base_crs.is_equivalent_to_with(&GeodeticCRS::Geographic(other), Equivalence::Equivalent)
    };
    let (prefix, base_epsg, code) = if equivalent(GeographicCRS::epsg_4326()) {
        let base_code = if north { 32_600 } else { 32_700 };
        let code = base_code + zone;
        ("WGS 84 / UTM zone ", GeographicCRS::epsg_4326(), code)
    } else if ((1..=22).contains(&zone) || zone == 59 || zone == 60)
        && north
        && equivalent(GeographicCRS::epsg_4267())
    {
        let code = if zone >= 59 {
            3370 + zone - 59
        } else {
            26_700 + zone
        };
        ("NAD27 / UTM zone ", GeographicCRS::epsg_4267(), code)
    } else if ((1..=23).contains(&zone) || zone == 59 || zone == 60)
        && north
        && equivalent(GeographicCRS::epsg_4269())
    {
        let code = if zone >= 59 {
            3372 + zone - 59
        } else {
            26_900 + zone
        };
        ("NAD83 / UTM zone ", GeographicCRS::epsg_4269(), code)
    } else {
        return None;
    };
    let name = format!("{prefix}{zone}{}", if north { "N" } else { "S" });
    let crs = ProjectedCRS::create(
        name.clone(),
        GeodeticCRS::Geographic(base_epsg),
        source.conversion.clone(),
        source.coordinate_system.clone(),
    )
    .with_identifier("EPSG", &code.to_string());
    let confidence = if name == source.name() {
        100
    } else if equivalent_name_bigger_differences(&name, source.name()) {
        90
    } else {
        70
    };
    Some(vec![(Crs::Projected(crs), confidence)])
}

/// The general search of `ProjectedCRS::identify` (crs.cpp:4937-5180). Without a factory there
/// is no candidate.
fn identify_projected_general(
    source: &ProjectedCRS,
    factory: Option<&AuthorityFactory>,
) -> Vec<(ProjectedCRS, i32)> {
    let Some(factory) = factory else {
        return Vec::new();
    };
    let db = factory.database_context();
    let this_name = source.name();
    let insignificant_name = insignificant(this_name);
    let Some(base_ellipsoid) = source.base_crs().ellipsoid() else {
        return Vec::new();
    };
    let semi_major = base_ellipsoid.semi_major_axis().si_value();
    let mut found_equivalent_name = false;
    let mut has_non_matching_id = false;
    let mut res: Vec<(ProjectedCRS, i32)> = Vec::new();
    if compatible_authority(source.identifiers(), factory) {
        // The CRS already has an identifier: the official object of that identifier, if the
        // database has it, and whether it is equivalent.
        for identifier in source.identifiers() {
            if !(factory.authority().is_empty() || identifier.code_space() == factory.authority()) {
                continue;
            }
            let looked = AuthorityFactory::create(db, identifier.code_space())
                .create_projected_crs(identifier.code());
            if let Ok(found) = looked {
                let matched =
                    source.is_equivalent_to_with_database(&found, Equivalence::ExceptAxisOrder, db);
                res.push((found, if matched { 100 } else { 25 }));
                if matched {
                    return res;
                }
            }
        }
        has_non_matching_id = true;
    } else if !insignificant_name {
        for approximate in [false, true] {
            let mode = if approximate {
                NameMatch::Approximate
            } else {
                NameMatch::Exact
            };
            let Ok(objects) = factory.create_projected_crs_by_name_matched(this_name, mode) else {
                break;
            };
            for (found, matched_name) in objects {
                let eq_name = equivalent_name_bigger_differences(this_name, &matched_name);
                found_equivalent_name |= eq_name;
                let found_semi_major = found
                    .base_crs()
                    .ellipsoid()
                    .map(|ellipsoid| ellipsoid.semi_major_axis().si_value());
                if found_semi_major
                    .is_some_and(|value| (semi_major - value).abs() <= 1e-4 * semi_major)
                    && add_projected_candidate(
                        source,
                        db,
                        &mut res,
                        &found,
                        CandidateFlags {
                            name_equivalent: eq_name,
                            non_matching_id: false,
                        },
                    ) == 100
                {
                    return res;
                }
            }
            if !res.is_empty() {
                break;
            }
        }
    }
    sort_projected_hits(this_name, &mut res);
    if !found_equivalent_name && res.first().is_none_or(|(_, confidence)| *confidence < 50) {
        let mut already_known: BTreeSet<(String, String)> = BTreeSet::new();
        for (found, _) in &res {
            if let Some(identifier) = found.identifiers().first() {
                already_known.insert((
                    identifier.code_space().to_owned(),
                    identifier.code().to_owned(),
                ));
            }
        }
        for candidate in factory.create_projected_crs_from_existing(source) {
            if let Some(identifier) = candidate.identifiers().first() {
                let key = (
                    identifier.code_space().to_owned(),
                    identifier.code().to_owned(),
                );
                if already_known.contains(&key) {
                    continue;
                }
            }
            add_projected_candidate(
                source,
                db,
                &mut res,
                &candidate,
                CandidateFlags {
                    name_equivalent: insignificant_name,
                    non_matching_id: has_non_matching_id,
                },
            );
        }
        sort_projected_hits(this_name, &mut res);
    }
    // Only the results of the highest confidence are kept.
    if res.len() >= 2 {
        let highest = res[0].1;
        res.retain(|(_, confidence)| *confidence == highest);
    }
    res
}

/// What `addCRS` of `ProjectedCRS::identify` knows of a candidate: whether its name is
/// equivalent to the name of the CRS (`eqName`), and whether the CRS has an identifier that the
/// database does not match (`hasNonMatchingId`).
#[derive(Clone, Copy)]
struct CandidateFlags {
    name_equivalent: bool,
    non_matching_id: bool,
}

/// `addCRS` of `ProjectedCRS::identify` (crs.cpp:4960-5012): adds `candidate` to `res` with
/// its confidence, and returns that confidence. A candidate with the name of the CRS replaces
/// the results before it.
fn add_projected_candidate(
    source: &ProjectedCRS,
    db: &DatabaseContext,
    res: &mut Vec<(ProjectedCRS, i32)>,
    candidate: &ProjectedCRS,
    flags: CandidateFlags,
) -> i32 {
    let CandidateFlags {
        name_equivalent: eq_name,
        non_matching_id: has_non_matching_id,
    } = flags;
    let this_name = source.name();
    let source_unit = source.coordinate_system.axes()[0].unit();
    let candidate_unit = candidate.coordinate_system.axes()[0].unit();
    let implicit = source.implicit_cs;
    let unit_match = implicit && same_unit(source_unit, candidate_unit);
    let source_datum = frame_name(source.base_crs());
    let candidate_datum = frame_name(candidate.base_crs());
    let mixed_unknown = (source_datum == "unknown") != (candidate_datum == "unknown");
    let equivalent_except_axis_order =
        source.is_equivalent_to_with_database(candidate, Equivalence::ExceptAxisOrder, db)
            || (unit_match
                && geodetic_equivalent_with_database(
                    source.base_crs(),
                    candidate.base_crs(),
                    Equivalence::ExceptAxisOrder,
                    db,
                )
                && source.deriving_conversion().is_equivalent_to_with(
                    candidate.deriving_conversion(),
                    Equivalence::Equivalent,
                ));
    if equivalent_except_axis_order && !mixed_unknown {
        let confidence = if candidate.name() == this_name {
            res.clear();
            if has_non_matching_id {
                70
            } else {
                100
            }
        } else if eq_name {
            90
        } else {
            70
        };
        res.push((candidate.clone(), confidence));
        return confidence;
    }
    let confidence = match (
        source.base_crs().ellipsoid(),
        candidate.base_crs().ellipsoid(),
    ) {
        (Some(left), Some(right))
            if ellipsoid_parameters_match(left, right)
                && source.deriving_conversion().is_equivalent_to_with(
                    candidate.deriving_conversion(),
                    Equivalence::Equivalent,
                ) =>
        {
            let axes_match = unit_match
                || directions_and_units(
                    source.coordinate_system.axes(),
                    candidate.coordinate_system.axes(),
                );
            if axes_match {
                let significant_datum = !source_datum.to_ascii_lowercase().starts_with("unknown")
                    && source_datum != "unnamed";
                if !significant_datum || datum_equivalent_loose(source, candidate) {
                    70
                } else {
                    60
                }
            } else {
                50
            }
        }
        _ => 25,
    };
    res.push((candidate.clone(), confidence));
    confidence
}

/// `isEquivalentTo` of two geodetic CRS with a database: two geographic CRS may have their
/// datum names aliased in the database.
fn geodetic_equivalent_with_database(
    left: &GeodeticCRS,
    right: &GeodeticCRS,
    criterion: Equivalence,
    db: &DatabaseContext,
) -> bool {
    match (left, right) {
        (GeodeticCRS::Geographic(left), GeodeticCRS::Geographic(right)) => {
            left.is_equivalent_to_with_database(right, criterion, db)
        }
        _ => left.is_equivalent_to_with(right, criterion),
    }
}

/// The datum of a CRS as `datumNonNull` names it (an ensemble is its own datum), `""` when it
/// has none.
fn frame_name(crs: &GeodeticCRS) -> String {
    owning_frame(crs)
        .map(|frame| frame.name().to_owned())
        .unwrap_or_default()
}

/// `IComparable` of the datums of the base CRS of `source` and `candidate` with the
/// `EQUIVALENT` criterion and no database: the names by `isEquivalentName`, the ellipsoid, the
/// epoch, and the prime meridian.
fn datum_equivalent_loose(source: &ProjectedCRS, candidate: &ProjectedCRS) -> bool {
    let (Some(left), Some(right)) = (
        owning_frame(source.base_crs()),
        owning_frame(candidate.base_crs()),
    ) else {
        return false;
    };
    let names_match = equivalent_name_bigger_differences(left.name(), right.name())
        || (left.name() == "unknown" && right.name() == "unnamed")
        || (left.name() == "unnamed" && right.name() == "unknown");
    names_match
        && ellipsoid_parameters_match(left.ellipsoid(), right.ellipsoid())
        && meridian_equivalent(left.prime_meridian(), right.prime_meridian())
        && same_epoch(left.frame_reference_epoch(), right.frame_reference_epoch())
}

/// The sort of `ProjectedCRS::identify` (crs.cpp:5043-5062): confidence first, then the exact
/// name of the CRS, then the name in byte order.
fn sort_projected_hits(this_name: &str, hits: &mut [(ProjectedCRS, i32)]) {
    hits.sort_by(|(left, left_confidence), (right, right_confidence)| {
        right_confidence
            .cmp(left_confidence)
            .then_with(|| prefer(left.name() == this_name, right.name() == this_name))
            .then_with(|| left.name().cmp(right.name()))
    });
}

/// `ProjectedCRS::identify` needs the base CRS with latitude first, as PROJ identifies a
/// longitude/latitude base in latitude/longitude order (crs.cpp:4866). Two dimensions only.
fn latitude_longitude_base(crs: &GeographicCRS) -> Option<GeographicCRS> {
    let axes = crs.coordinate_system().axes();
    if axes.len() >= 2 && axes[0].direction() == "east" && axes[1].direction() == "north" {
        let mut swapped = crs.clone();
        swapped.coordinate_system.axes.swap(0, 1);
        swapped.coordinate_system.axes.truncate(2);
        return Some(swapped);
    }
    None
}

fn easting_northing_metre(cs: &CartesianCS) -> bool {
    let axes = cs.axes();
    axes.len() >= 2
        && axes[0].direction() == "east"
        && axes[1].direction() == "north"
        && axes[0].unit().name() == "metre"
        && (axes[0].unit().conversion_to_si() - 1.0).abs() <= 1e-10
        && axes[1].unit().name() == "metre"
}

/// The properties of a promoted CRS (`CRS::promoteTo3D`, `createProperties`, crs.cpp:1341).
///
/// The promoted CRS keeps its name. Each domain keeps only its extent, without the scope,
/// since PROJ does not guarantee the scope in 3D. It has no identifier. Its remarks name
/// the 2D source (`Promoted to 3D from AUTH:CODE`) and then keep the source remarks.
fn promoted_properties(source: &Identified) -> Identified {
    let mut identified = Identified::named(source.name.clone());
    identified.domains = source
        .domains
        .iter()
        .filter(|domain| {
            domain.area.is_some()
                || domain.south.is_some()
                || domain.west.is_some()
                || domain.north.is_some()
                || domain.east.is_some()
        })
        .map(|domain| ObjectDomain {
            scope: None,
            ..domain.clone()
        })
        .collect();
    identified.remarks = match source.identifiers.as_slice() {
        [identifier] => {
            let mut remarks = format!(
                "Promoted to 3D from {}:{}",
                identifier.code_space(),
                identifier.code()
            );
            if !source.remarks.is_empty() {
                remarks.push_str(". ");
                remarks.push_str(&source.remarks);
            }
            remarks
        }
        _ => source.remarks.clone(),
    };
    identified
}
