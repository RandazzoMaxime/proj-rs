/// A coordinate: four `f64`, as PROJ's `PJ_COORD` carries them.
///
/// The axes are those of the CRS the point is in, in the CRS units: degrees for an angular
/// axis, the CRS unit (usually metres) for a linear one. `z` is the third axis, a height for a
/// geographic or projected CRS. `t` is the coordinate time in decimal years.
///
/// ```
/// use proj_rs::Point;
///
/// let p = Point::new(2.35, 48.85);
/// assert_eq!((p.x, p.y, p.z), (2.35, 48.85, 0.0));
/// assert!(p.t.is_infinite());
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    /// First axis.
    pub x: f64,
    /// Second axis.
    pub y: f64,
    /// Third axis.
    pub z: f64,
    /// Coordinate time in decimal years. `+inf` means no time.
    pub t: f64,
}

impl Point {
    /// A two-dimensional point: `z` is 0 and `t` is `+inf`, as `proj_coord(x, y, 0, HUGE_VAL)`.
    ///
    /// ```
    /// use proj_rs::Point;
    ///
    /// let p = Point::new(2.0, 3.0);
    /// assert_eq!((p.x, p.y, p.z), (2.0, 3.0, 0.0));
    /// ```
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self {
            x,
            y,
            z: 0.0,
            t: f64::INFINITY,
        }
    }

    /// A three-dimensional point, with `t` at `+inf`.
    ///
    /// ```
    /// use proj_rs::Point;
    ///
    /// assert_eq!(Point::xyz(1.0, 2.0, 3.0).z, 3.0);
    /// ```
    #[must_use]
    pub const fn xyz(x: f64, y: f64, z: f64) -> Self {
        Self {
            x,
            y,
            z,
            t: f64::INFINITY,
        }
    }

    /// A four-dimensional point.
    ///
    /// ```
    /// use proj_rs::Point;
    ///
    /// assert_eq!(Point::xyzt(1.0, 2.0, 3.0, 2020.0).t, 2020.0);
    /// ```
    #[must_use]
    pub const fn xyzt(x: f64, y: f64, z: f64, t: f64) -> Self {
        Self { x, y, z, t }
    }
}

impl Default for Point {
    /// The origin, with `z` at 0 and `t` at `+inf`.
    ///
    /// ```
    /// use proj_rs::Point;
    ///
    /// assert_eq!(Point::default(), Point::new(0.0, 0.0));
    /// ```
    fn default() -> Self {
        Self::new(0.0, 0.0)
    }
}

impl From<(f64, f64)> for Point {
    /// ```
    /// use proj_rs::Point;
    ///
    /// assert_eq!(Point::from((2.0, 3.0)), Point::new(2.0, 3.0));
    /// ```
    fn from((x, y): (f64, f64)) -> Self {
        Self::new(x, y)
    }
}
