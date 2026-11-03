# Port : extension PROJ-string pour les paramètres non utilisés

Source : PROJ 9.9.0, `src/iso19111/io.cpp` (récupéré depuis
`https://raw.githubusercontent.com/OSGeo/PROJ/9.9.0/src/iso19111/io.cpp`).

## Fonctions C++ lues

| Fonction (`io.cpp`) | Rôle |
|---|---|
| `PROJStringParser::createFromPROJString` (~12470) | Appelle `pj_create_internal`, retire les clés non reconnues, reconstruit `projString_`, puis lance les passes de construction. |
| bloc « Remove parameters not understood by PROJ » (~12790-12845) | Ne garde que les clés dont `pj->params[].used == 1`, une seule occurrence par clé (`foundKeys`), et `krovak`+`alpha` / `geoid_crs`+`geoidgrids` par cas particulier. Ne s'applique que si `steps_.size() == 1`. |
| `hasUnusedParameters` (~10763) | Vrai si une clé de `paramValues` n'est pas `usedByParser`, hors `no_defs`. Ne vaut que pour une étape. |
| `getParamValue`, `getParamValueK`, `hasParamValue` (~10700-10760) | Marquent la première occurrence seulement. |
| `buildDatum` (~11114), `buildPrimeMeridian` (~11057), `buildUnit` (~10942) | Lectures de datum, pm, unités. |
| `buildGeodeticCRS` (~11663) | `EXTENSION_PROJ4` si `hasUnusedParameters` ou `lon_0 != 0` (étape géographique). |
| `buildGeocentricCRS` (~11702) | `EXTENSION_PROJ4` si `hasUnusedParameters`. |
| `buildProjectedCRS` (~11906, ~12407) | Lectures par méthode, puis `EXTENSION_PROJ4` si `hasUnusedParameters`. |
| `buildEllipsoidalCS` / `buildSphericalCS` / `processAxisSwap` (~11500-11615) | `axis`, `geoc`, `vunits`, `vto_meter`, `geoidgrids`. |
| `buildBoundOrCompoundCRSIfNeeded` (~11761) | `nadgrids`, `towgs84`, `geoidgrids`, `geoid_crs` (seulement avec `geoidgrids`). |

## Paramètres lus par le constructeur (`usedByParser`)

Pour une chaîne à une étape, la liste lue dépend du type de CRS.

- **Commun** (toutes les formes) : `ellps`, `datum`, `R`, `a`, `b`, `rf`, `f`,
  `es`, `e`, `pm`, `nadgrids`, `towgs84` (`buildDatum`) ; `units` (`buildUnit` ou
  `ignoreRetVal`) ; `type` (dispatch) ; `geoidgrids` (`buildBoundOrCompound`),
  et `geoid_crs` seulement si `geoidgrids` est présent.
- **Géographique** (`longlat`, `lonlat`, `latlong`, y compris `+geoc`) : commun,
  plus `axis`, `geoc`, `lon_0` (lu, et provoque l'extension s'il est non nul).
  `vunits` et `vto_meter` sont lus seulement sans `+geoc`.
- **Géocentrique** (`geocent`) : commun seulement. `axis`, `to_meter`,
  `vunits`, `vto_meter` ne sont pas lus.
- **Projeté** : commun, plus `to_meter`, `axis`, `geoc` (la base géodésique est
  construite sur la même étape), `vunits`/`vto_meter` sans `+geoc`, les
  paramètres de la méthode, les flags `proj_aux` et les clés de sélection :
  - `tmerc` : `approx` ; `utm` : `approx`, `zone`, `south` (et non les
    paramètres de la méthode, car `utm` appelle `createUTM`) ;
  - `lcc` : `lat_0`, `lat_1`, `lat_2`, `k` ; `aeqd` : `guam` ; `geos` : `sweep` ;
  - `omerc` : `no_rot`, `no_uoff`, `no_off`, `lat_1`, `lon_1`, `lat_2`, `lon_2`,
    `alpha`, `gamma` ; `somerc` : `alpha`, `gamma`, `lonc`, `lon_0`, `k` ;
  - `krovak`, `mod_krovak` : `axis`, `czech` ;
  - `merc` : `a`, `b`, `lat_ts`, `k`, `R_C` ;
  - `stere` : `lat_0`, `k`, et `lat_ts` seulement si `lat_0` est polaire ;
  - `ortho` : `k`, `alpha` ; `cea` : `k`, `lat_ts`.

Un `proj_aux` `k=v` n'est lu que si la chaîne a `k=v` (`+shape=horizontal` ne
choisit pas la méthode `shape=square`, et `shape` reste inutilisé).

