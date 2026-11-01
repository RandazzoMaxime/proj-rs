# Port — import de `+towgs84`, `+nadgrids`, `+geoidgrids` et des unités verticales

Source : PROJ 9.9.0.

- `src/iso19111/io.cpp` : `PROJStringParser::createFromPROJString` (ligne ~12470),
  `PROJStringParser::Private::buildBoundOrCompoundCRSIfNeeded` (ligne ~11761),
  `buildUnit` (ligne ~10942), `buildEllipsoidalCS` (ligne ~11586), `linearUnitDescs`
  (ligne ~10863).
- `src/iso19111/crs.cpp` : `BoundCRS::createFromTOWGS84` (5973),
  `BoundCRS::createFromNadgrids` (5988), `BoundCRS::getHDatumPROJ4GRIDS` (6034),
  `BoundCRS::getVDatumPROJ4GRIDS` (6049), `BoundCRS::_exportToPROJString` (6175),
  `CompoundCRS::_exportToPROJString` (5522), `VerticalCRS::_exportToPROJString` (3722),
  `GeodeticCRS::addDatumInfoToPROJString` (2538).
- `src/iso19111/operation/transformation.cpp` : `Transformation::createTOWGS84` (1016),
  `createNTv2` (1068), `createGravityRelatedHeightToGeographic3D` (1115).
- `src/iso19111/operation/singleoperation.cpp` : `getPROJ4NadgridsCompatibleFilename` (1998),
  `getHeightToGeographic3DFilename` (2283).
- `src/iso19111/operation/parametervalue.cpp` : `ParameterValue::_exportToWKT` (225).

## Le problème

`create_crs_from_proj_string` lisait `+proj=longlat` (ou une projection) et **ignorait
en silence** `+towgs84`, `+nadgrids` et `+geoidgrids`. Un CRS décrit avec un
décalage de datum était donc rendu sans ce décalage : un nombre faux, sans
erreur. Le port ajoute l'objet lié que PROJ construit, de sorte que le
décalage reste dans le CRS (et que les opérations qui ne savent pas l'utiliser
échouent bruyamment, voir « Reste ouvert »).

## Ce que fait PROJ

`createFromPROJString` construit le CRS de base, puis appelle
`buildBoundOrCompoundCRSIfNeeded` :

1. `+nadgrids` a la priorité sur `+towgs84`. Un `+nadgrids` donne un `BoundCRS`
   vers EPSG:4326 par une transformation NTv2. Le CRS source de la
   transformation est le CRS géographique de la base, ramené sur Greenwich
   (`<nom> (with Greenwich prime meridian)`, axes latitude/longitude) quand
   son méridien d'origine n'est pas Greenwich.
