use crate::api::{BBox, Backend, Context, Crs, Error, Point};
use crate::engine::{Coord, Operation, TransformError as StepError};
use crate::iso19111::SpatialCriterion;
use crate::iso19111::{
    select_alternatives_for_crs, Alternatives, BallparkUse, BestOperation, BestPolicy, BoundingBox,
    CandidateFilter, CoordinateOperationContext, CoordinateOperationFactory, ObjectDomain,
    ProjLines, Selection, SelectionRequest,
};
use crate::proj::Direction;
use crate::proj_iso::swaps_for_visualization;

/// Points a slice goes through the batch path at a time.
const SLICE_CHUNK: usize = 4096;

/// What PROJ's `ONLY_BEST` option asks of the selection of the operation.
///
/// ```
/// use proj_rs::OnlyBest;
///
/// assert_eq!(OnlyBest::default(), OnlyBest::Default);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum OnlyBest {
    /// `ONLY_BEST` unset: the best operation is tried first, and a point it fails on is
    /// retried with the next candidate.
    #[default]
    Default,
    /// `ONLY_BEST=YES`: the best operation is the only one. If it needs a grid that is
    /// missing, building the transformer fails.
    Yes,
    /// `ONLY_BEST=NO`: operations whose grids are missing are not candidates at all.
    No,
}

/// Whether a ballpark transformation (a transformation of unknown accuracy, which ignores the
/// datum shift) may be a candidate.
///
/// ```
/// use proj_rs::Ballpark;
///
/// assert_eq!(Ballpark::default(), Ballpark::Allowed);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Ballpark {
    /// Ballpark transformations are candidates, as by default.
    #[default]
    Allowed,
    /// `ALLOW_BALLPARK=NO`: an operation with a ballpark step is not a candidate.
    Forbidden,
}

/// A candidate operation between two CRS, as `projinfo` lists it.
///
/// ```
/// use proj_rs::{Context, Crs, Transformer};
///
/// let ctx = Context::default();
/// let src = Crs::from_user_input(&ctx, "EPSG:4326").unwrap();
/// let dst = Crs::from_user_input(&ctx, "EPSG:2154").unwrap();
/// let t = Transformer::builder(&src, &dst).build().unwrap();
/// let candidates = t.operations().unwrap();
/// assert!(candidates[0].name().unwrap().contains("Lambert-93"));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct OperationInfo {
    name: Option<String>,
    accuracy: Option<f64>,
    proj_string: Option<String>,
    ballpark: bool,
}

impl OperationInfo {
    /// The name of the operation, `NAD27 to NAD83 (1)` style. `None` for the single
    /// operation a transformer was built with, which the selection returns as a PROJ string.
    ///
    /// ```
    /// # use proj_rs::{Context, Crs, Transformer};
    /// # let ctx = Context::default();
    /// # let src = Crs::from_user_input(&ctx, "EPSG:4326").unwrap();
    /// # let dst = Crs::from_user_input(&ctx, "EPSG:2154").unwrap();
    /// # let t = Transformer::builder(&src, &dst).build().unwrap();
    /// let first = &t.operations().unwrap()[0];
    /// assert!(first.name().is_some());
    /// ```
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The stated accuracy in metres, when the registry gives one.
    ///
    /// ```
    /// # use proj_rs::{Context, Crs, Transformer};
    /// # let ctx = Context::default();
    /// # let src = Crs::from_user_input(&ctx, "EPSG:4326").unwrap();
    /// # let dst = Crs::from_user_input(&ctx, "EPSG:4258").unwrap();
    /// # let t = Transformer::builder(&src, &dst).build().unwrap();
    /// let accuracies: Vec<_> = t.operations().unwrap().iter().map(|o| o.accuracy()).collect();
    /// assert!(!accuracies.is_empty());
    /// ```
    #[must_use]
    pub fn accuracy(&self) -> Option<f64> {
        self.accuracy
    }

