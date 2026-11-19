# Porting notes: where a file is looked for, and the user directory

Reference: PROJ 9.9.0 (tag `9.9.0` of `OSGeo/PROJ`). Operator decision: "do as PROJ does for
`PROJ_DATA`, for the release". Until this lot the default data directory was
`env!("CARGO_MANIFEST_DIR")/data/grids`, a path of the build machine fixed at compile time, and
the user directory was `%LOCALAPPDATA%\proj`, else `temp_dir()/proj` (`/tmp/proj` on Linux and
macOS, emptied at reboot). A binary built by CI looked for its grids on the CI machine.

Code: `src/datadir.rs` (the order and the directories), `src/gridpath.rs` (`resolve`, the
`grid_alternatives` rename), `src/network.rs` (`GridContext`, `proj.ini`).

## The order of `pj_open_lib_internal` (`src/filemanager.cpp`, 1544-1712)

The body is one `if / else if` chain. **The first branch whose condition holds decides.** Only
the user-writable directory, `../share/proj` and the compiled directory continue to the next
branch when the file is not there, because their condition includes opening the file. The
`PROJ_DATA` branch and the search-path branch end the search, found or not.

| # | PROJ | Line | Rust (`Lookup::find`) |
|---|---|---|---|
| 1 | `~/name`: `$HOME` + `/` + name, the name keeping its `~/`. No `$HOME`: not found | 1577-1585 | `is_tilde_slash`, `home` |
| 2 | `/name`, `./name`, `../name`, `C:/name` (and `.`, `..`, the empty name, which `strchr` of the NUL matches) | 1586-1596, 297-304 | `is_rel_or_absolute_filename` |
| 3 | `http://`, `https://`: the URL | 1597 | `gridpath::locate` returns none; the network code reads URLs |
| 4 | the file finder, when it returns a path | 1601-1606 | `Lookup::finder` |
| 5 | **paths of the context** (`proj_context_set_search_paths`): each + `/` + name, nothing else | 1608-1620 | `search_paths` |
| 6 | the user-writable directory, unless `PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY` is not empty | 1622-1634, 1538-1542 | `user_dir` |
| 7 | `PROJ_DATA`, else `PROJ_LIB` (empty = unset, an empty `PROJ_DATA` hides `PROJ_LIB`): split on `:` (`;` on Windows), empty entries kept, one pair of quotes stripped, first directory that has the file. **Ends the search** | 1637-1642, 1113-1131 | `proj_data` |
| 8 | `../share/proj` of the module, when it exists and has the file | 1643, 1401-1513 | `relative_share` |
| 9 | the compiled `PROJ_DATA` directory, when it has the file | 1645-1655 | `compiled` |
| 10 | (`PROJ_DATA_ENV_VAR_TRIED_LAST`, off by default: `PROJ_DATA` after 8 and 9) | 1657-1662 | not ported |
| 11 | the bare name, relative to the working directory | 1665-1668 | last line of `find` |

`proj.ini` (`pj_load_ini`, 2046) and the dictionaries of `+init=` take the same chain, so
`GridContext::load` reads `proj.ini` through `Lookup::find` as well, and `network::find_file`
is `gridpath::resolve`.

`FileManager::open_resource_file` (1786-1857) retries with the `grid_alternatives` name unless
the name is a path, a URL, `~/...` or `proj.db`: `gridpath::resolve`.

## `proj_info().searchpath` (`pj_get_default_searchpaths`, 1718-1759; `info.cpp`, 112-129)

The paths of the context when there are any, else: the user-writable directory (not when
skipped), then `PROJ_DATA` **as written, unsplit** when set, else `../share/proj` (when it
exists), then the compiled directory. Empty entries are dropped. `projinfo --searchpaths`
(`projinfo_lib.cpp`, 1861) prints them one per line. `Lookup::searchpath` does the same; the
test `search_paths_are_those_of_projinfo_of_proj` runs both `projinfo` on the same environments.
`library_info()` used to leave the user directory out and to list `data/grids`.

## The user-writable directory (`proj_context_get_user_writable_directory`, 1252-1321)

