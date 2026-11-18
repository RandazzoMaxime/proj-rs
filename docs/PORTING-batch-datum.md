# Port — le chemin par lots des changements de datum (`cart`, `helmert`, `push`/`pop`)

Lot 7, première partie. Fichiers : `src/projections/cart_batch.rs`, `helmert_batch.rs`,
`plan.rs` (`BatchStack`, `stack_balanced`), `legacy.rs` (`forward_batch`, `inverse_batch`),
`src/simd/mod.rs` (`Lanes3`, `run3`), `src/simd/drive.rs` (`groups3`). Tests :
`cart_batch_tests.rs`, `helmert_batch_tests.rs`, `batch_check.rs` (aide), et
`tests/convert_array_batch.rs`.

## Constat

Un changement de datum est une suite `push`, `cart`, `helmert`, `cart` inverse, `pop`
(`EPSG:4326 → EPSG:2100`, `28992`, `2056`, tout ce qui porte un `towgs84`). `Pipeline::forward_batch`
envoyait **tout** pipeline qui garde une pile par le chemin point par point : un verrou de mutex à
chaque `push` et chaque `pop`, et la projection finale, même quand elle a un noyau vectoriel, ne
voyait pas la tranche. Le seul test du dépôt sur ce cas (`a_pipeline_that_keeps_a_stack…`) vérifiait
l'accord des valeurs, pas la vitesse.

## Ce qui change

1. **`push` et `pop` sur la tranche.** `Step::stack_op` dit qu'une étape est un `push` ou un `pop`
   (`PushPop`). `plan::run_batch` les exécute sur `BatchStack` : un `push` sauve la valeur de chaque
   point, le `pop` suivant rend à chaque point la sienne. Un point en échec garde sa place dans la
   pile et n'est pas modifié. Le chemin ne sert que si `plan::stack_balanced` est vrai pour le sens :
   toute étape qui lit la pile est un `push` ou un `pop`, sur chaque axe les `pop` ne dépassent pas
   les `push` qui les précèdent, et les deux comptes finissent égaux. Un pipeline déséquilibré garde
   la pile d'un point au suivant, comme PROJ (la pile est dans l'objet pipeline) : seul l'ordre
   point par point le reproduit, il reste donc la boucle d'avant.
2. **`cart`** (`CartStep`) : `geodetic_to_cartesian` et `cartesian_to_geodetic` en quatre voies,
   aller et retour. `fwd_prepare` (porte de latitude, `lon_0`, `pm=`, latitude géocentrique) et la
   remise des unités restent ceux du scalaire, point par point ; un point que `prepare_forward`
   refuse garde son erreur. Un groupe de quatre qu'une voie sort du domaine du noyau (valeur non
   finie, pôle `x_phi ≤ 0`, `norm = 0`, `cos φ < 1e-6`) passe par la fonction scalaire.
3. **`helmert`, `molobadekas`** : sans taux (`dx`, `drx`, `ds`…) ni `theta`, la matrice, l'échelle
   et les translations ne dépendent pas du point. Le scalaire les refait pourtant à chaque point
   (`rotation_matrix`, `observed_dt`). `Fixed` les calcule une fois à la création ; un noyau à quatre
   voies suit `spatial_kernel`. Avec taux ou en plan (quatre paramètres), la boucle scalaire, qui
   ne change pas.
4. **`Legacy`** (`+proj=… +towgs84=…`, `+nadgrids`, `+axis`) : ses sous-étapes (`cart`, `helmert`,
   `cart`, grille verticale, `axisswap`) passent par la tranche, dans l'ordre de `datum_forward`
   et de `datum_inverse`.
5. **`Lanes3` et `run3`** : le driver de `Lanes`/`run` pour trois entrées et trois sorties, qui
   réutilise `V4`, `math.rs` et `kernels.rs`. Pas de second mécanisme : même découpage par quatre,
   même repli (`point`) pour la queue et pour un groupe que `lanes` refuse. Hors AVX2, `run3` boucle
   sur `point`.

## Mesure

Machine : Linux x86-64, Intel Xeon 2,1 GHz, AVX2 et FMA, 4 cœurs partagés. `--release`,
`taskset -c 2`. 1 000 000 points tirés d'une graine fixe dans l'aire d'usage. Une série est la
médiane de 5 passes après un échauffement, répétée jusqu'à deux séries consécutives à moins de 3 %
(10 au plus) ; la valeur d'une série est la médiane des séries. Deux passages complets, base et
version alternées, la table donne la moyenne des deux. Base : `origin/main` (`fe17893`), même
exemple de mesure. `convert_array` sur des couples `(x, y)`.

| cas | avant ns/pt | après ns/pt | écart |
|---|---:|---:|---:|
| `cart` aller (pipeline `unitconvert`, `cart`) | 28,2 | 13,9 | −50,8 % |
| `cart` retour | 34,8 | 15,6 | −55,1 % |
| `helmert` 7 paramètres | 13,2 | 6,1 | −53,8 % |
| `helmert` 3 paramètres | 12,8 | 5,9 | −53,3 % |
| datum 7 paramètres (`push`, `cart`, `helmert`, `cart`, `pop`) | 151,6 | 32,2 | −78,7 % |
| datum 3 paramètres | 154,5 | 31,1 | −79,8 % |
| `utm +ellps=GRS80 +towgs84=3` | 202,7 | 52,8 | −73,9 % |
| `utm +ellps=intl +towgs84=7` | 224,3 | 52,5 | −76,6 % |
| `EPSG:4326 → EPSG:2100` (3 paramètres, `tmerc`) | 241,5 | 55,9 | −76,9 % |
| `EPSG:4326 → EPSG:28992` (7 paramètres, `sterea`)¹ | 267,9 | 66,4 | −75,2 % |
| `EPSG:4326 → EPSG:2056` (3 paramètres, `somerc`)¹ | 300,5 | 85,7 | −71,5 % |