    /// The operation as a PROJ pipeline, in one line, when it can be written as one.
    ///
    /// ```
    /// # use proj_rs::{Context, Crs, Transformer};
    /// # let ctx = Context::default();
    /// # let src = Crs::from_user_input(&ctx, "EPSG:4326").unwrap();
    /// # let dst = Crs::from_user_input(&ctx, "EPSG:2154").unwrap();
    /// # let t = Transformer::builder(&src, &dst).build().unwrap();
    /// let text = t.operations().unwrap()[0].proj_string().unwrap().to_owned();
    /// assert!(text.contains("+proj=lcc"));
    /// ```
    #[must_use]
    pub fn proj_string(&self) -> Option<&str> {
        self.proj_string.as_deref()
    }

    /// Whether the operation is a ballpark transformation.
    ///
    /// ```
    /// # use proj_rs::{Context, Crs, Transformer};
    /// # let ctx = Context::default();
    /// # let src = Crs::from_user_input(&ctx, "EPSG:4326").unwrap();
    /// # let dst = Crs::from_user_input(&ctx, "EPSG:2154").unwrap();
    /// # let t = Transformer::builder(&src, &dst).build().unwrap();
    /// assert!(!t.operations().unwrap()[0].is_ballpark());
    /// ```
    #[must_use]
    pub fn is_ballpark(&self) -> bool {
        self.ballpark
    }
}

/// The operation a transformer runs.
#[derive(Clone, Debug)]
enum Route {
    /// One operation: PROJ returns it without alternatives.
    Single(Operation, String),
    /// Several candidates, chosen point by point as `proj_trans` does.
    Several(Box<Alternatives>),
    /// A single operation whose grids are missing: each point fails with the reason.
    Unavailable(String),
}

/// Whether the first two axes of a point are swapped at one end of the pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Swap {
    Kept,
    Swapped,
}

impl Swap {
    fn of(crs: &Crs) -> Self {
        if swaps_for_visualization(&crs.object) {
            Self::Swapped
        } else {
            Self::Kept
        }
    }
}

/// The axis order of the points a [`Transformer`] reads and writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AxisOrder {
    /// The order each CRS defines: latitude first for `EPSG:4326`.
    Authority,
    /// Longitude, latitude and easting, northing, as a GIS reads them.
    Gis,
}

/// A transformation from one [`Crs`] to another.
///
/// The points are in the axis order of the CRS (`EPSG:4326` is latitude, longitude), as
/// PROJ reads them, unless the builder asks for [`TransformerBuilder::normalize_axis_order`].
/// The operation is the one `cs2cs` picks: the best candidate of the registry, or, when
/// several cover different areas, the one that covers the point.
///
/// A transformer is `Send` but not `Sync`: it keeps the candidate the last point used.
///
/// ```
/// use proj_rs::{Context, Crs, Point, Transformer};
///
/// let ctx = Context::default();
/// let src = Crs::from_user_input(&ctx, "EPSG:4326").unwrap();
/// let dst = Crs::from_user_input(&ctx, "EPSG:32631").unwrap();
/// let t = Transformer::builder(&src, &dst).build().unwrap();
/// let p = t.forward(Point::new(48.85, 2.35)).unwrap(); // latitude, longitude
/// assert!((p.x - 452_314.89).abs() < 1e-2);
/// ```
#[derive(Clone, Debug)]
pub struct Transformer {
    route: Route,
    source: Crs,
    target: Crs,
    context: Context,
    order: AxisOrder,
    backend: Backend,
    source_swapped: Swap,
    target_swapped: Swap,
    area: Option<BBox>,
    ballpark: Ballpark,
}

/// Builds a [`Transformer`]. Made by [`Transformer::builder`].
///
/// ```
/// use proj_rs::{Backend, Context, Crs, Transformer};
///
/// let ctx = Context::default();
/// let src = Crs::from_user_input(&ctx, "EPSG:4326").unwrap();
/// let dst = Crs::from_user_input(&ctx, "EPSG:3857").unwrap();
/// let t = Transformer::builder(&src, &dst)
///     .normalize_axis_order()
///     .backend(Backend::Scalar)
///     .build()
///     .unwrap();
/// # let _ = t;
/// ```
#[derive(Clone, Debug)]
pub struct TransformerBuilder<'a> {
    source: &'a Crs,
    target: &'a Crs,
    area: Option<BBox>,
    only_best: OnlyBest,
    ballpark: Ballpark,
    order: AxisOrder,
    backend: Backend,
}

