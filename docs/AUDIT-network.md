# Audit du lot network

Référence : PROJ 9.9.0, `test/unit/test_network.cpp` (26 `TEST(networking, ...)`,
dont la plupart sous `#ifdef CURL_ENABLED`), `src/apps/projsync.cpp`,
`test/cli/test_projsync.sh`.

Légende : **porté** = assertions C++ reprises ; **partiel** = une partie
portée, le reste listé ; **écart** = pas de test, cause exacte donnée.

Le port actuel de la couche réseau est `src/network.rs` (`pub(crate)`) et la
façade `src/proj.rs` (`ProjBuilder`). Ce qui existe : activation du réseau
(`enable_network`, `PROJ_NETWORK`, `proj.ini`), `cdn_endpoint` et
`PROJ_NETWORK_ENDPOINT`, téléchargement d'une grille entière par `GET` simple
(`http_get`, redirections suivies, pas de `Range`), cache en fichiers dans le
répertoire utilisateur (`grid_cache_enable`, `set_user_writable_directory`),
et `set_file_finder`. Ce qui manque : callbacks réseau, requêtes par plages,
cache SQLite (TTL, taille max, verrou), API de téléchargement explicite,
résolution paresseuse des grilles au `proj_trans`.

Tests de l'API existants, hors liste C++ : `tests/network_grid.rs`
(téléchargement dans le cache, réseau off, chemin de recherche).

## Cas portés dans ce lot

| TEST() C++ | Test Rust | Statut |
|---|---|---|
| `networking.basic` (réseau off par défaut, pipeline nul, aucune requête) | `tests/unit_network_local.rs::basic_network_is_off_by_default_and_makes_no_request` | partiel : la partie « réseau off » est portée ; la partie « activé par `PROJ_NETWORK=ON` puis par l'API » (valeurs `1.9992776848`) ne l'est pas, voir plus bas |
| `networking.curl_invalid_resource` | `tests/unit_network_local.rs::curl_invalid_resource` | porté sur serveur local : 404, `proj` renvoie une erreur, une seule requête |

## Tableau complet des 26 TEST()

