# Suite de parité contre PROJ 9.9.0

But : garder, dans le dépôt, un filet de sécurité de plusieurs milliers de
conversions comparées à PROJ 9.9.0. La phase 2 réarchitecte le calcul pour la
performance. Elle doit rester précise : la suite dit tout écart de sortie.

La suite ne dépend plus d'un PROJ installé. Le corpus et la sortie de référence
sont dans le dépôt.

## Fichiers

| Fichier | Rôle |
|---|---|
| `tools/parity/generate.py` | Tire les entrées (graine fixe) et écrit `tests/parity/cases.jsonl`. Assemble les modules `common.py`, `catalog.py`, `reference.py`, `generator.py`, `families_cs2cs.py`, `families_proj.py`, `families_grids.py`. |
| `tools/parity/record.sh`, `record.py` | Lance PROJ 9.9.0 sur chaque invocation et écrit `tests/parity/expected.jsonl`. |
| `tests/parity/cases.jsonl` | Une invocation par ligne : outil, arguments, lignes d'entrée, genre de chaque champ. |
| `tests/parity/expected.jsonl` | Code de sortie et sortie standard de la référence, ligne à ligne. |
| `tests/parity/known.tsv` | Écarts connus : clé `id:ligne`, cause courte, renvoi à une fiche. |
| `tests/parity.rs` | Relance nos binaires `cs2cs`, `proj`, `cct` et compare. |
| `tools/parity/fetch-grids.sh` | Recrée le dossier des grilles du corpus avec grilles (`tests/parity/grids/grids.txt`). |
| `tests/parity/grids/` | Corpus avec grilles : `cases.jsonl`, `expected.jsonl`, `known.tsv`, `ops.tsv`, `grids.txt`. |

## Commandes

Depuis la racine du dépôt, après `cargo build --bins` :

```sh
# Régénérer le corpus (pas besoin de PROJ pour lire le résultat ; la référence sert aux entrées)
PROJ_REFERENCE_DIR=/chemin/proj-reference python3 tools/parity/generate.py

# Réenregistrer la référence
PROJ_REFERENCE_DIR=/chemin/proj-reference tools/parity/record.sh

# Lancer la suite, avec le résumé et le rapport détaillé
PARITY_REPORT=/tmp/parity.tsv cargo test --test parity -- --nocapture
```

`PROJ_REFERENCE_DIR` désigne un répertoire contenant `build/bin` et `build/data`
de PROJ 9.9.0. Sans cette variable, c'est `target/proj-reference`.

Le rapport `PARITY_REPORT` contient une ligne par écart : clé, famille, classe,
connu ou non, détail, et la commande de reproduction.

La suite tourne en environ deux minutes en debug sur quatre cœurs.

## Ce qui est comparé

Chaque ligne de sortie est comparée à la ligne attendue, champ par champ.

| Classe | Règle | Échec ? |
|---|---|---|
| Identique | Texte identique. | Non (compté) |
| Dans le seuil | Texte différent, valeurs dans le seuil. | Non (compté) |
| Erreur des deux côtés | `*`, `inf` ou `nan` des deux côtés. | Non (compté) |
| Statut | Erreur d'un seul côté, ou code de sortie nul d'un seul côté. | Oui |
| Forme | Nombre de champs ou de lignes différent. | Oui |
| Valeur | Champ numérique hors seuil. | Oui |

Seuils, par genre de champ (`kinds` dans `cases.jsonl`) :

- mètres (`m`) : 5e-4 m, soit 0,5 mm. C'est le défaut de `gie`.
- degrés (`deg`) : 5e-9 degré, soit environ 0,5 mm à l'équateur.
- temps (`t`) : 5e-9, en années décimales.
- facteurs d'échelle de `proj -S` (`fac`) : 1e-5 relatif. `proj -S` imprime
  avec `%g` (six chiffres). Un arrondi du sixième chiffre donne un écart relatif
  d'environ 7e-6. Le seuil du plan (1e-6) échouerait donc sur de l'arrondi.
  Près de zéro, le seuil devient absolu (1e-5) : la distorsion angulaire vaut
  0 d'un côté et 1e-6 degré de l'autre quand le bruit des différences finies de
  `pj_deriv` tombe autrement (`krovak`, `omerc`).
