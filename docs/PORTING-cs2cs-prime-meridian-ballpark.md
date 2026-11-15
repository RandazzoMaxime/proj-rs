# Port : ballpark à méridien altéré (écart 6, fermé)

Source : PROJ 9.9.0, tag `9.9.0`. `src/iso19111/operation/coordinateoperationfactory.cpp` :
`createOperationsGeogToGeog` (2923-3108, branches de méridien), `FilterResults` (1213-1440,
dont `hasOpThatContainsAreaOfInterestAndNoGrid` aux lignes 1359 et 1396),
`removeSyntheticNullTransforms` (1585), `findsOpsInRegistryWithIntermediate` (retour à 2279-2287),
`createOperationsFromDatabase` (4247-4585, branche `ALWAYS` à 4452-4456), `createOperations`
public (8066-8124).

## Constat

`cs2cs EPSG:4807 +to EPSG:2154`, point `43.2999763919 3.0333132353` (grades) : la référence
imprime `880453.0554161093 5764316.2342027659`, le port `* * inf` avant ce lot.

Le point est hors de `ntf_r93.gsb` (emprise 41° à 52° N). PROJ prend alors le candidat à aire
mondiale « Ballpark geographic offset » sur le méridien altéré. Le port ne le produisait pas.

## Cause

Deux défauts, dans l'ordre où ils interviennent.

1. Deux copies de la règle du méridien. `forced_ballpark_operations` (`factory.rs`) rendait
   `Vec::new()` quand les méridiens différaient, alors que `geographic_to_geographic`
   (`geodetic_ops.rs`, `createOperationsGeogToGeog`) porte les trois branches. Le correctif
   délègue à `geographic_to_geographic` avec `BallparkRule::Forced`.

2. Le candidat à méridien altéré ne survit pas à la recherche par CRS intermédiaire en
   `ALWAYS`. PROJ fait de `findsOpsInRegistryWithIntermediate` un simple `FilterResults(..., false)`
   (2279-2287) : filtre d'emprise et de précision, sans tri ni retrait. Le port appliquait en plus
   `select_candidates` (tri, `removeUninterestingOps`, dédoublonnage) à ce résultat intermédiaire.
   Or la liste intermédiaire contient un candidat sans grille, `NTF (Paris) to NTF (1) + NTF to
   WGS 84 (1) + Inverse of ETRS89-FRA [RGF93 v1] to WGS 84 (1)`, qui couvre la France. L'appel
   public lit ce candidat dans `hasOpThatContainsAreaOfInterestAndNoGrid` (1359, 1396), et
   `removeSyntheticNullTransforms` retire alors le ballpark, dernier de la liste (1585). Le port
   perdait ce candidat avant l'appel public, donc gardait le ballpark à tort.

Sonde : `libproj` 9.9.0 recompilé en local avec `coordinateoperationfactory.cpp` instrumenté
(hors du dépôt). En `ALWAYS`, `IGNORE_GRID_AVAILABILITY`, emprise `SMALLEST` : la liste
intermédiaire contient 5 candidats, dont le sans grille ; `hasOp...` vaut 1 ; le ballpark est
retiré. Sans le candidat sans grille (contexte `NEVER`), le ballpark reste.

## Correctif (lot C7)

- `factory.rs` : `forced_ballpark_operations` délègue à `geographic_to_geographic`, avec
  `BallparkRule::Forced` (repris du patch C5). `geodetic_ops.rs` : `geographic_to_geographic`
  devient `pub(super)`.
- `factory.rs` : la branche `ALWAYS` de la recherche par CRS intermédiaire applique `filter_only`
  (filtre seul, comme `FilterResults(..., false)`) au lieu de `select_candidates`. Les branches
  vertical (`vertical_geographic`) gardent `select_candidates` : PROJ y passe par la même
  fonction, à vérifier dans un lot séparé.
- `tests/unit_op_geog_concatenated_ntf.rs` : le test portait `IntermediateCRSUse::Allow`
  (`IF_NO_DIRECT_TRANSFORMATION`). PROJ écrit `ALWAYS` dans `test_operationfactory.cpp:1330`.
  Le test passe à `Always`. Un second test (`..._if_no_direct_keeps_the_ballpark`) fixe le
  résultat `IF_NO_DIRECT` de PROJ : 3 candidats, le troisième est le ballpark.

## Mesures (référence `libproj` 9.9.0)

| Contexte | Référence | Port avant | Port après |
|---|---|---|---|
| `ALWAYS`, `IGNORE_GRID_AVAILABILITY` (test 1330) | 2 | 2 (avec `Always` seul) | 2 |
| `IF_NO_DIRECT` (défaut de `cs2cs`), même contexte | 3 | 2 | 3 |
| `ALWAYS` avec le correctif de délégation seul | 2 | 3 | - |

- `cs2cs -d 10 EPSG:4807 +to EPSG:2154` au point ci-dessus : `880453.0554161093 5764316.2342027659`,
  identique à la référence à 1e-6 près (écart 1,16e-10 en x) ;
- `unit_op_geog_concatenated_ntf` : 2 passés ;
- `tools/compare-cs2cs.sh --no-build`, défaut et `-1` : 268 identiques / 4 écarts ≤ 1e-6 / 0 écart
  > 1e-6 / 0 statut (avant : 266 / 2 / 0 / 4) ;
- tests ciblés et `cli_yaml` : voir le rapport du lot.

## Tests ajoutés

- `tests/cs2cs_ntf_grid_ballpark.rs` : la paire ci-dessus, `PROJ_DATA=data:data/tests`
  (la grille `ntf_r93.gsb` est lue, le point est hors emprise).
- `tests/unit_network_ntf_outside_grid.rs` : `EPSG:4807 → EPSG:4171` à `60 10`, réseau ON,
  attendu `54.0000000000 11.3372291667`. Passe sur le CDN (`tests/net_cdn`) ; sans CDN, le test
  affiche un message et ne vérifie rien.
- `tests/unit_op_geog_concatenated_ntf.rs` : `Always` (2 candidats) et `IF_NO_DIRECT` (3 candidats).

Rouge avant le correctif (source de `HEAD`) : les deux tests `cs2cs_ntf_grid_ballpark` et
`unit_network_ntf_outside_grid` échouent. Le second test d'`unit_op_geog_concatenated_ntf` échoue
(2 au lieu de 3).

## Écarts ouverts

- Branches vertical de `findsOpsInRegistryWithIntermediate` : `select_candidates` reste, à
  comparer à PROJ (lot séparé).
- `EPSG:4807 → EPSG:2154` avec `--no-ballpark` : écart de 1,16e-10 en x avec la référence, qui
  était identique au bit près. Cause non trouvée ; dans la tolérance.
