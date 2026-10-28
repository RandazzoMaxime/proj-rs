//! [`Coord`] and [`Transform`] for `geo-types` 0.7.
//!
//! Enabled with the `geo-types` feature.

use geo_types::{coord, Geometry};

use crate::{Coord, CoordinateType, Proj, ProjError, Transform};

impl<T: CoordinateType + geo_types::CoordNum> Coord<T> for geo_types::Coord<T> {
    fn x(&self) -> T {
        self.x
    }

    fn y(&self) -> T {
        self.y
    }

    fn from_xy(x: T, y: T) -> Self {
        coord! { x: x, y: y }
    }
}

impl<T: CoordinateType + geo_types::CoordNum> Coord<T> for geo_types::Point<T> {
    fn x(&self) -> T {
        geo_types::Point::x(*self)
    }

    fn y(&self) -> T {
        geo_types::Point::y(*self)
    }

    fn from_xy(x: T, y: T) -> Self {
        Self::new(x, y)
    }
}

impl<T: CoordinateType + geo_types::CoordNum> Transform<T> for geo_types::Geometry<T> {
    type Output = Self;

    fn transformed(&self, proj: &Proj) -> Result<Self, ProjError> {
        let mut output = self.clone();
        output.transform(proj)?;
        Ok(output)
    }

    fn transform(&mut self, proj: &Proj) -> Result<(), ProjError> {
        match self {
            Geometry::Point(geometry) => geometry.transform(proj),
            Geometry::Line(geometry) => geometry.transform(proj),
            Geometry::LineString(geometry) => geometry.transform(proj),
            Geometry::Polygon(geometry) => geometry.transform(proj),
            Geometry::MultiPoint(geometry) => geometry.transform(proj),
            Geometry::MultiLineString(geometry) => geometry.transform(proj),
            Geometry::MultiPolygon(geometry) => geometry.transform(proj),
            Geometry::GeometryCollection(geometry) => geometry.transform(proj),
            Geometry::Rect(geometry) => geometry.transform(proj),
            Geometry::Triangle(geometry) => geometry.transform(proj),
        }
    }
}

impl<T: CoordinateType + geo_types::CoordNum> Transform<T> for geo_types::Coord<T> {
    type Output = Self;

    fn transformed(&self, proj: &Proj) -> Result<Self, ProjError> {
        let mut output = *self;
        output.transform(proj)?;
        Ok(output)
    }

    fn transform(&mut self, proj: &Proj) -> Result<(), ProjError> {
        *self = proj.convert(*self)?;
        Ok(())
    }
}

impl<T: CoordinateType + geo_types::CoordNum> Transform<T> for geo_types::Point<T> {
    type Output = Self;

    fn transformed(&self, proj: &Proj) -> Result<Self, ProjError> {
        let mut output = *self;
        output.transform(proj)?;
        Ok(output)
    }

    fn transform(&mut self, proj: &Proj) -> Result<(), ProjError> {
        self.0.transform(proj)
    }
}

impl<T: CoordinateType + geo_types::CoordNum> Transform<T> for geo_types::Line<T> {
    type Output = Self;

    fn transformed(&self, proj: &Proj) -> Result<Self, ProjError> {
        let mut output = *self;
        output.transform(proj)?;
        Ok(output)
    }

    fn transform(&mut self, proj: &Proj) -> Result<(), ProjError> {
        self.start.transform(proj)?;
        self.end.transform(proj)?;
        Ok(())
    }
}

impl<T: CoordinateType + geo_types::CoordNum> Transform<T> for geo_types::LineString<T> {
    type Output = Self;

    fn transformed(&self, proj: &Proj) -> Result<Self, ProjError> {
        let mut output = self.clone();
        output.transform(proj)?;
        Ok(output)
    }

    fn transform(&mut self, proj: &Proj) -> Result<(), ProjError> {
        proj.convert_array(&mut self.0)?;
        Ok(())
    }
}

