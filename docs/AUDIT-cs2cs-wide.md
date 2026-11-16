# Audit cs2cs : matrice large contre cs2cs 9.9.0

## Résumé chiffré

Outil : `tools/compare-cs2cs.sh`, cas : `tools/compare-cs2cs/cases.txt`.
Référence : `cs2cs` 9.9.0 compilé sans TIFF (`target/proj-reference`).

| Mode | Cas | identique | écart ≤ 1e-6 | écart > 1e-6 | statut différent | format texte |
|---|---|---|---|---|---|---|
| CRS séparés par des blancs (défaut) | 285 | 281 | 4 | 0 | 0 | 0 |
| un seul argument par CRS (`-1`) | 285 | 281 | 4 | 0 | 0 | 0 |

Historique (identique / écart ≤ 1e-6 / écart > 1e-6 / statut différent) :

| Étape | défaut | `-1` |
|---|---|---|
| Mesure initiale (`267bda5`) | 209 / 48 / 8 / 7 | 204 / 45 / 8 / 15 |
| Lot C1 (grades, écart 1) | 213 / 48 / 4 / 7 | 208 / 45 / 4 / 15 |
| Lots C2 (écarts 5, 7) et D (`proj` bit à bit, branche `c4c6a55` et suivants) | 262 / 2 / 4 / 4 | 262 / 2 / 4 / 4 |
| Lot C3b (`+init=epsg:`, écarts 3 et 4) | 264 / 2 / 2 / 4 | 264 / 2 / 2 / 4 |
| Lot C4 (ITRF2020 ↔ WGS 84, écart 2) | 266 / 2 / 0 / 4 | 266 / 2 / 0 / 4 |
| Lot C6 (méthode 9808, 3 cas ajoutés ; mesure avant C6 sur ces 275 cas : 266 / 2 / 3 / 4) | 269 / 2 / 0 / 4 | 269 / 2 / 0 / 4 |
| Lot C7 (ballpark à méridien altéré, écart 6) | 271 / 4 / 0 / 0 | 271 / 4 / 0 / 0 |
| Lots P4 (10 cas ajoutés, pivots et `--3d`, voir `PORTING-cs2cs-pivot-candidates.md`) | 281 / 4 / 0 / 0 | 281 / 4 / 0 / 0 |

Le lot D (ordre des opérations de PROJ dans merc, tmerc, lcc, atanh de la libm) fait passer 46 cas « écart ≤ 1e-6 » à « identique ».

- 275 cas, 229 paires distinctes, 60 cas avec option (`--only-best`, `--3d`,
  `-I`, `--no-ballpark`, `--s_epoch`, `--t_epoch`).
- L'écart toléré le plus grand vaut 2,8e-9 dans l'unité de sortie.
- Plus aucune ligne à statut différent. Les causes 1 à 7 sont corrigées ; la cause 6 l'est
  par le lot C7 (`PORTING-cs2cs-prime-meridian-ballpark.md`).
- Le cas 4267 vers 4269 : identique au `cs2cs` de référence dans les 6 variantes
  mesurées. L'écart connu décrit dans `PORTING-cs2cs-grid-policy.md` ne se
  reproduit plus (voir plus bas).
- Aucun écart n'est dû à une grille `.tif` présente côté port et absente côté
  référence (classe a) dans cette matrice.

## Méthode

```sh
cargo build --release --bin cs2cs
tools/compare-cs2cs.sh --no-build -o /tmp/cmp            # défaut
tools/compare-cs2cs.sh --no-build -1 -o /tmp/cmp1        # un argument par CRS
tools/compare-cs2cs.sh --no-build -k 'EPSG:4326|EPSG:4807'  # un cas
```

