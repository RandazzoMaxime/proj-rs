# Calcul par lots en AVX2, AVX-512 et NEON (`src/simd/`

PROJ 9.9.0 n'a pas de chemin vectoriel : `pj_fwd` et `pj_inv` convertissent un point à la fois
(`proj_trans_generic` boucle). Ce module est donc un ajout de la crate, pas un portage. La
référence des valeurs reste PROJ : `tests/simd_batch.rs` compare chaque lot à `cs2cs` 9.9.0.

## Architecture

```
src/simd/
  vec.rs      Vector<N> (le contrat, N voies), Mask, SimdToken (jeton + vectorize)
  mod.rs      Lanes (le contrat d'un noyau), run (le driver), batch_path
  v4.rs       x86-64 : Avx2 (preuve de support), V4 (quatre f64), Mask
  v8.rs       x86-64, feature `avx512` : Avx512, V8 (huit f64), Mask
  neon.rs     aarch64 : Neon (preuve de support), V4 (deux registres de deux f64), Mask
  math.rs     sin_cos tan exp log atanh asinh sinh atan atan2 hypot powf, en Rust, sur Vector<N>
  kernels.rs  briques partagées : adjlon, tsfn, phi2, latitude_in_domain
  drive.rs    la boucle par groupes de N (sur x86-64, le seul code compilé avec les extensions du jeton)
src/tmerc/batch.rs   Transverse Mercator : forward, inverse
src/lcc/batch.rs     Lambert conique conforme : forward, inverse
src/merc/batch.rs    Mercator, sphère et ellipsoïde : forward, inverse
```

Une projection écrit sa formule contre l'API sûre de `Vector<N>`, générique sur le nombre de
voies (opérateurs, `select`, `sin`, …). Elle n'écrit ni intrinsèque, ni `unsafe`, ni détection du
processeur. Pour ajouter une projection : implémenter `Lanes` (`point` : le chemin scalaire ;
`lanes` : `N` points, ou `None`), puis appeler `simd::run` depuis `forward_batch` /
`inverse_batch`. La formule tourne telle quelle sur AVX2 (4 voies), AVX-512 (8 voies) et NEON.

### Le modèle de sûreté

- `Avx2` ne s'obtient que par `Avx2::detect`, qui interroge le processeur : c'est le jeton
  `pulp::x86::V3` (`V3::try_new`, `is_x86_feature_detected!` : AVX, AVX2, FMA, BMI1, BMI2, LZCNT,
  POPCNT et les SSE, le niveau x86-64-v3, qu'ont tous les processeurs AVX2 et FMA).
- Un `V4` ou un `Mask` garde ce jeton (de taille nulle) : il n'existe qu'à partir d'un `Avx2`.
  Ses méthodes appellent les intrinsèques par les méthodes sûres du jeton (`simd.avx._mm256_add_pd`,
  `simd.fma._mm256_fmadd_pd`, …) ; le chargement et l'écriture sont des `bytemuck::cast` entre
  `[f64; 4]` et `__m256d`. Le dépôt n'a aucun `unsafe` ici : c'est `pulp` qui le porte.
- Le débit dépend d'une autre condition, qui n'est pas de sûreté : le code doit être inséré
  dans une fonction compilée avec ces extensions. C'est `V3::vectorize`, appelé par
  `drive::run_simd` et `run3_simd` avec une structure (`Groups`, `Groups3`) dont la méthode
  `call` est `#[inline(always)]`. Les méthodes de `V4` et les noyaux sont `#[inline(always)]`.
  Hors de cette fonction le code reste correct, mais les intrinsèques ne sont plus en ligne.
- Il n'y a ni `asm!` ni appel à la bibliothèque C : les fonctions mathématiques de `math.rs` sont du
  Rust écrit contre l'API sûre de `V4`.
- **Une fermeture n'est pas insérée en ligne comme le reste.** Elle n'hérite pas du
  `target_feature` de la fonction qui l'appelle, et LLVM ne l'insère pas toujours : ses
  intrinsèques deviennent alors des appels. Un `sinh` écrit avec deux fermetures coûtait 290 ns
  par appel de quatre voies au lieu de 9,5. Le code de `math.rs` et des noyaux utilise donc des
  fonctions `#[inline(always)]` de module, jamais une fermeture qui calcule avec des `V4` (la
  petite fermeture `k` qui diffuse une constante est insérée sans problème). Le même piège vaut
  pour `vectorize` : une fermeture `|| groups(...)` donnait un lot trente fois plus lent
  (Lambert-93 aller 19 → 494 ns par point) ; une structure qui implémente `NullaryFnOnce` ne l'est
  pas.

Le calcul vectoriel n'a plus de `unsafe` (voir « Sans `unsafe` : `pulp` » plus bas).
Avant le lot du module, chaque projection portait ses intrinsèques et sa détection (`simd.rs`,
`tmerc.rs` et `lcc.rs`) ; avant le lot des fonctions mathématiques, `libmvec` passait par `asm!`.

### Le driver et le repli

`run` découpe les tranches par groupes de `N` voies (8 en AVX-512, 4 en AVX2/NEON). Pour chaque groupe, `Lanes::lanes` rend les
`N` résultats, ou `None` quand une voie sort du domaine que le noyau vectoriel traite
(valeur non finie, latitude hors de ±90°, pôle d'un cône, `rho` nul, `|Ce|` au-delà de la limite
de Transverse Mercator, latitude non finie à l'inverse d'un cône). Le groupe passe alors par
`Lanes::point`, le chemin scalaire : ses points valent ceux de `forward` et `inverse` **au
bit près**, et un point refusé est écrit `NaN`. La queue (moins de `N` points) passe aussi par
`point`. Sans le jeu d
instructions, ou hors x86-64/aarch64, `run` boucle sur `point` : les résultats sont ceux du chemin
scalaire.

## Précision

Un lot AVX2 s'écarte du scalaire de l'erreur des fonctions mathématiques vectorielles (quelques
ULP), pas de la formule : chaque noyau suit sa version scalaire opération par opération. Mesuré
sur 200 000 points par projection (`tests/simd_batch.rs`, sonde retirée après la mesure), avec l'FMA
et les fonctions mathématiques de `math.rs` :

| projection | aller (m) | retour (degré) | aller avec `libmvec` (m) |
|---|---:|---:|---:|
| UTM 31 | 2,8e-9 | 2,8e-14 | 4,7e-9 |
| `tmerc` | 3,3e-9 | 2,8e-14 | 3,7e-9 |
| Lambert-93 | 3,7e-9 | 4,3e-14 | 8,4e-9 |
| Lambert, cône sud | 6,1e-9 | 4,6e-14 | 1,2e-8 |
| Web Mercator | 3,7e-9 | 1,4e-14 | 5,6e-9 |
| Mercator ellipsoïdal | 4,7e-9 | 2,1e-14 | 7,5e-9 |

Les écarts à l'aller sont plus petits qu'avec `libmvec` (dernière colonne) : les fonctions de
`math.rs` sont à 1 à 3 ULP de la libm, `libmvec` jusqu'à 4.

Les bornes des tests sont 1e-7 m et 1e-12° (marge de dix à vingt). Près du pôle opposé à un
cône, les coordonnées dépassent 1e10 m, où un ULP dépasse la borne absolue : le test admet en
plus 4e-15 relatifs.

Contre `cs2cs` 9.9.0 (2 003 points par projection, aller puis retour) : moins de 1e-6 m et
1e-9°, la précision de l'impression de `cs2cs` (`-d 9` et `-d 11`).

## FMA

Le chemin AVX2 est compilé avec `fma` (`avx2,fma`) et `V4::mul_add` fait un `vfmadd` : un produit et
une addition arrondis une seule fois. Les noyaux s'en servent là où une multiplication est suivie
d'une addition : les séries de Clenshaw de Transverse Mercator (`gatg`, `clenshaw_complex`), la
mise à l'échelle finale (`a * x + x0`) de chaque projection, le produit de `northing` de Lambert,
et le Newton de `phi2` (`tau * tau + 1`, `sig * sig + 1`). `adjlon` n'en a pas : le produit n'est
calculé que pour une longitude à ramener, et le gain serait nul.

- Le chemin **scalaire** n'a pas d'FMA : `f64::mul_add` y serait logiciel sans FMA matériel. Un
  lot ne vaut donc pas le scalaire au bit près, ce qu'il ne valait déjà pas (les fonctions
  mathématiques vectorielles diffèrent aussi) ; il est un peu plus proche de la valeur exacte.
