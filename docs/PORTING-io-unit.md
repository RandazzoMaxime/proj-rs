# Port — chaînes PROJ `+proj=longlat` (`io.projparse_longlat*`)

Source : PROJ 9.9.0, `test/unit/test_io.cpp` (groupe `io.projparse_*`, de
`projparse_longlat` à `projparse_longlat_pm_overriding_datum`) et
`src/iso19111/io.cpp` : `PROJStringParser::Private::buildDatum`,
`buildPrimeMeridian`, `fixupPrimeMeridan`, `overridePmIfNeeded`, la table
`datumDescs`, et `Ellipsoid::guessBodyName` (`datum.cpp`).

Ce lot porte 20 `TEST()` de `test_io.cpp`. Il ne touche ni aux `BoundCRS`
(`+towgs84=`, `+nadgrids=`), ni aux `CompoundCRS` (`+geoidgrids=`, `+vunits=`),
ni aux opérations (`createOperation` avec `axisswap` ou `unitconvert`). Ces
cas restent dans `docs/UNPORTED-UNIT.md`.

## Correspondance

| PROJ 9.9.0 `io.projparse_*` | PROJ-RS `tests/test_io.rs` |
|---|---|
| `projparse_longlat` | `io_projparse_longlat` |
| `projparse_longlat_datum_NAD83` | `io_projparse_longlat_datum_nad83` |
| `projparse_longlat_datum_NAD27` | `io_projparse_longlat_datum_nad27` |
| `projparse_longlat_datum_other` | `io_projparse_longlat_datum_other` |
| `projparse_longlat_ellps_WGS84` | `io_projparse_longlat_ellps_wgs84` |
| `projparse_longlat_ellps_GRS80` | `io_projparse_longlat_ellps_grs80` |
| `projparse_longlat_a_b` | `io_projparse_longlat_a_b` |
| `projparse_longlat_a_rf_WGS84` | `io_projparse_longlat_a_rf_wgs84` |
| `projparse_longlat_a_rf` | `io_projparse_longlat_a_rf` |
| `projparse_longlat_a_f_zero` | `io_projparse_longlat_a_f_zero` |
| `projparse_longlat_a_f_non_zero` | `io_projparse_longlat_a_f_non_zero` |
| `projparse_longlat_a_e` | `io_projparse_longlat_a_e` |
| `projparse_longlat_a_es` | `io_projparse_longlat_a_es` |
| `projparse_longlat_R` | `io_projparse_longlat_r` |
| `projparse_longlat_a` | `io_projparse_longlat_a` |
| `projparse_longlat_a_override_ellps` | `io_projparse_longlat_a_override_ellps` |
| `projparse_longlat_pm_paris` | `io_projparse_longlat_pm_paris` |
| `projparse_longlat_pm_ferro` | `io_projparse_longlat_pm_ferro` |
| `projparse_longlat_pm_numeric` | `io_projparse_longlat_pm_numeric` |
| `projparse_longlat_pm_overriding_datum` | `io_projparse_longlat_pm_overriding_datum` |

Les tests C++ comparent le WKT2 de `exportToWKT` (avec ou sans
`simulCurNodeHasId`). Le Rust utilise `create_crs_from_proj_string` puis
`Crs::export_to_wkt` avec `WktFormatter` en `Wkt2_2015`. Les deux tests qui
comparent des propriétés de l'ellipsoïde lisent `datum().ellipsoid()`.

## API

| PROJ 9.9.0 | Rust |
|---|---|
| `PROJStringParser().createFromPROJString("+proj=longlat ...")` | `create_crs_from_proj_string` (`src/iso19111/projstring.rs`) |
| `buildDatum` (`+R`, `+datum`, `+ellps`, `+a`, `+b`, `+rf`, `+f`, `+e`, `+es`) | `datum_from_proj`, avec `named_datum`, `named_ellipsoid_datum`, `identified_ellipsoid` |
| `buildPrimeMeridian` (`+pm`) | `prime_meridian_from_proj`, table `PRIME_MERIDIANS` |
| `datumDescs` | `DATUM_DESCS` |
| `->identify()` sur un ellipsoïde | `well_known_ellps_entry` (table `PJ_ELLPS`) |
| `fixupPrimeMeridan` | `meridian_for` |
| `overridePmIfNeeded` | `override_meridian` |
| `createGRF` | `frame_from_ellipsoid` |
| `Ellipsoid::guessBodyName` sans base | `guess_body` |

