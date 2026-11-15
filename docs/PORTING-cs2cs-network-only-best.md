# Fiche : `cs2cs` avec réseau actif, `ONLY_BEST` et point hors grille

Mesure contre PROJ 9.9.0 compilé avec curl et TIFF (`target/proj-reference-net`). Les deux binaires
lisent les mêmes données (`PROJ_DATA` = `build/data`, sans `data/tests`) et reçoivent le même
`PROJ_NETWORK`. Les grilles viennent de `https://cdn.proj.org` par le proxy de session.

## Mesure

Banc : 30 paires, deux points par paire (un dans la grille, un hors), quatre modes : réseau ON
défaut, ON `--only-best=no`, ON `--only-best`, et OFF défaut en témoin. Soit 240 comparaisons.
Les statuts suivent `tools/compare-cs2cs/compare.awk`.

- 237 comparaisons `same` (sortie identique octet à octet).
- 3 comparaisons `status` : même point, `EPSG:4807` vers `EPSG:4171` à `60 10` (hors de France), réseau
  ON, les trois modes. Décrit plus bas.
- Le témoin réseau est actif : pour 25 des 60 points, la référence sort un autre résultat avec le
  réseau ON qu'avec OFF. Les grilles du CDN sont donc utilisées.

Limites :

- Les points « hors emprise » sont choisis à la main. L'emprise exacte de chaque grille n'est pas
  vérifiée pour toutes les paires.
- EGM96 et EGM2008 sont des grilles mondiales : leurs deux points sont dans l'emprise.
- Le banc n'a pas de paire dont la grille est absente de la base et du CDN.

Le banc est hors dépôt : `scratchpad/net-lotF/bench.sh` (sorties dans `scratchpad/net-lotF/before/`,
tableau généré par `scratchpad/net-lotF/make_table.py`).

## Écart mesuré : point hors grille, réseau ON (fermé, lot C7)

Entrée `60 10`, `EPSG:4807` vers `EPSG:4171`, réseau ON.

- Référence : `54.0000000000 11.3372291667`, par le candidat « Ballpark geographic offset ».
- Port avant le lot C7 : `*	* inf`. Aucun candidat ne donne de résultat.
- Port après le lot C7 : `54.0000000000 11.3372291667`, identique. Le candidat sans grille
  de la recherche intermédiaire et la règle du méridien altéré sont portés. Détail :
  `PORTING-cs2cs-prime-meridian-ballpark.md`. Test : `tests/unit_network_ntf_outside_grid.rs`
  (réseau, CDN requis ; sans CDN le test ne vérifie rien).

Le texte ci-dessous décrit l'état avant le lot C7.

Cause. La référence liste trois candidats (`projinfo -s EPSG:4807 -t EPSG:4171 --spatial-test
intersects -o PROJ`, `PROJ_NETWORK=ON`) :

1. `EPSG:9337` (`fr_ign_gr3df97a.tif`, xyzgridshift) ;
2. `EPSG:7811` (`fr_ign_ntf_r93.tif`, hgridshift) ;
3. `unknown id, Transformation from NTF (Paris) to NTF (Paris) altered to use prime meridian of
   ETRS89-FRA [RGF93 v1] + Ballpark geographic offset ...`, sans identifiant, emprise mondiale.

Le port n'a que les candidats 1 et 2. Les deux grilles sont instanciables avec le réseau, mais le
point est hors de leur emprise : aucun repli n'existe, donc `NoOperation`.

Le candidat 3 vient de la branche « décalage de méridien » de `createOperationsGeogToGeog`
(`src/iso19111/operation/coordinateoperationfactory.cpp`, lignes 2923 à 3100 du tag 9.9.0). Le port
ne la porte pas : `src/iso19111/factory.rs`, `forced_ballpark_operations` (sa doc indique « a prime
meridian shift … not ported »).

PROJ liste aussi ce candidat sans réseau. Le port ne l'a pas non plus dans ce cas, mais le pivot
WGS 84 sans grille donne le même résultat qu'en PROJ (`53.9999833049 11.3368356819`) : l'écart est
masqué.

Repro, depuis un dossier temporaire `$D` (`PROJ_USER_WRITABLE_DIRECTORY`) :

