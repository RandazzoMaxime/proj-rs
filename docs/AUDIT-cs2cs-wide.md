# Audit cs2cs : matrice large contre cs2cs 9.9.0

## Résumé chiffré

Outil : `tools/compare-cs2cs.sh`, cas : `tools/compare-cs2cs/cases.txt`.
Référence : `cs2cs` 9.9.0 compilé sans TIFF (`target/proj-reference`).

| Mode | Cas | identique | écart ≤ 1e-6 | écart > 1e-6 | statut différent | format texte |
|---|---|---|---|---|---|---|
| CRS séparés par des blancs (défaut) | 272 | 213 | 48 | 4 | 7 | 0 |
| un seul argument par CRS (`-1`) | 272 | 208 | 45 | 4 | 15 | 0 |

Avant le lot C1 (unités grades) : défaut 209 / 48 / 8 / 7 ; `-1` 204 / 45 / 8 / 15.
Les 4 écarts numériques fermés sont les cas 4807 ci-dessous (écart 1).

- 272 cas, 226 paires distinctes, 60 cas avec option (`--only-best`, `--3d`,
  `-I`, `--no-ballpark`, `--s_epoch`, `--t_epoch`).
- L'écart toléré le plus grand vaut 2,8e-9 dans l'unité de sortie.
- Les 11 lignes à écart > 1e-6 ou à statut différent, en mode par défaut, se
  ramènent à 5 causes distinctes (2, 3-4, 6, 7). Elles sont classées par gravité
  plus bas. Le mode `-1` ajoute 8 lignes à statut différent (argument `+`
  multi-mots, cause 5 ci-dessous).
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
| `--only-best` | 11 | identique, sauf 4807 vers 2154 (statut, cause 6) |
| `--only-best=no` | 5 | identique |
| `--3d` | 8 | identique, sauf 4807 vers 2154 (statut, cause 6) |
| `-I` | 9 | identique (écart 1 fermé, lot C1) |
| `--no-ballpark` | 6 | identique |
| `--s_epoch`, `--t_epoch` | 21 | identique, sauf 3 cas (cause 7) |

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

**2. ITRF2020 (9988) vers WGS 84 (4978) : 0,33 à 0,42 m (classe c)**

```sh
tools/compare-cs2cs.sh --no-build -k 'EPSG:9988|EPSG:4978'
printf '4200985 168325 4780236\n' | target/proj-reference/build/bin/cs2cs -d 10 EPSG:9988 +to EPSG:4978
```

- Référence : `4200985.3753091125 168324.5839754438 4780235.6707992153`.
- Port : `4200985 168325 4780236` (identité).

Choix d'opération différent. La référence retient une opération composée sans
code EPSG (`unknown id`, 4 étapes, 2,01 m). Le port retient
`EPSG:10608` (0,01 m). `projinfo` n'affiche que `+proj=noop` pour
l'opération de la référence, ce qui cache le décalage de 0,4 m (non détaillé).
Le cas inverse `EPSG:4978 → EPSG:9988` a le même écart.

**3. `+init=epsg:27700` vers `+init=epsg:4326` : environ 100 m (classe c)**

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

Pour corriger, il faut router le côté `+init=epsg:` par la sélection ISO, avec l'override
d'ordre des axes de `io.cpp:12598-12612` (lon/lat, E/N). C'est ce qui a déjà cassé
`test_cs2cs_various.yaml#59`, `#67`, `#68` (`docs/PORTING-cli-rest.md`, point 3). Une paire
mixte (un côté `EPSG:`, un côté `+init=`) est refusée par le port aujourd'hui
(`no transformation … the selection writes no pipeline`), la référence la traite.

Non corrigé (lot C3, deux tentatives : l'hypothèse du `+towgs84` de l'export lié est réfutée, la
route objet est confirmée).

**4. `+init=epsg:4326` vers `+init=epsg:2154` : 1,04e-4 m en nord (classe b)**

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

**5. Un seul argument pour un `+proj` multi-mots : le port refuse (d)**

Cas : 8 lignes en mode `-1`. Les CRS `+proj=...` à plusieurs mots passés en
un seul argument échouent en port (code 3) et réussissent en référence :

```sh
target/proj-reference/build/bin/cs2cs -d 10 "+proj=longlat +datum=WGS84" +to "+proj=utm +zone=31 +datum=WGS84" <<< "2.2945 48.8582"
target/release/cs2cs -d 10 "+proj=longlat +datum=WGS84" +to "+proj=utm +zone=31 +datum=WGS84" <<< "2.2945 48.8582"
```

- Référence : `448251.7952059543 5411932.6776697328 0`.
- Port : `cannot initialize transformation`, `cause: unknown +proj=longlat +datum=WGS84`.

