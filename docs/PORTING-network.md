# Fiche de portage : réseau (test_network.cpp, projsync)

Audit détaillé : `docs/AUDIT-network.md`. Fiche du lot gie et réseau antérieur :
`docs/PORTING-gie-network.md`.

## Carte

| Source PROJ 9.9.0 | Rust |
|---|---|
| `src/filemanager.cpp` (téléchargement, cache), `src/networkfileapi` | `src/network.rs` (interne, `pub(crate)`) |
| `proj_context_set_network_callbacks` | `NetworkCallbacks` (trait public), `ProjBuilder::set_network_callbacks`, `src/network.rs` |
| `proj_context_set_url_endpoint`, `PROJ_NETWORK_ENDPOINT` | `ProjBuilder::set_url_endpoint`, `GridContext::load` |
| `grids.cpp:3222` (`pj_generic_grid_init`), `grids.cpp:3234` | `src/projections/gridshift/mod.rs` (`deferred`, `Deferred`) |
| `trans.cpp:284` (erreur réseau d'un candidat : arrêt du point) | `src/iso19111/alternatives.rs` (`AlternativeError::Network`) |
| `proj_trans_get_last_used_operation`, `proj_pj_info` | `Proj::last_used_operation`, `Proj::proj_info` (`candidate_name`) |
| `DatabaseContext::lookForGridInfo` (`factory.cpp:3400`) | `src/iso19111/factory.rs` (`look_for_grid_info`), `src/iso19111/select.rs` (`unrecorded_grid`) |
| `src/apps/gie.cpp` (`network_error`) | `src/gie.rs` (`errno_name`) |
| `test/unit/test_network.cpp` (26 TEST) | `tests/unit_network_core.rs` (5 cas), `tests/unit_network_endpoint_env.rs` (1 cas), `tests/unit_network_local.rs` (2 cas), `src/network.rs` `callback_tests` (2 tests hors liste) |
| serveur de test local (remplace `cdn.proj.org`) | `tests/net_support/mod.rs` |
| `src/apps/projsync.cpp`, `test/cli/test_projsync.sh` | non porté (voir l'audit) |

## Décisions et écarts volontaires

- **Erreur réseau.** PROJ renvoie `HUGE_VAL` et `proj_errno == PROJ_ERR_OTHER_NETWORK_ERROR`.
  Ici, `Proj::convert_*` et `project` renvoient `Err(ProjError::Network)`, c'est-à-dire une
  valeur typée au lieu d'un sentinelle et d'un errno global. `TransformError::NetworkUnavailable`
  est le type interne, traduit en `ProjError::Network` au niveau de `Proj`.
- **Construction paresseuse, seulement pour les candidats.** PROJ crée une opération dont
  une grille nommée manque et est téléchargeable, puis retente le téléchargement à chaque
  transformation. Ici, cela ne vaut que pour les candidats d'une paire de CRS
  (`grids_may_be_deferred`, activé par `with_deferred_grids` dans `select.rs`). Une chaîne
  `+proj=...` explicite échoue à la création, comme la référence (`proj_create` renvoie
  `NULL` quand le CDN ne donne pas la grille). Justification : la référence mesurée
  (sonde C contre `libproj` 9.9.0 de `target/proj-reference`, non versionnée) donne `NULL` pour `proj=vgridshift grids=nz_...`.
  La règle est donc celle du mécanisme des alternatives (`trans.cpp:284`), pas celle du
  nom de fichier. Le placeholder `grids=null` donne les axes et l'ellipsoïde avant tout
  chargement.
- **Pas de nouvelle tentative sur erreur réseau.** Un candidat en erreur réseau arrête le
  point (`trans.cpp:284`) ; la boucle de nouvelles tentatives ne passe pas au candidat sans
  grille, qui donnerait une identité silencieuse.
- **Callbacks : grille entière par plages, pas paresseuse.** `fetch_with_callbacks` lit la
  grille entière (`open(url, 0, 16384)`, puis `read_range` par 16384 octets jusqu'à la taille
  de `Content-Range`, puis `close`). PROJ lit les tuiles au calcul, ce qui fixe la séquence
  d'événements des cas `custom`, `getfilesize`, `simul_*`. Ces cas restent écarts (voir
  l'audit) : sans lecture paresseuse des grilles, la séquence ne peut pas coïncider.
- **Forme du trait.** `open` renvoie les premiers octets avec un handle (`OpenReply`), au lieu
  d'un tampon et d'une chaîne d'erreur passés en paramètres. Les erreurs sont typées
  (`NetworkError::Open`, `ReadRange`). Les en-têtes passent par `header`, avec une valeur
  par défaut `None`.
- **Dernier candidat.** `last_used_operation` porte le nom du candidat choisi au dernier
  point, qui est la description de `proj_pj_info` en PROJ.

## Ce qui reste ouvert

- Lecture paresseuse par plages des grilles (la grille est lue entière, `read_resource` et
  `gridpath::read`). Bloque `custom`, `getfilesize`, `simul_read_range_error`,
  `simul_file_change_while_opened`.
- Validation de cache par `ETag` et `Last-Modified`.
- Cache SQLite : TTL, taille max, verrou, `proj_grid_cache_clear` (lot cache, autre agent).
- API de téléchargement explicite (`is_download_needed`, `download_file`) et API de fichiers
  (`file_api`).
- Façade des opérations (`proj_create_operations`, contexte de fabrique) : bloque
  `pyproj_issue_1192` et `do_not_attempt_network_access_known_available_network_on`.
- `projsync` : non commencé.
- `basic` (partie « on ») : `fr_ign_ntf_r93.tif` absent du dépôt et de la référence.

## Commandes

```
cargo test --test unit_network_core --test unit_network_endpoint_env --test unit_network_local --test network_grid
cargo test --lib callback_tests
cargo +1.100.0 clippy --all-targets --all-features -- -D warnings
```
