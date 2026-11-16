# Fiche de portage : réseau (test_network.cpp, projsync)

Audit détaillé : `docs/AUDIT-network.md`. Fiche du lot gie et réseau antérieur :
`docs/PORTING-gie-network.md`.

## Carte

| Source PROJ 9.9.0 | Rust |
|---|---|
| `src/filemanager.cpp` (téléchargement, cache), `src/networkfileapi` | `src/network.rs` (interne, `pub(crate)`) |
| `proj_context_set_network_callbacks` | `NetworkCallbacks` (trait public), `ProjBuilder::set_network_callbacks`, `src/network.rs` |
| `NetworkFile`, `DOWNLOAD_CHUNK_SIZE`, cache de morceaux (`filemanager.cpp`) | `src/network_chunks.rs` (`RemoteFile`, `ChunkCache`, `RemoteReader`) |
| `GTiffGrid`, lecture des tuiles au calcul (`grids.cpp`, libtiff) | `src/projections/gridshift/geotiff.rs` (`load_remote`, `lazy_layout`, `raw_samples`), `src/projections/gridshift/tiles.rs` (`LazyIfd`, `read_raw`, `read_decoded`, `with_tiles`) |
| `proj_context_set_url_endpoint`, `PROJ_NETWORK_ENDPOINT` | `ProjBuilder::set_url_endpoint`, `GridContext::load` |
| `grids.cpp:3222` (`pj_generic_grid_init`), `grids.cpp:3234` | `src/projections/gridshift/mod.rs` (`deferred`, `Deferred`) |
| `trans.cpp:284` (erreur réseau d'un candidat : arrêt du point) | `src/iso19111/alternatives.rs` (`AlternativeError::Network`) |
| `proj_trans_get_last_used_operation`, `proj_pj_info` | `Proj::last_used_operation`, `Proj::proj_info` (`candidate_name`) |
| `DatabaseContext::lookForGridInfo` (`factory.cpp:3400`) | `src/iso19111/factory.rs` (`look_for_grid_info`), `src/iso19111/select.rs` (`unrecorded_grid`, `RemoteGrids`), `src/network.rs` (`open_remote_grid`, la branche réseau de `open_resource_file`, `filemanager.cpp:1859-1915`) |
| `proj_coordoperation_get_grid_used_count`, `proj_coordoperation_get_grid_used` (`c_api.cpp:7905-7990`) | `Proj::grids_needed` (`src/proj.rs`), sous `with_context` ; `RemoteGrids::Open` |
| `proj_create_operations` avec le contexte de fabrique (callbacks du contexte) | `ProjBuilder::create_operations` (`src/proj.rs`), enveloppe `with_context` comme les autres créations du builder |
| `src/apps/gie.cpp` (`network_error`) | `src/gie.rs` (`errno_name`) |
| `test/unit/test_network.cpp` (26 TEST) | `tests/unit_network_core.rs` (14 cas, dont `custom`, `getfilesize`, `simul_read_range_error`, `simul_file_change_while_opened`, `pyproj_issue_1192`, `do_not_attempt_network_access_known_available_network_on` et `second_creation_of_a_multi_chunk_grid`, cas propre au port), `tests/unit_network_grid_used.rs` (1 cas, `proj_coordoperation_get_grid_used`, contre le vrai CDN), `tests/unit_network_compressed_grid.rs` (2 cas hors liste : GeoTIFF déflaté planaire, en bandes et en tuiles, hors ligne), `tests/unit_network_compressed_cdn.rs` (1 cas hors liste : le GeoTIFF déflaté du CDN, contre le vrai CDN), `tests/unit_network_endpoint_env.rs` (1 cas), `tests/unit_network_local.rs` (2 cas), `tests/unit_network_grids.rs` (3 cas, contre le vrai CDN), `tests/unit_network_cache.rs` (5 cas, serveur local), `src/network.rs` `callback_tests` (2 tests hors liste) |
| serveur de test local (remplace `cdn.proj.org`) | `tests/net_support/mod.rs` |
| `proj_is_download_needed`, `proj_download_file` (`networkfilemanager.cpp:2523-2880`) | `src/network_download.rs` (`is_download_needed`, `download`) ; `ProjBuilder::is_download_needed`, `ProjBuilder::download_file`, `TtlSetting`, `DownloadProgress` (`src/proj.rs`) |
| `pj_find_file(…, disable_network)` (`filemanager.cpp:1942`), `proj_grid_info` avec le réseau du contexte (`grids.cpp:3928`), `lookForGridInfo` et ses deux résultats (`factory.cpp:3398`) | `ProjBuilder::find_local_file`, `ProjBuilder::grid_info`, `ProjBuilder::look_for_grid_info` (`GridLookup`, `src/iso19111/factory.rs`, `grid_lookup`) |
| `HTTPS_PROXY` / `https_proxy` (curl, tunnel `CONNECT`) | `src/network.rs` (`connect`, `https_proxy`, `open_tunnel`, `bypasses_proxy`) |
| `proj_context_set_fileapi`, `PROJ_FILE_API`, `FileManager::open/exists/mkdir/unlink/rename` (`filemanager.cpp:880-1100`, `1180-1213`) | `src/file_api.rs` (`FileApi`, `ApiFile`, `FileAccess`), `ProjBuilder::set_file_api` ; `src/network.rs` (`read_file`, `create_directory_all`, `create_for_write`, `remove_file`, `rename_file`, `is_file_with`) |
| `networking.download_whole_files` (`test_network.cpp:1859`, 37 assertions) | `tests/unit_network_download.rs` (1 cas, contre le vrai CDN) |
| `networking.file_api` (`test_network.cpp:2043`, 15 assertions) | `tests/unit_network_file_api.rs` (1 cas, contre le vrai CDN) |
| `networking.basic` (`test_network.cpp:121`, quatre états et une valeur) | `tests/unit_network_basic.rs` (1 cas, contre le vrai CDN) ; `tests/unit_network_local.rs` garde en plus le cas « off, aucune requête » |
| `networking.curl_hgridshift`, `curl_vgridshift`, `curl_vgridshift_vertcon` (`test_network.cpp:1109`, `1144`, `1180`) | `tests/unit_network_grids.rs` (3 cas, contre le vrai CDN) |
| sonde `initial_check` (`cdn_reachable`), partagée | `tests/net_cdn/mod.rs` (incluse par `#[path]` dans les deux cas) |
| `src/apps/projsync.cpp`, `test/cli/test_projsync.sh` | `src/bin/projsync.rs`, `src/projsync.rs` ; `tests/unit_projsync.rs` (22 cas, les 15 appels du script) ; cache et téléchargement : `docs/PORTING-network-cache.md` ; table de couverture : `docs/AUDIT-network.md`, section projsync |

## Décisions et écarts volontaires

- **Erreur réseau.** PROJ renvoie `HUGE_VAL` et `proj_errno == PROJ_ERR_OTHER_NETWORK_ERROR`.
  Ici, `Proj::convert_*` et `project` renvoient `Err(ProjError::Network)`, c'est-à-dire une
  valeur typée au lieu d'un sentinelle et d'un errno global. `TransformError::NetworkUnavailable`
  est le type interne, traduit en `ProjError::Network` au niveau de `Proj`.
  Mesuré pour une tuile refusée par `read_range` (`simul_read_range_error`, `libproj` 9.9.0) :
  `HUGE_VAL`, `proj_errno` 4099, et aucun appel après le `read_range` refusé (ni en-tête, ni
  nouvelle lecture) ; le handle reste ouvert jusqu'à `proj_destroy`.
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
- **Callbacks : lecture par morceaux, tuiles à la demande (`custom`).** La création ouvre le
  premier morceau, `open(url, 0, 16384)`, puis les en-têtes `Content-Range`, `Last-Modified`,
  `ETag`, puis `close` : c'est la séquence de PROJ à la création. Les morceaux lus sont gardés
  dans un cache partagé par les clones d'un même `ProjBuilder` (`ChunkCache`, 300 Mo, vidé en
  entier au-delà ; PROJ évince par âge). Une tuile est lue à son `TileOffsets` sur
  `TileByteCounts` octets (262144 pour `egm96_15`), pas sur la seule partie utile : le
  décodeur `tiff` lit 214016 octets pour la dernière rangée (721 lignes), ce qui donne une
  couverture de 229376 octets et non les 278528 de PROJ (mesuré). Le handle de la première
  tuile sert aux suivantes (`read_range`) et se ferme à la destruction de la grille (`Drop`) ;
  celui de la création se ferme avant la fin de `proj_create`.
