//! What one search reads from `proj.db` is kept until the search ends.
//!
//! A search for the operations between two CRS asks the database for the same row many times:
//! the datum behind an ensemble member, the name of a CRS, the extent of an object. This cache
//! keeps the rows of each query and the CRS built for the operations, so the second ask costs a
//! lookup. It lives in the connection of a `DatabaseContext`, holds nothing while no search runs,
//! and is emptied when the last search of the context ends and on any write.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use super::factory::Cell;
use super::model::Crs;

#[cfg(test)]
mod tests;

#[cfg(test)]
thread_local! {
    /// Set by a test to run the searches of its thread as if the cache did not exist.
    static OFF: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Most CRS the cache keeps: a pair of CRS and the pivots of a search fit well within it.
const CRS_CAPACITY: usize = 64;

/// Most bytes of query rows the cache keeps (the largest of 30 pairs measured fills half of it).
/// A search that reads more keeps what fit first.
const ROW_BUDGET: usize = 512 << 10;

/// Longest result worth keeping: a longer one costs more to copy than to read again.
const MAX_ROWS: usize = 64;

/// The rows of one query text, by the parameters it ran with.
type ByParams = HashMap<Box<[String]>, Vec<Vec<Cell>>>;

/// `(authority, code)` of a CRS built by `AuthorityFactory::create_shared_crs`.
type CrsKey = (String, String);

#[derive(Default)]
struct State {
    crs: Vec<(CrsKey, Arc<Crs>)>,
    rows: HashMap<Box<str>, ByParams>,
    row_bytes: usize,
}

/// The rows and CRS read by the searches now running on one database.
#[derive(Default)]
pub(super) struct SearchCache {
    /// Searches running on this database; the cache fills only while it is above zero.
    searching: AtomicUsize,
    /// Bumped by every `clear`: a read that began before a write does not store its rows after.
    generation: AtomicU64,
    state: Mutex<State>,
    /// Lookups answered by the cache.
    #[cfg(test)]
    hits: AtomicUsize,
}

/// Where a miss was read: the store drops what it read if a write came in between.
#[derive(Clone, Copy)]
pub(super) struct Ticket(Option<u64>);

impl SearchCache {
    fn state(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    fn active(&self) -> bool {
        #[cfg(test)]
        if OFF.with(std::cell::Cell::get) {
            return false;
        }
        self.searching.load(Ordering::Acquire) > 0
    }

    /// A search starts on this database.
    pub(super) fn begin(&self) {
        self.searching.fetch_add(1, Ordering::AcqRel);
    }

    /// A search ends; the last one empties the cache.
    pub(super) fn end(&self) {
        if self.searching.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.clear();
        }
    }

    /// Drop everything. Called on a write and when the last search ends.
    pub(super) fn clear(&self) {
        self.generation.fetch_add(1, Ordering::AcqRel);
        *self.state() = State::default();
    }

    /// The rows kept for `sql` with `params`.
    pub(super) fn rows(&self, sql: &str, params: &[String]) -> Option<Vec<Vec<Cell>>> {
        if !self.active() {
            return None;
        }
        let rows = self.state().rows.get(sql)?.get(params).cloned();
        #[cfg(test)]
        if rows.is_some() {
            self.hits.fetch_add(1, Ordering::Relaxed);
        }
        rows
    }

    /// Mark a miss, before it reads the database.
    pub(super) fn ticket(&self) -> Ticket {
        Ticket(
            self.active()
                .then(|| self.generation.load(Ordering::Acquire)),
        )
    }

    /// Keep the `rows` that `sql` with `params` read, if the cache is open and has room.
    pub(super) fn store_rows(
        &self,
        ticket: Ticket,
        sql: &str,
        params: &[String],
        rows: &[Vec<Cell>],
    ) {
        let Ticket(Some(generation)) = ticket else {
            return;
        };
        if rows.len() > MAX_ROWS {
            return;
        }
        let size = size_of_rows(rows) + params.iter().map(String::len).sum::<usize>();
        let mut state = self.state();
        if self.generation.load(Ordering::Acquire) != generation
            || state.row_bytes + size > ROW_BUDGET
        {
            return;
        }
        state.row_bytes += size;
        let by_params = match state.rows.get_mut(sql) {
            Some(by_params) => by_params,
            None => {
                state.row_bytes += sql.len();
                state.rows.entry(sql.into()).or_default()
            }
        };
        by_params.insert(params.into(), rows.to_vec());
    }

    /// The CRS kept for `key`.
    pub(super) fn crs(&self, key: &CrsKey) -> Option<Arc<Crs>> {
        if !self.active() {
            return None;
        }
        let crs = self
            .state()
            .crs
            .iter()
            .find(|(known, _)| known == key)
            .map(|(_, crs)| Arc::clone(crs));
        #[cfg(test)]
        if crs.is_some() {
            self.hits.fetch_add(1, Ordering::Relaxed);
        }
        crs
    }

    /// Keep `crs` for `key`, dropping the oldest CRS when the cache is full.
    pub(super) fn store_crs(&self, ticket: Ticket, key: CrsKey, crs: &Arc<Crs>) {
        let Ticket(Some(generation)) = ticket else {
            return;
        };
        let mut state = self.state();
        if self.generation.load(Ordering::Acquire) != generation {
            return;
        }
        if state.crs.len() >= CRS_CAPACITY {
            state.crs.remove(0);
        }
        state.crs.push((key, Arc::clone(crs)));
    }
}

#[cfg(test)]
impl SearchCache {
    /// Entries held: query results and CRS.
    pub(super) fn entries(&self) -> usize {
        let state = self.state();
        state.rows.values().map(HashMap::len).sum::<usize>() + state.crs.len()
    }

    pub(super) fn hits(&self) -> usize {
        self.hits.load(Ordering::Relaxed)
    }
}

fn size_of_rows(rows: &[Vec<Cell>]) -> usize {
    rows.iter()
        .map(|row| {
            std::mem::size_of::<Vec<Cell>>()
                + row
                    .iter()
                    .map(|cell| {
                        std::mem::size_of::<Cell>()
                            + match cell {
                                Cell::Text(text) => text.len(),
                                _ => 0,
                            }
                    })
                    .sum::<usize>()
        })
        .sum()
}
