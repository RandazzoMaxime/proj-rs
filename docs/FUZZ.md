# Fuzz : robustesse et différentiel aléatoire

Deux outils, deux questions.

| Question | Outil | Où |
|---|---|---|
| Une entrée absurde fait-elle paniquer, boucler ou épuiser la mémoire ? | `cargo-fuzz` (libFuzzer) | `fuzz/` |
| Sur des entrées tirées au hasard, le port donne-t-il la sortie de PROJ 9.9.0 ? | différentiel à graine | `tools/parity/fuzz_diff.py` |

## Robustesse : `fuzz/`

`fuzz/` est un paquet hors du workspace (`[workspace]` vide dans son `Cargo.toml`) : la
chaîne nightly et libFuzzer n'entrent pas dans le build normal. Les points d'entrée sont dans
`fuzz/src/lib.rs` (une fonction par genre d'entrée, octets en entrée) ; `fuzz/fuzz_targets/` ne
fait que les appeler.

| Cible | Entrée | Ce qu'elle exerce |
|---|---|---|
| `proj_string` | texte | `Operation::create`, `Proj::new`, `create_projection`, puis directe, inverse et aller-retour sur 12 points fixes (NaN, infinis, 1e300, sous-normaux) |
| `user_input` | texte | `create_from_user_input` (WKT, PROJJSON, chaîne PROJ, `AUTH:code`, URN), `create_object_from_user_input`, puis tous les exports (PROJJSON multi et mono-ligne, WKT2:2019, WKT2:2015 simplifié, WKT1 GDAL, WKT1 ESRI, chaîne PROJ) |
| `transform` | octets structurés | une paire de codes EPSG réels (`fuzz/epsg_codes.txt`), ou une projection de `PORTED` avec un suffixe de paramètres, ou deux textes séparés par `\0` ; coordonnées tirées des octets, valeurs pièges sur-représentées |
| `grid_reader` | 1 octet de type, puis le fichier | GeoTIFF, GTX, NTv1, NTv2, CTable2 lus comme grilles horizontale, verticale et générique (blocs, sous-grilles, valeurs) et utilisés par `hgridshift`, `vgridshift`, `deformation` |
| `model_files` | 1 octet de type, puis le JSON | fichier TIN shift (`tinshift`) et fichier maître de modèle de déformation (`defmodel`), analysés, évalués, et utilisés comme étape |

Les entrées de plus de 8 Ko de texte ou de 1 Mo de fichier sont ignorées. Les variables
`PROJ_DATA` (`data` et `data/tests`), `PROJ_NETWORK=OFF` sont posées par le harnais : aucune
exécution ne touche le réseau.

### Rejouer

```sh
rustup toolchain install nightly --profile minimal
cargo install cargo-fuzz --locked
cd fuzz
python3 seed.py                      # corpus d'amorce : tests/gie, data/tests, tests/parity/*
cargo +nightly fuzz build
./run.sh 1800                        # les cinq cibles en parallèle, 30 minutes chacune
./run.sh 600 proj_string grid_reader # certaines cibles seulement
```

`run.sh` règle `-timeout=10` (une exécution de plus de 10 s est une boucle), `-rss_limit_mb=2048`
et `-max_len` par cible. Les journaux sont dans `fuzz/logs/`, les plantages dans
`fuzz/artifacts/<cible>/`. Un plantage se rejoue et se minimise ainsi :

```sh
cargo +nightly fuzz run proj_string artifacts/proj_string/crash-<hash>
cargo +nightly fuzz tmin proj_string artifacts/proj_string/crash-<hash>
```

`fuzz/corpus/`, `fuzz/artifacts/`, `fuzz/logs/` et `fuzz/target/` ne sont pas versionnés. Un
plantage corrigé devient un test de non-régression dans `tests/fuzz_regressions.rs` (avec, au
besoin, un fichier de 500 octets dans `tests/fuzz_regressions/`).

Débit mesuré, sanitizer et couverture compris : 5 à 15 exécutions par seconde pour les cibles qui
ouvrent la base embarquée (`proj_string`, `user_input`, `transform`), 150 à 3 500 pour
`grid_reader` et `model_files`. Chaque `DatabaseContext::create()` copie et analyse la base
(10 ms sans sanitizer) ; la cible `user_input` n'en ouvre qu'une par exécution.

### Plantages trouvés (2026-11-16)

Cinq causes, une par commit, chacune avec son test dans `tests/fuzz_regressions.rs` ou en test
unitaire :

| Cible | Cause | Correction | Commit |
|---|---|---|---|
| `proj_string` | `Evaluator::forward` (defmodel) retire un tour de longitude à la fois : 1e300 ou l'infini ne changent jamais, boucle infinie | refuse une longitude de plus de 1e4 rad (PROJ la refuse à 10 rad dans `fwd_prepare`) | `5a944c8` |
| `grid_reader` | `chunk_bytes` alloue la taille d'un bloc TIFF déclarée par le fichier avant de la lire : 3,5 Ko réclament 3 Go | lecture par `Read::take`, comparaison de la longueur ensuite | `9a7a89f` |
| `grid_reader` | `tiff` 0.10.3 panique sur une balise `SampleFormat` vide (`sample_format[0]`) | les trois constructions de décodeur passent par `geotiff::contained` (`catch_unwind`) ; `tiff` 0.11.3 corrige mais exige Rust 1.85 (MSRV 1.83) | `c54ea0b` |
| `proj_string` | `strip_urn_prefix` coupe le texte à 12 octets : un caractère multi-octet à cheval sur l'octet 12 panique | `str::get` | `5866b89` |
| `grid_reader` | CTable2 : `largeur * hauteur * 8` déborde (2^31 - 1 chacune) | `checked_mul` | `c9ba504` |

Le harnais laisse passer les paniques de la crate `tiff` (hook dans `fuzz/src/lib.rs`) : libFuzzer
abandonne sur toute panique, même rattrapée.

Non corrigé, trouvé en route :

- **Recherche par nom lente.** Tout texte inconnu (`ZZZZZZZZZZZZ`, `+++++`) passe par la recherche
  de nom dans la base (`objects_from_name_matched`) : 2,7 s en debug, contre 27 ms pour PROJ
  9.9.0 (`cs2cs 'ZZZZZZZZZZZZ' +to EPSG:4326`). Ce n'est pas une boucle, mais elle fait tomber le
  débit des cibles `proj_string`, `user_input` et `transform` à 5 exécutions par seconde, et a
  fait déclarer un « timeout » à 10 s sous sanitizer. `run.sh` fixe donc `-timeout=60`.
- **Même motif de découpe que `strip_urn_prefix`**, non atteint par le fuzz mais à corriger de la
  même façon : `identify.rs:933`, `equiv.rs:412`, `wkt_import.rs:1000` (`texte[..prefixe.len()]`
  après un test de longueur en octets). Dans `src/iso19111/`, laissé à l'autre chantier.

### Durées et exécutions

Chaque cible a tourné au moins 1 900 s (31 min 40 s) sur le code final, en parallèle sur quatre
cœurs partagés, `-timeout=60 -rss_limit_mb=2048` :

| Cible | Durée | Exécutions | Résultat |
|---|---|---|---|
| `proj_string` | 1 900 s | 97 598 | aucun plantage |
| `user_input` | 1 900 s | 9 716 | aucun plantage |
| `transform` | 1 900 s | 3 774 | aucun plantage |
| `grid_reader` | 1 900 s | 129 599 | aucun plantage |
| `model_files` | 1 900 s | 2 976 116 | aucun plantage |

`user_input` et `model_files` ont tourné avant les correctifs CTable2 et lcc, qui ne les touchent
pas ; les trois autres ont été relancées après. Les passages interrompus par un plantage (cinq
minutes d'exploration, puis 31 minutes pour `proj_string`, `transform` et `grid_reader`) ne
comptent pas dans ce tableau.

## Différentiel aléatoire : `tools/parity/fuzz_diff.py`

Le tirage vient d'une graine. Il reprend les familles du corpus de parité à plus grande échelle
(paires de CRS de la `proj.db`, points dans l'emprise et dans le monde entier, options de
`cs2cs`, pipelines `cct`, toutes les projections de `proj`), puis trois familles aléatoires
(`tools/parity/families_random.py`) :

- `random-proj-fwd`, `random-proj-inv` : une projection, des paramètres tirés (ellipsoïde,
  `lon_0`, `lat_0`, `lat_1`, `lat_2`, `k_0`, `x_0`, `y_0`, `units`, `axis`, `over`) ;
- `random-cs2cs` : deux chaînes PROJ, géographique ou projetée, avec `+towgs84` ou datum, précision
  et sens tirés ;
- `random-cct` : pipelines de 2 à 5 étapes (`cart`, `helmert` à 3, 7 ou 15 paramètres,
  `molodensky`, `geoc`, `axisswap`, `unitconvert`, projections), directs ou `-I`.

La référence enregistre sa sortie, puis `cargo test --test parity parity_random_against_proj_9_9_0`
compare : même comparateur, mêmes seuils (0,5 mm, 5e-9 degré, 5e-9 an) et même rapport que
`tests/parity.rs` (voir `docs/PARITY.md`). Aucun registre d'écarts connus : tout écart échoue.

```sh
PROJ_REFERENCE_DIR=/chemin/proj-reference python3 tools/parity/fuzz_diff.py --seed 20261119
# --scale N (5 000 conversions par unité), --random N (invocations par famille aléatoire),
# --named-datums (datums nommés dans cs2cs), --no-compare (écrire le corpus sans comparer)
```

Le dossier `target/parity-random/<graine>/` reçoit `cases.jsonl`, `expected.jsonl`, `meta.json`
(options et empreinte sha1 du tirage), `report.tsv` (une ligne par écart, avec la commande de
reproduction) et `compare.log`. Même graine et mêmes options redonnent exactement le même corpus :
comparer le `sha1` de `meta.json`. Pour rejouer un corpus déjà écrit :

```sh
PARITY_RANDOM_DIR=$PWD/target/parity-random/20261119 PARITY_REPORT=/tmp/r.tsv \
  cargo test --test parity parity_random_against_proj_9_9_0 -- --ignored --nocapture
```

Le tirage évite les paramètres qu'une projection ne lit pas (liste de `proj -lP`) : PROJ les
ignore, le port les refuse (écart « paramètre non lu » ci-dessous), et le différentiel doit
chercher autre chose. `--named-datums` remet les datums nommés, qui exposent l'écart suivant.

### Passage de référence

Graine `20261119`, `--scale 3 --random 600 --points 8 --world 0.15` : 3 581 invocations,
30 775 conversions (`meta.json` : sha1 `53df5dbf9f95fe990c5c850649888402c047855e`).

| Avant correction | Conversions |
|---|---|
| Identiques (texte) | 29 453 |
| Dans le seuil | 1 305 |
| Erreur des deux côtés | 3 |
| Hors seuil | 12 |
| Statut | 2 |

Après les deux correctifs ci-dessous : 29 464 identiques, 1 306 dans le seuil, 3 erreurs des deux
côtés, **0 hors seuil**, **2 statut**, 0 forme.

Corrigés (un commit chacun, test dans `tests/fuzz_regressions.rs`) :

- `proj` appliquait `+axis` : PROJ le lit, le liste comme utilisé et ne l'applique pas (`08ee8e2`,
  graine 1, `random-proj-fwd-0002`).
- `lcc` : sans `+lat_1`, PROJ prend 0 et non `lat_0` ; avec `+lat_1` seul et sans `+lat_0`,
  l'origine est `lat_1` (`eed9b8f`, `random-cct-0313` et `0484` : 6 300 m et 1 585 km d'écart).

