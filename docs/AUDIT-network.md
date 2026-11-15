# Audit du lot network

Référence : PROJ 9.9.0, `test/unit/test_network.cpp` (26 `TEST(networking, ...)`,
dont la plupart sous `#ifdef CURL_ENABLED`), `src/apps/projsync.cpp`,
`test/cli/test_projsync.sh`.

Légende : **porté** = assertions C++ reprises, mêmes entrées ; **porté avec substitution**
= assertions C++ reprises, mais une entrée est remplacée (serveur local au lieu du CDN,
fixture synthétique, endpoint local), la substitution est nommée ; **partiel** = une partie
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
  Si c'est un GeoTIFF à tuiles ou à bandes, non compressé ou déflaté (Deflate, avec ou
  sans prédicteur, chunky ou planaire), ses tuiles ou bandes sont lues au calcul, comme
  PROJ : `open` puis `read_range` sur le même handle, gardé jusqu'à la destruction du
  `Proj`, et un cache de morceaux par contexte (`src/network_chunks.rs`,
  `src/projections/gridshift/tiles.rs`). La création ne lit que le premier morceau, comme
  PROJ, y compris pour la grille déflatée du CDN. Les autres grilles sont lues entières, par
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
- `ProjBuilder::set_file_api` (`src/file_api.rs`) : l'API de fichiers de PROJ
  (`proj_context_set_fileapi`, dix callbacks deviennent les traits `FileApi` et `ApiFile`).
  Passent par elle : la recherche des grilles (`exists`), la lecture des grilles, modèles et
  dictionnaires (`open`, `seek`, `read`), la création du répertoire utilisateur (`exists`,
  `mkdir`), et le téléchargement (`open` en création, `write`, `close`, `unlink`, `rename`).
  Mesuré : `docs/PORTING-network.md`, section « API de fichiers ».

Ce qui manque : lecture paresseuse des GeoTIFF compressés autrement qu'en Deflate (LZW,
PackBits, JPEG, CCITT, ... : lus entiers, comme les grilles non-TIFF) et des prédicteurs
autres que 1 à 3, rechargement au changement pour les autres étapes que `vgridshift`
(`hgridshift`, `gridshift`, `xyzgridshift` : le changement est enregistré, la grille n'est pas
rechargée), validation par `ETag` et `Last-Modified` du cache disque quand les callbacks servent
les grilles (le cache en mémoire, par URL, est fait par le lot `shared-validators`),
façade C des opérations
(`proj_create_operations` et contexte de fabrique C : pas de handle C. Les `TEST` de `test_c_api.cpp`
qui appellent `proj_create_operations` sont portés sur l'API Rust, voir
`docs/PORTING-capi-operation-factory-core.md`. La fabrique Rust est utilisée à la place,
par `ProjBuilder::create_operations` quand des callbacks sont enregistrés, voir
`do_not_attempt_network_access_known_available_network_on` et `pyproj_issue_1192`).

Tests de l'API existants, hors liste C++ : `tests/network_grid.rs`
(téléchargement dans le cache, réseau off, chemin de recherche).

## Cas portés dans ce lot

