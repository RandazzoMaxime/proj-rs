# Port : operationfactory, lot A (`operation`)

Source : PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, les 51 `TEST(operation, ...)`
listés dans `/tmp/port-lots/opfac-a.txt`. Fichier de test : `tests/unit_opfac_core_a.rs`.
Aucune modification de `src/`.

## Bilan

- Portés et verts : 4 / 51.
- Ouverts : 47 / 51. Une raison par cas ci-dessous.

Le lot est presque entièrement ouvert : la recherche d'opérations de la factory
(chaînes par datums intermédiaires, BoundCRS, grilles verticales, sens et ordre des
étapes) ne reproduit pas PROJ 9.9.0 sur ces cas. Les cas ouverts n'ont pas été
affaiblis ni passés en `#[ignore]`.

## Cas portés

| TEST C++ | Fonction Rust |
|---|---|
| `operation.projCRS_3D_to_geogCRS_3D_context` | `operation_proj_crs_3d_to_geog_crs_3d_context` |
| `operation.boundCRS_of_geogCRS_to_unrelated_geogCRS_context` | `operation_bound_crs_of_geog_crs_to_unrelated_geog_crs_context` |
| `operation.geogCRS_to_boundCRS_of_geogCRS` | `operation_geog_crs_to_bound_crs_of_geog_crs` |
| `operation.boundCRS_to_geogCRS_same_datum_context` | `operation_bound_crs_to_geog_crs_same_datum_context` |

## Cas ouverts

