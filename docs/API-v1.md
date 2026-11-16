# API v1 : trois façades, une surface stable

Statut : décidé (lot « API v1 »). Ce document fixe ce que la v1 promet, et ce qu'elle ne promet
pas. Il sert de cahier des charges aux lots qui l'implémentent.

## Le problème

Aujourd'hui, la racine du crate exporte tout : le modèle ISO 19111 (environ 120 types), le moteur
(`Operation`, `Xyzt`), les outils de test (`gie`), le CLI (`projinfo`, `projsync`) et la façade
georust (`Proj`, `ProjBuilder`). Tout ce qui est `pub` est couvert par semver. Publier la v1 en
l'état figerait des centaines de types internes, et la phase 2 (SIMD, GPU) ne pourrait plus rien
changer sans une v2.

## Les trois façades

| Façade | Pour qui | Où | Contrat |
|---|---|---|---|
| Native | les projets Rust neufs | racine de `proj-rs` | API Rust idiomatique, petite, stable |
| georust | les utilisateurs de `proj` 0.31 | crate `proj-rs-georust` (`lib name = "proj"`) | mêmes noms et signatures que `proj` 0.31 : changer une ligne de `Cargo.toml` suffit |
| C | les programmes hors Rust qui lient libproj | crate `proj-rs-capi` (`cdylib` + `staticlib`, `libproj`) | `proj.h` de PROJ 9.9.0, ABI comprise : relier sans recompiler |

Les trois appellent le même moteur. Aucune ne réimplémente une transformation.

### Native (racine de `proj-rs`)

```rust
let ctx = proj_rs::Context::default();             // réseau, chemins, cache : désactivés par défaut
let src = proj_rs::Crs::from_user_input(&ctx, "EPSG:4326")?;
let dst = proj_rs::Crs::from_user_input(&ctx, "EPSG:2154")?;
let t = proj_rs::Transformer::builder(&src, &dst)
    .normalize_axis_order()                         // longitude, latitude comme le SIG
    .build()?;
let p = t.forward(proj_rs::Point::new(2.35, 48.85))?;
t.forward_slice(&mut points)?;                      // lot : SIMD quand il existe
```

- `Context` : réseau, endpoint, chemins de recherche, répertoire utilisateur, cache. Clonable,
  `Send + Sync`.
- `Crs` : `from_user_input` (code, WKT, PROJJSON, chaîne PROJ), `to_wkt(WktVersion)`,
  `to_projjson()`, `to_proj_string()`, `name()`, `kind()`, `area_of_use()`.
- `Transformer` : `builder(&src, &dst)` puis `area_of_interest`, `only_best`, `allow_ballpark`,
  `normalize_axis_order`, `backend`, `build()`. Méthodes `forward`, `inverse`, `forward_slice`,
  `inverse_slice`, `operations()` (les candidats, comme `projinfo`), `last_used_operation()`.
- **Ordre des axes = celui de l'autorité**, comme PROJ (EPSG:4326 est latitude, longitude).
  `normalize_axis_order()` donne l'ordre SIG, comme `proj_normalize_for_visualization`.
- `Point { x, y, z, t }` : `f64`, `z` et `t` à 0 et `+inf` par défaut, comme `PJ_COORD`.
- `Backend` : `#[non_exhaustive] enum Backend { Auto, Scalar }`. `Auto` prend le meilleur chemin
  disponible (aujourd'hui AVX2 si présent). Les variantes `Simd` et `Gpu` s'ajouteront sans casser
  l'API, puisque l'enum est `#[non_exhaustive]`. Le trait des noyaux reste privé.
- `Error` : `#[non_exhaustive]`, avec un code d'erreur équivalent à `proj_errno`.
- Garantie : le résultat est celui de PROJ 9.9.0, à la tolérance de `docs/PARITY.md`. Le chemin
  par lots peut différer du scalaire de 11 nm au plus (`docs/PORTING-simd.md`).

### georust (`proj-rs-georust`)

Ré-exporte la façade qui existe aujourd'hui (`src/proj.rs` : `Proj`, `ProjBuilder`, `Coord`,
`Transform`, `ProjError`, `Area`, `wkt`, la fonctionnalité `geo-types`). Le crate s'appelle
`proj-rs-georust` et sa bibliothèque `proj` : un projet passe de `proj = "0.31"` à
`proj = { package = "proj-rs-georust", version = "1" }` sans toucher au code. Sa surface est
vérifiée contre celle de `proj` 0.31 par un test qui compile les exemples de la documentation de
`proj` 0.31.

### C (`proj-rs-capi`)

- Produit `libproj.so` / `libproj.dylib` / `proj.dll` et `libproj.a`, avec le `proj.h` de PROJ
  9.9.0 copié tel quel. Les noms et les signatures sont identiques ; la structure `PJ_COORD` et les
  constantes sont les mêmes.
- Couverture : les 294 fonctions de `proj.h`. Une fonction que le port ne sait pas honorer renvoie
  l'erreur de PROJ (`NULL` ou 0 et `proj_errno`), jamais un résultat faux.
- Preuve : les tests C de PROJ compilés contre notre bibliothèque. Ce sont `test_c_api.cpp` (152
  TEST, en ne gardant que ceux qui n'appellent que `proj.h`), `proj_context_test.cpp`,
  `proj_errno_string_test.cpp`, `proj_angular_io_test.cpp`, `include_proj_h_from_c.c` et
  `test_fork.c`. Leur liste filtrée est dans `capi/tests/README.md`, avec la raison de chaque
  exclusion.

## Ce qui n'est pas stable

Ces modules restent accessibles, parce que les binaires et les tests en dépendent, mais ils sont
`#[doc(hidden)]` et hors semver : `iso19111`, `gie`, `projinfo`, `projsync`, `listing`, `info`,
`errno`, `names`, `math`, le moteur (`Operation`, `Xyzt`, `CrsPipeline`) et les projections
nommées (`Tmerc`, `Lcc`, `Mercator`). La fonctionnalité `unstable` les rend visibles dans la
documentation, sous `proj_rs::unstable`, pour qui accepte qu'ils changent.

## Outils de stabilité

- `cargo-semver-checks` en CI sur `proj-rs` et `proj-rs-georust`, contre la dernière version
  publiée (à partir de la v1).
- MSRV déclarée (`rust-version`) et vérifiée par un job CI.
- `#![deny(missing_docs)]` sur la surface stable ; chaque item public a un exemple compilé.

## Ordre des lots

1. Workspace, façade native, masquage des internes, crate georust, semver-checks et MSRV en CI.
2. Crate C : l'ABI de `proj.h`, par groupes de fonctions (contexte et erreurs, création et
   transformation, objets ISO 19111, fabriques et opérations, grilles et réseau), chaque groupe
   prouvé par les tests C de PROJ qui le couvrent.
3. Publication : versions `1.0.0`, `publish = true`, noms vérifiés libres sur crates.io.
