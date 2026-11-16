use std::path::{Path, PathBuf};

use crate::network::GridContext;

/// Whether grids may be downloaded.
///
/// ```
/// use proj_rs::{Context, Network};
///
/// assert_eq!(Context::default().network(), Network::Disabled);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Network {
    /// No download. A grid that is not on disk is a missing grid.
    #[default]
    Disabled,
    /// A missing grid is downloaded from the endpoint of the [`Context`].
    Enabled,
}

/// Whether downloaded grid chunks are kept in the cache database.
///
/// ```
/// use proj_rs::{Cache, Context};
///
/// assert_eq!(Context::default().cache(), Cache::Disabled);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Cache {
    /// Nothing is read from or written to the cache.
    #[default]
    Disabled,
    /// Chunks are read from the cache when fresh and stored there after a download.
    Enabled,
}

/// The settings a [`Crs`](crate::Crs) and a [`Transformer`](crate::Transformer) are built
/// under: network, endpoint, search paths, user directory and grid cache.
///
/// The default context uses the embedded `proj.db`, no network, no extra search path and no
/// cache, whatever `PROJ_NETWORK` and `proj.ini` say. [`Context::from_environment`] reads them
/// as PROJ does. A context is a value: it is cheap to clone, and `Send + Sync`.
///
/// ```
/// use proj_rs::{Context, Network};
///
/// let ctx = Context::builder().network(Network::Enabled).build();
/// assert_eq!(ctx.network(), Network::Enabled);
/// assert_eq!(Context::default().network(), Network::Disabled);
/// ```
#[derive(Clone, Debug)]
pub struct Context {
    pub(crate) legacy: crate::context::Context,
    pub(crate) net: GridContext,
}

impl Default for Context {
    /// Network, extra search paths and cache all off.
    ///
    /// ```
    /// use proj_rs::{Cache, Context, Network};
    ///
    /// let ctx = Context::default();
    /// assert_eq!((ctx.network(), ctx.cache()), (Network::Disabled, Cache::Disabled));
    /// assert!(ctx.search_paths().is_empty());
    /// ```
    fn default() -> Self {
        Self::builder().build()
    }
}

impl Context {
    /// A builder that starts from [`Context::default`].
    ///
    /// ```
    /// use proj_rs::Context;
    ///
    /// let ctx = Context::builder().endpoint("https://example.org/grids").build();
    /// assert_eq!(ctx.endpoint(), "https://example.org/grids");
    /// ```
    #[must_use]
    pub fn builder() -> ContextBuilder {
        let mut net = GridContext::load();
        net.enabled = false;
        net.cache_enabled = false;
        ContextBuilder {
            context: Self {
                legacy: crate::context::Context::new(),
                net,
            },
        }
    }

    /// The context PROJ's own default would be: `PROJ_NETWORK`, `PROJ_NETWORK_ENDPOINT` and
    /// `proj.ini` are read.
    ///
    /// ```
    /// use proj_rs::Context;
    ///
    /// let ctx = Context::from_environment();
    /// assert!(!ctx.endpoint().is_empty());
    /// ```
    #[must_use]
    pub fn from_environment() -> Self {
        Self {
            legacy: crate::context::Context::new(),
            net: GridContext::load(),
        }
    }

    /// Whether grids may be downloaded.
    ///
    /// ```
    /// use proj_rs::{Context, Network};
    ///
    /// assert_eq!(Context::default().network(), Network::Disabled);
    /// ```
    #[must_use]
    pub fn network(&self) -> Network {
        if self.net.enabled {
            Network::Enabled
        } else {
            Network::Disabled
        }
    }

