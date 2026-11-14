# Porting sheet: deformation model (`defmodel`)

PROJ 9.9.0 sources: `src/transformations/defmodel.hpp`,
`defmodel_impl.hpp`, `defmodel.cpp`, and `test/unit/test_defmodel.cpp`. The
audit of the 13 `TEST()`s is in `docs/AUDIT-defmodel.md`.

## Map

The public API is `proj_rs::defmodel`. It is a thin module, like
`proj_rs::tinshift`, over `src/projections/models/defmodel.rs`. The `+proj=defmodel`
operation runs on the same types, so the public API is the engine's code.

| PROJ | Rust |
|---|---|
| `MasterFile::parse(text)` | `MasterFile::parse(&str) -> Result<MasterFile, DefmodelError>` |
| `MasterFile` accessors (`fileType()`, `extent()`, `timeExtent()`, `components()`, ...) | Same names in snake case, `&str` for strings (`""` when absent, as in PROJ) |
| `Component::timeFunction()->evaluateAt(dt)` | `TimeFunction::evaluate_at(decimal_year)`; the variants are `TimeFunction::{Constant, Velocity, Step, ReverseStep, Piecewise, Exponential}` |
| `ISO8601ToDecimalYear(dt)` | `decimal_year_from_iso8601(&str) -> Result<f64, DefmodelError>` |
| `ParsingException` | `DefmodelError::Parsing(String)` |
| `EvaluatorException` | `DefmodelError::Evaluator(String)` |
| `Evaluator(model, iface, a, b)` | `Evaluator::new(MasterFile, &provider, a, b) -> Result<Evaluator, DefmodelError>` |
| `Evaluator::forward(iface, x, y, z, t, ...)` | `Evaluator::evaluate(x, y, z, t) -> Option<[f64; 3]>` (`None` is `false`) |
| `Evaluator::inverse(iface, ...)` | `Evaluator::evaluate_inverse(x, y, z, t) -> Option<[f64; 3]>` |
| `EvaluatorIfacePrototype::open(filename)` | `GridProvider::open(&str) -> Option<Self::Set>` |
| `EvaluatorIfacePrototype::isGeographicCRS(def)` | `GridProvider::is_geographic_crs(&str) -> bool` |
| `GridSetPrototype::gridAt(x, y)` | `GridSet::grid_at(x, y) -> Option<&Self::Node>` |
| `GridPrototype` (`minx`, `resx`, `width`, `getLongLatOffset`, `getZOffset`, ...) | `Grid` trait: `min_x`, `res_x`, `width`, ..., and the offset methods, which default to `None` |

The C++ `iface` argument of `forward`/`inverse` is not in the Rust signatures.
The grid sets are opened when the `Evaluator` is built, so the evaluation
functions only read them.

`Grid` is exported from `proj_rs::defmodel` as `Grid`, from
`proj_rs::projections` as `DefmodelGrid`, because `proj_rs::grids::Grid` already
has that name.

## Deliberate differences

- **Grid opening at construction.** PROJ opens a component's grid set on the
  first point that needs it, and a failed open makes `forward` return `false`.
  Here `Evaluator::new` opens every grid of a non-`none` component, and a failure
  is `DefmodelError::Evaluator`. The engine already failed at build time, so
  `+proj=defmodel` behaves as before.
- **No cache.** `ComponentEx` caches the last time-function value and `GridEx`
  caches the geocentric cell. The cached values are the values of the uncached
  formulas, so the results are identical. The evaluation is `&self`, which
  keeps the `Step` of the engine shareable. The repeated-evaluation checks of
  `evaluator_horizontal_unit_metre` still assert identical results.
- **Geocentric conversion.** `geodetic_to_geocentric` and
  `geocentric_to_geodetic` are written here from the ellipsoid, not through
  PROJ's `cart` operation as the C++ test mock does. The latitude is a fixed
  point iteration, stopped at a step of `1e-15`. The height is not returned, as
  PROJ ignores it at this point. The test tolerances (1e-10 and 1e-9) hold.
- **ISO 8601 input.** The parser accepts the fixed form `YYYY-MM-DDTHH:MM:SSZ`
  only. PROJ's `sscanf` also accepts trailing characters.
- **Empty epochs.** An empty epoch is decimal year 0, as in PROJ. An empty
  `end_epoch` is `None`, as the C++ `endEpoch.toString().empty()` check says.
- **Inverse.** Ten iterations at most. When the iteration does not converge
  `evaluate_inverse` returns `None`, where PROJ returns `false`.
- **JSON depth.** PROJ rejects documents nested 128 levels deep. The parser of
  this crate does not check the depth.
- **Engine-only checks.** `+proj=defmodel` keeps three checks that the PROJ
  parser does not make: `file_type` must be `deformation_model_master_file`,
  `format_version` must be `1.0`, and each component with a displacement must
  use a `GeoTIFF` spatial model. The tests use `file_type` `GeoTIFF`, which the
  public parser accepts, as the C++ test does.
- **Logging.** `DEBUG_DEFMODEL` logging is not ported.

## Tests

- `tests/unit_defmodel_parse.rs`: `basic`, `full`, `error_cases`,
  `ISO8601ToDecimalYear`.
- `tests/unit_defmodel_time.rs`: the six `evaluate_*` tests.
- `tests/unit_defmodel_evaluator.rs`: the three `evaluator_*` tests, with
  synthetic grids through `GridProvider`.
- `tests/common/defmodel_json.rs`: the JSON tree and the minimal and full
  model builders.

## Commands

```bash
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 \
  cargo test --all-features --test unit_defmodel_parse --test unit_defmodel_time \
  --test unit_defmodel_evaluator --test proj_unit --test transform_export
cargo +1.100.0 clippy --all-targets --all-features -- -D warnings
```
