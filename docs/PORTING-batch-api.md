# La façade de conversion et les tranches (`convert_array`)

Ce lot ne porte pas de code PROJ : il mesure le coût de la couche de conversion de la crate, retire
deux surcoûts, et fait traverser une tranche de points au moteur d'un bloc, pour que le SIMD de
`docs/PORTING-simd.md` serve `Proj::convert_array`. PROJ convertit un point à la fois
(`proj_trans_generic` boucle sur `pj_fwd`) : l'ordre étape par étape sur une tranche n'a pas
d'équivalent dans PROJ.

## Ce que coûtait la façade

Un point par `Proj::convert` coûtait bien plus que la même opération appelée directement. Mesure
interleavée (variantes alternées, minimum de 60 passes, un cœur), `EPSG:4326` vers `EPSG:3857` :

| appel | ns par point |
|---|---:|
| `Operation::forward` | 42 |
| `Proj::new(chaîne).convert` | 79 |
| `Proj::new_known_crs(..).convert` | 93 |

Les instructions ne l'expliquaient pas (callgrind : 41 et 142 de plus sur 483), le temps oui :
un code de façade à faible débit d'instructions. Deux causes, trouvées par élimination :

1. **La fermeture d'erreur de `map_xyzt`.** Elle formate une `String` (`to_string`) et était insérée
   dans la fonction chaude. Elle passe dans `Proj::facade_error`, `#[cold]` et `#[inline(never)]`,
   et la chaîne `map_point`, `map_f64`, `map_xyz`, `map_xyzt` est `#[inline]`. `Proj::convert`
   natif : 79 → 43 ns, à 1,4 ns d'`Operation::forward`.
2. **`AxisSwap::permute`.** Une boucle indexée sur un tableau de quatre valeurs ; tout CRS qui écrit
   la latitude d'abord en porte un (et la façade en ajoute un à son entrée). `order=2,1` prend
   maintenant un échange direct. Façade ISO : 89 → 49 ns.

