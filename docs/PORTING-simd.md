# Calcul par lots en AVX2 (`src/simd/`)

PROJ 9.9.0 n'a pas de chemin vectoriel : `pj_fwd` et `pj_inv` convertissent un point à la fois
(`proj_trans_generic` boucle). Ce module est donc un ajout de la crate, pas un portage. La
référence des valeurs reste PROJ : `tests/simd_batch.rs` compare chaque lot à `cs2cs` 9.9.0.

## Architecture

```
src/simd/
  mod.rs      Lanes (le contrat d'un noyau), run (le driver), batch_path
  v4.rs       Avx2 (preuve de support), V4 (quatre f64), Mask
  math.rs     sin_cos tan exp log atanh asinh sinh atan atan2 hypot powf sur V4, en Rust
  kernels.rs  briques partagées : adjlon, tsfn, phi2, latitude_in_domain
  drive.rs    le seul code compilé avec avx2,fma : la boucle par groupes de quatre
src/tmerc/batch.rs   Transverse Mercator : forward, inverse
src/lcc/batch.rs     Lambert conique conforme : forward, inverse
src/merc/batch.rs    Mercator, sphère et ellipsoïde : forward, inverse
```

Une projection écrit sa formule contre l'API sûre de `V4` (opérateurs, `select`, `sin`, …). Elle
n'écrit ni intrinsèque, ni `unsafe`, ni détection du processeur. Pour ajouter une projection :
implémenter `Lanes` (`point` : le chemin scalaire ; `lanes` : quatre points, ou `None`), puis
appeler `simd::run` depuis `forward_batch` / `inverse_batch`.

### Le modèle de sûreté

- `Avx2` ne s'obtient que par `Avx2::detect`, qui interroge le processeur (AVX2 et FMA).
- Un `V4` ou un `Mask` n'existe qu'à partir d'un `Avx2`. Leurs méthodes appellent donc des
  intrinsèques que le processeur supporte : l'API est sûre, et chaque bloc `unsafe` de `v4.rs`
  cite cette preuve.
- Le débit dépend d'une autre condition, qui n'est pas de sûreté : le code doit être inséré
  dans une fonction `target_feature(enable = "avx2,fma")`. C'est `drive::groups`, la seule du
  module. Les méthodes de `V4` et les noyaux sont `#[inline(always)]`. Hors de cette fonction
  le code reste correct, mais les intrinsèques ne sont plus en ligne.
- Il n'y a ni `asm!` ni appel à la bibliothèque C : les fonctions mathématiques de `math.rs` sont du
  Rust écrit contre l'API sûre de `V4`.
- **Une fermeture n'est pas insérée en ligne comme le reste.** Elle n'hérite pas du
  `target_feature` de la fonction qui l'appelle, et LLVM ne l'insère pas toujours : ses
  intrinsèques deviennent alors des appels. Un `sinh` écrit avec deux fermetures coûtait 290 ns
  par appel de quatre voies au lieu de 9,5. Le code de `math.rs` et des noyaux utilise donc des
  fonctions `#[inline(always)]` de module, jamais une fermeture qui calcule avec des `V4` (la
  petite fermeture `k` qui diffuse une constante est insérée sans problème).

Le `unsafe` de la crate pour le calcul vectoriel est dans `src/simd/` et nulle part ailleurs.
Avant le lot du module, chaque projection portait ses intrinsèques et sa détection (`simd.rs`,
`tmerc.rs` et `lcc.rs`) ; avant le lot des fonctions mathématiques, `libmvec` passait par `asm!`.

### Le driver et le repli

`run` découpe les tranches par groupes de quatre. Pour chaque groupe, `Lanes::lanes` rend les
quatre résultats, ou `None` quand une voie sort du domaine que le noyau vectoriel traite
(valeur non finie, latitude hors de ±90°, pôle d'un cône, `rho` nul, `|Ce|` au-delà de la limite
de Transverse Mercator, latitude non finie à l'inverse d'un cône). Le groupe passe alors par
`Lanes::point`, le chemin scalaire : ses quatre points valent ceux de `forward` et `inverse` **au
bit près**, et un point refusé est écrit `NaN`. La queue (moins de quatre points) passe aussi par
`point`. Sans AVX2, ou hors x86-64, `run` boucle sur `point` : les résultats sont ceux du chemin
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
Linux, Windows et macOS. Les algorithmes sont ceux de fdlibm (la libm de Sun, reprise par FreeBSD et
musl) : réduction d'argument de Cody et Waite en trois morceaux de `pi/2` (précision relative du
reste près d'un multiple de `pi/2`), noyaux polynomiaux de Remez de `__kernel_sin` et
`__kernel_cos`, quatre plages de réduction pour `atan`, `log` par `log(1+f)` avec `s = f/(2+f)`,
sélections par masque à la place des branches. Les coefficients sont ceux de fdlibm, recopiés de
mémoire : c'est le banc de précision qui les valide.

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
| `sin`, `cos`, `exp`, `log`, `atan`, `atan2`, `hypot`, `powf` | 1 |
| `asinh`, `atanh`, `sinh` | 2 |
| `tan` (quotient de deux fonctions à 1 ULP) | 3 |