| Step | PROJ | Rust (`user_directory_from`) |
|---|---|---|
| 1 | `PROJ_USER_WRITABLE_DIRECTORY` if not empty ("for testing purposes") | same |
| 2 Windows | `SHGetKnownFolderPath(FOLDERID_LocalAppData)`, else `%LOCALAPPDATA%`, else `%TEMP%`, else `c:/users` | `%LOCALAPPDATA%`, else `%TEMP%`, else `c:/users` (see deviations) |
| 2 other | `$XDG_DATA_HOME` **when set, even empty** (an empty one gives `/proj`) | same |
|   | else `$HOME` when `access(home, W_OK) == 0`: macOS `$HOME/Library/Application Support`, else `$HOME/.local/share` | same, `access` called through `rustix::fs::access` (safe API, same system call) |
|   | else `/tmp` | same |
| 3 | `+ "/proj"` (a forward slash on Windows too: `C:\Users\u\AppData\Local/proj`) | same string |

The directory is **created only when asked** (`create` true: `proj_context_get_user_writable_directory(ctx, true)`,
`proj_context_set_user_writable_directory(.., create)`, the download and the cache database),
never when read. Reading it for a lookup or for `projinfo --searchpaths` creates nothing. The
`PROJ_USER_WRITABLE_DIRECTORY` and the other variables are read at each call here; PROJ keeps the
first value in the context. `cache.db` and the downloaded files were already written under
`GridContext::cache_directory`, which is this directory: they now land in
`~/.local/share/proj` and not in `/tmp/proj`.

## `../share/proj` (`pj_get_relative_share_proj`, 1401-1513)

PROJ takes the path of the **module that holds the function** (`dladdr`, `GetModuleHandleEx`):
`libproj.so` for a shared PROJ, the executable for a static one. It keeps what is before the
last two separators and appends `/share/proj`: `<prefix>/lib/libproj.so` and `<prefix>/bin/tool`
both give `<prefix>/share/proj`. `/bin/../` is folded and a leading `./` dropped on Unix; a name
with one directory gives `share/proj`, none gives `../share/proj`, both relative to the working
directory (`relative_share_proj_for` keeps these, tested). The result is computed **once per
process** (a `static`) and kept only when the directory exists. `<exe dir>/share/proj` is **not** a
PROJ location.

proj-rs is linked into the executable, so the module is the executable
(`std::env::current_exe()`, absolute, where PROJ gives the `argv[0]` of `dladdr` and reads
`/proc/self/exe` only for a static binary). The C API (`libproj_rs_capi`, a shared library) names
its own file with `proj_rs::info::set_module_path` (`dladdr` on Unix, `GetModuleHandleExW` on
Windows), so `<prefix>/lib/libproj.so.25` finds `<prefix>/share/proj` as `libproj` does.

## The compiled directory: `PROJ_RS_DATA_DIR`

PROJ compiles `CMAKE_INSTALL_FULL_DATADIR/proj` in with `EMBED_PROJ_DATA_PATH` (default on,
`src/lib_proj.cmake`, 311-313). proj-rs has no installation. Options weighed:

1. The source tree (`CARGO_MANIFEST_DIR/data/grids`): **rejected**, it is the bug.
2. Nothing but `../share/proj`: a distributor that installs grids elsewhere has no way.
3. An optional build-time variable, absent by default: **chosen.** `PROJ_RS_DATA_DIR=/usr/share/proj
   cargo build` is read by `option_env!` (Cargo rebuilds when it changes) and is the counterpart of
   `-DCMAKE_INSTALL_PREFIX`. No variable, no compiled directory; no path of the build machine.

So by default a binary searches the user directory, `PROJ_DATA` or `../share/proj`, and the
working directory: the layout of PROJ's own release tarballs (`bin/`, `share/proj/`).
The tests are unaffected: they put the fixtures on `PROJ_DATA` themselves
(`tests/common/mod.rs`, `gridpath::use_fixture_search_path`), as `proj_test_set_properties` does.
`data/grids/` is no longer read; the `.gitignore` entry is left for the checkouts that have one.

## Deviations kept

1. `proj_context_set_user_writable_directory` / `ProjBuilder::set_user_writable_directory`: a
   directory set on the context is read even when `PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY` is set.
   PROJ reads the variable only. The tests of `proj_context_test.cpp` ported here clear the
   variable first in C++; an in-process Rust test cannot clear it without racing the other
   tests, and an explicit directory is not the installation the variable keeps out.
