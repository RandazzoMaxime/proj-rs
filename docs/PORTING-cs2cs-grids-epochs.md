# Port cs2cs : grilles et époques (lot cs2cs-grids)

Lot partiel. Un cas du lot passe, 19 restent ouverts, avec la cause précise de chacun ci-dessous.

## Périmètre

Cas `tests/cli_yaml/test_cs2cs_various.yaml` ids 110, 116–135, 140, 141. Sur
`origin/main` (d920902), 20 de ces cas sont ouverts : 110, 116–128, 130, 132, 134, 135, 140, 141.
Les ids 129, 131, 133 passent déjà.

## Mesure

| | cas cs2cs ouverts | total ouverts |
|---|---|---|
| `origin/main` (d920902) | 40 | 90 |
| cette branche | 39 | 89 |

Seul `test_cs2cs_various.yaml#130` change d'état (diff des identifiants de `docs/UNPORTED-CLI.md`).
Aucun cas ne passe de réussi à ouvert.

## Changements

1. **Répertoire utilisateur** (`src/network.rs`, `default_user_dir`). Le port prenait
   `temp_dir()/proj`. PROJ 9.9 (`proj_context_get_user_writable_directory`) prend
   `PROJ_USER_WRITABLE_DIRECTORY`, puis Windows (`LOCALAPPDATA`), puis `XDG_DATA_HOME/proj`,
   puis `$HOME/Library/Application Support/proj` sur macOS ou `$HOME/.local/share/proj`.
   Le dossier temporaire contenait des grilles téléchargées (`us_nga_egm08_25.tif`,
   `de_adv_BETA2007.tif`, …) : les résultats dépendaient de l'état de la machine.
2. **Nom de grille de repli** (`src/network.rs` `grid_alternate`, `src/gridpath.rs` `resolve`).
   `open_resource_file` (`filemanager.cpp`) réessaie un nom absent avec
   `getProjGridName` (sans `.tif`) ou `getOldProjGridName` (avec `.tif`). Seul le premier
   était porté. Exemple : `de_adv_BETA2007.tif` s'ouvre comme `BETA2007.gsb`.
3. **Répertoires de recherche** (`src/gridpath.rs` `directories`). Les fixtures du dépôt
   ne sont cherchées que si `PROJ_DATA` est absent. PROJ ne cherche que `PROJ_DATA`, le
   répertoire utilisateur et le partage installé. Sinon `tmpdir` ne retirait pas la grille.
4. **Pipeline avec grille vers une cible géocentrique** (`src/crs.rs` `assemble`). L'étape
   `HorizontalGrid` était perdue : le pipeline n'avait ni la grille ni le `cart` de la cible.
   Désormais source géodésique, puis grille, puis `cart` de la cible, comme le pipeline PROJ
   `axisswap; unitconvert; hgridshift; cart +ellps=WGS84`.
5. **Bloqueurs de grille** (`src/crs.rs` `require_bridge`). Une grille absente ne bloque plus
   un choix disponible, comme `DISCARD_OPERATION_IF_MISSING_GRID` par défaut. Sans aucun
   choix, l'erreur `grid step` reste.
6. **Test unitaire** (`src/crs.rs`, `nad_grid_resolves_through_its_old_name`, remplace
   `nad_grid_is_not_a_coordinate`). L'ancienne prémisse était fausse : `us_noaa_conus.tif`
   s'ouvre par son ancien nom `conus`, présent dans les fixtures. Vérifié avec la référence
   (`PROJ_DEBUG=3` : `pj_open_lib(us_noaa_conus.tif) failed`, puis `pj_open_lib(conus) succeeded`).

## État des cas du lot

