//! One `Arc<Crs>` per CRS of a search: the operations of a search point to the CRS the search
//! was given instead of each owning a copy.

use std::cell::RefCell;
use std::sync::Arc;

use super::model::Crs;

thread_local! {
    static REGISTERED: RefCell<Vec<Arc<Crs>>> = const { RefCell::new(Vec::new()) };
}

/// Keeps CRS registered on this thread until it drops.
#[derive(Debug)]
pub(crate) struct Scope {
    registered_before: usize,
}

/// Register `crs` for [`Share`] until the returned scope drops. A registered CRS is borrowed
/// through its `Arc` (`&*arc`), and every `&Crs` that is that borrow finds the `Arc` again.
pub(crate) fn enter<'a>(crs: impl IntoIterator<Item = &'a Arc<Crs>>) -> Scope {
    REGISTERED.with(|registered| {
        let mut registered = registered.borrow_mut();
        let registered_before = registered.len();
        registered.extend(crs.into_iter().cloned());
        Scope { registered_before }
    })
}

impl Drop for Scope {
    fn drop(&mut self) {
        REGISTERED.with(|registered| registered.borrow_mut().truncate(self.registered_before));
    }
}

impl Scope {
    /// Whether no other search of this thread is registered: the scope of the outermost call.
    pub(crate) fn is_outermost(&self) -> bool {
        self.registered_before == 0
    }
}

/// A CRS an operation stores as its source, target or interpolation CRS.
pub(crate) trait Share {
    fn share(self) -> Arc<Crs>;
}

impl Share for Arc<Crs> {
    fn share(self) -> Arc<Crs> {
        self
    }
}

impl Share for &Arc<Crs> {
    fn share(self) -> Arc<Crs> {
        Arc::clone(self)
    }
}

impl Share for Crs {
    fn share(self) -> Arc<Crs> {
        Arc::new(self)
    }
}

/// The registered `Arc` when `self` is the CRS it holds (same address, so same object), else a
/// copy.
impl Share for &Crs {
    fn share(self) -> Arc<Crs> {
        REGISTERED
            .with(|registered| {
                registered
                    .borrow()
                    .iter()
                    .rev()
                    .find(|known| std::ptr::eq(Arc::as_ptr(known), self))
                    .cloned()
            })
            .unwrap_or_else(|| Arc::new(self.clone()))
    }
}
