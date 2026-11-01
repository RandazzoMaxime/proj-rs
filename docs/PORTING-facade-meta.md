# Port — façade : CRS composés (`EPSG:A+B`) et objets de métadonnées de coordonnées

Sources, PROJ 9.9.0 :

- `src/iso19111/operation/coordinateoperationfactory.cpp` : dispatch de
  `createOperations` (~3767 à 3805) : `compoundSrc && geogDst` vers
  `createOperationsCompoundToGeog`, `geodSrc && compoundDst` vers
  `applyInverse(createOperations(compound, geod))`.
- `createOperationsCompoundToGeog` (~6459) : « Only do a vertical transformation if
  the target CRS is 3D » (~6687). Pour une cible à deux axes, seule la partie
  horizontale est prise.
- `tests/cli_yaml/test_cs2cs_various.yaml` (lignes 1276 à 1330) : sortie attendue de
  `cs2cs` pour `EPSG:4326+3855` vers `EPSG:4979` sans la grille EGM2008.
- Métadonnées : `proj_coordinate_metadata_create` et
  `proj_coordinate_metadata_get_epoch` (API C de libproj) : un CRS et une époque.

## Le problème

`Proj::new_known_crs("EPSG:4326", "EPSG:4326+3855")` échouait à la création avec
`unknown CRS EPSG:4326+3855`. Le choix ISO (`best_pipeline_exact`) sait lire le nom
composé (`crs_from_authority_code`, port de `io.cpp`), mais `iso_pipeline` refusait
tout CRS qui n'était pas géographique ou projeté. L'ancien chemin (`Crs::parse`) ne
lit pas non plus le nom composé, d'où le message.

`ProjCreateError::MetadataObjectCreation` était renvoyé pour tout appel à
`coordinate_metadata_create`, et `coordinate_metadata_get_epoch` rendait toujours NaN.

## Ce que fait PROJ

Pour une paire dont un côté est composé, `createOperations` choisit la partie
horizontale du CRS composé et écrit la partie verticale quand la cible est en 3D. Une
cible à deux axes ne prend que l'horizontal. Dans l'autre sens (2D vers composé),
l'opération est l'inverse de « composé vers 2D » : aucune hauteur n'est calculée et
la cible reçoit une hauteur nulle.

Sans la grille EGM2008 sur disque, `cs2cs` par défaut (`--only-best` absent) rend la
hauteur inchangée (`0.000`), et `--only-best` échoue à la transformation avec
« Grid us_nga_egm08_25.tif is not available ». Une époque ne se transporte qu'avec une
transformation dépendante du temps.

## Ce que j'ai changé

| Fichier | Changement |
|---|---|
| `src/proj_iso.rs` | `iso_pipeline` accepte un CRS composé : la partie horizontale (premier de deux composants) fixe l'ordre des axes et les unités. Nouveau `refusal` : une conversion avant d'un CRS 2D vers un CRS composé est refusée (voir Écarts). `IsoPipeline::refusal(way)`. |
| `src/proj.rs` | Champ `epoch: Option<f64>` (`pub(crate)`, `None` dans `built`). Corps de `coordinate_metadata_create` (un CRS seul, sinon `MetadataObjectCreation`), de `coordinate_metadata_get_epoch`, et refus de `create_crs_to_crs_from_pj` si une époque est présente. Un bras dans `map_xyz` : la conversion refusée renvoie `ProjError::Conversion` (ou `Projection`) avec le message, sans appel au pipeline. |
| `src/proj_meta.rs` (nouveau) | `with_epoch`, `epoch_of`, `time_dependent_refusal`. |
| `src/lib.rs` | `mod proj_meta;`, à côté de `mod proj_iso;` qui est déclaré là. |
| `tests/georust_parity.rs` | `#[ignore]` retiré de `test_metadata_creation` et `test_set_endpoint`, qui passent sans modification de leurs valeurs. |
| `tests/proj_meta_facade.rs` (nouveau) | Huit cas : création du composé, conversion refusée avant, composé vers 2D qui garde l'horizontal (avant et inverse), époque lue et NaN sans époque, objet à partir d'une transformation refusé, objet source ou cible refusé (façade et `ProjBuilder`), objet qui ne convertit pas. |

## Écarts volontaires

