//! `Proj::convert_iter`: the points of an iterator, converted in blocks through the vector path.

use super::{Coord, CoordinateType, Proj, ProjError, SMALL_BATCH};

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
        // No room yet: an iterator of a few points never needs a block of 1024.
        block: Vec::new(),
        originals: Vec::new(),
        out: Vec::new().into_iter(),
        small: std::array::from_fn(|_| None),
        small_next: 0,
        small_len: 0,
    }
}

struct ConvertIter<'a, I, C, F> {
    proj: &'a Proj,
    points: I,
    block: Vec<C>,
    originals: Vec<(F, F)>,
    out: std::vec::IntoIter<Result<C, ProjError>>,
    /// The first points of an input too short for a block, kept here: no allocation for them.
    small: [Option<C>; SMALL_BATCH],
    small_next: usize,
    small_len: usize,
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
            // A short input is converted point by point, as it is asked for.
            if self.small_next < self.small_len {
                let point = self.small[self.small_next].take();
                self.small_next += 1;
                if let Some(point) = point {
                    return Some(self.proj.convert(point));
                }
                continue;
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
        self.small_next = 0;
        self.small_len = 0;
        for slot in &mut self.small {
            match self.points.next() {
                Some(point) => {
                    *slot = Some(point);
                    self.small_len += 1;
                }
                None => break,
            }
        }
        if self.small_len == 0 {
            return false;
        }
        if self.small_len < SMALL_BATCH {
            // The input ended: too few points for the vector path.
            return true;
        }
        // A small batch and perhaps more: a block, which grows as the points come.
        self.block.clear();
        self.originals.clear();
        self.small_len = 0;
        for slot in &mut self.small {
            if let Some(point) = slot.take() {
                self.originals.push((point.x(), point.y()));
                self.block.push(point);
            }
        }
        for point in self.points.by_ref().take(BLOCK - SMALL_BATCH) {
            self.originals.push((point.x(), point.y()));
            self.block.push(point);
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