impl<'a> TransformerBuilder<'a> {
    /// Restrict the candidates to the operations whose area of use meets `area`, and rank
    /// them by how well they cover it.
    ///
    /// ```
    /// use proj_rs::{BBox, Context, Crs, Transformer};
    ///
    /// let ctx = Context::default();
    /// let src = Crs::from_user_input(&ctx, "EPSG:4267").unwrap();
    /// let dst = Crs::from_user_input(&ctx, "EPSG:4269").unwrap();
    /// let t = Transformer::builder(&src, &dst)
    ///     .area_of_interest(BBox::new(-100.0, 35.0, -90.0, 40.0))
    ///     .build();
    /// assert!(t.is_ok());
    /// ```
    #[must_use]
    pub fn area_of_interest(mut self, area: BBox) -> Self {
        self.area = Some(area);
        self
    }

    /// What `ONLY_BEST` asks: see [`OnlyBest`].
    ///
    /// ```
    /// use proj_rs::{Context, Crs, OnlyBest, Transformer};
    ///
    /// let ctx = Context::default();
    /// let src = Crs::from_user_input(&ctx, "EPSG:4326").unwrap();
    /// let dst = Crs::from_user_input(&ctx, "EPSG:2154").unwrap();
    /// let t = Transformer::builder(&src, &dst).only_best(OnlyBest::Yes).build();
    /// assert!(t.is_ok());
    /// ```
    #[must_use]
    pub fn only_best(mut self, only_best: OnlyBest) -> Self {
        self.only_best = only_best;
        self
    }

    /// Whether a ballpark transformation may be a candidate. The default is
    /// [`Ballpark::Allowed`].
    ///
    /// ```
    /// use proj_rs::{Ballpark, Context, Crs, Transformer};
    ///
    /// let ctx = Context::default();
    /// let src = Crs::from_user_input(&ctx, "EPSG:4326").unwrap();
    /// let dst = Crs::from_user_input(&ctx, "EPSG:2154").unwrap();
    /// let t = Transformer::builder(&src, &dst).allow_ballpark(Ballpark::Forbidden).build();
    /// assert!(t.is_ok());
    /// ```
    #[must_use]
    pub fn allow_ballpark(mut self, ballpark: Ballpark) -> Self {
        self.ballpark = ballpark;
        self
    }

    /// Read and write longitude then latitude, easting then northing, as a GIS does, whatever
    /// the axis order of each CRS. This is `proj_normalize_for_visualization`.
    ///
    /// ```
    /// use proj_rs::{Context, Crs, Point, Transformer};
    ///
    /// let ctx = Context::default();
    /// let src = Crs::from_user_input(&ctx, "EPSG:4326").unwrap();
    /// let dst = Crs::from_user_input(&ctx, "EPSG:2154").unwrap();
    /// let t = Transformer::builder(&src, &dst).normalize_axis_order().build().unwrap();
    /// let p = t.forward(Point::new(2.35, 48.85)).unwrap(); // longitude, latitude
    /// assert!((p.x - 652_301.56).abs() < 1.0);
    /// ```
    #[must_use]
    pub fn normalize_axis_order(mut self) -> Self {
        self.order = AxisOrder::Gis;
        self
    }

    /// The backend slices run on. The default is [`Backend::Auto`].
    ///
    /// ```
    /// use proj_rs::{Backend, Context, Crs, Transformer};
    ///
    /// let ctx = Context::default();
    /// let src = Crs::from_user_input(&ctx, "EPSG:4326").unwrap();
    /// let dst = Crs::from_user_input(&ctx, "EPSG:2154").unwrap();
    /// assert!(Transformer::builder(&src, &dst).backend(Backend::Scalar).build().is_ok());
    /// ```
    #[must_use]
    pub fn backend(mut self, backend: Backend) -> Self {
        self.backend = backend;
        self
    }