Chaque cas passe les mêmes arguments aux deux binaires, avec le même stdin
(`-d 10` en tête). `PROJ_DATA` vaut `$R/build/data`, puis `data/`, puis
`data/tests`. Le comparateur classe chaque ligne : identique, écart numérique
(max |Δ| par champ), statut différent (erreur d'un seul côté), texte.

Les points de la matrice sont des points WGS 84 de régions (France, Europe,
Royaume-Uni, États-Unis, Japon, Australie, Nouvelle-Zélande, Brésil, Antarctique,
Groenland, Pologne, Canada, monde), convertis dans chaque CRS par la référence.
Chaque point est donc dans l'emprise du CRS. La matrice couvre :

- géographiques 2D et 3D entre datums (NAD27, NAD83, WGS 84, ETRS89, ED50,
  OSGB36, RGF93, NTF, Tokyo, GDA94, GDA2020, NZGD49, NZGD2000, SIRGAS 2000) ;
- géocentriques (4978, 4936, 7789, 9988) avec géographiques et projetés ;
- projetés (UTM, Lambert-93, LCC 3347, stéréo polaire 3031 et 3413, 27700,
  2154, 3857, 6933, 3035, 5070, 2180, 2193, 28356, 31983, 6677) ;
- composés avec hauteurs (`2154+5720`, `4326+5773`, `4326+3855`, `4258+5778`,
  `7405`, `5972`, `9518`, `6697`) ;
- dynamiques avec époques (ITRF2014 7789 et 9000, ITRF2020 9988, ETRF2000 7930
  et 7931 et 9067, ETRF2014 8401 et 9069, IGS97 9001), avec temps en quatrième
  coordonnée ;
- chaînes `+proj=...`, `+init=epsg:...`, `IGNF:`, `ESRI:`, un WKT sans blanc.

## Options

| Option | Cas | Résultat |
|---|---|---|
| `--only-best` | 11 | identique (4807 vers 2154 fermé par le lot C7) |
| `--only-best=no` | 5 | identique |
| `--3d` | 8 | 4807 vers 2154 à 1,2e-10 près (lot C7) |
| `-I` | 9 | identique (écart 1 fermé, lot C1) |
| `--no-ballpark` | 6 | 4807 vers 2154 à 1,2e-10 près (lot C7) |
| `--s_epoch`, `--t_epoch` | 21 | identique (cause 7 corrigée, 3 cas auparavant en statut différent) |

## Écarts, classés par gravité

Les commandes de reproduction utilisent `-k` : la même ligne de
`cases.txt` est rejouée seule.

### Écarts numériques silencieux (classe b, c)

**1. NTF (Paris), EPSG:4807 : unité grade (classe b) — fermé, lot C1**

Cause : le port traitait un CRS géographique en grades comme un CRS sans
degrés : sortie en grades, sans DMS. PROJ lit l'entrée en grades
(`cs2cs.cpp:274-313` `instantiate_crs`, `informat = strtod` car
`srcToRadians != pi/180`, `cs2cs.cpp:1014-1017`) et imprime tout résultat
géographique en degrés (`cs2cs.cpp:211-244`, `destToRadians * RAD_TO_DEG`).
Le port lit déjà l'entrée en grades (`EPSG:4807 EPSG:27572` du yaml) : seule la
sortie manquait. Correction : `Layout::AngularLongLat` dans `src/bin/cs2cs.rs`.

Cas : `EPSG:4326 → EPSG:4807`, `EPSG:4171 → EPSG:4807`,
`EPSG:2154 → EPSG:4807`, `EPSG:4807 → EPSG:2154 [-I]`. Avant : écart de 5,43°
en latitude et 0,758° en longitude.

```sh
printf '48.8582 2.2945 0\n' | target/proj-reference/build/bin/cs2cs -d 10 EPSG:4326 +to EPSG:4807
printf '48.8582 2.2945 0\n' | target/release/cs2cs -d 10 EPSG:4326 +to EPSG:4807
```

- Référence : `48.8582696327 -0.0420038433 0`.
- Port avant : `54.2869662585 -0.0466709370 0`. Port après : identique.

Test : `tests/cs2cs_grad_units.rs`, 8 cas (entrée et sortie en grades, un
deuxième CRS en grades `EPSG:4810`, DMS par défaut).

**2. ITRF2020 (9988) vers WGS 84 (4978) : 0,33 à 0,42 m (classe c) — fermé, lot C4**

```sh
tools/compare-cs2cs.sh --no-build -k 'EPSG:9988|EPSG:4978'
printf '4200985 168325 4780236\n' | target/proj-reference/build/bin/cs2cs -d 10 EPSG:9988 +to EPSG:4978
```

- Référence : `4200985.3753091125 168324.5839754438 4780235.6707992153`.
- Port avant : `4200985 168325 4780236` (identité, opération `EPSG:10608`).

Cause : le pas d'ensemble de `createOperations` ajoutait `EPSG:10608` seul
(0,01 m) devant le pivot datum. PROJ fixe `bTriedThroughIntermediateCRS` dans
le bloc intermédiaire (`coordinateoperationfactory.cpp:4502`) et saute le pas
d'ensemble (`:4524`) : le pivot WGS 84 (G2296) en geog2D (2,01 m, 4 étapes,
`unknown id`) reste seul. Correction : `tried_through_intermediate` dans
`src/iso19111/factory.rs`. Liste de 40 candidats identique à
`projinfo -s EPSG:9988 -t EPSG:4978 --spatial-test intersects`, premier compris.
Le cas inverse `EPSG:4978 → EPSG:9988` est identique aussi.

