# Le dos GPU (`feature = "gpu"`, `Proj::convert_array_gpu`)

Ce lot ne porte pas de code PROJ : il ajoute un second chemin de calcul pour les tranches, sur le GPU
(`wgpu` : Vulkan, Metal, DX12). PROJ n'a pas d'équivalent. Les valeurs de référence des tests sont
celles de PROJ 9.9.0 (`cs2cs` compilé par `tools/build-reference.sh`), et le chemin CPU de la crate,
déjà vérifié contre PROJ, sert de référence aux grilles de points.

La feature est **éteinte par défaut** : sans elle, `wgpu`, `pollster` et `bytemuck` ne sont pas dans
l'arbre de dépendances et aucune ligne de ce lot n'est compilée.

## Ce que fait l'API

```rust
let gpu = GpuContext::new()?;                      // adaptateur, périphérique, shader compilé
let proj = Proj::new_known_crs("EPSG:4326", "EPSG:32631", None)?;
let (_, rapport) = proj.convert_array_gpu(&mut points, &gpu)?;
```

`convert_array_gpu` rend le résultat de `convert_array`. Si **tous** les pas de la transformation ont
un noyau GPU, la tranche passe par le GPU, par blocs ; sinon elle passe par le chemin CPU, et le
`GpuReport` dit combien de points sont allés où et pourquoi (`Fallback`). Un appel ne rate jamais
faute de noyau. Les points d'un bloc que le GPU ne tranche pas (voir plus bas) font repasser **ce
bloc** par le CPU, qui rend sa réponse et son erreur.

## Un interpréteur, pas un shader par transformation

Écrire un shader par transformation obligerait à réécrire chaque projection. À la place :

- une transformation devient une **liste d'opérations** (`GpuProgram`), chacune agissant sur la paire
  (a, b) : échange d'axes et mise à l'échelle (`Affine`), préparation et finalisation de `Classic`
  (`PrepFwd`, `FinishFwd`, `PrepInv`, `FinishInv`), et un noyau (`MercFwd`, `TmercFwd`, `LccInv`...) ;
- un seul shader, l'**interpréteur** (`src/gpu/wgsl/interpreter.wgsl`), lit cette liste dans un buffer
  et l'exécute, un thread par point. Il est compilé une fois par `GpuContext` ;
- chaque `Step` se décrit lui-même avec `Step::lower_gpu` (`engine.rs`), et `Pipeline` assemble les
  siens depuis son plan (`plan::lower_gpu`), les suites d'échanges d'axes et de conversions d'unités
  déjà repliées en une seule opération `Affine`. Un pas sans noyau rend `false` : le tout reste sur le
  CPU.

Ajouter une projection au GPU, c'est donc : un `Opcode` (`program.rs`), sa fonction dans
`interpreter.wgsl`, un `gpu_op` sur le noyau et `Kernel::gpu_kernel`. Le reste (blocs, repli, rapport,
façade, axes de la façade ISO) est déjà là.

## Noyaux couverts

| noyau | sens | étape CPU |
|---|---|---|
| Transverse Mercator exact (Poder/Engsager, ordre 6) : `utm`, `tmerc`, `etmerc` | les deux | `Classic<Tmerc>` |
| Lambert conforme conique, ellipsoïdal ou sphérique, `n` de l'un ou l'autre signe | les deux | `Classic<Lcc>` |
| Mercator sphérique : `webmerc`, `merc` sans excentricité | les deux | `Classic<WebMerc>`, `Classic<Mercator>` |
| échanges d'axes et conversions d'unités | les deux | `Affine` du plan |

Toute paire du catalogue ISO qui n'est faite que de ces pas passe par le GPU : `EPSG:4326` vers
`EPSG:32631`, `2154`, `3857` et leurs inverses, `2154` vers `32631` (deux projections dans un seul
programme). Restent au CPU, par construction : les grilles, les changements de datum
(`cart`, `helmert`), les paires dont l'opération est choisie point par point (`Route::Alternatives`),
`+approx`, `+over`, `+geoc`, un pipeline avec `push` et `pop` (l'état est par point), Mercator
ellipsoïdal (son inverse itère), et toutes les autres projections.

## Précision

- **`f64`** quand le périphérique a `SHADER_F64` : Vulkan sur NVIDIA et AMD. Les tampons sont en `f64`.
- **Double-float** sinon (Metal, DX12, Intel, mobile) : un nombre est la somme de deux `f32`, environ
  48 bits de mantisse. Tout le calcul passe par les mêmes appels (`ad`, `sb`, `ml`, `dv`, `sq`) que
  le chemin `f64`, donc le même algorithme.

`sin`, `cos`, `atan2`, `exp` et `log` n'existent pas en `f64` dans les shaders. Naga accepte
`sin(x)` sur un `f64`, puis le pilote renvoie du faux sans erreur (lavapipe : `sin(2.0lf)` vaut 0).
Elles sont réécrites en WGSL (`interpreter.wgsl`, polynômes générés par `shader.rs`) : `sin` et `cos`
par réduction de Cody-Waite et série, `exp` par réduction par `ln 2` et série, `log` et `atan2` par
une estimation en `f32` du matériel suivie d'une correction à pleine précision (Halley, rotation
exacte), `asinh`, `sinh` et `atanh` par `exp` et `log`.