2. Windows: the user directory comes from `%LOCALAPPDATA%` and not from `SHGetKnownFolderPath`,
   which needs the Shell API; the variable is the known folder unless a user redirected it.
3. `$HOME` is tested with `access(2)` on Unix and with the read-only flag elsewhere.
4. `~/name` and `PROJ_DATA` entries on Windows: no UTF-8 / ANSI recoding (`getProjDataEnvVar`, 1135-1148).
5. The lookup is not cached per name (`lookupedFiles`): a file added during the run is seen.

## Removed (they were deviations from PROJ, listed in `docs/PORTING-grid-search-scope.md`)

- A name with a directory part (`tests/x.tif`) also tried as `<dir>/x.tif` in the searched
  directories (deviation 3): PROJ opens the name as given.
- Explicit search paths added to the list, not replacing it (deviation 1 there).
- A bare name in the working directory tried first (deviation 2): it is now the last resort, and
  never reached when `PROJ_DATA` or search paths are set.
- `;` and `|` as separators of `PROJ_DATA` on every platform and empty entries dropped.
- The user directory read only when the grid cache was enabled: the cache setting
  (`cache_enabled`) is about the chunk database, not about the lookup.
- The file finder is now part of the chain for grids too, as in PROJ, and a finder result that
  does not exist is "not found" (`find_file` used to return it unchecked).

## Left to the caller (outside this lot's files)

- `src/bin/cct.rs` and `src/bin/cs2cs.rs` open `env!("CARGO_MANIFEST_DIR")/data/proj.db` (the
  operation-by-name path of `cct`, `Catalog::open` of `cs2cs`): a path of the build machine too.
- `src/bin/projsync.rs` `--system-directory` takes the first `PROJ_DATA` directory; PROJ takes
  `../share/proj`, else the compiled directory (`projsync.cpp`, 147-153).

## Proofs

- `src/datadir.rs` tests: the three platforms of the user directory on one machine (the
  environment and `access` are parameters), the `../share/proj` string for each shape of module
  path, the name kinds, the order with every place holding the file, the replacement by search
  paths, the working directory last, the `searchpath` string.
- `tests/data_dirs.rs`: real `cct` and `projinfo` run from another working directory, with a home
  of their own: a grid in the user directory, in `share/proj` next to a copy of the binary, the
  masks, the order (a file that is not a grid marks which place was chosen), `proj.ini`, and
  `projinfo --searchpaths` against the `projinfo` of PROJ 9.9.0 for eleven environments on Unix, six on Windows (HOME,
  `XDG_DATA_HOME` empty or not, unwritable home, `PROJ_DATA` lists, `PROJ_LIB`, skip,
  `PROJ_USER_WRITABLE_DIRECTORY`).
- `tests/data_dirs_api.rs`: `ProjBuilder::user_writable_directory` and `lib_info()` against the
  same `projinfo`, creation only on request.
- `tests/data_dirs_network.rs` (feature `network`): `cache.db` is `<user directory>/cache.db`
  and is written there.

## `access(2)` without `unsafe`

`home_is_writable` declared `access` in an `extern "C"` block (two `unsafe`). It now calls
`rustix::fs::access(path, Access::WRITE_OK)`: the same system call, behind a safe API, so the
kernel still decides with the real user and group, root, ACLs and read-only mounts, as PROJ's
`access(home, W_OK)`. `strace` of `projinfo --searchpaths` with `HOME=/tmp` shows
`access("/tmp", W_OK) = 0` for PROJ 9.9.0 and for this port. `std` has no such call: the mode bits
of `metadata` (`readonly()`) disagree for root and miss ACLs and read-only mounts, so it is not used.
`tests/data_dirs.rs` (`search_paths_are_those_of_projinfo_of_proj`) compares the user directory
with PROJ's `projinfo` for a home without write permission (mode 0555, writable for root), a home
that is a file, and a home on a read-only mount (when the machine has one). No deviation remains.

`rustix` 1.1.5 was already in `Cargo.lock` (through `tempfile`); it is MIT / Apache-2.0,
rust-version 1.63, maintained by the Bytecode Alliance. With `default-features = false` and
`fs`, `std`, it adds `linux-raw-sys` and `bitflags` (already in the tree) on Linux. Windows keeps
the read-only flag, as before.
