//! The grid records a context has looked up, as PROJ keeps them: `cacheGridInfo_` of the
//! `DatabaseContext` of a context (`factory.cpp:860-871, 1222-1238`), read and filled by
//! `lookForGridInfo` (`factory.cpp:3398-3539`).
//!
//! An entry is keyed by the grid name, whether the network was on, and
//! `considerKnownGridsAsAvailable` (`makeCachedGridKey`). A grid that did not open is not tried
//! again by the same context while its entry is in the cache, so the `Cannot open <url>: <error>`
//! of a failed network open is logged once per entry. The cache is `lru11::Cache` of
//! `lru_cache.hpp:109-217` with PROJ's sizes: 128 keys, allowed to grow to 138 before it is
//! pruned back to the 128 most recently used. A read moves its key to the front, as an insert
//! does. `invalidateGridInfo` drops the four keys of a grid once it is downloaded
//! (`factory.cpp:3389-3396`, `networkfilemanager.cpp:2704`).

use std::cell::Cell;
use std::sync::Mutex;

use crate::iso19111::GridLookup;

/// `CACHE_SIZE` of `DatabaseContext::Private` (`factory.cpp:860`).
const MAX_SIZE: usize = 128;
/// The default elasticity of `lru11::Cache` (`lru_cache.hpp:109`).
const ELASTICITY: usize = 10;

/// `makeCachedGridKey`: the grid name, whether the network was on, and
/// `considerKnownGridsAsAvailable`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GridKey {
    pub(crate) name: String,
    pub(crate) network: bool,
    pub(crate) known: bool,
}

/// The grid records of one context, most recently used last.
#[derive(Debug, Default)]
pub(crate) struct GridInfoCache {
    entries: Mutex<Vec<(GridKey, GridLookup)>>,
}

impl GridInfoCache {
    /// `tryGet`: the record of `key`, which becomes the most recently used.
    pub(crate) fn get(&self, key: &GridKey) -> Option<GridLookup> {
        let mut entries = self.lock();
        let at = entries.iter().position(|(existing, _)| existing == key)?;
        if records_untouched() {
            return Some(entries[at].1.clone());
        }
        let entry = entries.remove(at);
        let found = entry.1.clone();
        entries.push(entry);
        Some(found)
    }

    /// `insert`: the record becomes the most recently used. A new key beyond the 138 allowed
    /// prunes the least recently used ones down to 128.
    pub(crate) fn insert(&self, key: GridKey, lookup: GridLookup) {
        if records_untouched() {
            return;
        }
        let mut entries = self.lock();
        if let Some(at) = entries.iter().position(|(existing, _)| *existing == key) {
            entries.remove(at);
            entries.push((key, lookup));
            return;
        }
        entries.push((key, lookup));
        if entries.len() > MAX_SIZE + ELASTICITY {
            let excess = entries.len() - MAX_SIZE;
            entries.drain(..excess);
        }
    }

    /// `invalidateGridInfo`: the four records of `name` are dropped.
    pub(crate) fn invalidate(&self, name: &str) {
        self.lock().retain(|(key, _)| key.name != name);
    }

