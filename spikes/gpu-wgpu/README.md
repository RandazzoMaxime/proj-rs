# Spike GPU : Web Mercator et UTM en WGSL (wgpu natif)

Jetable, hors du workspace : `proj-rs` n'a aucune dépendance `wgpu`. Ce dossier a son propre
`Cargo.toml` et dépend de `proj-rs` par chemin, pour la référence CPU.

## Ce que ça teste

Deux conversions parmi les plus utilisées au monde, dans les deux sens :

| noyau | algorithme (même que `src/merc.rs`, `src/tmerc.rs`) |
|---|---|
| EPSG:4326 ↔ EPSG:3857 | Web Mercator sphérique |
| EPSG:4326 ↔ EPSG:32631 | UTM 31N, Transverse Mercator « exact » (Poder/Engsager, ordre 6, coefficients de `src/projmath.rs`) |

Le noyau est écrit **une fois** (`src/kernels.wgsl`) avec des appels `ad`, `sb`, `ml`, `dv`, `ng`,
`sq` au lieu d'opérateurs. Deux préludes donnent le sens de ces appels :

- **`f64`** (`prelude_f64.wgsl`) : `f64` natif de WGSL. Demande `Features::SHADER_F64`
  (Vulkan, NVIDIA/AMD ; pas Metal, pas DX12, pas Intel Arc, pas le mobile). Buffers `f64`.
- **`df`** (`prelude_df.wgsl`) : double-float, deux `f32` par nombre (~48 bits de mantisse).
  Marche partout où wgpu marche, donc sur Mac et sur Windows.

`sin`, `cos`, `atan2`, `exp`, `log` n'existent pas en `f64` dans les shaders : naga les accepte,
puis le pilote renvoie du faux sans erreur (`sin(2.0lf)` donne 0 sur lavapipe). Elles sont donc
réécrites dans `kernels.wgsl` à partir de `+ - * /`, `floor` et `sqrt`, avec les polynômes générés
par `src/shaders.rs`. Pour `log` et `atan2`, le matériel f32 donne une estimation, corrigée
ensuite à pleine précision (Halley pour `log`, rotation exacte pour `atan2`).

## Pièges relevés

1. **Fast-math.** Un pilote qui réassocie replie `(a+b)-a` en `b` et détruit les termes d'erreur du
   double-float. Mesa/lavapipe le fait. Metal le fait par défaut, et wgpu 30 n'expose aucun
   réglage (seul `preserveInvariance` est activé). Parade : la fonction `bar` de
   `prelude_df.wgsl` (un `bitcast` et un ET avec un masque lu à l'exécution) rend la valeur opaque
   au compilateur. Le programme vérifie au démarrage que `two_sum` et `two_prod` sont exacts
   (`Contrôle double-float`). **S'il affiche ÉCHEC, le chemin df ne vaut rien sur cet appareil.**
2. **`fma`.** WGSL ne dit pas si `fma` est fusionnée : le produit exact passe par
   Veltkamp-Dekker, sans `fma`.
3. **`maxStorageBufferBindingSize`** vaut 128 Mio par défaut : calcul par blocs de 4 M points.

## Lancer

```bash
cd spikes/gpu-wgpu
cargo run --release                 # tailles 100 k, 1 M, 4 M points
cargo run --release -- --quick      # petit
cargo run --release -- --big        # jusqu'à 16 M points
cargo run --release -- --adapter apple   # filtre sur le nom de l'adaptateur
cargo run --release > resultat.md   # le suivi va sur stderr, le tableau sur stdout
```

Sans GPU (sandbox Linux) : `apt-get install mesa-vulkan-drivers`, puis
`VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json`. Lavapipe est un rendu logiciel :
précision valide, temps sans valeur.

La première compilation du crate `proj-rs` prend environ deux minutes.

## Lecture des tableaux

- **Précision** : écart contre `proj-rs` (PROJ 9.9.0) sur 200 000 points tirés dans l'emprise de la
  zone (lon 0 à 6°, lat −80 à 84° pour UTM ; monde entier, lat ±85° pour Mercator).
- **Noyau seul** : les données sont déjà sur le GPU. C'est le plafond de calcul.
- **Bout en bout** : conversion f64 vers paires f32, envoi, calcul, lecture, conversion inverse. C'est
  ce que paierait un appelant ; la colonne « dont CPU » isole la part de conversion et de copie.
- **CPU** : `Proj::convert`, `convert_array` (SIMD AVX2 si présent) et `convert_array` sur tous les
  cœurs (un `Proj` par thread, créé avant la mesure).

## Ce que le spike ne couvre pas

- Une seule zone UTM (31). Le noyau lit `lon0` dans les paramètres ; les autres zones n'ont pas
  été testées.
- Hors domaine : pas de `NaN` ni d'erreur par point, contrairement au CPU. Les points tirés
  restent dans le domaine.
- Pas de grilles, pas de Helmert, pas de pipeline : deux projections seulement.
- Pas de requête d'horodatage GPU : les temps sont mesurés à l'horloge, `submit` plus `poll`.
