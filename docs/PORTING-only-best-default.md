# Port — `PROJ_ONLY_BEST_DEFAULT` and `only_best_default` in proj.ini

Source: PROJ 9.9.0.

- `src/filemanager.cpp` `pj_load_ini()`, lines 1988-2107. The environment
  variable is read at 2023-2031, and the `proj.ini` key at 2101-2107.
- `src/crs_to_crs.cpp` `proj_create_crs_to_crs`, lines 494-504. The defaults
  are copied from the context when no `ONLY_BEST` option is given. Lines
  521-531 are the `ONLY_BEST=` option, which sets the same two flags.
- `src/ctx.cpp` lines 159-162. The context copy of the two flags.

The port keeps the setting on `GridContext` (`src/network.rs`), where
`pj_load_ini` already reads `proj.ini` for the network keys. PROJ keeps the
same two flags on `PJ_CONTEXT`.

## What PROJ does

| source | value | PROJ state | port state |
|---|---|---|---|
| `PROJ_ONLY_BEST_DEFAULT` unset | | no setting | `None` |
| `PROJ_ONLY_BEST_DEFAULT` set to `""` | | no setting, and `proj.ini` is ignored | `None` |
| `PROJ_ONLY_BEST_DEFAULT` | `YES`, `ON`, `TRUE`, any case | `ONLY_BEST=YES` | `Some(true)`, `OnlyBest::On` |
| `PROJ_ONLY_BEST_DEFAULT` | any other non-empty value (`NO`, `0`, `maybe`, ` yes`) | `ONLY_BEST=NO` | `Some(false)`, `OnlyBest::Discard` |
| `proj.ini` `only_best_default=` | key exact after trimming; value compared as above | `ONLY_BEST=YES` for `ON`, `YES`, `TRUE`, else `ONLY_BEST=NO`, even empty | `Some(truthy)` |
| `proj.ini` key ignored when | `PROJ_ONLY_BEST_DEFAULT` is set, even empty | | skipped via `IniSkip::only_best` |
| command line | `--only-best`, `--only-best=yes`, `--only-best=no` | wins over both | `opts.only_best.or_else(...)` |

Precedence is command line, then the environment variable, then `proj.ini`.

## Deviations

- The library API keeps its own default. `Proj::new_known_crs`, the georust
  facade and `Operation` do not read the default. PROJ applies it to every
  `proj_create_crs_to_crs` call. This lot applies it to `cs2cs` only, and the
  targets are both `cs2cs` cases. Open item.
- `cs2cs` resolves the default once per process in `main`. PROJ resolves it
  per context. For a single `cs2cs` run the two agree.
- `proj.ini` is read as UTF-8 text by `read_to_string`. A file that is not
  valid UTF-8 is treated as absent. PROJ parses the bytes. This was already
  so for the network keys and is not changed here.
- The public accessor `proj_rs::iso19111::only_best_default()` returns
  `OnlyBest`, not a `bool`, so no public signature takes a `bool`.

## Oracle table

Each scenario runs `cs2cs` from PROJ 9.9.0 and from proj-rs. Each runs in its
own directory that holds `proj.ini` when the scenario has one. The inputs are
`49 2 0` for `EPSG:4326+3855` to `EPSG:4979`, and `39 -3 0` for
`EPSG:4326+5773` to `EPSG:4326+5782`. The table keeps the rows that decide the
behaviour. The full 31 scenarios per pair were run with a scratch script that
is not part of the repository, and the summary below covers all of them.

| scenario | setting | PROJ 3855 | ours 3855 | PROJ 5773 exit | ours 5773 exit |
|---|---|---|---|---|---|
| none | | `49dN 2dE 0.000` | same | 0 | 0 |
| env `YES` | `PROJ_ONLY_BEST_DEFAULT=YES` | `* * inf` + grid message | same | 3 | 3 |
| env `NO` | `PROJ_ONLY_BEST_DEFAULT=NO` | ballpark | same | 0 | 0 |
| env empty | `PROJ_ONLY_BEST_DEFAULT=` | ballpark | same | 0 | 0 |
| env `maybe` | non-empty, not YES | ballpark | same | 0 | 0 |
| env `on` / `TRUE` | | error | same | 3 | 3 |
| env ` yes` (leading space) | | ballpark | same | 0 | 0 |
| ini `only_best_default=on` | | error | same | 3 | 3 |
| ini `only_best_default=YES` | | error | same | 3 | 3 |
| ini `only_best_default = true ` | spaces around `=` | error | same | 3 | 3 |
| ini `off` / empty / `bogus` | | ballpark | same | 0 | 0 |
| ini `on` with env `NO` | env wins | ballpark | same | 0 | 0 |
| ini `on` with env empty | env hides ini | ballpark | same | 0 | 0 |
| ini `off` with env `YES` | env wins | error | same | 3 | 3 |
| ini `on` with `--only-best=no` | option wins | ballpark | same | 0 | 0 |
| env `YES` with `--only-best=no` | option wins | ballpark | same | 0 | 0 |
| env `NO` with `--only-best` | option wins | error | same | 3 | 3 |
| ini `on` then `off` (duplicate) | last wins | ballpark | same | 0 | 0 |
| CRLF line endings | | error | same | 3 | 3 |
| `#only_best_default=on` | comment | ballpark | same | 0 | 0 |
| `only_best=on` | wrong key | ballpark | same | 0 | 0 |
| `ONLY_BEST_DEFAULT=on` | wrong case | ballpark | same | 0 | 0 |
| `only_best_default="on"` | quoted value | ballpark | same | 0 | 0 |

Result: the 3855 pair matches PROJ in all 31 scenarios, for exit code, stdout
and stderr. For the 5773 pair, exit codes and stderr match in all 31. Stdout
differs in 20 non-error rows. Those rows print the `EPSG:4326+5782` output as
`39dE<TAB>3dS 0.000` where PROJ prints `39.00<TAB>-3.00 0.00`. That format
difference is in the output path, not in the `ONLY_BEST` decision. The
explicit `--only-best=no` rows show it too. It is not changed by this lot. The
yaml harness compares numbers and not their format, so it does not flag it.

## Cases closed

- `tests/cli_yaml/test_cs2cs_various.yaml` #120 (`PROJ_ONLY_BEST_DEFAULT=YES`).
- `tests/cli_yaml/test_cs2cs_various.yaml` #121 (`only_best_default=on` in
  `proj.ini`).

Case #126 sets `PROJ_ONLY_BEST_DEFAULT: ""`, so it keeps the default. It stays
open for a different reason: its expected stderr is the `PROJ_DEBUG=2`
warning text. That is not changed by this lot.

## Unit tests

`src/network.rs`, module `tests`:

- `only_best_env_follows_pj_load_ini`
- `only_best_ini_key_follows_pj_load_ini`
- `only_best_env_hides_the_ini_key_even_when_empty`

Each assertion cites the PROJ scenario it copies.

## Commands

```bash
# PROJ 9.9.0 oracle
PROJ_DATA=<oracle build>/data:$PWD/data/tests PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES \
  <oracle build>/bin/cs2cs EPSG:4326+3855 EPSG:4979     # stdin: 49 2 0

# proj-rs
PROJ_DATA=$PWD/data:$PWD/data/tests PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES \
  target/debug/cs2cs EPSG:4326+3855 EPSG:4979           # stdin: 49 2 0

cargo test --lib network::
cargo test --test cli_yaml
```
