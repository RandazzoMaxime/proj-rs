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
  ouverte par son premier morceau, `open(url, 0, 16384)`, avec les en-têtes puis `close`.
  Si c'est un GeoTIFF à tuiles non compressé, ses tuiles sont lues au calcul, comme
  PROJ : `open` puis `read_range` sur le même handle, gardé jusqu'à la destruction du
  `Proj`, et un cache de morceaux par contexte (`src/network_chunks.rs`,
  `src/projections/gridshift/tiles.rs`). Les autres grilles sont lues entières, par
  morceaux de 16384 octets jusqu'à la taille du `Content-Range` (`fetch_with_callbacks`).
- Erreur typée au calcul : `ProjError::Network` (PROJ `PROJ_ERR_OTHER_NETWORK_ERROR`)
  quand la grille d'un candidat de paire de CRS est absente et que le réseau ne la
  donne pas. Le point s'arrête là, sans nouvelle tentative (`trans.cpp:284`).
- Construction paresseuse d'un candidat de paire de CRS dont la grille nommée est
  téléchargeable (`gridshift::deferred`). Hors de ces candidats, une chaîne
  `+proj=...` explicite échoue à la création, comme PROJ (`proj_create` renvoie
  `NULL`, vérifié avec la référence).
- `last_used_operation()` donne le nom du candidat (`ProjInfo::description`), comme
  `proj_pj_info` de `proj_trans_get_last_used_operation`.
- `Proj::grids_needed` (`proj_coordoperation_get_grid_used`) : une grille sans ligne en base et
  absente du disque est ouverte à `endpoint/nom` quand le réseau est actif (`network::open_remote_grid`,
  premier morceau par callbacks) ; son URL est celle-ci et elle est disponible. La fabrique et
  `is_proj_instantiable` ne l'interrogent pas (`RemoteGrids::Skip`), comme `factory.cpp` coupe le
  réseau.

Ce qui manque : lecture paresseuse des seuls GeoTIFF à tuiles non compressés (les
grilles en bandes, compressées, à bandes planaires, et les grilles non-TIFF
restent lues entières), validation de la fraîcheur d'un fichier ouvert par `ETag` et
`Last-Modified` pour les autres étapes que `vgridshift` et pour le cache disque (seule
`vgridshift` recharge, voir `docs/PORTING-network.md`), cache SQLite (TTL, taille max, verrou :
lot cache, autre agent),
API de téléchargement explicite, API de fichiers, façade C des opérations
(`proj_create_operations` et contexte de fabrique C : la fabrique Rust est utilisée à la place,
par `ProjBuilder::create_operations` quand des callbacks sont enregistrés, voir
`do_not_attempt_network_access_known_available_network_on` et `pyproj_issue_1192`).

Tests de l'API existants, hors liste C++ : `tests/network_grid.rs`
(téléchargement dans le cache, réseau off, chemin de recherche).

## Cas portés dans ce lot