```sh
echo "60 10" | LD_LIBRARY_PATH=$RN/build/lib PROJ_DATA=$RN/build/data PROJ_NETWORK=ON \
  PROJ_USER_WRITABLE_DIRECTORY=$D $RN/build/bin/cs2cs -d 10 EPSG:4807 +to EPSG:4171
# 54.0000000000	11.3372291667	0.0000000000
echo "60 10" | PROJ_DATA=$RN/build/data PROJ_NETWORK=ON PROJ_USER_WRITABLE_DIRECTORY=$D \
  target/release/cs2cs -d 10 EPSG:4807 +to EPSG:4171
# *	* inf
```

Pourquoi pas corrigé dans ce lot : porter la branche touche la génération des candidats pour toutes
les paires dont les méridiens diffèrent. C'est une refonte de la sélection, hors du périmètre.

Un test rouge a été écrit puis retiré (pas de test rouge commité). Il appelait
`select_alternatives("EPSG:4807", "EPSG:4171", BestPolicy::Warn, …)` avec `PROJ_NETWORK=ON` et
attendait `54.0` / `11.3372291667` à `1e-9` près ; il échoue avec `NoOperation`.

## Écarts relevés dans le code, non mesurés (ouverts)

1. Relance sous réseau (`src/iso19111/select.rs`, condition `if policy != BestPolicy::Discard && …`).
   PROJ ne relance jamais avec DISCARD quand le réseau est actif :
   `mayNeedToReRunWithDiscardMissing = (errorIf || warnIf) && !network` (`crs_to_crs.cpp:609-611`).
   Le port relance dès qu'aucun candidat non approximatif n'est instanciable. Essayé : ajouter
   `!network_enabled` à la condition ne change pas le cas ci-dessus, car les candidats de grille sont
   instanciables avec le réseau et la relance n'a pas lieu. Pas de cas mesuré ; non corrigé.
2. Filtre disque en DISCARD sous réseau (`select.rs`, `BestPolicy::Discard => discard_missing(ranked)`).
   PROJ n'applique pas ce filtre quand le réseau est actif (`KNOWN_AVAILABLE`). Déjà noté dans
   `docs/PORTING-cs2cs-grid-policy.md`, « Deviations and open points ». Le banc n'en montre aucun effet :
   `--only-best=no` ON est identique partout, sauf le point du cas ci-dessus.
3. Candidat unique sous `Warn` avec réseau (`select.rs`, `if policy != BestPolicy::Warn || instantiable`).
   Avec réseau, PROJ rend le candidat unique sans contrôle (`crs_to_crs.cpp:617-660`, `!mayNeedToReRun`).
   Le port vérifie l'instanciabilité sur le disque. Non mesuré ; à vérifier avant de conclure.

## Tests

Les suites ont tourné sur le code de ce lot avant et après. Aucun fichier `src/` n'a changé : les
mêmes chiffres valent avant et après. Aucun test ignoré.

- `cargo test --all-features --test cli_yaml`
- `cargo test --all-features --test unit_network_core --test unit_network_local --test network_grid --test unit_network_cache --test unit_network_grids -- --test-threads=1`
- `cargo test --all-features --test cs2cs_grid_policy_candidates --test alternatives_select --test candidate_filter`

## Tableau des mesures

Réf et Port : première valeur de sortie, lat et lon en degrés (`inf` = `*	* inf`). Statut : `same`
= identique. Le réseau ON défaut est le mode `ON` sans option.

