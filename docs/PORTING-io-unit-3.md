# Port — CRS géocentriques WKT (`wkt_parse.wkt*_geocentric*`) et test NKG

Ce lot a deux parties, dans deux commits.

1. **`src/projections/nkg.rs`** : le test `gie_urns_build_when_the_grid_is_present`,
   `#[ignore]` depuis son ajout, est remplacé par un test hors réseau.
2. **`tests/test_io.rs`** : 5 `TEST()` de `test_io.cpp` (groupe géocentrique), portés.

## 1. NKG : test hors réseau et erreur `grid`

### Ce que dit la base

`data/proj.db`, tables `helmert_transformation`, `other_transformation`,
`concatenated_operation` et `concatenated_operation_step`, auth `NKG` :

- **14 URN sans grille** : `P1_2008_{DK,EE,FI,LT,LV,NO,SE}` et
  `PAR_2020_{DK,EE,FI,LT,LV,NO,SE}`. Ce sont des lignes `helmert_transformation`
  (méthode 1033, position vector), sans étape `other`.
- **26 URN avec grille** : toutes les chaînes `concatenated_operation`. Chacune
  a au moins une étape `other_transformation` dont la méthode est
  `PROJString` avec `+grids=` ou `xyzgridshift`. Les 19 lignes `other` sont
  toutes de ce type, sans `grid_name`. Aucune URN de la liste ne peut donc se
  construire hors réseau.

### Cause de l'échec du test précédent

Le test attendait que les 26 URN se construisent avec les grilles CDN. Sans
grille, la première URN de contrôle renvoyait `IllegalValue("file")`, et non
`Unsupported("grid")`. `read_other` rendait les lignes `PROJString` avant de
vérifier `needs_grid`. L'étape `+proj=deformation +grids=…` partait donc dans
`Operation::create`, qui échoue sur le fichier absent.

`docs/PORTING-nkg.md` annonce pourtant `Unsupported("grid")` pour ces URN.

### Correction

- Une étape `PROJString` qui nomme une grille (`+grids=`, `+xy_grids=`,
  `+z_grids=`) est construite si chaque grille requise (sans `@`) est présente
  ou téléchargeable (`gridpath::available`). Sinon l'URN renvoie
  `Unsupported("grid")`. Le contrôle est fait par `grids_available`, dans
  `read_other`.
- Ce contrôle était absent de l'état précédent. Le premier correctif refusait
  toutes les URN à grille, même avec la grille présente. La CI `gie_run` avec
  `PROJ_NETWORK=ON` a alors échoué : `nkg.gie` tombait de 33 à 0 cas réussis,
  et le total de 6 856 à 6 823. Le correctif actuel rétablit ce compte.
- Le test `urns_without_a_grid_build_and_grid_urns_are_refused` remplace le
  test ignoré. Il construit les 14 URN sans grille et vérifie que les 26 URN
  avec grille renvoient `Unsupported("grid")`. Il tourne hors réseau, comme
  l'étape `cargo test --all-features` de la CI.
- `tests/gie/nkg.gie` n'est pas modifié.

## 2. Géocentrique WKT

Source : PROJ 9.9.0, `test/unit/test_io.cpp`, groupe `wkt_parse.*geocentric*`,
et le helper `checkGeocentric`. Côté crate : `WktObject::geodetic_crs`, variante
`GeodeticCRS::Geocentric`, et l'import des axes dans `src/iso19111/wkt_import.rs`
(nom `Geocentric X` et direction `geocentricX` dérivés de `(X)` ou de
`OTHER`/`NORTH`).

### Correspondance

| PROJ 9.9.0 `wkt_parse.*` | PROJ-RS `tests/test_io.rs` |
|---|---|
| `wkt2_geocentric` | `wkt_parse_wkt2_geocentric` |
| `wkt2_simplified_geocentric` | `wkt_parse_wkt2_simplified_geocentric` |
| `wkt1_geocentric` | `wkt_parse_wkt1_geocentric` |
| `wkt1_geocentric_with_z_OTHER` | `wkt_parse_wkt1_geocentric_with_z_other` |
| `wkt1_geocentric_missing_unit_and_axis` | `wkt_parse_wkt1_geocentric_missing_unit_and_axis` |

`checkGeocentric` devient `check_geocentric`. Les assertions sont celles de
PROJ : nom du CRS, trois axes (nom, abréviation, direction), datum
`World Geodetic System 1984`, ellipsoïde WGS 84 (demi-grand axe, aplatissement,
nom) et unité du méridien.

## API

Aucune API publique ajoutée. Le lot ne modifie pas `src/iso19111` : les
fonctions `geocentric_crs` et `check_geocentric` sont des helpers de test.

## Écarts volontaires

Aucun écart de comportement dans le lot WKT. Le test `wkt1_geocentric_missing_unit_and_axis`
passe en mode `Lenient`, comme `setStrict(false)` de PROJ.

## Comportements conservés

- Le WKT de `wkt1_geocentric_missing_unit_and_axis` contient quatre `]` après
  `SPHEROID`, alors que trois suffisent. Le source PROJ l'a aussi. En mode
  `Lenient`, le parseur l'accepte. En mode strict, il est refusé, comme dans PROJ.
- Sans `UNIT`, en mode `Lenient`, les axes reçoivent l'unité `metre` (test
  `wkt_parse_wkt1_geocentric_missing_unit_and_axis`).

## Non portés dans ce lot

Restent dans `docs/UNPORTED-UNIT.md` (section `test_io.cpp`, 365 entrées) :

- `wkt_parse.wkt2_geocentric_DEFININGTRANSFORMATION` : le test compare une chaîne
  WKT2 exportée à une valeur attendue. Ce lot ne l'a pas examiné.
- `wkt_parse.wkt1_geocentric_with_PROJ4_extension` : le test exige l'import de
  `EXTENSION["PROJ4", ...]`, `exportToWKT(WKT1_GDAL)` et
  `exportToPROJString(PROJ_4)` avec `+type=crs`. Non porté dans ce lot.
- Les autres groupes `wkt_parse.*`, `json_import.*` et `io.projstringformatter*`.
  `ProjStringFormatter` n'existe pas dans la crate ; `json_import.*` exige une
  sérialisation JSON des objets autres que les CRS (ellipsoïde, axe, méridien),
  que la crate n'expose pas.

## Vérification

Commandes lancées dans le worktree `lot-unit-io-3`. Le harnais `cli_yaml`
réécrit `docs/UNPORTED-CLI.md` à chaque suite complète. Ce fichier est restauré
avant commit et n'est pas inclus dans le lot.

```
cargo fmt --check                                                  # ok
cargo clippy --all-targets --all-features -- -D warnings           # ok, aucune sortie
cargo test --lib nkg                                               # 8 passed, 0 ignored
PATH=<proj-ref>/bin:$PATH PROJ_CS2CS_REQUIRED=1 \
  PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES \
  cargo test --all-features --no-fail-fast                         # EXIT 0, 0 failed, 0 ignored
#   tests/test_io.rs : 89 passed (84 avant)
#   tests/gie_run.rs : 1 passed (hors réseau)
PROJ_NETWORK=ON PROJ_USER_WRITABLE_DIRECTORY=<cache> \
  cargo test --all-features --test gie_run -- --nocapture          # nkg.gie passed=33; total passed=6856
```

Recompte du ledger `test_io.cpp` : 370 → 365. Total ouvert : 990 → 985. Portés
`test_io` : 84 → 89 (README). Total « ported » : 362 → 367.