| TEST() C++ | Test Rust | Statut |
|---|---|---|
| `networking.basic` (réseau off, `PROJ_NETWORK=ON`, retour à off, réseau par l'API, valeur) | `tests/unit_network_basic.rs::basic` | porté, contre le vrai CDN (`fr_ign_ntf_r93.tif`) : les quatre états et la valeur `1.9992776848` / `48.99993226`, mesurés avec `libproj` 9.9.0 ; section « networking.basic (porté) ». Hors liste : `tests/unit_network_local.rs::basic_network_is_off_by_default_and_makes_no_request` (aucune requête reçue par un serveur local). |
| `networking.curl_invalid_resource` | `tests/unit_network_local.rs::curl_invalid_resource` | porté sur serveur local : 404, `proj` renvoie une erreur, une seule requête |
| `networking.custom` | `tests/unit_network_core.rs::custom` | porté : séquence d'appels de PROJ à l'identique (création : `open(url, 0, 16384)`, trois en-têtes, `close` ; premier point (lat 49) : `open(url, 524288, 278528)`, trois en-têtes ; second point (lat -49) : `read_range(3670016, 278528)` sur le même handle, trois en-têtes ; `close` à `proj_destroy` ; seconde création sur le même builder sans aucun appel), valeurs `1.25`, `2.25`, `1.25`. La séquence des octets de tuile vient de `TileByteCounts` (262144 octets, couverture alignée à 278528) ; voir `docs/PORTING-network.md`. |
| `networking.getfilesize` | `tests/unit_network_core.rs::getfilesize` | porté : séquence d'appels de PROJ à l'identique (création : `open(url, 0, 16384)` sur une réponse de 550 octets, `Content-Range` à 4153510, trois en-têtes, `close` ; seconde création sur le même builder sans aucun appel). Le test ne transforme rien. Les bandes non compressées sont lues par morceaux, comme les tuiles ; voir `docs/PORTING-network.md`. |
| `networking.simul_open_error` | `tests/unit_network_core.rs::simul_open_error_fails_creation_after_one_open` | porté : une seule ouverture `(url, 0, 16384)`, erreur, pas de création |
| `networking.simul_read_range_error` | `tests/unit_network_core.rs::simul_read_range_error` | porté : séquence de PROJ à l'identique, mesurée sur `libproj` 9.9.0 (harnais C, mêmes callbacks) : création `open(url, 0, 16384)`, trois en-têtes, `close` ; premier point (lat 49) `open(url, 524288, 278528)`, trois en-têtes, `z` 1.25 ; second point (lat -49) `read_range(3670016, 278528)` refusé « read range error », puis aucun appel (pas d'en-tête), `z` `HUGE_VAL` et `proj_errno` 4099 (`PROJ_ERR_OTHER_NETWORK_ERROR`), port : `Err(ProjError::Network)` ; `close` à `proj_destroy`. Aucun changement d'implémentation : la lecture par tuiles de `custom` gère déjà l'erreur. |
| `networking.simul_file_change_while_opened` | `tests/unit_network_core.rs::simul_file_change_while_opened` | porté : séquence de PROJ à l'identique, mesurée sur `libproj` 9.9.0 (programme C, callbacks qui journalisent, fixture `egm96_15_uncompressed_truncated.tif`) : création `open(url, 0, 16384)`, trois en-têtes, `close` ; transformation (lat 49) `open(url, 524288, 278528)`, trois en-têtes dont `Last-Modified` = `some_date CHANGED!!!!`, `close` du fichier 2, `open(url, 0, 16384)` du fichier 3, trois en-têtes ; `z` 1.25 ; `close` du fichier 3 à `proj_destroy`. Port : détection par `RemoteFile::observe`, rechargement de `vgridshift` (`VerticalGrids`) et second passage du point. Écarts de la fiche : `PORTING-network.md`. |
| `networking.simul_file_change_while_opened` (deuxième création, cas propre au port) | `tests/unit_network_core.rs::simul_file_change_between_two_creations`, `tests/unit_network_core.rs::simul_file_change_with_a_new_builder` | porté : séquences mesurées avec `libproj` 9.9.0 (sonde C, même fixture, mêmes en-têtes). Même builder, cache désactivé ou activé, contexte neuf : création 2 sans appel, changement détecté au premier calcul sur une tuile non en cache, rechargement de la grille, valeur 2.25 ; l'ancienne opération lit le fichier changé sans recharger. Deux builders (cas deux processus, cache désactivé chez PROJ) : rien n'est détecté. Non portés : les cas deux processus avec le cache disque (TTL valide, TTL dépassée), voir `docs/PORTING-network.md`. |
| `networking.network_endpoint_env_variable` | `tests/unit_network_endpoint_env.rs::network_endpoint_env_variable` | porté : `PROJ_NETWORK_ENDPOINT` lu au builder, `EPSG:4269 -> EPSG:4152` construit, transformation en erreur réseau |
| `networking.network_endpoint_api_and_not_reachable_gridshift` | `tests/unit_network_core.rs::endpoint_api_not_reachable_gridshift` | porté (`NAD83 to NAD83(HARN) (47)`, aller, aller, inverse) |
| `networking.network_endpoint_api_and_not_reachable_hgridshift` | `tests/unit_network_core.rs::endpoint_api_not_reachable_hgridshift` | porté (`MGI to ETRS89 (8)`) |
| `networking.network_endpoint_api_and_not_reachable_xyzgridshift` | `tests/unit_network_core.rs::endpoint_api_not_reachable_xyzgridshift` | porté (`NTF to ETRS89-FRA [RGF93 v1] (1)`) |
| `networking.network_endpoint_api_and_not_reachable_vgridshift` | `tests/unit_network_core.rs::endpoint_api_not_reachable_vgridshift` | porté (`POSGAR 2007 to SRVN16 height (1)`) |
| `networking.do_not_attempt_network_access_known_available_network_on` | `tests/unit_network_core.rs::do_not_attempt_network_access_known_available_network_on` | porté : mesuré sur `libproj` 9.9.0 (sonde C reproduisant le TEST, `probe2.c` non versionnée) : `EPSG:4326` vers `EPSG:4267`, `KNOWN_AVAILABLE`, `PARTIAL_INTERSECTION`, réseau on, callbacks enregistrés : 79 opérations (noms et ordre identiques, réseau off aussi), 0 appel de callback ; témoin : `+proj=hgridshift +grids=proj_rs_no_such_grid.tif` donne `NULL` et un `open`. Port : `ProjBuilder::create_operations` (nouvelle API, enveloppe `with_context` comme les autres créations du builder), liste exacte, 0 appel, témoin à 1 appel. `ASSERT_TRUE` de `proj_context_set_network_callbacks` : sans objet en Rust (l'enregistrement ne renvoie rien) ; voir `docs/PORTING-network.md`. |
| `networking.proj_coordoperation_get_grid_used` | `tests/unit_network_grid_used.rs::proj_coordoperation_get_grid_used` | porté contre le vrai CDN : les trois assertions du TEST (`shortName`, `fullName` vide, `url` `https://cdn.proj.org/nz_linz_nzgd2000-c120100904-grid01.tif`), deux créations, sans callbacks, gardé par la sonde `cdn_reachable` (sortie anticipée sans `206`). Valeurs de `libproj` 9.9.0 sur le même fichier (460261 octets) : `available` 1, `directDownload` 0, `openLicense` 0, `packageName` vide, un grid. Le port : `Proj::grids_needed` ouvre une grille sans ligne en base à `endpoint/nom` quand le réseau est actif (`network::open_remote_grid`, `RemoteGrids::Open`). Le vrai fichier a révélé un défaut corrigé : la seconde création avec callbacks échouait sur une grille de plus d'un morceau (`tests/unit_network_core.rs::second_creation_of_a_multi_chunk_grid`, rouge puis vert ; `src/network_chunks.rs`, taille gardée par URL dans le cache partagé). La séquence d'appels sur le GeoTIFF déflaté du CDN, mesurée avec `libproj` 9.9.0 par callbacks qui journalisent, est reprise par `tests/unit_network_compressed_cdn.rs` (création `open(url, 0, 16384)`, trois en-têtes, `close` ; points `open(url, 16384, 16384)` et `read_range(344064, 131072)`), avec les valeurs de `libproj` ; elle était lue entière (28 `read_range`). Écarts : `docs/PORTING-network.md`. |
| `networking.download_whole_files` | `tests/unit_network_download.rs::download_whole_files` | porté, contre le vrai CDN : les 37 assertions une à une (section « download_whole_files (porté) ») ; valeurs de PROJ 9.9.0 mesurées avec `libproj`. |

Tests unitaires ajoutés hors liste C++ (`src/network.rs`, `mod callback_tests`) :
la lecture entière d'une grille par callbacks en plages de 16384 octets
(`open` puis `read_range` aux offsets 16384 et 32768, un seul `close`), et
l'extraction de la taille depuis `Content-Range`.

Tests ajoutés pour le GeoTIFF compressé (hors liste C++, valeurs et séquences de `libproj`
9.9.0 mesurées sur les mêmes octets) : `tests/unit_network_compressed_grid.rs` (deux
grilles hors ligne, `data/tests/planar_deflate_predictor3.tif` en bandes et
`planar_deflate_predictor3_tiled.tif` en tuiles de 32 pixels, planaires, Deflate et
prédicteur 3, fabriquées par `tools/make-planar-deflate-grid.py`) et
`tests/unit_network_compressed_cdn.rs` (le vrai `nz_linz_nzgd2000-c120100904-grid01.tif`, par
requêtes `Range` au CDN, gardé par la sonde `cdn_reachable`). Le rouge a été vu avant le
correctif : création avec `read_range` jusqu'à la fin du fichier (16384 puis 15321 octets sur
le fichier hors ligne, 28 `read_range` sur le CDN). Le correctif rend aussi la taille des
morceaux demandés identique à celle de PROJ : `NetworkFile::read` ne tronque pas la plage à
la fin du fichier (`read_range(344064, 131072)` pour une grille de 460261 octets), ce que
faisait `RemoteFile::fetch`.

