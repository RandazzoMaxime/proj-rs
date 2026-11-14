# Fiche de portage : cache de grilles et projsync

Audit : `docs/AUDIT-network.md`, section « Cache et projsync ». Fiche de la couche
réseau : `docs/PORTING-network.md`.

## Carte

| Source PROJ 9.9.0 | Rust |
|---|---|
| `src/networkfilemanager.cpp` : `DiskChunkCache`, `NetworkChunkCache`, `NetworkFilePropertiesCache`, `proj_grid_cache_clear`, `pj_context_get_grid_cache_filename`, `proj_is_download_needed`, `proj_download_file` (partie fichier entier) | `src/network_cache.rs` (interne) et `src/projsync.rs` (`is_download_needed`, `download`) |
| `ctx->gridChunkCache` et `proj_grid_cache_set_*` | `src/proj.rs` : `ProjBuilder::set_grid_cache_enabled`, `set_grid_cache_path`, `grid_cache_path`, `set_grid_cache_max_size_mb`, `set_grid_cache_ttl`, `clear_grid_cache` ; enums `GridCacheMode`, `GridCacheTtl` |
| `src/apps/projsync.cpp` | `src/bin/projsync.rs` (options, affichage, codes de sortie), `src/projsync.rs` (index, sélection, téléchargement) |
| `test/unit/test_network.cpp` : `cache_basic`, `proj_grid_cache_clear`, `cache_saturation`, `cache_ttl`, `cache_lock` | `tests/unit_network_cache.rs` |
| `test/unit/test_network.cpp` : `curl_hgridshift`, `curl_vgridshift`, `curl_vgridshift_vertcon` | `tests/unit_network_grids.rs` |
| `test/cli/test_projsync.sh` | `tests/unit_projsync.rs`, index de test `tests/projsync/files.geojson`, `tests/projsync/dummy.geojson` |
| grilles de test (GeoTIFF float32 écrits à la volée) | `tests/net_support/grids.rs` |
| serveur local avec en-têtes `ETag` / `Last-Modified` | `tests/net_support/mod.rs` (`spawn_with_headers`) |

## Format du cache

- `cache.db` dans le répertoire utilisateur (`PROJ_USER_WRITABLE_DIRECTORY`, ou
  `set_user_writable_directory`), ou le fichier donné à `set_grid_cache_path`.
- Schéma identique à PROJ : `properties`, `downloaded_file_properties`, `chunk_data`,
  `chunks`, `linked_chunks`, `linked_chunks_head_tail`. Chunks de 16 KiB.
- Les clés étrangères sont coupées, comme dans le SQLite de PROJ. Le SQLite embarqué
  (`rusqlite` bundled) les active par défaut, et l'insertion ou l'invalidation échoue sans
  cela.
- Verrou : `BEGIN EXCLUSIVE`, réessais de 5 ms (10 fois), 10 ms (10 fois), puis 100 ms,
  jusqu'à `PROJ_LOCK_MAX_ITERS` (30 par défaut). Le délai d'attente SQLite est à 0 : sinon
  SQLite attend 5 s à chaque essai et le verrou ne se mesure plus.
- Taille : 300 Mo par défaut (`cache_size_MB`), sans limite à 0. `set_grid_cache_max_size_mb(Some(0))`
  lit `PROJ_GRID_CACHE_MAX_SIZE_BYTES`, comme `proj_grid_cache_set_max_size`.