- texte (`text`) : la ligne entière, à l'identique. Sert aux listes de candidats
  de `projinfo --summary` (famille `projinfo-candidates` du gros corpus).
- DMS (`dms`) : le champ entier, à l'identique. Sert aux sorties de `geod` en
  degrés, minutes, secondes ; un champ DMS qui diffère est un écart de valeur,
  même sur une ligne qui porte aussi une erreur (`nan`).

`cs2cs` écrit toujours trois valeurs (z = 0 sans hauteur), quatre avec le temps.
Le corpus en tient compte.

Une entrée vide (`input` = liste vide) n'envoie aucun octet, comme
`record.py` : la référence ne lit alors rien. Une ligne vide est une entrée
à part entière (`geod-blank`).

## Registre des écarts

`known.tsv` liste les écarts connus. Règles :

- un écart listé ne fait pas échouer la suite ;
- un écart nouveau fait échouer la suite ;
- un écart listé qui a disparu fait échouer la suite : retirer sa ligne.

`PARITY_UPDATE_KNOWN=1 cargo test --test parity` réécrit le registre avec les
écarts observés. À n'utiliser qu'en connaissance de cause : les causes non
renseignées s'écrivent « à classer ».

## Seuil d'alerte

Si plus de 5 % des conversions sont hors seuil, s'arrêter et rapporter avant de
figer un nouveau corpus. Le premier passage est à 4,4 %.

## Ajouter une famille

1. Ajouter une fonction `fam_*` dans le module de sa famille (`families_cs2cs.py`, `families_proj.py`, `families_grids.py`) et l'appeler dans `build` (`generate.py`).
2. Donner à chaque invocation le genre de chaque champ de sortie (`kinds`).
3. Régénérer `cases.jsonl`, réenregistrer `expected.jsonl`, relancer la suite.
4. Classer chaque nouvel écart : grille absente d'un côté, bug du port, écart
   volontaire documenté dans `docs/PORTING-*.md`, ou bruit d'arrondi.

Le générateur écarte les paires qui exigent une grille absente (`projinfo` :
« grid missing ») et les invocations sans résultat exploitable.

## Premier passage

Corpus de graine `20261115` : 710 invocations, 4236 conversions.

| Classe | Conversions |
|---|---|
| Identique (texte) | 3974 |
| Dans le seuil | 68 |
| Erreur des deux côtés | 1 |
| Hors seuil | 187 (4,4 %) |
| Statut (code de sortie) | 1 invocation |
| Forme (lignes absentes) | 6 |

Les 194 clés du registre (187 valeurs, 6 formes, 1 statut) portent chacune une
cause. Le port n'est pas corrigé dans ce lot : c'est la phase de correction.

## État actuel

Après les lots de correction (P1 à P4, voir `docs/PORTING-cs2cs-pivot-candidates.md`,
`docs/PORTING-mod-ster.md`, `docs/PORTING-cli-rest.md`) : 4171 identiques (texte),
64 dans le seuil, 1 erreur des deux côtés, 0 hors seuil, 0 statut, 0 forme. Le registre
`known.tsv` ne contient plus aucun écart. Le lot geod (`docs/PORTING-cli-geod.md`) a
mis le signe des NaN de `%f` : `proj-fwd-0063` (`-nan`) devient identique, soit
4172 identiques et 0 erreur des deux côtés.

Les 64 « dans le seuil » sont des derniers chiffres : le plus grand écart est sous 1e-9 m,
six ordres de grandeur sous le seuil de 5e-4 m. Ils ne font pas échouer le test.

