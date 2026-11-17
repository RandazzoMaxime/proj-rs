# Plan d'exécution d'un pipeline : les étapes affines fusionnées

Ce lot ne porte pas de code PROJ. `Pipeline::forward` appelait une étape après l'autre, et sur
`4326 → 2154` ce qui entoure `lcc` (`axisswap`, `unitconvert`, la boucle) coûtait environ 150
instructions par point. Une suite d'étapes qui ne font que permuter, changer le signe ou mettre à
l'échelle les quatre axes est maintenant calculée en une seule application.

## Ce qui est fusionné

Une étape se déclare affine pour un sens donné (`Step::affine(inverse)`). Elle rend alors une
permutation signée et mise à l'échelle des quatre axes : `sortie[k] = entrée[src[k]] × facteur[k]`.
Deux étapes le font :

| étape | affine | pas affine (garde ses étapes) |
|---|---|---|
| `axisswap` | toujours : `+order` à 2, 3 ou 4 axes, signes, `+axis` | |
| `unitconvert` | `xy_in`, `xy_out`, `z_in`, `z_out`, sans temps | `t_in` ou `t_out` ; sens inverse dont l'entrée est en radians (la longitude est ramenée dans ]-π, π] par `adjlon`) ; sens aller vers des radians avec `+z_0` ou une unité verticale (la hauteur est refaite par `vfr_meter × (z + z0)`) |

Une étape qui lit la pile (`push`, `pop`), `noop`, une projection, `helmert`, `cart` ne sont jamais
affines : elles coupent la suite. Les suites sont cherchées sens par sens, dans l'ordre où le
pipeline les exécute : une étape `+omit_fwd` (`+omit_inv` pour l'inverse) ne tourne pas et ne coupe
pas la suite qui l'entoure ; `+inv` sur une étape ou sur le pipeline entier retourne le sens de
chaque étape avant la recherche. Une suite réduite à l'identité ne laisse aucune étape.

## Ce qui ne bouge pas

- **La définition.** `Pipeline::steps` garde les étapes telles que le texte les liste : les unités
  d'entrée et de sortie du pipeline (`input_kind`, `output_kind`) et leur contrôle à la création
  viennent d'elles. Le plan est un second champ, `forward_plan` et `inverse_plan`. `Proj::def`,
  `definition_in_authority_order`, `projinfo`, `proj -V` et `cct --verbose` lisent le texte ou le
  modèle, jamais le plan (`tests/affine_plan.rs`, et le corpus `parity_export`).
- **Les erreurs.** Une étape affine ne rend jamais d'erreur ; les autres gardent leur ordre. Sur une
  tranche, un point déjà en échec n'est pas touché.
- **Les valeurs.** Les facteurs d'une suite se multiplient entre eux. Pour un seul facteur et des
  signes (`order=2,1` puis `deg → rad`, tous les pipelines de CRS à un changement d'unité) le
  produit est exact : mêmes bits qu'avant. Pour deux facteurs (`ft → m` puis `m → km`) le produit
  peut différer d'un bit de `(x × a) × b`. La règle de tolérance de `docs/PARITY.md` (1 µm,
  1e-11 degré) le permet ; aucun seuil, aucun registre n'a bougé. Sur le corpus de
  `tests/affine_plan`, 140 lignes sur 1 933 ne sont plus identiques à la lettre ; l'écart relatif
  le plus grand est 2e-15, et un zéro y change de signe (`-0.0` contre `0.0`).
- **La pile de `push` / `pop`** n'est pas touchée.

## Garanties

`tests/affine_plan/golden.txt` est la sortie du code qui exécutait chaque étape (1 933 lignes :
200 chaînes tirées d'une graine, aller et inversées, et 9 pipelines écrits à la main — suite
coupée par un temps, par `adjlon`, par `+z_0`, par `push`, par `ortho` qui refuse un point —, cinq
points chacun ; une chaîne que la création refuse compte une ligne). Le test relit les nombres et exige l'accord à 1e-13 relatif près, le même texte d'erreur,
et au moins 90 % de lignes identiques à la lettre (93 % aujourd'hui). Il a été capturé sur le code
d'avant et passe sur le code d'après ; remplacer la fusion par une permutation fausse, ou
retirer le garde de `+z_0`, le fait échouer.

## Mesure

`valgrind --tool=callgrind`, instructions par point (différence de deux tailles d'entrée, pour
retirer la création), et `taskset -c 2`, minimum de 9 passes alternées, `ns/pt` indicatif (VM
partagée). Binaire `--release`, `Proj::new_known_crs` puis `convert` point par point.

| paire | instr./pt avant | après | ns/pt avant → après | `convert_array` ns/pt |
|---|---:|---:|---:|---:|
| 4326 → 2154 | 842 | 759 (-9,9 %) | 114,6 → 106,5 | 33,4 → 31,6 |
| 4326 → 32631 | 1 132 | 1 049 (-7,3 %) | 151,7 → 144,3 | 39,6 → 37,0 |
| 2154 → 32631 | 1 693 | 1 690 | 312,6 → 309,3 | 87,7 → 87,9 |
| 4326 → 28992 | 2 254 | 2 174 (-3,5 %) | 315,8 → 311,8 | 308,9 → 292,8 |
| 4326 → 2056 | 2 554 | 2 470 (-3,3 %) | 372,1 → 358,1 | 363,7 → 351,6 |
| 4230 → 4326 | 550 | 549 | 47,8 → 47,6 | 33,8 → 33,2 |
| 4326 → 27700 | 1 790 | 1 707 (-4,6 %) | 205,0 → 195,3 | 171,9 → 166,5 |

Le gain est une constante d'environ 80 instructions par point là où le pipeline a un
`axisswap` et un `unitconvert` : 10 % sur une conversion de projection pure, 3 à 5 % quand un
changement de datum (`cart`, `helmert`, `push`, `pop`) pèse plus lourd. `2154 → 32631` n'a aucune
étape affine : pas de gain, pas de perte. `4230 → 4326` n'est qu'un `noop`.

Une première version, avec `Exec` sans étiquette explicite et la boucle appelée au lieu d'être
incluse, perdait 8 à 30 instructions sur les pipelines sans étape affine : `#[inline(always)]` sur
`plan::run` et `#[repr(u8)]` sur `Exec` les rendent.