- TTL : 86 400 s par défaut (`cache_ttl_sec`). `GridCacheTtl::Never` ou 0 : jamais expiré
  (PROJ n'expire que si `ttl > 0`).

## Écarts

1. **Fichier entier, pas de lecture par plages.** Un téléchargement écrit tous ses chunks,
   du dernier au premier, pour que la tête de la liste LRU soit le chunk 0 : c'est ce que
   `cache_basic` attend. Cause : la couche réseau lit le fichier complet
   (`http_get_validated`), sans `Range`. Le lot « lecture par plages » devra écrire les
   seuls chunks lus, comme PROJ. Les deux compteurs de `cache_saturation` (2 chunks, 2
   lignes de liens) restent vrais, car la grille de test fait 9 chunks.
2. **Pas de requête conditionnelle.** Une entrée expirée est téléchargée en entier. PROJ
   envoie `If-None-Match` / `If-Modified-Since` et garde le cache sur un `304`. Les
   en-têtes `ETag` et `Last-Modified` sont stockés, comme PROJ.
3. **`cache_lock`.** PROJ compte le temps en secondes entières (`time_t`, `>= 1`). Ici la
   durée réelle est vérifiée contre le calendrier de PROJ : 25 essais, soit 650 ms au
   minimum.
4. **Valeurs de référence.** Le CDN (`cdn.proj.org`) n'est pas accessible. Les grilles sont
   des fixtures à nous, et les valeurs (`1.9992776848`, `-31.89`, `-0.15` du C++) ne sont pas
   reprises : chaque test porte la valeur que PROJ 9.9.0 donne sur les mêmes octets.
5. **`proj_assign_context`** (dans les trois cas `curl_*`) n'est pas porté : le contexte
   est porté par le builder, il n'y a rien à réassigner.
6. **Binaire de référence.** `tools/build-reference.sh` compile PROJ sans curl ni TIFF.
   Dans ce build, `cs2cs` répond `Network functionality not available`, ne lit pas les
   `.tif`, et c'est la cause du `could not find required grid(s)` des fixtures. Les valeurs
   viennent donc de `target/proj-reference/build-net` (curl et TIFF activés). Même avec
   curl, ce build télécharge mal depuis un serveur sans `206` (`python3 -m http.server`) :
   `proj_download_file` échoue sur `files.geojson`. Les calculs de grille sont faits sur
   les fichiers locaux (mêmes octets).
7. **`projsync` : options.** `--user-directory` (nom de la cible par défaut), `--help` et
   `--version` ne sont pas dans PROJ 9.9.0 : ils sont ajoutés. `--system-directory` prend le
   premier répertoire de `PROJ_DATA` ou `PROJ_LIB`, et échoue sans l'un des deux : PROJ
   utilise un répertoire « share » compilé, que la crate n'expose pas.
8. **`projsync` : téléchargement.** Même écart que le point 2 : un fichier expiré est
   téléchargé à nouveau. La ligne `Downloading ...` du mode réel n'est pas comparée au
   binaire de référence (point 6) : le texte vient de `projsync.cpp`, lignes 588-613.
9. **`projsync` : index.** L'index des tests est un fichier de test de même forme que
   `files.geojson`, pas celui du CDN. Les sorties en mode `--dry-run`, `--list-files` et
   messages d'erreur sont celles du binaire de référence sur ce même index (data version
   1.25).
10. **Booléens.** Les options publiques n'ont pas de paramètre `bool` : `GridCacheMode`,
    `GridCacheTtl`, `SpatialTest`, `WorldCoverage`, `VersionFiltering`. `grid_cache_enable(bool)`
    reste, pour la compatibilité, et s'appuie sur `set_grid_cache_enabled`.

## Commandes

```
cargo test --test unit_network_cache --test unit_network_grids --test unit_projsync --test network_grid
```

Binaire de référence avec curl et TIFF (une fois, sous Debian/Ubuntu avec `libcurl4-openssl-dev`
et `libtiff-dev`) :

```
cmake -S target/proj-reference/src -B target/proj-reference/build-net \
  -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=OFF -DENABLE_TIFF=ON -DENABLE_CURL=ON \
  -DBUILD_PROJSYNC=ON -DEXE_SQLITE3=$PWD/target/proj-reference/shim/sqlite3
cmake --build target/proj-reference/build-net
```

Comparaison d'une grille (exemple, `cct` sur la fixture écrite par
`tests/net_support/grids.rs::cache_grid`) :

```
echo "2 49 0 0" | PROJ_DATA=target/proj-reference/build-net/data \
  LD_LIBRARY_PATH=target/proj-reference/build-net/lib \
  target/proj-reference/build-net/bin/cct -d 10 +proj=pipeline \
  +step +proj=unitconvert +xy_in=deg +xy_out=rad \
  +step +proj=hgridshift +grids=<fichier.tif> \
  +step +proj=unitconvert +xy_in=rad +xy_out=deg
```