| Paire | Point (entrée) | Réf ON | Réf OFF | Port ON | Statut ON défaut | Statut ON `--only-best=no` | Statut ON `--only-best` | Statut OFF |
|---|---|---|---|---|---|---|---|---|
| NAD27-NAD83 `EPSG:4267 -> EPSG:4269` | 40 -100 | 40.000006 -100.000406 | 39.999997 -100.000416 | 40.000006 -100.000406 | same | same | same | same |
| NAD27-NAD83 `EPSG:4267 -> EPSG:4269` | 60 -90 | 60.000247 -89.999880 | 60.000227 -89.999928 | 60.000247 -89.999880 | same | same | same | same |
| NAD27-WGS84 `EPSG:4267 -> EPSG:4326` | 40 -100 | 40.000004 -100.000405 | 39.999997 -100.000416 | 40.000004 -100.000405 | same | same | same | same |
| NAD27-WGS84 `EPSG:4267 -> EPSG:4326` | 60 -90 | 60.000247 -89.999880 | 60.000227 -89.999928 | 60.000247 -89.999880 | same | same | same | same |
| NAD27-UTM15N `EPSG:4267 -> EPSG:26915` | 40 -100 | -97811.282475 4451296.818111 | -97812.214588 4451295.894317 | -97811.282475 4451296.818111 | same | same | same | same |
| NAD27-UTM15N `EPSG:4267 -> EPSG:26915` | 60 -90 | 667300.279555 6655233.320529 | 667297.669285 6655230.948030 | 667300.279555 6655233.320529 | same | same | same | same |
| NAD83-HARN `EPSG:4269 -> EPSG:4152` | 40 -100 | 39.999999 -99.999999 | 39.999993 -99.999991 | 39.999999 -99.999999 | same | same | same | same |
| NAD83-HARN `EPSG:4269 -> EPSG:4152` | 60 -90 | 60.000000 -90.000000 | 60.000000 -90.000000 | 60.000000 -90.000000 | same | same | same | same |
| NAD83-WGS84 `EPSG:4269 -> EPSG:4326` | 40 -100 | 39.999999 -99.999999 | 40.000000 -100.000000 | 39.999999 -99.999999 | same | same | same | same |
| NAD83-WGS84 `EPSG:4269 -> EPSG:4326` | 60 -90 | 60.000000 -90.000000 | 60.000000 -90.000000 | 60.000000 -90.000000 | same | same | same | same |
| NAD27-CSRS `EPSG:4267 -> EPSG:4617` | 50 -100 | 50.000001 -100.000425 | 50.000054 -100.000320 | 50.000001 -100.000425 | same | same | same | same |
| NAD27-CSRS `EPSG:4267 -> EPSG:4617` | 20 -80 | 20.000000 -80.000000 | 20.000000 -80.000000 | 20.000000 -80.000000 | same | same | same | same |
| ED50-ETRS89-ES `EPSG:4230 -> EPSG:4258` | 40 -3 | 39.998810 -3.001289 | 39.998812 -3.001284 | 39.998810 -3.001289 | same | same | same | same |
| ED50-ETRS89-ES `EPSG:4230 -> EPSG:4258` | 60 10 | 60.000000 10.000000 | 60.000000 10.000000 | 60.000000 10.000000 | same | same | same | same |
| ED50-WGS84 `EPSG:4230 -> EPSG:4326` | 40 -3 | 39.998810 -3.001289 | 39.998812 -3.001284 | 39.998810 -3.001289 | same | same | same | same |
| ED50-WGS84 `EPSG:4230 -> EPSG:4326` | 60 10 | 59.999544 9.998594 | 59.999544 9.998594 | 59.999544 9.998594 | same | same | same | same |
| NTF-RGF93 `EPSG:4807 -> EPSG:4171` | 48.85 2.35 | 43.965001 4.451651 | 43.965005 4.451646 | 43.965001 4.451651 | same | same | same | same |
| NTF-RGF93 `EPSG:4807 -> EPSG:4171` | 60 10 | 54.000000 11.337229 | 53.999983 11.336836 | inf | status | status | status | same |
| NTF-L93 `EPSG:4275 -> EPSG:2154` | 48.85 2.35 | 652249.814340 6861295.764195 | 652248.448658 6861295.464177 | 652249.814340 6861295.764195 | same | same | same | same |
| NTF-L93 `EPSG:4275 -> EPSG:2154` | 60 10 | 1101905.458628 8133859.303626 | 1101873.927571 8133865.918316 | 1101905.458628 8133859.303626 | same | same | same | same |
| OSGB36-E `EPSG:27700 -> EPSG:4258` | 530000 180000 | 51.503992 -0.128329 | 51.503991 -0.128354 | 51.503992 -0.128329 | same | same | same | same |
| OSGB36-E `EPSG:27700 -> EPSG:4258` | 100000 1000000 | 58.782730 -7.193959 | 58.782764 -7.193942 | 58.782730 -7.193959 | same | same | same | same |
| OSGB36-geo `EPSG:4277 -> EPSG:4258` | 51.5 -0.1 | 51.500513 -0.101584 | 51.500512 -0.101609 | 51.500513 -0.101584 | same | same | same | same |
| OSGB36-geo `EPSG:4277 -> EPSG:4258` | 60 10 | 60.000000 10.000000 | 60.000000 10.000000 | 60.000000 10.000000 | same | same | same | same |
| NZGD49-NZGD2000 `EPSG:4272 -> EPSG:4167` | -41.3 174.8 | -41.298276 174.800190 | -41.298288 174.800164 | -41.298276 174.800190 | same | same | same | same |
| NZGD49-NZGD2000 `EPSG:4272 -> EPSG:4167` | -20 160 | -19.998040 160.000088 | -19.998040 160.000088 | -19.998040 160.000088 | same | same | same | same |
| NZGD49-WGS84 `EPSG:4272 -> EPSG:4326` | -41.3 174.8 | -41.298276 174.800190 | -41.298288 174.800164 | -41.298276 174.800190 | same | same | same | same |
| NZGD49-WGS84 `EPSG:4272 -> EPSG:4326` | -20 160 | -19.998040 160.000088 | -19.998040 160.000088 | -19.998040 160.000088 | same | same | same | same |
| EGM96-vert `EPSG:4326 -> EPSG:4326+5773` | 40 -100 0 | 40.000000 -100.000000 | 40.000000 -100.000000 | 40.000000 -100.000000 | same | same | same | same |
| EGM96-vert `EPSG:4326 -> EPSG:4326+5773` | -45 170 0 | -45.000000 170.000000 | -45.000000 170.000000 | -45.000000 170.000000 | same | same | same | same |
| EGM2008-vert `EPSG:4979 -> EPSG:4326+3855` | 40 -100 0 | 40.000000 -100.000000 | 40.000000 -100.000000 | 40.000000 -100.000000 | same | same | same | same |
| EGM2008-vert `EPSG:4979 -> EPSG:4326+3855` | -45 170 0 | -45.000000 170.000000 | -45.000000 170.000000 | -45.000000 170.000000 | same | same | same | same |
| RAF-vert `EPSG:4258 -> EPSG:4258+5720` | 50 10 0 | 50.000000 10.000000 | 50.000000 10.000000 | 50.000000 10.000000 | same | same | same | same |
| RAF-vert `EPSG:4258 -> EPSG:4258+5720` | -40 -60 0 | -40.000000 -60.000000 | -40.000000 -60.000000 | -40.000000 -60.000000 | same | same | same | same |
| GDA94-GDA2020 `EPSG:4283 -> EPSG:7844` | -33.87 151.21 | -33.869987 151.210005 | -33.869987 151.210005 | -33.869987 151.210005 | same | same | same | same |
| GDA94-GDA2020 `EPSG:4283 -> EPSG:7844` | 10 10 | 9.999992 10.000007 | 9.999992 10.000007 | 9.999992 10.000007 | same | same | same | same |
| GDA94-WGS84 `EPSG:4283 -> EPSG:4326` | -33.87 151.21 | -33.870000 151.210000 | -33.870000 151.210000 | -33.870000 151.210000 | same | same | same | same |
| GDA94-WGS84 `EPSG:4283 -> EPSG:4326` | 10 10 | 10.000000 10.000000 | 10.000000 10.000000 | 10.000000 10.000000 | same | same | same | same |
| CH1903-CH1903p `EPSG:4149 -> EPSG:4150` | 46.95 7.45 | 46.950001 7.450001 | 46.950000 7.450000 | 46.950001 7.450001 | same | same | same | same |
| CH1903-CH1903p `EPSG:4149 -> EPSG:4150` | 60 10 | 60.000000 10.000000 | 60.000000 10.000000 | 60.000000 10.000000 | same | same | same | same |
| CH1903-WGS84 `EPSG:4149 -> EPSG:4326` | 46.95 7.45 | 46.948678 7.449048 | 46.948678 7.449048 | 46.948678 7.449048 | same | same | same | same |
| CH1903-WGS84 `EPSG:4149 -> EPSG:4326` | 60 10 | 59.997154 9.998167 | 59.997154 9.998167 | 59.997154 9.998167 | same | same | same | same |
| LV03-LV95 `EPSG:21781 -> EPSG:2056` | 600000 200000 | 2600000.083056 1200000.066080 | 2600000.000000 1200000.000000 | 2600000.083056 1200000.066080 | same | same | same | same |
| LV03-LV95 `EPSG:21781 -> EPSG:2056` | 100000 50000 | 2100000.000000 1050000.000000 | 2100000.000000 1050000.000000 | 2100000.000000 1050000.000000 | same | same | same | same |
| DHDN-ETRS89 `EPSG:4314 -> EPSG:4258` | 50 9 | 49.998852 8.998955 | 49.998851 8.998953 | 49.998852 8.998955 | same | same | same | same |
| DHDN-ETRS89 `EPSG:4314 -> EPSG:4258` | 60 10 | 59.997713 9.998659 | 59.997713 9.998659 | 59.997713 9.998659 | same | same | same | same |
| Amersfoort-ETRS89 `EPSG:4289 -> EPSG:4258` | 52.1 5.2 | 52.099018 5.199591 | 52.099019 5.199592 | 52.099018 5.199591 | same | same | same | same |
| Amersfoort-ETRS89 `EPSG:4289 -> EPSG:4258` | 60 10 | 59.998097 9.998766 | 59.998097 9.998766 | 59.998097 9.998766 | same | same | same | same |
| RD-WGS84 `EPSG:28992 -> EPSG:4326` | 155000 463000 | 52.155172 5.387204 | 52.155172 5.387204 | 52.155172 5.387204 | same | same | same | same |
| RD-WGS84 `EPSG:28992 -> EPSG:4326` | 600000 100000 | 48.729565 11.435643 | 48.729565 11.435643 | 48.729565 11.435643 | same | same | same | same |
| Tokyo-JGD2000 `EPSG:4301 -> EPSG:4612` | 35.7 139.7 | 35.703235 139.696771 | 35.703235 139.696771 | 35.703235 139.696771 | same | same | same | same |
| Tokyo-JGD2000 `EPSG:4301 -> EPSG:4612` | 10 100 | 10.005440 100.000512 | 10.005440 100.000512 | 10.005440 100.000512 | same | same | same | same |
| Tokyo-JGD2011 `EPSG:4301 -> EPSG:6668` | 35.7 139.7 | 35.703235 139.696771 | 35.703235 139.696771 | 35.703235 139.696771 | same | same | same | same |
| Tokyo-JGD2011 `EPSG:4301 -> EPSG:6668` | 10 100 | 10.005440 100.000512 | 10.005440 100.000512 | 10.005440 100.000512 | same | same | same | same |
| BD72-WGS84 `EPSG:31370 -> EPSG:4326` | 150000 170000 | 50.840411 4.368752 | 50.840411 4.368752 | 50.840411 4.368752 | same | same | same | same |
| BD72-WGS84 `EPSG:31370 -> EPSG:4326` | 500000 50000 | 49.658722 9.219465 | 49.658722 9.219465 | 49.658722 9.219465 | same | same | same | same |
| NAD83-WGS84-2 `EPSG:4152 -> EPSG:4326` | 40 -100 | 40.000007 -100.000009 | 40.000007 -100.000009 | 40.000007 -100.000009 | same | same | same | same |
| NAD83-WGS84-2 `EPSG:4152 -> EPSG:4326` | 60 -90 | 60.000000 -90.000000 | 60.000000 -90.000000 | 60.000000 -90.000000 | same | same | same | same |
| NAD83-EGM96-2 `EPSG:4269 -> EPSG:4326+5773` | 40 -100 0 | 39.999999 -99.999999 | 40.000000 -100.000000 | 39.999999 -99.999999 | same | same | same | same |
| NAD83-EGM96-2 `EPSG:4269 -> EPSG:4326+5773` | -45 170 0 | -45.000000 170.000000 | -45.000000 170.000000 | -45.000000 170.000000 | same | same | same | same |

ON differs from OFF (reference, default): 25 / 60