Cause (lecture du code) : `src/bin/cs2cs.rs`, branche des arguments `+`
(vers la ligne 1466). Un argument contenant un blanc est entouré de guillemets
avant d'être passé à la définition. PROJ (`cs2cs.cpp`, lignes 690-700) joint
seulement les arguments `+` par un blanc. Le cas `+nadgrids="dir with space"`
(`tests/cli_yaml/test_cs2cs_datumfile.yaml`, ligne 18) utilise la forme
guillemetée de PROJ et reste valable : la correction doit distinguer les deux
formes. Les mêmes CRS passés en mots séparés (mode par défaut) donnent le
résultat de la référence.

**6. NTF (Paris) vers RGF93, statut : le port renvoie `* * inf` (d) — ouvert, cause réattribuée**

Cas : `EPSG:4807 → EPSG:4171`, `EPSG:4807 → EPSG:2154`, et ses variantes
`--only-best` et `--3d`. Le lot C1 supposait une cause d'unité. La mesure
l'infirme : la ligne `* * inf` est la même avant et après la correction des
grades.

```sh
tools/compare-cs2cs.sh --no-build -k 'EPSG:4807|EPSG:2154'
printf '43.2999763919 3.0333132353\n' | target/proj-reference/build/bin/cs2cs -d 10 EPSG:4807 +to EPSG:2154
printf '43.2999763919 3.0333132353\n' | target/release/cs2cs -d 10 EPSG:4807 +to EPSG:2154
```

- Référence (`PROJ_DATA` du comparateur, avec `data/tests/ntf_r93.gsb`) :
  `880453.0554161093 5764316.2342027659`.
- Port : `* * inf`.

Sans la grille `data/tests/ntf_r93.gsb` (`PROJ_DATA` = la seule référence), le
port et la référence donnent les mêmes valeurs à 1e-10 près :
`880407.3954572410 5764330.9733427446` (4807 vers 2154),
`38.9698363107 2.7305003045` (2154 vers 4807), `38.9701211876 5.0666926700`
(4807 vers 4171). Ce sont les valeurs de `tests/cs2cs_grad_units.rs`.

Avec la grille, la référence ouvre `fr_ign_ntf_r93.tif` (absent), puis
`ntf_r93.gsb` : la table `grid_alternatives` de `proj.db` relie les deux
(`old_proj_grid_name`). Le port répond `* * inf` en mode par défaut, avec
`--only-best` et avec `--3d`. Il lit `nadgrids=fr_ign_ntf_r93.tif` sans erreur
(même valeur que la référence). La défaillance est donc dans la sélection ISO
(`PROJ_RS_TRACE_PATH=1` : `path=iso`) ou dans le pipeline de l'opération
retenue. Non vérifié. Prochaine étape : comparer l'opération retenue avec
`projinfo -s EPSG:4807 -t EPSG:2154 -o PROJ` et la trace `PROJ_DEBUG=3`. La
fiche de référence est `PORTING-cs2cs-grid-policy.md`.

**7. Époques sur un CRS statique (d)**

Cas : `EPSG:7789 → EPSG:4936 [--s_epoch 2015 --t_epoch 2010]`,
`EPSG:4936 → EPSG:7789 [--s_epoch 2019]`, `EPSG:9988 → EPSG:4936 [--s_epoch
2005 --t_epoch 2015]`. La référence refuse avec « Coordinate epoch should not
be provided for a static CRS » (code 3). Le port accepte.

```sh
tools/compare-cs2cs.sh --no-build -k 'EPSG:4936|EPSG:7789|--s_epoch 2019'
printf '4200985 168325 4780236\n' | target/proj-reference/build/bin/cs2cs -d 10 --s_epoch 2019 EPSG:4936 +to EPSG:7789
```

La règle de la référence ne s'applique pas à `EPSG:4978` : `--t_epoch 2020`
y est accepté des deux côtés. Les époques sur `EPSG:7930`, `EPSG:9067` et
`EPSG:4937` concordent. Cause non vérifiée (la base de données ou le contrôle
de l'époque).

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

- `tests/cli_yaml` n'a pas été lancé : aucun changement dans `src/`.
- Les messages d'erreur sur la sortie d'erreur (stderr) ne sont pas comparés.
  Ils diffèrent (par exemple `proj_create: crs not found` contre
  `no transformation`). Le comparateur ne classe que stdout et les codes retour.
- La cause de l'écart 6 est réattribuée à la grille (`ntf_r93.gsb`), pas à l'unité ; non corrigée.
  Les causes des écarts 3 et 4 sont mesurées (points 3 et 4), pas corrigées.
- `+init=epsg:4807` (hors matrice) : PROJ imprime en DMS, longitude d'abord, un
  point qui n'est pas celui du port (`+init=epsg:4326 +to +init=epsg:4807` sur
  `48.8582 2.2945` : PROJ `46d31'12.678"E 2d17'30.494"N`). Le port garde sa
  disposition d'avant le lot C1. Voie `+init` : même famille que les écarts 3 et 4.
- `projinfo` refuse la syntaxe `+init=` en mode non PROJ4 : les choix
  d'opération de ces paires ne sont pas lus par `projinfo`.
- Le candidat de `EPSG:4326 → EPSG:4936` diffère (référence 2, port 1), sans
  effet numérique sur les points testés.
