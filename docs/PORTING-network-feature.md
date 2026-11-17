# Fiche de portage : la feature `network`

Décision de l'opérateur : « on fait comme PROJ, pas d'OpenSSL dans le binaire ». PROJ n'a de
client réseau que compilé avec `ENABLE_CURL` (`CMakeLists.txt:277`). proj-rs suit : le client HTTP
est derrière la feature Cargo `network`, désactivée par défaut, qui seule tire `native-tls`
(OpenSSL sous Linux, SChannel sous Windows, Security.framework sous macOS).

## Ce que PROJ 9.9.0 fait sans libcurl

Source : `src/networkfilemanager.cpp`.

| PROJ | Sans curl | Rust |
|---|---|---|
| `FileManager::fillDefaultNetworkInterface` (2233) | `open = no_op_network_open`, `close = no_op_network_close`, pas de `read_range` | `NoNetwork` (`src/network.rs`), client par défaut du contexte quand le réseau est actif |
| `no_op_network_open` (2208) | échec, message « Network functionality not available » | `NetworkError::Open` / `ReadRange` avec ce message |
| `proj_context_set_enable_network` (2343) | pose le drapeau ; rend `enabled && open != no_op_network_open` | `ProjBuilder::enable_network` rend `Ok(1)` seulement si des callbacks sont posés, `Ok(0)` sinon ; le drapeau est posé dans les deux cas |
| `proj_context_is_network_enabled` (2368) | rend le drapeau | `ProjBuilder::network_enabled`, `Context::network` : inchangés |
| `proj_context_set_network_callbacks` (2300) | remplace `open`/`close`/`read_range`/`get_header_value` | `ProjBuilder::set_network_callbacks`, `proj_context_set_network_callbacks` du crate `capi` : marchent sans la feature |
| `FileManager::open` (`filemanager.cpp:981`) | URL ou nom court avec réseau actif : appelle `open` du contexte, qui échoue | `network::open_remote_grid`, `fetch_missing` lisent par `reading_callbacks()` : le `NoNetwork` échoue, la grille est absente, même erreur que réseau coupé |
| `src/apps/CMakeLists.txt:82` | `projsync` « requires Curl » | `[[bin]] projsync` a `required-features = ["network"]` |

`PROJ_NETWORK=ON` et `network = on` de `proj.ini` posent le drapeau, rien de plus (`filemanager.cpp:1996`,
`2074`).

## Décision : le client en clair part avec la feature

Sans curl PROJ n'a aucun client, `http://` compris : le seul client est celui de l'utilisateur,
fourni par `proj_context_set_network_callbacks`. Le client HTTP/1.1 de proj-rs (`src/network_http.rs`,
`src/network_builtin.rs`) est donc entièrement derrière la feature, `http` et `https`. Les tests sur
serveur local (`tests/net_support`) passent par lui et ne tournent qu'avec la feature.

Reste disponible sans la feature : `NetworkCallbacks`, `ProjBuilder::set_network_callbacks`, le cache
de morceaux, la lecture par plages, `projsync::download` (qui échoue sans client, comme
`proj_download_file` sans curl), l'API publique en entier. Aucune méthode publique n'est
conditionnée par `cfg`.

## Écarts

- `proj_context_set_enable_network` rend un `Result<u8, ProjError>` (signature georust), jamais
  `Err`.
- Les crates `capi` et `georust` ont leur propre feature `network` qui active celle de `proj-rs`, sans
  l'activer par défaut. Sans elle, `proj_context_set_enable_network(ctx, 1)` rend 0 du `capi`, comme
  une libproj sans curl.

## Preuves

- `tests/network_feature_off.rs` (sans la feature) : valeur de retour de l'activation avec et sans
  callbacks, grille distante absente avec la même erreur que réseau coupé et aucune requête reçue
  par un serveur local, `PROJ_NETWORK=ON`, callbacks seuls clients.
- Les 12 fichiers de tests réseau et le test `parity_network` ont `#![cfg(feature = "network")]`.
- `cargo tree -e normal` sans la feature ne liste ni `native-tls` ni `openssl` ; `ldd` d'un binaire
  release ne liste ni `libssl` ni `libcrypto`.
