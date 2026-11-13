# Lot `io-alternatives-closing` : fermeture de cinq tests PROJ 9.9.0

Branche de travail : worktree `agent-af43ed9c65011ff8e`, base `main`. Un commit, pas de push, pas de PR.

Les valeurs attendues viennent de `test/unit/test_c_api.cpp`, `test/unit/test_operationfactory.cpp` et
`test/unit/test_io.cpp` (PROJ 9.9.0), jamais du code testé. Les commandes `cs2cs` et `projinfo` de référence
(`target/proj-reference/build/bin`, `PROJ_DATA` du même build) ont servi à mesurer les écarts.

## Bilan réel

| Test (C++) | État | Fichier de test Rust |
|---|---|---|
| `CApi.proj_normalize_for_visualization_with_alternatives` | fermé | `tests/unit_capi_normalize_alternatives.rs` |
| `CApi.proj_coordoperation_create_inverse` | fermé | `tests/unit_capi_coordop_create_inverse.rs` |
| `CApi.proj_clone_of_obj_with_alternative_operations` | fermé | `tests/unit_capi_clone_alternatives.rs` |
| `operation.createOperation_bound_of_spherical_ocentric_to_same_type` | fermé | `tests/unit_op_bound_spherical_ocentric.rs` |
| `io.createFromUserInput` | **ouvert** (blocs objets portés, voir section 5) | `tests/unit_io_create_from_user_input.rs`, `tests/unit_io_create_object_blocks.rs` |

Quatre lignes retirées de `docs/UNPORTED-UNIT.md` (64 entrées au départ, 60 à l'arrivée). Le brief annonçait
24 tests dans ce fichier : le fichier en contient 64 à `HEAD`, le décompte ci-dessus est celui du fichier.

`docs_in_sync` n'a pas été lancé avec `UPDATE_DOCS` (consigne du lot). Les compteurs de `README.md`,
`ROADMAP.md` et `docs/UNIT-TRIAGE.md` ne correspondent donc plus au fichier : le coordinateur doit régénérer
ces fichiers après fusion.

## 1. `CApi.proj_normalize_for_visualization_with_alternatives`

**Cause.** Le candidat choisi par `proj_trans` était déjà enregistré dans `Alternatives` (`current`, équivalent
de `P->iCurCoordOp`, `trans.cpp` 233-348). Ce qui manquait : `IsoPipeline::reported_definition` et
`Proj::proj_info` ignoraient ce candidat (`info.cpp` 170-171).

**Changement.** `src/iso19111/alternatives.rs` : `Alternatives::current_item`. `src/proj_iso.rs` :
`IsoPipeline::current_item`, `description` (nom du candidat du dernier point), `reported_definition`
(définition du candidat du dernier point, normalisée comme avant). `src/proj.rs` : `proj_info` utilise
`description`.

**Écart trouvé en chemin, corrigé.** `Proj::proj_info` appelle `has_inverse`, qui sondait l'inverse au point
(0, 0) à travers les candidats. Cette sonde changeait le candidat enregistré : `proj_pj_info` après un point
rapportait le ballpark. `has_inverse` restaure maintenant l'état des points (`Alternatives::save_state`,
`restore_state`, `PointState`). `proj_trans` ne fait pas cette sonde en PROJ.

## 2. `CApi.proj_coordoperation_create_inverse`

**Cause.** `Proj::new` gardait seulement le texte d'un pipeline. Il n'y avait ni inverse, ni options de rendu
`MULTILINE`, `INDENTATION_WIDTH`, `MAX_LINE_LENGTH` (`ProjLines` n'a que `Multi` et `Single` à 2 espaces et
80 colonnes).

**Changement.** `src/iso19111/pipeline.rs` :
- `ProjStringLayout` (`Single`, `Multi { indentation_width, max_line_length }`), avec `From<ProjLines>`
  (2 et 80, comme avant). `PipelineFormatter::finish_layout` ; `render` prend le layout. Les appels existants
  de `finish(ProjLines)` ne changent pas.
- `Pipeline` : les étapes d'un pipeline. `parse` (le parseur existant), `inverse` (ordre inversé, indicateur
  `inv` basculé ; `push` et `pop` s'échangent quand ils deviennent inversés, car PROJ écrit `pop` pour
  `inv +proj=push`), `render` (le formateur existant, avec le layout).

