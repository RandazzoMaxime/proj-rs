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
| `DatabaseContext::lookForGridInfo` (`factory.cpp:3400`) | `src/iso19111/factory.rs` (`look_for_grid_info`), `src/iso19111/select.rs` (`unrecorded_grid`) |
| `src/apps/gie.cpp` (`network_error`) | `src/gie.rs` (`errno_name`) |
| `test/unit/test_network.cpp` (26 TEST) | `tests/unit_network_core.rs` (8 cas, dont `custom`, `getfilesize` et `simul_read_range_error`), `tests/unit_network_endpoint_env.rs` (1 cas), `tests/unit_network_local.rs` (2 cas), `src/network.rs` `callback_tests` (2 tests hors liste) |
| serveur de test local (remplace `cdn.proj.org`) | `tests/net_support/mod.rs` |
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
- **En-têtes demandés, non comparés.** `Last-Modified` et `ETag` sont demandés après chaque
  appel pour garder la séquence de PROJ, mais leurs valeurs ne sont pas comparées : la
  détection d'un fichier changé pendant l'ouverture (`simul_file_change_while_opened`) reste
  un écart.
- **Forme du trait.** `open` renvoie les premiers octets avec un handle (`OpenReply`), au lieu
  d'un tampon et d'une chaîne d'erreur passés en paramètres. Les erreurs sont typées
  (`NetworkError::Open`, `ReadRange`). Les en-têtes passent par `header`, avec une valeur
  par défaut `None`.
- **Dernier candidat.** `last_used_operation` porte le nom du candidat choisi au dernier
  point, qui est la description de `proj_pj_info` en PROJ.

## Ce qui reste ouvert

- Lecture par morceaux pour les GeoTIFF compressés (deflate), à prédicteur ou planaires : ils
  sont lus entiers (`read_whole`), la séquence de PROJ n'est pas reproduite. Non mesuré ici :
  un GeoTIFF en bandes compressé ou planaire, lu par morceaux avec la séquence de PROJ.
- Lecture paresseuse hors `gridshift` : `read_resource` (modèles `deformation`, `tinshift`,
  `defmodel`, `nkg`) lit encore les grilles entières (`fetch_missing`, `fetch_with_callbacks`).
- Validation de la fraîcheur d'un fichier ouvert (`ETag`, `Last-Modified`) : bloque
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