    /// Select the operation and build the transformer.
    ///
    /// # Errors
    ///
    /// [`Error::NoOperation`] when no operation connects the two CRS or the filters keep none,
    /// and [`Error::MissingGrid`] under [`OnlyBest::Yes`] when the best operation needs a grid
    /// that is not available.
    ///
    /// ```
    /// use proj_rs::{Context, Crs, Transformer};
    ///
    /// let ctx = Context::default();
    /// let src = Crs::from_user_input(&ctx, "EPSG:4326").unwrap();
    /// let dst = Crs::from_user_input(&ctx, "EPSG:32631").unwrap();
    /// assert!(Transformer::builder(&src, &dst).build().is_ok());
    /// ```
    pub fn build(self) -> Result<Transformer, Error> {
        let policy = match self.only_best {
            OnlyBest::Default => BestPolicy::Warn,
            OnlyBest::Yes => BestPolicy::Error,
            OnlyBest::No => BestPolicy::Discard,
        };
        let route = self.select(policy)?;
        Ok(Transformer {
            route,
            source: self.source.clone(),
            target: self.target.clone(),
            context: self.source.context.clone(),
            order: self.order,
            backend: self.backend,
            source_swapped: Swap::of(self.source),
            target_swapped: Swap::of(self.target),
            area: self.area,
            ballpark: self.ballpark,
        })
    }

    fn select(&self, policy: BestPolicy) -> Result<Route, Error> {
        let context = &self.source.context;
        let db = context
            .legacy
            .database()
            .map_err(|error| Error::Crs(error.to_string()))?;
        let request = SelectionRequest {
            policy,
            filter: CandidateFilter {
                accuracy: None,
                ballpark: match self.ballpark {
                    Ballpark::Allowed => crate::iso19111::Ballpark::Allowed,
                    Ballpark::Forbidden => crate::iso19111::Ballpark::Forbidden,
                },
            },
            authority: "",
            area: self.area.map(domain_of).transpose()?,
        };
        let selection = crate::network::with_context(&context.net, || {
            select_alternatives_for_crs(
                &db,
                self.source.object.clone(),
                self.target.object.clone(),
                (self.source.name(), self.target.name()),
                request,
            )
        })
        .map_err(|error| Error::NoOperation(error.to_string()))?;
        match selection {
            Selection::Several(alternatives) => Ok(Route::Several(Box::new(alternatives))),
            Selection::Single(BestOperation::Pipeline { definition, .. }) => {
                let operation = Operation::create(&definition)
                    .map_err(|error| Error::NoOperation(error.to_string()))?;
                Ok(Route::Single(operation, definition))
            }
            Selection::Single(BestOperation::MissingGrid { message }) => {
                if policy == BestPolicy::Error {
                    Err(Error::MissingGrid(message))
                } else {
                    Ok(Route::Unavailable(message))
                }
            }
            Selection::Single(BestOperation::Refused { message }) => {
                Err(Error::MissingGrid(message))
            }
        }
    }
}

fn domain_of(area: BBox) -> Result<ObjectDomain, Error> {
    BoundingBox::create(area.west, area.south, area.east, area.north)
        .map(ObjectDomain::from_bounding_box)
        .map_err(|error| Error::InvalidInput(error.to_string()))
}

impl Transformer {
    /// A builder for the transformation from `source` to `target`, under the context of
    /// `source`.
    ///
    /// ```
    /// use proj_rs::{Context, Crs, Transformer};
    ///
    /// let ctx = Context::default();
    /// let src = Crs::from_user_input(&ctx, "EPSG:4326").unwrap();
    /// let dst = Crs::from_user_input(&ctx, "EPSG:2154").unwrap();
    /// let t = Transformer::builder(&src, &dst).build().unwrap();
    /// # let _ = t;
    /// ```
    #[must_use]
    pub fn builder<'a>(source: &'a Crs, target: &'a Crs) -> TransformerBuilder<'a> {
        TransformerBuilder {
            source,
            target,
            area: None,
            only_best: OnlyBest::Default,
            ballpark: Ballpark::Allowed,
            order: AxisOrder::Authority,
            backend: Backend::Auto,
        }
    }