- **2D vers composé : conversion refusée, pas la hauteur nulle de PROJ.** La sélection
  écrit pour `EPSG:4326` vers `EPSG:4326+3855` une opération nulle (`Inverse of Null
  geographic offset`) : la hauteur ellipsoïdique (0) devient une hauteur EGM2008.
  C'est une hauteur inventée. La façade crée l'objet (le test le demande) et refuse la
  conversion avant, avec `ProjError::Conversion`. La conversion arrière (composé vers
  2D) garde l'horizontal et n'invente rien. La règle ne dépend pas des grilles : elle
  tient aussi quand la grille est présente, car la sélection ne donne pas de géoïde
  pour ce sens. Ce point est à reprendre avec la sélection (voir Reste ouvert).
- **Composé vers 3D sans grille : création refusée.** `EPSG:4326+3855` vers
  `EPSG:4979` sans grille donne une ballpark, approchée : la façade la refuse à la
  création, comme pour les autres approchées (`tests/proj_facade.rs`). PROJ rend la
  hauteur inchangée par défaut. Le message est celui de l'ancien chemin
  (`unknown CRS`), trompeur pour un nom connu.
- **Époque : refus systématique.** PROJ n'applique l'époque que là où la transformation
  dépend du temps. Ici, toute transformation dont une extrémité porte une époque est
  refusée à la création, sans chercher si elle dépend du temps. Pas d'époque ignorée.
- **Objet de métadonnées sans CRS seul.** `coordinate_metadata_create` sur une
  transformation renvoie `MetadataObjectCreation`. Libproj le refuse aussi.
- **Métadonnées : pas de conversion.** L'objet d'un CRS est un CRS : `convert` et
  `project` renvoient l'erreur « CRS is not a transform », comme sur le CRS seul.

## Reste ouvert

- **Sélection du composé vers 3D avec `--only-best`.** Avec `OnlyBest::On`,
  `EPSG:4326+3855` vers `EPSG:4979` donne la ballpark (noop) en premier, alors que
  PROJ classe le géoïde `Inverse of WGS 84 to EGM2008 height (1)` en premier : c'est
  son échec attendu dans `test_cs2cs_various.yaml`. Cause probable, non vérifiée : le
  classement de `select.rs` (`precedes`) ne reproduit pas celui de PROJ pour les
  candidats composés. Relève du lot `cs2cs` (`--only-best`), non traité ici.
- **Message trompeur.** Une paire composée sans opération exacte (grille absente,
  ballpark) reçoit `unknown CRS`, alors que le CRS existe. Il faudrait un message
  dédié dans `new_known_crs`.
- **Géoïde pour le sens 2D vers composé.** Le candidat géoïde manque (la sélection
  ne liste que la nulle). PROJ fait la même chose (inverse de la nulle), donc ce n'est
  pas un écart de port, mais la façade ne peut pas donner la hauteur géoïdale avec la
  grille présente. Un cas de réseau avec grille reste à vérifier.
- **`Proj` `Debug`, aire, WKT, PROJJSON, `def()`** : ouverts, hors de ce lot. Le
  champ `epoch` apparaît dans le `Debug` (même écart que pour le reste du `Debug`).
- **Hauteur d'un CRS composé dans `convert_xyz`.** Une conversion 3D depuis un
  composé n'est pas testée au-delà du cas horizontal.

## Commandes et codes de sortie

| Commande | Code | Note |
|---|---|---|
| `cargo test --all-features --test georust_parity -- --ignored test_metadata_creation test_set_endpoint` (avant changement) | 101 | les deux échouent : `MetadataObjectCreation`, `unknown CRS EPSG:4326+3855` |
| même commande (après) | 0 | les deux passent |
| `cargo test --all-features --test proj_meta_facade --test georust_parity` | 0 | 8 passés (nouveau fichier), 20 passés et 5 ignorés (parité) |
| `cargo fmt` puis `cargo fmt --check` | 0 | `tests/proj_meta_facade.rs` reformaté |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | |
| `cargo test --all-features --no-fail-fast` (avant regénération des documents) | 101 | 809 passés, 1 échec : `counts_in_docs_match_the_ledgers` (README désaligné sur le compte georust, attendu) |
| `UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg` | 0 | README et ROADMAP mis à jour (compte georust 18 vers 20) |
| `cargo test --all-features --no-fail-fast` (final) | 0 | 810 passés, 0 échec |