| TEST C++ | Raison |
|---|---|
| `operation.geogCRS_to_geogCRS_context_match_by_name` | Copie de NAD27 sans identifiant : 36 opérations au lieu de 79 ; pas de correspondance par nom. |
| `operation.geogCRS_to_geogCRS_context_inverse_needed` | `setUsePROJAlternativeGridNames` absent du contexte ; sans ce réglage, 2 opérations au lieu de 3. |
| `operation.geogCRS_to_geogCRS_context_NAD27_to_WGS84_G1762` | Moins de 78 opérations NAD27 vers WGS 84 (G1762). |
| `operation.geogCRS_to_geogCRS_context_datum_ensemble` | Ensemble WGS 84 non reconnu : « Ballpark geographic offset » au lieu de « ETRS89 to WGS 84 (1) ». |
| `operation.vertCRS_to_geogCRS_context` | 2 opérations au lieu de 3 pour EGM2008 vers WGS 84 ; réglage des noms de grilles absent. |
| `operation.geog3DCRS_to_geog2DCRS_plus_vertCRS_context` | Bloc C++ désactivé par `#if no_longer_work_since_epsg_13_001` dans PROJ 9.9.0 : aucune assertion active. |
| `operation.geog3DCRS_to_vertCRS_depth_context` | Liste vide pour ETRS89 3D vers CD Norway depth (chaîne vgridshift absente). |
| `operation.geog3DCRS_to_geog2DCRS_plus_vertCRS_depth_context` | Rust choisit `+inv +proj=affine +s33=-1` au lieu du pipeline vgridshift. |
| `operation.geogCRS_to_geogCRS_context_lonlat_vs_latlon_crs` | 3 opérations au lieu de 5 pour RGNC91-93 vers RGNC15. |
| `operation.geogCRS_to_geogCRS_context_concatenated_operation` | Le second candidat NTF porte `+inv +proj=hgridshift` au lieu de `+proj=hgridshift`. |
| `operation.geogCRS_to_geogCRS_context_ED50_to_WGS72_no_NTF_intermediate` | Moins de 2 opérations ED50 vers WGS 72. |
| `operation.geogCRS_to_geogCRS_CH1903_to_CH1903plus_context` | 2 opérations au lieu de 1 pour CH1903 vers CH1903+. |
| `operation.geogCRS_to_geogCRS_init_IGNF_to_init_IGNF_context` | `+init=IGNF:NTFG` refusé par `create_crs_from_proj_string_with_database` (erreur de création). |
| `operation.geogCRS_without_id_to_geogCRS_3D_context` | Opérations de la CRS sans identifiant non équivalentes à celles de la WKT2 équivalente. |
| `operation.geogCRS_promoted_to_3D_to_geogCRS_3D_context` | NAD83 vers NAD83(2011) en un saut au lieu de la chaîne HARN / FBN / NSRS2007. |
| `operation.geogCRS_3D_to_geogCRS_3D_different_datum_context` | « Ballpark geographic offset » au lieu de la chaîne ETRS89 vers CH1903+. |
| `operation.geocentric_to_geogCRS_3D_different_datum_context` | Chaîne différente : conversion et « Ballpark » au lieu de la chaîne ETRS89 vers CH1903+. |
| `operation.createBetweenGeodeticCRSWithDatumBasedIntermediates` | « Ballpark geographic offset » au lieu de la chaîne par datums intermédiaires ITRF2008 / ITRF2014. |
| `operation.esri_projectedCRS_to_geogCRS_with_ITRF_intermediate_context` | « Ballpark » au lieu de la chaîne ITRF2000 vers ITRF2005 (projection ESRI). |
| `operation.geogCRS_to_geogCRS_with_intermediate_no_ids` | « Ballpark geographic offset » au lieu de la chaîne ITRF2014, ETRF2014, NKG, EST97. |
| `operation.geogCRS_3D_source_datum_name_is_alias_to_geogCRS` | `+proj=noop` correct, mais le nom « Null geographic offset » devient « Ballpark geographic offset ». |
| `operation.geogCRS_with_towgs84_to_geocentric` | 0 opération de GEOGCS TOWGS84 (3D) vers GEOCCS WGS 84 au lieu de 1. |
| `operation.geogCRS_different_from_baseCRS_to_projCRS_context_compatible_area` | 1 opération au lieu de 4 pour NTF (Paris) vers UTM 31N. |
| `operation.projCRS_no_id_to_geogCRS_context` | Opérations de la projetée sans identifiant non équivalentes à celles de la WKT2 équivalente. |
| `operation.geogCRS_3D_to_projCRS_with_2D_geocentric_translation` | `unitconvert +z_in=m +z_out=m` avant `push +v_3` non émis. |
| `operation.projCRS_to_projCRS_different_baseCRS` | Étape `+proj=longlat +ellps=clrk80ign +pm=paris` ajoutée au lieu de `+pm=paris` sur l'UTM 32. |
| `operation.projCRS_to_projCRS_context_grid_offsets` | 1 opération au lieu de 2 pour Karbala 1979 vers IGRS (décalage de grille). |
| `operation.projCRS_to_projCRS_context_grid_offsets_non_metre_unit_noop` | Moins de 2 opérations pour NAD83(2011) vers NAD83(HARN) Jackson (ftUS). |
| `operation.projCRS_to_geogCRS_concatenated_operation_extent_none` | `setDiscardSuperseded` absent ; sans étendue, aucune opération ED87 n'est produite. |
| `operation.boundCRS_of_geogCRS_to_geodCRS` | Erreur de création d'opération pour BoundCRS NTF vers géocentrique WGS 84. |
| `operation.boundCRS_of_geogCRS_to_geodCRS_not_related_to_hub` | 0 opération au lieu de 1 pour BoundCRS vers ETRS89 géocentrique. |
| `operation.createOperation_boundCRS_identified_by_datum` | Assertion du compteur de requêtes SQL (`getQueryCounter`) sans équivalent exposé ; cas gardé ouvert. |
| `operation.boundCRS_of_clrk_66_geogCRS_to_nad83_geogCRS` | Assertion du compteur de requêtes SQL (`getQueryCounter() + 2`) sans équivalent exposé ; cas gardé ouvert. |
| `operation.boundCRS_of_clrk_66_projCRS_to_nad83_geogCRS` | `hgridshift +grids=ntv1_can.dat,conus` absent de la sortie. |
| `operation.boundCRS_of_projCRS_to_geogCRS` | Étape géographique intermédiaire ajoutée (`+inv +proj=longlat +ellps=clrk80ign +pm=paris`). |
| `operation.boundCRS_to_geogCRS_hubCRS_and_targetCRS_same_but_baseCRS_not` | Sortie `+proj=unitconvert +z_in=us-ft +z_out=m` sans les étapes `xy_in=deg` / `xy_out=deg`. |
| `operation.boundCRS_to_boundCRS` | Erreur de création d'opération entre deux BoundCRS UTM (TOWGS84 à 7 paramètres). |
| `operation.boundCRS_to_boundCRS_noop_for_TOWGS84` | Erreur de création d'opération entre deux BoundCRS TOWGS84 identiques. |
| `operation.boundCRS_to_boundCRS_unralated_hub` | Erreur de création d'opération (hub sans rapport). |
| `operation.boundCRS_of_projCRS_towgs84_to_boundCRS_of_projCRS_nadgrids` | Erreur de création d'opération (UTM NAD83 TOWGS84 vers UTM NAD27 nadgrids). |
| `operation.boundCRS_of_projCRS_towgs84_non_metre_unit_to_geocentric` | Erreur de création d'opération (Mercator en pieds TOWGS84 vers géocentrique). |
| `operation.boundCRS_to_boundCRS_with_base_geog_crs_different_from_source_of_transf` | Erreur de création d'opération (LCC et longlat TOWGS84 via WKT2_2019). |
| `operation.boundCRS_with_basecrs_with_extent_to_geogCRS` | Erreur de création d'opération pour BOUNDCRS WKT avec ABRIDGEDTRANSFORMATION NAD83. |
| `operation.ETRS89_3D_to_proj_string_with_geoidgrids_nadgrids` | Sortie sans `vgridshift naptrans2008.gtx` ni les étapes `z` : le géoïde n'est pas appliqué. |
| `operation.nadgrids_with_pm` | Étape géographique intermédiaire ajoutée (`+inv +proj=longlat +ellps=intl +pm=lisbon`). |
| `operation.towgs84_pm_3d` | 0 opération de TMerc Bessel pm=ferro TOWGS84 (3D) vers longlat GRS80 (3D) au lieu de 1. |
| `operation.WGS84_G1762_to_compoundCRS_with_bound_vertCRS` | Ordre des étapes différent (`unitconvert` avant `axisswap`) et `axisswap` supplémentaire. |