## Ce qui a été implémenté

`src/iso19111/projstring.rs` :

- `keeps_proj_string` remplace le cas spécial `over`/`lon_wrap`
  (`OPERATION_ONLY_PARAMS`, supprimé). Il applique la règle de
  `buildGeodeticCRS`, `buildGeocentricCRS` et `buildProjectedCRS`, et le
  `lon_0 != 0` géographique.
- `has_unused_parameters` : une clé reconnue, non lue, ou une deuxième
  occurrence d'une clé lue, déclenche l'extension. `proj`, `title`, `inv` et
  `no_defs` ne comptent pas.
- `recognized` : une clé non reconnue par `pj` est ignorée (ni compte, ni dans
  le texte de l'extension). La liste reconnue est `engine::GLOBAL_KEYS` (rendue
  `pub(crate)`), plus les paramètres et `proj_aux` des méthodes du nom PROJ.
  `wktext` n'est pas reconnu (test PROJ `io.projparse_longlat_wktext`).
- `extension_tokens` reconstruit le texte comme `projString_` : premières
  occurrences des clés reconnues, sans `title`.
- `attach_proj_extension` couvre maintenant les CRS géocentriques et sphériques
  (avant, seuls les géographiques et projetés recevaient l'extension).
- `proj_based_projected` ne pose plus l'extension inconditionnellement.
- `method_by_code` a été remplacé par une recherche par nom PROJ
  (`named_methods`) : `method.code` ne retrouvait pas les méthodes
  `somerc`, `stere`, `peirce_q`.

Tests : `tests/unit_proj_unused_params.rs` (16 tests). Les attendus viennent du
C++ (`EXTENSION_PROJ4` = la chaîne d'entrée sans `title`/`inv` et sans clés non
reconnues ; les tests d'export reprennent les chaînes des tests PROJ déjà
présents dans le dépôt).

## Écarts et points ouverts

- **Approximation de `used`** : PROJ marque les clés lues pendant les appels,
  dans l'ordre et avec des court-circuits. Ici les lectures sont listées par
  type de CRS. Les court-circuits (`omerc` avec `no_rot`, `ortho` avec `alpha`
  selon `k`) ne sont pas reproduits : une clé lue seulement dans une branche
  court-circuitée est comptée comme lue.
- **Méthodes** : les paramètres lus sont l'union des méthodes de même nom PROJ,
  et non de la méthode choisie. Une clé d'une autre variante peut donc être
  comptée comme lue.
- **CRS PROJ-based** (`ob_tran`, `mbt_s`, ...) : pas de table de paramètres ;
  toute clé est reconnue, donc une clé inconnue de PROJ garde la chaîne.
- **`+to_meter` sur `geocent`** : PROJ lit seulement `+units` (`buildUnit(step,
  "units", "")`). La crate lit aussi `to_meter` pour l'unité. Le nouveau critère
  rend `to_meter` non lu, donc l'extension est gardée, mais l'unité n'est pas
  changée. Non corrigé ici (hors périmètre).
- **Remarque `PROJ CRS string: ...`** : elle est dans `crs.cpp` 9.9.0
  (`CRS::setProperties`, préfixe `PROJ CRS string: `, suffixe `. `). Le port
  l'écrit comme PROJ pour toutes les extensions, et non plus seulement pour
  `+over`.
- **Casse** : PROJ compare les clés avec `ci_equal`. Ici la comparaison est
  sensible à la casse.
- **`k` / `k_0`** : `getParamValueK` ne marque que la première des deux. La
  règle `same_param` les traite comme une seule clé lue, donc la seconde compte
  comme non lue, comme en C++.
- **`cea` avec `+R_A`** : non vérifié contre la table de PROJ (le `proj_aux` de
  `cea` n'est pas dans cette liste).

## Commandes et résultats

```
cargo fmt --check                                  # OK
cargo clippy --all-targets --all-features -- -D warnings   # OK
PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast
```

| Run | passés | échoués | ignorés |
|---|---|---|---|
| baseline (branche avant modification) | 1367 | 0 | 26 |
| après règle, avant corrections | 1359 | 8 | 26 |
| après corrections (`pj` reconnu, `zone`, `somerc` `lon_0`, `k=v`) | 1373 | 9 | 26 |
| final | 1383 | 0 | 26 |

Le total final inclut les 16 tests de `unit_proj_unused_params`. Le ratchet
gie (`tests/gie/passing.txt`) n'a pas été modifié, le compte n'a pas changé.
