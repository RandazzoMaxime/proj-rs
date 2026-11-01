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

## Résultats contre l'oracle (PROJ 9.9.0, tolérance 1e-3)

| Cas | Points | Résultat |
|---|---|---|
| `EPSG:4326` vers `EPSG:2154` | 4 | égal (écart < 1e-3) : `648237.102854 6862249.443404` au premier point |
| `EPSG:4326` vers `EPSG:5070` | 3 | égal : `-338390.587551 1894100.140043` pour (-100, 40) |
| `EPSG:4326` vers `EPSG:2230` | 1 | égal : `6693625.672175 3497301.591803` (pieds US) |
| `EPSG:4326` vers `EPSG:3035` | 1 | égal sur les valeurs principales du brief : est `3756581.008815`, nord `2890102.602314` |
| `EPSG:2230` vers `EPSG:26946` | 1 | égal |
| `EPSG:2154` vers `EPSG:32631` | 1 | **ouvert**, voir « Reste ouvert » |

Le pipeline existe pour les six paires : aucun cas `gap: no pipeline`.

## Reste ouvert

**Conflit d'oracle sur `EPSG:2154` vers `EPSG:32631` (cas ignoré, à trancher par le mainteneur).**
La valeur PROJ du brief, `452014.05470595 5411682.55322704`, est égale au chiffre près
à la sortie de l'ancien constructeur de pipeline EPSG (`crs.rs`, `pipeline_of`) : le
brief la donne comme sortie de PROJ, mais c'est notre valeur de l'ancien chemin.
La chaîne que cette crate sélectionne (`EPSG:1671`, `ETRS89-FRA [RGF93 v1] to WGS 84 (1)`,
tous paramètres nuls, donc une identité) donne `452013.6230587 5411682.0731233`.
Trois éléments disent que cette valeur est celle de PROJ :

- le test CI `tests/agree_cs2cs.rs` attend que la façade (qui donne la même identité)
  concorde avec `cs2cs` à 1e-2 près sur cette paire ;
- les écarts « 0.47 à 0.51 m » du brief sont exactement les écarts entre l'ancien
  chemin et l'identité, sur les trois paires 4326→2154, 2154→32631, 4326→3035 ;
- la valeur principale du brief pour `EPSG:4326` vers `EPSG:2154` est l'identité.

Le paquet PROJ 9.9.0 de conda-forge a été téléchargé dans le dossier de travail, mais son
exécution a été refusée par l'outil dans cette session : la confirmation reste à faire.
Le cas porte la valeur du brief, inchangée, avec `#[ignore]`.

**La ligne « PROJ prints » de `EPSG:4326` vers `EPSG:3035` du brief** (`2890102.16524512
3756580.53450501`) est la même sortie de l'ancien chemin. Elle diffère de 0.47 m de la
valeur principale du brief, que le pipeline reproduit. La valeur principale est celle
qui est testée.

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