## Écarts volontaires vers PROJ

- Compteur de requêtes SQL : `DatabaseContext::getQueryCounter` n'est pas exposé. Les
  deux cas qui l'assertent restent ouverts, l'assertion n'est pas retirée.
- API : `CoordinateOperationContext::create(db, authority)` avec des setters, à la place
  de `create(authFactory, extent, accuracy)`. `BoundCRS::from_towgs84` remplace
  `BoundCRS::createFromTOWGS84`. Les noms de fonctions Rust suivent les noms C++ en snake_case.
- Formatage : les chaînes PROJ sont produites par `export_to_proj_string_with_database`
  dans tous les cas. C++ utilise parfois un formatteur sans base ; la sortie est la même
  sur les 4 cas portés.

## Commandes lancées et résultats

- `curl -sSfL https://raw.githubusercontent.com/OSGeo/PROJ/9.9.0/test/unit/test_operationfactory.cpp`
  : 12490 lignes, 272 `TEST(`.
- `cargo test --all-features --test unit_opfac_core_a` sur un brouillon de 49 fonctions
  (tests des 51 cas, moins les 2 cas à compteur SQL et le bloc `#if` désactivé) : 5 verts,
  44 rouges. Le brouillon comptait un cas hors lot (retiré depuis).
- `cargo fmt --check` : OK.
- `cargo clippy --all-targets --all-features -- -D warnings` : OK.
- `PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast` : OK
  (voir la sortie finale dans la PR).

## Non fait

- `src/bin/bench.rs` et `bench/history.jsonl` (CONTRIBUTING, point 6) : non modifiés. Le lot
  ne porte pas de projection ni de pipeline numérique à chronométrer.
- `docs/UNPORTED-UNIT.md` : non modifié, à la demande. Les 4 cas portés y figurent encore ;
  `UPDATE_DOCS=1 cargo test --test docs_in_sync` reste à lancer par le mainteneur.