## Gros corpus (`tests/parity/wide`)

Graine `20261116`, échelle 6, 12 points par invocation, 15 % des points tirés dans le
monde entier (hors emprise du CRS compris), 1000 paires `projinfo --summary` :

```sh
python3 tools/parity/generate.py --seed 20261116 --scale 6 --points 12 --world 0.15 \
  --projinfo 1000 --out tests/parity/wide/cases.jsonl
tools/parity/record.sh --cases tests/parity/wide/cases.jsonl --out tests/parity/wide/expected.jsonl
cargo test --test parity -- --ignored --nocapture      # une dizaine de minutes en debug
```

4274 invocations : 48 551 conversions (`cs2cs`, `proj`, `cct`) et 1000 listes de candidats,
soit 50 928 lignes comparées. Le test `parity_wide_against_proj_9_9_0` est ignoré par défaut
(il apparaît « 1 ignored », jamais « passed » sans avoir tourné).

Premier passage : 355 écarts. Corrigés depuis : la chaîne composé → géocentrique sans emprise
ni précision, l'emprise des ballparks, l'inverse de Cassini, `proj -S` sur `s2`, `alsk`,
`gs50`. Listes `projinfo` : 3 fermées dans le lot Q2 (identifiant des candidats horizontaux seuls,
`%.15g` de la précision sommée), 35 restent ouvertes. État actuel : 50 223 identiques, 560 dans le
seuil, aucun écart de valeur sur les conversions. Le registre `tests/parity/wide/known.tsv` garde
131 lignes, toutes de 35 listes `projinfo` : ensemble de candidats différent entre CRS composés,
verticaux et géocentriques (111), texte d'un candidat (14), emprise « World » contre « World. » (6).
L'emprise EPSG:9602 ferme cinq de ces listes mais en rouvre une autre (`projinfo-candidates-0291`) :
annulée, voir `docs/PORTING-concat-area-of-use.md` item 4. Aucune ne change les coordonnées de
`cs2cs` sur le corpus.

Repli de `proj_trans` reproduit tel quel : pour un point hors de toutes les emprises, PROJ prend
le premier candidat sans grille, sans nouvel essai, même si sa chaîne PROJ n'a pas pu être
construite. `EPSG:10644 → EPSG:11069` : le premier candidat convertit deux fois les degrés en
radians (bug de PROJ, « Mismatched units between step 1 and 2 », reproduit à l'identique par le
port), donc `* * inf` hors de Saba (`tests/cs2cs_fallback_broken_candidate.rs`). Candidat à une
correction en phase 2.

## Corpus avec grilles (`tests/parity/grids`)

Le petit et le gros corpus n'ont aucune grille : la référence `target/proj-reference`
est compilée sans TIFF, et le générateur écarte les paires qui demandent une grille
absente. Ce corpus utilise `target/proj-reference-net` (compilée avec TIFF et curl) et
les grilles du CDN `https://cdn.proj.org`.

Choix des opérations : 150 opérations de grille EPSG non dépréciées (`hgridshift`,
`gridshift`, `vgridshift`, `geoid_like`, `geocentricoffset`, `velocity_grid`), tirées
avec la graine `20261117` parmi les fichiers de 30 Mo au plus, équilibrées par méthode et
par pays. Elles tiennent dans `tests/parity/grids/ops.tsv`. Elles touchent 151 fichiers,
287 Mo : `tests/parity/grids/grids.txt`.