Tests : `tests/ensemble_step_pivot.rs` (ordre des candidats ; sortie de
`cs2cs` contre les valeurs PROJ). Les écarts 3 et 4 restent ouverts.

**3. `+init=epsg:27700` vers `+init=epsg:4326` : environ 100 m (classe c), fermé par le lot C3b**

Après le lot : le port donne `-0.1283539405 51.5039908276 0`, identique à la référence à 10
décimales. Le texte ci-dessous est la mesure d'avant le lot.

```sh
tools/compare-cs2cs.sh --no-build -k '+init=epsg:27700|+init=epsg:4326'
printf '530000 180000 0\n' | target/proj-reference/build/bin/cs2cs -d 10 +init=epsg:27700 +to +init=epsg:4326
printf '530000 180000 0\n' | target/release/cs2cs -d 10 +init=epsg:27700 +to +init=epsg:4326
```

- Référence : `-0.1283539405 51.5039908276 0`, identique à `EPSG:27700 → EPSG:4326`.
- Port : `-0.1267476799 51.5034800370 0` (écart 0,0016° en longitude, 0,0005° en latitude).

Cause mesurée (lot C3). La référence ne passe pas `+init=epsg:27700` par une chaîne PROJ :
`cs2cs` appelle `proj_create("+init=epsg:27700 +type=crs")` (`apps/cs2cs.cpp:833`), et la
branche `init=epsg:` de `PROJStringParser` renvoie l'objet CRS de la base, avec son datum OSGB36
et ses opérations EPSG (`src/iso19111/io.cpp:12514-12620`). Vérifié :

- `cs2cs +init=epsg:27700 +type=crs` (référence) donne le résultat de `EPSG:27700`. Sa liste
  d'opérations porte « + axis order change », ce qui est la forme objet.
