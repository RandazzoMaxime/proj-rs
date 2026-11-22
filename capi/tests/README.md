# C tests of proj-rs

The proof of the C library is the C tests of PROJ 9.9.0, compiled against `libproj` of proj-rs
and run. The tests are not copied and not changed: they are read from a PROJ 9.9.0 tree, and
`test_c_api.cpp` only loses the cases listed in `excluded.txt`.

```bash
tools/build-reference.sh                 # PROJ 9.9.0 in target/proj-reference (2 minutes)
capi/tests/run.sh                        # everything below, on the debug build
capi/tests/run.sh --release c_api        # one test, release build
GTEST_FILTER='CApi.proj_trans_bounds*' capi/tests/run.sh c_api
capi/tests/each.sh target/capi-tests/test_c_api   # one process per case, to see every failure
```

`run.sh` needs `g++` and `gcc`, the SQLite3 headers and the sources of googletest (Debian:
`apt install libgtest-dev`, which has 1.14; PROJ fetches 1.15.2, the tests do not depend on the
difference). `dist.sh` installs the library under the names of libproj: `libproj.so.25`,
`libproj.so`, `libproj.a` (`libproj.25.dylib` and `libproj.dylib` on macOS).

## What runs

| Step | What it proves |
|---|---|
| `symbols` | every function and variable of `proj.h` (292 functions and `pj_release`) is exported, and nothing else named `proj_*` or `pj_*` |
| `abi` | `abi_layout.c`, generated from `src/ffi.rs`, states in C the size, alignment and every field offset of the 27 structures of `proj.h` (210 assertions). It compiles against `capi/include/proj.h` and against the header of the PROJ tree. `src/ffi.rs` asserts the same numbers in Rust at compile time, so Rust and C agree on every structure |
| `lists` | `lists.c` prints the five `proj_list_*` tables; the output is compared with the one of the reference libproj (491 identical lines) |
| `errno_string` | `proj_errno_string_test.cpp`: 3 tests |
| `angular_io` | `proj_angular_io_test.cpp`: 5 tests |
| `context` | `proj_context_test.cpp`: 5 tests, 1 excluded |
| `include_h` | `include_proj_h_from_c.c`: `proj.h` and `proj_experimental.h` compile as C |
| `fork` | `test_fork.c`: contexts survive a `fork()` |
| `datadir` | `datadir.c`: `proj_context_get_user_writable_directory` and `proj_info().searchpath` are those of the reference libproj under ten environments (home, `XDG_DATA_HOME`, `PROJ_DATA`, `PROJ_LIB`, skip, `PROJ_USER_WRITABLE_DIRECTORY`, `proj_context_set_search_paths`), and the library installed in `<prefix>/lib` lists `<prefix>/share/proj` |
| `embedded_db` | `embedded_db.c`: with a search path or `PROJ_DATA` that holds no `proj.db`, `EPSG:32631` is still created from the built-in `proj.db`, where the reference libproj fails. A deliberate deviation, see `docs/PORTING-database-open.md` |
| `pj_info_order` | `pj_info_order.c`: `proj_pj_info().definition` of `proj_create_crs_to_crs` keeps the `axisswap` steps of a lat-first CRS until `proj_normalize_for_visualization`, as the reference libproj reports it, for 7 pairs |
| `normalize_operation` | `normalize_operation.c`: `proj_normalize_for_visualization` of the operations `proj_create_operations` lists (the path QGIS takes for every transform), of CRS (one written as a PROJ string), and of PROJ strings without `+type=crs` (operations), which give NULL without `proj_errno`: identical to the reference libproj for 6 pairs and 5 objects |
| `latin1_input` | `latin1_input.c`: a WKT handed over in Latin-1, as QGIS does (`QString::toLatin1`), with a degree sign that is not UTF-8: `proj_create_from_wkt`, `proj_create` and `proj_context_guess_wkt_dialect` give what the reference libproj gives for 2 WKT |
| `crs_to_crs_describe` | `crs_to_crs_describe.c`: the object of `proj_create_crs_to_crs` is the operation itself when one candidate is left, as in PROJ: type, name, accuracy, WKT, PROJJSON and `proj_pj_info`, raw and normalized, identical to the reference libproj for 7 pairs (6 single operations, 1 set of candidates) |
| `create_any_object` | `create_any_object.c`: `proj_create` of objects that are not CRS (operations, datums, ensembles, ellipsoids, prime meridians) given by URN, WKT or PROJJSON, as `createFromUserInput` reads them: type, name and identifier identical to the reference libproj for 13 texts |
| `export_objects` | `export_objects.c`: `proj_as_wkt` (WKT2:2019, WKT1_GDAL), `proj_as_projjson` and `proj_get_name` of ellipsoids, prime meridians (Greenwich and Paris in grads), datums of every kind, a datum ensemble and coordinate systems, identical to the reference libproj for 13 objects |
| `derived_temporal` | `derived_temporal.c` with `data/`: a derived geographic CRS (rotated pole, PROJJSON and WKT) and a temporal CRS (PROJJSON): `proj_get_type`, the coordinate system and its axes, the ellipsoid, the datum, the geodetic CRS, the base and the deriving conversion, identical to the reference libproj for 3 definitions |
| `last_used_operation` | `last_used_operation.c`: `proj_trans_get_last_used_operation` is NULL before the first point and gives the operation used after it, for 2 single-operation pairs and a PROJ string pipeline, as the reference libproj |
| `crs_to_crs_describe` | `crs_to_crs_describe.c`: the object of `proj_create_crs_to_crs` is the operation itself when one candidate is left, as in PROJ: type, name, accuracy, WKT, PROJJSON and `proj_pj_info`, raw and normalized, identical to the reference libproj for 7 pairs (6 single operations, 1 set of candidates) |
| `deriving_conversion` | `deriving_conversion.c`: the conversion `proj_crs_get_coordoperation` returns for 3 projected CRS, a geographic CRS and a derived projected CRS (`data/derived_projected.wkt`): its source, its target, whether it is instantiable and its PROJ string, identical to the reference libproj |
| `single_crs_cs` | `single_crs_cs.c`: `proj_crs_get_coordinate_system` of geographic, projected, vertical, compound and bound CRS, with the error it logs, identical to the reference libproj (a bound or compound CRS has none) |
| `c_api` | `test_c_api.cpp`: 152 `TEST`, 32 excluded, 120 pass |