    /// Transform one point from the source CRS to the target CRS.
    ///
    /// # Errors
    ///
    /// [`Error`] when the point has no image: outside the projection domain or the grids, or
    /// covered by no candidate, and [`Error::InvalidCoordinate`] when `x`, `y` or `z` is not
    /// finite.
    ///
    /// ```
    /// use proj_rs::{Context, Crs, Point, Transformer};
    ///
    /// let ctx = Context::default();
    /// let src = Crs::from_user_input(&ctx, "EPSG:4326").unwrap();
    /// let dst = Crs::from_user_input(&ctx, "EPSG:2154").unwrap();
    /// let t = Transformer::builder(&src, &dst).build().unwrap();
    /// let p = t.forward(Point::new(48.85, 2.35)).unwrap();
    /// assert!((p.y - 6_861_302.73).abs() < 1.0);
    /// ```
    pub fn forward(&self, point: Point) -> Result<Point, Error> {
        self.run(point, Direction::Forward)
    }

    /// Transform one point from the target CRS back to the source CRS.
    ///
    /// # Errors
    ///
    /// As [`Self::forward`], and [`Error::NoInverse`] when the operation has no inverse.
    ///
    /// ```
    /// use proj_rs::{Context, Crs, Point, Transformer};
    ///
    /// let ctx = Context::default();
    /// let src = Crs::from_user_input(&ctx, "EPSG:4326").unwrap();
    /// let dst = Crs::from_user_input(&ctx, "EPSG:2154").unwrap();
    /// let t = Transformer::builder(&src, &dst).build().unwrap();
    /// let back = t.inverse(Point::new(652_301.56, 6_861_302.73)).unwrap();
    /// assert!((back.x - 48.85).abs() < 1e-3 && (back.y - 2.35).abs() < 1e-3);
    /// ```
    pub fn inverse(&self, point: Point) -> Result<Point, Error> {
        self.run(point, Direction::Inverse)
    }

    /// Transform a slice in place, source CRS to target CRS.
    ///
    /// The slice goes through the batch path of the [`Backend`], which can differ from
    /// [`Self::forward`] by 11 nm at most. On an error the slice is left unchanged, and the
    /// error is the one of the first failing point.
    ///
    /// # Errors
    ///
    /// [`Error`] when any point has no image.
    ///
    /// ```
    /// use proj_rs::{Context, Crs, Point, Transformer};
    ///
    /// let ctx = Context::default();
    /// let src = Crs::from_user_input(&ctx, "EPSG:4326").unwrap();
    /// let dst = Crs::from_user_input(&ctx, "EPSG:2154").unwrap();
    /// let t = Transformer::builder(&src, &dst).build().unwrap();
    /// let mut points = [Point::new(48.85, 2.35), Point::new(45.76, 4.84)];
    /// t.forward_slice(&mut points).unwrap();
    /// assert!(points[1].x > 800_000.0);
    /// ```
    pub fn forward_slice(&self, points: &mut [Point]) -> Result<(), Error> {
        self.run_slice(points, Direction::Forward)
    }

    /// Transform a slice in place, target CRS back to source CRS. Same contract as
    /// [`Self::forward_slice`].
    ///
    /// # Errors
    ///
    /// [`Error`] when any point has no image.
    ///
    /// ```
    /// use proj_rs::{Context, Crs, Point, Transformer};
    ///
    /// let ctx = Context::default();
    /// let src = Crs::from_user_input(&ctx, "EPSG:4326").unwrap();
    /// let dst = Crs::from_user_input(&ctx, "EPSG:2154").unwrap();
    /// let t = Transformer::builder(&src, &dst).build().unwrap();
    /// let mut points = [Point::new(652_301.56, 6_861_302.73)];
    /// t.inverse_slice(&mut points).unwrap();
    /// assert!((points[0].x - 48.85).abs() < 1e-3);
    /// ```
    pub fn inverse_slice(&self, points: &mut [Point]) -> Result<(), Error> {
        self.run_slice(points, Direction::Inverse)
    }