impl<T: CoordinateType + geo_types::CoordNum> Transform<T> for geo_types::Polygon<T> {
    type Output = Self;

    fn transformed(&self, proj: &Proj) -> Result<Self, ProjError> {
        let mut output = self.clone();
        output.transform(proj)?;
        Ok(output)
    }

    fn transform(&mut self, proj: &Proj) -> Result<(), ProjError> {
        let mut exterior_result = Ok(());
        self.exterior_mut(|exterior| {
            exterior_result = exterior.transform(proj);
        });
        exterior_result?;
        let mut interiors_result = Ok(());
        self.interiors_mut(|interiors| {
            interiors_result = interiors
                .iter_mut()
                .try_for_each(|interior| interior.transform(proj));
        });
        interiors_result?;
        Ok(())
    }
}

impl<T: CoordinateType + geo_types::CoordNum> Transform<T> for geo_types::MultiPoint<T> {
    type Output = Self;

    fn transformed(&self, proj: &Proj) -> Result<Self, ProjError> {
        let mut output = self.clone();
        output.transform(proj)?;
        Ok(output)
    }

    fn transform(&mut self, proj: &Proj) -> Result<(), ProjError> {
        proj.convert_array(&mut self.0)?;
        Ok(())
    }
}

impl<T: CoordinateType + geo_types::CoordNum> Transform<T> for geo_types::MultiLineString<T> {
    type Output = Self;

    fn transformed(&self, proj: &Proj) -> Result<Self, ProjError> {
        let mut output = self.clone();
        output.transform(proj)?;
        Ok(output)
    }

    fn transform(&mut self, proj: &Proj) -> Result<(), ProjError> {
        for line in &mut self.0 {
            line.transform(proj)?;
        }
        Ok(())
    }
}

impl<T: CoordinateType + geo_types::CoordNum> Transform<T> for geo_types::MultiPolygon<T> {
    type Output = Self;

    fn transformed(&self, proj: &Proj) -> Result<Self, ProjError> {
        let mut output = self.clone();
        output.transform(proj)?;
        Ok(output)
    }

    fn transform(&mut self, proj: &Proj) -> Result<(), ProjError> {
        for polygon in &mut self.0 {
            polygon.transform(proj)?;
        }
        Ok(())
    }
}

impl<T: CoordinateType + geo_types::CoordNum> Transform<T> for geo_types::GeometryCollection<T> {
    type Output = Self;

    fn transformed(&self, proj: &Proj) -> Result<Self, ProjError> {
        let mut output = self.clone();
        output.transform(proj)?;
        Ok(output)
    }

    fn transform(&mut self, proj: &Proj) -> Result<(), ProjError> {
        for geometry in &mut self.0 {
            geometry.transform(proj)?;
        }
        Ok(())
    }
}

impl<T: CoordinateType + geo_types::CoordNum> Transform<T> for geo_types::Rect<T> {
    type Output = Self;

    fn transformed(&self, proj: &Proj) -> Result<Self, ProjError> {
        let mut output = *self;
        output.transform(proj)?;
        Ok(output)
    }

    fn transform(&mut self, proj: &Proj) -> Result<(), ProjError> {
        let converted = Self::new(proj.convert(self.min())?, proj.convert(self.max())?);
        *self = converted;
        Ok(())
    }
}

impl<T: CoordinateType + geo_types::CoordNum> Transform<T> for geo_types::Triangle<T> {
    type Output = Self;

    fn transformed(&self, proj: &Proj) -> Result<Self, ProjError> {
        let mut output = *self;
        output.transform(proj)?;
        Ok(output)
    }

    fn transform(&mut self, proj: &Proj) -> Result<(), ProjError> {
        let first = proj.convert(self.v1())?;
        let second = proj.convert(self.v2())?;
        let third = proj.convert(self.v3())?;
        *self = Self::unchecked_winding(first, second, third);
        Ok(())
    }
}
