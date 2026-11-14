# Fiche de portage : réseau (test_network.cpp, projsync)

Audit détaillé : `docs/AUDIT-network.md`. Fiche du lot gie et réseau antérieur :
`docs/PORTING-gie-network.md`.

## Carte

| Source PROJ 9.9.0 | Rust |
|---|---|
| `src/filemanager.cpp` (téléchargement, cache), `src/networkfileapi` | `src/network.rs` (interne, `pub(crate)`) |
| `proj_context_set_network_callbacks` | `NetworkCallbacks` (trait public), `ProjBuilder::set_network_callbacks`, `src/network.rs` |
| `NetworkFile`, `DOWNLOAD_CHUNK_SIZE`, cache de morceaux (`filemanager.cpp`) | `src/network_chunks.rs` (`RemoteFile`, `ChunkCache`, `RemoteReader`) |
| `GTiffGrid`, lecture des tuiles au calcul (`grids.cpp`, libtiff) | `src/projections/gridshift/geotiff.rs` (`load_remote`, `raw_samples`), `src/projections/gridshift/tiles.rs` (`LazyIfd`, `with_tiles`) |
| `proj_context_set_url_endpoint`, `PROJ_NETWORK_ENDPOINT` | `ProjBuilder::set_url_endpoint`, `GridContext::load` |
| `grids.cpp:3222` (`pj_generic_grid_init`), `grids.cpp:3234` | `src/projections/gridshift/mod.rs` (`deferred`, `Deferred`) |
| `trans.cpp:284` (erreur réseau d'un candidat : arrêt du point) | `src/iso19111/alternatives.rs` (`AlternativeError::Network`) |
| `proj_trans_get_last_used_operation`, `proj_pj_info` | `Proj::last_used_operation`, `Proj::proj_info` (`candidate_name`) |
| `DatabaseContext::lookForGridInfo` (`factory.cpp:3400`) | `src/iso19111/factory.rs` (`look_for_grid_info`), `src/iso19111/select.rs` (`unrecorded_grid`, `RemoteGrids`), `src/network.rs` (`open_remote_grid`, la branche réseau de `open_resource_file`, `filemanager.cpp:1859-1915`) |
| `proj_coordoperation_get_grid_used_count`, `proj_coordoperation_get_grid_used` (`c_api.cpp:7905-7990`) | `Proj::grids_needed` (`src/proj.rs`), sous `with_context` ; `RemoteGrids::Open` |
| `proj_create_operations` avec le contexte de fabrique (callbacks du contexte) | `ProjBuilder::create_operations` (`src/proj.rs`), enveloppe `with_context` comme les autres créations du builder |
| `src/apps/gie.cpp` (`network_error`) | `src/gie.rs` (`errno_name`) |
| `test/unit/test_network.cpp` (26 TEST) | `tests/unit_network_core.rs` (12 cas, dont `custom`, `getfilesize`, `simul_read_range_error`, `simul_file_change_while_opened`, `proj_coordoperation_get_grid_used`, `pyproj_issue_1192` et `do_not_attempt_network_access_known_available_network_on`), `tests/unit_network_endpoint_env.rs` (1 cas), `tests/unit_network_local.rs` (2 cas), `src/network.rs` `callback_tests` (2 tests hors liste) |
| serveur de test local (remplace `cdn.proj.org`) | `tests/net_support/mod.rs` |
| `proj_is_download_needed`, `proj_download_file` (`networkfilemanager.cpp:2523-2880`) | `src/network_download.rs` (`is_download_needed`, `download`) ; `ProjBuilder::is_download_needed`, `ProjBuilder::download_file`, `TtlSetting`, `DownloadProgress` (`src/proj.rs`) |
| `pj_find_file(…, disable_network)` (`filemanager.cpp:1942`), `proj_grid_info` avec le réseau du contexte (`grids.cpp:3928`), `lookForGridInfo` et ses deux résultats (`factory.cpp:3398`) | `ProjBuilder::find_local_file`, `ProjBuilder::grid_info`, `ProjBuilder::look_for_grid_info` (`GridLookup`, `src/iso19111/factory.rs`, `grid_lookup`) |
| `HTTPS_PROXY` / `https_proxy` (curl, tunnel `CONNECT`) | `src/network.rs` (`connect`, `https_proxy`, `open_tunnel`, `bypasses_proxy`) |
| `networking.download_whole_files` (`test_network.cpp:1859`, 37 assertions) | `tests/unit_network_download.rs` (1 cas, contre le vrai CDN) |
| `src/apps/projsync.cpp`, `test/cli/test_projsync.sh` | non porté (voir l'audit) |

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
- **Restriction de la lecture par morceaux.** Seuls les GeoTIFF à tuiles ou à bandes, non
  compressés, sans prédicteur, à échantillons entrelacés, sont lus par morceaux. Les autres
  (deflate, prédicteur, bandes planaires, non-TIFF) sont lus entiers par `read_whole` : leur
  séquence d'appels n'est pas celle de PROJ (il y a les `read_range` du reste du fichier). Le décodage des tuiles est
  ici (`raw_samples` : u16, i16, u32, i32, f32, f64, ordre d'octets du fichier) ; le décodeur
  `tiff` ne sert qu'à lire les IFD.
- **Nouvelle passe par point (`with_tiles`).** Un échantillon d'une tuile non lue donne `None`
  et la tuile est notée. Le point est recalculé une fois la tuile lue, si bien que
  l'interpolation ne voit jamais un échantillon absent. Une lecture en échec termine le point
  par `NetworkUnavailable`, comme une grille absente.
- **Fichier changé pendant l'ouverture (`simul_file_change_while_opened`).** Après chaque
  appel, les trois en-têtes sont demandés dans l'ordre de PROJ. La première réponse qui donne
  une taille (`Content-Range`) fixe la référence (taille, `Last-Modified`, `ETag`). Une réponse
  suivante différente met le fichier en `changed` et vide le cache de morceaux du contexte
  (`clearMemoryCache` de PROJ, puis `m_hasChanged`) : `network_chunks::RemoteFile::observe`.
  Le point en cours est arrêté à la lecture de tuile qui le constate (`tiles::with_tiles_watch`),
  la grille en vigueur est lâchée (son fichier se ferme), puis rechargée depuis
  `(url, 0, 16384)`, et le point est refait (`Vertical::transform`, `VerticalGrids::reload`).
  Le cache de morceaux garde la tuile lue après le changement, donc le second passage ne
  fait aucun appel. Mesuré avec `libproj` 9.9.0 (sonde C, `probe.c` non versionnée) : la
  séquence est celle du test, `close` du fichier 2 avant `open` du fichier 3, et `close` du
  fichier 3 à `proj_destroy`. Une création ferme son fichier après lecture (`load_all`), comme
  PROJ (mesuré : `close` avant la transformation), mais un rechargement le garde ouvert.
  Écarts : seule `vgridshift` recharge ; `hgridshift`, `gridshift` et `xyzgridshift` gardent
  leurs grilles figées (même mécanisme, pas de cas de la liste qui le demande). `reload`
  recharge toutes les grilles de l'étape, PROJ seulement celle du changement (un seul fichier
  dans le test). Une lecture entière (`read_whole`) ne déclenche pas de rechargement. Le
  nombre de rechargements par point est borné à `MAX_RELOADS` (8), PROJ boucle sans borne.
- **Forme du trait.** `open` renvoie les premiers octets avec un handle (`OpenReply`), au lieu
  d'un tampon et d'une chaîne d'erreur passés en paramètres. Les erreurs sont typées
  (`NetworkError::Open`, `ReadRange`). Les en-têtes passent par `header`, avec une valeur
  par défaut `None`.
- **Dernier candidat.** `last_used_operation` porte le nom du candidat choisi au dernier
  point, qui est la description de `proj_pj_info` en PROJ.
- **Fabrique d'opérations (`pyproj_issue_1192`).** Pas de façade C : le contexte de fabrique
  est `CoordinateOperationContext`, avec `GridAvailabilityUse::IgnoreGridAvailability` et
  `SpatialCriterion::PartialIntersection`, et la liste vient de
  `CoordinateOperationFactory::create_operations`. Aucune API ajoutée. Le drapeau réseau de
  `proj_coordoperation_is_instantiable(ctx, P)` est traduit par
  `CoordinateOperation::is_proj_instantiable` : réseau activé donne `KnownAvailable`
  (`considerKnownGridsAsAvailable` : une grille que la base connaît compte comme disponible),
  réseau désactivé donne `UseForSorting` (grilles sur disque seulement). Mesuré (sonde C
  contre `libproj` 9.9.0, non versionnée) : 10 opérations dans les deux cas, mêmes noms et même
  ordre ; instanciables 8 réseau off, 10 réseau on.
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
  **Octets de substitution.** Le fichier CDN `nz_linz_nzgd2000-c120100904-grid01.tif` n'est pas
  dans le dépôt et le proxy refuse `cdn.proj.org`. Le test sert les 956 octets de
  `data/tests/egm96_15_uncompressed_truncated.tif` sous cette URL (`ServeOneFile`) et refuse les
  autres URL. Les trois assertions du TEST portent sur le nom et l'URL, pas sur le contenu : elles
  sont les mêmes avec les octets de la référence, qui ne sont pas disponibles. Mesuré avec les
  mêmes octets (sonde C `grid_used_probe.c`, non versionnée) : `short` le nom, `full` vide,
  `url` le CDN, `direct` 0, `open` 0, `available` 1 ; une seule `open(url, 0, 16384)` pour les
  deux créations, et aucun appel pendant les deux `proj_coordoperation_get_grid_used`. Le test
  asserte ces drapeaux et ce compte, mesurés avec ces octets. Ce qui dépend du vrai fichier n'est
  pas porté : la séquence d'appels de callbacks sur le vrai GeoTIFF, et `available` d'un vrai
  fichier (ouvert ou non, la valeur de l'URL ne change pas).
  **Garde `networkAccessOK`.** Le TEST sort tôt quand le CDN ne répond pas. Le port ne sort pas :
  les callbacks répondent sans I/O, donc le test tourne partout.
  **Sans callbacks.** Non testé : le chemin HTTP télécharge le fichier entier. Il n'est atteint
  que si le réseau est actif sans callbacks.
- **Réseau absent ici (`networkAccessOK`).** `initial_check` est faux : `cdn.proj.org` est refusé
  (403 du proxy), donc le test C++ sort tôt. Le port ne dépend pas du CDN et n'est pas conditionné
  par `networkAccessOK`. La partie réseau activé est mesurée avec le CDN refusé ; la réponse de
  PROJ est la même (10 instanciables) : l'instanciabilité ne dépend que du drapeau, pas du
  téléchargement. Un CDN qui répond n'a pas été mesuré.
- **Id de `proj_pj_info`.** PROJ donne `pipeline` pour chaque opération instanciable (mesuré).
  Le test vérifie `proj_info().id == "pipeline"` sur les opérations construites hors ligne (8),
  et la chaîne `+proj=pipeline` pour les deux autres (grilles absentes). La description de PROJ
  est le nom de l'opération, vérifié par la liste des noms.

- **Sonde `initial_check` non portée.** Le TEST C++ n'a aucune assertion. Il pose
  `networkAccessOK` par un `GET` sur `cdn.proj.org` (`Range: 0-1`, code 206), et seul
  `basic` le lit. Pas de test Rust : il n'y a pas de valeur PROJ à comparer, et une sonde
  CDN ne peut pas tourner dans le bac à sable (proxy en 403). Le défaut
  `https://cdn.proj.org` est confirmé contre `libproj` 9.9.0 (`proj_context_get_url_endpoint`).
  Détail : `docs/AUDIT-network.md`, ligne 1.

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

## Ce qui reste ouvert

- Lecture par morceaux pour les GeoTIFF compressés (deflate), à prédicteur ou planaires : ils
  sont lus entiers (`read_whole`), la séquence de PROJ n'est pas reproduite. Non mesuré ici :
  un GeoTIFF en bandes compressé ou planaire, lu par morceaux avec la séquence de PROJ.
- Lecture paresseuse hors `gridshift` : `read_resource` (modèles `deformation`, `tinshift`,
  `defmodel`, `nkg`) lit encore les grilles entières (`fetch_missing`, `fetch_with_callbacks`).
- Rechargement au changement pour `hgridshift`, `gridshift` et `xyzgridshift` (seule
  `vgridshift` recharge), et pour les lectures entières (`read_whole`).
- Validation de cache par `ETag` et `Last-Modified` : cache disque (lot cache).
- Cache SQLite : TTL, taille max, verrou, `proj_grid_cache_clear` (lot cache, autre agent).
- API de fichiers (`file_api`). L'API de téléchargement est sur `ProjBuilder` (voir Décisions) ; ni
  façade C, ni vérification par `Range`.
- Façade C des opérations (`proj_create_operations`, contexte de fabrique C) : absente. Le cas
  de `do_not_attempt_network_access_known_available_network_on` est couvert par
  `ProjBuilder::create_operations`, sans façade C.
- Création paresseuse d'une opération seule dont la grille est absente (réseau activé) : PROJ
  construit `P` sans la grille (`proj_list_get`), la crate ne le fait que pour les candidats d'une
  paire (`grids_may_be_deferred`), et aucune API publique ne construit une opération seule. Les
  deux opérations 00 et 02 de `pyproj_issue_1192` ne sont donc vérifiées que par leur chaîne PROJ.
- Grille sans ligne, réseau actif, sans callbacks : la sonde télécharge le fichier entier (pas
  de `Range` dans le client HTTP du crate), PROJ n'en lit que le premier morceau. Sans test.
- `proj_coordoperation_is_instantiable` avec réseau actif : PROJ passe par `gridsNeeded`
  (`singleoperation.cpp:339-356`), donc interroge le réseau pour une grille sans ligne ; le port
  (`is_proj_instantiable`, `RemoteGrids::Skip`) ne le fait pas. Non mesuré pour une grille sans
  ligne, et ce lot ne modifie pas `pyproj_issue_1192`.
- `proj_coordoperation_get_grid_used` ne porte pas de drapeau `gridopen` : l'API Rust donne
  `available` seulement.
- Grille `http(s)://` sans ligne : son URL est prise telle quelle, sans sonde (comportement
  antérieur, `unrecorded_grid`) ; PROJ n'écrit l'URL qu'après une ouverture réussie.
- `docs/UNPORTED-UNIT.md` liste encore `networking.pyproj_issue_1192` et
  `networking.do_not_attempt_network_access_known_available_network_on` : ce lot ne les a pas retirés
  (consigne : ne pas éditer le ledger). Le retrait suit avec `UPDATE_DOCS=1 cargo test --test
  docs_in_sync`.
- `projsync` : non commencé.
- `basic` (partie « on ») : `fr_ign_ntf_r93.tif` absent du dépôt et de la référence. Sa
  condition `networkAccessOK` vient de `initial_check`, non porté (voir ci-dessus).
- `pj_find_file` avec `disable_network = false` : non porté. `find_local_file` ne regarde que le
  disque.
- `docs/UNPORTED-UNIT.md` liste encore `networking.download_whole_files`. Ce lot ne l'a pas retiré
  (consigne : ne pas éditer le ledger) : le retrait suit avec `UPDATE_DOCS=1 cargo test --test
  docs_in_sync`, par le mainteneur.

## Commandes

```
cargo test --test unit_network_core --test unit_network_endpoint_env --test unit_network_local --test network_grid
cargo test --test unit_network_download
cargo test --test unit_network_core proj_coordoperation_get_grid_used
cargo test --lib callback_tests
cargo +1.100.0 clippy --all-targets --all-features -- -D warnings
```
