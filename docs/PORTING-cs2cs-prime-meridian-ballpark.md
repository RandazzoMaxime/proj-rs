# Port : ballpark à méridien altéré (écart 6, fiche d'état)

Source : PROJ 9.9.0, tag `9.9.0`. `src/iso19111/operation/coordinateoperationfactory.cpp` :
`createOperationsGeogToGeog` (2927-3110, branches de méridien), `createOperationsGeodToGeod`
(5236), `createOperationsFromDatabase` (vers 4300-4580), `FilterResults` (1213-1440),
`removeSyntheticNullTransforms` (1585). `src/trans.cpp` (230-345) : repli entre candidats.

## Constat

`cs2cs EPSG:4807 +to EPSG:2154`, point `43.2999763919 3.0333132353` (grades) : la référence
imprime `880453.0554161093 5764316.2342027659`, le port `* * inf`.

Le point est hors de `ntf_r93.gsb` (emprise 41° à 52° N, lue dans l'en-tête NTv2). PROJ
prend alors le candidat à aire mondiale `Inverse of Transformation from ETRS89-FRA [RGF93 v1]
to NTF (Paris) altered to use prime meridian of ETRS89-FRA [RGF93 v1] + Ballpark geographic
offset from NTF (Paris) altered ...` (`trans.cpp`, repli sur les autres candidats). Ce candidat
n'existe pas dans la liste du port.

| Mesure | Référence | Port |
|---|---|---|
| `projinfo -s EPSG:4807 -t EPSG:2154 --grid-check none` | 3 candidats | 2 candidats |
| `projinfo ... --grid-check discard_missing` | 2 | 1 |

## Cause

`factory.rs`, `forced_ballpark_operations` et `ballpark_operations` rendent `Vec::new()` dès que
les méridiens de Paris et de Greenwich diffèrent. Le port de `createOperationsGeogToGeog`, qui
gère ces trois branches (`geodetic_ops.rs`, `geographic_to_geographic`), n'est pas appelé
pour ce cas. Il existe donc deux copies de la même règle, dont une incomplète.

## Correctif essayé (non commité)

`forced_ballpark_operations` délègue les deux CRS géographiques à
`geodetic_ops::geographic_to_geographic` avec `BallparkRule::Forced`. Le port de la copie
sans méridien est supprimé. Mesures :

- le point ci-dessus : `880453.0554161093 5764316.2342027659`, identique à la référence ;
- `projinfo --grid-check none` : 3 candidats, le troisième nommé comme dans PROJ ;
- matrice `tools/compare-cs2cs.sh`, défaut : 259 identiques / 2 écarts ≤ 1e-6 / 4 écarts >
  1e-6 / 7 statuts → 261 / 4 / 4 / 3 ; `-1` : 251 / 2 / 4 / 15 → 253 / 4 / 4 / 11. Les quatre
  lignes 4807 s'améliorent ; aucune ne s'aggrave ;
- `cargo test --test cs2cs_grad_units --test cs2cs_grid_policy_candidates --test facade_libproj_grids` : vert ;
- `cargo test --test cli_yaml` : une exécution sur quatre a rendu `test_projinfo.yaml#83`
  en sortie 101 (panique). Les trois autres exécutions et 150 lancements isolés de ce cas
  sont verts. Cause non trouvée.

## Régression bloquante

`cargo test --all-features` : 260 binaires, 2102 passés, 1 échec :
`tests/unit_op_geog_concatenated_ntf.rs:27`, `left: 3, right: 2`.

Le test reprend `TEST(operation, geogCRS_to_geogCRS_context_concatenated_operation)` de
PROJ 9.9.0 (`src/test/unit/test_operationfactory.cpp`, ligne 1330). Ce contexte est
`IGNORE_GRID_AVAILABILITY` et `IntermediateCRSUse::ALWAYS`. PROJ rend 2 candidats, le
port en rend 3 : le ballpark à méridien altéré s'ajoute aux deux candidats passant par le
CRS intermédiaire.

Mesure côté port (instrumentation temporaire, retirée) : `has_perfect_accuracy_result` vaut
`false` sur les deux candidats trouvés, `(1)` et `(2)`. Le port ne coupe donc pas à cet
endroit.

Hypothèses non tranchées :

- le filtre de `FilterResults` appliqué au résultat final, ou `removeSyntheticNullTransforms`
  (`coordinateoperationfactory.cpp:1585`) ;
- le retour anticipé de `createOperationsFromDatabase` (vers 4574), qui n'est pris qu'avec
  `doFilterAndCheckPerfectOp`, donc avec le mode `ALWAYS`.

Un correctif qui retire le ballpark pour `ALWAYS` seul serait une rustine : il ne suivrait
pas PROJ. Le correctif n'est donc pas commité.

## Prochaine étape

1. Lire dans PROJ 9.9.0 le chemin exact qui retire le ballpark en `ALWAYS` (les deux pistes
   ci-dessus, puis `createOperations` au niveau supérieur). Le porter entier, sans
   simplification.
2. Réappliquer le correctif de délégation, puis vérifier dans cet ordre :
   `unit_op_geog_concatenated_ntf`, `cs2cs_grid_policy_candidates`, `cli_yaml`, la matrice
   `-1` et défaut, et la suite complète.
3. Ajouter un test de la paire dans `tests/` (binaire propre : le chemin de recherche
   `PROJ_DATA` est global au processus) : `select_alternatives("EPSG:4807", "EPSG:2154",
   BestPolicy::Warn, ...)` avec `PROJ_DATA=data:data/tests`, point `43.29997639190
   3.03331323530 0`, attendu `880453.0554161093 5764316.2342027659`.