- **Bandes lues par morceaux (`getfilesize`).** Une image en bandes non compressée est lue
  comme les tuiles (`Layout::Strips`) : la bande est lue à son `StripOffsets` sur
  `StripByteCounts` octets, la dernière bande ne donne que ses lignes, `RowsPerStrip` est
  plafonné à la hauteur (une bande unique peut valoir 2^32 - 1). `Content-Range` donne la taille
  du fichier même quand la réponse est plus courte : avant ce lot, une réponse courte fixait la
  fin du fichier, ce qui aurait bloqué la lecture à 550 octets. Sans `Content-Range`, la règle
  reste celle d'avant. Mesuré : une transformation après création demande `open(url, 950272,
  16384)` dans PROJ, le même bloc que le port.
- **GeoTIFF compressé lu par morceaux (`lazy_layout`).** Un GeoTIFF à tuiles ou à bandes, à
  échantillons entrelacés ou planaires, est lu par morceaux si sa compression est aucune (1), le
  Deflate (8) ou l'ancien Deflate (32946), et son prédicteur 1, 2 ou 3. Un morceau sans
  compression ni prédicteur est lu comme ses octets (`raw_samples`, `Reading::Raw`), comme avant.
  Un morceau compressé, ou prédit, est lu par le décodeur `tiff` (`Reading::Decoded`) : celui-ci
  annule la compression et le prédicteur. Les octets du morceau sont lus une fois, par un seul
  `read_at` (arrondi aux morceaux de 16384 octets : c'est ce que PROJ mesure, un `read_range` par
  bande ou tuile), puis donnés au décodeur par une fenêtre (`RemoteReader::set_window`) qui ne lit
  rien d'autre : hors de la fenêtre, la lecture est la fin des données. La création ne lit que
  `open(url, 0, 16384)` et les en-têtes, y compris pour le GeoTIFF déflaté du CDN.
  Mesuré, hors ligne et sur le CDN, appels et valeurs égaux à `libproj` 9.9.0 (voir « GeoTIFF
  compressé, mesures »). Limites : les autres compressions (PackBits, JPEG, CCITT) et les
  prédicteurs autres que 1 à 3 restent lus entiers par `read_whole` (LZW : voir « GeoTIFF
  compressé, mesures ») ; le prédicteur 2 et l'ancien
  Deflate sont acceptés et ne sont pas mesurés contre `libproj`. JPEG et CCITT ne sont pas lus
  (« GeoTIFF JPEG et CCITT »). Un morceau de tuile partielle
  d'un plan autre que le premier est rendu par `tiff` 0.10.3 avec ses lignes de remplissage : le
  décodage ne prend que les lignes de l'image (`read_decoded`), car `chunk_data_dimensions` lit
  le remplissage du bas à partir de l'index du morceau et non de sa ligne dans le plan. La
  largeur, elle, est juste. Le décodage des bandes non compressées reste `raw_samples` (`u16`,
  `i16`, `u32`, `i32`, `f32`, `f64`, ordre d'octets du fichier) ; le décodeur `tiff` lit les IFD.
- **Taille de la plage demandée (`fetch`).** `NetworkFile::read` (`networkfilemanager.cpp:1349`)
  demande `nBlocs * 16384` octets à partir du morceau, sans tronquer à la fin du fichier : un
  `read_range(344064, 131072)` pour une grille de 460261 octets (mesuré). `RemoteFile::fetch`
  tronquait à la taille du fichier (`read_range(344064, 116197)`) ; il ne tronque plus. Un
  morceau qui commence après la fin n'est pas demandé, comme avant. Écart restant : PROJ
  augmente le nombre de blocs demandés pour des lectures séquentielles (doublement, plafonné à
  64 blocs) et s'arrête au premier bloc déjà en cache ; `fetch` demande la plage manquante
  entière, de son premier au dernier morceau manquant (sans heuristique). Les mesures ci-dessous
  n'ont pas de lecture séquentielle, donc pas de différence visible.
- **Nouvelle passe par point (`with_tiles`).** Un échantillon d'une tuile non lue donne `None`
  et la tuile est notée. Le point est recalculé une fois la tuile lue, si bien que
  l'interpolation ne voit jamais un échantillon absent. Une lecture en échec termine le point
  par `NetworkUnavailable`, comme une grille absente.
- **Fichier changé pendant l'ouverture (`simul_file_change_while_opened`).** Après chaque
  appel, les trois en-têtes sont demandés dans l'ordre de PROJ. La taille (`Content-Range`),
  `Last-Modified` et `ETag` d'une URL sont gardés dans le cache partagé du builder
  (`ChunkCache::properties`, par URL, comme `gNetworkFileProperties` de PROJ) : chaque réponse de
  n'importe quel `RemoteFile` de cette URL est comparée à eux. Une réponse qui diffère, pour un
  fichier qui n'a pas encore vu le changement, vide les morceaux de cette URL
  (`ChunkCache::forget_chunks`), met les propriétés à celles de la réponse, et marque le fichier
  `changed` (`network_chunks::RemoteFile::observe`). Les lectures de tuiles de la passe en cours se
  terminent, comme PROJ finit l'interpolation avant `hasChanged` (`tiles::with_tiles_watch`). La
  grille en vigueur est ensuite lâchée (son fichier se ferme), rechargée depuis `(url, 0, 16384)`
  (`with_reloads`, `GridSets::reload`), et le point est refait. Le cache de morceaux garde la tuile
  lue après le changement, donc le second passage ne fait aucun appel. Les trois en-têtes ne sont
  demandés qu'avant le changement : après un appel sur un fichier qui l'a vu, PROJ ne les demande
  plus (`m_hasChanged`, `networkfilemanager.cpp:1431`), et `observe` fait de même.

  Séquences mesurées avec `libproj` 9.9.0 (sonde C non versionnée, dans le scratchpad de la
  session ; fixture `egm96_15_uncompressed_truncated.tif`, tailles et en-têtes du test) :
  - même processus, cache de grilles désactivé ou activé, création 2 sur le même contexte ou sur un
    contexte neuf : création 2 sans appel ; premier calcul (lat 49, tuile 524288 non en cache) :
    `open(url, 524288, 278528)`, trois en-têtes du fichier changé, `close`, `open(url, 0, 16384)`,
    trois en-têtes, valeur 2.25 ; calcul sur l'ancienne opération (lat -49, tuile 3670016) :
    `open`, trois en-têtes, aucun rechargement, valeur 2.25 ; fermetures à `proj_destroy`.
  - deux processus, cache désactivé : création 2 fait `open(url, 0, 16384)` et lit les en-têtes du
    fichier changé, qui deviennent la référence ; le calcul ne détecte rien (`open` de la tuile,
    trois en-têtes, pas de rechargement), valeur 2.25.
  - deux processus, cache activé, TTL 86400 : création 2 sans appel (propriétés et morceau 0 lus dans
    `cache.db`) ; calcul comme dans le même processus (détection, rechargement).
  - deux processus, cache activé, TTL 1 s, création 2 après 2 s : propriétés expirées, la création 2
    relit `open(url, 0, 16384)` ; le calcul ne détecte rien, comme le cache désactivé.

  Ce lot porte les cas de même builder (`simul_file_change_between_two_creations`) et de deux
  builders en deux processus, cache désactivé (`simul_file_change_with_a_new_builder`). Les cas
  deux processus avec le cache disque, et les écarts, sont plus bas.
- **Forme du trait.** `open` renvoie les premiers octets avec un handle (`OpenReply`), au lieu
  d'un tampon et d'une chaîne d'erreur passés en paramètres. Les erreurs sont typées
  (`NetworkError::Open`, `ReadRange`). Les en-têtes passent par `header`, avec une valeur
  par défaut `None`.
- **Dernier candidat.** `last_used_operation` porte le nom du candidat choisi au dernier
  point, qui est la description de `proj_pj_info` en PROJ.
- **Fabrique d'opérations (`pyproj_issue_1192`).** Pas de façade C : le contexte de fabrique
  est `CoordinateOperationContext`, avec `GridAvailabilityUse::IgnoreGridAvailability` et
  `SpatialCriterion::PartialIntersection`, et la liste vient de
  `CoordinateOperationFactory::create_operations`. Le drapeau réseau de
  `proj_coordoperation_is_instantiable(ctx, P)` est traduit par
  `CoordinateOperation::is_proj_instantiable` : réseau activé donne `KnownAvailable`
  (`considerKnownGridsAsAvailable` : une grille que la base connaît compte comme disponible),
  réseau désactivé donne `UseForSorting` (grilles sur disque seulement). Mesuré (sonde C
  contre `libproj` 9.9.0, CDN joignable, cache de grilles off, non versionnée) : 10 opérations
  dans les deux cas, mêmes noms et même ordre ; instanciables 8 réseau off (opérations 00 et 02
  sans grille locale), 10 réseau on ; les chaînes PROJ sont identiques dans les deux cas.
- **Instanciation d'une opération (`ProjBuilder::proj_from_operation`, ajouté par ce lot).**
  Équivalent de `proj_list_get` (`pj_obj_create`) : la chaîne PROJ de l'opération est construite
  avec la base, puis lue comme `Proj::new`. Réseau actif : une grille absente mais téléchargeable
  n'est pas ouverte à la création, comme `defer_grid_opening` de PROJ (mesuré : zéro `open` pendant
  `proj_list_get`, `proj_coordoperation_is_instantiable` et `proj_pj_info`, avec des callbacks qui
  refusent tout). Le port : `grids_defer_at_creation` (`src/network.rs`), que lit
  `gridshift::deferred` pour sauter la tentative de création d'une grille téléchargeable ; les
  candidats de paire (`grids_may_be_deferred`) gardent leur comportement. Réseau inactif, ou grille
  ni sur disque ni téléchargeable : erreur (`ProjCreateError`). PROJ rend alors un `PJ` sans id ;
  écart d'API assumé, asserté par le test.
- **`Proj::proj_info` ne lit pas la grille (ajouté par ce lot).** `has_inverse` teste
  l'inverse en passant un point dans le pipeline, ce qui, sur une grille différée, la lisait
  (mesuré par le test : deux `open` au `proj_info`). `with_inverse_probe` fait répondre le
  placeholder pendant ce seul test, sans lire la grille ; PROJ ne fait aucune E/S dans
  `proj_pj_info`.
- **Fabrique `IGNORED` avec réseau (écart, non porté).** Avec `IGNORED` et le réseau actif,
  PROJ ouvre à la création les grilles distantes des lignes de la base : sonde C avec callbacks
  refusés, `proj_create_operations` pour `EPSG:4326` vers `EPSG:2964` fait deux `open`
  (`https://cdn.proj.org/ca_nrc_ntv2_0.tif`, `https://cdn.proj.org/us_noaa_alaska.tif`). Le port
  (`CoordinateOperationFactory::create_operations`) ne les ouvre pas. Aucun effet mesuré sur la liste
  ni sur les drapeaux (mêmes valeurs avec les ouvertures refusées). Les deux grilles ont une ligne
  avec `open_license = 1` (`proj.db`).