2. Sinon `+towgs84` (3 ou 7 termes) donne une transformation « Geocentric
   translations » (3 termes) ou « Position Vector transformation » (7 termes),
   du CRS géodésique de la base vers EPSG:4326, nommée
   `Transformation from <nom> to WGS84`. Avec une base de données attachée, sept
   termes sont comparés aux lignes Helmert EPSG : si le jeu correspond à une
   rotation « coordinate frame » écrite avec le signe « position vector »,
   les trois rotations sont inversées (issue PROJ #4170).
3. `+geoidgrids` ajoute un CRS vertical `unknown` (datum
   `unknown using geoidgrids=<fichier>`, hauteur liée à la gravité, dans l'unité
   `+vunits`/`+vto_meter`), lié par `GravityRelatedHeight to Geographic3D` à
   EPSG:4979 (ou à ce que dit `+geoid_crs` : `WGS84`, `horizontal_crs`). Le
   résultat est un `CompoundCRS` `unknown` de la base et du vertical lié.

Autres points du même chemin :

- Une chaîne sans `+proj` qui porte `+vunits` ou `+vto_meter` est un `VerticalCRS`.
- Un CRS géographique avec `+vunits`/`+vto_meter` et sans `+geoidgrids` a un
  troisième axe, `ellipsoidal height (h)`, dans cette unité.
- `buildUnit` lit le nom d'unité dans `linearUnitDescs` ; `+vto_meter` l'emporte,
  cherche d'abord une unité connue de même facteur (tolérance relative 1e-10),
  sinon crée une unité `unknown`.
- L'export PROJ d'un `BoundCRS` écrit `+geoidgrids`/`+geoid_crs`, sinon
  `+nadgrids`, sinon `+towgs84`. Un `CompoundCRS` écrit chaque composant dans la
  même chaîne, `+no_defs +type=crs` une seule fois.
- L'export WKT2 abrégé d'un paramètre d'échelle en ppm écrit `1 + ppm × 1e-6`
  (`ParameterValue::_exportToWKT`), d'où `PARAMETER["Scale difference",1.000007`.

## Ce qui est porté

| Rust | PROJ |
|---|---|
| `bound_import::bind_if_needed` (`src/iso19111/bound_import.rs`) | `buildBoundOrCompoundCRSIfNeeded` |
| `bind_towgs84` | `Transformation::createTOWGS84`, `BoundCRS::createFromTOWGS84` |
| `bind_nadgrids` | `BoundCRS::createFromNadgrids`, `Transformation::createNTv2` |
| `bind_geoid`, `geoid_hub` | bloc `geoidgrids` et `geoid_crs` de `buildBoundOrCompoundCRSIfNeeded` |
| `parse_towgs84` + `DatabaseContext::to_wgs84_autocorrect_wrong_values` | correction des signes, issue #4170 |
| `build_unit`, `LINEAR_UNITS` | `buildUnit`, `linearUnitDescs` |
| `vertical_from_units` | branche « steps vides » de `createFromPROJString` |
| `ellipsoidal_height_axis` | troisième axe de `buildEllipsoidalCS` |
| `create_crs_from_proj_string_with_database(definition, db)` | `PROJStringParser::attachDatabaseContext` |
| `export_bound`, `export_compound` (`src/iso19111/bound_export.rs`) | `BoundCRS::_exportToPROJString`, `CompoundCRS::_exportToPROJString` |
| `Transformation::nadgrids_filename` (`bound.rs`) | `getPROJ4NadgridsCompatibleFilename` |
| `Transformation::height_to_geographic_3d_filename` (correction) | `getHeightToGeographic3DFilename` |
| `add_measure` (`wkt.rs`, correction) | `ParameterValue::_exportToWKT`, mode abrégé |

`create_from_user_input` passe sa base de données à l'import : la correction
des signes y est active, comme avec `PROJStringParser().attachDatabaseContext`.
`create_crs_from_proj_string` seul n'a pas de base et ne corrige rien, comme
`PROJStringParser()` sans contexte.

## Tests

Source : `test/unit/test_io.cpp`, PROJ 9.9.0. Les 10 tests sont dans
`tests/test_io_bound.rs`, avec les valeurs attendues de PROJ, sans changement.

| PROJ `io.projparse_*` | Rust (`tests/test_io_bound.rs`) |
|---|---|
| `longlat_towgs84_3_terms` | `io_projparse_longlat_towgs84_3_terms` |
| `longlat_towgs84_7_terms` | `io_projparse_longlat_towgs84_7_terms` |
| `longlat_towgs84_7_terms_autocorrect` | `io_projparse_longlat_towgs84_7_terms_autocorrect` |
| `longlat_nadgrids` | `io_projparse_longlat_nadgrids` |
| `longlat_nadgrids_towgs84_ignored` | `io_projparse_longlat_nadgrids_towgs84_ignored` |
| `longlat_geoidgrids` | `io_projparse_longlat_geoidgrids` |
| `longlat_geoidgrids_vunits` | `io_projparse_longlat_geoidgrids_vunits` |
| `longlat_vunits` | `io_projparse_longlat_vunits` |
| `vunits` | `io_projparse_vunits` |
| `vto_meter` | `io_projparse_vto_meter` |

Les 10 lignes correspondantes sont retirées de `docs/UNPORTED-UNIT.md`.

Tests Rust complémentaires (`bound_import::tests`) : table d'unités et facteur,
nombre de termes de `+towgs84`, priorité de `+nadgrids`, `+towgs84` sur un CRS
projeté, aller-retour de `+geoid_crs=horizontal_crs`, `+nadgrids=@null` du
Mercator web, trois axes avec `+vunits`.

## Écarts volontaires

- **Pas de renommage de grille à l'export.** PROJ applique
  `substitutePROJAlternativeGridNames` quand une base est attachée ; l'export
  `export_to_proj_string` n'a pas de base. Le nom de grille écrit est celui du CRS.
- **`ignoreNadgrids_` réduit au Mercator web.** PROJ ne le pose que pour
  `+proj=merc` avec `+a`=`+b`, `+lat_ts` nul, `+k`=1 et `+nadgrids=@null`. Le port
  teste la même condition (`ignores_nadgrids`) ; il n'ajoute pas le reste du
  traitement du Mercator web (méthode 1024, voir « Reste ouvert »).
- **Export de grille horizontale.** `nadgrids_filename` couvre NTv2 et NADCON
  (fichier de latitude, les deux fichiers étant présents). La méthode GTiff de
  PROJ (`PROJ_WKT2_NAME_METHOD_HORIZONTAL_SHIFT_GTIFF`) n'existe pas dans ce port.
- **Export composé.** `export_compound` retire `+no_defs +type=crs` de chaque
  composant et l'écrit une fois. PROJ le fait dans un seul formateur ; le texte
  est le même pour les cas testés.
- **Erreurs.** Une exception PROJ (`ParsingException`, `InvalidOperation`) est une
  `FactoryError::Invalid`. Le message de `+towgs84` à 2 ou 5 termes est celui de
  PROJ (`Invalid number of elements in TOWGS84Parameters`).

## Quirks conservés

- `+nadgrids` l'emporte sur `+towgs84` : le second est ignoré sans avertissement.
- Les trois termes de `+towgs84` s'exportent sur sept (`1.2,2,3,0,0,0,0`).
- Avec `+geoidgrids`, le CRS horizontal reste en 2D même si `+vunits` est donné ;
  l'unité va au CRS vertical (la transformation reste en mètres).
- Un datum `+datum=WGS84` avec `+towgs84` s'exporte en `+ellps=WGS84 +towgs84=…`,
  sans `+datum=`.

## Reste ouvert

- **Opérations sur un CRS lié.** `CoordinateOperationFactory::create_operations`
  ne gère pas un `BoundCRS` en source ou en cible (comportement déjà en place pour
  un `TOWGS84` WKT1). `projinfo -s '+proj=longlat +ellps=intl +towgs84=… +type=crs'
  -t EPSG:4326` rend donc `Candidate operations found: 0`. Avant ce lot, le
  `+towgs84` était supprimé et la sortie était `+proj=noop`, un résultat faux.
  Le décalage est désormais dans le CRS ; son usage dans une opération est un
  autre lot.
- **CRS géocentrique.** `+proj=geocent` n'est pas importé comme CRS
  (`unsupported +proj=geocent`) ; `bind_towgs84` sait choisir EPSG:4978 et les
  méthodes 1031/1033 pour ce cas, mais il n'est pas atteint.
- **Autres CRS du même chemin.** `+proj=pipeline` comme CRS, `+axis=enu|neu|swu`
  (opération `axisswap`), le Mercator web (`io.projparse_merc_google_mercator*`,
  méthode 1024 `Popular Visualisation Pseudo Mercator`) et les autres tests de
  `docs/PORTING-io-unit-4.md` restent ouverts.
- **`+vunits` sur un CRS projeté.** PROJ ne lit pas `+vunits` pour un CRS
  projeté sans `+geoidgrids` ; le port non plus.
- **`create_from_user_input`.** Une chaîne qui ne contient que `+vunits=… +type=crs`
  n'est pas reconnue comme chaîne PROJ par `create_from_user_input` (PROJ ne la
  reconnaît pas non plus, qui exige `proj=`, `init=` ou `title=`). Les tests
  `vunits` et `vto_meter` passent par `create_crs_from_proj_string`, comme
  `PROJStringParser().createFromPROJString`.