`create_crs_from_proj_string` garde sa signature. `well_known_pm_name` lit la
même table `PRIME_MERIDIANS` que `+pm=NOM`. `well_known_ellps` reste pour
l'export et renvoie l'identifiant de la ligne trouvée.

## Écarts volontaires

- **Corps céleste sans base de données.** `guessBodyName` interroge
  `proj.db` pour les rayons qui ne sont pas celui de la Terre. Ici, seul le
  rayon moyen terrestre (6 375 000 m, tolérance 0,7 %) donne `Earth`, sinon
  `Non-Earth body`. C'est le comportement de `PROJStringParser` sans contexte,
  et `wkt_import.rs` utilise les mêmes constantes.
- **`->identify()`** cherche dans la table `PJ_ELLPS` du crate, pas dans toute
  la base des ellipsoïdes EPSG. Une correspondance hors de cette table reste
  un ellipsoïde `unknown`, alors que PROJ pourrait lui donner un nom EPSG.
- **`+pm=` numérique** accepte seulement un nombre décimal en degrés. PROJ
  accepte aussi la notation DMS (`dmstor`), non portée ici.
- **`+towgs84=` et `+nadgrids=`** ne sont pas lus par ce lot. Le suffixe
  `using towgs84=` du nom de datum n'est donc pas écrit non plus.

## Comportements conservés

- Sans `+datum`, `+ellps` ni nombre, le datum est WGS 84 EPSG 6326. Avec
  `+pm=` non Greenwich, ce même datum devient `Unknown based on WGS 84
  ellipsoid`, avec le méridien donné (`overridePmIfNeeded`).
- `+datum=carthage` garde son nom `Carthage` et son code EPSG 6223, même avec
  un `+pm=` : `datumDescs` ne passe pas par `overridePmIfNeeded`.
- Un `+a` avec `+rf` qui ne correspond à aucun ellipsoïde nommé donne le datum
  `unknown`. Sur un corps autre que la Terre, le méridien de Greenwich devient
  `Reference meridian`.
- `+a` avec `+ellps=` garde le nouveau demi-grand axe et l'aplatissement de
  l'ellipsoïde nommé. L'assertion `298.25722356300003` de PROJ est le même
  double que `298.257223563` (écrit ainsi dans le test pour rester sous
  `clippy::excessive_precision`).
- `+pm=paris` est en grades (`PrimeMeridian::paris`, 2.5969213 grad), tandis
  que les autres noms du tableau sont en degrés.

## Non portés dans ce lot

Restent dans `docs/UNPORTED-UNIT.md` :

- `io.projparse_longlat_axis_enu`, `_axis_neu`, `_axis_swu` : chacun compare
  aussi `CoordinateOperationFactory::createOperation(EPSG:4326, crs)` avec
  `exportToPROJString`. Dans ce crate, `create_operation_proj_string` ne couvre
  que la projection dérivée de sa base, pas un couple géographique avec
  inversion d'axes.
- `io.projparse_longlat_towgs84_*`, `_nadgrids*`, `_geoidgrids*`, `_vunits`,
  `_vto_meter`, `_axisswap`, `_unitconvert_*`, `_complex` : CRS liés, pipelines
  et opérations, hors du périmètre de `buildDatum`.

## Vérification

```
cargo fmt --check                                              # ok
cargo clippy --all-targets --all-features -- -D warnings       # ok, aucune sortie
PATH=<proj-ref>/bin:$PATH PROJ_CS2CS_REQUIRED=1 \
  PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES \
  cargo test --all-features --test test_io io_projparse
# test result: ok. 20 passed; 0 failed; 0 ignored
cargo test --all-features --no-fail-fast                       # EXIT 0
```

Le harnais `cli_yaml` réécrit `docs/UNPORTED-CLI.md` à chaque exécution de la
suite complète. Ce fichier est restauré avant commit et n'est pas inclus dans
le lot.