- **Hypothèse « sonder le CDN » (`is_proj_instantiable`, non changé).** Ce lot supposait que
  l'instanciabilité avec réseau actif devait sonder le CDN. Pour cette liste ce n'est pas le cas :
  `lookForGridInfo` (`iso19111/factory.cpp`) compte disponible une grille à ligne qui a un paquet
  ou une licence ouverte, et les deux grilles de la liste ont la licence ouverte. Le port fait la
  même chose (`KnownAvailable`), et le résultat mesuré est identique. La sonde de PROJ
  (`resolveFullFilename` avec le réseau actif, `filemanager.cpp`) vise les lignes sans paquet ni
  licence ouverte ; ce cas n'est pas dans la liste et n'est pas porté.
- **Fabrique sous le contexte du builder (`do_not_attempt_network_access_known_available_network_on`).**
  Les callbacks sont attachés au `ProjBuilder`, pas à `CoordinateOperationContext`, donc la
  fabrique ne les voit pas sans un passage par le builder. Ajout : `ProjBuilder::create_operations`,
  qui applique `with_context(&self.net, ...)` comme `proj_known_crs`. Sans cette enveloppe, un
  appel réseau éventuel de la fabrique échapperait aux callbacks et le compteur ne le verrait pas.
  Mesuré (sonde C contre `libproj` 9.9.0, `probe2.c` non versionnée) : `EPSG:4326` vers
  `EPSG:4267`, `KNOWN_AVAILABLE`, `PARTIAL_INTERSECTION`, réseau on : 79 opérations, les mêmes
  noms dans le même ordre qu'en réseau off, 0 appel de callback. Le test asserte la liste exacte
  (79 noms), plus forte que `EXPECT_GE(num_operations, 10)` de PROJ, qui reste aussi asserté.
  Témoin : `+proj=hgridshift +grids=proj_rs_no_such_grid.tif` donne `NULL` et un seul `open`
  en PROJ ; le port donne le même compte, ce qui prouve que le compteur est câblé. Aujourd'hui
  la fabrique ne fait aucun appel réseau (les grilles sont cherchées sur disque) : le test
  garde cette propriété, il ne prouve pas qu'un appel existe ailleurs.
  Écarts : (a) `ASSERT_TRUE(proj_context_set_network_callbacks(...))` : `set_network_callbacks`
  ne renvoie rien et ne peut pas échouer ; l'assertion n'a pas d'équivalent, l'état réseau est
  vérifié par `network_enabled()`. (b) L'endpoint est une adresse locale sans serveur, comme les
  autres cas de ce lot, au lieu de `cdn.proj.org` : une fabrique qui tenterait un accès passerait
  par les callbacks, rien n'atteint donc le réseau.
- **Grille sans ligne, ouverte par le réseau (`proj_coordoperation_get_grid_used`).** Le TEST
  veut `url` = `https://cdn.proj.org/nz_linz_nzgd2000-c120100904-grid01.tif` pour une grille que
  `proj.db` ne connaît pas (aucune ligne pour `c120100904`, mesuré sur les 479 lignes de
  `grid_alternatives`). PROJ n'obtient cette URL que si `open(endpoint/nom)` réussit dans
  `FileManager::open_resource_file` : `lookForGridInfo` prend alors l'URL, `fullFilename` est vidé,
  et `gridAvailable` vaut vrai (`factory.cpp`, branche sans ligne). Le port fait la même chose
  dans `unrecorded_grid` avec `RemoteGrids::Open`, uniquement dans `Proj::grids_needed`, sous le
  contexte du builder. La sonde est `network::open_remote_grid` : avec callbacks, `read_at(0, 16384)`,
  donc `open(url, 0, 16384)` puis `close`, comme PROJ ; sans callbacks, un `GET` du fichier entier
  (le client HTTP du crate n'envoie pas de `Range`). Mesuré : sans octets servis, `proj_create`
  renvoie `NULL` (« could not find required grid(s) »), donc la création sert de preuve que la
  grille est ouverte. Les autres appelants (`factory.rs`, `compound.rs`, `projinfo.rs`,
  `is_proj_instantiable`) gardent `RemoteGrids::Skip` : la fabrique coupe le réseau
  (`factory.cpp:4363`, `DisableNetwork`), et ce lot ne change pas ses listes.
  **Contre le vrai CDN.** `tests/unit_network_grid_used.rs` interroge `cdn.proj.org` comme le
  TEST C++, sans callbacks (le client HTTP du crate, `HTTPS_PROXY` suivi). Il est gardé par la
  sonde `cdn_reachable` : sans réponse `206`, il sort tôt, comme `networkAccessOK`. Répertoire
  utilisateur temporaire, cache de grilles désactivé. Les trois assertions du TEST sont là, et
  deux créations. Mesuré avec `libproj` 9.9.0 (TIFF+curl, `PROJ_DATA` de la référence) sur le même
  fichier de 460261 octets (GeoTIFF déflaté, 55×52) : `proj_create` non NULL aux deux créations,
  `shortName` le nom, `fullName` vide, `packageName` vide, `url` le CDN, `directDownload` 0,
  `openLicense` 0, `available` 1, un seul grid. Le port donne les mêmes valeurs. Il n'y a pas de
  drapeau `gridopen` dans l'API C 9.9.0 (`proj_coordoperation_get_grid_used` ne renvoie que
  `available`).
  **Séquence d'appels sur le vrai fichier (mesurée, non assertée).** PROJ, callbacks qui
  journalisent : création 0 `open(url, 0, 16384)`, trois en-têtes, `close` ; création 1 aucun appel.
  Le crate : création 0 `open(url, 0, 16384)`, puis 28 `read_range` de 16384 octets (jusqu'à
  `458752`), trois en-têtes après chaque appel, `close` ; création 1 aucun appel. Le GeoTIFF
  déflaté est lu entier : voir « Ce qui reste ouvert ». Les valeurs renvoyées ne changent pas.
  **Défaut trouvé et corrigé : seconde création avec callbacks.** Sur ce fichier, avec callbacks,
  la seconde création échouait (`ProjError("illegal value for +grids")`, PROJ : non NULL). La
  taille du fichier était gardée par le `RemoteFile` de la première création, pas dans le cache de
  morceaux partagé : la seconde lisait le premier morceau et le prenait pour le fichier entier.
  Correction : `network_chunks::ChunkCache` garde la taille par URL (`sizes`), vidée avec les
  morceaux (`clearMemoryCache`). Test rouge, sortie avant correction :
  `creation 1: proj_create is not NULL: Some(ProjError("illegal value for +grids"))`, dans
  `tests/unit_network_core.rs::second_creation_of_a_multi_chunk_grid` (`data/tests/egm96_15.gtx`,
  259240 octets, 16 morceaux, sous `https://foo/`). Vert après correction. PROJ mesuré sur ce
  même fichier : une seule `open(url, 0, 16384)` pour les deux créations, aucun appel à la seconde.
  Écart fermé par le lot `shared-validators` : la référence `Last-Modified`/`ETag` et la taille
  sont par URL dans le cache partagé, plus par `RemoteFile`. Un fichier modifié entre deux créations
  du même builder est détecté au premier calcul qui lit une tuile non en cache, comme PROJ
  (`simul_file_change_between_two_creations`). Voir « Fichier changé pendant l'ouverture ».
  **Garde `networkAccessOK`.** Le TEST sort tôt quand le CDN ne répond pas ; le test du crate fait
  de même (`cdn_reachable`).
