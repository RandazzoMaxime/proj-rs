# Port — `io.projparse_*` : paramètres au niveau du CRS (lot io-7, lot B)

Source : PROJ 9.9.0, `test/unit/test_io.cpp` (téléchargé depuis
`raw.githubusercontent.com/OSGeo/PROJ/9.9.0`), groupe `io.projparse_*` des
unités linéaires, du titre, de `+proj=geocent`, des chaînes refusées et du
Mercator de Google. Côté crate : `tests/test_io_units.rs`,
`src/iso19111/projstring.rs`, `src/iso19111/proj_crs.rs` (nouveau),
`src/iso19111/equiv.rs`.

Ce lot porte **18 `TEST()`** sur les 23 du lot. Il laisse **5 tests ouverts**,
avec la cause de chacun. Les chaînes attendues sont celles de PROJ, sans
modification.

## 0. Base

La base de travail était `main` (`ad1451a`). Les lots io-4, io-5, io-bound et
le tri `docs/UNIT-TRIAGE.md` ne sont que sur `feat/pensive-allen-fkoqxo`
(`cd23575`). La branche du worktree a donc été avancée en avant rapide sur
`cd23575` (aucun commit local à l'époque). Le lot repart de là.

## 1. Portés (18)

| PROJ 9.9.0 `io.projparse_*` | PROJ-RS `tests/test_io_units.rs` | Ce qui est vérifié |
|---|---|---|
| `projected_units` | `io_projparse_projected_units` | `+units=us-ft` : `LENGTHUNIT["US survey foot"]`, `False easting` 1 |
| `projected_to_meter_known` | `io_projparse_projected_to_meter_known` | `+to_meter` égal à un facteur connu : même unité |
| `projected_to_meter_unknown` | `io_projparse_projected_to_meter_unknown` | `+to_meter=0.1234` : unité `unknown` |
| `projected_vunits` | `io_projparse_projected_vunits` | `+vunits=ft` : troisième axe en pieds |
| `longlat_title` | `io_projparse_longlat_title` | `+title` nomme le CRS géographique et son datum, export `+ellps=intl +towgs84` |
| `projected_title` | `io_projparse_projected_title` | `+title` nomme le CRS projeté, base `unknown`, export `+proj=utm +zone=43 +south` |
| `longlat_errors` | `io_projparse_longlat_errors` | 17 chaînes refusées (voir la section 4) |
| `projected_errors` | `io_projparse_projected_errors` | 4 chaînes refusées |
| `errors` | `io_projparse_errors` | 9 chaînes refusées (la chaîne vide, `foo`, `inv`, `step`, …) |
| `geocent` | `io_projparse_geocent` | `GEODCRS` cartésien 3D, WKT2 sur une ligne exact |
| `geocent_towgs84` | `io_projparse_geocent_towgs84` | méthode `Geocentric translations (geocentric domain)`, export `+proj=geocent … +towgs84=1,2,3,0,0,0,0` |
| `non_earth_ellipsoid` | `io_projparse_non_earth_ellipsoid` | `+R=1` : l'export redonne la chaîne |
| `merc_google_mercator` | `io_projparse_merc_google_mercator` | `+a=b=6378137` : méthode 1024, datum WGS 84, export `+proj=merc +a=6378137 +b=6378137 … +nadgrids=@null` |
| `merc_google_mercator_non_metre_unit` | `io_projparse_merc_google_mercator_non_metre_unit` | `+units=ft` : nom `WGS 84 / Pseudo-Mercator (unit ft)`, WKT2_2019 multiligne exact, aller-retour équivalent |
| `merc_not_quite_google_mercator` | `io_projparse_merc_not_quite_google_mercator` | `+lon_0=10` : méthode 1024, datum `unknown using nadgrids=@null` |
| `longlat_wktext` | `io_projparse_longlat_wktext` | `+foo +wktext` ignorés, export `+datum=WGS84` |
| `geocent_wktext` | `io_projparse_geocent_wktext` | export `+proj=geocent +datum=WGS84 +units=m` |
| `projected_wktext` | `io_projparse_projected_wktext` | export `+proj=merc … +datum=WGS84 +units=m` |

Les noms de test Rust sont ceux de PROJ en minuscules, avec le préfixe
`io_projparse_` du fichier.

Trois de ces tests (`longlat_wktext`, `projected_wktext`, `non_earth_ellipsoid`)
et `projected_title` passaient déjà avant ce lot. Les autres exigent les
changements de la section 3.

## 2. Ouverts, avec la cause (5)

### A. `projected_unknown` (`+proj=mbt_s`)

`projected_from_proj_string` refuse `+proj=mbt_s` : `unsupported +proj=mbt_s
for a CRS`. PROJ construit alors la méthode générique `PROJ mbt_s`, avec les
paramètres de la chaîne, `EXTENSION["PROJ4", ...]` et un export WKT1_GDAL
relu. Cette méthode générique n'existe pas dans le port. Le test attend aussi
la forme WKT1 avec `EXTENSION`, que le port n'écrit pas.

Fichier : `src/iso19111/projstring.rs`, `projected_from_proj_string`.

### B. `cart_unit` et `cart_unit_numeric` (`+proj=pipeline +step +proj=cart …`)

Une chaîne `+proj=pipeline` comme CRS n'est pas importée : `unsupported
+proj=pipeline for a CRS`. Le test attend un `GeodeticCRS` cartésien, puis
l'opération `EPSG:4326` vers ce CRS, avec un `+proj=axisswap`, un
`unitconvert` et le pipeline `+proj=cart` exporté. Même cause que le groupe L
de `docs/PORTING-io-unit-4.md`.

Fichier : `src/iso19111/projstring.rs`, `crs_from_proj_string` (pas de branche
pipeline).

### C. `geoc` (`+proj=longlat +geoc`)

Le port importe `+geoc` comme un `GeographicCRS` ordinaire. PROJ construit un
`GeodeticCRS` sphérique (`buildSphericalCS`) dont `isSphericalPlanetocentric()`
est vrai, et dont l'export échoue (`FormattingException`). Le port exporte
`+proj=longlat +datum=WGS84`, sans erreur. Il n'y a pas non plus de méthode
publique `is_spherical_planetocentric`. Le test ne peut pas passer sans
ajouter ce concept à l'API.

Fichier : `src/iso19111/projstring.rs`, `geographic_from_proj_string`.

### D. `init` (`init=epsg:4326`, `init=ITRF2000:ITRF2005`)

Le test appelle `createFromUserInput(text, db, true)`, avec le drapeau de
compatibilité PROJ 4 qui ordonne les axes de `EPSG:4326` en longitude puis
latitude. `create_from_user_input(text, db)` (`src/iso19111/json.rs`) n'a pas ce
paramètre. Le test attend aussi une opération `helmert` exportée depuis
`init=ITRF2000:ITRF2005` et un pipeline `+init=epsg:4326`. Ces deux cas ne sont
pas sondés ; le test est laissé ouvert sans avoir vérifié leur sortie.

## 3. Changements dans `src`

Tous les changements sont dans `src/iso19111/`.

1. **Unités linéaires** (`projstring.rs`, `projected_from_proj_string`). `+units`
   et `+to_meter` passent par `bound_import::build_unit`, comme `buildUnit`. Avant,
   seule l'unité `m` était acceptée. Une valeur linéaire est donnée en mètres
   puis rangée dans l'unité du CRS (`value / facteur`). Les axes prennent cette
   unité.
2. **`+vunits` sur une projection** (`projstring.rs`). `ellipsoidal_height_axis`
   ajoute un troisième axe au CRS géographique de base et au CRS projeté, comme
   `buildEllipsoidalCS` et `buildProjectedCRS`. Le CS passe par
   `CartesianCS::create_allowing_mixed_units`, sans contrainte d'unité commune.
3. **Titre et nappe de grille dans le datum** (`proj_crs.rs`). `+title` nomme le
   CRS géographique et son datum quand le datum vient d'un ellipsoïde (`+R`,
   `+ellps`, `+a`). `+nadgrids` ajoute ` using nadgrids=<fichier>` au nom du
   datum (`createGRF`). Le CRS projeté de base n'a pas de titre : seule la
   nappe compte. Un `+datum=` nommé garde son nom.
4. **`+proj=geocent`** (`proj_crs.rs`, `geocentric_from_proj_string`). Un CRS
   géocentrique sur le datum de la chaîne, avec les unités `+units`. `+towgs84`
   et `+nadgrids` restent gérés par `bound_import`.
5. **Mercator à pseudo-Mercator** (`proj_crs.rs`, `pseudo_mercator`, et
   `projstring.rs`). `+proj=merc` avec `+a=+b`, `+lat_ts` nul, `+k=1` et
   `+nadgrids=@null` choisit la méthode EPSG 1024, et non plus 9804 ou 9805. Si
   `+a=6378137` et que `+lon_0`, `+lat_0`, `+x_0`, `+y_0` sont nuls, le CRS est
   le Pseudo-Mercator Web de `createPseudoMercator` : base EPSG:4326, nom `WGS
   84 / Pseudo-Mercator` (suffixé par ` (unit X)` si `+units` n'est pas `m`),
   paramètres en mètres. Sinon le CRS garde sa base, avec le datum de nappe.
6. **Clés hors méthode** (`projstring.rs`, `NON_METHOD_KEYS`). `to_meter`,
   `vunits`, `vto_meter`, `nadgrids`, `geoidgrids`, `geoid_crs` et `wktext` ne
   bloquent plus la couverture des paramètres d'une méthode. Avant, `+to_meter`
   faisait échouer `tmerc`.
7. **Export du pseudo-Mercator** (`projstring.rs`, `ProjectedCRS::export_with`
   et `pseudo_mercator_proj`). Un CRS de méthode 1024 s'exporte comme PROJ 9.9 :
   `+proj=merc +a=R +b=R +lat_ts=0 +lon_0 +x_0 +y_0 +k=1 +units +nadgrids=@null`.
   Avant, il sortait `+proj=webmerc`. L'export de la **conversion** seule
   (`buildop.rs`, `+proj=webmerc`) n'a pas changé : `operation_webmerc_export_conversion`
   le teste toujours.
8. **Export géocentrique lié** (`projstring.rs`, `GeocentricCRS::export_with`).
   Un CRS géocentrique lié à WGS 84 s'exporte en `+proj=geocent +ellps +towgs84
   +units=m`. Avant, l'export refusait ce cas (`a bound geocentric CRS has no
   PROJ string in this build`).
9. **Équivalence du CRS projeté** (`equiv.rs`, `ProjectedCRS::is_equivalent_to_with`).
   Ajout d'une méthode publique qui prend un critère, comme `GeodeticCRS`.
   `Strict` garde le comportement de `is_equivalent_to`, à l'identique. Pour
   les autres critères, le nom n'est plus comparé, et les axes ne le sont que
   par direction et unité. C'est le critère `EQUIVALENT` de PROJ, utilisé par
   `merc_google_mercator_non_metre_unit`. Le test ne peut pas utiliser
   `is_equivalent_to` : le CS de base lu depuis le WKT n'a pas les mêmes
   identifiants que `EPSG:4326` au sens strict.
   **Ajout d'API publique.** À relire avec le mainteneur.
9b. **Module nouveau** : `src/iso19111/proj_crs.rs` (titre, nappes, géocentrique,
   pseudo-Mercator). `src/iso19111/mod.rs` le déclare.

Les tests `tests/test_io_bound.rs` et `tests/test_io.rs` passent avec ces
changements (section 7).

## 4. Limite des erreurs (`longlat_errors`, `projected_errors`, `errors`)

Ces trois tests vérifient seulement que la chaîne est refusée, comme
`EXPECT_THROW`. Sept des chaînes refusées sont des `+proj=pipeline` (3 dans
`longlat_errors`, 1 dans `projected_errors`, 3 dans `errors`). Le port les
refuse parce que les pipelines comme CRS ne sont pas importés (`unsupported
+proj=pipeline for a CRS`), et non pour la raison de PROJ. Pour ces sept cas,
la vérification est donc plus faible que celle de PROJ. Les autres chaînes
(`+datum=unknown`, `+ellps=unknown`, `+a=invalid`, `+f=0`, `+axis=foo`,
`+units=foo`, `+x_0=foo`, …) sont refusées par la validation propre au port.

Le cas `+=x;proj=pipeline step proj=push +type=crs` n'a pas d'attente dans
PROJ : il doit seulement revenir. Le port revient aussi.

## 5. Ledger et compteurs

- `docs/UNPORTED-UNIT.md` : 18 lignes retirées de `## test_io.cpp`. Le total
  ouvert passe de 924 à 906, et `test_io` de 328 à 310.
- `docs/UNIT-TRIAGE.md` et `docs/progress.svg` : régénérés par
  `UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg`.
- `README.md` et `ROADMAP.md` : **non modifiés**, comme demandé. Leurs compteurs
  restent ceux de la base. Le test `counts_in_docs_match_the_ledgers` échoue
  donc (« README.md is out of date with the ledgers »). Les valeurs à
  régénérer sont : `test_io.cpp` 126 → 144 portés, 328 → 310 ouverts ; total
  unités 924 → 906 ouverts, 428 → 446 portés. Le mainteneur les reporte
  lors de la fusion.

## 6. Vérification

Commandes lancées dans le worktree `agent-affbc38c2046f1dde`.

```
cargo test --test test_io_units                           # ok, 18 passed, 0 failed
cargo test --all-features --no-fail-fast                  # voir la section 7
cargo clippy --all-targets --all-features -- -D warnings  # ok, aucune sortie
cargo fmt --check                                         # ok
UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg   # ok, ledger et tri régénérés
git checkout README.md ROADMAP.md                         # restaurés
```

`cli_yaml` n'a réécrit `docs/UNPORTED-CLI.md` dans aucun des lancements : aucun
cas CLI ne change.

## 7. Suite complète

Avant le retrait des 18 lignes du ledger, `cargo test --all-features
--no-fail-fast` passait entièrement (code de sortie 0, 32 binaires avec
résultats). Après le retrait, la seule erreur restante est
`counts_in_docs_match_the_ledgers` (section 5), pour les compteurs de
`README.md`. Aucun autre test ne régresse.
