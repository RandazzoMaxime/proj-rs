# R (sf, terra) contre la bibliothèque C de proj-rs

R est le langage que la géographie statistique a choisi : `sf` (simple features, vecteurs) et
`terra` (raster et vecteurs) en sont les deux paquets géospatiaux de référence, tous deux
construits sur GDAL. Le code compilé de ces paquets lie le GDAL du système, qui lie
`libproj.so.25` : chaque passe de ce script échange la bibliothèque sous lui par
`LD_LIBRARY_PATH` (même soname). GDAL fait tout le travail PROJ — WKT de `OGRSpatialReference`,
`OGRCreateCoordinateTransformation`, chaînes PROJ construites pour les étendues — et terra
appelle libproj elle-même à un endroit : `proj_pipelines`, qui liste les opérations d'une paire
de CRS par l'API C (`proj_create_operations`, `proj_coordoperation_is_instantiable`), et le
message de son option réseau `projNetwork` qui signale les grilles de datum non téléchargeables.

Cette suite valide donc le côté GDAL de libproj (déjà couvert par GMT), et, pour la première
fois, le chemin du **listing des opérations** appelé par un vrai client, réseau activé et
grilles absentes du disque.

## Méthode

`tools/clients/r.sh` construit R 4.5.0 depuis sa source CRAN (`R-4.5.0.tar.gz`, sha256
`3b33ea11…3ce1` vérifié), installe `sf` 1.1-3, `terra` 1.9-60 et `lwgeom` 0.2-17 depuis leurs
tarballs CRAN (sha256 vérifiés), et toutes leurs dépendances (`Depends`, `Imports`, `LinkingTo`,
récursivement : Rcpp, s2, units, wk, classInt, DBI, e1071…) plus `tinytest`, `testthat`, `ncdf4`,
`tibble`, `sp` et `dplyr` depuis l'instantané de CRAN du 2026-11-24 servi par Posit Package
Manager : la date fixe les versions, que chaque passe liste dans `out-*/packages.txt`. Il lance
ensuite chaque suite deux fois : avec la libproj de PROJ 9.9.0 (la référence de
`tools/build-reference.sh`, sans libcurl ni libtiff) puis avec notre `libproj`
(`capi/tests/dist.sh --release`). Les deux passes lisent le même `PROJ_DATA` (le `share/proj` de
la référence, puis les grilles de tests de proj-rs), réseau coupé (`PROJ_NETWORK=OFF`) ; terra
l'active en cours de route comme le fait son option `projNetwork`, et chaque ouverture réseau
répond alors `Network functionality not available` des deux côtés.

**Quelle libproj est chargée.** Le lanceur `bin/R` source `etc/ldpaths`, qui met
`R_LD_LIBRARY_PATH` devant `LD_LIBRARY_PATH` : une libproj de `/usr/local/lib` ou du système y
passerait devant celle de la passe, et les deux passes compareraient la même bibliothèque. Chaque
passe pose donc `LD_LIBRARY_PATH` et `R_LD_LIBRARY_PATH` sur son répertoire, puis un processus R
charge `sf` et `terra`, lit `/proc/self/maps` et fait échouer le script si une libproj mappée
n'est pas celle de la passe (`out-*/libcheck.txt`). Sortie de la relance :

```
-- ref
expected libproj directory: <work>/ref-prefix/lib
mapped: <work>/ref-prefix/lib/libproj.so.25.9.9.0 -> <work>/ref-prefix/lib/libproj.so.25.9.9.0
sf: 1.1.3  terra: 1.9.60  GDAL: 3.8.4  PROJ (sf): 9.4.0  PROJ (terra): 9.4.0
libproj check: ok
-- ours
expected libproj directory: <work>/ours-prefix/lib
mapped: <work>/ours-prefix/lib/libproj.so.25 -> <work>/ours-prefix/lib/libproj.so.25
sf: 1.1.3  terra: 1.9.60  GDAL: 3.8.4  PROJ (sf): 9.4.0  PROJ (terra): 9.4.0
libproj check: ok
```

