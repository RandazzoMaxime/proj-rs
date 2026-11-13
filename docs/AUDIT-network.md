# Audit du lot network

Référence : PROJ 9.9.0, `test/unit/test_network.cpp` (26 `TEST(networking, ...)`,
dont la plupart sous `#ifdef CURL_ENABLED`), `src/apps/projsync.cpp`,
`test/cli/test_projsync.sh`.

Légende : **porté** = assertions C++ reprises ; **partiel** = une partie
portée, le reste listé ; **écart** = pas de test, cause exacte donnée.

Le port de la couche réseau est `src/network.rs` (`pub(crate)`) et la façade
`src/proj.rs` (`ProjBuilder`). Ce qui existe : activation du réseau
(`enable_network`, `PROJ_NETWORK`, `proj.ini`), `cdn_endpoint` et
`PROJ_NETWORK_ENDPOINT` (lus à la construction du builder), téléchargement d'une
grille entière par `GET` simple (`http_get`, redirections suivies, pas de `Range`),
cache en fichiers dans le répertoire utilisateur (`grid_cache_enable`,
`set_user_writable_directory`), `set_file_finder`, et, dans ce lot :

- `NetworkCallbacks` (trait, `NetworkHandle`, `OpenReply`, `NetworkError`),
  enregistrable par `ProjBuilder::set_network_callbacks`. Une grille absente est
  lue entière par un `open(url, 0, 16384)`, puis des `read_range` de 16384 octets
  jusqu'à la taille du `Content-Range`, puis `close` (`fetch_with_callbacks`).
- Erreur typée au calcul : `ProjError::Network` (PROJ `PROJ_ERR_OTHER_NETWORK_ERROR`)
  quand la grille d'un candidat de paire de CRS est absente et que le réseau ne la
  donne pas. Le point s'arrête là, sans nouvelle tentative (`trans.cpp:284`).
- Construction paresseuse d'un candidat de paire de CRS dont la grille nommée est
  téléchargeable (`gridshift::deferred`). Hors de ces candidats, une chaîne
  `+proj=...` explicite échoue à la création, comme PROJ (`proj_create` renvoie
  `NULL`, vérifié avec la référence).
- `last_used_operation()` donne le nom du candidat (`ProjInfo::description`), comme
  `proj_pj_info` de `proj_trans_get_last_used_operation`.

Ce qui manque : lecture paresseuse par plages des grilles (la grille est lue
entière à la construction, `read_resource`), validation de cache par `ETag` et
`Last-Modified`, cache SQLite (TTL, taille max, verrou : lot cache, autre agent),
API de téléchargement explicite, API de fichiers, façade des opérations
(`proj_create_operations` et contexte de fabrique).

Tests de l'API existants, hors liste C++ : `tests/network_grid.rs`
(téléchargement dans le cache, réseau off, chemin de recherche).

## Cas portés dans ce lot

| TEST() C++ | Test Rust | Statut |
|---|---|---|
| `networking.basic` (réseau off par défaut, pipeline nul, aucune requête) | `tests/unit_network_local.rs::basic_network_is_off_by_default_and_makes_no_request` | partiel : la partie « réseau off » est portée ; la partie « activé par `PROJ_NETWORK=ON` puis par l'API » (valeurs `1.9992776848`) ne l'est pas, voir plus bas |
| `networking.curl_invalid_resource` | `tests/unit_network_local.rs::curl_invalid_resource` | porté sur serveur local : 404, `proj` renvoie une erreur, une seule requête |
| `networking.simul_open_error` | `tests/unit_network_core.rs::simul_open_error_fails_creation_after_one_open` | porté : une seule ouverture `(url, 0, 16384)`, erreur, pas de création |
| `networking.network_endpoint_env_variable` | `tests/unit_network_endpoint_env.rs::network_endpoint_env_variable` | porté : `PROJ_NETWORK_ENDPOINT` lu au builder, `EPSG:4269 -> EPSG:4152` construit, transformation en erreur réseau |
| `networking.network_endpoint_api_and_not_reachable_gridshift` | `tests/unit_network_core.rs::endpoint_api_not_reachable_gridshift` | porté (`NAD83 to NAD83(HARN) (47)`, aller, aller, inverse) |
| `networking.network_endpoint_api_and_not_reachable_hgridshift` | `tests/unit_network_core.rs::endpoint_api_not_reachable_hgridshift` | porté (`MGI to ETRS89 (8)`) |
| `networking.network_endpoint_api_and_not_reachable_xyzgridshift` | `tests/unit_network_core.rs::endpoint_api_not_reachable_xyzgridshift` | porté (`NTF to ETRS89-FRA [RGF93 v1] (1)`) |
| `networking.network_endpoint_api_and_not_reachable_vgridshift` | `tests/unit_network_core.rs::endpoint_api_not_reachable_vgridshift` | porté (`POSGAR 2007 to SRVN16 height (1)`) |