Je n'ai pas isolé la part de chaque cause au-delà de ces deux mesures, ni expliqué au niveau du
processeur pourquoi la fermeture coûtait autant (`perf` n'est pas installé ici). Le résultat tient
sur les mesures.

Après les deux corrections la façade ne coûte plus que 3 à 6 ns de plus que l'opération nue
(`EPSG:4326` vers `EPSG:2154` : 78,9 contre 76,1 ns ; `EPSG:2154` vers `EPSG:32631` : 327 contre
321 ns). Le reste est le calcul des projections.

## Le contrat de lot

`Step::forward_batch` et `Step::inverse_batch` (`engine.rs`) transforment une tranche de `Coord` sur
place, avec un emplacement `Option<TransformError>` par point :

- un point refusé garde son entrée et reçoit l'erreur dans `failed` ;
- un point déjà en échec est ignoré par les étapes suivantes : l'erreur d'un point est celle de la
  première étape qui le refuse, comme quand les points passent un par un ;
- le défaut est la boucle sur `forward` ou `inverse` : une étape sans noyau vectoriel est inchangée.

| couche | ce qu'elle fait d'une tranche |
|---|---|
| `Pipeline` | étape par étape sur toute la tranche, pour que `Classic` voie tous les points ; point par point si le pipeline garde une pile (`push`, `pop`), dont l'état est par point |
| `Classic` | par blocs de 256 points : `fwd_prepare` en scalaire (porte de latitude, `pm=`, `lon_0`, `adjlon`, latitude géocentrique), le noyau vectoriel à origine nulle (`ProjectedBatch`), puis `fwd_finalize` en scalaire (`a`, faux est et nord, unités) ; un point que le noyau refuse (`NaN`), ou une entrée `NaN`, repasse par le scalaire ; pas de vecteur sous `+over` (`projections/classic_batch.rs`) |
| `IsoPipeline::map_batch` | une opération unique : la tranche d'un bloc, entre les deux échanges d'axes de la façade ; plusieurs candidats : un point à la fois (`proj_trans` choisit par point) |
| `Proj::fill_array` | des blocs de 4096 points ; le premier point en échec du premier bloc en échec rend son erreur |

Les projections qui ont un noyau vectoriel : Transverse Mercator exact (donc `+proj=utm` et
`+proj=tmerc` par défaut), Lambert conique conforme, Mercator sans `+over`, et `+proj=webmerc`
(le noyau sphérique de Mercator, à rayon `a`, `k0 = 1`). `webmerc` n'est pas un `Classic` avec
lot : `Classic::with_batch` reste privé à `engine`, donc `WebMercStep` (`projections/webmerc_batch.rs`)
enveloppe l'étape scalaire et ne remplace que `forward_batch` et `inverse_batch`. Sous `+over`, pas
de vecteur, comme `Classic`.

### Un point refusé par le noyau repasse par le scalaire

Le noyau écrit `NaN` pour un point qu'il refuse (hors domaine, non fini). Sa raison n'est pas dite :
`Classic` rappelle `forward` ou `inverse` pour ce point et prend son résultat. L'erreur d'un
point en échec est donc celle du chemin scalaire, au même texte (`tests/convert_array_batch.rs`).

Depuis la fusion avec `main`, Transverse Mercator, Lambert et Mercator sont des étapes `Classic`
(préparation et finalisation de PROJ autour d'un noyau à origine nulle) : le noyau vectoriel est
construit à la même origine nulle, et seule la boucle du noyau change de chemin. Une entrée `NaN`
donne `NaN` comme `convert`, sans erreur, comme PROJ (`a_nan_point_gives_what_convert_gives`).

## Ce qui change pour l'utilisateur

- `convert_array` et `project_array` s'accordent avec `convert` et `project` à 1e-8 m près pour une
  projection qui a un noyau vectoriel (11 nm au plus mesuré, `docs/PORTING-simd.md`), aux mêmes
  bits sinon. Les tests prennent 1e-7 m et 1e-11°.
- Un point en échec arrête l'appel à la fin de son bloc de 4096 points, pas à la fin de la tranche
  ni au point lui-même. Les points du bloc après le point en échec sont donc transformés : cela
  compte pour un téléchargement de grille ou le choix d'un candidat (`last_used_operation`). Avant,
  l'appel s'arrêtait au point. La tranche reste inchangée, et l'erreur est celle du premier point
  en échec.

## Mesures

`bench/record.sh`, un cœur épinglé, `cloud-4c-avx512`, nanosecondes par point sur 200 000 points.
« Origine » est le run fait avant les corrections (cas `facade_*` ajoutés au bench, pas encore
commités : ligne `dirty` de `bench/history.jsonl`). « Latence » est après les deux corrections,
« lots » après le contrat de lot (dernière ligne de l'historique).

| cas | origine | latence | lots |
|---|---:|---:|---:|
| `convert` 4326 vers 2154 | 125,2 | 83,7 | 83,5 |
| `convert_array` 4326 vers 2154 | 130,2 | 88,0 | 33,5 |
| `convert` 4326 vers 32631 | 145,9 | 109,6 | 108,8 |
| `convert_array` 4326 vers 32631 | 152,3 | 114,7 | 40,4 |
| `convert` 4326 vers 3857 | 108,0 | 55,0 | 51,5 |
| `convert_array` 4326 vers 3857 | 104,9 | 56,3 | 42,2 |
| `convert` 2154 vers 32631 | 348,2 | 347,2 | 341,8 |
| `convert_array` 2154 vers 32631 | 351,9 | 345,9 | 99,8 |
| `convert` 4267 vers 4326 | 341,3 | 307,8 | 267,7 |
| `convert_array` 4267 vers 4326 | 351,3 | 312,9 | 283,9 |

`convert_array`, origine contre lots : 3,9× (Lambert), 3,8× (UTM), 2,5× (Web Mercator), 3,5×
(Lambert vers UTM), 1,2× (NAD27 vers WGS 84). Un run varie de quelques pour cent d'un lancement à
l'autre ; sur cette machine partagée, un même appel est monté à deux fois son temps pendant
plusieurs secondes : les comparaisons ci-dessus alternent les variantes ou prennent le minimum.

Web Mercator (`webmerc`, `WebMercStep`), même machine, `convert_array` sur 1 000 000 points, minimum
de 15 passes, `taskset -c 3` : 4326 vers 3857 de 59 à 63 ns/pt avant, 27 ns après ; 3857 vers 4326
de 44 ns avant, 19 ns après. Instructions par point (callgrind, différence entre 100 000 et
300 000 points) : 412 et 399 avant, 272 et 213 après. Écart maximal entre le lot et le point par
point sur 300 000 points : 4,7e-9 m (4326 vers 3857) et 1,4e-14° (3857 vers 4326).

## Ce qui n'est pas fait

- **`EPSG:4267` vers `EPSG:4326` (NAD27 vers WGS 84)** a plusieurs candidats : `proj_trans` choisit
  par point (`Alternatives::transform`), et le lot ne peut pas le grouper sans changer ce choix.
  Le gain mesuré (351 → 284 ns) vient des deux corrections de latence seulement.
- `Operation` n'expose pas `forward_batch` publiquement : le lot sert `Proj::convert_array`
  et `project_array`. L'exposer demande de fixer le type de l'erreur par point.
- Pas de noyau vectoriel pour les projections coniques autres que Lambert, `stere`, Helmert
  (9 ns par point) et les grilles.
- Le chemin de lot pour la sélection de `cs2cs` (`Selection::Several`, `Alternatives::forward`)
  n'existe pas.
- La fusion des étapes triviales (`axisswap`, `unitconvert`) à la construction du pipeline : la
  mesure ne la justifie plus (une étape qui ne fait rien coûte 2,5 ns).