    /// The candidate operations between the two CRS, as `projinfo --spatial-test intersects`
    /// lists them: the ones the transformer chooses among. The area of interest and the
    /// ballpark setting of the builder apply.
    ///
    /// # Errors
    ///
    /// [`Error::NoOperation`] when the registry cannot be read or no candidate can be built.
    ///
    /// ```
    /// use proj_rs::{Context, Crs, Transformer};
    ///
    /// let ctx = Context::default();
    /// let src = Crs::from_user_input(&ctx, "EPSG:4267").unwrap();
    /// let dst = Crs::from_user_input(&ctx, "EPSG:4269").unwrap();
    /// let t = Transformer::builder(&src, &dst).build().unwrap();
    /// assert!(t.operations().unwrap().len() > 1);
    /// ```
    pub fn operations(&self) -> Result<Vec<OperationInfo>, Error> {
        let db = self
            .context
            .legacy
            .database()
            .map_err(|error| Error::NoOperation(error.to_string()))?;
        let mut request = CoordinateOperationContext::create(&db, "");
        request
            .set_spatial_criterion(SpatialCriterion::PartialIntersection)
            .set_area_of_interest(self.area.map(domain_of).transpose()?)
            .set_ballpark_use(match self.ballpark {
                Ballpark::Allowed => BallparkUse::Allow,
                Ballpark::Forbidden => BallparkUse::Forbid,
            });
        let operations = crate::network::with_context(&self.context.net, || {
            CoordinateOperationFactory::create().create_operations(
                &self.source.object,
                &self.target.object,
                &request,
            )
        })
        .map_err(|error| Error::NoOperation(error.to_string()))?;
        Ok(operations
            .iter()
            .map(|operation| OperationInfo {
                name: Some(operation.name().to_owned()),
                accuracy: operation.accuracy(),
                proj_string: operation
                    .export_to_proj_string_with_database(&db, ProjLines::Single)
                    .ok(),
                ballpark: operation.has_ballpark_transformation(),
            })
            .collect())
    }

    /// The operation the last point went through. With several candidates it is the one that
    /// point chose, `None` before the first point. With a single operation it is that
    /// operation, which the selection returns as a PROJ string without a name.
    ///
    /// ```
    /// use proj_rs::{Context, Crs, Point, Transformer};
    ///
    /// let ctx = Context::default();
    /// let src = Crs::from_user_input(&ctx, "EPSG:4267").unwrap();
    /// let dst = Crs::from_user_input(&ctx, "EPSG:4269").unwrap();
    /// let t = Transformer::builder(&src, &dst).build().unwrap();
    /// assert!(t.last_used_operation().is_none());
    /// t.forward(Point::new(38.0, -95.0)).unwrap();
    /// assert!(t.last_used_operation().is_some());
    /// ```
    #[must_use]
    pub fn last_used_operation(&self) -> Option<OperationInfo> {
        match &self.route {
            Route::Several(alternatives) => {
                let item = alternatives.current_item()?;
                Some(OperationInfo {
                    name: Some(item.name.clone()),
                    accuracy: (item.accuracy >= 0.0).then_some(item.accuracy),
                    proj_string: item.definition.clone(),
                    ballpark: false,
                })
            }
            Route::Single(_, definition) => Some(OperationInfo {
                name: None,
                accuracy: None,
                proj_string: Some(definition.clone()),
                ballpark: false,
            }),
            Route::Unavailable(_) => None,
        }
    }

