# Audit: `test_defmodel.cpp` (13 TEST)

Source: PROJ 9.9.0, `src/test/unit/test_defmodel.cpp` (1520 lines), read line by
line against the Rust tests below. Every `ASSERT_*`/`EXPECT_*` of the C++ has a
Rust `assert*!` at the same place in the same order.

Status: 13 of 13 closed. Nothing is left open.

| C++ `TEST(defmodel, ...)` | Rust test | Scope |
|---|---|---|
| `basic` | `tests/unit_defmodel_parse.rs::defmodel_basic` | Missing or ill-typed required keys (one removal per key of the minimal model), `definition_crs` mismatch, `extent` and `time_extent` shape |
| `full` | `tests/unit_defmodel_parse.rs::defmodel_full` | All master-file accessors, the six time-function variants and their fields |
| `error_cases` | `tests/unit_defmodel_parse.rs::defmodel_error_cases` | Unsupported keywords, and the two combinations rejected by the parser (degree with geocentric; degree with geocentric bilinear) and the two accepted |
| `ISO8601ToDecimalYear` | `tests/unit_defmodel_parse.rs::defmodel_iso8601_to_decimal_year` | Exact values (leap years, year end) and the invalid dates |
| `evaluate_constant` | `tests/unit_defmodel_time.rs::defmodel_evaluate_constant` | |
| `evaluate_velocity` | `tests/unit_defmodel_time.rs::defmodel_evaluate_velocity` | |
| `evaluate_step` | `tests/unit_defmodel_time.rs::defmodel_evaluate_step` | |
| `evaluate_reverse_step` | `tests/unit_defmodel_time.rs::defmodel_evaluate_reverse_step` | |
| `evaluate_piecewise` | `tests/unit_defmodel_time.rs::defmodel_evaluate_piecewise` | Interior values, `before_first` and `after_last` variants, no epoch, one epoch, two identical epochs |
| `evaluate_exponential` | `tests/unit_defmodel_time.rs::defmodel_evaluate_exponential` | Exact equality as in C++, including the end epoch clamp |
| `evaluator_horizontal_unit_degree` | `tests/unit_defmodel_evaluator.rs::defmodel_evaluator_horizontal_unit_degree` | Exact node, interior point, out of the model (lon, lat, time), out of the component (skipped), time factor zero, vertical and 3d |
| `evaluator_horizontal_unit_metre` | `tests/unit_defmodel_evaluator.rs::defmodel_evaluator_horizontal_unit_metre` | The 14 points (bilinear and geocentric bilinear, horizontal and 3d), the repeated-evaluation checks, `evaluate_inverse`, and the geocentric horizontal offset method |
| `evaluator_projected_crs` | `tests/unit_defmodel_evaluator.rs::defmodel_evaluator_projected_crs` | Projected evaluation, and the three `EvaluatorException` cases (degree unit, geocentric method, geocentric bilinear) |

## Test inputs

The C++ grids are synthetic, injected through `GridPrototype` and
`GridSetPrototype`. The Rust tests inject the same grids through
`proj_rs::defmodel::{Grid, GridSet, GridProvider}`. No grid file is read by
these tests, so no file from `data/tests` is needed.

The `float` literals of the metre grid (`0.4f`, `0.84f`, ...) are stored as
`f64::from(0.4_f32)`, which is the value the C++ mock stores.

The JSON documents are built by `tests/common/defmodel_json.rs`, which does the
edits of `nlohmann::json` (`j[...] = `, `erase`, `clear`, `push_back`) by path.
No JSON crate is in the offline registry, so the helper is local.

## Other tests of the same model

`tests/proj_unit.rs` (`defmodel_basic_and_rejected_json`,
`defmodel_error_cases`, `defmodel_iso8601_and_time_functions`) and
`tests/transform_export.rs::defmodel_1079_9084` predate this lot. They still
pass and still cover `+proj=defmodel` through `Operation`. They are not
removed.

## Not ported

Nothing from `test_defmodel.cpp` is left out. The C++ behaviours that have no
counterpart in the Rust API are listed in `docs/PORTING-defmodel.md`, under
deliberate differences.