    /// The number of records, for the tests.
    #[cfg(test)]
    fn len(&self) -> usize {
        self.lock().len()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<(GridKey, GridLookup)>> {
        self.entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

thread_local! {
    static SUSPENDED: Cell<u32> = const { Cell::new(0) };
}

/// `body` with the network of the current context off, then on again as it was: the
/// `proj_context_set_enable_network(ctx, false)` that PROJ wraps around a lookup that must not
/// reach the network (`DisableNetwork` of `rejectOpDueToMissingGrid`, `factory.cpp:4333-4359`;
/// the nested `lookForGridInfo` of `open_resource_file`, `filemanager.cpp:1871-1879`; the local
/// lookup of `lookForGridInfo` under `considerKnownGridsAsAvailable`, `factory.cpp:3503-3507`).
pub(crate) fn with_network_off<T>(body: impl FnOnce() -> T) -> T {
    struct Restore;
    impl Drop for Restore {
        fn drop(&mut self) {
            SUSPENDED.with(|depth| depth.set(depth.get().saturating_sub(1)));
        }
    }
    SUSPENDED.with(|depth| depth.set(depth.get() + 1));
    let _restore = Restore;
    body()
}

thread_local! {
    static UNTOUCHED: Cell<u32> = const { Cell::new(0) };
}

/// `body` reading the grid records without changing them: a record found is not moved to the
/// front, and a lookup that finds none is not recorded. For the lookups of this port that PROJ
/// does not make: the second ranking of the candidates of `proj_create_crs_to_crs`, which reads
/// the characteristics of the list `createOperations` sorted already.
pub(crate) fn with_records_untouched<T>(body: impl FnOnce() -> T) -> T {
    struct Restore;
    impl Drop for Restore {
        fn drop(&mut self) {
            UNTOUCHED.with(|depth| depth.set(depth.get().saturating_sub(1)));
        }
    }
    UNTOUCHED.with(|depth| depth.set(depth.get() + 1));
    let _restore = Restore;
    body()
}

fn records_untouched() -> bool {
    UNTOUCHED.with(Cell::get) > 0
}

/// Whether [`with_network_off`] is running on this thread.
pub(crate) fn network_suspended() -> bool {
    SUSPENDED.with(Cell::get) > 0
}

#[cfg(test)]
mod tests {
    use super::{GridInfoCache, GridKey};
    use crate::iso19111::{GridInfo, GridLookup};

    /// The network on, `considerKnownGridsAsAvailable` off: the lookups of the sort.
    const SORT: (bool, bool) = (true, false);
    /// The network off, `considerKnownGridsAsAvailable` on: the lookup before a network open.
    const BEFORE_OPEN: (bool, bool) = (false, true);
    const DISCARD: (bool, bool) = (false, false);

    fn key(name: &str, (network, known): (bool, bool)) -> GridKey {
        GridKey {
            name: name.to_owned(),
            network,
            known,
        }
    }

    fn record(available: Option<()>) -> GridLookup {
        GridLookup {
            in_database: true,
            info: GridInfo {
                full_name: String::new(),
                package_name: String::new(),
                url: String::new(),
                direct_download: true,
                open_license: true,
                available: available.is_some(),
            },
        }
    }

    fn name(index: usize) -> String {
        format!("grid_{index}.tif")
    }

    /// `lru11::Cache(128, 10)`: 138 keys stay, the 139th prunes the 11 oldest.
    #[test]
    fn grows_to_138_then_keeps_the_128_most_recent() {
        let cache = GridInfoCache::default();
        for index in 0..138 {
            cache.insert(key(&name(index), SORT), record(None));
        }
        assert_eq!(cache.len(), 138);
        assert!(cache.get(&key(&name(0), SORT)).is_some());
        cache.insert(key(&name(138), SORT), record(None));
        assert_eq!(cache.len(), 128);
        // grid_0 was read last before the insert: it stays, grid_1 to grid_11 go.
        assert!(cache.get(&key(&name(0), SORT)).is_some());
        for index in 1..=11 {
            assert!(cache.get(&key(&name(index), SORT)).is_none(), "{index}");
        }
        assert!(cache.get(&key(&name(12), SORT)).is_some());
    }

    /// The network state and `considerKnownGridsAsAvailable` are part of the key, and
    /// `invalidateGridInfo` drops the four records of a name.
    #[test]
    fn keys_carry_the_network_and_known_flags() {
        let cache = GridInfoCache::default();
        cache.insert(key("a.tif", SORT), record(None));
        cache.insert(key("a.tif", BEFORE_OPEN), record(Some(())));
        cache.insert(key("b.tif", SORT), record(None));
        assert!(cache.get(&key("a.tif", DISCARD)).is_none());
        assert!(!cache.get(&key("a.tif", SORT)).expect("a").info.available);
        assert!(
            cache
                .get(&key("a.tif", BEFORE_OPEN))
                .expect("a")
                .info
                .available
        );
        cache.invalidate("a.tif");
        assert!(cache.get(&key("a.tif", SORT)).is_none());
        assert!(cache.get(&key("a.tif", BEFORE_OPEN)).is_none());
        assert!(cache.get(&key("b.tif", SORT)).is_some());
    }
}
