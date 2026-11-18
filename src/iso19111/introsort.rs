//! `std::sort` of libstdc++, which `FilterResults::sort` calls on the candidates
//! (coordinateoperationfactory.cpp:1552). The sort is not stable: the order of the elements that
//! compare equal, and so the copy kept by `removeUninterestingOps`, is the one of this algorithm.

const THRESHOLD: usize = 16;

/// The permutation `std::sort(first, last, less)` applies to `len` elements: the original index
/// of each element of the result, in order.
pub(crate) fn sorted_indices(len: usize, less: impl Fn(usize, usize) -> bool) -> Vec<usize> {
    let mut order: Vec<usize> = (0..len).collect();
    if len > 1 {
        let depth = usize::try_from(2 * len.ilog2()).unwrap_or(usize::MAX);
        let mut sorter = Sorter {
            order: &mut order,
            less: &less,
        };
        sorter.introsort_loop(0, len, depth);
        sorter.final_insertion_sort(len);
    }
    order
}

struct Sorter<'a, F: Fn(usize, usize) -> bool> {
    order: &'a mut [usize],
    less: &'a F,
}

impl<F: Fn(usize, usize) -> bool> Sorter<'_, F> {
    fn lt(&self, left: usize, right: usize) -> bool {
        (self.less)(self.order[left], self.order[right])
    }

    fn introsort_loop(&mut self, first: usize, mut last: usize, mut depth: usize) {
        while last - first > THRESHOLD {
            if depth == 0 {
                self.partial_sort(first, last);
                return;
            }
            depth -= 1;
            let cut = self.unguarded_partition_pivot(first, last);
            self.introsort_loop(cut, last, depth);
            last = cut;
        }
    }

    fn unguarded_partition_pivot(&mut self, first: usize, last: usize) -> usize {
        let mid = first + (last - first) / 2;
        self.move_median_to_first(first, first + 1, mid, last - 1);
        self.unguarded_partition(first + 1, last, first)
    }

    fn move_median_to_first(&mut self, result: usize, a: usize, b: usize, c: usize) {
        if self.lt(a, b) {
            if self.lt(b, c) {
                self.order.swap(result, b);
            } else if self.lt(a, c) {
                self.order.swap(result, c);
            } else {
                self.order.swap(result, a);
            }
        } else if self.lt(a, c) {
            self.order.swap(result, a);
        } else if self.lt(b, c) {
            self.order.swap(result, c);
        } else {
            self.order.swap(result, b);
        }
    }

    fn unguarded_partition(&mut self, mut first: usize, mut last: usize, pivot: usize) -> usize {
        loop {
            while self.lt(first, pivot) {
                first += 1;
            }
            last -= 1;
            while self.lt(pivot, last) {
                last -= 1;
            }
            if first >= last {
                return first;
            }
            self.order.swap(first, last);
            first += 1;
        }
    }

    fn final_insertion_sort(&mut self, last: usize) {
        if last > THRESHOLD {
            self.insertion_sort(0, THRESHOLD);
            for index in THRESHOLD..last {
                self.unguarded_linear_insert(index);
            }
        } else {
            self.insertion_sort(0, last);
        }
    }

    fn insertion_sort(&mut self, first: usize, last: usize) {
        for index in first + 1..last {
            if self.lt(index, first) {
                self.order[first..=index].rotate_right(1);
            } else {
                self.unguarded_linear_insert(index);
            }
        }
    }

    fn unguarded_linear_insert(&mut self, mut index: usize) {
        while self.lt(index, index - 1) {
            self.order.swap(index, index - 1);
            index -= 1;
        }
    }

    fn partial_sort(&mut self, first: usize, last: usize) {
        self.make_heap(first, last);
        let mut end = last;
        while end - first > 1 {
            end -= 1;
            let value = self.order[end];
            self.order[end] = self.order[first];
            self.adjust_heap(first, 0, end - first, value);
        }
    }

    fn make_heap(&mut self, first: usize, last: usize) {
        let len = last - first;
        if len < 2 {
            return;
        }
        let mut parent = (len - 2) / 2;
        loop {
            let value = self.order[first + parent];
            self.adjust_heap(first, parent, len, value);
            if parent == 0 {
                return;
            }
            parent -= 1;
        }
    }

    fn adjust_heap(&mut self, first: usize, hole: usize, len: usize, value: usize) {
        let top = hole;
        let mut hole = hole;
        let mut child = hole;
        while child < len.saturating_sub(1) / 2 {
            child = 2 * (child + 1);
            if self.lt(first + child, first + child - 1) {
                child -= 1;
            }
            self.order[first + hole] = self.order[first + child];
            hole = child;
        }
        if crate::compat::is_multiple_of(len, 2) && child == (len - 2) / 2 {
            child = 2 * (child + 1);
            self.order[first + hole] = self.order[first + child - 1];
            hole = child - 1;
        }
        let mut parent = hole.saturating_sub(1) / 2;
        while hole > top && (self.less)(self.order[first + parent], value) {
            self.order[first + hole] = self.order[first + parent];
            hole = parent;
            parent = hole.saturating_sub(1) / 2;
        }
        self.order[first + hole] = value;
    }
}