(`<work>` est `target/clients/r`. `PROJ (sf)` et `PROJ (terra)` sont les versions des en-têtes
avec lesquels les paquets ont été compilés, le `proj.h` 9.4 d'Ubuntu ; la bibliothèque chargée est celle de la ligne `mapped`.)

Les suites sont :

- les 25 fichiers `*.R` des tests de sf (`R CMD BATCH --vanilla --no-timing`, chacun dans une
  copie fraîche du répertoire de tests, puisqu'ils écrivent des fichiers à côté d'eux), un
  fichier de sortie chacun, `testthat.R` compris ;
- la suite tinytest de terra : projections de rasters et de vecteurs, `extract`, crop, appels
  `proj_pipelines` sur des paires nord-américaines dont les grilles (NADCON, HPGN) manquent.

**Comparaison.** Chaque fichier de sortie est normalisé en retirant les seules durées imprimées
par R et tinytest, puis comparé à l'identique ; tout le reste compte, lignes
`Cannot open https://cdn.proj.org/…: Network functionality not available` comprises, et leur
nombre par passe est rapporté et comparé. Le script échoue (code 1) sur un fichier différent, un
fichier présent d'un seul côté (les deux répertoires sont parcourus), l'absence de la ligne
`All ok, N results` de terra d'un côté ou de l'autre, un nombre de sorties sf différent du nombre
de scripts, ou des comptes de `Cannot open` différents. La tolérance effective est celle de
l'impression de R : 7 chiffres significatifs par défaut (`options(digits = 7)`), et ce que les
tests eux-mêmes vérifient (`expect_equal` de tinytest et testthat à 1,5e-8 relatif, `all.equal`).

## Les écarts trouvés, et ce qu'ils étaient vraiment

Chaque écart est réduit en sonde C d'un appel, comparée à la libproj de référence par
`capi/tests/run.sh network_listing missing_input` (`capi/tests/network_listing.c`,
`capi/tests/missing_input.c`), et côté Rust par `tests/network_listing.rs`. Les sondes ont été
lancées sur trois bibliothèques : la référence, proj-rs avant cette PR (`eb38492`) et après.

1. **L'ordre du listing.** Réseau activé et aucune grille ouvrable, `proj_create_operations`
   de `EPSG:4326` vers `EPSG:5070` sous `PJ_CRS_EXTENT_NONE` (le défaut `use = "NONE"` de
   `proj_pipelines`) rendait, avant, l'opération à grille « NAD83 to WGS 84 (6) » en deuxième,
   devant les Helmert « (3) » et « (2) » (Hawaii, Alaska) que PROJ met en deuxième et troisième.
   La cause : `gridpath::available` croyait une grille du CDN disponible sur sa seule ligne de
   `proj.db`, là où le tri de PROJ (`USED_FOR_SORTING`) l'ouvre (`lookForGridInfo` →
   `resolveFullFilename`, `factory.cpp:3441-3450`). `available` tente maintenant l'ouverture ;
   les 53 opérations (48 sous `PJ_CRS_EXTENT_SMALLEST`) sortent dans l'ordre de PROJ.

2. **Les messages émis à l'intérieur d'un appel.** L'ouverture qui échoue logue
   `Cannot open <url>: <erreur>` sur le logger du contexte appelant
   (`networkfilemanager.cpp:1323`) : 47 lignes pour le listing `SMALLEST`, 49 pour `NONE`, une
   par grille. proj-rs n'avait aucun canal pour ces messages. `src/log_channel.rs` en donne un,
   par fil : la bibliothèque C le pointe sur son contexte au premier appel avec ce contexte
   (`context`), et la garde de chaque point d'entrée remet le précédent au retour, si bien que
   le canal ne survit jamais à l'appel ni au contexte (`proj_context_destroy` le vide aussi s'il
   le désigne). Le message n'est pas émis pendant l'instanciation d'une chaîne PROJ par
   `proj_create`, où PROJ installe son propre logger et ne garde que la dernière ligne d'erreur
   (`io.cpp:12761-12810`), ni une seconde fois quand un point refusé est rejoué pour trouver son
   message.

3. **L'ouverture à la création d'une paire.** Une fois `available` devenu une ouverture, les
   chemins qui construisent une paire, un pipeline ou une marque d'instanciabilité l'auraient
   tentée à la création ; PROJ n'en tente aucune (`proj_create_crs_to_crs` demande
   `PROJ_GRID_AVAILABILITY_KNOWN_AVAILABLE` réseau activé, `crs_to_crs.cpp:572-576`). Ces chemins
   (`select.rs`, et `crs.rs` et `nkg.rs` du constructeur de pipelines EPSG) lisent
   `gridpath::known_available`, qui ne tente rien. Ce n'était pas un défaut d'avant la PR (la
   version `eb38492` tentait aussi 0 ouverture) mais la condition pour que le point 1 n'en crée
   pas : 0 ouverture comptée par les callbacks réseau pour trois paires des deux côtés, et le test
   unitaire de `crs.rs` en compte 5 si `available` y revient.

