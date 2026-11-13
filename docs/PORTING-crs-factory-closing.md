# Lot : fermeture de tests CRS et factory (test_crs.cpp, test_factory.cpp)

Référence : PROJ 9.9.0, `src/test/unit/test_crs.cpp` et `test_factory.cpp`.
Les valeurs attendues viennent du C++ ou des binaires 9.9.0 (`projinfo`, avec
`PROJ_DATA=target/proj-reference/build/data`), jamais du code testé.

## Bilan

| Test | État | Fichier de test |
|---|---|---|
| `crs.scope_area_bbox_remark` | fermé | `tests/unit_crs_scope_area_bbox_remark.rs` |
| `factory.AuthorityFactory_createDynamicVerticalDatum` | fermé, comportement 9.9.0 | `tests/unit_factory_dynamic_vdatum.rs` |
| `FactoryWithTmpDatabase.custom_projected_crs` | fermé | `tests/unit_factory_custom_projected_crs.rs` |
| `FactoryWithTmpDatabase.custom_derived_projected_crs` | fermé | `tests/unit_factory_custom_derived_projected_crs.rs` |
| `factory.objectInsertion` | ouvert (voir plus bas) | aucun |

Quatre lignes sont retirées de `docs/UNPORTED-UNIT.md`. La ligne
`factory.objectInsertion` reste.

## crs.scope_area_bbox_remark

Le modèle n'avait ni étendue verticale ni étendue temporelle sur un domaine.
Ajouts :

- `ObjectDomain` porte `vertical: Option<VerticalExtent>` et
  `temporal: Option<TemporalExtent>`, avec les accesseurs publics
  `vertical_extent()` et `temporal_extent()`.
- `VerticalExtent` devient public (`minimum()`, `maximum()`, `unit()`) avec une
  égalité qui compare les bornes et l'unité. `TemporalExtent` (`start()`,
  `stop()`) est nouveau.
- WKT2 : import de `VERTICALEXTENT[min,max{,LENGTHUNIT}]` (le mètre à défaut) et
  de `TIMEEXTENT[start,stop]` ; export après `BBOX`, dans l'ordre de PROJ.
- PROJJSON : lecture et écriture de `vertical_extent` et `temporal_extent` dans
  les usages. Quand le domaine porte son extension verticale, l'extension
  verticale du `Identified` n'est pas écrite une seconde fois.

Pas de changement de la base : `factory.rs` (`create_extent`) lit toujours les
seules colonnes de `extent` ; l'étendue verticale et temporelle des usages
stockés n'est pas encore lue depuis `proj.db`.

## factory.AuthorityFactory_createDynamicVerticalDatum

Le `TEST()` de PROJ 9.9.0 est sous `#ifdef no_more_dynamic_vertical_datum`
(`test_factory.cpp`, l. 320-330) : il n'est pas compilé. Il attend que EPSG:1096
soit une `DynamicVerticalReferenceFrame` d'époque 2000. La base 9.9.0 dit autre
chose :

```text
$ projinfo -k datum EPSG:1096 -o PROJJSON
{"type": "VerticalReferenceFrame", "name": "Norway Normal Null 2000:2018", ...}
```

Le test porte donc ce comportement : `VerticalReferenceFrame` nommé « Norway
Normal Null 2000:2018 », sans époque de référence de trame.

## FactoryWithTmpDatabase.custom_projected_crs et custom_derived_projected_crs

Ces deux tests construisent une base temporaire. Ce qui a été ajouté :

- `DatabaseContext::with_empty_structure()` : une base en mémoire avec les
  tables, vues et triggers du `proj.db` embarqué, la version de layout, et aucune
  ligne d'autorité. C'est le `createStructure` du C++. Écart : le DDL est recopié
  depuis le `proj.db` embarqué (`sqlite_master`), et seules les deux lignes
  `DATABASE.LAYOUT.VERSION.*` de `metadata` sont recopiées.
- `DatabaseContext::execute_sql(&str)` : exécute du SQL sur cette base. Le C++
  l'appelle `execute()` ; c'est la seule voie d'écriture publique.
- `tests/support/fake_epsg.rs` : les 44 instructions de `populateWithFakeEPSG`,
  extraites du C++ dans le même ordre, et `database()` qui fait
  `createStructure` + `PRAGMA foreign_keys = 1` + le remplissage.

Expansion des `text_definition` :

