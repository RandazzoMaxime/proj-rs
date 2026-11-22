# pyproj contre la bibliothèque C de proj-rs

Nos registres (suite `gie`, tests unitaires de PROJ, cas des outils, corpus de parité) sont à
zéro écart. Ils ne disent pas si un vrai programme écrit pour libproj marche avec la nôtre. Ce
document mesure ça sur pyproj, le binding Python de PROJ, le plus utilisé, qui a sa propre suite de
tests.

## Méthode

`tools/clients/pyproj.sh` compile pyproj 3.8.0 (source épinglée par son sha256) deux fois :

- contre PROJ 9.9.0, installé depuis la référence de `tools/build-reference.sh` ;
- contre notre `libproj` (`capi/tests/dist.sh --release`).

Il lance la suite de pyproj dans les deux cas : hors réseau (`-m "not network"`), avec le même
dossier de données (le `share/proj` de la référence). Une différence vient donc de la bibliothèque,
pas de l'environnement. `tools/clients/compare_junit.py` compare les deux rapports test par test,
et classe par cause les tests qui passent avec PROJ et échouent avec nous.

## Résultat (pyproj 3.8.0, le 2026-11-20)

| | PROJ 9.9.0 | proj-rs |
|---|---:|---:|
| Tests réussis | 985 | 431 |
| Passent avec PROJ, échouent avec proj-rs | | 276 |
| Passent avec PROJ, non lancés avec proj-rs (module qui ne se charge pas) | | 278 |
| Passent avec proj-rs, échouent avec PROJ | | 0 |

Les 11 échecs de la référence viennent de l'environnement (par exemple `projsync`, que la
référence ne compile pas) : ils ne comptent pas.

## Ce qui manque, par ordre d'importance

1. **`geodesic.h` et les 21 fonctions `geod_*` ne sont pas exportés.** PROJ installe cette API
   publique (`geod_init`, `geod_inverse`, `geod_direct`, les lignes et les polygones), et pyproj
   ne compile pas sans elle (`Geod`). Le calcul existe déjà en Rust dans le dépôt (portage de
   `geodesic.c`). Il manque l'interface C, avec des structures identiques à celles de
   `geodesic.h`, puisque l'appelant les alloue lui-même. **Bloquant.** Pour mesurer le reste, le
   script précharge le `geodesic.c` de PROJ dans le seul passage proj-rs, et le dit.
2. **Plantage dans `proj_operation_factory_context_set_allowed_intermediate_crs`** : une lecture
   au-delà du `NULL` de fin de liste. `TransformerGroup(..., pivot_crs=...)` plantait tout le
   processus. **Corrigé** par la PR #265.
3. **PROJJSON : un objet qui n'est pas un CRS n'est pas importé** (109 tests, plus les modules
   `test_crs.py` et `test_crs_maker.py`, qui ne se chargent pas) : `proj_create` sur un PROJJSON
   de type `CoordinateOperation`, `CoordinateSystem`, `Ellipsoid`, `GeodeticReferenceFrame`,
   `PrimeMeridian` ou `DatumEnsemble`.
4. **`proj_create` sur une chaîne PROJ sans `+type=crs`** (75 tests, plus le module
   `test_datum_shift.py`) : nous rendons un CRS (`proj_get_type` donne 10, 12 ou 15, géocentrique,
   géographique 2D ou projeté) là où PROJ rend une opération. La classe `Proj` de pyproj passe par
   là.
5. **`+init=<fichier>:<clé>`** (19 tests) : `+init=ITRF2008:ITRF2000`, avec les fichiers
   d'initialisation du dossier de données, n'est pas lu.
6. **URN** : un ensemble de datums (`urn:ogc:def:ensemble:EPSG::6326`, 14 tests) et une opération
   (`urn:ogc:def:coordinateOperation:EPSG::1671`, 3 tests) ne se créent pas.
7. **`proj_create_crs_to_crs_from_pj` échoue** (13 tests), **un `NULL` rendu là où PROJ rend une
   valeur** (12), **`proj_normalize_for_visualization` échoue** (5), et 26 cas isolés : listes de
   métadonnées vides, textes de `__repr__`, un ballpark au lieu de « ED50 to WGS 84 (1) ».
   Liste complète : `target/clients/pyproj/proj-rs/results.xml.regressions.tsv` après le script.
8. **pyproj exige un fichier `proj.db` sur disque.** Sans lui, `import pyproj` lève
   `DataDirError`, même si notre bibliothèque n'en a pas besoin : pyproj vérifie le fichier avant
   d'appeler PROJ. Pour un remplacement transparent, l'archive libproj devrait porter
   `share/proj/proj.db` (10 Mo), comme PROJ.

Les points 3 et 4 cachent 278 tests qui n'ont pas tourné : les chiffres bougeront quand ils seront
réglés. Le script est à relancer après chaque lot.

## Suite

Un lot par point, dans l'ordre : 1, 4, 3, puis 5 à 8. Ensuite, le même exercice sur GDAL, qui
utilise l'API C de PROJ plus largement, mais dont la compilation coûte bien plus cher.