4. **Le point dont la grille ne s'ouvre pas.** Avant cette PR, la transformation d'un point par
   une paire dont la meilleure opération demande une grille absente rendait déjà l'infini et
   `errno` 4099 (`PROJ_ERR_OTHER_NETWORK_ERROR`), comme PROJ : il manquait seulement la ligne
   `Cannot open https://cdn.proj.org/us_noaa_conus.tif: Network functionality not available`,
   qu'apporte le point 2. (Une version antérieure de ce document affirmait qu'on rendait NaN
   avec `errno` 0 : la sonde lancée sur `eb38492` le dément.)

5. **`proj_list_get_count(NULL)`** rendait -1 ; PROJ rend 0 (`c_api.cpp:8699-8704`).

6. **Les arguments manquants ou du mauvais type** de sept fonctions, vérifiés dans
   `iso19111/c_api.cpp` et par 35 appels de `missing_input.c` :
   - `proj_create_projected_crs`, `proj_create_derived_projected_crs`,
     `proj_crs_create_bound_vertical_crs`, `proj_create_operations` : un NULL donne
     `missing required input` et `PROJ_ERR_OTHER_API_MISUSE` (proj-rs disait `wrong or missing
     argument` ou `missing argument`) ;
   - `proj_create_geographic_crs_from_datum` et `proj_create_geocentric_crs_from_datum` : un
     datum NULL donne `Missing input datum_or_datum_ensemble` et `PROJ_ERR_OTHER` ; un datum du
     mauvais type, `datum or datumEnsemble should be set` et `One of Datum or DatumEnsemble should
     be defined` ;
   - un argument du mauvais type donne NULL sans rien loguer ni poser d'`errno` dans les deux
     constructeurs projetés, dans `proj_create_derived_geographic_crs` et pour un système de
     coordonnées non ellipsoïdal de `proj_create_geographic_crs_from_datum` ; il donne
     `vert_crs is not a VerticalCRS`, `hub_geographic_3D_crs is not a CRS` (PROJ accepte tout
     CRS comme pivot) et `source_crs is not a CRS or a CoordinateMetadata` ailleurs ;
   - un NULL que PROJ déréférence (les trois arguments de `proj_create_derived_geographic_crs`,
     le système de coordonnées de `proj_create_geographic_crs_from_datum`) donne NULL sans
     message ; la sonde ne les appelle pas.

## Coût du canal de messages

Le canal installé par `context` ne coûte qu'une comparaison quand le contexte ne change pas.
`bench/footprint/c/convert.c` (`points` : un million de `proj_trans`, `EPSG:4326` vers
`EPSG:2154`), médiane de 11 lancements, temps de création retiré :

| bibliothèque | par point |
|---|---:|
| proj-rs `eb38492` (sans canal) | 120 ns |
| PR telle que relue (un `Arc` alloué à chaque appel) | 174 ns |
| cette version | 123 ns |

## Résultat de la relance (2026-11-25)

Ubuntu 24.04, GDAL 3.8.4 et GEOS 3.12.1 d'Ubuntu, R 4.5.0, sf 1.1-3, terra 1.9-60, lwgeom 0.2-17,
46 paquets en tout (Rcpp 1.1.2, s2 1.1.13, units 1.0-1, wk 0.9.5, classInt 0.4-11, DBI 1.3.0,
e1071 1.7-17, tinytest 1.4.3, testthat 3.3.2, ncdf4 1.24, tibble 3.3.1, sp 2.2-4, dplyr 1.2.1…),
liste identique dans les deux passes. Sortie de `tools/clients/r.sh` :

```
identical after normalization: 26 of 27 files
ref: 25 of 25 sf outputs; terra: All ok, 981 results; Cannot open lines: 99; failures: none
ours: 25 of 25 sf outputs; terra: All ok, 981 results; Cannot open lines: 158; failures: none
the Cannot open counts differ
the two passes do NOT agree
```

| | PROJ 9.9.0 | proj-rs |
|---|---:|---:|
| Sorties sf identiques après normalisation | 25 sur 25 | 25 sur 25 |
| tinytest de terra | All ok, 981 résultats | All ok, 981 résultats |
| Lignes `Cannot open` (sf + terra) | 99 (4 + 95) | 158 (4 + 154) |
| Échecs de `R CMD BATCH` ou de la suite | aucun | aucun |