- **`pyproj_issue_1192` et `networkAccessOK`.** Le test retourne tôt quand la sonde
  (`tests/net_cdn`) ne répond pas, comme le TEST. Ce lot l'a exécuté avec le vrai CDN (sonde
  `206`), donc sans fichier de substitution. Les valeurs mesurées avec le CDN joignable sont celles
  décrites plus haut (10 opérations, 8 et 10 instanciables, id `pipeline`) : aucune n'a changé.
- **Id de `proj_pj_info`.** PROJ donne `pipeline` pour chaque opération instanciable, réseau on
  et off (mesuré). Le test asserte cet id pour chaque opération instanciable construite par
  `proj_from_operation`, sous les deux réglages, et une erreur pour les non instanciables. Le nom
  de l'opération (`proj_pj_info().description` en PROJ) est vérifié par la liste des noms.

- **Sonde `initial_check`.** Le TEST C++ n'a aucune assertion ; il pose `networkAccessOK` par un `GET`
  sur `cdn.proj.org` (`Range: 0-1`, code 206). Pas de test de la sonde elle-même : il n'y a pas de
  valeur PROJ à comparer. Sa requête est reprise par `tests/net_cdn/mod.rs` (`cdn_reachable`), qui garde
  `basic`, `download_whole_files` et `file_api`. Le défaut `https://cdn.proj.org` est confirmé contre
  `libproj` 9.9.0 (`proj_context_get_url_endpoint`). Détail : `docs/AUDIT-network.md`, ligne 1.

- **Téléchargement en une requête, sans `Range`.** PROJ vérifie un fichier par une requête
  d'un octet et le télécharge par `Range`, en morceaux de `PROJ_FULL_FILE_CHUNK_SIZE` (1 MiB par
  défaut). Le client HTTP du crate n'envoie pas de `Range` : la vérification et le téléchargement
  lisent le fichier entier en une requête, puis l'écrivent par morceaux de la même taille. Les
  rappels de progression sont ceux de PROJ (un après chaque morceau, sauf le premier) et
  l'enregistrement dans `cache.db` aussi. Ce qui change : le trafic (331904 octets au lieu d'un
  octet pour la vérification). Avec des callbacks réseau enregistrés, le téléchargement lit par
  morceaux comme PROJ (`RemoteFile::read_at`).
- **TTL.** `TtlSetting::Ignore` (`ignore_ttl_setting = TRUE`) n'envoie aucune requête : c'est le
  code de PROJ (`if (!ignore_ttl_setting)` dans `proj_is_download_needed`). Le commentaire de
  l'en-tête de PROJ dit le contraire ; le cas `download_whole_files` mesure le code.
- **Contexte.** Le crate n'a pas de contexte par défaut modifiable. Le cas C++ bascule le réseau
  du contexte par défaut et d'un second contexte ; ici chaque état est un `ProjBuilder`
  (`enable_network`), et un `Proj` garde le réseau du builder qui l'a créé. `grid_info` et
  `look_for_grid_info` prennent donc le réseau de leur builder.
- **Rappel de progression.** Le rappel reçoit le `&ProjBuilder` qui télécharge
  (`DownloadProgress = dyn FnMut(&ProjBuilder, f64) -> bool`). La comparaison
  `vectPct.back().first == ctx` devient `ptr::eq`, sur chaque rappel.
- **Erreurs.** Sans réseau, serveur muet, rappel qui arrête, ou fichier impossible à écrire :
  `ProjError::Network` ou `ProjError::Path`. PROJ renvoie `false` et journalise.
- **Enregistrement.** `lastModified` et `etag` absents sont stockés comme `''` ; PROJ les stocke
  comme `NULL`. La comparaison est la même, PROJ lisant `NULL` comme `""`.
- **Cache désactivé.** Comme PROJ, un téléchargement sans cache écrit le fichier puis échoue
  (`ProjError::Network`), car `DiskChunkCache::open` n'ouvre rien.
- **`lookForGridInfo` sans ligne.** La branche sans ligne ne consulte que le disque. PROJ ouvre
  aussi la grille à distance quand le réseau est actif (`open_resource_file`). Le cas n'en a pas
  besoin : après le téléchargement la grille est sur disque, et avant, le réseau est coupé.
