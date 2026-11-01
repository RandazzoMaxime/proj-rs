# Port — façade : définition, `Debug` et aire d'usage (`proj_info`)

Sources, PROJ 9.9.0 (`src/`, téléchargées depuis `raw.githubusercontent.com/OSGeo/PROJ/9.9.0/`) :

- `src/init.cpp` : `pj_init_ctx` garde la liste des paramètres dans `PIN->params` (ligne 552).
  `append_default_ellipsoid_to_paralist` (lignes 321 à 365) ajoute `ellps=GRS80` à une chaîne
  qui ne nomme ni datum, ni ellipsoïde, ni `no_defs`. `pj_datum_set` n'est appelé que hors
  pipeline (ligne 564).
- `src/datum_set.cpp` : `pj_datum_set` ajoute en fin de liste `ellps=` puis la définition du
  datum (`towgs84=` ou `nadgrids=`), sans tester si ces paramètres existent déjà. Le premier
  `datum=` est pris.
- `src/datums.cpp` : table `pj_datums`, dix entrées. `GRS80` n'y figure pas.
- `src/iso19111/c_api.cpp` : `proj_get_area_of_use_ex` (lignes 2009 à 2068). Il lit l'usage 0
  seulement, son nom est la description de l'usage, la boîte n'est lue que si le premier élément
  géographique est une boîte. Sinon la boîte vaut `-1000` et la réponse reste vraie.

## Ce que fait PROJ

Le brief disait que `pj_get_def` écrit la liste des paramètres puis les ajouts de
`pj_datum_set`. Je n'ai pas trouvé le corps de `pj_get_def` dans les fichiers 9.9.0 récupérés
(`init.cpp`, `datum_set.cpp`, `internal.cpp`, `param.cpp`, `list.cpp`, `iso19111/c_api.cpp`).
La règle a donc été établie par mesure. `pyproj` lit `proj_pj_info(P).definition` (propriété
`definition` de `pyproj/_transformer.pyx`). Il embarque PROJ 9.8.1, faute de binaire 9.9.0 ici.

La définition n'est pas la liste d'entrée réécrite. `+proj=longlat` donne
`proj=longlat datum=WGS84 no_defs ellps=WGS84 towgs84=0,0,0`, et
`+proj=tmerc … +ellps=GRS80 +units=m +no_defs` donne `proj=utm zone=32 ellps=GRS80 units=m no_defs`.
La chaîne est lue comme CRS, écrite en PROJ, puis initialisée comme `pj_init` :

