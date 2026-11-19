# Construction d'un `Transformer` : les candidats se construisent à leur premier usage

Ce lot ne change aucun résultat. Il change le moment où les grilles des candidats sont lues : à la
construction du `Transformer`, pour tous les candidats, avant ce lot ; au premier point qui utilise
un candidat, après. Le port diffère de PROJ sur ce point, voir plus bas.

## Mesure

Profil dev (opt-level 0, sans réglage du profil), 2 cœurs, corpus de grilles de
`tools/parity/fetch-grids.sh` (151 fichiers, 287 Mo). Mesures du chronomètre autour de
`Transformer::builder(...).build()`, puis du premier point, sur le même binaire de test.

| Paire (corpus avec grilles) | Construction avant | Construction après | Premier point |
|---|---:|---:|---|
| `EPSG:6364` → `EPSG:8801`, défaut (warn) | 11,8 s | 0,55 s | 3,4 s après ; 0,4 ms avant |
| `EPSG:6364` → `EPSG:8801`, `ONLY_BEST=NO` | 5,9 s | 0,36 s | 1,9 s après |
| `EPSG:4979` → `EPSG:4326+5773` | 290 ms | 59 ms | 0,2 s après ; 53 µs avant |
| `EPSG:4979` → `EPSG:3855` | 56 ms | 55 ms | inchangé |
| `EPSG:4326+3855` → `EPSG:4979` | 104 ms | 104 ms | inchangé |

Pic de mémoire (RSS, `ru_maxrss`) pour `EPSG:6364` → `EPSG:8801` : construction 140 Mo avant,
22 Mo après ; construction puis un point : 140 Mo avant, 72 Mo après.

Le premier point de `EPSG:6364` → `EPSG:8801` est plus lent qu'avant (3,4 s contre 0,4 ms) : avant, le
point ne payait rien parce que la construction avait déjà lu les grilles de tous les candidats. La
somme construction + premier point passe de 11,8 s à 4,0 s, et la mémoire baisse.

Le test `tests/transformer_build_speed.rs` (corpus avec grilles, `#[ignore]`) échoue sur le code
d'avant (12,5 s, limite 3 s) et passe sur celui-ci (0,55 s et 0,36 s).

## Où le temps partait

Mesure avec des chronomètres temporaires dans `select_alternatives_for_crs` et `Preparer::prepare`
(non livrés), puis profil `callgrind` d'une seule ouverture de grille :

1. `Preparer::prepare` créait le pipeline des 34 candidats (`Operation::create`). Chaque création
   lit ses grilles en entier, comme PROJ à la création. Pour cette paire, six créations lisent
   `us_noaa_g2018u0.tif` (quatre fois) et `us_noaa_geoid03_conus.tif` (deux fois), 1,8 à 1,9 s
   chacune en dev : environ 11 s des 11,8 s de la construction. Un candidat qu'aucun point
   n'atteint n'a jamais besoin de sa grille.
2. Le décodage d'une grille flottante de 16 Mo prend 1,9 s en dev. Dans ce décodage, 80 % sont dans
   le crate `tiff` (inflate, prédicteur flottant), compilé en opt-level 0 par le profil dev : hors
   de portée sans changer le profil, et le lot ne le change pas. Notre part, `samples::place`, copiait
   un échantillon à la fois avec contrôles d'indices. Notre code (`place` et les boucles du
   décodage) fait environ 20 % du décodage ; `place` copie maintenant une ligne de pixels par
   tranche, avec les mêmes indices et les mêmes erreurs.