```sh
RN=/chemin/proj-reference-net          # PROJ 9.9.0 avec TIFF et curl
# 1. Tirer les opérations (réseau : tailles CDN). À refaire seulement si le choix change.
PROJ_REFERENCE_DIR=$RN python3 tools/parity/generate.py --grid-select --seed 20261117 --grid-count 150
# 2. Télécharger les grilles (287 Mo) dans target/parity-grids
tools/parity/fetch-grids.sh target/parity-grids
# 3. Générer le corpus, puis enregistrer la référence
PROJ_REFERENCE_DIR=$RN python3 tools/parity/generate.py --grids-dir target/parity-grids \
  --points 28 --seed 20261117 --out tests/parity/grids/cases.jsonl
PROJ_REFERENCE_DIR=$RN tools/parity/record.sh --cases tests/parity/grids/cases.jsonl \
  --out tests/parity/grids/expected.jsonl --grids-dir target/parity-grids
# 4. Lancer (ignoré par défaut ; sans PARITY_GRIDS_DIR le test échoue)
PARITY_GRIDS_DIR=$PWD/target/parity-grids cargo test --test parity parity_grids -- --ignored --nocapture
```

Pour chaque opération, le générateur tire quatre couples de CRS au plus. Un couple
ne reste que si la première opération de `projinfo` sans grille manquante utilise une
grille présente dans le dossier. Par paire, trois invocations `cs2cs` : points dans
l'emprise de l'opération (`in`), points du monde entier, donc souvent hors grille (`out`),
et sens inverse `-I` (`inv`). Les pipelines directs `cct` (une étape `hgridshift`,
`gridshift` ou `vgridshift`) complètent le corpus.

Mesure : 460 invocations, 10 276 conversions (`in` 3 836, `out` 1 918, `inv` 3 836,
`cct` 686), 137 paires. Limites : les points « dans la grille » sont tirés dans le
rectangle de l'opération, pas dans la couverture exacte de la grille ; `velocity_grid`
(2 opérations) ne donne aucune paire ; `us_nga_egm08_25` (80 Mo) est écarté.

Premier passage (graine `20261117`, 28 points) :

| Classe | Conversions |
|---|---|
| Identique (texte) | 10 275 |
| Dans le seuil | 0 |
| Erreur des deux côtés | 0 |
| Hors seuil | 1 (0,01 %) |
| Statut (code de sortie) | 0 |
| Forme (lignes absentes) | 0 |

Le seul écart, `cs2cs-grid-out-geoid_like-0024:8` (`EPSG:6364 → EPSG:8801`, point
33,918 N 99,467 W, dans le CONUS), donnait 3797,540472221235 au lieu de 3797,495905946656
(4,5 cm). Cause : la référence lit `build/data/proj.ini` (`only_best_default = off`), donc
PROJ écarte les candidats dont une grille manque. Le harnais ne charge pas ce fichier : PROJ
prend alors sa politique par défaut (`Warn`), garde le candidat NAD83(FBN) à géoïde ballpark
et donne la valeur du port. Le harnais passe `PROJ_ONLY_BEST_DEFAULT=NO`, le même réglage.
Avec les deux réglages, le port et PROJ concordent (`tests/cs2cs_grid_geoid_tie.rs`).
Le registre `tests/parity/grids/known.tsv` ne contient plus d'écart.

Seuil d'alerte : 0,01 % hors seuil, sous les 5 %.

## Suite geod (`tests/parity/geod`)

`geod` et `invgeod` ont leur corpus à part : le petit corpus reste figé. Test
`parity_geod_against_proj_9_9_0` (non ignoré, quelques secondes).

```sh
python3 tools/parity/generate.py --geod 400 --out tests/parity/geod/cases.jsonl
PROJ_REFERENCE_DIR=target/proj-reference tools/parity/record.sh \
  --cases tests/parity/geod/cases.jsonl --out tests/parity/geod/expected.jsonl
cargo test --test parity parity_geod_against_proj_9_9_0 -- --nocapture
```

Familles (`fam_geod`, graine par défaut) :

