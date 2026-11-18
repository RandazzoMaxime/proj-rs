# Porting notes: the hermetic context

Reference: PROJ 9.9.0. `pj_load_ini` (`src/filemanager.cpp`, 1988-2105), `pj_open_lib_internal`
(1544-1712), the `ONLY_BEST` default of `proj_create_crs_to_crs` (`src/crs_to_crs.cpp`, 494-535).

Code: `ContextBuilder::hermetic` (`src/api/context.rs`), `GridContext::{hermetic, load_with}`
(`src/network.rs`), `Lookup::hermetic` (`src/datadir.rs`), the chunk cache and the downloads
(`src/network.rs`, `src/network_chunks.rs`, `src/network_download.rs`), the default `ONLY_BEST` of
`Transformer` (`src/api/transformer.rs`). Tests: `tests/context_hermetic.rs`, and
`datadir::tests::hermetic_lookup_reads_only_the_search_path_or_the_explicit_directory`.

## Why

A PROJ context reads its environment and its `proj.ini` wherever they are: a service or a test
that builds a `Context` gets the network, another endpoint or `ONLY_BEST` from the machine it runs
on. `Context::builder().hermetic()` is the opposite: the context depends on what its builder was
given, and on nothing else. PROJ has no such mode; this is a deliberate deviation.

## What a hermetic context reads, and what it does not

Read: the search path, the user-writable directory set on the builder, the settings of the
builder, the embedded `proj.db`, and the file names the caller writes as paths (`/x`, `./x`,
`C:/x`).

Not read:

- `PROJ_NETWORK`, `PROJ_NETWORK_ENDPOINT`, `PROJ_ONLY_BEST_DEFAULT`;
- `proj.ini`, wherever PROJ would look: the search path, `PROJ_DATA` or `PROJ_LIB`, the
  user-writable directory, the directory of the executable (`../share/proj`), the compiled
  directory (`PROJ_RS_DATA_DIR`);
- `HOME`, for `~/` names;
- the working directory. PROJ's last resort in `pj_open_lib_internal` (a bare name, relative to
  the working directory) is skipped.

Defaults are those of PROJ with no `proj.ini` and no variable: network off, endpoint
`https://cdn.proj.org`, `ONLY_BEST` unset (PROJ's warning).

## Downloads and the chunk cache

PROJ writes downloads and `cache.db` to its user-writable directory. A hermetic context writes
them only to the directory its builder set. Without one, it cannot download and it keeps no cache:
`Context::cache()` then reports `Disabled`, and `Context::user_directory()` is empty. The cache
file is never resolved against the working directory.

## Order of the builder calls

`search_path`, `user_directory`, `endpoint`, `network`, `cache` and `hermetic` are stored on the
builder and applied by `build()`. The environment is read by `build()` only, and only when the
context is not hermetic, so `hermetic()` gives the same context wherever it is called.

## Behavior changes outside the hermetic mode

These change results for contexts that are not hermetic. Each one follows PROJ 9.9.0.

1. `proj.ini` in the builder's search path or user directory is read by `build()`. PROJ reads
   `proj.ini` on the first lookup, after `proj_context_set_search_paths`, through the paths in
   force then, so the search path replaces `PROJ_DATA` for it. Before, the builder read the file
   from the default places only, and not from the search path.
2. An unset `ONLY_BEST` (`OnlyBest::Default`) takes the context's default: `PROJ_ONLY_BEST_DEFAULT`,
   else `only_best_default` of `proj.ini`. `YES`, `ON` or `TRUE` make it `OnlyBest::Yes`, which
   fails on a missing grid; any other non-empty value makes it `OnlyBest::No`. This is what
   `proj_create_crs_to_crs` does when the option is not given (`crs_to_crs.cpp`, the
   `errorIfBestTransformationNotAvailableDefault` of `pj_load_ini`). Before, it was always the
   warning.

Reference runs for the second point, PROJ 9.9.0 `cs2cs` with `PROJ_DATA` set to its data
directory, which has no `us_noaa_conus.tif` (`target/proj-reference/build/data`):

    echo "40 -100" | cs2cs EPSG:4267 EPSG:4326 -f %.9f
    # variable unset, and PROJ_ONLY_BEST_DEFAULT=NO:
    39.999996882	-100.000415589 0.000000000
    # PROJ_ONLY_BEST_DEFAULT=YES: the operation fails, the point is `*	* inf`, and the message is
    # "Attempt to use coordinate operation NAD27 to WGS 84 (48) failed. Grid us_noaa_conus.tif is
    # not available."

## Open points

These are read from the process, not from the context, and a hermetic context does not stop them.

- `PROJ_AUX_DB` is read by the database layer (`src/projdb.rs`: `attach_aux`, `shared_read_only`,
  `open_lazy_or_copy`) for every connection to `proj.db`. The database entry points (`with_db`,
  `DatabaseContext::create`) take no context, and crs, nkg, defmodel and listing use them too.
  Closing this needs the context threaded through the database layer: a lot of its own.
- `PROJ_IGNORE_CELESTIAL_BODY` is read in `src/iso19111/factory.rs` and changes CRS creation.
  The same holds: process-wide, not a context setting.
- Tuning knobs, which select no data: `PROJ_FULL_FILE_CHUNK_SIZE` (`src/network_download.rs`),
  `PROJ_LOCK_MAX_ITERS` (`src/network_cache.rs`). `PROJ_GRID_CACHE_MAX_SIZE_BYTES` is read by the
  legacy `Proj` API (`src/proj.rs`), which is not a `Context`.
- The method is on the public `ContextBuilder` (`src/api/context.rs`), where `search_path` and
  the network settings live. The builder of `src/context.rs` (`proj_rs::unstable::ContextBuilder`)
  holds only the database settings and no grid setting, so it has no `hermetic()`.
- The legacy `Proj` and `ProjBuilder` (`src/proj.rs`) still use `GridContext::load()`. They are
  not hermetic.
- `proj_rs::iso19111::only_best_default()` and the `cs2cs` binary read `PROJ_ONLY_BEST_DEFAULT`:
  they are the command line's default, not a context's.

## Commands

    CARGO_PROFILE_DEV_DEBUG=0 cargo +1.100.0 test --all-features --test context_hermetic
    cargo +1.100.0 test --all-features --lib datadir