Les 25 sorties sf (testthat compris : `[ FAIL 0 | WARN 4 | SKIP 28 | PASS 806 ]` des deux côtés), `packages.txt` et
`failures.txt` absents des deux côtés : identiques. Le seul fichier qui diffère est le journal de
terra, et seulement par ses lignes `Cannot open` : les 981 vérifications passent des deux côtés,
avec les mêmes valeurs, et toutes les autres lignes sont identiques.

## Ce qui reste

**Le nombre de lignes `Cannot open` d'une suite de listings sur un même contexte** (95 contre 154
dans `test_project.R` de terra). PROJ garde les réponses de `lookForGridInfo` dans le
`DatabaseContext` du contexte, un LRU de 128 clés qui monte à 138 avant d'être élagué
(`cacheGridInfo_`, `factory.cpp:860-871, 3421-3432, 3537` ; `lru_cache.hpp:109-217`), la clé
étant le nom de la grille, l'état du réseau et `considerKnownGridsAsAvailable` : une grille qui
ne s'est pas ouverte n'est pas retentée par le même contexte tant que son entrée n'est pas
évincée. proj-rs n'a pas ce cache et retente à chaque listing. Sur un contexte, réseau activé,
`EPSG:5070` vers `EPSG:4326` (`PJ_CRS_EXTENT_NONE`, `PARTIAL_INTERSECTION`) donne, appel après
appel :

| appel | PROJ 9.9.0 | proj-rs |
|---|---:|---:|
| `USED_FOR_SORTING` | 49 | 49 |
| `DISCARD_OPERATION_IF_MISSING_GRID` | 0 | 49 |
| `USED_FOR_SORTING`, aire d'intérêt -100 30 -90 40 | 1 | 10 |
| `USED_FOR_SORTING` | 33 | 49 |
| `EPSG:4326` vers `EPSG:5070`, `USED_FOR_SORTING` | 0 | 49 |
| le premier appel, sur un nouveau contexte | 49 | 49 |

Un premier essai de port (le LRU dans `DatabaseContext`, la consultation du tri et du filtre
`DISCARD` à travers lui, la clé que `open_resource_file` insère avant d'ouvrir,
`filemanager.cpp:1866-1876`) donnait 49, 0, 0, 0, 0, 49 : les évictions de PROJ dépendent de
toute la suite de ses appels à `lookForGridInfo` (sous les noms officiels et PROJ des grilles,
avec et sans réseau, à chaque tri intermédiaire de la fabrique), que la fabrique de proj-rs ne
reproduit pas appel pour appel, et la sortie de cache après un téléchargement
(`invalidateGridInfo`, `networkfilemanager.cpp:2704`) manquait. Il a été retiré : il faut porter
cette suite d'appels pour que les comptes coïncident. Les valeurs, l'ordre des listings et les
verdicts n'en dépendent pas. Reproduction : `tools/clients/r.sh`, puis
`diff target/clients/r/out-ref/terra-tinytest.log target/clients/r/out-ours/terra-tinytest.log`.

Écarts relevés par les sondes C, présents avant cette PR (`eb38492`), hors des chemins de ces
suites, non corrigés ici (`network_listing.c` ne les compare pas, et le dit) :

- `proj_create` d'un pipeline dont la grille ne s'ouvre pas, par exemple
  `+proj=pipeline +step +inv +proj=hgridshift +grids=us_noaa_kshpgn.tif` : PROJ logue
  `proj_create: Error 1029 (File not found or invalid): pipeline: Pipeline: Bad step definition:
  inv (File not found or invalid)`, proj-rs `…: pipeline: could not find required grid(s).`
  (même code, même nombre de lignes) ;
- réseau coupé, `proj_create_operations` de `EPSG:4326` vers `EPSG:5070` sous
  `PJ_CRS_EXTENT_NONE` laisse `errno` 1029 sur le contexte chez PROJ, 0 chez proj-rs ;
- réseau coupé, `proj_list_get` d'une opération dont la grille manque logue chez PROJ
  `hgridshift: could not find required grid(s).` et
  `pipeline: Pipeline: Bad step definition: inv (File not found or invalid)`, rien chez proj-rs ;
- réseau activé, `gridpath::known_available` tient pour disponible toute grille que le point de
  terminaison servirait, là où `lookForGridInfo(considerKnownGridsAsAvailable)` ne le fait que
  pour une grille de `proj.db` avec un paquet ou une URL sous licence ouverte : une grille
  inconnue de `proj.db` est ouverte par PROJ, pas par proj-rs.
