//! `GeodeticCRS::identify` when an authority factory is present.
//!
//! The no-factory path stays in `equiv`. With a factory, a hardcoded WGS 84 /
//! NAD27 / NAD83 candidate is returned only when the name is exactly the same
//! and the CRS is equivalent. Every other match comes from `proj.db`.

use super::buildop::{OpProps, UtmHemisphere};
use super::equiv::{
    directions_and_units, ellipsoid_parameters_match, equivalent_name, identify_axes_match,
    meridian_equivalent, same_epoch, IdentifyAxisOrder,
};
use super::factory::{AuthorityFactory, DatabaseContext, NameMatch};
use super::model::{
    Axis, CartesianCS, Conversion, Crs, Datum, DatumEnsemble, GeodeticCRS, GeodeticReferenceFrame,
    GeographicCRS, Identified, Identifier, ProjectedCRS, UnitOfMeasure,
};

const UNKNOWN_BASED_ON: &str = "Unknown based on ";

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
                        let confidence = if equivalent_name(name, crs.name()) { 90 } else { 70 };
                        hits.push(Hit { crs, confidence });
                        above_25 = true;
                    } else {
                        hits.push(Hit { crs, confidence: 25 });
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
    /// Geographic and geodetic CRS use [`GeodeticCRS::identify_with`]. A
    /// projected CRS uses the UTM shortcut. Other kinds have no match.
    #[must_use]
    pub fn identify_with(&self, factory: Option<&AuthorityFactory>) -> Vec<(Crs, i32)> {
        match self {
            Self::Geographic(crs) => crs.identify_with(factory),
            Self::Geodetic(crs) => crs.identify_with(factory),
            Self::Projected(crs) => crs.identify_with(factory),
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
        GeodeticCRS::Geographic(crs) if crs.coordinate_system().axes().len() == 2 => "geographic 2D",
        GeodeticCRS::Geographic(_) => "geographic 3D",
        GeodeticCRS::Spherical(_) => "",
    }
}

fn insignificant(name: &str) -> bool {
    name.is_empty() || name.eq_ignore_ascii_case("unknown") || name.eq_ignore_ascii_case("unnamed")
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
        if equivalent_name(crs.name(), candidate.name()) && equivalent && name_equal {
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
                let meridian_matches = meridian_equivalent(
                    candidate_frame.prime_meridian(),
                    frame.prime_meridian(),
                );
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
        right.confidence.cmp(&left.confidence).then_with(|| {
            prefer(
                left.crs.name() == name,
                right.crs.name() == name,
            )
        })
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
    owning_frame(crs)
        .is_some_and(|frame| equivalent_name(frame.ellipsoid().name(), crs.name()))
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
        return equivalent_name(stripped, right.name())
            || names_alias(left, stripped, right, right.name(), db);
    }
    if right_prefixed && !left_prefixed {
        let stripped = &right.name()[2..];
        return equivalent_name(left.name(), stripped)
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
        (!official.is_empty() && equivalent_name(name, &official))
            || aliases.iter().any(|alias| equivalent_name(name, alias))
    };
    matches(this_name) && matches(other_name)
}

fn alias_from_name(this_name: &str, other_name: &str, db: &DatabaseContext) -> bool {
    db.get_aliases("", "", this_name, "geodetic_datum", "")
        .iter()
        .any(|alias| equivalent_name(other_name, alias))
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
                    let height_matches = found.coordinate_system().axes().get(2).is_some_and(|axis| {
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
        stamp_promoted(&mut crs.identified);
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

impl ProjectedCRS {
    /// `ProjectedCRS::promoteTo3D`.
    ///
    /// The new axis is ellipsoidal height, upwards, in the horizontal unit.
    /// The base geographic CRS is promoted with `db`. Identifiers are not kept.
    #[must_use]
    pub fn promote_to_3d(&self, db: Option<&DatabaseContext>) -> Self {
        if self.coordinate_system.axes.len() != 2 {
            return self.clone();
        }
        let base = match &self.base {
            GeodeticCRS::Geographic(crs) => {
                GeodeticCRS::Geographic(crs.promote_to_3d_with_db(db))
            }
            other => other.clone(),
        };
        let unit = self.coordinate_system.axes[0].unit().clone();
        let mut coordinate_system = self.coordinate_system.clone();
        coordinate_system
            .axes
            .push(Axis::new("Ellipsoidal height", "h", "up", unit));
        let mut identified = Identified::named(self.name());
        identified.domains = self.identified.domains.clone();
        if self.identifiers().len() == 1 {
            let identifier = &self.identifiers()[0];
            identified.remarks = format!(
                "Promoted to 3D from {}:{}",
                identifier.code_space(),
                identifier.code()
            );
        }
        Self {
            identified,
            base,
            conversion: self.conversion.clone(),
            coordinate_system,
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
        }
    }

    /// `ProjectedCRS::identify` for a UTM zone on WGS 84, NAD27, or NAD83.
    ///
    /// One base match at confidence 70 or more, an easting/northing metre
    /// system, and a transverse Mercator with UTM parameters returns that zone.
    /// An exact CRS name is confidence 100.
    #[must_use]
    pub fn identify_with(&self, factory: Option<&AuthorityFactory>) -> Vec<(Crs, i32)> {
        if factory.is_some_and(|factory| {
            !(factory.authority().is_empty() || factory.authority() == "EPSG")
        }) {
            return Vec::new();
        }
        let Some((zone, hemisphere)) = self.conversion.utm_zone() else {
            return Vec::new();
        };
        if !easting_northing_metre(&self.coordinate_system) {
            return Vec::new();
        }
        let base = self.base.identify_with(factory);
        if base.len() != 1 || base[0].1 < 70 {
            return Vec::new();
        }
        let Crs::Geographic(base_crs) = &base[0].0 else {
            return Vec::new();
        };
        let (name, code) = if base_crs.is_equivalent_to(&GeographicCRS::epsg_4326()) {
            utm_name_code("WGS 84 / UTM zone ", zone, hemisphere, 32_600, 32_700)
        } else if hemisphere == UtmHemisphere::North
            && ((1..=22).contains(&zone) || zone == 59 || zone == 60)
            && base_crs.is_equivalent_to(&GeographicCRS::epsg_4267())
        {
            let code = if zone >= 59 {
                3370 + zone - 59
            } else {
                26_700 + zone
            };
            (format!("NAD27 / UTM zone {zone}N"), code)
        } else if hemisphere == UtmHemisphere::North
            && ((1..=23).contains(&zone) || zone == 59 || zone == 60)
            && base_crs.is_equivalent_to(&GeographicCRS::epsg_4269())
        {
            let code = if zone >= 59 {
                3372 + zone - 59
            } else {
                26_900 + zone
            };
            (format!("NAD83 / UTM zone {zone}N"), code)
        } else {
            return Vec::new();
        };
        let conversion = match hemisphere {
            UtmHemisphere::North => Conversion::create_utm_north(&OpProps::new(), zone),
            UtmHemisphere::South => Conversion::create_utm_south(&OpProps::new(), zone),
        };
        let Ok(conversion) = conversion else {
            return Vec::new();
        };
        let crs = ProjectedCRS::create(
            name.clone(),
            GeodeticCRS::Geographic(match code {
                26_700..=26_722 | 3_370 | 3_371 => GeographicCRS::epsg_4267(),
                26_900..=26_923 | 3_372 | 3_373 => GeographicCRS::epsg_4269(),
                _ => GeographicCRS::epsg_4326(),
            }),
            conversion,
            CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
        )
        .with_identifier("EPSG", &code.to_string());
        let confidence = if name == self.name() {
            100
        } else if equivalent_name(&name, self.name()) {
            90
        } else {
            70
        };
        vec![(Crs::Projected(crs), confidence)]
    }
}

fn utm_name_code(
    prefix: &str,
    zone: i32,
    hemisphere: UtmHemisphere,
    north_base: i32,
    south_base: i32,
) -> (String, i32) {
    match hemisphere {
        UtmHemisphere::North => (format!("{prefix}{zone}N"), north_base + zone),
        UtmHemisphere::South => (format!("{prefix}{zone}S"), south_base + zone),
    }
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

fn stamp_promoted(identified: &mut Identified) {
    let identifiers = identified.identifiers.clone();
    identified.identifiers.clear();
    if identifiers.len() == 1 {
        identified.remarks = format!(
            "Promoted to 3D from {}:{}",
            identifiers[0].code_space(),
            identifiers[0].code()
        );
    }
}