    /// The grid download endpoint, `https://cdn.proj.org` unless it was set.
    ///
    /// ```
    /// use proj_rs::Context;
    ///
    /// assert!(Context::default().endpoint().starts_with("https://"));
    /// ```
    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.net.endpoint
    }

    /// The search paths added to the context, in order. The data directories of the
    /// environment (`PROJ_DATA`) are searched before them.
    ///
    /// ```
    /// use proj_rs::Context;
    ///
    /// let ctx = Context::builder().search_path("/data/grids").build();
    /// assert_eq!(ctx.search_paths().len(), 1);
    /// ```
    #[must_use]
    pub fn search_paths(&self) -> &[PathBuf] {
        self.net.added_search_paths()
    }

    /// The directory downloaded grids and the cache database are written to.
    ///
    /// ```
    /// use proj_rs::Context;
    ///
    /// let ctx = Context::builder().user_directory("/tmp/proj-user").build();
    /// assert_eq!(ctx.user_directory(), std::path::Path::new("/tmp/proj-user"));
    /// ```
    #[must_use]
    pub fn user_directory(&self) -> PathBuf {
        self.net.cache_directory()
    }

    /// Whether downloaded grid chunks are cached.
    ///
    /// ```
    /// use proj_rs::{Cache, Context};
    ///
    /// assert_eq!(Context::default().cache(), Cache::Disabled);
    /// ```
    #[must_use]
    pub fn cache(&self) -> Cache {
        if self.net.cache_enabled {
            Cache::Enabled
        } else {
            Cache::Disabled
        }
    }
}

/// Builds a [`Context`]. Made by [`Context::builder`].
///
/// ```
/// use proj_rs::{Cache, Context, Network};
///
/// let ctx = Context::builder()
///     .network(Network::Enabled)
///     .cache(Cache::Enabled)
///     .build();
/// assert_eq!(ctx.cache(), Cache::Enabled);
/// ```
#[derive(Clone, Debug)]
pub struct ContextBuilder {
    context: Context,
}

impl ContextBuilder {
    /// Allow or forbid grid download.
    ///
    /// ```
    /// use proj_rs::{Context, Network};
    ///
    /// let ctx = Context::builder().network(Network::Enabled).build();
    /// assert_eq!(ctx.network(), Network::Enabled);
    /// ```
    #[must_use]
    pub fn network(mut self, network: Network) -> Self {
        self.context.net.enabled = network == Network::Enabled;
        self
    }

    /// The grid download endpoint. A missing grid is requested at `endpoint/<file name>`.
    ///
    /// ```
    /// use proj_rs::Context;
    ///
    /// let ctx = Context::builder().endpoint("https://example.org").build();
    /// assert_eq!(ctx.endpoint(), "https://example.org");
    /// ```
    #[must_use]
    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.context.net.endpoint = endpoint.into();
        self
    }

    /// Add a grid search path, after the ones already given.
    ///
    /// ```
    /// use proj_rs::Context;
    ///
    /// let ctx = Context::builder().search_path("/a").search_path("/b").build();
    /// assert_eq!(ctx.search_paths().len(), 2);
    /// ```
    #[must_use]
    pub fn search_path(mut self, path: impl AsRef<Path>) -> Self {
        self.context
            .net
            .push_search_path(path.as_ref().to_path_buf());
        self
    }

    /// The directory downloaded grids and the cache database are written to.
    ///
    /// ```
    /// use proj_rs::Context;
    ///
    /// let ctx = Context::builder().user_directory("/tmp/u").build();
    /// assert!(ctx.user_directory().ends_with("u"));
    /// ```
    #[must_use]
    pub fn user_directory(mut self, path: impl AsRef<Path>) -> Self {
        self.context.net.set_cache_dir(path.as_ref().to_path_buf());
        self
    }

    /// Keep or drop the cache of downloaded grid chunks.
    ///
    /// ```
    /// use proj_rs::{Cache, Context};
    ///
    /// let ctx = Context::builder().cache(Cache::Enabled).build();
    /// assert_eq!(ctx.cache(), Cache::Enabled);
    /// ```
    #[must_use]
    pub fn cache(mut self, cache: Cache) -> Self {
        self.context.net.cache_enabled = cache == Cache::Enabled;
        self
    }

    /// The context.
    ///
    /// ```
    /// use proj_rs::Context;
    ///
    /// let _ctx = Context::builder().build();
    /// ```
    #[must_use]
    pub fn build(self) -> Context {
        self.context
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_is_send_and_sync() {
        fn is_send_sync<T: Send + Sync>() {}
        is_send_sync::<Context>();
    }
}