**Fast-math.** Un pilote qui réassocie replie `(a+b)-a` en `b` et détruit les termes d'erreur du
double-float. Mesa (lavapipe) le fait, Metal le fait par défaut, et `wgpu` 30 n'a aucun réglage pour
l'éviter. Deux parades :

1. `bar` (`prelude_df.wgsl`) : un `bitcast` suivi d'un ET avec un masque lu à l'exécution rend la
   somme opaque au compilateur. Elle entoure chaque somme et chaque produit exacts.
2. Au démarrage, `GpuContext` fait tourner un contrôle (`canary`) : `two_sum(1, 2^-30)` doit rendre
   `(1, 2^-30)` et `two_prod(1+2^-12, 1+2^-12)` doit rendre `(1+2^-11, 2^-24)`. S'il échoue, le
   double-float est refusé sur ce périphérique (`GpuError::Unusable`) et, quand `f64` manque aussi,
   aucun contexte n'est rendu : l'appelant reste sur le CPU.

Le produit exact passe par Veltkamp-Dekker : WGSL ne garantit pas que `fma` soit fusionnée.

**Ce que le double-float ne tient pas : Mercator près du pôle.** Une latitude en double-float est
connue à 48 bits, soit 5e-15 rad, et Mercator multiplie cela par `R / cos φ` : 4e-7 m à 85°, bien
plus au-delà. Mesuré : 0,94 µm au pire à 85°, tout juste sous la règle de parité. Au-delà de 85,1° le
noyau Mercator du double-float lève le drapeau et le CPU répond ; en `f64` seul le pôle lui-même
est écarté. La réduction d'argument de `sin` et `cos` soustrait `π/2` en trois morceaux `f32` (72
bits) pour que cette constante n'ajoute pas sa propre erreur à celle de l'entrée.

## Points que le GPU ne tranche pas

Le noyau lève un drapeau par bloc (un `atomicOr` sur un mot) ; le bloc entier repasse alors par le
chemin CPU, qui décide. Ce sont : une valeur non finie ou NaN, une latitude au-delà du pôle (le CPU
écrête ou refuse selon `EPS_LAT`), une longitude à plus de 10 radians, un point hors du domaine du
noyau (`ce` de Transverse Mercator au-delà de `CE_LIMIT`, pôle de Lambert, `|φ|` au-delà de 85,1°
en Mercator double-float et à 1e-9 du pôle en Mercator `f64`), `rho` nul au Lambert inverse, un `tau` de Newton à plus de `2/√ε`. La liste est
volontairement plus large que celle du CPU : un faux positif coûte un bloc au CPU, un faux négatif
serait un résultat faux.

Le contrat de `convert_array` est gardé : une erreur est celle que le CPU donne pour le premier point
fautif, les blocs avant lui sont convertis et celui qui échoue garde son entrée, une valeur qui ne
tient pas dans le type de la tranche (`f32`) est signalée à la fin.

## Mesures

Elles viennent de lavapipe (rendu logiciel de Mesa, `f64` natif disponible), la seule chose
disponible ici. **Aucune mesure de vitesse sur un vrai GPU n'a été faite.**

Écart contre PROJ 9.9.0 (`cs2cs`, 5 points par paire) : au plus 9,5e-8 m en double-float et 3,7e-9 m
en `f64` ; 4e-13 degré au plus au retour. Écart contre le chemin CPU sur 200 000 points par conversion
(monde entier en Web Mercator, zone entière en UTM, France entière en Lambert) :

| | double-float | `f64` |
|---|---:|---:|
| aller, mètres, au plus | 9,4e-7 (Mercator à 85°), 1,5e-7 pour les autres | 4,7e-9 |
| retour, degrés, au plus | 2,0e-12 | 6,8e-14 |

La règle de parité du dépôt (1 µm, 1e-11 degré, `docs/PARITY.md`) tient dans les deux cas, sans
tolérance propre au GPU. Le double-float de Mercator n'a presque pas de marge à 85°.

## Le banc

`cargo run --release --features gpu --bin proj-rs-gpu-bench` (`--quick`, `--big` jusqu'à 16 millions
de points, `--sizes a,b,c`) écrit du Markdown sur stdout. Pour chacune des six conversions (Web
Mercator, UTM 31N, Lambert-93, dans les deux sens) et chaque précision que le périphérique offre :