| # | TEST() C++ | Statut | Cause exacte si écart |
|---|---|---|---|
| 1 | `initial_check` | écart | Sonde réseau : `GET` avec `Range: 0-1` sur `cdn.proj.org`, attend `206`. La crate ne fait que des `GET` complets (`http_exchange`, `src/network.rs`) : pas de `Range`. |
| 2 | `basic` | partiel | Partie « off » portée (ligne 1 de la table). Partie « on » non portée : les valeurs `1.9992776848` / `48.9999322600` viennent de `fr_ign_ntf_r93.tif`, absent du dépôt, et aucune valeur de référence n'a pu être produite sur un fixture local (voir `curl_hgridshift`). |
| 3 | `curl_invalid_resource` | porté | Voir ci-dessus. |
| 4 | `custom` | écart | Exige `proj_context_set_network_callbacks` (open, close, get_header_value, read_range) : aucune API publique. La séquence exacte d'événements (ouverture à `offset 0 / 16384`, en-têtes `Content-Range`, `Last-Modified`, `ETag`, fermeture, puis lecture à `offset 524288`) suppose une couche de blocs de PROJ qui n'existe pas en Rust. |
| 5 | `getfilesize` | écart | Mêmes callbacks (`proj_context_set_network_callbacks`), fichier `test_vgrid_single_strip_truncated.tif` servi tronqué. |
| 6 | `simul_open_error` | écart | Callbacks : l'erreur d'ouverture est renvoyée par le callback (`error_string`), la crate n'a pas de chemin d'erreur de transport à exposer. |
| 7 | `simul_read_range_error` | écart | Callbacks : erreur de `read_range`, pas de lecture par plages dans la crate. |
| 8 | `simul_file_change_while_opened` | écart | Callbacks + changement d'`ETag` / `Last-Modified` entre deux lectures : la crate ne lit ni n'envoie ces en-têtes ; il faudrait la lecture par plages et la validation de cache. Le serveur local (`tests/net_support`) ne sait pas encore servir des `206` ni faire varier les en-têtes. |
| 9 | `curl_hgridshift` | écart | Grille CDN `fr_ign_gr3df97a.tif` absente du dépôt. Valeurs de référence : tentées avec le `proj` de référence sur `tests/gie/tests/test_hgrid_with_subgrid.tif` (et `test_hgrid_degree.tif`), refusées avec `could not find required grid(s)` quel que soit le chemin (absolu, ou via `PROJ_DATA`). Cause de ce refus non élucidée ; sans valeur PROJ, pas de port. |
| 10 | `curl_vgridshift` | écart | Même cause que 9 (`egm08_25.tif`, absente ; pas de fixture de référence chargée par `proj`). |
| 11 | `curl_vgridshift_vertcon` | écart | Même cause que 9 (`vertcone.tif`, absente). |
| 12 | `network_endpoint_env_variable` | écart | `PROJ_NETWORK_ENDPOINT=http://0.0.0.0/` est lu (`GridContext::load`). Mais PROJ construit l'opération et renvoie `HUGE_VAL` au `proj_trans` ; ici `ProjBuilder::proj` échoue dès la construction (`fetch_missing` appelé par `src/projections/models/mod.rs:131` et `src/gridpath.rs:70`). Grille `wvhpgn.tif` absente du dépôt. |
| 13 | `network_endpoint_api_and_not_reachable_gridshift` | écart | Construction paresseuse absente (même cause que 12). Attendu PROJ : `P` non nul, `HUGE_VAL`, `proj_errno == PROJ_ERR_OTHER_NETWORK_ERROR`, `proj_trans_get_last_used_operation` de description `NAD83 to NAD83(HARN) (47)`, répété trois fois dont sens inverse. Ni `proj_errno` réseau ni `last_used_operation` ne sont exposés. |
| 14 | `network_endpoint_api_and_not_reachable_xyzgridshift` | écart | Même cause que 13. |
| 15 | `network_endpoint_api_and_not_reachable_hgridshift` | écart | Même cause que 13. |
| 16 | `network_endpoint_api_and_not_reachable_vgridshift` | écart | Même cause que 13. |
| 17 | `cache_basic` | écart | Attend la base SQLite du cache de grilles de PROJ (`pj_context_get_grid_cache_filename`, lignes lues avec `sqlite3_step`). La crate écrit des fichiers bruts (`store_cache`, `src/network.rs`), sans base. Le test compte aussi l'activité réseau par callbacks. |
| 18 | `proj_grid_cache_clear` | écart | `proj_grid_cache_set_filename`, `proj_grid_cache_clear` : absents de la façade (seul `grid_cache_enable` existe). |
| 19 | `cache_saturation` | écart | `proj_grid_cache_set_max_size` absent ; la base SQLite n'existe pas (voir 17). |
| 20 | `cache_ttl` | écart | `proj_grid_cache_set_ttl` absent (valeur `-1` refusée, `86400` accepté) ; pas de métadonnées de fraîcheur. |
| 21 | `cache_lock` | écart | Verrou de cache multi-processus (`proj_grid_cache_set_filename` + table d'entrées) absent ; `std::fs` seul ne fournit pas le protocole de PROJ. |
| 22 | `download_whole_files` | écart | `proj_is_download_needed(ctx, name, ignore_ttl)`, `proj_download_file(ctx, name, ignore_ttl, cbk, user)` (avec progression `(ctx, pct)`) et `pj_find_file` : aucune API publique. `decide_download` et `fetch_missing` existent en interne (`pub(crate)`). Pas de base SQLite pour les tests d'état (`lookForGridInfo`). |
| 23 | `file_api` | écart | `proj_context_set_fileapi` (open, read, write, close, exists, mkdir, unlink, rename) absent ; `set_file_finder` ne couvre que la recherche par nom. |
| 24 | `proj_coordoperation_get_grid_used` | écart | `proj_coordoperation_get_grid_used` (short_name, full_name, package, url, direct_download, open_license, available) absent de la façade. La table `grid_alternatives` (`src/projdb.rs`, `direct_download`) a les données, pas l'API. Grille `nz_linz_nzgd2000-c120100904-grid01.tif` absente du dépôt. |
| 25 | `pyproj_issue_1192` | écart | `proj_create_operation_factory_context`, `proj_operation_factory_context_set_grid_availability_use`, `proj_coordoperation_is_instantiable` : la logique existe (`GridAvailabilityUse` `src/iso19111/factory.rs:103`, `is_proj_instantiable` `src/iso19111/select.rs:800`) mais aucune n'est exposée à la façade. Le test vérifie aussi l'absence d'activité réseau par callbacks. |
| 26 | `do_not_attempt_network_access_known_available_network_on` | écart | `proj_create_operations` avec `PROJ_GRID_AVAILABILITY_KNOWN_AVAILABLE` (`GridAvailabilityUse` existe en interne) et contrôle « aucune activité réseau » par callbacks : callbacks absents. |

Compte : 26 TEST() C++ ; 1 porté (`curl_invalid_resource`) ; 1 partiel
(`basic`) ; 24 écarts.

## projsync (`src/apps/projsync.cpp`, `test/cli/test_projsync.sh`) : écart total

Non commencé dans ce lot. Le binaire `[[bin]] projsync` n'existe pas, et
`test/cli/test_projsync.sh` (212 lignes) n'est pas porté. Ce qu'il faut :
options (`--source-id`, `--area-of-use`, `--file`, `--all`, `--list-files`,
`--dry-run`, `--system-directory`, `--user-directory`, `--quiet`,
`--verbose`, `--version`, `--help`) ; lecture de `files.geojson` avec un
analyseur JSON minimal sans dépendance ; téléchargement via le même client
HTTP que `src/network.rs` (qui n'a pas de `pub` d'entrée de téléchargement de
fichier, voir 22). Le serveur local de `tests/net_support` sert déjà les
routes nécessaires à un test `projsync`.

## Commandes

```
cargo test --test unit_network_local --test network_grid
```
