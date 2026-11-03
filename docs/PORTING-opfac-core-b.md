# Port — `opfac-core-b`

PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, lot `opfac-b` : les 51 noms de `TEST()` listés dans
`/tmp/port-lots/opfac-b.txt`, du `transformation_height_to_PROJ_string` au
`compoundCRS_to_geogCRS_with_vertical_unit_change`. Source : `test/unit/test_operationfactory.cpp` du tag
9.9.0 (téléchargé en `raw.githubusercontent.com`).

Les tests portés sont dans `tests/unit_opfac_core_b.rs`. Chaque fonction garde le nom C++ en snake case.
Les valeurs attendues (noms d'opérations, chaînes PROJ, nombres) sont copiées du C++.

**Bilan : 2 cas portés et verts, 49 cas ouverts.** Les cas ouverts sont listés plus bas, avec une raison
par cas.

## Tests portés (2)

| C++ `TEST` | Fonction Rust |
|---|---|
| `compoundCRS_with_bound_of_projected_and_bound_of_vertical_to_geog3D` | `operation_compound_crs_with_bound_of_projected_and_bound_of_vertical_to_geog3_d` |
| `compoundCRS_to_geogCRS_3D_with_same_geog_src_target_interp_context` | `operation_compound_crs_to_geog_crs_3_d_with_same_geog_src_target_interp_context` |

Aucune modification de `src/` : les deux cas passent avec l'engine actuel.

## Cas non portés (49)

Ouverts sans test dans le fichier (raison d'API ou de compilation) :

| C++ `TEST` | Raison |
|---|---|
| `transformation_height_to_PROJ_string` | `Transformation::gridsNeeded` n'existe pas dans l'API Rust. |
| `compoundCRS_to_compoundCRS_concatenated_operation_with_two_vert_transformation` | Code désactivé dans PROJ 9.9.0 (`#if no_longer_work_since_epsg_13_001`) : rien à caractériser. |
| `compoundCRS_to_compoundCRS_using_intermediate_of_horizontal_transform` | `GridAvailabilityUse::KnownAvailable` n'existe pas (seuls `UseForSorting` et `IgnoreGridAvailability`). |
| `compoundCRS_to_geogCRS_3D_KNOWN_AVAILABLE_context` | Même cause : `GridAvailabilityUse::KnownAvailable` absent. |

Échecs à l'exécution (test rédigé, puis retiré du fichier pour garder la suite verte) :

| C++ `TEST` | Raison (observé) |
|---|---|
| `compoundCRS_with_boundProjCRS_and_boundVerticalCRS_to_geogCRS` | Chaîne : `+inv +proj=longlat +pm=paris` en trop, `pm` non rattaché à l'étape `utm` (écart déjà noté dans `PORTING-opfactory-core-b.md`). |
| `compoundCRS_with_boundProjCRS_with_ftus_and_boundVerticalCRS_to_geogCRS` | Le simplificateur de pipeline fusionne `unitconvert xy_in=deg` après `inv tmerc` (sortie en radians), chaîne différente de PROJ (`src/iso19111/pipeline.rs`). |
| `compoundCRS_with_boundGeogCRS_and_geoid_to_geodCRS_NAD2011_ctxt` | L'opération « Inverse of unknown + … geocentric » n'est pas produite par `createOperations`. |
| `compoundCRS_with_vert_bound_to_bound_geog3D` | La grille `egm96_15.gtx` disparaît de l'opération bound → bound : il ne reste que l'axisswap. |
| `geocent_to_compoundCRS` | Aucune opération géocentrique → `+proj=longlat` avec `+nadgrids`/`+geoidgrids`/`+geoid_crs`. |
| `geocent_to_compoundCRS_context` | Contexte EPSG : 0 opération au lieu de 1 pour EPSG:4978 vers la même cible. |
| `compoundCRS_to_compoundCRS` | Aucune opération UTM31+ODN → UTM32+ODN : un compound à vertical identique n'est pas construit (l'UTM seule fonctionne). |
| `compoundCRS_to_compoundCRS_with_vertical_transform` | Aucune opération : le `BoundCRS` VERTCON n'entre pas dans la composition du compound. |
| `compoundCRS_to_compoundCRS_with_bound_crs_in_horiz_and_vert` | Aucune opération longlat `+nadgrids`/`+geoidgrids` → longlat `+nadgrids`/`+geoidgrids`. |
| `compoundCRS_to_compoundCRS_with_bound_crs_in_horiz_and_vert_same_geoidgrids` | Aucune opération (même famille de `+nadgrids`/`+geoidgrids`). |
| `compoundCRS_to_compoundCRS_with_bound_crs_in_horiz_and_vert_same_geoidgrids_different_vunits` | Aucune opération (même famille, avec `+vunits=us-ft`). |
| `compoundCRS_to_compoundCRS_with_bound_crs_in_horiz_and_vert_same_nadgrids_same_geoidgrids` | Aucune opération ; `+proj=noop` attendu. |
| `compoundCRS_to_compoundCRS_with_bound_crs_in_horiz_and_vert_same_towgs84_same_geoidgrids` | Aucune opération avec `+towgs84=0,0,0` et `+geoidgrids`. |
| `compoundCRS_to_compoundCRS_with_bound_crs_in_horiz_and_vert_same_ellsp_but_different_towgs84_different_geoidgrids` | Aucune opération avec des `+towgs84` différents. |
| `compoundCRS_to_compoundCRS_with_bound_crs_in_horiz_and_vert_WKT1_same_geoidgrids_context` | 0 opération au lieu de 1 (COMPD_CS WKT1, TOWGS84 et grille NAVD88). |
| `compoundCRS_to_compoundCRS_issue_2232` | Aucune opération entre les deux COMPD_CS. |
| `compoundCRS_to_compoundCRS_context` | La liste EPSG 7406 → 5500 n'atteint pas 152 opérations. |
| `compoundCRS_to_compoundCRS_context_helmert_noop` | Aucune opération EPSG:4326+5773 → EPSG:4258+5773 ; `+proj=noop` attendu. |
| `compoundCRS_to_compoundCRS_WGS84_EGM96_to_ETRS89_Belfast` | Aucune opération EPSG:4326+5773 → EPSG:4173+5732. |
| `compoundCRS_to_compoundCRS_WGS84_EGM96_to_WGS84_Belfast` | Renvoie le ballpark « EGM96 height to Belfast height » au lieu de la chaîne EPSG. |
| `compoundCRS_to_compoundCRS_OSGB36_BNG_ODN_height_to_WGS84_EGM96` | Aucune opération EPSG:27700+5701 → EPSG:9707. |
| `compoundCRS_to_compoundCRS_context_helmert` | Aucune opération EPSG:9464 → EPSG:9463. |
| `compoundCRS_to_compoundCRS_GDA94_AHD_to_GDA2020_AVWS` | Aucune opération EPSG:9464 → EPSG:9462 (composition verticale EPSG absente). |
| `compoundCRS_with_horizontal_boundCRS_to_compoundCRS_and_same_vertical` | Aucune opération AGD66 bound → EPSG:7855+5711. |
| `compoundCRS_to_compoundCRS_concatenated_operation_with_two_vert_transformation_and_different_source_dest_interp` | Aucune opération EPSG:4313+5710 → EPSG:4289+5709. |
| `compoundCRS_to_compoundCRS_issue_2720` | 0 opération au lieu de 1 (Orthographic + EGM96 WKT1 vers WGS 84 + EGM96). |
| `compoundCRS_to_compoundCRS_issue_3328` | Renvoie le ballpark « EGM96 height to CGVD28 height » au lieu de la chaîne `push +v_1 +v_2` EPSG. |
| `compoundCRS_to_compoundCRS_WGS84_EGM2008_to_RD_new_NAP_height` | 0 opération au lieu de 4 (EPSG:9518 → EPSG:7415). |
| `compoundCRS_to_compoundCRS_concatenated_operation_with_two_vert_transformation_and_ballpark_geog` | Renvoie un ballpark « CGVD28 → CGVD2013 » au lieu de la chaîne NAD83(CSRS)v6. |
| `compoundCRS_to_compoundCRS_issue_3152_ch1903lv03_ln02_bound` | Aucune opération (BoundCRS Helmert CH1903 vers EGM2008, LN02). |
| `compoundCRS_to_compoundCRS_issue_3191_BD72_Ostend_height_to_WGS84_EGM96` | Aucune opération EPSG:4313+5710 → EPSG:9707. |
| `compoundCRS_to_compoundCRS_issue_4550` | Aucune opération (NAD83(HARN) ftUS + NAVD88 vers UTM 10N + NAVD88). |
| `vertCRS_to_vertCRS_context` | 3 opérations au lieu de 4 (NGVD29 → NAVD88). |
| `vertCRS_to_vertCRS_New_Zealand_context` | 1 opération au lieu de 2 (NZVD2016 → Auckland 1946). |
| `vertCRS_to_vertCRS_pivot_context` | 5705 → 5706 renvoie un ballpark au lieu de `geogoffset +dh=28` + axisswap. |
| `compoundCRS_to_geogCRS_3D_context` | Première opération « Ballpark geographic offset from NAD27 » au lieu de « NAD27 to WGS 84 (79) + … ». |
| `compoundCRS_to_geogCRS_3D_with_3D_helmert_context` | Ballpark EGM96 au lieu de la chaîne EGM96 + Helmert CH1903+. |
| `compoundCRS_to_geogCRS_3D_with_3D_helmert_same_geog_src_target_context` | `+proj=noop` au lieu du push/pop `v_1 v_2` et `vgridshift` EGM96. |
| `compoundCRS_to_geogCRS_3D_with_null_helmert_same_geog_src_target_context` | `+proj=noop` au lieu du `vgridshift` EGM96 (EPSG:4258+5773 → EPSG:4937). |
| `compoundCRS_to_geogCRS_3D_WGS84_to_GDA2020_AHD_Height` | Ballpark « AHD height to WGS 84 » au lieu de « GDA2020 to AHD height (1) + GDA2020 to WGS 84 (2) ». |
| `compoundCRS_to_geogCRS_2D_promote_to_3D_context` | 1 opération au lieu de 133 (EPSG:4269+5703 → NAD83 3D). |
| `compoundCRS_of_projCRS_to_geogCRS_3D_context` | 1 opération au lieu de 16 (SPCS83 Californie zone 1 + NAVD88 → NAD83 3D). |
| `compoundCRS_from_wkt_without_id_to_geogCRS` | 1 opération au lieu de 3 : le compound WKT sans identifiant ne donne pas les opérations d'EPSG:6318+5703. |
| `compoundCRS_of_projCRS_from_wkt_without_id_or_extent_to_geogCRS` | Le compound WKT sans identifiant n'est pas équivalent à EPSG:32129+5703 (`is_equivalent_to` faux). |
| `compoundCRS_to_geogCRS_with_vertical_unit_change` | 1 opération au lieu de 3 : la liste ftUS n'a pas la longueur de la liste mètres. |

Les 49 cas ouverts se répartissent en deux familles. Quatre n'ont pas de test : une API absente, un code
désactivé dans 9.9.0. Les 45 autres ont un test rédigé qui échoue. Ces échecs viennent surtout de la
composition des compounds avec les opérations EPSG verticales (listes vides ou ballpark à la place de la
chaîne enregistrée).

## Écarts volontaires

- Les textes WKT du C++ sont passés à `create_from_user_input` (au lieu de `WKTParser`), comme dans les
  autres lots.
- `EXPECT_*` non bloquants : traduits en `assert!`. Aucun cas n'est affaibli.
- `PROJStringFormatter::create(Convention::PROJ_5, db)` : `export_to_proj_string` ne prend pas de base
  de données, la sortie est identique pour les deux cas portés.

## Défauts d'engine relevés (non corrigés dans ce lot)

- `src/iso19111/pipeline.rs` : la fusion des `unitconvert` produit `+xy_in=deg +z_in=us-ft +xy_out=rad
  +z_out=m` juste après `+inv +proj=tmerc` (sortie en radians). PROJ garde une étape `z`-seule. La
  conversion `xy_in=deg` sur des radians semble fausse ; non vérifiée numériquement.
- Compound → compound avec vertical registré : `createOperations` ne compose pas la partie verticale
  EPSG (`compoundCRS_to_compoundCRS*`, `compoundCRS_to_geogCRS_3D*`).

## Commandes et résultats

- `cargo fmt --check` : OK (sortie vide, code 0).
- `cargo clippy --all-targets --all-features -- -D warnings` : OK (code 0).
- `PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast` : code 0,
  1343 passés, 0 échec, 26 ignorés (préexistants dans d'autres fichiers, aucun dans
  `unit_opfac_core_b`). `docs_in_sync` : 24 passés.
- `cargo test --test unit_opfac_core_b` : 2 passés.
- Brouillon des 47 cas (hors dépôt, `/tmp/unit_opfac_core_b_draft.rs`) : 2 passés, 45 échoués, comme ci-dessus.
- `docs/UNPORTED-UNIT.md`, `docs/UNIT-TRIAGE.md`, `README.md`, `ROADMAP.md` : non modifiés (consigne).
  Les deux cas portés restent listés dans `docs/UNPORTED-UNIT.md` : à retirer par le mainteneur.
- Étape 6 de CONTRIBUTING (bench) : non faite. `bench/record.sh` mesure le débit des transformations
  par point, sans cas correspondant aux tests de fabrique ; une ligne d'historique exige une machine de
  référence.
