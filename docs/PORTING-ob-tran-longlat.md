# Porting sheet: `ob_tran` with `+o_proj=longlat`

PROJ 9.9.0 reference: `test/cli/test_cs2cs_various.yaml`, case "Check ob_tran
with o_proj=longlat (#1525)", and the case right after it (inverse handling).

## Behavior

`+proj=ob_tran +o_proj=longlat` returns geographic **degrees**. Before this lot
the target was treated like `latlon`, so the rotated angles came out in radians
and `cs2cs` printed `-2.3038346 0.8028515` where PROJ prints `-132 46`.

## What changed

- `src/projections/ob_tran.rs`: `longlat` and `lonlat` build a child
  `proj=unitconvert xy_in=rad xy_out=deg`, reusing the existing child path, so
  the inverse converts degrees back to radians before the rotation.
- `latlon` and `latlong` keep the radians result. The gie suite pins it
  (`builtins.gie`, `more_builtins.gie`), so it is untouched.

## Deliberate deviation and its ceiling

- Only `longlat` is pinned by PROJ 9.9.0 test data. `lonlat` takes the same path
  as an assumption, not from a PROJ test.
- `latlong` stays on the radians path. Whether PROJ treats it as `latlon` (radians)
  or as `longlat` (degrees) is not pinned by any test we have. Upgrade path: check
  PROJ `src/operation/ob_tran.cpp` for the `o_proj` string comparison and align it.

## Ledger

`test_cs2cs_various.yaml#89` (the case above) and `#90` (its inverse) now pass and
leave `docs/UNPORTED-CLI.md`.

## Commands run

- `devhub run --kind test -- cargo test --all-features --test cli_yaml` : passes.
- `devhub run --kind test --env UPDATE_DOCS=1 -- cargo test --test docs_in_sync`
  and the same for `progress_svg`: regenerated README, ROADMAP, UNIT-TRIAGE and
  `docs/progress.svg`.
- `cargo fmt --check` : clean.
- Full `cargo test --all-features --no-fail-fast` : the only failure is
  `agree_cs2cs::named_crs_pairs_match_cs2cs` (EPSG:4326 to EPSG:5070, about 0.16 m
  off). It also fails on the base commit without this change, so it is not caused
  by this lot. Open separately.

# ob_tran avec o_proj=moll (lot P2)

Référence PROJ 9.9.0 : `src/projections/ob_tran.cpp`. Cas de parité `proj-fwd-0102` :
`+proj=ob_tran +o_proj=moll +o_lat_p=45 +o_lon_p=0` (sans `+ellps`, donc GRS80).

## Cause

Le binaire `proj` ajoute `+break_cs2cs_recursion` à la définition, après les paramètres
de l'utilisateur. `ob_tran` reconstruit la définition de la cible (`moll`) à partir de
ces paramètres. Une création stricte de la cible refusait cette clé inconnue : sortie
en erreur, code 3, là où PROJ calcule. La clé ne sert qu'à la construction de PROJ
(`create.cpp`, anti-récursion cs2cs) ; elle n'a pas sa place dans la cible.

## Changement

- `src/projections/ob_tran.rs` : `child_definition` ignore `break_cs2cs_recursion`.
- Le chemin `longlat` / `latlon` ne passe pas par `child_definition` : il ne change pas.

## Mesure

- `cargo test --test unit_proj_ob_tran` : `1 passed; 0 failed`, six points avant et
six points inverse, valeurs de `proj -f %.12f` et `proj -I` du binaire de référence.
- Parité : `proj-fwd-0102` (sept entrées, dont le code de sortie) sort du registre.
  Statut 1 → 0, forme 6 → 0 ; hors seuil 152, aucun écart nouveau.
- `gie_run`, `gie_self_tests`, `cli_yaml`, `cli_binary` : verts.
