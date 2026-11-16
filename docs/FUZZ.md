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

### Résultats

Les plantages trouvés, leur cause et leur test sont dans la section « Plantages trouvés ».

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

### Écarts consignés

Sections remplies après les passages ci-dessous.
