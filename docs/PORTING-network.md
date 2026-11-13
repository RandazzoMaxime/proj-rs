# Fiche de portage : réseau (test_network.cpp, projsync)

Audit détaillé : `docs/AUDIT-network.md`. Fiche du lot gie et réseau antérieur :
`docs/PORTING-gie-network.md`.

## Carte

| Source PROJ 9.9.0 | Rust |
|---|---|
| `src/filemanager.cpp` (téléchargement, cache), `src/networkfileapi` | `src/network.rs` (interne, `pub(crate)`) |
| `ProjBuilder::enable_network`, `set_url_endpoint`, `grid_cache_enable`, `set_user_writable_directory` | `src/proj.rs` (façade) |
| `test/unit/test_network.cpp` (26 TEST) | `tests/unit_network_local.rs` (2 cas), `tests/network_grid.rs` (existant) |
| serveur de test local (remplace `cdn.proj.org`) | `tests/net_support/mod.rs` |
| `src/apps/projsync.cpp`, `test/cli/test_projsync.sh` | non porté (voir l'audit) |

## Écarts

- Les 26 TEST() de `test_network.cpp` ne sont pas portés : 1 porté, 1 partiel,
  24 écarts. La liste et la cause de chacun sont dans `docs/AUDIT-network.md`.
- Pas de callbacks réseau (`proj_context_set_network_callbacks`) ni de
  `proj_context_set_fileapi` : la lecture de grille se fait par un `GET`
  complet, jamais par plages.
- Pas de cache SQLite (TTL, taille max, verrou) : le cache est un répertoire
  de fichiers.
- Construction des opérations eager : une grille manquante fait échouer
  `proj_create`, PROJ la résout au `proj_trans` avec
  `PROJ_ERR_OTHER_NETWORK_ERROR`.
- `projsync` : non commencé. Le binaire et son analyseur JSON de `files.geojson`
  restent à écrire.
- Le serveur local ne sert pas de `206`, ni de variations d'`ETag` ou de
  `Last-Modified`, car la crate n'en a pas besoin pour l'instant.

## Commandes

```
cargo test --test unit_network_local --test network_grid
```
