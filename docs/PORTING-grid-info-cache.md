# Fiche de portage : les fiches de grilles d'un contexte (`cacheGridInfo_`)

## Ce que fait PROJ 9.9.0

`DatabaseContext::lookForGridInfo` (`factory.cpp:3398-3539`) répond, pour un nom de grille, par
son nom de fichier, son paquet, son URL, ses drapeaux de licence et de téléchargement, et sa
disponibilité. La réponse est gardée dans `cacheGridInfo_`, un `lru11::Cache` du `DatabaseContext`
du contexte (`factory.cpp:860-871, 1222-1238`) :

- la clé est le nom, l'état du réseau du contexte au moment de l'appel et
  `considerKnownGridsAsAvailable` (`makeCachedGridKey`) ;
- 128 clés, qui peuvent monter à 138 avant d'être élaguées aux 128 plus récemment lues ou
  écrites (`lru_cache.hpp:109-217`, élasticité 10) ; une lecture remonte la clé en tête ;
- une grille dont l'ouverture réseau a échoué (`Cannot open <url>: <error>`,
  `networkfilemanager.cpp:1323`) n'est pas retentée par ce contexte tant que sa fiche reste : le
  nombre de lignes `Cannot open` d'un appel dépend de tous les appels d'avant sur le contexte ;
- `invalidateGridInfo` retire les quatre fiches d'un nom avant un téléchargement
  (`networkfilemanager.cpp:2690-2705`) ;
- un nouveau `DatabaseContext` part vide : contexte créé ou cloné (`projCppContext::clone`),
  chemin de base posé (`c_api.cpp:372-390`), `proj_cleanup` (`closeDb`).

Les appels qui passent par lui, dans l'ordre où ils ouvrent et lisent le cache :

| appel PROJ | réseau | `considerKnown…` |
|---|---|---|
| `FilterResults::sort` (`coordinateoperationfactory.cpp:1477-1500`), chaque opération dans l'ordre d'avant le tri, chaque grille dans l'ordre des paramètres | celui du contexte | politique `KNOWN_AVAILABLE` |
| `rejectOpDueToMissingGrid` (`factory.cpp:4329-4366`) | coupé (`DisableNetwork`) | politique `KNOWN_AVAILABLE` |
| la recherche qui précède une ouverture réseau dans `open_resource_file` (`filemanager.cpp:1859-1879`), sous le nom PROJ d'un nom sans `.tif` | coupé | oui |
| `proj_coordoperation_get_grid_used_count` (une fois par objet, `gridsNeededAsked`) et `proj_coordoperation_is_instantiable` (`c_api.cpp:7551, 7920-7930`) | celui du contexte | réseau activé |
| `proj_grid_get_info_from_database` (`c_api.cpp:974`) | celui du contexte | non |
| `createBoundCRSToWGS84IfPossible` (`crs.cpp:626`), les transformations verticales (`coordinateoperationfactory.cpp:6719, 7466`) | celui du contexte | oui |

## Le port

- `src/grid_info.rs` : le LRU (`GridInfoCache`, mêmes tailles, même ordre d'éviction) et
  `with_network_off`, le `proj_context_set_enable_network(ctx, false)` des appels ci-dessus.
- Le cache vit dans le `GridContext` du constructeur (`ProjBuilder`), partagé par ses clones
  comme les objets d'un contexte PROJ partagent son `DatabaseContext`. La bibliothèque C le
  renouvelle là où PROJ refait le `DatabaseContext` (`ProjBuilder::reset_grid_records` dans
  `proj_context_clone`, `proj_context_set_database_path`, `proj_cleanup`).
- `DatabaseContext::look_for_grid` (`src/iso19111/factory.rs`) est `lookForGridInfo` : la fiche en
  cache, sinon la requête, `resolveFullFilename` (disque, puis réseau précédé de la recherche
  imbriquée), et l'insertion après les recherches imbriquées, comme PROJ.
- `gridsNeeded` (`grid_descriptions`, `src/iso19111/select.rs`) cherche chaque nom dans l'ordre
  des paramètres et des étapes, un nom répété compris, puis rend l'ensemble trié de PROJ. Le tri
  des candidats le lit pour toutes les politiques, sous les noms PROJ des grilles.
- Le second classement des candidats de `proj_create_crs_to_crs` (`rank`, `select.rs`), que PROJ
  ne fait pas, lit les fiches sans les déplacer ni en ajouter (`with_records_untouched`).
- `capi/src/ops.rs` garde la liste des grilles sur l'objet (`grids_needed`), et lance les
  recherches sous le réseau et le cache du contexte (`ProjBuilder::within`).

Vérification : `capi/tests/run.sh grid_miss_cache` (`capi/tests/grid_miss_cache.c`, mêmes
comptes que la libproj de référence pour 28 appels) et `tests/grid_miss_cache.rs`. La suite des
clés lues par les deux bibliothèques pour ces séries a aussi été comparée ligne à ligne (gdb sur
`lookForGridInfo` côté PROJ) : identique.

## Écarts volontaires

- La recherche imbriquée d'une ouverture réseau faite au moment de transformer un point (lecture
  d'une grille par `gridshift`, sans `DatabaseContext` à portée) lit la base embarquée, là où
  PROJ lit celle du contexte ; seule une base de contexte dont `grid_alternatives` diffère de la
  base embarquée y verrait une autre fiche.
- Le repli de `proj_trans` sur la première opération sans grille (`trans.cpp:322`) ne passe pas
  par le cache : il ne logue rien (`considerKnown…` vrai) et ne touche qu'à l'ordre d'éviction.
- `proj_context_set_autoclose_database` ne vide pas le cache après chaque appel, où PROJ ferme le
  `DatabaseContext`.
