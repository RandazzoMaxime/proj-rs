# Port — bound CRS with a projected base

Case: a bound CRS whose base is a projected CRS (`+proj=utm +zone=55 +south +ellps=GRS80 +towgs84=0,0,0,0,0,0,0 +units=m +no_defs +nadgrids=@GDA94_GDA2020_conformal.gsb +type=crs`) to a geographic CRS. Reported as "the grid is replaced by a ballpark, PROJ keeps the `hgridshift`".

**Result: the report does not hold against PROJ 9.9.0. No source change.** For `+proj=longlat +ellps=GRS80 +type=crs`, PROJ 9.9.0 also returns `Inverse of UTM zone 55S + Ballpark geographic offset from unknown to unknown`, with no grid. The crate already returns that operation. The grid is kept by PROJ for the other geographic targets (WGS 84, GDA94, GDA2020), and the crate matches PROJ there too (see the table).

## Oracle

Expected values come from PROJ 9.9.0 itself, not from this crate. `projinfo` 9.9.0 was taken from the Homebrew bottle `proj` 9.9.0 (`brew fetch proj`, then the bottle tarball was extracted into a temporary directory). The bottle's `libtiff` install name was patched in that temporary copy, and nothing was installed system-wide. Command, per target `T`:

```sh
PROJ_DATA=<bottle>/share/proj DYLD_LIBRARY_PATH=<bottle>/lib \
  projinfo -s "+proj=utm +zone=55 +south +ellps=GRS80 +towgs84=0,0,0,0,0,0,0 +units=m +no_defs +nadgrids=@GDA94_GDA2020_conformal.gsb +type=crs" -t "$T" -o PROJ
```

| Target `T` | PROJ 9.9.0 operations | Crate |
|---|---|---|
| `+proj=longlat +ellps=GRS80 +type=crs` | 1: `Inverse of UTM zone 55S + Ballpark geographic offset from unknown to unknown` | same |
| `+proj=longlat +datum=WGS84 +type=crs` | 1: `Inverse of UTM zone 55S + unknown to WGS84 + axis order change (2D)`, `hgridshift` kept | same |
| `EPSG:4283` (GDA94) | 3: `(1)`, `(2)` with `hgridshift`, then `Ballpark geographic offset from WGS 84 to GDA94` | same, same names, same PROJ strings |
| `EPSG:7844` (GDA2020) | 3: `(2)`, `(3)` with `hgridshift`, then `Ballpark geographic offset from WGS 84 to GDA2020` | same, same names, same PROJ strings |

## Why the target `+ellps=GRS80` gets a ballpark (C++ read, 9.9.0)

- `createOperationsBoundToGeog` (`src/iso19111/operation/coordinateoperationfactory.cpp`): the base geographic CRS of the UTM is `Unknown based on GRS 1980 ellipsoid using nadgrids=@GDA94_GDA2020_conformal.gsb`. The name suffix comes from `buildDatum` in `src/iso19111/io.cpp` (`datumNameSuffix`), and is added for `+ellps` datums too.
- The target datum is `Unknown based on GRS 1980 ellipsoid`. The two names differ. `isSameGeodeticDatum` and the first datum test in `createOperationsBoundToGeog` both fail, since `GeodeticReferenceFrame::hasEquivalentNameToUsingAlias` (`src/iso19111/datum.cpp`) returns false for names starting with `Unknown based on`.
- The hub is `EPSG:4326` (`BoundCRS::createFromNadgrids` in `src/iso19111/crs.cpp`), so the hub branch needs `hub == target`, which is false. The hub-to-target leg is then `Ballpark geographic offset from WGS 84 to unknown`, and the exclusion `opsLast.size() > 1 || !opLast->hasBallparkTransformation()` drops it.
- The last step is `createOperations(baseCRS, targetCRS)`, which is `createOperationsDerivedTo` (inverse UTM, then the geographic ballpark). That gives the reported string.
- The PROJ-string fallback (`createOperationsFromProj4Ext`) runs only when a `EXTENSION_PROJ4` is set (`hasUnusedParameters` in `io.cpp`). This string has no unused parameter, so the fallback does not apply.

Rust read: `src/iso19111/bound_ops.rs` (`bound_to_geog`, `with_endpoints`), `src/iso19111/bound_import.rs` (`bind_nadgrids`), `src/iso19111/proj_crs.rs` (`named_datum`, `grid_datum`), `src/iso19111/equiv.rs` (`same_geodetic_datum`). They follow the same steps and give the same operations.

## Where the grid is kept in PROJ

The `hgridshift`-kept case in `tests/unit_opfac_core_c.rs` (`createOperation_on_crs_with_bound_crs_and_wktext`) is a different path. Its source has `+ignored1 +ignored2=val +wktext`, so `EXTENSION_PROJ4` is set and `createOperationsFromProj4Ext` writes a PROJ pipeline. It does not go through `bound_to_geog`. Its target is also projected, not geographic.

`docs/PORTING-opfac-core-c.md`, section "Bound CRS with a projected base", says that `bound_to_geog` must keep the grid for the geographic case, and that the `wktext` case needs that change. Both statements are wrong, per the reading above and the oracle. That section is not edited here (owned elsewhere), and it should be corrected by its owner.

## Tests

`tests/unit_bound_projected_base.rs`, all passing:

- `operation_bound_utm_nadgrids_to_geog_ellps_only_is_ballpark_as_proj`
- `operation_bound_utm_nadgrids_to_geog_wgs84_keeps_hgridshift_as_proj`
- `operation_bound_utm_nadgrids_to_gda94_keeps_hgridshift_as_proj`
- `operation_bound_utm_nadgrids_to_gda2020_keeps_hgridshift_as_proj`

Not covered: a projected target from a bound projected source (open in `docs/PORTING-opfac-core-c.md`), and the Clarke 1866 to NAD83 special case (not ported, see `bound_ops.rs`).