- Le port expanse `+init=epsg:27700` en `+proj=tmerc … +ellps=airy`, identique à l'export PROJ
  de 27700 (sans `+towgs84` : l'unique candidat vers WGS 84 est une grille, `createBoundCRSToWGS84IfPossible`
  n'en tire aucun). La paire devient alors une ballpark.
- La référence donne `-0.1267476799 51.5034800370` pour cette même chaîne écrite en ligne :
  le calcul du port est juste, c'est la route qui diffère.

Correction (lot C3b) : le côté `+init=epsg:` passe par la sélection ISO, avec l'override d'ordre
des axes de `io.cpp:12598-12612`. Le passage naïf avait cassé `test_cs2cs_various.yaml#59`,
`#67`, `#68` ; avec l'override ils restent verts (`docs/PORTING-cli-rest.md`, point 3). Une paire
mixte (un côté `EPSG:`, un côté `+init=`) passe aussi, comme dans la référence.

L'hypothèse du `+towgs84` de l'export lié (lot C3) était réfutée, la route objet est confirmée.

**4. `+init=epsg:4326` vers `+init=epsg:2154` : 1,04e-4 m en nord (classe b), fermé par le lot C3b**

Après le lot : `648237.1028543997 6862249.4434044389 0`, à 1e-10 m près de la référence
(`648237.1028543998`). Le même chemin que l'écart 3 le corrige.

```sh
tools/compare-cs2cs.sh --no-build -k '+init=epsg:4326|+init=epsg:2154'
printf '2.2945 48.8582 0\n' | target/proj-reference/build/bin/cs2cs -d 10 +init=epsg:4326 +to +init=epsg:2154
printf '2.2945 48.8582 0\n' | target/release/cs2cs -d 10 +init=epsg:4326 +to +init=epsg:2154
```

- Référence : `648237.1028543998 6862249.4434044389 0`, identique à `EPSG:4326 → EPSG:2154`.
- Port : `648237.1028553281 6862249.4435083456 0` (écart 1,04e-4 m en nord, 9e-7 m en est).

Mesuré (lot C3) : le même texte d'expansion du port, écrit en ligne après `+towgs84=0,0,0,0,0,0,0`,
donne la valeur exacte de la référence. Le `+towgs84` nul n'est donc pas en cause. L'écart
apparaît seulement quand la cible est `+init=epsg:2154`. La cause précise n'est pas localisée
(`expand_init`, `src/engine.rs`, ou la lecture des paramètres injectés). Même route objet que le
point 3 : la correction du point 3 devrait la couvrir aussi, à vérifier.

### Statuts différents (classe d)

**5. Un seul argument pour un `+proj` multi-mots : le port refuse (d) — corrigé (lot C2, écart 5)**

Cas : 8 lignes en mode `-1`. Les CRS `+proj=...` à plusieurs mots passés en
un seul argument échouaient en port (code 3) et réussissent en référence :

```sh
target/proj-reference/build/bin/cs2cs -d 10 "+proj=longlat +datum=WGS84" +to "+proj=utm +zone=31 +datum=WGS84" <<< "2.2945 48.8582"
target/release/cs2cs -d 10 "+proj=longlat +datum=WGS84" +to "+proj=utm +zone=31 +datum=WGS84" <<< "2.2945 48.8582"
```

- Référence : `448251.7952059543 5411932.6776697328 0`.
- Port : `cannot initialize transformation`, `cause: unknown +proj=longlat +datum=WGS84`.

Cause : `src/bin/cs2cs.rs`, branche des arguments `+`. Un argument contenant un
blanc était entouré de guillemets avant d'être joint à la définition. PROJ
(`cs2cs.cpp`, lignes 690-700) joint les arguments `+` par un blanc, sans les
citer. Correction : le guillemetage est supprimé. Le cas
`+nadgrids="dir with space/myconus"` (argument déjà cité) reste valable, car le
guillemet fait partie de l'argument. Test : `tests/cli_yaml/test_cs2cs_argv.yaml`
(6 cas, valeurs de la référence). Les mêmes CRS passés en mots séparés (mode par
défaut) donnaient déjà le résultat de la référence.

Mesure (`tools/compare-cs2cs.sh --no-build -1`) : avant 204 identiques, 45 à
écart ≤ 1e-6, 8 à écart > 1e-6, 15 à statut différent ; après 209, 48, 8, 7. Le
mode défaut ne change pas (209, 48, 8, 7).

Écart restant lié au même cas : `+nadgrids=dir with space/myconus` en un
argument sans guillemets. PROJ donne `*	* inf` (code 0), le port refuse
`+with` (code 3). Cause distincte, hors lot : voir `PORTING-cli-rest.md`.

**6. NTF (Paris) vers RGF93, statut : le port renvoyait `* * inf` (d) — corrigé (lot C7)**

Cas : `EPSG:4807 → EPSG:4171`, `EPSG:4807 → EPSG:2154`, et ses variantes
`--only-best`, `--3d`, `--no-ballpark`. Le lot C1 supposait une cause d'unité. La mesure
l'infirme : la ligne `* * inf` est la même avant et après la correction des
grades. Causes : copie incomplète de la règle du méridien altéré, et candidat sans grille
retiré trop tôt pendant la recherche par CRS intermédiaire (`ALWAYS`). Détail et
correctif : `PORTING-cs2cs-prime-meridian-ballpark.md`.

```sh
tools/compare-cs2cs.sh --no-build -k 'EPSG:4807|EPSG:2154'
printf '43.2999763919 3.0333132353\n' | target/proj-reference/build/bin/cs2cs -d 10 EPSG:4807 +to EPSG:2154
printf '43.2999763919 3.0333132353\n' | target/release/cs2cs -d 10 EPSG:4807 +to EPSG:2154
```

- Référence (`PROJ_DATA` du comparateur, avec `data/tests/ntf_r93.gsb`) :
  `880453.0554161093 5764316.2342027659`.
- Port avant le lot : `* * inf`.

Sans la grille `data/tests/ntf_r93.gsb` (`PROJ_DATA` = la seule référence), le
port et la référence donnent les mêmes valeurs à 1e-10 près :
`880407.3954572410 5764330.9733427446` (4807 vers 2154),
`38.9698363107 2.7305003045` (2154 vers 4807), `38.9701211876 5.0666926700`
(4807 vers 4171). Ce sont les valeurs de `tests/cs2cs_grad_units.rs`.

La référence ouvre `fr_ign_ntf_r93.tif` (absent), puis `ntf_r93.gsb` via la table
`grid_alternatives`. Le point est hors de l'emprise de la grille (41° à 52° N) : PROJ
prend le candidat à aire mondiale, un ballpark à méridien altéré (Paris vers Greenwich)
que le port ne construit pas (`projinfo --grid-check none` : 3 candidats chez PROJ, 2
chez le port avant le lot). Après le lot : `880453.0554161093 5764316.2342027659`, et la
matrice passe à 268 identiques / 4 écarts ≤ 1e-6. Le test `unit_op_geog_concatenated_ntf`
portait `IF_NO_DIRECT` là où PROJ écrit `ALWAYS` : il est corrigé, et un test fixe le
résultat `IF_NO_DIRECT` (3 candidats).

**7. Époques sur un CRS statique (d) — corrigé (lot C2, écart 7)**

Cas : `EPSG:7789 → EPSG:4936 [--s_epoch 2015 --t_epoch 2010]`,
`EPSG:4936 → EPSG:7789 [--s_epoch 2019]`, `EPSG:9988 → EPSG:4936 [--s_epoch
2005 --t_epoch 2015]`. La référence refuse avec « Coordinate epoch should not
be provided for a static CRS » (code 3). Le port acceptait.

```sh
tools/compare-cs2cs.sh --no-build -k 'EPSG:4936|EPSG:7789|--s_epoch 2019'
printf '4200985 168325 4780236\n' | target/proj-reference/build/bin/cs2cs -d 10 --s_epoch 2019 EPSG:4936 +to EPSG:7789
```

Cause : PROJ crée la métadonnée de coordonnées de chaque côté qui a une époque
(`proj_coordinate_metadata_create`, `src/iso19111/coordinates.cpp`,
`CoordinateMetadata::create`). Un CRS non dynamique n'est accepté que si son
CRS géodésique a des opérations de mouvement ponctuel dans la base. WGS 84
compte comme dynamique : `EPSG:4326`, `EPSG:4979` et `EPSG:4978` acceptent
une époque. `EPSG:4936` et `EPSG:4258` ne l'acceptent pas. Le port avait la
même règle (`src/iso19111/coordinate_metadata.rs`) mais `cs2cs` ne l'appelait
pas. Correction : `check_epochs` dans `src/bin/cs2cs.rs`, avec le même code de
sortie (3) et la même cause. Test : `tests/cli_yaml/test_cs2cs_epoch.yaml`
(10 cas, dont 5 refus et 5 acceptations, sorties de la référence).

Les époques sur `EPSG:7930`, `EPSG:9067` et `EPSG:4937` concordaient déjà.

## Cas 4267 vers 4269

Lu : `PORTING-cs2cs-grid-policy.md` (écart « 40 -100 »). Mesuré ici : identique
dans les 6 variantes (`EPSG:4267|EPSG:4269` avec défaut, `--only-best`,
`--only-best=no`, `--3d`, `-I`, `--no-ballpark`), sur 4 points, et
`EPSG:4267 → EPSG:4326` idem.

```sh
printf '40 -100 0\n' | target/proj-reference/build/bin/cs2cs -d 10 EPSG:4267 +to EPSG:4269
```

Les deux donnent `40.0000058947 -100.0004058367`. Le chiffre de la fiche
(`39.99999688 -100.00041559`) vient d'un autre réglage de politique et n'est
plus reproduit. La cause de l'écart d'origine (politique DISCARD, pivots)
est corrigée dans la fiche citée. Il n'en reste aucun dans cette matrice.

Le fichier de grille NADCON (`us_noaa_nadcon5_nad27_nad83_1986_conus.tif`)
manque aux deux côtés : la référence et le port tombent sur le même repli.
L'écart ne peut donc pas apparaître ici, même en cas de lecture de `.tif`.

## Grilles (classe a)

Aucun écart n'est dû à une grille présente côté port seulement. Dans les
sorties des deux côtés, les seuls fichiers de grille manquants sont
`us_noaa_nadcon5_nad27_nad83_1986_conus.tif`, `fr_ign_RAF18.tif`,
`fr_ign_gr3df97a.tif`, `es_ign_SPED2ETV2.tif`, `us_noaa_*`. Les grilles
présentes dans `data/` (`ntf_r93.gsb`, `BETA2007.gsb`,
`fr_ign_RAGTBT2016.tif`) ne sont nommées par aucune opération de la matrice.
Le port sait lire les `.tif` (`src/projections/gridshift/geotiff.rs`), mais la
matrice ne le met pas à l'épreuve.

## Non vérifié et non fait

- `tests/cli_yaml` : 3 suites vertes après le lot C6. Le cas `test_cs2cs_init_epsg.yaml#10`
  (paire mixte `EPSG:2046` vers `+init=epsg:4326`) est fermé par le lot C6, voir la fiche
  `PORTING-cs2cs-init-towgs84.md`.
- Projection sud-orientée (méthode 9808, `EPSG:2046` à `2055`) : fermée par le lot C6. Cause :
  une permutation d'axes finale `-1,-2` que PROJ n'écrit pas quand la méthode porte `+axis=wsu`.
  Trois cas (`EPSG:2046`, `EPSG:4326`, `EPSG:22275`) entrent dans `cases.txt`.
- Les messages d'erreur sur la sortie d'erreur (stderr) ne sont pas comparés.
  Ils diffèrent (par exemple `proj_create: crs not found` contre
  `no transformation`). Le comparateur ne classe que stdout et les codes retour.
- L'écart 6 reste ouvert : cause trouvée (candidat ballpark à méridien modifié non construit),
  correctif non commité (`PORTING-cs2cs-prime-meridian-ballpark.md`). Les écarts 3 et 4 sont fermés (lot C3b).
- `+init=epsg:4807` (hors matrice) : PROJ imprime en DMS, longitude d'abord, un
  point qui n'est pas celui du port (`+init=epsg:4326 +to +init=epsg:4807` sur
  `48.8582 2.2945` : PROJ `46d31'12.678"E 2d17'30.494"N`). Le port garde sa
  disposition d'avant le lot C1. Voie `+init` : même famille que les écarts 3 et 4.
- `projinfo` refuse la syntaxe `+init=` en mode non PROJ4 : les choix
  d'opération de ces paires ne sont pas lus par `projinfo`.
- Le candidat de `EPSG:4326 → EPSG:4936` diffère (référence 2, port 1), sans
  effet numérique sur les points testés.
- `projinfo -s EPSG:7789 -t EPSG:4978 --spatial-test intersects` : port 58 candidats,
  référence 56 (lot C4 sans effet : 58 avant et après). La liste diffère sur des
  pivots ITRF2014 vers NZGD2000 et NAD83 (numéros d'étape ou candidats en plus) ;
  cause non mesurée.