1. lecture de la chaîne comme CRS (`create_from_user_input`) ;
2. écriture en chaîne PROJ (`Crs::export_to_proj_string`), avec un marqueur `+type=crs` ;
3. `pj_init` : suppression du `+` et du marqueur `type=crs`, qui n'est pas un paramètre PROJ
   (libproj ne l'affiche pas) ;
4. `pj_datum_set` : pour le premier `datum=`, ajout de `ellps=` puis de la définition du datum,
   en fin de liste.

L'étape 4 est celle du brief. Les étapes 1 à 3 manquaient au brief, et sans elles
`+proj=longlat` et `+proj=tmerc` donnent un autre texte que libproj.

## Ce qui a changé

| Fichier | Changement |
|---|---|
| `src/proj_info.rs` (nouveau) | `operation_definition` (étapes 1 à 4), `proj_name`, `crs_area_of_use`, `impl Debug for Proj`, table `PROJ_DATUMS` (les dix datums de PROJ, avec leur ellipsoïde, vérifiée contre `PJ_ELLPS`). |
| `src/proj.rs` | Corps de `area_of_use`, `proj_info` et `def`. `Proj` ne dérive plus `Debug` (`#[derive(Clone)]`). |
| `src/lib.rs` | `mod proj_info;`. |
| `tests/georust_parity.rs` | Trois `#[ignore]` retirés : `test_definition`, `test_debug`, `test_area_of_use`. Passent : 21 sur 25 (était 18). |
| `tests/facade_info.rs` (nouveau) | Cinq tests : 14 définitions contre libproj, 4 refus, `id` et description, aire de `EPSG:4326` (`World.`), aire d'une chaîne PROJ (toujours une erreur). |

API publique : `impl fmt::Debug for Proj` (à la main, remplace le dérivé). Aucune signature
publique ajoutée ou modifiée.

## Écarts volontaires

- **Refus explicite.** Une chaîne que la crate ne lit pas donne `ProjError::Definition`, pas le
  texte d'entrée. Le brief demandait cela pour un datum inconnu ; je l'étends aux autres cas.
- **Aire d'usage.** `usage 0` du CRS (`domains().first()`), comme `proj_get_area_of_use`. Je n'ai
  pas pris `crs_extent`, qui intersecte les composantes d'un CRS composé. libproj ne fait pas cela.
- **Pas de boîte.** `None` au lieu de `-1000`.
- **`ellps=GRS80` par défaut non codé.** L'export écrit toujours un ellipsoïde, un datum ou une
  sphère. La règle de `pj_init` ne s'applique donc jamais à cette entrée.
- **`GRS80` comme datum.** La crate le lit, PROJ le refuse. `def()` le refuse aussi, mais
  `Proj::new` l'accepte encore (écart existant, non traité ici).
- **`proj_info` d'une chaîne PROJ.** `id` vaut le `proj=` de la définition, `description` vaut
  `PROJ-based coordinate operation`, `definition` vaut `def()`. Les autres façades (CRS, paire
  `new_known_crs`) gardent l'ancien `proj_info`.

## Résultats contre libproj (pyproj, PROJ 9.8.1)

Trente-huit chaînes (`+proj=longlat`, UTM, Lambert, Mercator, tous les datums de la table, des
`+towgs84=`, `+pm=`, `+R=`, `+a=`/`+b=`, un pipeline et `+init=`) :

| Résultat | Nombre |
|---|---|
| texte identique à libproj | 33 |
| refus des deux côtés (`+datum=GRS80`, `+proj=gridshift` sans grille) | 2 |
| refus explicite de la crate, libproj répond (pipeline, `cart`, `+init=`) | 3 |

Aucune réponse fausse : chaque écart est un refus.

## Ce qui reste ouvert

- **`+proj=pipeline`.** Refusé. libproj lit une étape unique comme un CRS
  (`proj=longlat datum=NAD27 …`). Les pipelines de plusieurs étapes ne sont pas lus.
- **`+proj=cart`.** Refusé. libproj donne `proj=geocent ellps=WGS84 units=m no_defs`. L'export
  ISO de la crate refuse `cart`.
- **`+init=`.** Refusé. libproj développe `epsg:4326`. La crate ne le lit pas dans ce chemin.
- **Objets construits à partir d'un CRS** (`EPSG:`, WKT, PROJJSON). `def()` rend toujours le
  texte donné, comme avant. Non vérifié contre libproj.
- **`new_known_crs` et `proj_known_crs`.** `def()` et `proj_info()` inchangés (texte de la paire).
- **`area_of_use` d'une transformation** (`new_known_crs`). Toujours `UnknownAreaOfUse`, comme le
  demande le brief.
- **Référence.** Les mesures viennent de PROJ 9.8.1. Le binaire 9.9.0 n'est pas disponible ici :
  la CI doit confirmer les valeurs de `tests/facade_info.rs` contre 9.9.0.
- **Coût.** `def()`, `Debug` et `area_of_use()` ouvrent `proj.db` à chaque appel. Pas de cache.

## Commandes et codes de sortie

Les sondes (une chaîne par ligne, comparées à `pyproj`) ont utilisé un exemple temporaire,
supprimé avant le commit.

| Commande | Code | Note |
|---|---|---|
| `cargo test --all-features --test georust_parity -- --ignored test_definition test_debug test_area_of_use` | 0 | avant le retrait des `#[ignore]` : 3 passés |
| `cargo test -q --all-features --test facade_info --test georust_parity` | 0 | 5 passés ; georust : 21 passés, 4 ignorés |
| `cargo fmt --check` | 0 | après `cargo fmt` (deux fichiers reformatés) |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | |
| `cargo test --all-features --no-fail-fast` (avant régénération) | 101 | seul `counts_in_docs_match_the_ledgers` échoue : le README dit 18 |
| `UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg` | 0 | README et ROADMAP : 18 devient 21 ; UNIT-TRIAGE et `progress.svg` ne changent pas |
| `cargo test --test docs_in_sync --test progress_svg` (sans `UPDATE_DOCS`) | 0 | |
| `cargo test --all-features --no-fail-fast` (final) | 0 | aucun `FAILED` dans la sortie |

Le ROADMAP garde, dans le texte de la ligne 2, la phrase « Still open : … the `Debug` and
`def()` text ». Elle n'est plus vraie ; elle reste à réécrire par le mainteneur, le brief
interdisant d'éditer la prose de ce fichier.