- l'écart au chemin CPU sur 200 000 points, avec le verdict contre la règle de parité ;
- le calcul seul (données déjà sur le périphérique, `Proj::gpu_dispatch_seconds`, caché), en ns par
  point et en Mpts/s ;
- l'appel entier (`convert_array_gpu` : conversion au format du périphérique, envoi, calcul,
  lecture), en ns par point, et le rapport au CPU **sur un cœur** et **sur tous les cœurs**
  (`project_array`, un `Proj` par thread créé avant l'horloge) : au-dessus de 1 le GPU gagne.

Un adaptateur logiciel (llvmpipe, WARP, SwiftShader) est signalé en tête : ses temps ne disent rien
d'un GPU. Le banc `proj-rs-bench --features gpu` garde ses quatre cas `gpu_convert_array_*` pour
l'historique (`bench/history.jsonl`) ; le banc GPU, lui, produit un rapport, pas une ligne d'historique.

## Mémoire

Un appel alloue, sur le périphérique, trois tampons de `16 × chunk_points` octets (entrée, sortie,
lecture) plus quelques octets, et les libère au retour : rien n'est gardé entre deux appels. Défaut :
1 048 576 points par bloc, soit 48 Mio pendant l'appel, borné par les limites du périphérique.
Côté hôte, par bloc : le bloc lui-même (`Vec<[f64; 2]>`), la copie de lecture, et en double-float la copie en `f32` à l'envoi, chacune de 16 octets par point.

## Écarts et limites

- **Pas bit à bit.** Le GPU a ses propres `sin` et `log` : l'écart au CPU est de l'ordre de 1e-7 m en
  double-float, pas zéro. `convert_array_gpu` le dit dans sa documentation.
- **`f64` en WGSL.** La documentation de `Features::SHADER_F64` parle de SPIR-V. Le type `f64` est
  accepté par le front WGSL de naga 30 et fonctionne sur lavapipe ; Metal et DX12 n'ont pas la
  feature et passent par le double-float. Un changement de naga peut le casser : les tests le
  verraient.
- **Un seuil, `min_points`.** Sous 262 144 points un appel reste au CPU (un envoi coûte plus que le
  calcul). C'est une estimation, pas une mesure : à régler avec `proj-rs-bench` sur la machine.
- **Pas de recouvrement.** Les blocs passent l'un après l'autre (envoi, calcul, lecture) ; un
  double tampon cacherait la latence du transfert.
- **MSRV.** `wgpu` 30 demande Rust 1.87, la crate déclare 1.83. Le job `msrv` de la CI compile
  `--all-features` : il ne passe qu'avec la bascule de la version minimale.
- **Les machines sans GPU** (la CI Linux) n'exécutent pas les tests d'intégration : ils rendent la
  main sans adaptateur. `PROJ_GPU_REQUIRED=1` en fait un échec. Les tests de lowering
  (`src/gpu/array.rs`) tournent partout. Sur Windows et macOS la CI peut trouver un adaptateur
  logiciel (WARP, Metal) et alors exécuter tout.

## Correspondance des tests

| test | ce qu'il garde |
|---|---|
| `tests/gpu_parity.rs::proj_values_*` | les valeurs de PROJ 9.9.0, dans les deux précisions |
| `web_mercator_over_the_world`, `utm_*`, `lambert93_over_france`, `two_projections_in_one_pipeline` | le GPU contre le CPU, aller et retour, sur des milliers de points, plusieurs blocs |
| `native_proj_strings_in_radians` | `Proj::new` : `utm`, `tmerc`, deux `lcc` (`n` de chaque signe, sphère), `webmerc`, `merc` sphérique |
| `a_transform_without_a_kernel_runs_on_the_cpu`, `projections_without_a_kernel_are_unsupported` | le repli, et son résultat identique au CPU bit à bit |
| `a_small_call_stays_on_the_cpu` | le seuil |
| `mercator_near_the_pole_stays_within_the_rule` | Mercator au-delà de 85° : le double-float passe au CPU, le `f64` reste sur le GPU, la règle tient |
| `a_block_with_a_point_the_gpu_does_not_decide_goes_to_the_cpu`, `an_error_is_the_cpu_error`, `a_longitude_wraps_as_on_the_cpu` | les points hors décision et les erreurs |
| `single_precision_slices` | une tranche `f32` |
| `src/gpu/array.rs::tests`, `src/gpu/program.rs::tests` | ce que chaque transformation devient, sans périphérique |

## Commandes

```bash
cargo test --features gpu --test gpu_parity        # un adaptateur, ou lavapipe
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json cargo test --features gpu --test gpu_parity
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo run --release --features gpu --bin proj-rs-bench        # cas gpu_convert_array_*
cargo run --release --features gpu --bin proj-rs-gpu-bench    # le banc GPU, en Markdown
```