- Une ligne `projected_crs` dont seul `text_definition` est renseigné est lue
  par `create_from_user_input` (WKT ou chaîne PROJ). Une chaîne PROJ reçoit
  `+type=crs`, comme le fait PROJ pour une définition de table. Le résultat prend
  le nom, l'identifiant, l'indicateur `deprecated` et les usages de la ligne. Un
  `BoundCRS` (`+towgs84`) donne son lien en `canonical_bound`.
- `create_derived_projected_crs` essaie d'abord les colonnes, puis le texte
  `DERIVEDPROJCRS` (même logique que pour `projected_crs`).
- Une chaîne PROJ à projection inconnue de la table (`+proj=mbt_s`) reste une
  CRS « PROJ-based ». Deux corrections l'ont permis :
  - la méthode `PROJ <nom>` s'écrit en `+proj=<nom>` avec ses termes
    (`buildop.rs`, `proj_based_string`). Avant, l'export échouait avec
    « Unsupported conversion method » ;
  - le texte d'extension (`EXTENSION[PROJ4]`) n'est écrit en export PROJ que s'il
    porte un paramètre autre que `+proj`, `+type` et `+towgs84`
    (`projstring.rs`, `ProjectedCRS::export_with`). Ainsi
    `+proj=mbt_s +unused_flag +type=crs` s'exporte en
    `+proj=mbt_s +datum=WGS84 +units=m +no_defs +type=crs`, comme `projinfo`
    9.9.0. Une première tentative, qui supprimait l'extension de toute CRS
    PROJ-based, a cassé `io_projparse_projected_unknown` et
    `io_projparse_ob_tran_rhealpix` (`tests/unit_io_core_b.rs`) : elle a été
    retirée.

Ces deux tests passent, avec l'identification de chaînes PROJ dans la base et
`get_crs_info_list`.

## factory.objectInsertion : ouvert

Le `TEST()` (`test_factory.cpp`, l. 4411-5252) demande :

- une session d'insertion (`startInsertStatementsSession`,
  `stopInsertStatementsSession`) dont les objets sont visibles pour `identify`
  tant qu'elle est ouverte, et disparaissent après l'arrêt ;
- `suggestsCodeFor`, qui n'existe pas dans le crate ;
- l'insertion de CRS verticales, composées, liées (`BoundCRS`), de datums
  dynamiques, d'ensembles de datums et de méthodes de projection hors EPSG.

`src/iso19111/insert.rs` produit seulement du texte et refuse les CRS
verticales, composées et liées. Il n'y a pas de session, et l'insertion n'est pas
exécutée sur la base. Le test n'a pas été commencé au-delà de cette analyse.

## Notes

- Le brief du lot citait `tests/common/tmp_db.rs`,
  `tests/unit_factory_tmp_db_2.rs`, `docs/PORTING-factory-tmpdb-2.md` et
  `docs/PORTING-database-open.md`. Aucun de ces fichiers n'existe sur cette
  branche : le helper de base temporaire est donc celui de `tests/support/`.
- `docs/PORTING-shape.md` (lignes 98-102) et `docs/PORTING-factory-unit-2.md`
  disent que la base temporaire n'est pas possible. Ce n'est plus vrai pour
  `custom_projected_crs` et `custom_derived_projected_crs` ; ces fiches ne sont
  pas modifiées dans ce lot.

## Commandes

- `cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings` : OK.
- `cargo test --test unit_crs_scope_area_bbox_remark --test unit_factory_dynamic_vdatum --test unit_factory_custom_projected_crs --test unit_factory_custom_derived_projected_crs` : OK.
- Suite complète : `PATH=target/proj-reference/build/bin:$PATH PROJ_DATA=target/proj-reference/build/data cargo test --all-features --no-fail-fast`.
  Seuls échouent les tests de `docs_in_sync` (`unit_triage_is_current`,
  `counts_in_docs_match_the_ledgers`) et `progress_chart_is_current`, attendus
  puisque les compteurs de `UNPORTED-UNIT.md` ont changé et que `UPDATE_DOCS`
  n'a pas été lancé.
- `docs/UNPORTED-UNIT.md` modifié à la main ; `UPDATE_DOCS` non lancé (les
  chiffres de `README.md`, `ROADMAP.md`, `docs/UNIT-TRIAGE.md` doivent être
  régénérés, sinon `docs_in_sync` échoue).