- La précision mesurée contre le scalaire ne bouge pas : voir la table ci-dessus (UTM aller 3,7 →
  4,7 nm, les autres écarts identiques à 1 % près), et `cs2cs` 9.9.0 reste d'accord à 1e-6 m.
- **Gain mesuré** (`bench`, un cœur, quatre exécutions alternées de l'ancien et du nouveau binaire,
  minimum de chacun) : UTM aller 27,3 → 24,7 ns (1,10×, les plages des quatre exécutions ne se
  chevauchent pas) ; Lambert-93 retour 58,2 → 56,8 ; Lambert-93 aller, UTM retour, Web Mercator
  aller et retour : de 1 à 2 %, au niveau du bruit. L'FMA ne compte que là où les séries de
  Clenshaw dominent : l'essentiel du temps ailleurs était dans les appels `libmvec` (mesuré avant
  le lot des fonctions mathématiques).

## Fonctions mathématiques (`math.rs`)

`sin_cos`, `tan`, `exp`, `log`, `atan`, `atan2`, `hypot`, `asinh`, `atanh`, `sinh` et `powf` sont
écrites en Rust contre `V4`, sans `libmvec`, sans `asm!` et sans dépendance : le même code sert
Linux, Windows et macOS. Les algorithmes viennent de deux sources :

- **fdlibm** (la libm de Sun, reprise par FreeBSD et musl) : réduction d'argument de Cody et Waite
  en trois morceaux de `pi/2` (précision relative du reste près d'un multiple de `pi/2`), noyaux
  polynomiaux de `__kernel_sin` et `__kernel_cos`, `exp`, la queue de `log` en double précision
  étendue de `powf`.
- **Cephes** (S. L. Moshier), tel que le reprend la bibliothèque vectorielle d'Agner Fog (licence
  Apache-2.0) : `atan` (trois plages, fraction rationnelle de degré (4, 5) en `z^2`), `log`
  (fraction rationnelle de degré (5, 5) sur la mantisse centrée en 1) et `atan2` (réduction au
  premier octant, puis la même fraction). Une fraction rationnelle coûte moins qu'un polynôme de
  degré 22 pour la même précision.