- **Banc.** Aucun cas ajouté à `src/bin/bench.rs` : le lot ne touche pas un chemin
  de calcul.

## Autres changements dans `src`

- `src/iso19111/wkt.rs` : `add_measure` écrit `1 + ppm × 1e-6` pour une échelle en
  `parts per million` dans une transformation abrégée (`BOUNDCRS`). Avant, la
  valeur en ppm était écrite telle quelle (`7` pour `1.000007`), et PROJ relit
  ce champ comme `(valeur - 1) × 1e6` ppm.
- `src/iso19111/bound.rs` : `height_to_geographic_3d_filename` reconnaît aussi la
  méthode PROJ `GravityRelatedHeight to Geographic3D`, qui n'a pas de code EPSG.
- `src/iso19111/projstring.rs` : `ProjExtras` porte `+geoid_crs` et `+nadgrids`.
  `Crs::export_to_proj_string` exporte un `CompoundCRS`, qui était refusé.

## Vérification

Commandes lancées dans le worktree.

```
cargo test --test test_io_bound                           # ok, 10 passed
cargo test --all-features --no-fail-fast                  # ok, tous les binaires
cargo clippy --all-targets --all-features -- -D warnings  # ok
cargo fmt --check                                         # ok
```

`cli_yaml` réécrit `docs/UNPORTED-CLI.md`. Le même fichier est obtenu avec ou sans
ce lot (comparaison sur un export de `HEAD`) : les lignes qui changent
(`test_cs2cs_various.yaml#126`, `#127`, `#128` et `test_projinfo.yaml#58`) viennent de
l'écart entre le fichier commité et l'état de `main`, pas de ce lot. Aucun cas
qui passait ne régresse.

`docs/UNIT-TRIAGE.md` et `tests/docs_in_sync.rs` ne sont pas sur cette base
(`ad1451a`) : `cargo test --test docs_in_sync` échoue avec `no test target named
docs_in_sync`. `README.md` et `ROADMAP.md` ne sont pas modifiés ; leurs compteurs
(`test_io.cpp` portés/ouverts) sont à régénérer quand la base les porte.
