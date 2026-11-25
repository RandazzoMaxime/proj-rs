# pyproj contre la bibliothèque C de proj-rs

Nos registres (suite `gie`, tests unitaires de PROJ, cas des outils, corpus de parité) sont à
zéro écart. Ils ne disent pas si un vrai programme écrit pour libproj marche avec la nôtre. Ce
document mesure ça sur pyproj, le binding Python de PROJ, le plus utilisé, qui a sa propre suite de
tests.

## Méthode

`tools/clients/pyproj.sh` compile pyproj 3.8.0 (source épinglée par son sha256) deux fois :

- contre PROJ 9.9.0 compilé avec TIFF et curl, comme le construisent les distributions et les
  wheels de pyproj (`PROJ_REF_TIFF_CURL=ON tools/build-reference.sh`, à partir des sources de la
  référence) ;
- contre notre `libproj` (`capi/tests/dist.sh --release`, avec la fonctionnalité `network`).

Il lance toute la suite de pyproj dans les deux cas, tests marqués `network` compris (ils lisent
`cdn.proj.org`, par le proxy HTTPS quand il y en a un), avec le même dossier de données (le
`share/proj` de PROJ 9.9.0 installé) et les mêmes paquets Python épinglés, dépendances
optionnelles des tests comprises (shapely, pandas, xarray). La commande `pyproj` du venv est sur
le `PATH` (`test_cli.py` la lance). Une différence vient donc de la bibliothèque, pas de
l'environnement. `tools/clients/compare_junit.py` compare les deux rapports test par test, classe
par cause les tests qui passent avec PROJ et échouent avec nous, et le script échoue dès qu'un
test a une issue différente dans les deux passes.

### Quelle libproj chaque passe charge

