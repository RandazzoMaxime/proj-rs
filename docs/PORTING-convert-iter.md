# Conversion d'un itérateur par blocs (`Proj::convert_iter`)

Décision de l'opérateur (« go »). Fichiers : `src/proj.rs` (`Proj::convert_iter`),
`src/proj/convert_iter.rs` (l'itérateur), `tests/convert_iter.rs`.

## Pourquoi

`Proj::convert` transforme un point à la fois : environ 100 ns pour `EPSG:4326` vers `EPSG:2154`.
`Proj::convert_array` passe une tranche au moteur d'un bloc et atteint le SIMD : environ 45 ns par
point. Un appelant qui tient un itérateur, sa source étant une base, un fichier ou un flux, ne peut
pas écrire sa tranche sans la copier : il paie le point par point. `convert_iter` regroupe ses points
par blocs de 1024 et les passe par le chemin de `convert_array`.

## Contrat

- Les items sont ceux de `points.map(|p| proj.convert(p))`, dans le même ordre, un item par point.
- Une **valeur** concorde avec `convert` à la tolérance de `convert_array` (AVX2 : au plus 11 nm,
  `docs/PORTING-simd.md`). Elle n'est pas identique au bit près.
- Une **erreur** est celle que `convert` donne pour son point. L'itérateur continue après.
- Un bloc qui échoue est converti de nouveau point par point : `convert_array` laisse la tranche
  partiellement convertie (`docs/PORTING-memory.md`), donc on ne lit pas son résultat après un `Err`.
  Coût : un bloc fautif est converti deux fois, ce qui n'arrive que sur une erreur.
- Lecture à la demande : l'itérateur prend au plus 1024 points d'avance sur les items rendus. Une
  source sans fin reste utilisable (testé).
- Les coordonnées sont `Coord<F>` pour `F: CoordinateType` (`f64`, `f32`), comme `convert` : pas de
  contrainte `Clone` sur le type de point. Les coordonnées d'origine sont gardées en `(F, F)`.

## Mesures

Machine : 4 vCPU Xeon 2,1 GHz, un cœur (`taskset -c 2`), `--release`. 10 000 000 de points dans une
boîte de la France, meilleure de 2 passes, trois séries alternées (`point`, `iter`, `array`) :

| Chemin | ns par point (3 séries) |
|---|---:|
| `convert` point par point | 100, 111, 100 |
| `convert_iter` (blocs de 1024) | 58, 46, 59 |
| `convert_array` (tranche) | 48, 42, 41 |

La machine bouge d'une série à l'autre (la tranche valait 42 ns, puis 51 ns sur une autre série) :
seules les comparaisons alternées comptent. `convert_iter` va environ deux fois plus vite que
`convert`, et l'écart avec la tranche est dans le bruit de la machine.

Autres mesures, du même programme :

- **Taille du bloc** (tranche, 10 M de points, 256 et 4096 points par bloc) : 40,1 et 41,6 ns, soit
  −4 % au mieux. Le bloc le plus gros ne fait pas perdre de cache : le calcul limite, pas la mémoire
  (36,7 ns sur 4096 points tenus en cache, contre 39,6 ns sur 10 M). Le bloc de l'itérateur reste à
  1024 : 64, 256, 1024 et 4096 donnent le même résultat dans le bruit.
- **Copie de sécurité** d'un bloc (sans elle, pour mesurer seulement) : 49 à 52 ns sans, 47 à 58 avec.
  Le coût est dans le bruit.

## Profil du point par point (callgrind, instructions par point)

Run à 1 point retiré, pour ne compter que le point :

- `point` : ~819 instructions par point. Le calcul de Lambert-93 (`pow`, `sin`, `cos`, `exp`) pèse
  ~60 %. L'interpréteur des étapes (`Pipeline::forward` 82, `IsoPipeline::map` 56) et l'enveloppe de
  `convert` (59) pèsent environ un quart.
- `array` : ~287 instructions par point. Noyau vectoriel de Lambert-93 118, enveloppe batch de
  `Classic` 79, mise en forme des points de `fill_array` 49, boucle des étapes 24.

Pistes non faites, par ordre de gain attendu :

1. Enveloppe batch de `Classic` (79 instructions par point sur la tranche) : le plus gros surcoût du
   chemin tranche qui ne soit pas du calcul.
2. `fill_array` (49) : une première passe valide chaque coordonnée, une seconde les recopie. Une
   seule passe, ou un chemin sans conversion pour `f64`, retire une partie de ce coût.
3. Chemin point par point spécialisé pour les opérations simples (environ un quart du coût du point).

`sin_cos` n'est pas une piste : dans la bibliothèque standard de Rust, `f64::sin_cos` appelle
`sin` puis `cos`.

## Ce qui n'est pas couvert

- `Transformer` (`api/transformer.rs`) a son propre `forward_slice`, sans itérateur. Même idée
  applicable, non faite.
- `project_iter` (projection inverse) : même structure, non faite.
- Pas de chemin spécial pour une opération unique : le même code sert aux candidats.
