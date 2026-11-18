//! `Proj::convert_iter`: the points of an iterator, converted in blocks through the vector path.

use super::{Coord, CoordinateType, Proj, ProjError};

/// Points taken from the iterator, and converted at once. Large enough for the vector kernels,
/// small enough that a block stays in the cache of the processor.
const BLOCK: usize = 1024;

pub(crate) fn convert_iter<'a, I, C, F>(
    proj: &'a Proj,
    points: I,
) -> impl Iterator<Item = Result<C, ProjError>> + 'a
where
    I: IntoIterator<Item = C>,
    I::IntoIter: 'a,
    C: Coord<F> + 'a,
    F: CoordinateType + 'a,
{
    ConvertIter {
        proj,
        points: points.into_iter(),
        block: Vec::with_capacity(BLOCK),
        originals: Vec::with_capacity(BLOCK),
        out: Vec::new().into_iter(),
    }
}

struct ConvertIter<'a, I, C, F> {
    proj: &'a Proj,
    points: I,
    block: Vec<C>,
    originals: Vec<(F, F)>,
    out: std::vec::IntoIter<Result<C, ProjError>>,
}

impl<I, C, F> Iterator for ConvertIter<'_, I, C, F>
where
    I: Iterator<Item = C>,
    C: Coord<F>,
    F: CoordinateType,
{
    type Item = Result<C, ProjError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(item) = self.out.next() {
                return Some(item);
            }
            if !self.refill() {
                return None;
            }
        }
    }
}

impl<I, C, F> ConvertIter<'_, I, C, F>
where
    I: Iterator<Item = C>,
    C: Coord<F>,
    F: CoordinateType,
{
    /// Convert the next block of input. `false` once the input is spent.
    fn refill(&mut self) -> bool {
        self.block.clear();
        self.originals.clear();
        for point in self.points.by_ref().take(BLOCK) {
            self.originals.push((point.x(), point.y()));
            self.block.push(point);
        }
        if self.block.is_empty() {
            return false;
        }
        self.out = if self.proj.convert_array(&mut self.block).is_ok() {
            self.block.drain(..).map(Ok).collect::<Vec<_>>().into_iter()
        } else {
            // A failing block is partly converted: each point is converted again on its own,
            // so each item is what `convert` gives for its point, the failing ones included.
            self.originals
                .iter()
                .map(|&(x, y)| self.proj.convert(C::from_xy(x, y)))
                .collect::<Vec<_>>()
                .into_iter()
        };
        true
    }
}