Les polynômes sont évalués par le **schéma d'Estrin** (`estrin4`, `estrin5`, `estrin12`) et non par
Horner : des paires `c0 + c1 x` calculées en parallèle puis combinées par `x^2`, `x^4`, `x^8`. La
chaîne de FMA dépendants passe de 11 à 4 pour `exp` et de 5 à 3 pour une fraction de `log` ou de
`atan`, ce qui compte parce que le processeur recouvre les itérations indépendantes d'une boucle de
lots mais pas les FMA d'une même chaîne. Les coefficients de fdlibm étaient recopiés de mémoire, ceux
de Cephes depuis le code de la crate `wide` (Apache-2.0, MIT et Zlib, qui l'a de la bibliothèque
d'Agner Fog) : c'est le banc de précision qui les valide. Cephes est redistribué sous
licence BSD 3 clauses avec l'accord de son auteur, comme dans SciPy ; l'attribution est dans
[`NOTICE.md`](../NOTICE.md).

- **`sin_cos`** rend les deux d'une seule réduction. Les noyaux s'en servent là où ils calculaient
  un sinus et un cosinus du même angle (trois paires pour Transverse Mercator aller).
- **`powf`** calcule `log(x)` en double précision étendue (deux `f64`, erreur relative de l'ordre de
  `2^-65`) puis l'exponentielle de `y * log(x)` : l'erreur ne grossit pas avec `|y * log(x)|`.
  Un `exp(y * log(x))` en `f64` ferait de l'ordre de 5 ULP pour un exposant de l'ordre de 2.
- **`asinh`, `atanh`, `sinh` et `atan` ont un chemin rapide** quand toutes les voies sont petites :
  la série entière pour `|x| < 0.15` (`asinh`, `atanh`), `< 0.5` (`sinh`), pas de réduction de plage
  pour `|x| < 7/16` (`atan`). Les arguments des noyaux le sont presque toujours (`e * sin(phi)`
  vaut au plus 0,08). Si une voie ne l'est pas, les quatre passent par le chemin général : le
  résultat ne dépend que de la précision, pas du chemin.
- **Hors du domaine rapide**, une voie est recalculée par la fonction de la bibliothèque standard
  (`NaN`, infinis, `exp` au-delà de 700, `log` d'un nul, d'un négatif ou d'un sous-normal, `sin` au-delà
  de `2^20`, `powf` d'une base négative…) : le résultat aux cas limites est celui du scalaire.

Précision mesurée contre la libm de `std` (`accuracy_matches_the_libm`, 250 000 groupes de
quatre par cas en release, de larges domaines : `sin` et `cos` sur `[-1e5, 1e5]` et près des
multiples de `pi/2`, `exp` sur `[-700, 700]`, `log` sur `[1e-300, 1e300]`, `atan`, `hypot` et
`atan2` sur dix à deux cents décades) :

| fonction | erreur max (ULP) |
|---|---:|
| `sin`, `cos`, `exp`, `log`, `atan`, `hypot`, `powf` | 1 |
| `sinh`, `atan2` | 2 |
| `asinh`, `atanh` | 3 |
| `tan` (quotient de deux fonctions à 1 ULP) | 3 |

Les bornes des tests sont ces mesures plus un ; `libmvec` annonce jusqu'à 4 ULP. Un second test,
`special_values_match_the_libm`, compare `NaN`, infinis, zéros signés, négatifs, sous-normaux et
dépassements à la libm, pour les fonctions à un argument et toutes les paires de `atan2`, `hypot` et
`powf`. La référence de `atanh` est symétrisée : `f64::atanh` de `std` vaut `0.5 * ln_1p(2x / (1 - x))`
et perd presque tous ses chiffres pour `x` proche de -1 (31 000 ULP mesurés), alors que ces
fonctions travaillent sur `|x|`.

### Vitesse

Un cœur épinglé, quatre exécutions alternées des binaires, minimum de chacun, ns par point.
« Voie par voie » est l'ancien repli de Windows et macOS (la libm appelée une voie à la fois, forcé
ici sur Linux : ce n'est donc pas la CRT de Microsoft) ; « `libmvec` » est le chemin de Linux avant
le lot des fonctions mathématiques ; « fdlibm » est la première version de `math.rs` (`atan` à
polynôme de degré 22, `log` de fdlibm, Horner) ; « maintenant » ajoute Cephes et Estrin. La
colonne « voie par voie » vient de la mesure du lot précédent (même machine, même jour, autre
session) : les rapports qui l'utilisent sont approximatifs. Les trois autres colonnes viennent de
la même série de mesures alternées.

| cas | voie par voie | `libmvec` | fdlibm | maintenant | maintenant / voie par voie | `libmvec` / maintenant |
|---|---:|---:|---:|---:|---:|---:|
| UTM 31 aller | 60,2 | 24,5 | 20,5 | 18,8 | 3,2× | 1,30× |
| Lambert-93 aller | 41,9 | 19,3 | 23,9 | 19,1 | 2,2× | 1,01× |
| UTM 31 retour | 71,0 | 21,1 | 25,7 | 24,7 | 2,9× | 0,85× |
| Lambert-93 retour | 97,3 | 56,6 | 52,6 | 47,9 | 2,0× | 1,18× |
| Web Mercator aller | 33,7 | 11,0 | 12,6 | 12,0 | 2,8× | 0,91× |
| Web Mercator retour | 30,7 | 6,0 | 11,5 | 8,8 | 3,5× | 0,68× |

- **Windows et macOS x86-64 :** 2,0 à 3,5× plus vite que le repli voie par voie. Je n'ai pas mesuré
  la CRT de Microsoft ni macOS : le chiffre est celui d'un repli forcé sur Linux.
- **Linux, face à `libmvec` :** devant sur UTM aller (1,30×) et Lambert-93 retour (1,18×), à égalité
  sur Lambert-93 aller, derrière sur UTM retour (0,85×), Web Mercator aller (0,91×) et Web Mercator
  retour (0,68× : `sinh` puis `atan` sur des arguments qui ne sont pas petits, donc sans chemin
  rapide). Le but de ces lots est la portabilité et la précision ; le coût sur Linux est de 0 à 3 ns
  par point selon la projection.
- **Par fonction** (un appel de quatre voies, un cœur, avant → maintenant) : `atan` 12,1 → 7,0 (2,5
  sur le chemin rapide), `atan2` 14,7 → 10,7, `log` 6,8 → 6,1, `exp` 6,6 → 5,7, `powf` 22,9 → 20,3,
  `sinh` 18,6 → 10,3 sur des arguments mélangés (3 en série), `atanh` 13,5 → 12,5 (3 en série),
  `sin_cos` 9,0, `hypot` 3,1. La crate `wide` (mesurée à part, `target-cpu=native`) fait `atan` en 4,7,
  `ln` en 4,1 et `atan2` en 8,3, avec un autre banc : je n'ai pas isolé la cause de l'écart restant
  (les chemins hors domaine rapide et les sélections de `V4` sont des pistes, non mesurées).

## Fonctions scalaires : retenues et rejetées

`Proj::convert` et `Operation::forward` calculent un point à la fois. Callgrind sur
`Proj::new_known_crs("EPSG:2154", "EPSG:32631")` : 2042 instructions par point, dont les deux tiers
dans la libm de glibc (`atan2` 210, `sincos` 300, `pow` 100, `hypot`, `sinh`, `atanh`, `asinh`,
`atan`). `src/fmath.rs` remplace une partie de ces appels par des séries de `math.rs` en une voie,
Rust pur, sans FMA (`f64::mul_add` est logiciel sans FMA matériel), donc valables sur toutes les
cibles. Elles ne rendent plus les bits de glibc : la parité avec PROJ est la tolérance de
`docs/PARITY.md` (1e-6 m, 1e-11 degré, facteurs à 1e-7 relatif).

Latence d'un appel, `fmath` contre glibc :

| fonction | `fmath` / glibc (ns) | verdict |
|---|---:|---|
| `hypot` (`sqrt(max² + min²)`) | 13,5 / 23,9 | **retenue**, gagne 44 % |
| `asinh` (série, `|x| < 0,1`) | 13,0 / 32,0 | **retenue**, gagne 60 % |
| `sinh` (série, `|x| < 0,1`) | 10,6 / 26,0 | **retenue**, gagne 60 % |
| `atanh` (série, `|x| < 0,1` ; `e sin(phi)` ≤ 0,082) | 12,3 / 24,5 | **retenue**, gagne 50 % |
| `atan2` (Cephes) | 24 à 30 / 33 à 34 | rejetée : 10 à 27 %, aucun gain sur le point entier (246,6 contre 245,2 ns) |
| `sin_cos` (fdlibm) | 18,7 à 23,2 / 21,6 à 22,6 | rejetée : plus lent en débit (15 à 17,5 contre 8 à 9,8 ns), glibc a des noyaux FMA |
| `atan` (Cephes) | 21,6 à 22,3 / 20,8 à 24,0 | rejetée : aucun gain |

Hors de leur domaine (`|x| >= 0,1`, valeur non finie, dépassement), `hypot`, `sinh` et `asinh`
retombent sur la fonction de la bibliothèque standard, et `atanh` sur `fmath::atanh_libm` (la `atanh`
de glibc, voir plus bas) : le résultat aux cas limites est celui de la libm. `hypot` est utilisée par `Tmerc` (forward et inverse) et `Lcc`
(inverse), `asinh` par le forward de `Tmerc`, `sinh` par l'inverse de `Tmerc` et de `Lcc` et par
`projmath::sinhpsi2tanphi`, `atanh` partout où `projmath::atanh` est appelée (`Lcc`, Mercator
ellipsoïdal, `tsfn`). Le repli portable de `Tmerc::forward_batch` appelle les mêmes fonctions : il
reste bit à bit celui de `Tmerc::forward`. Le `asinh` de Mercator et de Web Mercator reste celui de
la libm.

Un cœur épinglé (`taskset -c 2`), minimum de 14 passes alternées de 300 000 points ; instructions
par point en différence de deux runs callgrind (100 000 et 300 000 points) :

| cas | instr. avant | instr. après | ns avant | ns après |
|---|---:|---:|---:|---:|
| 2154 vers 32631 | 2042 | 1694 | 334,0 | 246,0 |
| 4326 vers 2154 | 893 | 852 | 127,1 | 112,9 |
| 4326 vers 32631 | 1148 | 1073 | 123,7 | 117,4 |
| 4326 vers 3857 | 558 | 558 | 88,5 | 88,9 |

4326 vers 3857 n'est pas touché (Web Mercator sphérique : ni `atanh` ni `hypot`). Les ns sont indicatifs (machine
partagée) ; une mesure antérieure sur un poste plus calme donnait 326 → 264, 100 → 90 et 142 → 121 ns.
Les points 4326 sont écrits (longitude, latitude) comme le veut `new_known_crs`.

Écart contre l'ancien code (la libm), 100 000 points par cas sur la France :

| cas | écart max |
|---|---:|
| 2154 vers 32631 | 5,6e-9 m |
| 4326 vers 2154 | 1,9e-9 m |
| 4326 vers 32631 | 9,3e-10 m |
| 32631 vers 2154 | 3,7e-9 m |
| 2154 vers 4326 | 4,3e-14 degré |
| 32631 vers 4326 | 1,4e-14 degré |
| 4326 vers 3857, 3857 vers 4326 | 0 |

Facteurs (`Proj::factors`, 3 000 points par CRS) : `h`, `k`, `s`, convergence et dérivées à 1,1e-11
relatif au plus ; distorsion angulaire à 3,7e-8 degré absolu au plus. L'angle méridien-parallèle `theta'` et les
axes de l'ellipse de Tissot (`proj -S` imprime ces derniers) montent à 2,3e-8 relatif sur UTM 31 (la règle des facteurs est donc 1e-7) :
près d'une projection conforme, `a` et `b` sortent d'une différence qui amplifie le bruit des
différences finies, d'où le 1e-7 de la règle.

Le terme relatif linéaire de la règle (1e-13) vient de `cs2cs-inverse-0105:4` de la suite de parité : UTM/LCC (EPSG:32150) vers NZMG (EPSG:27200) sur un point hors domaine, 7e11 m. L'inverse Lcc de `fmath` rend une latitude à 1 ULP de la libm (7e-15 degré) ; le polynôme de degré 9 de NZMG l'amplifie environ 5e6 fois : 4e-3 m, 5,7e-15 relatif, au-dessus des 5e-4 m absolus de la suite.

Le reste de la piste « calculs redondants » ne donne rien : LLVM fusionne déjà `sin` et `cos` d'un
même argument en `sincos`, et regrouper `sin(phi)` et `cos(phi)` de `Lcc::fwd_unit` laisse 844
instructions par point. `pow` (100 instructions, une fois par point dans `Lcc::inv_unit`) est déjà
rapide.

### `atanh` hors de la série : la formule de glibc, sans appel C

`projmath::atanh` appelait la `atanh` de la libm C par un `extern "C"` (deux `unsafe`, et deux de
plus dans les tests de `fmath`). Elle appelle maintenant `fmath::atanh_libm`, la formule de
`__ieee754_atanh` de glibc (`sysdeps/ieee754/dbl-64/e_atanh.c`) : `x` sous `2^-28`,
`0.5 * log1p(t + t * |x| / (1 - |x|))` avec `t = 2|x|` sous 0,5, `0.5 * log1p(2|x| / (1 - |x|))`
sous 1, l'infini à ±1, NaN au-delà, puis le signe de `x`. Son seul appel, `log1p`, est
`f64::ln_1p`, c'est-à-dire la `log1p` de la libm de la plateforme : rien n'est recopié de la libm
elle-même. `f64::atanh` n'est pas cette formule (`0.5 * ln_1p(2x / (1 - x))` pour tout `x`) et
diffère d'un ULP sur environ un tiers des entrées sous 0,5.

- **Bits (glibc 2.39, x86-64)** : `fmath::tests::matches_the_c_libm_bit_for_bit` compare
  `atanh_libm` à la vraie `atanh` de la libm, appelée par un programme C compilé pendant le test
  (`cc -O0 -fno-builtin … -lm`), au bit près, NaN compris : 14 070 020 entrées en `--release`
  (1,5 million en debug), sans un seul écart. Les entrées : les cas limites (zéros signés, ±0,5,
  ±1, NaN des deux signes, infinis, sous-normaux, `f64::MAX`), 20 000 ULP de chaque côté de
  `2^-28`, 0,1, 0,5 et 1 (des deux signes), 5 millions de valeurs uniformes dans [-1, 1], 5 millions
  en échelle logarithmique de 1e-12 à 1, 1,25 million de 1e-320 à 1e-8, 1,25 million de `1 - u`
  (`u` de 1e-16 à 0,1) et 1,25 million de motifs de 64 bits quelconques. Le test échoue bien quand
  on remplace la branche sous 0,5 par celle sous 1 (premier écart à `x = 0,0999…`, un ULP).
- **Vitesse** : `projmath::atanh` sur un million de valeurs de [0,1, 0,99] (le domaine hors série),
  `taskset -c 2`, meilleure de 30 passes, deux séries alternées : 12,07 et 11,79 ns par appel avant
  (libm), 11,44 et 11,21 après. Le chemin chaud (`|x| < 0,1`, la série) ne change pas.
- **Écart volontaire hors de glibc.** Sous macOS et Windows, l'ancien code appelait la `atanh` de
  la libm de la plateforme (celle qu'utilise PROJ sur cette plateforme). Il n'y a pas d'appel sûr à
  cette fonction : le résultat est maintenant la formule de glibc avec la `log1p` de la plateforme,
  qui peut s'écarter d'un ULP de la `atanh` de Apple ou de Microsoft. La parité avec PROJ est la
  tolérance de `docs/PARITY.md` (1e-6 m, 1e-11 degré), que `fmath` admet déjà : ses séries
  s'écartent de 1 à 2 ULP de la libm. La preuve au bit près ne tourne que sur glibc.

## Sans `unsafe` : `pulp`

`v4.rs` (53 `unsafe`) et `drive.rs` (4) appelaient les intrinsèques de `std::arch` et la fonction
`target_feature` du driver dans des blocs `unsafe`. Sous Rust 1.83, le plancher du dépôt, les
intrinsèques sont des `unsafe fn`, et l'appel d'une fonction `target_feature` depuis du code sans
ces extensions l'est toujours. Les deux passent maintenant par `pulp` 0.22.3 (MIT, de l'auteur de
`faer`, maintenu ; compile sous 1.83) : ses jetons ont une méthode sûre par intrinsèque, et
`V3::vectorize` exécute une fonction compilée avec les extensions du jeton.

- **Les mêmes instructions, les mêmes bits.** Chaque méthode de `V4` appelle la même intrinsèque
  qu'avant, dans le même ordre, avec l'FMA là où elle y était. `tests/simd_bits.rs`, écrit et figé
  avant le changement, prend l'empreinte des sorties de 28 cas : `forward_batch` et
  `inverse_batch` d'UTM 31, de Lambert-93 et de Web Mercator sur 40 003 points dont un sur 97 envoie
  son groupe au chemin scalaire (NaN, infini, pôle, hors domaine), et `convert_array` puis
  `project_array` de 11 paires (Lambert, Transverse Mercator, Mercator, Web Mercator, Lambert
  azimutal, stéréographique oblique, Mercator oblique suisse, Helmert par les géocentriques). Les 28
  empreintes, et celles de `tests/convert_array_golden.rs`, sont identiques avant et après.
- **Instructions.** Callgrind sur la boucle de Lambert-93 aller (400 000 points) : 118,0
  instructions par point avant, 118,5 après (+0,4 %, deux instructions par groupe de quatre : le
  passage des arguments et l'allocation des registres ; le corps de la boucle est le même).
- **Le passage des arguments compte.** Une première version passait à `vectorize` une structure de
  neuf mots (le noyau et quatre tranches), que `pulp` passe par pointeur : Lambert-93 aller était
  3,3 % plus lent (26,04 → 26,86 ns, minimum de 40 passes, 8 séries alternées, banc direct sur un
  million de points). Les sorties passent maintenant par une seule référence : la structure tient en
  huit mots et passe en registres (26,04 → 26,04 à 26,22 ; UTM 31 aller 25,7 → 25,3).
- **Vitesse** (`proj-rs-bench`, `taskset -c 2`, l'ancien et le nouveau binaire alternés, 6 séries
  chacun ; minimum, puis médiane) : `utm31_batch_center` -0,4 % / -0,8 %, `utm31_batch_edge`
  -1,3 % / -1,0 %, `lambert93_batch` +0,1 % / -0,1 %, `utm31_inverse_batch` -2,1 % / -5,4 %,
  `lambert93_inverse_batch` -1,2 % / -3,0 %, `webmerc_batch` +1,3 % / -1,6 %,
  `webmerc_inverse_batch` -1,1 % / -1,2 %, `facade_convert_array_*` de -1,1 % à -0,5 % sur le
  minimum. Sur les 214 cas du banc, la médiane des écarts du minimum est 0,0 % (de -2,5 % à +3,3 %,
  le bruit de la machine ; les cas qui n'appellent pas le chemin AVX2 bougent d'autant).
- **Détection.** `V3::try_new` demande aussi BMI1, BMI2, LZCNT et POPCNT, en plus d'AVX2 et FMA :
  tous les processeurs AVX2 et FMA connus les ont (Intel depuis Haswell, AMD depuis Excavator). Un
  processeur virtuel qui masquerait BMI mais exposerait AVX2 prendrait le chemin scalaire.
- **Dépendances ajoutées** (x86-64 seulement) : `pulp`, `bytemuck`, `num-complex` (et
  `num-traits`), `paste`, `reborrow`, `raw-cpuid`, `libm`, `cfg-if`, `pulp-wasm-simd-flag` ;
  licences MIT, Apache-2.0 ou Zlib.

## NEON (aarch64)

Sur aarch64 le lot tourne par groupes de quatre, comme AVX2 sur x86-64 (huit avec AVX-512). `V4` a deux réalisations, que
`mod.rs` choisit par l'architecture : `v4.rs`, un registre `__m256d`, et `neon.rs`, deux
`float64x2_t` (les voies 0 et 1, puis 2 et 3). Elles ont la même API : le driver, `kernels.rs`,
`math.rs` et les formules des projections sont les mêmes pour les deux. Le jeton de preuve
s'appelle `Token` ; c'est `Avx2` sur x86-64 et `Neon` sur aarch64. La porte `cfg` du code par lots
est `simd_lanes`, que `build.rs` pose pour ces deux architectures.

Les intrinsèques passent par `pulp`, comme pour AVX2 (`pulp::aarch64::Neon`) : `neon.rs` n'a pas de
`unsafe`. NEON fait partie du socle des cibles aarch64, donc il n'y a pas de fonction
`target_feature` à entrer : `Neon::vectorize` appelle la fonction, et le code est en ligne là où
il est appelé.

**Les mêmes bits qu'AVX2.** AVX2 et NEON arrondissent de la même façon : IEEE 754 pour
l'addition, le produit, le quotient et la racine, et un FMA fusionné (`vfmadd` d'un côté, `fmla` de
l'autre) pour `mul_add`. `math.rs` ne compose les fonctions que par l'API de `V4`, dans le même
ordre : le résultat d'une fonction est donc le même sur les deux architectures. Trois opérations
ont des instructions qui diffèrent, et `neon.rs` les rend identiques :

- `max` et `min` : `maxpd` rend la seconde opérande sur un `NaN` ou une égalité (donc `0.0` contre
  `-0.0` rend la seconde), `fmax` rend le `NaN`. Ici : une comparaison, puis `select`.
- Les masques. AVX2 lit le bit de signe de chaque voie (`blendv`, `movemask`), et `sign_mask` rend
  la valeur elle-même comme masque. NEON veut des voies toutes à un ou toutes à zéro. Un `Mask`
  NEON est toujours une voie entière, ce qui satisfait tous les usages de l'autre.
- La valeur entière d'un `f64` entier. AVX2 n'a pas de conversion `f64` vers entier 64 bits et
  passe par l'astuce de `1,5 * 2^52`. NEON convertit directement (`fcvtzs`) : sur un entier exact,
  c'est le même résultat.

**Fonctions mathématiques propres, pas `libmvec` ni Accelerate.** Le plan de
`docs/PORTING-batch-path.md` était d'appeler `libmvec` AdvSIMD sous Linux et Accelerate sous macOS.
Ce n'est pas ce qui est fait : le même code sert les deux architectures, sans glibc 2.38, sans
dépendance à macOS, avec une erreur connue (1 à 3 ULP de la libm, `libmvec` jusqu'à 4) et les mêmes
bits partout.

### Vérification

Faite sous `qemu-aarch64` (compilation croisée vers `aarch64-unknown-linux-gnu`, glibc aarch64),
puis par la CI sur Apple silicon et sur Linux ARM :

- `bits_are_the_same_on_every_architecture` (`math.rs`) : une empreinte par fonction (`sin_cos`,
  `tan`, `exp`, `log`, `atan`, `asinh`, `atanh`, `sinh`, `atan2`, `hypot`, `powf`), mesurée sur le
  chemin AVX2. Ce sont celles de NEON. Les entrées sont construites sans `ln` ni `exp` : avec la
  libm de macOS elles différaient, et l'empreinte aussi (trouvé sur Apple silicon).
- `kernels_keep_their_bits_on_every_architecture` (`kernel_bits_tests.rs`) : l'aller et le retour de
  `laea` équatorial, `somerc`, `sterea` (ellipsoïde et sphère), `cart` (géodésique et
  géocentrique) et `helmert` (deux conventions), sur 4 000 points chacun, tous dans le domaine : un
  groupe qui sortirait du domaine passerait par le scalaire, donc par la libm de la plateforme.
  Mêmes bits que sur AVX2, sur glibc seulement (Linux x86-64 et Linux ARM) : la mise en place des
  projections appelle la libm de la plateforme, et macOS comme Windows donnent d'autres constantes
  (`sterea` l'a montré). Sur Apple silicon, la preuve est celle des 11 fonctions de `math.rs`, qui
  n'appellent aucune libm. `laea` oblique n'y est pas : sa mise en place appelle le `sin` et
  l'`atan` de la libm, qui donnent d'autres constantes sur Apple silicon (l'empreinte différait).
  Par la même cause, le test sterea du monde entier tolère 2e-11 en relatif, à des milliers de
  kilomètres, contre le scalaire.
- `assert_vector_taken` : sur NEON aussi, au moins une valeur de chaque noyau diffère du scalaire
  au dernier bit, ce qui prouve que le chemin vectoriel a tourné. Et les tolérances de
  `docs/PARITY.md` tiennent.
- `the_vector_path_is_taken_on_an_aarch64_processor` : `batch_path()` rend `neon`. Ce test a été
  écrit d'abord, et échouait.
- Les tests de précision de `math.rs` contre la libm (1 à 3 ULP) passent aussi sur NEON.
- Les 28 empreintes de `tests/simd_bits.rs` et celles de `tests/convert_array_golden.rs` sont celles
  de la libm glibc de x86-64 pour les groupes de repli : ces tests restent réservés à x86-64. Sur
  NEON, 27 des 28 empreintes de `simd_bits` sont les mêmes, et la dernière (le retour
  `EPSG:4326` vers `EPSG:3035`) diffère, par du code scalaire qui appelle la libm : le noyau
  vectoriel de `laea` n'est pas appelé dans ce couple.

Ce qui reste connu : `fmath::tests::matches_the_c_libm_bit_for_bit` échoue sous `qemu` sur Linux
aarch64, avec et sans le chemin NEON. Il compare l'`atanh` de `fmath` à celui de la libm de la
cible.

## AVX-512 (x86-64, feature `avx512`)

`v8.rs` ajoute une troisième réalisation de `Vector`, à huit voies dans un registre `__m512d`, derrière
la feature cargo `avx512` (off par défaut, relayée par `proj-rs-georust`). Le jeton est
`pulp::x86::V4` (`V4::try_new` : le niveau x86-64-v4, AVX512F/BW/CD/DQ/VL plus le niveau v3), et
les intrinsèques passent par les méthodes sûres de `pulp` comme pour AVX2 : pas de `unsafe` dans
le dépôt. `batch_path()` rend `avx512` quand le processeur l'a et la feature est compilée, sinon
`avx2` ; `run` essaie AVX-512 d'abord, puis AVX2, puis NEON, puis le scalaire.

La feature est off par défaut pour le plancher 1.83 : les types AVX-512 de `std::arch` sont
stables depuis Rust 1.89, et la feature `x86-v4` de `pulp` ne compile pas sous 1.83. Le job
`msrv` de la CI liste donc les features explicitement, sans `avx512`. Construire avec
`--features avx512` exige Rust >= 1.89.

Chaque opération rend les bits de son homologue AVX2 voie par voie (IEEE 754, FMA fusionné) ;
seuls les masques diffèrent de forme (registres de bits `__mmask8` : `vblendmpd`, `vpmovq2m`,
les comparaisons rendent le masque directement) et `round`/`floor` passent par `vrndscalepd`
(arrondi exact). Aux empreintes de `math.rs`, 9 fonctions sur 11 rendent les mêmes bits qu'à
4 voies ; `asinh` et `atanh` ont une branche « toutes les voies petites » dont le résultat dépend
de la composition du groupe, donc du nombre de voies (les deux restent dans les bornes d'ULP du
test d'exactitude).

### Mesures AVX-512 contre AVX2

Xeon 6985P-C (Emerald Rapids), `proj-rs-bench`, un million de points par cas, ns/point, petit
des deux passages : les lots par noyau vectoriel gagnent 1,1× à 1,8×, les appels point par point
ne changent pas.

| cas | AVX2 (4 voies) | AVX-512 (8 voies) | gain |
|---|---:|---:|---:|
| UTM 31 aller (lot) | 19,3 | 10,8 | 1,8× |
| UTM 31 retour (lot) | 23,7 | 14,3 | 1,7× |
| Lambert-93 aller (lot) | 18,8 | 11,3 | 1,7× |
| Lambert-93 retour (lot) | 45,4 | 31,1 | 1,5× |
| Web Mercator aller (lot) | 11,6 | 7,8 | 1,5× |
| Web Mercator retour (lot) | 9,0 | 5,7 | 1,6× |
| `convert_array` 4326 → 32631 | 26,6 | 20,1 | 1,3× |
| `convert_array` 4326 → 2154 | 27,7 | 24,8 | 1,1× |
| `convert_array` 4326 → 3857 | 19,9 | 16,1 | 1,2× |
| Helmert 7 (lot) | 9,0 | 8,9 | 1,0× |
| UTM 31 aller (point par point) | 76,1 | 75,8 | 1,0× |

## Mesures

`bench/record.sh`, un cœur épinglé (`taskset -c 2`), machine `cloud-4c-avx512` (le chemin
vectoriel est AVX2 ; l'AVX-512 n'est pas utilisé). Nanosecondes par point sur un million de
points. « Scalaire » est la boucle sur `forward` ou `inverse` du même run.

| cas | scalaire | lot avant | lot après | gain sur le scalaire |
|---|---:|---:|---:|---:|
| UTM 31 aller | 89,1 | 37,6 | 27,4 | 3,3× |
| Lambert-93 aller | 62,0 | 20,3 | 19,8 | 3,1× |
| UTM 31 retour | 107,9 | (scalaire) | 23,0 | 4,7× |
| Lambert-93 retour | 208,2 | (scalaire) | 59,1 | 3,5× |
| Web Mercator aller | 46,2 | (scalaire) | 11,1 | 4,2× |
| Web Mercator retour | 35,3 | (scalaire) | 6,0 | 5,9× |

Les deux lignes de `bench/history.jsonl` (`8cfd059`, puis le commit de ce lot) sont celles dont
viennent les colonnes « avant » et « après » : un run varie de quelques pour cent d'un lancement
à l'autre.

Le noyau Transverse Mercator aller ne calculait avant que les sinus et cosinus en vecteur : les
séries de Clenshaw, `asinh` et `adjlon` restaient voie par voie. Elles sont maintenant en
vecteur, d'où 37,6 → 27,4.

### Sur aarch64

Mesuré sur les machines de la CI, `main` (chemin portable) puis la branche NEON, tour à tour sur la
même machine, 7 passages, le plus petit temps, en nanosecondes par point : un Apple M1 (virtuel,
`macos-latest`) et un Neoverse N2 (`ubuntu-24.04-arm`, partie CPU `0xd49`). Une machine partagée :
l'écart d'un passage à l'autre peut atteindre quelques pour cent, et les cas que NEON ne touche pas
(`utm31_forward` à la pièce, `op_gridshift`) bougent d'autant.

| cas | M1 portable | M1 NEON | gain | N2 portable | N2 NEON | gain |
|---|---:|---:|---:|---:|---:|---:|
| UTM 31 aller (lot) | 44,5 | 26,9 | 1,7× | 71,5 | 45,6 | 1,6× |
| Lambert-93 aller (lot) | 58,5 | 28,2 | 2,1× | 65,7 | 41,6 | 1,6× |
| UTM 31 retour (lot) | 97,6 | 32,3 | 3,0× | 104,1 | 53,1 | 2,0× |
| Lambert-93 retour (lot) | 144,4 | 60,5 | 2,4× | 165,0 | 79,5 | 2,1× |
| Web Mercator aller (lot) | 35,3 | 14,9 | 2,4× | 49,1 | 27,5 | 1,8× |
| Web Mercator retour (lot) | 42,6 | 10,5 | 4,1× | 41,8 | 20,4 | 2,1× |
| `convert_array` 4326 → 2154 | 64,9 | 36,4 | 1,8× | 76,0 | 51,7 | 1,5× |
| `convert_array` 2154 → 32631 | 241,7 | 95,4 | 2,5× | 275,3 | 135,6 | 2,0× |

Les appels point par point (`forward`, `inverse`) ne changent pas : 1,00×.

## Ce qui n'est pas couvert

- `Proj::convert_array` et `project_array` utilisent ces lots pour Transverse Mercator exact,
  Lambert conique conforme, Mercator sans `+over` et Web Mercator (`docs/PORTING-batch-api.md`).
  `Proj::convert`,
  `Operation::forward` et les points un à un restent scalaires.
- Les autres projections (aea, eqc, stere, …) et Helmert n'ont pas de noyau vectoriel. Helmert
  fait 9 ns par point en scalaire.
- Hors x86-64 et aarch64 : boucle scalaire (`Tmerc` aller a un repli portable). Les fonctions de
  `math.rs` ne sont écrites que pour `V4` : une autre architecture demande une troisième
  réalisation de `V4`.
- Pas de mesure de vitesse sur Windows ni macOS : seulement le repli forcé sur Linux, voir plus haut.
  La CI compile et teste le chemin sur Windows à chaque PR.
- Il n'y a plus de dépendance à glibc (ni `libmvec`, ni la version 2.35 qu'exigeaient `asinh`, `sinh`,
  `atan`, `tan`, `hypot` et `pow`) : l'édition de liens n'a plus besoin de `-lmvec`.