¹ Ces deux lignes cumulent aussi le noyau de `docs/PORTING-batch-oblique.md`.

Part de chaque pièce, mesurée en les ajoutant l'une après l'autre (une série par ligne,
indicative) sur le datum à 7 paramètres : `push`/`pop` seuls 148,0 → 73,1 ns (−51 %) ; puis `cart`
73,1 → 35,9 (−51 %) ; puis `helmert` 35,9 → 28,6 (−20 %). Chaque pièce dépasse les 10 % demandés ;
aucune n'a été écartée. `push`/`pop` seul sert aussi un processeur sans AVX2 : le reste du gain y
dépend des noyaux, qui n'y tournent pas (voir « Limites »).

Contrôles, code inchangé : `EPSG:4326 → EPSG:2154` 31,0 → 29,4 ns (−5 %, bruit), et les couples à
plusieurs candidats (`EPSG:4326 → EPSG:3035`, `EPSG:4326 → EPSG:2169`) restent à ±3 %.

## Parité

La référence est le chemin scalaire de la crate, point par point (`Operation::forward`,
`Operation::inverse`). Écart maximal sur 200 000 points de l'aire d'usage, `convert_array` contre
`convert` :

| cas | écart absolu max | écart relatif max |
|---|---:|---:|
| `cart` aller | 1,9e-9 m | 6e-16 |
| `cart` retour (sortie en degrés) | 1,8e-15 ° | 2,5e-16 |
| `helmert` 7 paramètres | 9,3e-10 m | 2,2e-16 |
| datum 7 paramètres, géographique à géographique | 3,6e-14 ° | 7,2e-16 |
| `utm +towgs84` 3 et 7 paramètres | 2,8e-9 m et 4,7e-9 m | 2e-15 |
| `EPSG:4326 → EPSG:2100` | 2,8e-9 m | 1,4e-15 |

Les trois sont sous le seuil du dépôt (1e-6 m ou 1e-13 relatif, 1e-11°). L'écart est celui des
fonctions de `math.rs` et des FMA ; sans taux ni rotation, `helmert` est même identique au bit
(3 paramètres : 0 écart).

Tests (tolérance de `tests/common/tolerance.rs`, mètres 1e-6 ou 1e-13 relatif, radians 1,75e-13) :

- `cart_batch_tests.rs` : sept définitions (GRS80, `lon_0` et `pm=`, sphère, `+units=km`,
  `+over`, ellipsoïde aplati, `+geoc`), 3 000 points tirés et les pôles, `±π`, les latitudes
  refusées, `NaN`, les infinis, une hauteur non finie ; au retour, la surface, l'axe, le centre
  de la Terre, l'équateur, des valeurs à 1e12 m. Chaque point doit rendre la même valeur ou la
  même erreur que le scalaire. Une hauteur fausse de 2e-6 m écrite dans le noyau fait échouer le
  test (vérifié, puis annulé).
- `helmert_batch_tests.rs` : position vector et coordinate frame, 3 et 7 paramètres, `+exact`,
  échelle seule, Molodensky-Badekas, unités ; avec taux ou `theta`, la sortie doit être au bit
  celle du scalaire. Une translation fausse de 1e-5 fait échouer le test (vérifié, puis annulé).
- `tests/convert_array_batch.rs` : cinq cibles à datum (`2100`, `28992`, `2056`, `2169`, `5514`),
  aller et retour, 9 001 points ; `+towgs84` sur une projection, aller et retour ; des `push`/`pop`
  imbriqués, un `pop` sans `push` (pipeline déséquilibré, donc la boucle d'avant), un point refusé
  entre un `push` et son `pop`.
- Chemin portable : les mêmes tests avec `Avx2::detect` forcé à `None` (modification locale, non
  commitée) : 33 tests de la bibliothèque et 15 de `convert_array_batch` passent.

## Limites

- Les couples à plusieurs candidats (`Selection::Several`, `EPSG:4326 → EPSG:3035`, `4267 → 4326`)
  restent point par point : `proj_trans` choisit par point (`docs/PORTING-batch-api.md`).
- Un `helmert` à taux (`dx`, `drx`, `ds`) garde la boucle scalaire : sa matrice dépend de l'époque
  de chaque point. Une version qui la refait seulement quand l'époque change est possible, non
  mesurée.
- Hors AVX2 (aarch64, x86-64 sans FMA), `cart` et `helmert` rejouent la fonction scalaire point
  par point : seul le gain de la pile reste. Aucun noyau NEON. Le build aarch64 n'est pas
  vérifié ici (pas de cible installée) : les `cfg(target_arch = "x86_64")` suivent ceux de
  `tmerc/batch.rs`.
