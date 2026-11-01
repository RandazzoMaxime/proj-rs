# Port — conversion avec ses CRS source et cible (étape 1 : modèle et export)

Sources, PROJ 9.9.0 (`src/iso19111/operation/`) :

- `conversion.cpp` : `Conversion::_exportToPROJString` (ligne ~3768). Le bloc
  `applySourceCRSModifiers` écrit le CRS géographique de la source en sens inverse
  (~3887 à 3923) et le CRS projeté de la source avec `addUnitConvertAndAxisSwap`
  inversé. Le bloc `applyTargetCRSModifiers` (~4422 à 4475) écrit l'ellipsoïde puis
  l'unité et l'ordre des axes de la cible.
- `crs.cpp` : `ProjectedCRS::addUnitConvertAndAxisSwap` (~4717) : `unitconvert
  +xy_in=m +xy_out=<unité>` quand l'unité n'est pas le mètre, puis l'échange d'axes.
- `concatenatedoperation.cpp` et `singleoperation.cpp` : une étape inversée est
  écrite entre `startInversion` et `stopInversion`.
- `coordinateoperationfactory.cpp` : `isPROJExportable` (~1503 à 1511) est vrai dès
  que `exportToPROJString` réussit, grilles comprises.

## Le problème

Une opération dont une étape est une conversion (par exemple `EPSG:4326` vers
`EPSG:2154` : `Inverse of ETRS89-FRA [RGF93 v1] to WGS 84 (1) + Lambert-93`) ne se
laissait pas écrire en pipeline PROJ. Une `Conversion` ne porte pas de CRS, donc
`ConcatenatedOperation::export_steps` refusait l'étape et `select::export` refusait
une conversion inversée. `best_pipeline` échouait, `cs2cs` retombait sur le
constructeur de pipeline EPSG (`src/crs.rs`, `pipeline_of`), qui choisit son propre
décalage de datum.

## Ce que fait PROJ

Une `SingleOperation` a un CRS source et un CRS cible. La projection est écrite
avec les modificateurs de ces deux CRS : l'axe et l'unité de la source (en sens
inverse quand la source est projetée), puis la projection sur l'ellipsoïde, puis
l'unité et l'ordre des axes de la cible. Une conversion inversée est la même écriture
entre `startInversion` et `stopInversion`, la direction avant allant de la cible
vers la source.

## Étape 1 : ce qui a changé

| Fichier | Changement |
|---|---|
| `src/iso19111/model.rs` | `Conversion` porte `source` et `target` (`Option<Box<Crs>>`, comme `Transformation`). Accesseurs `source_crs`, `target_crs`. |
| `src/iso19111/buildop.rs` | `Conversion::inverse` échange la paire, comme `Transformation::inverse`. `with_endpoints` (interne). |
| `src/iso19111/derived.rs` | `derived_parts` rend la conversion de dérivation avec sa paire (base vers CRS dérivé). L'inverse et la chaîne (`derived_to`) la gardent. |
| `src/iso19111/factory.rs` | Branche « même autorité » de `create_candidates` : la conversion porte la paire (base vers projeté). |
| `src/iso19111/projstring.rs` | `operation_proj_string` devient `projection_steps` (liste d'étapes) plus un pipeline. Nouvelle étape `unitconvert` vers l'unité projetée (pieds, pieds US). |
| `src/iso19111/conversion_export.rs` (nouveau) | `Conversion::export_steps` et `export_pipeline` : écrit une conversion à partir de sa paire, inversée entre `startInversion` et `stopInversion`. |
| `src/iso19111/transform_export.rs` | `ConcatenatedOperation::export_steps` écrit une étape conversion au lieu de la refuser. |
| `src/iso19111/select.rs` | `export` utilise la paire d'une conversion qui en porte une. `GridSteps` (voir plus bas). |
| `src/iso19111/pipeline.rs` | `ingest_proj_string` et son analyseur ne sont plus réservés aux tests. |
| `tests/iso_pipeline_oracle.rs` (nouveau) | Onze cas, un par point, `best_pipeline` puis `Operation::create` puis valeurs PROJ 9.9.0, tolérance 1e-3. |

Le cas `Conversion` sans paire garde son refus : `export_steps` rend la même erreur
qu'avant, et `Conversion::export_to_proj_string` refuse toujours une conversion
inversée, même avec sa paire (`tests/derived_operation.rs::inverse_conversion_has_no_forward_proj_string`
passe sans modification).

## Écarts volontaires

- **Pas de paire pour les conversions de la base et de WKT.** Seules les conversions
  que la fabrique chaîne reçoivent une paire. La conversion issue d'une transformation
  de méthode « axis order reversal » (`factory.rs`, ~3527) n'en reçoit pas, pour ne
  pas changer `cs2cs` sur ces cas.
- **`GridSteps`.** Le classement des candidats de `createOperations` (`projinfo`,
  `sort_candidates`) compte les étapes de grille comme écrivables, comme
  `isPROJExportable`. Le choix de `cs2cs` (`best_pipeline_with`) garde l'écriture sans
  grille qu'il avait déjà, pour que son pipeline ne change pas. Sans cette séparation,
  `projinfo EPSG:23031 EPSG:4326 --bbox …` donnait 10 candidats au lieu de 9 : la
  chaîne à grille `ED50 to WGS 84 (41)` passait après la variante ballpark, que la
  sélection retire, et la règle de retrait ne voyait plus la ballpark en dernier.
- **Unité projetée.** `operation_proj_string` écrivait le résultat en mètres pour une
  cible en pieds. Il écrit maintenant `unitconvert +xy_in=m +xy_out=us-ft` (ou la
  forme `ft`), comme `addUnitConvertAndAxisSwap`. Effet de bord voulu : `cs2cs
  EPSG:4326 EPSG:2230` donne maintenant les valeurs PROJ.

## Étape 2 : `cs2cs`

Aucun changement de code : `cs2cs` prend déjà le choix ISO (`select_pipeline`) en premier,
et le pipeline qu'il obtient est désormais celui de PROJ pour les paires du brief. Vérifié
avec le binaire de cette crate, entrée en ordre d'autorité (latitude d'abord pour
`EPSG:4326`), sortie à 8 décimales, contre les valeurs PROJ :