- **Proxy HTTPS.** Ajouté : `https_proxy` puis `HTTPS_PROXY` (l'ordre de curl), `no_proxy` puis
  `NO_PROXY` par nom ou par domaine (pas de plage CIDR), un `CONNECT` par requête. Seules les
  requêtes `https` passent par le proxy. Un proxy avec identifiants est ignoré : connexion directe.
  C'est la condition pour que le CDN soit joignable dans ce bac à sable, où le DNS direct échoue.
- **Sonde.** `cdn_reachable` est la requête de `initial_check` (`Range: bytes=0-1`, `206`), faite
  par le test et non par le client du crate, pour qu'un téléchargement cassé ne passe pas pour un
  CDN injoignable. Le test retourne tôt si la sonde échoue, comme PROJ, et il l'écrit dans la
  sortie du test. Ce retour anticipé ne vérifie rien : c'est le seul cas où le test ne porte pas.

## networking.basic (porté)

Le TEST (`test/unit/test_network.cpp:121`) est porté en entier dans `tests/unit_network_basic.rs`. Table
ligne par ligne, valeurs et commandes : `docs/AUDIT-network.md`, section « networking.basic (porté) ». Choix
du port :

- **Environnement.** `putenv("PROJ_NETWORK=ON")` devient `std::env::set_var`, et `putenv("PROJ_NETWORK=")`
  devient `set_var("PROJ_NETWORK", "")`, qui est « absent » pour PROJ comme pour `nonempty_env`. Au départ
  la variable est aussi posée vide, pour que l'état « off » ne dépende pas de l'environnement de l'appelant.
  Le cas est le seul test de son binaire, donc aucune course sur l'environnement.
- **Contexte.** Chaque état est un `ProjBuilder`. `ProjBuilder::new` lit `PROJ_NETWORK` à sa création, comme
  `proj_context_create`. L'état 4 pose `enable_network(true)`, comme `proj_context_set_enable_network`.
  `proj_cleanup` est sans objet : le contexte par défaut interne (`default_context`, `OnceLock` de
  `src/network.rs`) ne sert pas aux créations du builder, qui passent par `with_context`, et les états ne
  partagent aucun cache mémoire entre builders.
- **Cache de grilles.** Coupé pour les états 1, 3 et 4 (`set_grid_cache_enabled(GridCacheMode::Disabled)`),
  comme `proj_grid_cache_set_enable(ctx, false)`. L'état 2 le garde à sa valeur par défaut, comme le TEST.
- **Répertoire utilisateur.** Un dossier temporaire du processus (`set_user_writable_directory`), supprimé à la
  fin. Rien n'est écrit dans le dépôt ni dans le répertoire de l'utilisateur.
- **Réseau du CDN.** Quand la sonde `cdn_reachable` échoue, les états qui exigent le CDN ne sont pas
  vérifiés, comme `if (networkAccessOK)` ; les états « off » le sont toujours. Quand la sonde répond, les
  quatre états et la valeur sont vérifiés contre `https://cdn.proj.org/fr_ign_ntf_r93.tif`.
- **Preuve de vérification.** Le test a passé dès le premier lancement, sans modification de `src/`. Pour
  prouver que l'assertion de valeur est active, la valeur attendue a été remplacée par `1.9992776858` : échec
  (`longitude 1.9992776847752205 against PROJ 1.9992776858`), puis rétablissement de la valeur de PROJ.

## networking.curl_* (porté contre le vrai CDN)

Les trois TEST (`test/unit/test_network.cpp:1109`, `1144`, `1180`) sont portés dans
`tests/unit_network_grids.rs`, avec les grilles réelles de `https://cdn.proj.org`. Ils ne
dépendent plus de fixtures : les trois grilles de substitution (`fr_ign_gr3df97a.tif`,
`us_nga_egm08_25.tif`, `us_noaa_vertcone.tif`, écrites par `tests/net_support/grids.rs`) ont été
retirées, leurs seuls appelants étant ces trois cas. Table ligne par ligne :
`docs/AUDIT-network.md`, lignes 9 à 11 et « Cas portés ». Choix du port :

- **Garde `networkAccessOK`.** Chaque cas sort tôt quand la sonde `cdn_reachable` échoue, comme
  `if (!networkAccessOK) return;`. Le cas ne sort pas en silence sans le dire : il l'écrit dans la
  sortie du test.
- **Contexte.** Un `ProjBuilder` par cas, cache de grilles coupé (`proj_grid_cache_set_enable(false)`),
  réseau activé (`proj_context_set_enable_network(true)`), répertoire utilisateur temporaire du
  processus (`set_user_writable_directory`), supprimé à la fin. Le cache étant coupé, rien n'y est écrit.
- **Assertions.** Les trois `EXPECT_NEAR` de chaque TEST sont repris avec leurs tolérances (1e-8 pour
  lat/lon, 1e-2 pour la hauteur), et `ASSERT_NE(P, nullptr)` devient le `expect` de la création.
  Les valeurs de libproj 9.9.0 sont ajoutées, mesurées avec un programme C contre `libproj` 9.9.0
  (réseau et TIFF, `%.17g`, `PROJ_DATA` de la référence), et vérifiées à 1e-9. Mesures :

  | cas | entrée | sortie (lat, lon, hauteur) |
  |---|---|---|
  | `curl_hgridshift` | `EPSG:4275` vers `EPSG:4171`, `(lon, lat) = (2, 49)` | `48.999932260028217`, `1.9992776848010316`, `0` |
  | `curl_vgridshift` | `EPSG:4979` vers `EPSG:4326+3855`, `(150, -30, 0)` | `-29.999999999999996`, `150`, `-31.889799118042117` |
  | `curl_vgridshift_vertcon` | `EPSG:4269+7968` vers `EPSG:4269+5703`, `(-80, 40, 0)` | `40`, `-80`, `-0.15375280380249004` |

  Les grilles utilisées sont celles du journal de `libproj` (`PROJ_DEBUG=3`) : `fr_ign_gr3df97a.tif`,
  `us_nga_egm08_25.tif`, `us_noaa_vertcone.tif`. Les commentaires C++ écrivent `egm08_25.tif` et
  `vertcone.tif` ; le CDN et PROJ demandent les noms préfixés `us_nga_` et `us_noaa_`.
- **Ce que le vrai fichier change par rapport aux fixtures.** Les fixtures donnaient des valeurs de
  référence qui ne sont pas celles du CDN : `14.6249995232` au lieu de `-31.889799118042117` pour
  `curl_vgridshift`, `-0.1295000091` au lieu de `-0.153752803802490` pour `curl_vgridshift_vertcon`,
  et pour `curl_hgridshift` un décalage de `48.9968893575` / `1.9999925459` au lieu de
  `48.9999322600` / `1.9992776848`. Les trois cas passent contre le vrai CDN dès la première exécution,
  sans changement de `src/` : aucun écart avec PROJ n'a été trouvé.
- **Preuve que l'assertion de valeur est active.** La hauteur attendue de `curl_vgridshift_vertcon`
  a été remplacée par `-0.1295000091` (la valeur de la fixture) : échec, `height -0.15375280380249004
  against PROJ`, dans `tests/unit_network_grids.rs:145`. La valeur a été rétablie avant les
  lancements suivants.
- **Non porté.** `proj_assign_context` sur la transformation, puis la destruction du contexte : le
  port n'a pas d'équivalent (`ProjBuilder` possède son réseau, voir « Contexte » plus haut).

## API de fichiers

La fiche de `file_api` : les callbacks de `PROJ_FILE_API` deviennent deux traits publics,
`FileApi` (`open`, `exists`, `mkdir`, `unlink`, `rename`) et `ApiFile` (`Read + Write + Seek`),
enregistrés par `ProjBuilder::set_file_api`, comme `set_network_callbacks`. Écarts :

- **`close` est le `Drop`.** Un `ApiFile` est fermé quand il est lâché (`close_cbk` de PROJ, appelé
  par le destructeur de `FileApiAdapter`). Le cas de test le mesure dans son `Drop`.
- **`user_data` est l'état du trait** (`Arc` partagé), pas un pointeur passé à chaque appel.
- **Pas de `tell`** : c'est `Seek::stream_position`. Pas de `version` ni de `PROJ_OPEN_ACCESS` à part :
  `FileAccess` (`ReadOnly`, `ReadUpdate`, `Create`).
- **`set_file_api` ne renvoie rien.** PROJ renvoie `0` si un des dix callbacks manque
  (`filemanager.cpp:1197`) ; le trait est complet à la compilation, donc `ASSERT_TRUE(proj_context_set_fileapi(...))`
  n'a pas d'équivalent.
- **Un `FileApi` passe par le contexte, comme les callbacks réseau.** `GridContext` le porte ; un `Proj`
  créé par le builder le garde, et ses grilles se lisent à travers lui (`with_context`).
- **Lecture d'une grille.** `read_file` fait `seek(0, END)`, `seek(0, SET)`, puis `read` jusqu'à la fin,
  comme `FileManager::open` pour `proj.ini` (`filemanager.cpp:2049-2055`). PROJ lit les TIFF par `seek`
  et `read` de libtiff (tuiles et IFD, `grids.cpp:969-995`). Le port lit le fichier entier avec le
  décodeur du crate : la séquence d'appels n'est pas celle de PROJ, seule la présence de `read` et de
  `seek` est vérifiée.
- **Ordre de l'exists de répertoire.** `create_directory_all` fait `exists` de chaque niveau depuis le
  bas, puis `mkdir` depuis le haut, comme `CreateDirectoryRecursively` (`filemanager.cpp:1155`).

Mesuré avec `libproj` 9.9.0 (sonde C hors dépôt, non versionnée : `probe.c`, `PROJ_FULL_FILE_CHUNK_SIZE=30000`,
répertoire utilisateur absent, TIFF et curl), la séquence du téléchargement est : `open` en lecture seule
(vérification du fichier local par `is_download_needed`, `networkfilemanager.cpp:2541`), `exists` (répertoire),
`exists` (parent), `mkdir`, deux autres `open` en lecture seule (non identifiés), `open` en création,
12 `write` (11 de 30000 octets, puis 1904), `close`, `unlink` de la cible, `rename` du temporaire, `exists`
(répertoire, `cache.db`). Aucun `read`. La lecture de la grille, réseau coupé : `open` en lecture seule,
`seek`, `tell`, `read`, `close`, et `z` = 36.590999603271491. Le port vérifie la présence du fichier local
par `exists`, sans `open` : il ne fait donc pas ces trois `open` en lecture seule, et le reste de la séquence
est le même.

**Opérations restées sur `std::fs`** (hors de l'API) : `proj.ini`, lu par `GridContext::load` avant que
l'application ait pu poser son `FileApi` (`network.rs`, `read_proj_ini`) ; le répertoire de la base de
cache de morceaux et son `clear` (`network_cache.rs`, `DiskChunkCache::open`, `clear`) ; `projsync`
(`src/projsync.rs`, téléchargement d'index) ; la recherche de fichiers de la CLI gie (`src/gie.rs`,
`is_file`). Les grilles, modèles, dictionnaires `+init=FILE:SECTION` et métadonnées de grille passent par
l'API (`gridpath::resolve`, `gridpath::read`, `network::read_file`).

## GeoTIFF compressé, mesures

Mesures avec `libproj` 9.9.0 (callbacks qui journalisent, `proj=vgridshift grids=<nom>
multiplier=1`, `proj_trans(PJ_FWD)`), sur les mêmes octets que les tests. Création de chaque grille :
`open(url, 0, 16384)`, trois en-têtes, `close`, comme le crate maintenant.

- Hors ligne, en bandes (`data/tests/planar_deflate_predictor3.tif`, 31705 octets, planaire,
  Deflate, prédicteur 3, une bande par morceau aux décalages 530, 12171 et 23018). `(20, 50)`,
  bande 2 (`vertical_offset`), morceau 1 : `open(url, 16384, 16384)`, trois en-têtes, z =
  0.8661957234144211. `(41, 37)`, même morceau : aucun appel, z = 0.6664930284023288.
  Test : `tests/unit_network_compressed_grid.rs::a_point_reads_the_strip_it_needs`.
- Hors ligne, en tuiles (`data/tests/planar_deflate_predictor3_tiled.tif`, 30094 octets, tuiles
  de 32 pixels sur 100 par 70, 36 tuiles, planaire, Deflate, prédicteur 3). `(20.125, 49.875)`, tuile
  intérieure 26409 à 27364 : `open(url, 16384, 16384)`, trois en-têtes, z = 0.859148681163788.
  `(34.625, 42.875)`, tuile de bord (4 par 6 pixels utiles, 29927 à 30094) : aucun appel, z =
  0.880941569805145. Test : `a_tiled_grid_reads_the_tile_of_each_point`.
- Le CDN (`nz_linz_nzgd2000-c120100904-grid01.tif`, 460261 octets, 4 images, planaires, Deflate,
  prédicteur 3, une bande par morceau, 55 par 52 pixels pour l'image de base). L'image choisie pour
  `(171.9, -43.55)` a sa bande 2 (`vertical_offset`) à 347367, 112894 octets : `read_range(344064,
  131072)`, les morceaux 21 à 28 (le bloc aligné, comme pour les autres lectures). `(169, -45)` :
  `open(url, 16384, 16384)`. Test : `tests/unit_network_compressed_cdn.rs`.

Non mesuré contre `libproj` : GeoTIFF déflaté chunky (`PlanarConfiguration` 1), prédicteur 2, ancien
Deflate (32946), tuiles de bord chunky. Ces cas passent par le même décodeur ; leurs valeurs et leurs
appels ne sont pas établis avec `libproj`.

LZW (`Compression` 5) et PackBits (`Compression` 32773) : décodés par `tiff_codec` (`lzw.rs`,
`packbits.rs`, prédicteurs dans `predictor.rs`), en fichier local comme sur le réseau. Le crate `tiff`
n'a pas son lecteur LZW dans ce build (feature `lzw` absente ; l'activer ajouterait `weezl`).

Mesures contre `libproj` 9.9.0 lié à libtiff (`proj-reference-net`, `cct -d 17`, `vgridshift`) :
- LZW prédicteur 3, little-endian : `data/tests/lzw_predictor3_planar.tif` (bandes) et
  `lzw_predictor3_planar_tiled.tif` (tuiles de bord) ont les échantillons de
  `planar_deflate_predictor3*.tif` à l'octet près ; mêmes valeurs que ce Deflate. Appels sur le
  réseau : modèle du Deflate (`open(url, 32768, 16384)` pour le morceau de la bande 2), non mesurés.
- LZW prédicteur 3, big-endian : `lzw_be_predictor3_planar.tif` et `..._tiled.tif`. libtiff donne les
  valeurs du little-endian. Les plans du prédicteur sont les mêmes octets dans les deux ordres ; le
  décodeur les range dans l'ordre du fichier (`undo_float`). Une première version, aux plans rangés
  à l'envers, donnait 1e35 : écart attrapé par la mesure.
- PackBits sans prédicteur : `packbits_planar_tiled.tif`, mêmes valeurs que le Deflate.
- PackBits avec prédicteur 3 : `packbits_predictor3_planar_tiled.tif`. libtiff ignore le prédicteur
  avec PackBits : une constante `0x3F` revient brute (`0.7470588`), là où LZW la traite. Mesure à
  (21.3, 45.7) : 4.7599535295655623e-35 avec PackBits, 0.5760 pour le même échantillon prédit en LZW.
  Cause probable : libtiff n'enregistre pas le prédicteur pour PackBits (non vérifié dans ses
  sources). Le crate suit libtiff.
- JPEG (`Compression` 7) et CCITT (2, 3, 4) : libtiff refuse le décodage (« Not a JPEG file »,
  « Bits/sample must be 1 »). PROJ ouvre la grille et ses valeurs sont `inf`, sans erreur. Le
  crate fait pareil (lot Q9, voir « GeoTIFF JPEG et CCITT »). Old JPEG (6) : PROJ refuse la
  création (« could not find required grid(s) »), le crate aussi, avec un autre message.
- Prédicteur 2 en big-endian (LZW) : non mesuré.

Fixtures : `tools/make-lzw-grid.py` (`--tiled`, `--big-endian`, `--codec packbits`,
`--predictor 1`). Test local : `tests/unit_grid_codecs.rs` ; réseau :
`tests/unit_network_compressed_grid.rs` (`a_big_endian_lzw_*`, `a_packbits_grid_*`).

## GeoTIFF JPEG et CCITT

Fixtures : `data/tests/planar_deflate_predictor3_jpeg.tif` (`Compression` 7) et
`..._ccitt.tif` (2). Ce sont les octets du Deflate planaire, avec le tag 259 changé
(`tools/patch-compression-tag.py`) : libtiff ne décode rien, PROJ ouvre la grille quand même.

Mesures contre `libproj` 9.9.0 lié à libtiff (`proj-reference-net`, `cct -d 17 +proj=vgridshift
+grids=<fichier> +multiplier=1`, entrée `lon lat 0`) :

| `Compression` | `(20, 50)` | `(41.5, 37.2)` | `(21.5, 45.5)` | `(0, 0)` hors grille | code de sortie |
|---|---|---|---|---|---|
| 7 (JPEG) | `inf` | `inf` | `inf` | `TRANSFORMATION ERROR` | 0 |
| 2 (CCITT RLE) | `inf` | `inf` | | `TRANSFORMATION ERROR` | 0 |
| 3 (CCITT T4) | `inf` | `inf` | | `TRANSFORMATION ERROR` | 0 |
| 4 (CCITT T6) | `inf` | `inf` | | `TRANSFORMATION ERROR` | 0 |
| 6 (old JPEG) | creation refusée : `could not find required grid(s)` | | | | 1 |

Sortie de libtiff (sur stderr) : « Not a JPEG file: starts with 0x78 0x9c » pour 7, « Bits/sample
must be 1 for Group 3/4 encoding/decoding » pour 2. La valeur `inf` est celle de `z` ; la
sortie ne porte aucune erreur de transformation.

Le port (même commande) : création réussie pour 2 et 7, `inf` aux points dans la grille, comme
PROJ. Hors grille, il imprime `*` et sort en 0 ; PROJ imprime `TRANSFORMATION ERROR` : écart de
présentation déjà présent sur les grilles Deflate, non traité ici. Avant le lot : création
refusée (`illegal value for +grids`, code 1) pour 2 et 7. Pour 6, le port refuse la création
comme PROJ, mais avec un autre message (`illegal value for +grids`, code 1).

Test : `tests/unit_grid_codecs.rs::a_jpeg_or_ccitt_grid_opens_and_its_values_are_infinite`
(JPEG et CCITT 2). Les codes 3 et 4 sont mesurés, sans fixture ni test. Le chemin réseau
(`Mode::Lazy`) n'appelle aucun morceau pour ces images, PROJ non mesuré sur ce point.

### Inventaire des GeoTIFF du CDN

Index `https://cdn.proj.org/files.geojson` : 393 fichiers `.tif` sur 32 pays (65 autres : README,
JSON, grilles non-TIFF). En-tête lu (`Range: bytes=0-65535`) pour 384 fichiers ; les 9 dont l'IFD
est au-delà (8 danois, 1 hongrois) lus entiers. Un seul LZW : `cz_cuzk_table_-y-x_3_v1710.tif`.

| Compression | Prédicteur | Planaire | Disposition | Échantillons | Grilles | Pays |
|---|---|---|---|---|---|---|
| Deflate | 3 | planaire | tuiles | float32 ×1 | 93 | 21 |
| Deflate | 3 | planaire | bandes | float32 ×2 | 91 | 9 |
| Deflate | 3 | planaire | bandes | float32 ×1 | 78 | 13 |
| Deflate | 3 | planaire | bandes | float32 ×3 | 28 | 4 |
| Deflate | 3 | planaire | bandes | float32 ×4 | 25 | 8 |
| Deflate | 3 | chunky | bandes | float32 ×2 | 11 | 1 |
| Deflate | 3 | planaire | tuiles | float32 ×2 | 10 | 5 |
| Deflate | 3 | planaire | tuiles | float32 ×4 | 10 | 6 |
| Deflate | 2 | planaire | tuiles | int32 ×1 | 10 | 1 |
| Deflate | 3 | chunky | tuiles | float32 ×1 | 8 | 2 |
| Deflate | 3 | chunky | tuiles | float32 ×2 | 8 | 1 |
| Deflate | 2 | chunky | bandes | float32 ×2 | 7 | 1 |
| Deflate | 3 | chunky | tuiles | float32 ×3 | 6 | 1 |
| Deflate | 3 | planaire | tuiles | float32 ×3 | 2 | 2 |
| Deflate | 3 | chunky | bandes | float32 ×3 | 2 | 1 |
| Deflate | 3 | planaire | bandes | float32 ×6 | 1 | 1 |
| Deflate | 3 | planaire | tuiles | float32 ×6 | 1 | 1 |
| Deflate | 1 | planaire | bandes | float32 ×3 | 1 | 1 |
| **LZW** | 3 | chunky | bandes | float32 ×2 | 1 | 1 |

Les sept grilles danoises Deflate, prédicteur 2, float32, chunky, en bandes
(`dk_sdfi_{s34j,gs,gsb,kk,os,s34s,s45b}_2022.tif`, horizontales, `hgridshift`) sont lues par le crate
`tiff`. Mesurées contre `libproj` 9.9.0 : écart nul à 12 décimales sur un à quatre points dans
l'emprise de chacune (`proj-reference-net`, `cct -d 12`).

## projsync

Source : `src/apps/projsync.cpp` (652 lignes), `test/cli/test_projsync.sh` (212 lignes). Port :
`src/bin/projsync.rs` (options, affichage, codes de sortie), `src/projsync.rs` (index JSON, sélection,
téléchargement avec enregistrement dans `cache.db`), `tests/unit_projsync.rs` (22 tests). Les tests
tournent sur un serveur local (`files.geojson` et une fixture de 17 octets `proj-rs test grid` à la place
de `fr_ign_ntf_r93.tif`), avec l'index de test `tests/projsync/files.geojson`. Les sorties attendues sont
celles du `projsync` 9.9.0 de référence sur le même index (`--local-geojson-file`), l'endpoint remplacé.

**Options.** Toutes les options de `projsync.cpp` sont lues : `--endpoint`, `--user-writable-directory`,
`--system-directory`, `--target-dir`, `--local-geojson-file`, `--list-files`, `--source-id`, `--area-of-use`,
`--file`, `--bbox`, `--spatial-test`, `--dry-run`, `--exclude-world-coverage`, `--all`,
`--no-version-filtering`, `-q`/`--quiet`, `--verbose`. Ajouts Rust : `--user-directory` (alias de
`--user-writable-directory`), `--help`, `--version`.

Écarts vérifiés dans le code :

- `--user-writable-directory` : sans effet, comme le C++. La cible par défaut est
  `ProjBuilder::user_writable_directory()`, qui lit `PROJ_USER_WRITABLE_DIRECTORY` (`src/network.rs`,
  `default_user_dir`).
- `--system-directory` : le C++ prend le répertoire relatif de la bibliothèque, puis `PROJ_DATA` compilé.
  Le Rust prend le premier chemin de `PROJ_DATA`, sinon de `PROJ_LIB` (`system_directory`,
  `src/bin/projsync.rs`), et sort en erreur si aucun n'est défini.
- Téléchargement non conditionnel : un fichier dont l'enregistrement a expiré est téléchargé de nouveau
  en entier, sans requête conditionnelle (en-tête de `src/projsync.rs`, `is_download_needed`).
- Index : `tests/projsync/files.geojson` (et la constante `DUMMY` de `tests/unit_projsync.rs`, miroir du
  `dummy.geojson` du script), pas le `files.geojson` du CDN.

**Options sans aucun test dans `tests/unit_projsync.rs`** : `--spatial-test` (`contains`, `intersects`),
`-q`/`--quiet`, `--no-version-filtering`, `--system-directory`, `--user-writable-directory`
(et `--user-directory`).

**Cas du script `test_projsync.sh`** :

| Lignes du script | Cas | Test Rust | Statut |
|---|---|---|---|
| 22-27 | saut si `cdn.proj.org` ne répond pas | sans objet (serveur local) | sans objet |
| 34-40 | `--list-files --target-dir` ; ligne `at_bev_README.txt,,at_bev` | `list_files_fetches_the_index_and_records_it` | couvert (liste entière comparée) |
| 42-51 | `PROJ_USER_WRITABLE_DIRECTORY` ; `cache.db` et `files.geojson` présents | le même test, avec `--target-dir` et les deux fichiers vérifiés | partiel : la variable d'environnement n'est pas testée |
| 53-59 | `--all --dry-run` | `all_dry_run_lists_every_file` | couvert |
| 62-68 | `--source-id fr_ign --dry-run` | `source_id_selects_by_part_of_the_id` | couvert |
| 71-78 | `--source-id non_existing` : code 1, avertissement, liste | `unknown_source_id_fails_and_lists_the_known_ones` | couvert |
| 81-87 | `--area-of-use France --dry-run` | `area_of_use_selects_by_part_of_the_name` | couvert |
| 90-97 | `--area-of-use non_existing` : code 1, avertissement, liste | `unknown_area_of_use_fails_and_lists_the_known_ones` | couvert |
| 100-111 | `--file ntf_r93` : téléchargement, fichier présent | `file_downloads_once_and_then_says_it_is_already_there` | couvert (fichier de 17 octets, pas le TIF du CDN) |
| 113-119 | second `--file ntf_r93` : « already downloaded » | le même test | couvert |
| 124-131 | `--file non_existing` : code 1, avertissement, liste | `unknown_file_fails_and_lists_the_known_ones` | couvert |
| 134-141 | `--bbox 2,49,3,50 --dry-run` | `bbox_selects_the_grids_that_overlap_it` | couvert |
| 144-154 | `--exclude-world-coverage` | `exclude_world_coverage_drops_the_world_grids` | couvert |
| 157-184 | trois fenêtres de l'antiméridien | `bbox_across_the_antimeridian_finds_the_pacific_grids` | couvert |
| 186-206 | `--source-id au_ga --verbose --dry-run --local-geojson-file dummy.geojson` (versions retirée et ajoutée, sans version) | `verbose_reports_the_files_left_out_by_version` | couvert |
| 208-212 | nettoyage, code 0 | sans objet | sans objet |

Le test `file_downloads_once_and_then_says_it_is_already_there` vérifie la sortie et le fichier téléchargé.
Le contenu du fichier n'est pas comparé au binaire de référence : c'est la fixture de 17 octets, pas le TIF du CDN.

Commandes : `cargo test --test unit_projsync`.

## Rechargement par étape (lot Q8)

Mesures : sonde C hors dépôt, contre `libproj` 9.9.0 de `target/proj-reference-net` (TIFF et curl),
callbacks qui journalisent, la version B servie à partir de la deuxième ouverture (`Last-Modified`
`some_date CHANGED!!!!`). Fichiers de 64 × 64 pixels de 0,5° : `hgridshift` (2 échantillons, latitude
puis longitude, en secondes d'arc), `gridshift` (2 échantillons, `TYPE=HORIZONTAL_OFFSET`),
`xyzgridshift` (3 échantillons, en mètres, en pipeline géocentrique). Source : `src/grids.cpp`,
`src/transformations/{gridshift,xyzgridshift,deformation,tinshift}.cpp`.

| Étape | PROJ 9.9.0 | Port |
|---|---|---|
| `vgridshift` | `read_vgrid_value` : `hasChanged` après les lectures, `VerticalShiftGridSet::reopen`, point refait (`grids.cpp:3684`) | `Vertical::transform` → `with_reloads` |
| `hgridshift` | `pj_hgrid_apply_internal` (`grids.cpp:3429`, `3447`) et `pj_hgrid_value` (`3576`) : `reopen`, point refait (`shouldRetry`) | `Horizontal::forward`, `inverse` → `with_reloads` |
| `gridshift` | `grid_apply_internal` (`transformations/gridshift.cpp:622`, `667`) : `GenericShiftGridSet::reopen`, `shouldRetry` | `Generic::forward`, `inverse` → `with_reloads` |
| `xyzgridshift` | aucun `reopen` : `get_grid_values` se rappelle sans fin (`xyzgridshift.cpp:102-106`), SIGSEGV, code de sortie 139 (mesuré, pile `get_grid_values` répétée sous `gdb`) | `XyzShift::transform` → `with_reloads` : écart, voir plus bas |
| `deformation`, chemin à trois échantillons | même code que `xyzgridshift` (`deformation.cpp:121-125`), non mesuré | non porté : les grilles de `deformation` sont lues entières |
| `tinshift` (JSON) | le fichier est lu entier à la création (`tinshift.cpp:140`) : rien n'est ouvert ensuite | sans objet |

Séquence mesurée, `hgridshift` et `gridshift` (identique à l'inverse) : création `open(url, 0, 16384)`,
trois en-têtes, `read_range(32768, 16384)` (l'IFD), trois en-têtes, `close`. Transformation :
`open(url, 16384, 16384)`, en-têtes `CHANGED`, `read_range(32768, 32768)` (la fin de la bande, sans
en-tête), `close`, puis `open(url, 0, 16384)`, trois en-têtes `CHANGED`, et le point est refait sur la
version B. L'opération créée avant le changement, dans le même builder, ne fait aucun appel : elle lit
le morceau rechargé dans le cache partagé, et donne la valeur de la version B.

Écarts restants :
- Tailles de lecture : PROJ lit le reste de la bande dans la même passe (`read_range(32768, 32768)`) ;
  le port ne lit que les morceaux que l'interpolation demande (lectures par morceaux de `geotiff.rs`).
  Après le rechargement, le port relit l'IFD (`read_range(32768, 16384)`) que PROJ a déjà en cache.
  Les tests vérifient l'ordre des appels de PROJ (ouverture du changement, fermeture, rechargement),
  les valeurs, et l'absence d'appel de l'opération restée ancienne. Le journal exact n'est pas repris.
- `xyzgridshift` : PROJ ne donne aucune valeur sur un fichier changé (SIGSEGV). Le port recharge comme
  les autres étapes ; la valeur attendue est celle de PROJ pour la version B lue sans changement.

Tests : `tests/unit_network_grid_change.rs` (huit cas : aller, inverse, deux opérations, sans changement,
pour `hgridshift` et `gridshift` ; `xyzgridshift`).

## Ce qui reste ouvert

- GeoTIFF compressés en prédicteurs hors 1 à 3 : lus entiers (`read_whole`), la séquence de PROJ
  n'est pas reproduite. LZW et PackBits sont lus par morceaux sur le réseau et entiers en local ;
  voir « GeoTIFF compressé, mesures ». JPEG et CCITT ne sont pas lus (lot Q9, « GeoTIFF JPEG et
  CCITT ») : le crate ne fait aucun appel réseau pour eux, PROJ non mesuré sur ce point.
- GeoTIFF JPEG ou CCITT de 8 bits : refusé à la création par le crate (`supported_sample` : entiers
  de 16 ou 32 bits, flottants de 32 ou 64). PROJ non mesuré sur un tel fichier.

- Lecture par morceaux pour les GeoTIFF compressés (deflate), à prédicteur ou planaires : ils
  sont lus entiers (`read_whole`), la séquence de PROJ n'est pas reproduite. Mesuré sur le vrai
  GeoTIFF déflaté du CDN (`nz_linz_nzgd2000-c120100904-grid01.tif`, 460261 octets) : à la création,
  le crate fait 28 `read_range` que PROJ ne fait pas (PROJ : `open` et en-têtes seulement). Les
  valeurs renvoyées sont les mêmes. Non mesuré : un GeoTIFF en bandes compressé ou planaire, lu par
  morceaux avec la séquence de PROJ.
- Lecture paresseuse hors `gridshift` : `read_resource` (modèles `deformation`, `tinshift`,
  `defmodel`, `nkg`) lit encore les grilles entières (`fetch_missing`, `fetch_with_callbacks`).
- Rechargement : `deformation` (chemin à trois échantillons), les lectures entières (`read_whole`) et
  `read_resource` (modèles `deformation`, `tinshift`, `defmodel`, `nkg`) ne rechargent pas. Voir
  « Rechargement par étape ».
- Validation par `ETag` et `Last-Modified` du cache disque : sans callbacks, faite à l'expiration par la
  requête du premier morceau (fiche `PORTING-network-cache.md`, point 2). Avec les callbacks, le crate
  n'utilise pas `cache.db` : PROJ le fait quand le cache est activé (mesuré : deux processus, TTL valide,
  détection au calcul ; TTL dépassée, pas de détection). Le cache en mémoire, par URL, est fait (lot
  `shared-validators`).
- Propriétés par builder, pas par processus. PROJ garde les propriétés et les morceaux dans des
  statiques du processus : une création sur un autre contexte du même processus voit le changement
  (mesuré : détection au calcul). Le crate les garde par `ProjBuilder` : un second builder ne voit
  pas le fichier du premier (`simul_file_change_with_a_new_builder`, le cas deux processus où PROJ
  ne détecte rien).
- Suppression par URL. PROJ vide tous les morceaux en mémoire au changement (`clearMemoryCache`) ;
  le crate ne vide que ceux de l'URL. Non mesuré avec deux URL dans un builder.
- Création sans comparaison. Le `NetworkFile::open` de PROJ, quand le morceau 0 n'est pas en
  cache, remplace les propriétés sans les comparer ; le crate les compare. Il signale donc un
  changement là où PROJ n'en signale pas, et seulement si le morceau 0 manque alors que les
  propriétés sont là (éviction du cache, cas non atteint par les tests).
- Taille sans `Content-Range` : la taille déduite d'une réponse courte n'est plus partagée entre
  deux `RemoteFile` de la même URL (elle l'était avant ce lot). PROJ n'a pas ce cas : sans
  `Content-Range`, la création échoue.
- Cache SQLite : TTL, taille max, verrou, `proj_grid_cache_clear` ont des tests (`tests/unit_network_cache.rs`,
  cinq cas sur serveur local, voir l'audit) ; les écarts de la couche cache sont dans `docs/PORTING-network-cache.md`.
- API de fichiers : routée pour la recherche, les lectures de grilles, modèles et dictionnaires, le
  répertoire utilisateur et le téléchargement (voir « API de fichiers »). Restent sur `std::fs` :
  `proj.ini`, le répertoire et l'effacement du cache de morceaux, `projsync`, la CLI gie. Ni façade C,
  ni vérification par `Range`.
- Façade C des opérations (`proj_create_operations`, contexte de fabrique C) : absente. Le cas
  de `do_not_attempt_network_access_known_available_network_on` est couvert par
  `ProjBuilder::create_operations`, sans façade C.
- Création paresseuse d'une opération seule (réseau activé) : portée par
  `ProjBuilder::proj_from_operation` (ce lot). Les opérations 00 et 02 de `pyproj_issue_1192` sont
  construites et asserties (id `pipeline`), pas seulement par leur chaîne PROJ.
- Lecture de la grille au premier point d'une opération construite par `proj_from_operation` :
  le chemin de transformation différé existe (`Deferred::resolve`), mais ce lot ne l'asserte pas.
- Grille sans ligne, réseau actif, sans callbacks : la sonde télécharge le fichier entier (pas
  de `Range` dans le client HTTP du crate), PROJ n'en lit que le premier morceau. Les valeurs
  sont testées contre le vrai CDN (`tests/unit_network_grid_used.rs`), la requête ne l'est pas.
- `proj_coordoperation_is_instantiable` avec réseau actif : PROJ passe par `gridsNeeded`
  (`singleoperation.cpp:339-356`), donc interroge le réseau pour une grille sans ligne ; le port
  (`is_proj_instantiable`, `RemoteGrids::Skip`) ne le fait pas. Non mesuré pour une grille sans
  ligne. Ce lot ne modifie pas `is_proj_instantiable` (voir « Hypothèse » plus haut).
- `proj_coordoperation_get_grid_used` ne porte pas de drapeau `gridopen` : l'API Rust donne
  `available` seulement.
- Grille `http(s)://` sans ligne : son URL est prise telle quelle, sans sonde (comportement
  antérieur, `unrecorded_grid`) ; PROJ n'écrit l'URL qu'après une ouverture réussie.
- `docs/UNPORTED-UNIT.md` liste encore `networking.pyproj_issue_1192` et
  `networking.do_not_attempt_network_access_known_available_network_on` : ce lot ne les a pas retirés
  (consigne : ne pas éditer le ledger). Le retrait suit avec `UPDATE_DOCS=1 cargo test --test
  docs_in_sync`.
- `projsync` : porté (lot cache). Fiche : `docs/PORTING-network-cache.md` ; couverture des 15 appels du
  script : `docs/AUDIT-network.md`, section projsync.
- `basic` : porté en entier (voir « networking.basic (porté) »). Rien ne reste ouvert pour ce cas.
- `pj_find_file` avec `disable_network = false` : non porté. `find_local_file` ne regarde que le
  disque.

## Lot Q5 : GeoTIFF tuilés et volume

- Tuilés dans le lecteur des modèles (`+proj=deformation`) : corrigé, `src/projections/models/grid_tiff.rs`
  (blocs = bandes ou tuiles). Le CDN `ca_nrc_NAD83v70VG.tif` est tuilé, planar, Deflate, prédicteur 3.
  Tests : `grid_tiff_tests.rs`. Fiche : `docs/PORTING-cs2cs-network-only-best.md`, « Lot Q5 ».
- Volume mesuré sur le corpus réseau : le port prend le fichier entier à chaque grille (416 requêtes
  sans `Range`, 1,50 Go), PROJ lit des plages (1155 requêtes, 86 Mo). Non corrigé, lot de lecture paresseuse.

## Commandes

```
cargo test --test unit_network_core --test unit_network_endpoint_env --test unit_network_local --test network_grid
cargo test --test unit_network_download
cargo test --test unit_network_file_api
cargo test --test unit_network_grid_used
cargo test --test unit_network_core second_creation_of_a_multi_chunk_grid
cargo test --test unit_network_basic
cargo test --lib callback_tests
cargo test --test unit_network_cache --test unit_network_grids
cargo test --test unit_projsync
cargo +1.100.0 clippy --all-targets --all-features -- -D warnings
```