| Famille | Outil | Entrée | Sortie comparée |
|---|---|---|---|
| `geod-inv` | `geod -I` | `lat1 lon1 lat2 lon2` | azimuts et distance |
| `geod-fwd` | `geod` | `lat1 lon1 azimut distance` | point d'arrivée et azimut retour |
| `invgeod-inv` | `invgeod` | comme `geod-inv` | comme `geod-inv` |
| `geod-blank` | `geod -I` | lignes vides ou blanches mêlées | comme `geod-inv` |
| `geod-line` | `geod` | `+lat_1 +lon_1 …` puis `+n_S`, `+del_S` ou `+n_A` | une ligne par point |
| `geod-err` | `geod`, `invgeod` | options ou entrées fausses | code de sortie, sortie |

Les points couvrent antipodes, quasi-antipodes, pôles, équateur, antiméridien,
distances nulles, très longues distances. Les invocations tirent un ellipsoïde
(WGS84, GRS80, clrk66, intl, bessel, sphère, `+R=`, `+a`/`+rf`, `+a`/`+b`), une
unité `+sunits` (m, km, us-ft, mi) et un format (DMS par défaut, `-a`, `-p`,
`-f`, `-F`, `-w`, `-W`). Genres : `dms` (texte exact), `deg` (5e-9 degré) et
`m` (5e-4 m).

Corpus généré le 2026-11-15 : 1511 invocations, 6514 entrées, 8290 conversions
(entrées et lignes de sortie du mode ligne). Les références : `geod`, `invgeod`
de PROJ 9.9.0. La fiche de port est `docs/PORTING-cli-geod.md`.

Premier passage : 350 écarts, tous dans `known.tsv` avec leur cause (348 lignes
vides qui perdaient leur saut de ligne, 2 NaN en DMS). Corrigés depuis : 8290
conversions identiques, aucun écart connu, aucun écart hors seuil.

## Exports d'objets (`tests/parity/export`)

Famille `projinfo-export` (`tools/parity/families_export.py`, option `--export N`) : CRS
de tous types, opérations, datums et ellipsoïdes EPSG, tirés dans la `proj.db` de la
référence. Projections et conversions sont stratifiées par méthode. Chaque objet sort en
PROJ, WKT2:2019, WKT2:2015, WKT1_GDAL, WKT1_ESRI et PROJJSON (`projinfo CODE -o FORMAT -q`),
et un objet sur cinq aussi en `--single-line`. Comparaison ligne à ligne, genre `text`.

```sh
python3 tools/parity/generate.py --seed 20261118 --export 1500 --out tests/parity/export/cases.jsonl
tools/parity/record.sh --cases tests/parity/export/cases.jsonl --out tests/parity/export/expected.jsonl
cargo test --test parity parity_export -- --ignored --nocapture     # environ 9 min en debug
```

8935 invocations, 1441 objets : 5897 CRS EPSG, 2232 opérations (`-k operation`), 434 datums,
310 ellipsoïdes, 62 définitions PROJ avec `+towgs84`. Aucune invocation ne sort en erreur
dans la référence.

Premier passage, non corrigé : 326 998 lignes, 225 541 identiques, 2 838 hors seuil de texte,
98 619 « forme » (lignes absentes ou en trop) ; soit 31 % de lignes différentes, au-delà du
seuil de 20 % : les causes sont à classer avant toute correction. Les cinq principales :

| Cause | Lignes |
|---|---|
| PROJJSON non écrit pour `-k operation`, `-k datum`, `-k ellipsoid` (« cannot write PROJJSON ») | 82 757 |
| `--single-line` ignoré pour les WKT (sortie sur plusieurs lignes) | 11 446 |
| Définitions PROJ (`+towgs84`) : PROJJSON, WKT en `BOUNDCRS` au lieu de `CONVERSION` | 3 915 |
| `-k operation` en WKT et PROJ : texte des objets, WKT1 vide dans la référence | 3 027 |
| CRS EPSG : texte des WKT | 312 |

Le test `parity_export_against_proj_9_9_0` est ignoré par défaut : environ 9 minutes en
debug, au-delà du seuil de 30 secondes. Pas encore de `known.tsv` : les écarts ne sont pas
classés.
