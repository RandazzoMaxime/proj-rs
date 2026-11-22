# Refusing a PROJ string at creation

PROJ instantiates a PROJ string before it makes the object `proj_create` returns
(`createFromPROJString`, `io.cpp:12799`). An operation that `pj_init` or the pipeline refuses
gives no object: `proj_create` returns NULL and the error stays on the context.

proj-rs made a PROJ-based operation from the text, and only the transform built from it
failed. `proj_create` now builds the operation in the caller's context first, and returns NULL
when that fails (`capi/src/pj.rs`, `create`). `proj_based_operation_of_text` refuses the same
strings, except for a missing grid: it has no context to search for one.

A pipeline whose step runs inverted without an inverse is refused, as `pipeline.cpp:533-555`
does ("Inverse operation for %s is not available").

`capi/tests/invalid_operations.c` runs `proj_create` on the 196 operations of PROJ's `gie`
suite that expect a failure, against the reference libproj.

## Deliberate deviations

Two degenerate operations are refused at creation by proj-rs, where PROJ makes the object and
fails at the first point (`capi/tests/invalid_operations.deviations`):

- `+proj=lcc +lat_1=2D32 +lat_2=0 +a=6378137 +b=0.2`: an eccentricity of almost 1. proj-rs
  refuses `es > 0.999` and a cone whose constants are not finite (`src/lcc.rs`).
- `+proj=tpeqd +lat_1=90 +lat_2=90 ...`: both control points on the pole. PROJ's `z02 == 0`
  test misses it because `cos(π/2)` is not 0; proj-rs tests the poles (`src/projections/tpeqd.rs`).

Both fail either way: making the object, then failing every point, would need the transform of
these degenerate cones to fail exactly as PROJ's, for no use.

## Not in this lot

The error codes and texts are not all PROJ's yet: the refusal carries the code of the error
family (`proj_string_refusal`) and the engine's own message, except for `k`/`k_0` and `lat_0`.
