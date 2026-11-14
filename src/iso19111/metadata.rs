//! `metadata::Citation`, `metadata::Extent` and `common::DataEpoch`: the metadata objects of PROJ's
//! ISO 19111 model that carry no authority code of their own.

use super::model::{BoundingBox, FactoryError, Measure, TemporalExtent, VerticalExtent};

/// `metadata::Citation`: a title, in the simplest form PROJ reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Citation {
    pub(crate) title: Option<String>,
}

impl Citation {
    /// Citation titled `title`.
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
        }
    }

    /// Title, when the citation has one.
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }
}

/// `metadata::Extent`: a description, and the geographic, vertical and temporal elements of a usage.
#[derive(Clone, Debug, PartialEq)]
pub struct Extent {
    description: Option<String>,
    geographic: Vec<BoundingBox>,
    vertical: Vec<VerticalExtent>,
    temporal: Vec<TemporalExtent>,
}

impl Extent {
    /// Extent from its parts. Any of them may be empty.
    #[must_use]
    pub fn create(
        description: Option<String>,
        geographic: Vec<BoundingBox>,
        vertical: Vec<VerticalExtent>,
        temporal: Vec<TemporalExtent>,
    ) -> Self {
        Self {
            description,
            geographic,
            vertical,
            temporal,
        }
    }

    /// `Extent::createFromBBOX`: the extent of one geographic box, in degrees. The box is checked
    /// and widened as [`BoundingBox::create`] does.
    ///
    /// # Errors
    ///
    /// A bound is NaN, or the south bound is north of the north bound.
    pub fn from_bbox(west: f64, south: f64, east: f64, north: f64) -> Result<Self, FactoryError> {
        Ok(Self::create(
            None,
            vec![BoundingBox::create(west, south, east, north)?],
            Vec::new(),
            Vec::new(),
        ))
    }

    /// Description of the extent, when it has one.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Geographic elements, in the order they were given.
    #[must_use]
    pub fn geographic_elements(&self) -> &[BoundingBox] {
        &self.geographic
    }

    /// `Extent::isEquivalentTo`: the same description and the same elements, in the same order.
    #[must_use]
    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        self.description == other.description
            && self.geographic == other.geographic
            && self.vertical.len() == other.vertical.len()
            && self.temporal.len() == other.temporal.len()
    }

    /// `Extent::contains`: the geographic elements contain the other's. Only one element on each
    /// side is compared, as PROJ does. With more, the answer is `true`.
    #[must_use]
    pub fn contains(&self, other: &Self) -> bool {
        match (self.geographic.as_slice(), other.geographic.as_slice()) {
            ([left], [right]) => left.contains(right),
            _ => true,
        }
    }

    /// `Extent::intersects`: the geographic elements intersect. Only one element on each side is
    /// compared, as PROJ does. With more, the answer is `true`.
    #[must_use]
    pub fn intersects(&self, other: &Self) -> bool {
        match (self.geographic.as_slice(), other.geographic.as_slice()) {
            ([left], [right]) => left.intersects(right),
            _ => true,
        }
    }

    /// `Extent::intersection`: the common part of two extents with one geographic element each.
    /// When one contains the other, that one is returned as it is. `None` when they do not meet,
    /// or when either side has other than one geographic element.
    #[must_use]
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let ([left], [right]) = (self.geographic.as_slice(), other.geographic.as_slice()) else {
            return None;
        };
        if self.contains(other) {
            return Some(other.clone());
        }
        if other.contains(self) {
            return Some(self.clone());
        }
        let common = left.intersection(right)?;
        Some(Self::create(None, vec![common], Vec::new(), Vec::new()))
    }
}

/// `common::DataEpoch`: the epoch of a coordinate, as a measure of time.
#[derive(Clone, Debug, PartialEq)]
pub struct DataEpoch {
    coordinate_epoch: Measure,
}

impl DataEpoch {
    /// Epoch `coordinate_epoch`.
    #[must_use]
    pub fn new(coordinate_epoch: Measure) -> Self {
        Self { coordinate_epoch }
    }

    /// The coordinate epoch.
    #[must_use]
    pub fn coordinate_epoch(&self) -> &Measure {
        &self.coordinate_epoch
    }
}