Tests unitaires ajoutés hors liste C++ (`src/network.rs`, `mod callback_tests`) :
la lecture entière d'une grille par callbacks en plages de 16384 octets
(`open` puis `read_range` aux offsets 16384 et 32768, un seul `close`), et
l'extraction de la taille depuis `Content-Range`.

## Tableau complet des 26 TEST()

| # | TEST() C++ | Statut | Cause exacte si écart |
|---|---|---|---|
| 1 | `initial_check` | écart, sans objet | Sonde de disponibilité du CDN : `GET` avec `Range: 0-1` sur `cdn.proj.org`, attend `206`. Elle ne sert qu'à conditionner les autres cas (`networkAccessOK`) ; ici le serveur local remplace le CDN et les cas ne sont pas conditionnés. La crate n'envoie pas de `Range`. |
| 2 | `basic` | partiel | Partie « off » portée (ligne 1 de la table). Partie « on » non portée : les valeurs `1.9992776848` / `48.9999322600` viennent de `fr_ign_ntf_r93.tif`, absent du dépôt et de `build/data` de la référence ; sans grille, aucune valeur PROJ à comparer. |
| 3 | `curl_invalid_resource` | porté | Voir « Cas portés ». |
| 4 | `custom` | écart | Les callbacks existent, mais la séquence d'événements de PROJ suppose une lecture paresseuse : la grille est ouverte à `proj_create` (`open(0, 16384)`, en-têtes, `close`), puis rouverte au calcul à `offset 524288` et lue à `3670016`. La crate lit la grille entière à la construction (`read_resource`, `src/projections/models/mod.rs`) : il faut une lecture de grille par plages. |
| 5 | `getfilesize` | écart | Même cause que 4 : `open(0, 16384)` seul à la création, puis une seconde création qui rouvre le fichier tronqué ; la taille est lue dans `Content-Range`, sans lecture des tuiles. |
| 6 | `simul_open_error` | porté | Voir « Cas portés ». |
| 7 | `simul_read_range_error` | écart | Même cause que 4 : l'erreur de `read_range` doit survenir au calcul (`HUGE_VAL`) après une ouverture à `524288`, ce qui suppose la lecture par plages. |
| 8 | `simul_file_change_while_opened` | écart | Même cause que 4, plus la validation par `ETag` / `Last-Modified` entre deux lectures : la crate ne compare pas les en-têtes d'un fichier déjà ouvert. |
| 9 | `curl_hgridshift` | écart | Grille CDN `fr_ign_gr3df97a.tif` absente du dépôt. Valeurs de référence : tentées avec le `proj` de référence sur `tests/gie/tests/test_hgrid_with_subgrid.tif` (et `test_hgrid_degree.tif`), refusées avec `could not find required grid(s)` quel que soit le chemin. Cause de ce refus non élucidée ; sans valeur PROJ, pas de port. |
| 10 | `curl_vgridshift` | écart | Même cause que 9 (`egm08_25.tif`, absente ; pas de fixture de référence chargée par `proj`). |
| 11 | `curl_vgridshift_vertcon` | écart | Même cause que 9 (`vertcone.tif`, absente). |
| 12 | `network_endpoint_env_variable` | porté | Voir « Cas portés ». |
| 13 | `network_endpoint_api_and_not_reachable_gridshift` | porté | Voir « Cas portés ». |
| 14 | `network_endpoint_api_and_not_reachable_xyzgridshift` | porté | Voir « Cas portés ». |
| 15 | `network_endpoint_api_and_not_reachable_hgridshift` | porté | Voir « Cas portés ». |
| 16 | `network_endpoint_api_and_not_reachable_vgridshift` | porté | Voir « Cas portés ». |
| 17 | `cache_basic` | écart, lot cache | Attend la base SQLite du cache de grilles de PROJ (`pj_context_get_grid_cache_filename`). La crate écrit des fichiers bruts (`store_cache`). Traité par le lot cache (`src/network_cache.rs`, autre agent). |
| 18 | `proj_grid_cache_clear` | écart, lot cache | `proj_grid_cache_set_filename`, `proj_grid_cache_clear` : absents de la façade ; même lot. |
| 19 | `cache_saturation` | écart, lot cache | `proj_grid_cache_set_max_size` absent ; la base SQLite n'existe pas (voir 17). |
| 20 | `cache_ttl` | écart, lot cache | `proj_grid_cache_set_ttl` absent (valeur `-1` refusée, `86400` acceptée) ; pas de métadonnées de fraîcheur. |
| 21 | `cache_lock` | écart, lot cache | Verrou de cache multi-processus absent. |
| 22 | `download_whole_files` | écart | `proj_is_download_needed(ctx, name, ignore_ttl)`, `proj_download_file` (avec progression) et `pj_find_file` : aucune API publique. L'état testé (`lookForGridInfo`) est dans la base du cache (lot cache). |
| 23 | `file_api` | écart | `proj_context_set_fileapi` (open, read, write, close, exists, mkdir, unlink, rename) absent. Toutes les lectures de grilles passent par `std::fs` (`gridpath::read`, `read_resource`) : une API de fichiers demande de les router toutes. |
| 24 | `proj_coordoperation_get_grid_used` | écart | Le test exige que `proj_create("proj=vgridshift grids=nz_linz_nzgd2000-c120100904-grid01.tif")` réussisse avec le CDN, et que l'URL soit `https://cdn.proj.org/...`. Mesuré avec le `libproj` de référence (`proj_create` avec l'endpoint par défaut, réseau bloqué) : `NULL`, « could not find required grid(s) ». Et le `proj.db` de référence n'a aucune ligne pour ce nom (479 lignes de `grid_alternatives`, aucune pour `c120100904`) : PROJ laisse donc l'URL vide. Le port de l'assertion demande le fichier CDN, qui n'est pas dans le dépôt. Le port de `GridInfo` (`lookForGridInfo`) suit PROJ et est déjà en place. |
| 25 | `pyproj_issue_1192` | écart | `proj_create_operation_factory_context`, `proj_operation_factory_context_set_grid_availability_use`, `proj_coordoperation_is_instantiable` : aucune façade. La logique existe en Rust public (`CoordinateOperationFactory::create_operations`, `GridAvailabilityUse`, `Proj` candidats), mais pas branchée sur le contexte réseau du builder. |
| 26 | `do_not_attempt_network_access_known_available_network_on` | écart | Même cause que 25 : `proj_create_operations` avec `PROJ_GRID_AVAILABILITY_KNOWN_AVAILABLE`, et le contrôle « aucune activité réseau » par callbacks, qui exige que la fabrique passe par le contexte du builder. |

Compte : 26 TEST() C++ ; 7 portés (`curl_invalid_resource`, `simul_open_error`,
`network_endpoint_env_variable`, et les quatre `not_reachable`) ; 1 partiel
(`basic`) ; 18 écarts, dont 5 du lot cache (17 à 21) et 4 à lecture par plages
(4, 5, 7, 8).

## projsync (`src/apps/projsync.cpp`, `test/cli/test_projsync.sh`) : écart total

Non commencé dans ce lot. Le binaire `[[bin]] projsync` n'existe pas, et
`test/cli/test_projsync.sh` (212 lignes) n'est pas porté. Ce qu'il faut :
options (`--source-id`, `--area-of-use`, `--file`, `--all`, `--list-files`,
`--dry-run`, `--system-directory`, `--user-directory`, `--quiet`,
`--verbose`, `--version`, `--help`) ; lecture de `files.geojson` avec un
analyseur JSON minimal sans dépendance ; téléchargement via le même client
HTTP que `src/network.rs`. Ce lot ne le touche pas (autre agent).

## Commandes

```
cargo test --test unit_network_core --test unit_network_endpoint_env --test unit_network_local --test network_grid
cargo test --lib callback_tests
```