## Excluded: `excluded.txt`

A case is excluded when it cannot pass whatever the library does:

- 32 cases of `test_c_api.cpp` call the C++ API of PROJ (`osgeo::proj::...`: `WKTFormatter`,
  `PropertyMap`, `GeographicCRS::EPSG_4326`, `Transformation::create`...) to build their input
  or to check a result. The C++ library does not exist in this port. `find_cpp_cases.py` lists
  them from the linker log, and `filter_tests.py` cuts them out of the source; the list in
  `excluded.txt` names the first C++ symbol of each.
- 1 case of `proj_context_test.cpp` (`proj_context_set_ca_bundle_path`) reads
  `ctx->ca_bundle_path`, a member of the C++ structure of `proj_internal.h`.

## Pending: `pending.txt`

Empty. A case that does not pass yet goes there, with its reason on the same line; it is skipped at
run time, and `PENDING=0 capi/tests/run.sh` runs it anyway.

## Files

| File | Role |
|---|---|
| `run.sh`, `dist.sh`, `each.sh` | build, install, run |
| `excluded.txt`, `pending.txt` | `Suite.name  # reason` |
| `filter_tests.py`, `find_cpp_cases.py` | cut the excluded cases, find them |
| `check_symbols.py` | the `symbols` step |
| `abi_layout.c` | the `abi` step, generated by `capi/tools/gen_abi_check.py` |