`src/proj.rs` : champ `Proj::pipeline` renseigné par `Proj::new` pour un pipeline ; `coordoperation_create_inverse`
(retourne un `Proj` dont les étapes sont celles de l'inverse) ; `as_proj_string(layout)`.

**Écarts.** Seul un `Proj` créé par `Proj::new` garde des étapes. Les paramètres globaux et `title=` ne sont
pas gardés (comme dans `parse_proj_string`). Un `Proj` issu d'une paire (`new_known_crs`) n'a pas d'étapes :
l'inverse et `as_proj_string` le refusent avec une erreur explicite.

## 3. `CApi.proj_clone_of_obj_with_alternative_operations`

**Cause.** Il manquait `proj_trans_get_last_used_operation` et `proj_is_equivalent_to` côté `Proj`, et le
test (NAD27 vers NAD83, sans grilles) dépend du choix du candidat.

**Changement.** `src/proj.rs` (nouveau bloc `impl`) : `Proj::last_used_operation` (le candidat du dernier point
en `Proj` autonome), `Proj::is_equivalent_to`. `src/proj_iso.rs` : `IsoPipeline::is_equivalent_to`,
`is_alternatives`. `src/iso19111/alternatives.rs` : `Alternatives::is_equivalent_to`, qui compare les listes
comme `PJCoordOperation::operator==` (`proj_internal.h` 424-433) : nom, boîtes, précision, texte PROJ de chaque
candidat.

**Vérifications.** La liste des candidats de `EPSG:4267` vers `EPSG:4269` avec `--grid-check discard_missing`
est identique entre `projinfo` de référence et notre `projinfo` (dix candidats, pivots WGS 84 en tête). Le
résultat de `cs2cs` de référence (lon -59.999170, lat 40.499995) est celui de la façade après correction de
l'ordre des coordonnées ci-dessous.

**Ordre des coordonnées (pas un écart de port).** Le C++ passe `(40.5, -60)` : EPSG:4267 est en latitude
puis longitude. `Proj::new_known_crs` normalise la paire en longitude puis latitude. Le test passe donc
`(-60, 40.5)` ; l'en-tête du fichier le dit.

**Écarts.** `last_used_operation` pour une paire à un seul candidat rend toujours une copie. PROJ ne le rend
qu'après un point transformé (`trans.cpp` 371-376, index 0 fictif) ; ce port ne note pas ce point. La
comparaison de deux `Proj` autonomes (`PROJ` string ou CRS seul) se fait sur le texte, pas sur l'objet ISO :
deux codes d'un même CRS ne sont pas équivalents ici.

## 4. `operation.createOperation_bound_of_spherical_ocentric_to_same_type`

Le `docs/PORTING-unit-niche-ocentric.md` (section 1.5) disait que la fusion des deux Helmert manquait. Ce
n'était pas la cause : la fusion `1 - 4 = -3` existe déjà dans les règles de réécriture de `pipeline.rs`.
Trois causes réelles :

1. **Dispatch absent.** `createOperations` (`coordinateoperationfactory.cpp` 3675-3683) envoie un bound dont
   la base est sphérique planetocentrique vers `createOperationsFromBoundOfSphericalPlanetocentric`
   (5467-5519), avant `createOperationsBoundToBound`. Le port ne le faisait pas.
   Changement : `src/iso19111/bound_ops.rs` `bound_of_spherical_planetocentric` (le CRS géographique intermédiaire
   `… (geographic)` prend la transformation liante, puis `createOperations` vers la cible, puis
   `compute_metadata`) ; `src/iso19111/factory.rs` `create_candidates_at` (dispatch `boundSrc`, puis
   `boundDst` en `else`, comme PROJ).
2. **Extrémité bound dans la conversion géographique/géocentrique.** `Conversion::_exportToPROJString` prend la
   base d'un bound (`conversion.cpp` 3838-3845). Changement : `src/iso19111/transform_export.rs`, `bound_base`
   dans `geographic_geocentric`.
3. **Source sphérique du Helmert.** L'export de la transformation liante exige un CRS géographique (`cart`).
   Résolu par le point 1 : la source devient le CRS géographique intermédiaire, comme dans PROJ.

Chaîne obtenue : `unitconvert`, `inv geoc GRS80`, `push v_3`, `cart GRS80`, `helmert -3 -3 -3`, `inv cart clrk66`,
`pop v_3`, `geoc clrk66`, `unitconvert`, identique au C++.

## 5. `io.createFromUserInput` : ouvert

**Porté dans ce lot** (`create_object_from_user_input`, `src/iso19111/json.rs`) :
- `UserObject` gagne `VerticalDatum`, `Ensemble`, `PrimeMeridian`, `Ellipsoid`, `Operation`,
  `CoordinateMetadata`. `export_to_json` refuse ces genres (« not written by this port »).
- `urn:ogc:def:{datum|ensemble|meridian|ellipsoid|coordinateOperation}:AUTH:VERSION:CODE`, avec le repli de
  version de `createFromURNPart` (`read_urn_object`).
- `urn:ogc:def:coordinateOperation,` suivi de `coordinateOperation:AUTH::CODE` : opérations concaténées par
  `compute_metadata` (nom `A + B`). Une forme inattendue est une erreur de parsing.
- `AUTH:CODE@année` et `CRS @ année` : `CoordinateMetadata`. Un CRS statique sans mouvement est refusé.

**Non porté** (le test reste ouvert, `docs/UNPORTED-UNIT.md` le garde) :
- Composés non conformes : `EPSG:4326+4326` (géographique 4979 avec WKT COMPD_CS), `EPSG:32631+4326`
  (projeté à base 3D), `EPSG:4326+4258` (`InvalidCompoundCRSException`). La voie `+` non conforme de la
  factory n'est pas portée (`io.cpp` 8029-8160 et compléments).
- Recherche par nom (`searchObject`, `io.cpp` 8157-8299) : alias (`WGS84`), noms approchés (`UTM zone 31N`,
  `WGS 84 UTM 31 north`, `NGF IGN69 height`, `WGS 84 + EGM96`), ensembles par nom, rejets (`foobar`, `foo`,
  `omerc`, `WGS 85`). `crs_from_name` ne lit que les noms exacts.
- `NN2000:2018 height` (la garde `!text.contains(['[', '+', ':'])` de `crs_from_name` le refuse) et `ID74`.

Tests : `tests/unit_io_create_object_blocks.rs` (trois fonctions : URN typées, opération concaténée, époque).
Le test C++ n'est pas fermé.

## Commandes lancées

- `cargo fmt` puis `cargo fmt --check` : propre.
- `cargo clippy --all-targets --all-features -- -D warnings` : propre (l'`#[expect]` de `UserObject` n'est plus
  nécessaire et a été retiré).
- Tests ciblés : `unit_capi_normalize_alternatives`, `unit_capi_coordop_create_inverse`,
  `unit_capi_clone_alternatives`, `unit_op_bound_spherical_ocentric`, `unit_io_create_object_blocks`.
- Suite complète : `cargo test --all-features --no-fail-fast` avec `PATH` et `PROJ_DATA` de la référence,
  hors `docs_in_sync` et `progress_svg` (résultat dans le rapport du lot).