| TEST() C++ | Test Rust | Statut |
|---|---|---|
| `networking.basic` (réseau off par défaut, pipeline nul, aucune requête) | `tests/unit_network_local.rs::basic_network_is_off_by_default_and_makes_no_request` | partiel : la partie « réseau off » est portée ; la partie « activé par `PROJ_NETWORK=ON` puis par l'API » (valeurs `1.9992776848`) ne l'est pas, voir plus bas |
| `networking.curl_invalid_resource` | `tests/unit_network_local.rs::curl_invalid_resource` | porté sur serveur local : 404, `proj` renvoie une erreur, une seule requête |
| `networking.custom` | `tests/unit_network_core.rs::custom` | porté : séquence d'appels de PROJ à l'identique (création : `open(url, 0, 16384)`, trois en-têtes, `close` ; premier point (lat 49) : `open(url, 524288, 278528)`, trois en-têtes ; second point (lat -49) : `read_range(3670016, 278528)` sur le même handle, trois en-têtes ; `close` à `proj_destroy` ; seconde création sur le même builder sans aucun appel), valeurs `1.25`, `2.25`, `1.25`. La séquence des octets de tuile vient de `TileByteCounts` (262144 octets, couverture alignée à 278528) ; voir `docs/PORTING-network.md`. |
| `networking.getfilesize` | `tests/unit_network_core.rs::getfilesize` | porté : séquence d'appels de PROJ à l'identique (création : `open(url, 0, 16384)` sur une réponse de 550 octets, `Content-Range` à 4153510, trois en-têtes, `close` ; seconde création sur le même builder sans aucun appel). Le test ne transforme rien. Les bandes non compressées sont lues par morceaux, comme les tuiles ; voir `docs/PORTING-network.md`. |
| `networking.simul_open_error` | `tests/unit_network_core.rs::simul_open_error_fails_creation_after_one_open` | porté : une seule ouverture `(url, 0, 16384)`, erreur, pas de création |
| `networking.simul_read_range_error` | `tests/unit_network_core.rs::simul_read_range_error` | porté : séquence de PROJ à l'identique, mesurée sur `libproj` 9.9.0 (harnais C, mêmes callbacks) : création `open(url, 0, 16384)`, trois en-têtes, `close` ; premier point (lat 49) `open(url, 524288, 278528)`, trois en-têtes, `z` 1.25 ; second point (lat -49) `read_range(3670016, 278528)` refusé « read range error », puis aucun appel (pas d'en-tête), `z` `HUGE_VAL` et `proj_errno` 4099 (`PROJ_ERR_OTHER_NETWORK_ERROR`), port : `Err(ProjError::Network)` ; `close` à `proj_destroy`. Aucun changement d'implémentation : la lecture par tuiles de `custom` gère déjà l'erreur. |
| `networking.simul_file_change_while_opened` | `tests/unit_network_core.rs::simul_file_change_while_opened` | porté : séquence de PROJ à l'identique, mesurée sur `libproj` 9.9.0 (programme C, callbacks qui journalisent, fixture `egm96_15_uncompressed_truncated.tif`) : création `open(url, 0, 16384)`, trois en-têtes, `close` ; transformation (lat 49) `open(url, 524288, 278528)`, trois en-têtes dont `Last-Modified` = `some_date CHANGED!!!!`, `close` du fichier 2, `open(url, 0, 16384)` du fichier 3, trois en-têtes ; `z` 1.25 ; `close` du fichier 3 à `proj_destroy`. Port : détection par `RemoteFile::observe`, rechargement de `vgridshift` (`VerticalGrids`) et second passage du point. Écarts de la fiche : `PORTING-network.md`. |
| `networking.network_endpoint_env_variable` | `tests/unit_network_endpoint_env.rs::network_endpoint_env_variable` | porté : `PROJ_NETWORK_ENDPOINT` lu au builder, `EPSG:4269 -> EPSG:4152` construit, transformation en erreur réseau |
| `networking.network_endpoint_api_and_not_reachable_gridshift` | `tests/unit_network_core.rs::endpoint_api_not_reachable_gridshift` | porté (`NAD83 to NAD83(HARN) (47)`, aller, aller, inverse) |
| `networking.network_endpoint_api_and_not_reachable_hgridshift` | `tests/unit_network_core.rs::endpoint_api_not_reachable_hgridshift` | porté (`MGI to ETRS89 (8)`) |
| `networking.network_endpoint_api_and_not_reachable_xyzgridshift` | `tests/unit_network_core.rs::endpoint_api_not_reachable_xyzgridshift` | porté (`NTF to ETRS89-FRA [RGF93 v1] (1)`) |
| `networking.network_endpoint_api_and_not_reachable_vgridshift` | `tests/unit_network_core.rs::endpoint_api_not_reachable_vgridshift` | porté (`POSGAR 2007 to SRVN16 height (1)`) |
| `networking.do_not_attempt_network_access_known_available_network_on` | `tests/unit_network_core.rs::do_not_attempt_network_access_known_available_network_on` | porté : mesuré sur `libproj` 9.9.0 (sonde C reproduisant le TEST, `probe2.c` non versionnée) : `EPSG:4326` vers `EPSG:4267`, `KNOWN_AVAILABLE`, `PARTIAL_INTERSECTION`, réseau on, callbacks enregistrés : 79 opérations (noms et ordre identiques, réseau off aussi), 0 appel de callback ; témoin : `+proj=hgridshift +grids=proj_rs_no_such_grid.tif` donne `NULL` et un `open`. Port : `ProjBuilder::create_operations` (nouvelle API, enveloppe `with_context` comme les autres créations du builder), liste exacte, 0 appel, témoin à 1 appel. `ASSERT_TRUE` de `proj_context_set_network_callbacks` : sans objet en Rust (l'enregistrement ne renvoie rien) ; voir `docs/PORTING-network.md`. |
| `networking.proj_coordoperation_get_grid_used` | `tests/unit_network_core.rs::proj_coordoperation_get_grid_used` | porté, avec octets de substitution : les trois assertions du TEST (`shortName`, `fullName` vide, `url` `https://cdn.proj.org/nz_linz_nzgd2000-c120100904-grid01.tif`), deux créations. Mesuré sur `libproj` 9.9.0 (sonde C `grid_used_probe.c`, non versionnée, mêmes callbacks) : avec les octets servis, `available` 1, `directDownload` 0, `openLicense` 0, une seule `open(url, 0, 16384)` pour les deux créations, aucun appel pendant `proj_coordoperation_get_grid_used` ; sans callbacks, `proj_create` renvoie `NULL`. Le port : `Proj::grids_needed` ouvre une grille sans ligne en base à `endpoint/nom` quand le réseau est actif (`network::open_remote_grid`, `RemoteGrids::Open`). Le test ne reproduit pas la garde `networkAccessOK` : il tourne toujours, puisque les callbacks répondent sans I/O. Écarts : `docs/PORTING-network.md`. |

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
| 4 | `custom` | porté | Voir « Cas portés ». |
| 5 | `getfilesize` | porté | Voir « Cas portés ». Mesuré avant le port : PROJ 9.9.0 fait `open(url, 0, 16384)`, `Content-Range`, `Last-Modified`, `ETag`, `close` à la création, puis aucun appel ; une transformation demande `open(url, 950272, 16384)` (sonde C). Le port lit les bandes à `StripOffsets`/`StripByteCounts` par morceaux, la dernière bande ne comptant que ses lignes, et `Content-Range` donne la taille même si la réponse est plus courte. Vérifié contre le fichier local, non commité : `test_hgrid_strip.tif` (4 lignes, bandes de 3, dernière bande d'une ligne) et `test_vgrid_bigendian.tif` (bande unique, gros-boutiste) donnent les mêmes bits sur un balayage du globe au degré (16 points dans la grille, aucun écart). |
| 6 | `simul_open_error` | porté | Voir « Cas portés ». |
| 7 | `simul_read_range_error` | porté | Voir « Cas portés ». |
| 8 | `simul_file_change_while_opened` | porté | Voir « Cas portés ». |
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
| 24 | `proj_coordoperation_get_grid_used` | porté, avec octets de substitution | Les trois assertions sont portées (nom, nom complet vide, URL `https://cdn.proj.org/...`), à chaque création. Le fichier CDN n'est pas dans le dépôt et le réseau est coupé : sans octets, `proj_create` renvoie `NULL` (mesuré, `libproj` 9.9.0, « could not find required grid(s) »). Le test sert donc les 956 octets de la fixture egm96 tronquée sous cette URL. Les valeurs asserties ne dépendent pas du contenu ; ce qui en dépend (séquence d'appels avec le vrai fichier, `available` d'un vrai fichier) n'est pas mesuré. Voir « Cas portés dans ce lot ». |
| 25 | `pyproj_issue_1192` | porté | `tests/unit_network_core.rs::pyproj_issue_1192`. Pas de façade C : le contexte de fabrique est `CoordinateOperationContext` (public), avec `GridAvailabilityUse::IgnoreGridAvailability` et `SpatialCriterion::PartialIntersection`. La liste (10 opérations, noms et ordre) et les drapeaux d'instanciabilité (réseau off : 8 vraies ; réseau on : 10) sont ceux de `libproj` 9.9.0 mesurés avec une sonde C. Écarts et lacune (opérations à grille absente, réseau on) : `docs/PORTING-network.md`. |
| 26 | `do_not_attempt_network_access_known_available_network_on` | porté | Voir « Cas portés dans ce lot ». La façade C reste absente : la fabrique passe par `ProjBuilder::create_operations`, qui applique le contexte du builder. |

Compte : 26 TEST() C++ ; 14 portés (`curl_invalid_resource`, `simul_open_error`,
`simul_file_change_while_opened`,
`simul_read_range_error`, `network_endpoint_env_variable`, `custom`, `getfilesize`, `proj_coordoperation_get_grid_used` (avec octets de substitution), `pyproj_issue_1192`, `do_not_attempt_network_access_known_available_network_on`, et les quatre
`not_reachable`) ; 1 partiel (`basic`) ; 11 écarts, dont 5 du lot cache (17 à 21) et `initial_check` (sans objet).

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

## Cache et projsync (lot network-cache-projsync)

Fiche : `docs/PORTING-network-cache.md`. Les lignes du tableau des 26 TEST() ci-dessus
ne sont pas réécrites ; ce qui suit les complète pour les cas de ce lot.

| TEST() C++ | Test Rust | Statut |
|---|---|---|
| `networking.cache_basic` | `tests/unit_network_cache.rs::cache_basic` | porté, sur serveur local : tête LRU = chunk 0, un second build ne fait aucune requête. Écart : fichier entier, pas de lecture par plages (fiche, point 1). |
| `networking.proj_grid_cache_clear` | `tests/unit_network_cache.rs::proj_grid_cache_clear` | porté : la base est créée par un build, supprimée par `clear_grid_cache`. |
| `networking.cache_saturation` | `tests/unit_network_cache.rs::cache_saturation` | porté : limite de 2 chunks par `PROJ_GRID_CACHE_MAX_SIZE_BYTES=32768`, 2 lignes dans `chunk_data`, `chunks` et `linked_chunks`. |
| `networking.cache_ttl` | `tests/unit_network_cache.rs::cache_ttl` | porté : TTL négatif = données corrompues utilisées (build en échec), TTL 86400 = téléchargement de nouveau, `lastChecked`, `lastModified` et `etag` mis à jour. |
| `networking.cache_lock` | `tests/unit_network_cache.rs::cache_lock` | porté : build sans cache après 25 essais, durée au moins 650 ms (calendrier de PROJ). Écart : PROJ vérifie `>= 1` s en secondes entières (fiche, point 3). |
| `networking.curl_hgridshift` | `tests/unit_network_grids.rs::curl_hgridshift` | porté avec fixture `fr_ign_gr3df97a.tif` (xyzgridshift, trois échantillons) ; valeurs du binaire réseau de référence sur les mêmes octets (fiche, points 4 et 6). `proj_assign_context` non porté (point 5). |
| `networking.curl_vgridshift` | `tests/unit_network_grids.rs::curl_vgridshift` | porté avec fixture `us_nga_egm08_25.tif` ; valeur de référence `14.6249995232`, pas `-31.89` du CDN. |
| `networking.curl_vgridshift_vertcon` | `tests/unit_network_grids.rs::curl_vgridshift_vertcon` | porté avec fixture `us_noaa_vertcone.tif` ; valeur de référence `-0.1295000091`, pas `-0.15` du CDN. |

Non portés dans ce lot, hors de sa liste : `networking.basic` (partie réseau activé),
`initial_check`, `network_endpoint_*`, `download_whole_files`, `file_api`,
`do_not_attempt_network_access_known_available_network_on`, et les cas à callbacks
(`simul_*`) : voir les lignes du tableau ci-dessus.

### projsync (`src/apps/projsync.cpp`, `test/cli/test_projsync.sh`)

Porté : `src/bin/projsync.rs` (options de `projsync.cpp`, plus `--help` et `--version`),
bibliothèque `src/projsync.rs` (analyseur JSON sans dépendance avec la limite de
profondeur 128, index, sélection, téléchargement avec enregistrement dans `cache.db`).
`tests/unit_projsync.rs` : 20 tests. Tous les cas du script sont là (liste, `--all`,
`--source-id`, `--area-of-use`, `--file` avec deuxième passage « already downloaded »,
`--bbox` dont les fenêtres de l'antiméridien, `--exclude-world-coverage`, `--verbose` avec
versions retirées ou ajoutées) ; les sorties attendues sont celles du binaire de référence
sur le même index. Écarts : téléchargement non conditionnel, `--system-directory` lié à
`PROJ_DATA`/`PROJ_LIB`, index de test au lieu de `files.geojson` du CDN, téléchargement
réel non comparé au binaire de référence (fiche, points 7 à 9).

Commande : `cargo test --test unit_projsync --test unit_network_cache --test unit_network_grids`