    fn swaps(&self, way: Direction) -> (Swap, Swap) {
        if self.order == AxisOrder::Authority {
            return (Swap::Kept, Swap::Kept);
        }
        match way {
            Direction::Forward => (self.source_swapped, self.target_swapped),
            Direction::Inverse => (self.target_swapped, self.source_swapped),
        }
    }

    fn run(&self, point: Point, way: Direction) -> Result<Point, Error> {
        let (from, to) = self.swaps(way);
        let coord = to_coord(point, from)?;
        let out = crate::network::with_context(&self.context.net, || self.map(coord, way))?;
        Ok(from_coord(out, point.t, to))
    }

    fn run_slice(&self, points: &mut [Point], way: Direction) -> Result<(), Error> {
        let (from, to) = self.swaps(way);
        let mut coords = points
            .iter()
            .map(|point| to_coord(*point, from))
            .collect::<Result<Vec<Coord>, Error>>()?;
        crate::network::with_context(&self.context.net, || self.map_slice(&mut coords, way))?;
        for (point, coord) in points.iter_mut().zip(coords) {
            *point = from_coord(coord, point.t, to);
        }
        Ok(())
    }

    fn map(&self, coord: Coord, way: Direction) -> Result<Coord, Error> {
        match (&self.route, way) {
            (Route::Single(operation, _), Direction::Forward) => Ok(operation.forward(coord)?),
            (Route::Single(operation, _), Direction::Inverse) => Ok(operation.inverse(coord)?),
            (Route::Several(alternatives), Direction::Forward) => Ok(alternatives.forward(coord)?),
            (Route::Several(alternatives), Direction::Inverse) => Ok(alternatives.inverse(coord)?),
            (Route::Unavailable(message), _) => Err(Error::MissingGrid(message.clone())),
        }
    }

    fn map_slice(&self, coords: &mut [Coord], way: Direction) -> Result<(), Error> {
        match (&self.route, self.backend) {
            (Route::Single(operation, _), Backend::Auto) => {
                for chunk in coords.chunks_mut(SLICE_CHUNK) {
                    let mut failed: Vec<Option<StepError>> = vec![None; chunk.len()];
                    match way {
                        Direction::Forward => operation.forward_batch(chunk, &mut failed),
                        Direction::Inverse => operation.inverse_batch(chunk, &mut failed),
                    }
                    if let Some(error) = failed.into_iter().flatten().next() {
                        return Err(error.into());
                    }
                }
                Ok(())
            }
            _ => {
                let mapped = coords
                    .iter()
                    .map(|coord| self.map(*coord, way))
                    .collect::<Result<Vec<_>, _>>()?;
                coords.copy_from_slice(&mapped);
                Ok(())
            }
        }
    }
}

/// The engine coordinate of `point`, its first two axes swapped when `swap`. A coordinate that
/// is not finite is refused, never carried through. A time of `+inf` means no time, which the
/// engine reads as 0.
fn to_coord(point: Point, swap: Swap) -> Result<Coord, Error> {
    if !(point.x.is_finite() && point.y.is_finite() && point.z.is_finite() && !point.t.is_nan()) {
        return Err(Error::InvalidCoordinate);
    }
    let (x, y) = if swap == Swap::Swapped {
        (point.y, point.x)
    } else {
        (point.x, point.y)
    };
    Ok(Coord {
        x,
        y,
        z: point.z,
        t: if point.t.is_infinite() { 0.0 } else { point.t },
    })
}

/// The point of an engine coordinate, its first two axes swapped when `swap`. A point that came
/// in with no time (`+inf`) goes out with none.
fn from_coord(coord: Coord, time_in: f64, swap: Swap) -> Point {
    let (x, y) = if swap == Swap::Swapped {
        (coord.y, coord.x)
    } else {
        (coord.x, coord.y)
    };
    Point {
        x,
        y,
        z: coord.z,
        t: if time_in.is_infinite() {
            time_in
        } else {
            coord.t
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transformer_is_send() {
        fn is_send<T: Send>() {}
        is_send::<Transformer>();
    }
}