## Tableau complet des 26 TEST()

| # | TEST() C++ | Statut | Cause exacte si écart |
|---|---|---|---|
| 1 | `initial_check` | sans objet (0 assertion), retiré du registre | Décision du mainteneur : un TEST sans assertion n'a rien à porter, il sort du registre des tests ouverts sans test Rust nommé `initial_check`. Le TEST (lignes 90-111 de `test_network.cpp`) n'a aucun `ASSERT` ni `EXPECT` : c'est une sonde. `GET https://cdn.proj.org/fr_ign_ntf_r93.tif` avec `CURLOPT_RANGE` `0-1` (le format `%llu-%llu` de PROJ), puis `networkAccessOK = (code == 206)`. PROJ traite tout code `< 300` comme un succès pour une plage (`networkfilemanager.cpp`, lignes 1774 à 1786). Consommatrice : `basic` (lignes 143 et 169, `if (networkAccessOK)`), portée en entier (ligne 2). Rien à porter : sans assertion, il n'y a pas de valeur PROJ à reprendre. La sonde est reprise par `tests/net_cdn/mod.rs` (`cdn_reachable`, même requête, `206` attendu), utilisée par `tests/unit_network_basic.rs`, `tests/unit_network_download.rs` et `tests/unit_network_file_api.rs` : le CDN répond ici par `HTTPS_PROXY`, que le client HTTP suit depuis ce lot (`CONNECT`, `src/network.rs`). Le défaut de l'endpoint, `https://cdn.proj.org`, est confirmé contre le `libproj` 9.9.0 de référence (`proj_context_get_url_endpoint`, réseau désactivé par défaut). |
| 2 | `basic` | porté | Les quatre états et la valeur, contre le vrai CDN : voir « networking.basic (porté) ». |
| 3 | `curl_invalid_resource` | porté avec substitution | Substitution : un serveur local répond 404 à `/i_do_not.exist/my.tif`, au lieu de `https://i_do_not.exist/my.tif` (nom qui ne se résout pas). Même attente : `proj` renvoie une erreur, une seule requête (`tests/unit_network_local.rs`). |
| 4 | `custom` | porté | Voir « Cas portés ». |
| 5 | `getfilesize` | porté | Voir « Cas portés ». Mesuré avant le port : PROJ 9.9.0 fait `open(url, 0, 16384)`, `Content-Range`, `Last-Modified`, `ETag`, `close` à la création, puis aucun appel ; une transformation demande `open(url, 950272, 16384)` (sonde C). Le port lit les bandes à `StripOffsets`/`StripByteCounts` par morceaux, la dernière bande ne comptant que ses lignes, et `Content-Range` donne la taille même si la réponse est plus courte. Vérifié contre le fichier local, non commité : `test_hgrid_strip.tif` (4 lignes, bandes de 3, dernière bande d'une ligne) et `test_vgrid_bigendian.tif` (bande unique, gros-boutiste) donnent les mêmes bits sur un balayage du globe au degré (16 points dans la grille, aucun écart). |
| 6 | `simul_open_error` | porté | Voir « Cas portés ». |
| 7 | `simul_read_range_error` | porté | Voir « Cas portés ». |
| 8 | `simul_file_change_while_opened` | porté | Voir « Cas portés ». |
| 9 | `curl_hgridshift` | porté contre le vrai CDN | `fr_ign_gr3df97a.tif` lu sur `https://cdn.proj.org`, le TEST C++ entier (trois `EXPECT_NEAR`) et les valeurs de libproj 9.9.0 mesurées sur cette grille. Voir « Cas portés » et `docs/PORTING-network.md`. |
| 10 | `curl_vgridshift` | porté contre le vrai CDN | `us_nga_egm08_25.tif` (le nom que PROJ demande au CDN ; le commentaire C++ dit `egm08_25.tif`) lu sur le CDN, le TEST entier. |
| 11 | `curl_vgridshift_vertcon` | porté contre le vrai CDN | `us_noaa_vertcone.tif` (le nom demandé au CDN ; le commentaire C++ dit `vertcone.tif`) lu sur le CDN, le TEST entier. |
| 12 | `network_endpoint_env_variable` | porté avec substitution | Endpoint `http://127.0.0.1:<port fermé>/` au lieu de `http://0.0.0.0/` (même absence de serveur ; le test le dit lui-même). `tests/unit_network_endpoint_env.rs`. Voir « Cas portés ». |
| 13 | `network_endpoint_api_and_not_reachable_gridshift` | porté avec substitution | Endpoint `http://127.0.0.1:<port fermé>` au lieu de `http://0.0.0.0` (`proj_context_set_url_endpoint`). `tests/unit_network_core.rs::endpoint_api_not_reachable_gridshift`. Voir « Cas portés ». |
| 14 | `network_endpoint_api_and_not_reachable_xyzgridshift` | porté avec substitution | Même substitution d'endpoint que 13. `tests/unit_network_core.rs::endpoint_api_not_reachable_xyzgridshift`. |
| 15 | `network_endpoint_api_and_not_reachable_hgridshift` | porté avec substitution | Même substitution d'endpoint que 13. `tests/unit_network_core.rs::endpoint_api_not_reachable_hgridshift`. |
| 16 | `network_endpoint_api_and_not_reachable_vgridshift` | porté avec substitution | Même substitution d'endpoint que 13. `tests/unit_network_core.rs::endpoint_api_not_reachable_vgridshift`. |
| 17 | `cache_basic` | porté avec substitution | Serveur local au lieu du CDN : `/fr_ign_ntf_r93.tif`, grille synthétique (`grids::cache_grid()`) avec `Last-Modified` et `ETag`. La base est lue par `rusqlite` au lieu de `sqlite3`. `tests/unit_network_cache.rs::cache_basic` (base SQLite `cache.db`, tête LRU, second build sans requête). |
| 18 | `proj_grid_cache_clear` | porté avec substitution | Même serveur local et même grille que 17 ; `clear_grid_cache` supprime la base. `tests/unit_network_cache.rs::proj_grid_cache_clear`. |
| 19 | `cache_saturation` | porté avec substitution | Même serveur local que 17 ; limite de 32768 octets lue dans `PROJ_GRID_CACHE_MAX_SIZE_BYTES`, deux lignes dans `chunk_data`, `chunks` et `linked_chunks`. `tests/unit_network_cache.rs::cache_saturation`. |
| 20 | `cache_ttl` | porté avec substitution | Même serveur local que 17 ; les trois états (`Never`, `Seconds(86400)`, entrée expirée) sont assertés sur `properties` et `chunk_data`. `tests/unit_network_cache.rs::cache_ttl`. |
| 21 | `cache_lock` | porté avec substitution | Même serveur local que 17 ; verrou `BEGIN EXCLUSIVE` tenu par une seconde connexion, `PROJ_LOCK_MAX_ITERS=25`, durée d'au moins 650 ms. `tests/unit_network_cache.rs::cache_lock`. |
| 22 | `download_whole_files` | porté | `tests/unit_network_download.rs::download_whole_files`, contre le vrai CDN (`dk_sdfe_dvr90.tif`, 331904 octets). Les 37 assertions sont portées une à une (section « download_whole_files (porté) »). Écarts : `docs/PORTING-network.md`. |
| 23 | `file_api` | porté | `tests/unit_network_file_api.rs::file_api`, contre le vrai CDN : les 15 assertions (section « file_api (porté) »). L'API est `ProjBuilder::set_file_api`. Écarts : `docs/PORTING-network.md`. |
| 24 | `proj_coordoperation_get_grid_used` | porté contre le vrai CDN | Les trois assertions sont portées (nom, nom complet vide, URL `https://cdn.proj.org/...`), à chaque création, contre le fichier réel (`tests/unit_network_grid_used.rs`). Pas de drapeau `gridopen` dans l'API C 9.9.0. Séquence d'appels à la création sur le GeoTIFF déflaté : écart mesuré, non assertée. Voir « Cas portés dans ce lot ». |
| 25 | `pyproj_issue_1192` | porté en entier | `tests/unit_network_core.rs::pyproj_issue_1192`, contre le vrai CDN (`tests/net_cdn`, retour tôt si `networkAccessOK` échoue ; exécuté ici, la sonde répond). Asserté, réseau on puis off : la liste (10 opérations, noms et ordre), l'instanciabilité de chaque opération (off : 8 vraies, on : 10), et pour chaque opération instanciable l'id `pipeline` de `proj_pj_info` via `ProjBuilder::proj_from_operation` (ajouté par ce lot, équivalent de `proj_list_get`), y compris les opérations 00 et 02 dont la grille n'est pas sur disque ; avec des ouvertures refusées, zéro `open` à l'instanciation. Mesuré contre `libproj` 9.9.0 (sonde C, CDN joignable, cache de grilles off). Écarts (fabrique `IGNORED` avec réseau qui ouvre les grilles, opération non instanciable rendue en erreur au lieu d'un PJ sans id) : `docs/PORTING-network.md`. |
| 26 | `do_not_attempt_network_access_known_available_network_on` | porté | Voir « Cas portés dans ce lot ». La façade C reste absente : la fabrique passe par `ProjBuilder::create_operations`, qui applique le contexte du builder. |

Compte : 26 TEST() C++ ; 1 sans objet (`initial_check`, 0 assertion, retiré du registre) ; 14 portés (`basic`, `proj_coordoperation_get_grid_used`, `curl_hgridshift`, `curl_vgridshift`, `curl_vgridshift_vertcon`, `custom`, `getfilesize`, `simul_open_error`, `simul_read_range_error`, `simul_file_change_while_opened`, `download_whole_files`, `file_api`, `pyproj_issue_1192`, `do_not_attempt_network_access_known_available_network_on`) ; 11 portés avec substitution (`curl_invalid_resource`, `network_endpoint_env_variable`, les quatre `network_endpoint_api_*`, les cinq cas de cache des lignes 17 à 21) ; 0 partiel ; 0 écart.

## download_whole_files (porté)

Le TEST C++ (`test/unit/test_network.cpp:1859`, 37 lignes `EXPECT_`/`ASSERT_`) est porté dans
`tests/unit_network_download.rs::download_whole_files`, contre le vrai CDN : il télécharge
`https://cdn.proj.org/dk_sdfe_dvr90.tif` (331904 octets) dans le répertoire utilisateur d'un dossier
temporaire. Comme le TEST, il retourne tôt si le CDN ne répond pas (`networkAccessOK`). La sonde est la
requête de `initial_check` (`cdn_reachable` : `GET` de `fr_ign_ntf_r93.tif` avec `Range: bytes=0-1`,
`206` attendu), faite par le test lui-même et non par le client du crate : un téléchargement cassé ne
peut donc pas passer pour un CDN injoignable.

Ce qui bloquait, et ce qui a été levé :

1. Le client HTTP du crate ne suivait pas `HTTPS_PROXY`. Ici le CDN n'est joignable que par `CONNECT`
   (le DNS direct échoue). Ajouté dans `src/network.rs` (`connect`, `https_proxy`, `open_tunnel`,
   `bypasses_proxy`). Sans lui, la sonde échouerait et le cas rendrait un succès vide.
2. Les API manquaient : `proj_is_download_needed`, `proj_download_file` avec progression,
   `pj_find_file` sans réseau, `lookForGridInfo` avec `found` et `gridAvailable`, `proj_grid_info`
   avec le réseau du contexte. Ajoutées sur `ProjBuilder` (`src/network_download.rs`, `src/proj.rs`),
   avec `TtlSetting`, `DownloadProgress` et `GridLookup` (`src/iso19111/factory.rs`).
3. Les octets : le fichier est sur le CDN et le test le télécharge. Aucun fichier de substitution
   n'est fabriqué ni copié dans le dépôt.

Mesures de PROJ 9.9.0 (`libproj` 9.9.0 TIFF et curl, sonde C hors dépôt, `PROJ_DATA` de la référence,
réseau par `HTTPS_PROXY`) :

- `proj_grid_info("dk_sdfe_dvr90.tif")`, réseau activé : `filename` vide, `gridname`
  `dk_sdfe_dvr90.tif`, `format` `gtiff`.
- Avant le téléchargement : `proj_is_download_needed` 1 ; `pj_find_file(disable_network)` 0, chaîne
  vide ; `lookForGridInfo` faux ; `gridAvailable` faux.
- `proj_download_file` sans rappel : 1. Le fichier fait 331904 octets. Ensuite `pj_find_file` 1, et
  `proj_create` du pipeline `vgridshift` réussit.
- `z` = 36.590999603271491 (lon 12, lat 56).
- `lastChecked = 0`, puis `proj_is_download_needed(…, true)` : 0, et `lastChecked` reste 0 ;
  `(…, false)` : 0, et `lastChecked` est mis à jour ; `lastModified = 'foo'`, puis `(…, false)` : 1.
- `proj_download_file` avec rappel : 1, trois rappels. Pourcentages 0.60258387967605087 (200000
  octets), 0.90387581951407636 (300000), 1 (331904). Le premier morceau n'est pas rapporté.

Les 37 assertions du TEST, une à une (ligne du C++, assertion, équivalent du port, statut) :

| Ligne C++ | Assertion | Équivalent dans `tests/unit_network_download.rs` | Statut |
|---|---|---|---|
| 1875 | `proj_grid_info` : `filename` vide | `grid_info.filename.is_none()` | porté |
| 1876 | `proj_grid_info` : `gridname` = `dk_sdfe_dvr90.tif` | `grid_info.gridname` | porté |
| 1877 | `proj_grid_info` : `format` = `gtiff` | `grid_info.format` | porté |
| 1889 | `lookForGridInfo` faux (pas de ligne en base) | `!before.in_database` | porté |
| 1892 | `gridAvailable` faux avant téléchargement | `!before.info.available` | porté |
| 1896 | `proj_is_download_needed` vrai | `is_download_needed(GRID, Respect)` | porté |
| 1899 | `pj_find_file(disable_network)` faux | `find_local_file(GRID).is_none()` | porté |
| 1902 | chaîne de sortie vide | même condition : pas de chemin | porté |
| 1904 | `proj_download_file` vrai | `download_file(GRID, Respect, None)` réussit | porté |
| 1907 | `pj_find_file` vrai après téléchargement | `find_local_file(GRID)` renvoie un chemin | porté |
| 1910 | chemin non vide | `!local.as_os_str().is_empty()` | porté |
| 1914 | `lookForGridInfo` faux après téléchargement (pas de ligne) | `!after.in_database` | porté |
| 1917 | `gridAvailable` vrai après téléchargement | `after.info.available` | porté |
| 1920 | le fichier `dk_sdfe_dvr90.tif` s'ouvre | `File::open(dir.join(GRID))` | porté |
| 1932 | `proj_create` du pipeline `vgridshift` réussit | `ctx.clone().proj(PIPELINE)` | porté |
| 1939 | `z` = 36.5909996032715 à 1e-10 près | `convert_xyz(12, 56, 0)`, `(z - EXPECTED_Z).abs() <= 1e-10` | porté |
| 1944 | `proj_is_download_needed` faux après téléchargement | `is_download_needed(GRID, Respect)` | porté |
| 1950 | `cache.db` ouvrable | `rusqlite::Connection::open` sur `grid_cache_path()` | porté |
| 1956 | `prepare` de `UPDATE ... lastChecked = 0` non nul | `execute` (erreur sinon) | porté |
| 1957 | `UPDATE` exécuté (`SQLITE_DONE`) | `execute` renvoie `Ok` | porté |
| 1963 | TTL ignoré : `proj_is_download_needed(…, true)` faux | `is_download_needed(GRID, Ignore)` | porté |
| 1969 | `cache.db` ouvrable (second accès) | `Connection::open` | porté |
| 1975 | `SELECT lastChecked` : préparation non nulle | `query_row` | porté |
| 1976 | `SELECT lastChecked` : ligne (`SQLITE_ROW`) | `query_row` renvoie une ligne | porté |
| 1977 | `lastChecked` vaut 0 | `assert_eq!(last_checked, 0)` | porté |
| 1983 | recontrôle sur le CDN : faux, `lastChecked` expiré | `is_download_needed(GRID, Respect)` | porté |
| 1989 | `cache.db` ouvrable (troisième accès) | `Connection::open` | porté |
| 1995 | `SELECT lastChecked` : préparation non nulle | `query_row` | porté |
| 1996 | `SELECT lastChecked` : ligne | `query_row` | porté |
| 1997 | `lastChecked` mis à jour (non nul) | `assert_ne!(last_checked, 0)` | porté |
| 2007 | `UPDATE ... lastModified = 'foo'` : préparation | `execute` | porté |
| 2008 | `UPDATE` exécuté | `execute` | porté |
| 2013 | `proj_is_download_needed` vrai après `lastModified` invalide | `is_download_needed(GRID, Respect)` | porté |
| 2026 | `proj_download_file` avec progression vrai | `download_file(GRID, Respect, Some(&mut progress))` | porté |
| 2028 | trois rappels de progression | `calls.len() == 3` | porté |
| 2029 | le contexte reçu par le rappel est celui du téléchargement | `ptr::eq(downloader, &ctx)` sur chaque rappel : le rappel reçoit le `&ProjBuilder` | porté, adapté |
| 2030 | dernier pourcentage = 1.0 | `pcts.last() == Some(1.0)` | porté |

Le test ajoute deux assertions qui ne sont pas dans le TEST : les deux pourcentages intermédiaires,
mesurés sur `libproj` 9.9.0 (0.60258387967605087 et 0.90387581951407636), à 1e-15 près.

Écarts et choix du port (détail dans `docs/PORTING-network.md`) : le client HTTP n'envoie pas de
`Range`, donc la vérification et le téléchargement lisent le fichier entier en une requête ; le
contexte est un `ProjBuilder` (le cas C++ bascule le contexte par défaut) ; `TtlSetting::Ignore` ne
fait aucune requête, comme le code de PROJ (son commentaire d'en-tête dit l'inverse) ; la branche sans
ligne de `lookForGridInfo` ne lit que le disque.

## file_api (porté)

Le TEST C++ (`test/unit/test_network.cpp:2043`) est porté dans `tests/unit_network_file_api.rs::file_api`,
contre le vrai CDN, avec la même garde que `download_whole_files` (sonde partagée :
`tests/net_cdn/mod.rs`, la requête de `initial_check`). Le TEST installe un `PROJ_FILE_API` à dix
callbacks, télécharge `dk_sdfe_dvr90.tif` à travers eux, puis lit la grille à travers eux, réseau coupé.
Le port : `ProjBuilder::set_file_api` (`src/file_api.rs`), avec le trait `FileApi` (`open`, `exists`,
`mkdir`, `unlink`, `rename`) et le trait `ApiFile` (`Read + Write + Seek`). Le cas enregistre les
appels par nom (`open`, `read`, `write`, `seek`, `close`, `exists`, `mkdir`, `unlink`, `rename`) au lieu
des drapeaux `in_*` ; chaque callback est `std::fs`.

Séquence mesurée avec `libproj` 9.9.0 (sonde C hors dépôt, non versionnée ; TIFF et curl ;
`PROJ_FULL_FILE_CHUNK_SIZE=30000` ; répertoire utilisateur absent) :

- téléchargement : `open` en lecture seule de la cible (vérification de `is_download_needed`,
  `networkfilemanager.cpp:2541`), `exists` du répertoire puis de son parent, `mkdir`, deux autres
  `open` en lecture seule (non identifiés), `open` en création, 12 `write` (11 de 30000 octets, puis
  1904), `close`, `unlink` de la cible, `rename` du temporaire, `exists` du répertoire (ouverture de
  `cache.db`). Aucun `read`. Le port vérifie la présence de la cible par `exists`, sans ces `open` ;
- lecture : `proj_create` réseau coupé, puis la transformation : `open` en lecture seule, `seek`,
  `tell`, `read` par tuiles, `close` ; `z` = 36.590999603271491, comme `download_whole_files`.

Les 15 assertions du TEST, une à une :

| Ligne C++ | Assertion | Équivalent dans `tests/unit_network_file_api.rs` | Statut |
|---|---|---|---|
| 2137 | `proj_context_set_fileapi` vrai | `set_file_api` (ne renvoie rien) | sans objet : le trait est complet à la compilation ; PROJ renvoie `0` si un callback manque (`filemanager.cpp:1197`) |
| 2139 | `proj_is_download_needed` vrai | `is_download_needed(GRID, Respect)` | porté |
| 2141-2142 | `proj_download_file` vrai | `download_file(GRID, Respect, None)` réussit | porté |
| 2144 | `in_open` | `seen("open")` | porté |
| 2145 | `!in_read` après le téléchargement | `!seen("read")` | porté |
| 2146 | `in_write` | `seen("write")` | porté |
| 2147 | `in_close` | `seen("close")` (le `Drop` du fichier) | porté |
| 2148 | `in_exists` | `seen("exists")` | porté |
| 2149 | `in_mkdir` | `seen("mkdir")` | porté |
| 2150 | `in_unlink` | `seen("unlink")` | porté |
| 2151 | `in_rename` | `seen("rename")` | porté |
| 2162 | `proj_create` non nul | `proj(PIPELINE)` réussit | porté |
| 2169 | `z` = 36.5909996032715 à 1e-10 | `convert_xyz(12, 56, 0)`, `\|z - EXPECTED_Z\| <= 1e-10` | porté |
| 2173 | `in_read` | `seen("read")` | porté |
| 2174 | `in_seek` | `seen("seek")` | porté |

Écarts (détail dans `docs/PORTING-network.md`) : `close` est le `Drop` du fichier, et `user_data` est
l'état du trait. La lecture d'une grille fait un `seek` à la fin, un `seek` au début, puis `read` (comme
`FileManager::open` pour `proj.ini`, `filemanager.cpp:2049`), au lieu des `seek`/`read` par tuiles de
libtiff. Les opérations restées sur `std::fs` sont listées dans `docs/PORTING-network.md`.

## networking.basic (porté)

Le TEST C++ (`test/unit/test_network.cpp:121`) est porté en entier dans `tests/unit_network_basic.rs::basic`,
contre le vrai fichier du CDN, `https://cdn.proj.org/fr_ign_ntf_r93.tif`. Il enchaîne quatre états du même
pipeline (`hgridshift` sur ce fichier) et une valeur :

| Ligne C++ | Assertion | Équivalent dans `tests/unit_network_basic.rs` | Statut |
|---|---|---|---|
| 90-111 | `initial_check` : `networkAccessOK` (`206` attendu) | `cdn_reachable()` (`tests/net_cdn`), sonde sans assertion | porté |
| 133 | `ASSERT_EQ(P, nullptr)`, réseau off par défaut | `!network_enabled()`, `proj(PIPELINE).is_err()` | porté |
| 143-145 | `PROJ_NETWORK=ON` : `ASSERT_NE(P, nullptr)` si `networkAccessOK` | `ProjBuilder::new()` après `PROJ_NETWORK=ON`, `is_ok()` si la sonde répond | porté |
| 158 | `ASSERT_EQ(P, nullptr)` après `PROJ_NETWORK=` (vide) | `is_err()` sur un builder neuf | porté |
| 169-175 | réseau par l'API : `ASSERT_NE(P, nullptr)` si `networkAccessOK`, sinon `ASSERT_EQ(P, nullptr)` et retour | `enable_network(true)`, `is_ok()` ou `is_err()` | porté |
| 180 | `EXPECT_NEAR(longitude, 1.9992776848, 1e-10)` | `convert((2.0, 49.0))`, `(lon - 1.9992776848).abs() <= 1e-10` | porté |
| 181 | `EXPECT_NEAR(lat, 48.9999322600, 1e-10)` | même transformation, `(lat - 48.9999322600).abs() <= 1e-10` | porté |

Valeurs de PROJ 9.9.0, mesurées avec `libproj` 9.9.0 (sonde C hors dépôt, mêmes quatre états, TIFF et curl,
CDN joint par `HTTPS_PROXY`) : `proj_create` renvoie `NULL` (erreur 1029, fichier de grille introuvable ou
invalide) hors réseau et après `PROJ_NETWORK=` vide ; il renvoie un pipeline avec `PROJ_NETWORK=ON` et avec
l'API ; la transformation de (2, 49) donne lon `1.9992776847752205`, lat `48.999932259999646`. Le port
donne la même longitude, au bit près.

Ce qui a été fait, dans l'ordre : le test a été écrit, puis lancé avant toute modification du code. Il a passé
du premier coup : le port existant (grille par le CDN, `PROJ_NETWORK`, `enable_network`) suffisait, et aucune
API n'a été ajoutée. Pour vérifier que l'assertion de valeur est active, la valeur attendue a été
remplacée par `1.9992776858` : le test échoue (`longitude 1.9992776847752205 against PROJ 1.9992776858`),
puis la valeur de PROJ est rétablie.

Écarts (détail dans `docs/PORTING-network.md`) : `PROJ_NETWORK` est posé par `set_var`, vide pour « absent »
comme `putenv` ; le cas est le seul test de son binaire, donc sans course sur l'environnement ; chaque état
est un `ProjBuilder` (pas le contexte par défaut) ; le répertoire utilisateur est un dossier temporaire ; le
cache de grilles est coupé pour les états 1, 3 et 4, comme dans le TEST. `proj_cleanup` n'a pas d'équivalent :
`ProjBuilder::new` relit `PROJ_NETWORK` à chaque appel.

Le cas local `tests/unit_network_local.rs::basic_network_is_off_by_default_and_makes_no_request` reste en
place, hors liste : il vérifie qu'un état « off » n'envoie aucune requête, ce que le CDN réel ne permet pas
d'observer.

## Commandes

```
cargo test --test unit_network_core --test unit_network_endpoint_env --test unit_network_local --test network_grid
cargo test --test unit_network_basic
cargo test --test unit_network_grids
cargo test --test unit_network_download
cargo test --test unit_network_file_api
cargo test --lib callback_tests
```

## Cache et projsync (lot network-cache-projsync)

Fiche : `docs/PORTING-network-cache.md`. Les lignes du tableau des 26 TEST() ci-dessus
sont à jour pour ces cas ; ce qui suit détaille les cas de ce lot.

| TEST() C++ | Test Rust | Statut |
|---|---|---|
| `networking.cache_basic` | `tests/unit_network_cache.rs::cache_basic` | porté, sur serveur local : tête LRU = chunk 0, un second build ne fait aucune requête. Écart : fichier entier, pas de lecture par plages (fiche, point 1). |
| `networking.proj_grid_cache_clear` | `tests/unit_network_cache.rs::proj_grid_cache_clear` | porté : la base est créée par un build, supprimée par `clear_grid_cache`. |
| `networking.cache_saturation` | `tests/unit_network_cache.rs::cache_saturation` | porté : limite de 2 chunks par `PROJ_GRID_CACHE_MAX_SIZE_BYTES=32768`, 2 lignes dans `chunk_data`, `chunks` et `linked_chunks`. |
| `networking.cache_ttl` | `tests/unit_network_cache.rs::cache_ttl` | porté : TTL négatif = données corrompues utilisées (build en échec), TTL 86400 = téléchargement de nouveau, `lastChecked`, `lastModified` et `etag` mis à jour. |
| `networking.cache_lock` | `tests/unit_network_cache.rs::cache_lock` | porté : build sans cache après 25 essais, durée au moins 650 ms (calendrier de PROJ). Écart : PROJ vérifie `>= 1` s en secondes entières (fiche, point 3). |
| `networking.curl_hgridshift` | `tests/unit_network_grids.rs::curl_hgridshift` | porté contre le vrai CDN (`fr_ign_gr3df97a.tif`) : trois assertions C++ aux tolérances de PROJ, plus les valeurs de libproj 9.9.0 mesurées à 1e-9. `proj_assign_context` non porté (point 5 de la fiche). |
| `networking.curl_vgridshift` | `tests/unit_network_grids.rs::curl_vgridshift` | porté contre le vrai CDN (`us_nga_egm08_25.tif`) : trois assertions C++ et la hauteur mesurée `-31.889799118042117`. |
| `networking.curl_vgridshift_vertcon` | `tests/unit_network_grids.rs::curl_vgridshift_vertcon` | porté contre le vrai CDN (`us_noaa_vertcone.tif`) : trois assertions C++ et la hauteur mesurée `-0.15375280380249004`. |

### projsync (`src/apps/projsync.cpp`, `test/cli/test_projsync.sh`)

Porté : `src/bin/projsync.rs` (options de `projsync.cpp`, plus `--help`, `--version` et
`--user-directory`), bibliothèque `src/projsync.rs` (analyseur JSON sans dépendance avec la
limite de profondeur 128, index, sélection, téléchargement avec enregistrement dans `cache.db`).
Fiche : `docs/PORTING-network-cache.md`, points 7 à 9.

`tests/unit_projsync.rs` : 22 tests, tous passés le 2026-11-15 (`cargo test --test unit_projsync` :
`test result: ok. 22 passed; 0 failed; 0 ignored`). Deux tests ont été ajoutés pour des
invocations du script que la suite ne couvrait pas : `default_endpoint_is_the_cdn` (sans
`--endpoint`) et `default_target_is_the_user_writable_directory` (sans `--target-dir`, avec
`PROJ_USER_WRITABLE_DIRECTORY`).

Table de couverture : chaque `$EXE ...` de `test/cli/test_projsync.sh` (212 lignes, 15 appels).

| Ligne du script | Invocation | Test Rust | Statut |
|---|---|---|---|
| 35 (contrôles 44-51) | `--list-files --target-dir D` ; `cache.db` et `files.geojson` présents | `list_files_fetches_the_index_and_records_it` (liste exacte, 7 lignes) | porté |
| 54 | `--all --dry-run` | `all_dry_run_lists_every_file` | porté |
| 63 | `--source-id fr_ign --dry-run` | `source_id_selects_by_part_of_the_id` | porté |
| 72 | `--source-id non_existing` (code 1, avertissement, valeurs connues) | `unknown_source_id_fails_and_lists_the_known_ones` (stderr exact) | porté |
| 82 | `--area-of-use France --dry-run` | `area_of_use_selects_by_part_of_the_name` | porté |
| 91 | `--area-of-use non_existing` (code 1) | `unknown_area_of_use_fails_and_lists_the_known_ones` (stderr exact) | porté |
| 101 | `--file ntf_r93` (téléchargement) | `file_downloads_once_and_then_says_it_is_already_there`, premier passage | porté |
| 114 | `--file ntf_r93` (second passage) | même test, « already downloaded » | porté |
| 125 | `--file non_existing` (code 1) | `unknown_file_fails_and_lists_the_known_ones` (stderr exact) | porté |
| 135 | `--bbox 2,49,3,50 --dry-run` | `bbox_selects_the_grids_that_overlap_it` | porté |
| 145 | `--bbox 2,49,3,50 --exclude-world-coverage --dry-run` | `exclude_world_coverage_drops_the_world_grids` | porté |
| 158, 168, 178 | `--bbox 170,-90,-170,90`, `170,-90,190,90`, `-190,-90,-170,90` | `bbox_across_the_antimeridian_finds_the_pacific_grids` (les trois fenêtres) | porté |
| 198 | `--source-id au_ga --verbose --dry-run --local-geojson-file dummy.geojson` | `verbose_reports_the_files_left_out_by_version` | porté |
| 54 à 206, après `PROJ_USER_WRITABLE_DIRECTORY` (l. 42) | sans `--target-dir` : la cible est le répertoire utilisateur | `default_target_is_the_user_writable_directory` (ajouté) | porté |
| 54 à 206 | sans `--endpoint` : `https://cdn.proj.org` | `default_endpoint_is_the_cdn` (ajouté) | porté |

Le script saute tout son contenu si `cdn.proj.org` ne répond pas (l. 22-27). Les tests Rust
n'ont pas ce garde-fou : ils tournent sur serveur local et sur `tests/projsync/files.geojson`.

Vérification contre le binaire de référence (PROJ 9.9.0 avec curl et TIFF,
`target/proj-reference-net/build/bin/projsync`) : les 15 appels ont été joués, avec
`--endpoint https://cdn.proj.org --local-geojson-file tests/projsync/files.geojson`, sur la
référence et sur le port (`target/debug/projsync`). Sortie standard, sortie d'erreur et code de
sortie sont identiques pour les 15. Le téléchargement réel (`--file ntf_r93`) a aussi été joué sur
les deux : le CDN répond ici par `HTTPS_PROXY`, et les deux fichiers reçus sont identiques
(93581 octets, `cmp`). Les deux cas par défaut (sans `--endpoint`, avec
`PROJ_USER_WRITABLE_DIRECTORY`) donnent la même sortie sur les deux binaires. Les constantes
attendues de `tests/unit_projsync.rs` (`LIST`, `ALL`, `UNKNOWN_*`, `VERBOSE_DUMMY`) sont les
sorties de la référence, à l'endpoint près.

Écarts, avec leur cause :
- téléchargement non conditionnel : un fichier dont l'enregistrement a expiré est téléchargé de
  nouveau, sans `If-None-Match` (fiche cache, points 2 et 8) ;
- `--system-directory` prend le premier répertoire de `PROJ_DATA` ou `PROJ_LIB`, pas le répertoire
  « share » compilé de PROJ (fiche cache, point 7) ;
- index : `tests/projsync/files.geojson`, de même forme que le `files.geojson` du CDN, pas celui du
  CDN (fiche cache, point 9). La comparaison porte sur cet index, pas sur l'index du CDN.

Commande : `cargo test --test unit_projsync --test unit_network_cache --test unit_network_grids`