Les bornes des tests sont ces mesures plus un ; `libmvec` annonce jusqu'à 4 ULP. Un second test,
`special_values_match_the_libm`, compare `NaN`, infinis, zéros signés, négatifs, sous-normaux et
dépassements à la libm, pour les fonctions à un argument et toutes les paires de `atan2`, `hypot` et
`powf`. La référence de `atanh` est symétrisée : `f64::atanh` de `std` vaut `0.5 * ln_1p(2x / (1 - x))`
et perd presque tous ses chiffres pour `x` proche de -1 (31 000 ULP mesurés), alors que ces
fonctions travaillent sur `|x|`.

### Vitesse

Un cœur épinglé, quatre exécutions alternées des trois binaires, minimum de chacun, ns par point.
« Voie par voie » est l'ancien repli de Windows et macOS (la libm appelée une voie à la fois, forcé
ici sur Linux : ce n'est donc pas la CRT de Microsoft) ; « `libmvec` » est le chemin de Linux avant
ce lot.

| cas | voie par voie | `libmvec` | `math.rs` | `math.rs` / voie par voie | `libmvec` / `math.rs` |
|---|---:|---:|---:|---:|---:|
| UTM 31 aller | 60,2 | 24,9 | 20,6 | 2,9× | 1,21× |
| Lambert-93 aller | 41,9 | 19,3 | 23,9 | 1,8× | 0,81× |
| UTM 31 retour | 71,0 | 21,8 | 26,0 | 2,7× | 0,84× |
| Lambert-93 retour | 97,3 | 56,9 | 52,7 | 1,8× | 1,08× |
| Web Mercator aller | 33,7 | 11,0 | 12,4 | 2,7× | 0,88× |
| Web Mercator retour | 30,7 | 6,2 | 11,6 | 2,6× | 0,53× |

- **Windows et macOS x86-64 :** 1,8 à 2,9× plus vite que le repli voie par voie. Je n'ai pas mesuré
  la CRT de Microsoft ni macOS : le chiffre est celui d'un repli forcé sur Linux.
- **Linux, face à `libmvec` :** à peu près neutre. Plus rapide sur UTM aller (1,21×) et Lambert-93
  retour (1,08×), plus lent sur Lambert-93 aller (0,81×), UTM retour (0,84×), Web Mercator aller
  (0,88×) et surtout Web Mercator retour (0,53×, de 6,2 à 11,6 ns : `sinh` puis `atan` sur des
  arguments qui ne sont pas petits, donc sans chemin rapide). Le but de ce lot est la portabilité et
  la précision, pas la vitesse sur Linux ; le coût est de 0 à 5 ns par point selon la projection.
- **Par fonction** (un appel de quatre voies, un cœur) : `sin_cos` 9 ns, `exp` 6,6, `log` 6,8, `atan`
  12 (3 sur le chemin rapide), `atan2` 15, `atanh` 13 (3 en série), `sinh` 9,5 (3 en série),
  `hypot` 3, `powf` 23.

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

## Ce qui n'est pas couvert

- `Proj::convert_array` et `project_array` utilisent ces lots pour Transverse Mercator exact,
  Lambert conique conforme et Mercator sans `+over` (`docs/PORTING-batch-api.md`). `Proj::convert`,
  `Operation::forward` et les points un à un restent scalaires.
- Les autres projections (aea, eqc, stere, …) et Helmert n'ont pas de noyau vectoriel. Helmert
  fait 9 ns par point en scalaire.
- Hors x86-64 : boucle scalaire. Pas de NEON (les fonctions de `math.rs` seraient la base d'une
  implémentation NEON de `V4`).
- Pas de mesure de vitesse sur Windows ni macOS : seulement le repli forcé sur Linux, voir plus haut.
  La CI compile et teste le chemin sur Windows à chaque PR.
- Il n'y a plus de dépendance à glibc (ni `libmvec`, ni la version 2.35 qu'exigeaient `asinh`, `sinh`,
  `atan`, `tan`, `hypot` et `pow`) : l'édition de liens n'a plus besoin de `-lmvec`.