`tools/clients/libproj_proof.sh` (le même que pour GRASS, PDAL, SpatiaLite et libgeotiff) le
vérifie dans chaque passe, et le script échoue sinon : `ldd` des 9 modules de pyproj qui lient
`libproj.so.25` résout le fichier de la passe, aucun objet de leur chaîne n'a de `DT_RPATH`
(pyproj se lie avec un `DT_RUNPATH`), une vraie transformation (`Transformer.from_crs`) tourne sous
`LD_DEBUG=libs` et l'intercepteur `tools/clients/proj_count.c`, puis l'intercepteur reste chargé
pendant tout `pytest`. Python charge libproj par `dlopen` avec `RTLD_LOCAL` (dépendance d'un module
d'extension) : `proj_count.c` cherche alors la fonction suivante par le chemin de la libproj
chargée, `RTLD_NEXT` ne la voyant pas.

```
reference: ldd _crs.cpython-313-x86_64-linux-gnu.so: libproj.so.25 => .../pyproj/ref-prefix/lib/libproj.so.25
reference: no DT_RPATH that could load another libproj
reference: python: (500000.0000000007, 4982950.400226552)
reference: LD_DEBUG=libs: calling init: .../pyproj/ref-prefix/lib/libproj.so.25
reference: interposer: .../ref-prefix/lib/libproj.so.25.9.9.0, proj_create=4/0 proj_create_crs_to_crs_from_pj=1/0 proj_trans=1/1
reference: interposer: 0 processes mapped a libproj at start, 1 called into it, all .../ref-prefix/lib/libproj.so.25.9.9.0
proj-rs: ldd _crs.cpython-313-x86_64-linux-gnu.so: libproj.so.25 => .../pyproj/ours-prefix/lib/libproj.so.25
proj-rs: no DT_RPATH that could load another libproj
proj-rs: python: (500000.0000000007, 4982950.400226552)
proj-rs: LD_DEBUG=libs: calling init: .../pyproj/ours-prefix/lib/libproj.so.25
proj-rs: interposer: .../ours-prefix/lib/libproj.so.25, proj_create=4/0 proj_create_crs_to_crs_from_pj=1/0 proj_trans=1/1
proj-rs: interposer: 0 processes mapped a libproj at start, 1 called into it, all .../ours-prefix/lib/libproj.so.25
```

## Résultat (pyproj 3.8.0, le 2026-11-25)

Machine : Linux x86-64 (Ubuntu 24.04), 4 vCPU. pyproj 3.8.0, Python 3.13.18, pytest 9.2.0,
numpy 2.5.5, shapely 2.2.0, pandas 3.1.1, xarray 2026.9.0. PROJ 9.9.0 (`1df2be6`) avec TIFF et
curl ; proj-rs au commit `07edeec` de `main`. Exécution complète du script : 3 min 24 s (PROJ avec
TIFF et curl déjà construit ; la première fois, sa construction ajoute 3 minutes), `pytest` en
21 s avec PROJ, 63 s avec proj-rs.

| | PROJ 9.9.0 | proj-rs |
|---|---:|---:|
| Tests lancés (plus 192 sous-tests de chaque côté) | 1045 | 1045 |
| Réussis | 1044 | 1044 |
| Échoués | 0 | 0 |
| Ignorés par pyproj lui-même (`@pytest.mark.skip`, PROJ#2425) | 1 | 1 |
| Issue différente entre les deux passes | | 0 |

**pyproj 3.8.0 passe sa suite complète avec la libproj de proj-rs, test par test comme avec
PROJ 9.9.0.** Le seul test ignoré l'est sans condition dans les sources de pyproj
(`test_illegal_transformation`, `@pytest.mark.skip(reason="https://github.com/OSGeo/PROJ/issues/2425")`).

Le banc a d'abord tourné sur `8838d92`, avant #363 : 1044 contre 1043. Le test
`test_transformer.py::test_transformer_group__download_grids` échouait avec proj-rs seul :
`proj_coordoperation_get_grid_used` et `proj_coordoperation_is_instantiable` cherchaient les
grilles sur `PROJ_DATA` seulement, pas sur les chemins posés par `proj_context_set_search_paths`
(pyproj `append_data_dir`), si bien que les deux grilles téléchargées restaient
`available=False` et que pyproj les téléchargeait de nouveau
(`Expected '_download_resource_file' to not have been called. Called 2 times.`). En C, avec les
chemins `[<dossier de proj.db>, data/tests]` et sans `PROJ_DATA`, PROJ 9.9.0 donnait
`us_noaa_alaska.tif available=1 file=alaska` et `instantiable=1`, `8838d92` `available=0 file=`
et `instantiable=0`, pour 6 grilles de `EPSG:4326` vers `EPSG:2964` et de `EPSG:4267` vers
`EPSG:4269`. Le cache des grilles par contexte de #363 (`lookForGridInfo` sous le contexte) donne
le même résultat que PROJ 9.9.0 sur ces 6 grilles, et le test passe.

## Ce que le banc ne mesurait pas avant

La version précédente de ce document (exécution du 2026-11-20 : 985 réussis avec PROJ, 431 avec
proj-rs) date d'avant l'export de `geodesic.h`, de l'import PROJJSON des objets qui ne sont pas des
CRS et des chaînes PROJ sans `+type=crs` (1.1.0). Relancé tel quel sur `main`, le script donnait
985 réussis des deux côtés, 11 échoués des deux côtés, 25 ignorés et 24 non lancés. Ces 60 tests
venaient de l'environnement, et sont maintenant lancés :

- **11 échecs de `test_cli.py`** (`FileNotFoundError`) : la commande `pyproj` du venv n'était pas
  sur le `PATH` ;
- **24 tests ignorés** faute de shapely (21), xarray (2) et pandas (1), dépendances de test de
  pyproj (`pyproject.toml`, groupe `test`), désormais installées et épinglées ;
- **24 tests `network` désélectionnés** (`-m "not network"`) : `cdn.proj.org` répond par le proxy,
  ils tournent ;
- la référence était compilée **sans TIFF ni curl** : `test_transformer_group__download_grids`
  échouait aussi avec PROJ (`pipeline: Bad step definition: inv (File not found or invalid)` sur
  le `.tif` téléchargé). D'où la référence avec TIFF et curl, et notre bibliothèque avec
  `network`.

Le script préchargeait aussi à tort le `geodesic.c` de PROJ dans la passe proj-rs : son test
`nm -D ... | grep -q ' geod_init$'`, sous `set -o pipefail`, échouait quand `grep -q` fermait le
tube avant la fin de `nm` (SIGPIPE), si bien que les `geod_*` mesurés étaient ceux de PROJ. Le
préchargement est retiré ; le script exige que notre bibliothèque exporte `geod_init`, et les tests
de `Geod` passent sur notre implémentation.