| Paire | Points | Résultat |
|---|---|---|
| `EPSG:4326` vers `EPSG:2154` | 4 | égal aux 4 points |
| `EPSG:4326` vers `EPSG:5070` | 3 | égal aux 3 points, dont (-84.4, 33.7) : `1065090.92619740` (était 16524808) |
| `EPSG:4326` vers `EPSG:2230` | 1 | égal |
| `EPSG:4326` vers `EPSG:3035` | 1 sur 4 | égal au point d'Eiffel (`N 2890102.60231350`, `E 3756581.00881472`) |
| `EPSG:2154` vers `EPSG:32631` | 2 | égal aux 2 points |

`tests/agree_cs2cs.rs` : `cli_gap` passe à faux pour ces cinq paires. Le test juge
chaque point : une paire marquée qui concorde sur un point échoue en CI (« now agrees
with PROJ, clear the gap flag »). Pour `EPSG:4326` vers `EPSG:3035`, le point d'Eiffel
concorde ; on ne peut pas garder le drapeau à vrai sans échouer. Les trois autres points
de cette paire n'ont pas de valeur PROJ dans le brief : la CI les jugera.

## Résultats contre l'oracle (PROJ 9.9.0, tolérance 1e-3)

| Cas | Points | Résultat |
|---|---|---|
| `EPSG:4326` vers `EPSG:2154` | 4 | égal (écart < 1e-3) : `648237.102854 6862249.443404` au premier point |
| `EPSG:4326` vers `EPSG:5070` | 3 | égal : `-338390.587551 1894100.140043` pour (-100, 40) |
| `EPSG:4326` vers `EPSG:2230` | 1 | égal : `6693625.672175 3497301.591803` (pieds US) |
| `EPSG:4326` vers `EPSG:3035` | 1 | égal sur les valeurs principales du brief : est `3756581.008815`, nord `2890102.602314` |
| `EPSG:2230` vers `EPSG:26946` | 1 | égal |
| `EPSG:2154` vers `EPSG:32631` | 2 | égal : `452013.62305875 5411682.07312335` au premier point, `500000.0 5149603.36165604` au second |

Le pipeline existe pour les six paires : aucun cas `gap: no pipeline`.

## Correction de l'oracle du brief

Le brief donnait pour `EPSG:2154` vers `EPSG:32631` la valeur `452014.05470595
5411682.55322704`. C'était la sortie de l'ancien constructeur de pipeline EPSG
(`crs.rs`, `pipeline_of`), et non la valeur de PROJ : le brief lisait la première valeur
de la ligne du journal CI (`Ok(got) vs PROJ (want)`), qui est notre cs2cs. La valeur
PROJ est celle qui suit `vs PROJ`, à savoir `452013.62305875 5411682.07312335` pour
`(652000, 6862000)` et `500000.0 5149603.36165604` pour `(700000, 6600000)`. C'est la
chaîne identité `EPSG:1671` (paramètres nuls), que cette crate sélectionne. Le cas est
réactivé avec ces valeurs.

Même correction pour la ligne « PROJ prints » de `EPSG:4326` vers `EPSG:3035` du brief
(`2890102.16524512 3756580.53450501`) : elle était l'ancienne sortie. La valeur PROJ est
la principale, `E 3756581.00881472, N 2890102.6023135`, testée et atteinte.

**Étapes 2 et 3 non commencées.** Elles dépendent de l'oracle complet (les drapeaux
`cli_gap` et `known_gap` ne se mettent à faux que sur des paires vérifiées). La règle
d'arrêt du brief s'applique : étape 1 non vérifiée, arrêt.

**Documents générés désalignés.** Le cas `test_cs2cs_various.yaml#100` passe désormais.
`docs/UNPORTED-CLI.md` est réécrit (ligne retirée, ligne #125 modifiée : elle échoue
toujours, sur un écart numérique au lieu d'un code de sortie 3). `cs2cs` ouverts :
22 → 21. Le README, le ROADMAP et `docs/progress.svg` portent encore 22, donc
`counts_in_docs_match_the_ledgers` et `progress_chart_is_current` échouent. Je ne les
ai pas régénérés : `UPDATE_DOCS=1 cargo test --test docs_in_sync` et
`UPDATE_DOCS=1 cargo test --test progress_svg` sont à lancer par le mainteneur.

## Commandes et codes de sortie

| Commande | Code | Note |
|---|---|---|
| `cargo test --all-features --no-fail-fast` (référence, avant changement) | 0 | 781 passés, 12 ignorés |
| `cargo fmt --check` (après changements) | 0 | |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | |
| `cargo test --all-features --no-fail-fast` (final) | 101 | 789 passés, 2 échecs documentaires (ci-dessus), 13 ignorés |
| `cargo test --all-features --test iso_pipeline_oracle` | 0 | 10 passés, 1 ignoré |
| `cargo test --all-features --test derived_operation` | 0 | 4 passés, dont `inverse_conversion_has_no_forward_proj_string` |
| `cargo test --all-features --test cli_yaml` | 0 | 1 passé ; réécrit `docs/UNPORTED-CLI.md` (ligne #100 retirée) |
