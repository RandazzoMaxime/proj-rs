# georust/proj contre proj-rs

georust/proj est le binding Rust de PROJ : la crate `proj`, que proj-rs veut remplacer en changeant
une ligne de `Cargo.toml`. Ce document mesure deux choses avec ses propres tests :

1. la crate `proj` elle-même, liée à notre `libproj` au lieu de celle de PROJ ;
2. ses tests et ses exemples écrits sur notre façade, comme le ferait un utilisateur qui écrit
   `proj = { package = "proj-rs", ... }`.

## Méthode

`tools/clients/georust.sh` clone georust/proj au tag `v0.31.0` (commit épinglé), la version dont
la façade reprend l'API.

- **Liée** : la crate est compilée une fois contre PROJ 9.9.0 (installé depuis la référence de
  `tools/build-reference.sh`). Ses tests unitaires et ses exemples de documentation tournent avec
  PROJ, puis avec notre `libproj` chargée à sa place par `LD_LIBRARY_PATH` (même soname, RUNPATH).
  Le script compare les résultats test par test.
- **Façade** : `tools/clients/georust_extract.py` copie les modules de test de `src/proj.rs` et
  `src/geo_types.rs`, et les exemples de documentation (`///`, `//!` et
  `cfg_attr(feature = "geo-types", doc = ...)`), dans une crate qui dépend de proj-rs sous le
  nom `proj`. `use super::*` y devient `use proj::*` et `use proj::wkt::*`, les chemins publics
  qu'un utilisateur écrit.

Les deux passes tournent hors réseau, avec le même dossier de données. Les tests de la
fonctionnalité `network` ne tournent pas, comme dans la configuration par défaut de georust.

## Résultat (georust/proj 0.31.0, le 2026-11-22)

| | Tests unitaires | Exemples de documentation |
|---|---:|---:|
| Liée à PROJ 9.9.0 | 26 sur 26 | 24 sur 24 |
| Liée à notre `libproj` | 26 sur 26, mêmes résultats | 24 sur 24, mêmes résultats |
| Façade proj-rs | 24 sur 26 (1 ignoré, 1 écart voulu) | 24 sur 24 |

Les deux tests de la façade qui ne passent pas :

- `test_wkt` construit `WktOptions` par ses champs privés. Ce code ne compile qu'à l'intérieur de
  georust : un utilisateur n'a que `WktOptions::default()`, chez georust comme chez proj-rs. Le
  test est ignoré, avec cette raison, et n'est pas compté comme réussi.
- `test_searchpath` est `#[should_panic]` chez georust, avec le commentaire « This failure is a bug
  in libproj » : `ProjBuilder::lib_info` appelle `proj_info()`, qui lit le contexte par défaut et
  ignore les chemins de `set_search_paths`. La façade donne les chemins du constructeur
  (`tests/proj_facade.rs`), donc `/foo` est bien le dernier chemin et le test ne panique pas. C'est
  un écart voulu : reproduire ce défaut n'apporterait rien à un utilisateur.

`proj-sys` 0.27 écrit le chemin de pkg-config entre guillemets (`{:?}`), si bien que rustc ne le
trouve pas hors de `/usr/lib`. Le script passe donc le chemin par `RUSTFLAGS`. Ce défaut est celui
de georust, il est sans rapport avec la bibliothèque testée.
