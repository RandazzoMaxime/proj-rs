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

## Codes and texts

A refusal carries PROJ's error code and the line `proj_create` logs:
`Error <code> (<text of the code>): <step>: <message>`, as `createFromPROJString` writes it
(`io.cpp:12879-12886`). The engine's `CreateError::Rejected` holds the code and the message of
the check that refused, in PROJ's words: the common parameters of `pj_init` (shape, units,
`lon_wrap`, `axis`, `pm`), the pipeline, the grid steps, `deformation`, `tinshift`, `defmodel`,
`axisswap`, `topocentric`, `helmert`, `molodensky` and the projections whose checks the `gie`
suite reaches. `CreateError::NoShape` is `pj_init_ctx: Must specify ellipsoid or sphere`, the
last line `pj_ell_set` leaves; `CreateError::Syntax` is a refusal of the PROJ string parser,
logged without a code. A text that is no WKT and names no object is `unrecognized format /
unknown name`, error 1025.

`+proj=utm` without `+zone` takes the zone of `lon_0`, as `PJ_PROJECTION(utm)` does; it was
refused.

`capi/tests/invalid_operations.c` compares the result, the code and the last error line.

## Deliberate deviations

Four of the 196 operations differ (`capi/tests/invalid_operations.deviations`).

Two degenerate operations are refused at creation by proj-rs, where PROJ makes the object and
fails at the first point:

- `+proj=lcc +lat_1=2D32 +lat_2=0 +a=6378137 +b=0.2`: an eccentricity of almost 1. proj-rs
  refuses `es > 0.999` and a cone whose constants are not finite (`src/lcc.rs`).
- `+proj=tpeqd +lat_1=90 +lat_2=90 ...`: both control points on the pole. PROJ's `z02 == 0`
  test misses it because `cos(π/2)` is not 0; proj-rs tests the poles (`src/projections/tpeqd.rs`).

Both fail either way: making the object, then failing every point, would need the transform of
these degenerate cones to fail exactly as PROJ's, for no use.

Two files that are not JSON (`+proj=defmodel +model=proj.ini`, `+proj=tinshift +file=proj.ini`)
are refused with PROJ's code and `invalid model:`, but the rest of PROJ's line is the message
of its JSON library (`[json.exception.parse_error.101] ...`), which proj-rs does not reproduce.

## The error code on the context

`proj_create` leaves `proj_context_errno` where PROJ leaves it. PROJ applies these rules, read
from `iso19111/c_api.cpp` (`proj_create`, `proj_log_error`, `pj_obj_create`) and `init.cpp`:

- An instantiation that succeeds clears the code: `pj_init` calls `proj_errno_reset`
  (`init.cpp:792`). That is any PROJ string, a CRS with `+type=crs` included, an operation
  from the database, and `proj_create_crs_to_crs`. A CRS from the database or a WKT is not
  instantiated and keeps the code.
- An instantiation that fails sets its code, over the previous one.
- After the instantiation, a refusal of `createFromUserInput` sets a code only when none is
  left (`proj_log_error`): PROJ_ERR_OTHER (4096) for a code missing from the database
  (`crs not found: EPSG:1671`), PROJ_ERR_INVALID_OP_WRONG_SYNTAX (1025) for a text that does
  not parse (WKT, PROJJSON, `name@epoch`, unknown name).
- `init=epsg:` and `init=IGNF:` are refused without the PROJ 4 rules (1025), before any
  instantiation.
- An operation from the database whose PROJ string `pj_init` refuses (a missing grid) is still
  returned, with the code of that refusal (1029). `proj_trans` on it sets
  PROJ_ERR_OTHER_NO_INVERSE_OP (4098) without a log line, and `proj_pj_info` gives its name, its
  accuracy and an empty definition, as for a CRS.

A CRS PROJ string is instantiated without `type=crs`, `nadgrids` and `geoidgrids`:
`createFromPROJString` adds `+disable_grid_presence_check`, so a missing grid of a CRS is not
an error. `+towgs84` takes up to seven terms and fills the others with 0, as `pj_datum_set`
does; the CRS model still refuses a count other than 3 or 7 (`Invalid number of elements in
TOWGS84Parameters`, 4096).

`capi/tests/errno_parity.c` compares 72 definitions on a fresh context and after a refused PROJ
string, and `capi/tests/crs_proj_strings.c` the PROJ strings libproj exports for every EPSG,
ESRI and IGNF CRS (9,692 strings with and without `+type=crs`).

One deviation is left: for an operation whose pipeline step fails (`EPSG::1312`, `EPSG::15958`),
PROJ logs `pipeline: Pipeline: Bad step definition: proj=hgridshift (File not found or
invalid)`; proj-rs sets the same code without that line (`capi/tests/errno_parity.deviations`).