| Cas | État | Cause |
|---|---|---|
| 130 | **passe** | Changements 2 et 4 (grille `de_adv_BETA2007.tif`, pipeline géocentrique). |
| 110 | ouvert | Message « Grid … is not available » absent. Sous `--only-best`, PROJ garde les opérations à grille manquante (`KNOWN_AVAILABLE`, `pj_warn_about_missing_grid` dans `trans.cpp`). Avec les noms `NTF` et `RGF93`, notre `best_operation` retient `NTF_To_RGF_1993_1` (Helmert, sans grille), pas l'opération à grille (1) que PROJ retient. Cause probable dans `precedes` (`select.rs`), à comparer à `SortFunction` (`coordinateoperationfactory.cpp`), non vérifiée ligne à ligne. |
| 116–121, 123, 126–128 | ouverts | CRS composé `EPSG:A+B` (horizontal + vertical). `create_operations` (`factory.rs`) ne traite que les CRS avec identifiant EPSG direct ; un composé implicite n'en a pas et renvoie zéro opération (`best_pipeline` échoue, puis la façade répond `unknown CRS`). Manque : `createOperationsCompoundToGeog` (`coordinateoperationfactory.cpp`). |
| 122 | ouvert | `NAD27 → NAD83` avec `--only-best` : il faut le message de grille et `*`. Notre chemin échoue avant : « EPSG datum 6267 to 6269 needs a grid ». |
| 124 | ouvert | Chaîne `+datum=NAD27` sans grille. Notre sortie `1402285.98` ≠ `1402288.54`, opération de repli différente. |
| 125 | ouvert | `EPSG:26915 → EPSG:26715` (GH-3613) passe par WGS 84 : pivot de datum non porté (`createOperationsWithDatumPivot`). |
| 132 | ouvert | Grille absente : PROJ choisit `Ballpark geographic offset from PD/83 to WGS 84` (`projinfo` le confirme). Le point (50.5, 10) donne `4003461.37` en référence. L'identité lat/lon WGS 84 donne `4003345.54` (vérifié avec `cs2cs +proj=longlat +ellps=WGS84 +to +proj=geocent`). Le décalage de ~116 m n'est pas expliqué, donc rien n'est porté. |
| 134, 135 | ouverts | Point hors de l'emprise de la grille. PROJ écarte l'opération par zone d'utilisation et retombe sur le ballpark (identité : `49N 2E` → `4189881.02`, vérifié). Nous renvoyons `*`. Il faut le repli à l'exécution de `proj_trans` (`pj_get_suggested_operation` puis nouvel essai). |
| 140, 141 | ouverts | `ITRF2014 ↔ GDA2020` : PROJ retient `ITRF2014 to GDA2020 (1)` (Helmert avec taux, `t_epoch=2020`). Nous ne trouvons que `ITRF_2014_To_GDA2020_Null` (export « Unimplemented »), faute de recherche par pivot de datum. En plus, `--s_epoch`/`--t_epoch` ne sont pas transmis : `Engine::Pipe` met `t: 0.0`, la façade n'a pas de `t`. |

## Écarts volontaires et quirks conservés

- `default_user_dir` ne lit plus le dossier temporaire. Un ancien cache dans `$TMPDIR/proj`
  n'est plus pris en compte, ce qui est le comportement PROJ.
- Le repli `.tif` → ancien nom suit `open_resource_file`. Le test unitaire modifié en
  documente la raison.
- Le ballpark `Ballpark geographic offset` n'est pas encore porté : un essai (identité
  lat/lon) a fait régresser `#133` et a été retiré.

## Commandes lancées

- Harnais, sur `origin/main` : `test_cs2cs_various` 40 cas cs2cs ouverts, 90 au total.
- Harnais, cette branche : `PATH=…/proj-ref/bin:$PATH PROJ_CS2CS_REQUIRED=1 PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --test cli_yaml` : résultat `ok`, 39 cas cs2cs ouverts, 89 au total, aucune régression.
- `cargo test --all-features --no-fail-fast` (mêmes variables) : 487 réussis, 0 échec.
- `cargo clippy --all-targets --all-features -- -D warnings` : propre. `cargo fmt --check` : propre.