3. Les grilles lues au premier point sont celles des candidats utilisés : une par fichier, aucun
   fichier relu (compteur d'ouvertures temporaire pendant la construction et le premier point).

## Ce qui change

- `Item.operation` (`src/iso19111/alternatives.rs`) est un `OnceLock<Option<Operation>>`, rempli par
  `Item::pipeline` au premier usage. `Alternatives` garde le contexte de grilles de la sélection
  (`Option<Box<GridContext>>`, boîte pour ne pas agrandir `Selection::Several` au-delà de la
  comparaison de `large_enum_variant`) : le candidat créé plus tard lit les mêmes grilles, sous le
  même contexte, que s'il avait été créé à la sélection.
- `Preparer::prepare` (`src/iso19111/select.rs`) ne crée plus le pipeline. La sélection capture le
  contexte de grilles une fois (`active_or_environment`, déjà appelé pour `network_enabled`).
- La façade (`src/proj_iso.rs`) garde son comportement : ses candidats sont construits tout de suite,
  sous le contexte actif, comme avant (`pipeline(None)`), et `Proj::last_used_operation` lit
  `item.operation` déjà rempli. Pas de lecture supplémentaire de `proj.ini`.
- `samples::place` (`src/projections/gridshift/geotiff/samples.rs`) copie une ligne de pixels par
  tranche (`copy_from_slice` ou un `step_by` pour les plans séparés). Les erreurs sont les mêmes : un
  indice hors de la tranche ou de la destination, une tranche de zéro pixel ne fait rien. Trois
  tests unitaires à valeurs calculées à la main (`place_*`).

## Résultats

- Résultats bit à bit identiques avant et après, sur `EPSG:6364` → `EPSG:8801` (quatre points de
  l'intérieur du Texas), `EPSG:4979` → `EPSG:3855`, `EPSG:4979` → `EPSG:4326+5773` et
  `EPSG:4326+3855` → `EPSG:4979` (`OUT` du sondage avant/après).
- `cargo test --test parity parity_grids_against_proj_9_9_0 -- --ignored` : 460 invocations,
  10 276 conversions, 0 hors seuil, comme avant. `cs2cs_grid_geoid_tie` passe.
- Benchmark `proj-rs-bench` en release, avant/après, trois passes puis dix passes entrelacées :
  médiane des 214 cas à 0,0 % ; 213 cas sur 214 à moins de 2 % sur le minimum.
  `lambert93_batch` est à +3,9 % en médiane (26,8 contre 27,8 ns). Son code n'a pas changé : les
  instructions de `Lcc::forward_batch` sont identiques dans les deux binaires (même taille, seuls
  bougent les déplacements relatifs et l'adresse de la fonction, décalée de 160 octets). L'écart vient
  donc de l'alignement de la fonction dans le binaire, pas de ce lot ; il n'est pas corrigé ici.
- Les cas de la suite (`cargo test --all-features`) passent : bibliothèque, 308 cibles d'intégration,
  81 tests de documentation. `cargo +1.100.0 clippy --all-targets --all-features -- -D warnings`,
  `cargo fmt --check`, `cargo +1.83.0 check -p proj-rs --lib --all-features`.

## Écart vers PROJ

PROJ crée les candidats d'une paire à sa création (`pj_create_prepared_operations`, `pj_obj_create`).
Ce port les crée au premier point qui les utilise. Les résultats sont les mêmes parce que la création
est déterministe et lit les mêmes fichiers sous le même contexte. Deux conséquences :

- Un fichier de grille supprimé ou modifié entre la construction et le premier point est lu dans son
  état du premier point, comme PROJ le ferait pour une grille différée au premier point.
- Un candidat dont la création échoue (`Operation::create`) est tenté au premier point qui le
  choisit, et non à la construction ; il est ensuite écarté comme avant. Les candidats non
  instanciables (grille absente) restent décidés à la construction, comme avant.

## Non traité

- Le temps restant de la construction de `EPSG:6364` → `EPSG:8801` (0,4 s) est dans
  `create_operations` (recherche des candidats, requêtes SQLite) : mesuré, pas touché.
- Le décodage d'une grille de 16 Mo reste à 1,6 à 1,9 s en dev, pour la même raison que plus haut
  (le crate `tiff` et `miniz_oxide` sont compilés en opt-level 0).
- Le candidat retenu d'une route `Single` (un seul candidat après la préparation) n'est plus créé
  deux fois : avant ce lot, il l'était dans `Preparer::prepare`, puis à nouveau dans
  `Transformer::select`. Ce n'est pas la paire mesurée ici, qui a plusieurs candidats.

## Commandes

```sh
# Avant/après de la construction (dev) : le test échoue sur le code d'avant
PARITY_GRIDS_DIR=target/parity-grids cargo test --test transformer_build_speed -- --ignored
# Parité sur le corpus avec grilles, et le test de l'égalité des candidats géoïde
PARITY_GRIDS_DIR=target/parity-grids cargo test --test parity parity_grids_against_proj_9_9_0 -- --ignored
PARITY_GRIDS_DIR=target/parity-grids cargo test --test cs2cs_grid_geoid_tie -- --ignored
# Benchmark release, avant/après
cargo build --release --bin proj-rs-bench && ./target/release/proj-rs-bench
```