### Écarts consignés

Non corrigés, parce qu'ils touchent un choix de conception ou une architecture :

1. **Pipeline : paires `étape` / `+inv étape` retirées par PROJ.** `pj_optimize_pipeline` supprime
   deux étapes adjacentes inverses l'une de l'autre. `+proj=axisswap +order=2,1 +step +proj=cart
   +ellps=GRS67 +step +inv +proj=cart +ellps=GRS67` rend donc `lat lon h t` pour un point dont la
   « latitude » dépasse 90° ; le port exécute `cart` et refuse le point (`*`). Graine `20261119`,
   `random-cct-0193` (2 lignes, statut). Le port n'optimise pas le pipeline.
2. **Paramètre non lu.** PROJ ignore `+lat_1` sur `sinu` ; `Operation::create` le refuse
   (`UnknownParameter`, `UnknownKeys::Refuse`, `engine.rs`), et `docs/PORTING-sphere-ops.md` le dit.
   `cct +proj=pipeline +step +proj=sinu +ellps=sphere +lat_1=33.3 +k_0=0.99` : code 0 chez PROJ,
   code 1 ici. Seul `proj` (`create_projection_keeping_unknown`) suit PROJ. Le tirage aléatoire
   évite ces paramètres (liste de `proj -lP`).
3. **Datum nommé contre ellipsoïde avec `+towgs84`** (`--named-datums`, graine 7, `--scale 1
   --random 200`, 1 310 invocations, 9 522 conversions : 328 hors seuil et 143 statut, tous dans
   59 invocations `random-cs2cs`, aucun dans `proj` ni `cct`). Exemple :
   `cs2cs -d 9 +proj=longlat +datum=NAD83 +to +proj=longlat +ellps=aust_SA +towgs84=-105,-275,-140`
   donne `10.002413662 20.000724771` chez PROJ et `* * inf` ici ; avec `+datum=WGS84` ou un
   ellipsoïde nu à la place de `NAD83`, les deux concordent. Avec `+datum=carthage`, les valeurs
   diffèrent de 2e-4 à 5e-3 degré. Les pairs d'ellipsoïdes et de `+towgs84` seuls concordent
   sur les 30 775 conversions de la graine `20261119`.
4. **`proj -V` sur `imoll`** : PROJ écrit « Final Earth figure: sphere », le port l'ellipsoïde
   (`+proj=imoll +units=km +axis=neu`). Hors conversions.

Rejouer les écarts 1 et 2 : `python3 tools/parity/fuzz_diff.py --seed 20261119`. L'écart 3 :
`--seed 7 --scale 1 --random 200 --named-datums`.
