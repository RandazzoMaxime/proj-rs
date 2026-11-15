# Calcul par lots en AVX2 (`src/simd/`)

PROJ 9.9.0 n'a pas de chemin vectoriel : `pj_fwd` et `pj_inv` convertissent un point à la fois
(`proj_trans_generic` boucle). Ce module est donc un ajout de la crate, pas un portage. La
référence des valeurs reste PROJ : `tests/simd_batch.rs` compare chaque lot à `cs2cs` 9.9.0.

## Architecture

```
src/simd/
  mod.rs      Lanes (le contrat d'un noyau), run (le driver), batch_path
  v4.rs       Avx2 (preuve de support), V4 (quatre f64), Mask
  math.rs     sin cos tan exp log atanh asinh sinh atan atan2 hypot powf sur V4
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
- Les appels `libmvec` passent par `asm!` sur des registres `ymm`, qui exige la fonctionnalité
  `avx` de la fonction englobante : chacun est une petite fonction `target_feature` interne à sa
  méthode (`math.rs`).

Le `unsafe` de la crate pour le calcul vectoriel est dans `src/simd/` et nulle part ailleurs.
Avant ce lot, chaque projection portait ses intrinsèques et sa détection (`simd.rs`,
`tmerc.rs` et `lcc.rs`).

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
sur 200 000 points par projection (`tests/simd_batch.rs`, sonde retirée après la mesure) :

| projection | aller (m) | retour (degré) |
|---|---:|---:|
| UTM 31 / `tmerc` | 3,7e-9 | 2,8e-14 |
| Lambert-93 | 8,4e-9 | 4,3e-14 |
| Lambert, cône sud | 1,1e-8 | 4,6e-14 |
| Web Mercator | 5,6e-9 | 1,4e-14 |
| Mercator ellipsoïdal | 7,5e-9 | 2,8e-14 |

Les bornes des tests sont 1e-7 m et 1e-12° (marge de dix à vingt). Près du pôle opposé à un
cône, les coordonnées dépassent 1e10 m, où un ULP dépasse la borne absolue : le test admet en
plus 4e-15 relatifs.

Contre `cs2cs` 9.9.0 (2 003 points par projection, aller puis retour) : moins de 1e-6 m et
1e-9°, la précision de l'impression de `cs2cs` (`-d 9` et `-d 11`).

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
- Hors Linux avec glibc, les fonctions mathématiques sont celles de la bibliothèque C, une voie
  à la fois : seule l'arithmétique reste vectorielle. `batch_path()` rend `avx2` dans ce cas
  aussi. Ce chemin a été vérifié une fois en forçant le repli sur Linux (compilation, clippy et
  `tests/simd_batch.rs`, `cs2cs` compris) ; la CI Windows le compile et le teste à chaque PR, mais
  aucune mesure de vitesse n'a été faite sur Windows.
- Hors x86-64 : boucle scalaire. Pas de NEON.
- `libmvec` : `asinh`, `sinh`, `atan`, `tan`, `hypot` et `pow` demandent glibc 2.35 (Ubuntu
  22.04), comme `atanh` avant ce lot.
