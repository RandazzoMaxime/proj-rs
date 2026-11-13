# Fiche : fermeture des tests `test_operationfactory.cpp` (lot opfactory-closing-c)

Source : PROJ 9.9.0, `test/unit/test_operationfactory.cpp`. Les valeurs attendues viennent de
`projinfo` 9.9.0 (binaire de référence, `PROJ_DATA` du build de référence) et du C++, jamais du
code testé.

## Bilan

| Test PROJ | Fichier Rust | État |
|---|---|---|
| `operation.boundCRS_to_geogCRS_hubCRS_and_targetCRS_same_but_baseCRS_not` | `tests/unit_op_vertical_bound_context.rs` | fermé, passe (plus d'`#[ignore]`) |
| `operation.compoundCRS_to_compoundCRS_concatenated_operation_with_two_vert_transformation` | `tests/unit_opfactory_closing_b.rs` | fermé, passe (plus d'`#[ignore]`) |

Lignes retirées de `docs/UNPORTED-UNIT.md` : les deux. `README.md`, `ROADMAP.md` et
`docs/UNIT-TRIAGE.md` ne sont pas régénérés (voir le compte rendu du lot).

## 1. `boundCRS_to_geogCRS_hubCRS_and_targetCRS_same_but_baseCRS_not`

Entrée : `COMPD_CS` NAD83 (GEOGCS avec `TOWGS84[0,0,0,0,0,0,0]`) + `VERT_CS` « Ellipsoid (US Feet) »
(datum 2002). Cible : EPSG:4979.

Sortie attendue (`projinfo -s <WKT> -t EPSG:4979 -o PROJ --grid-check none --spatial-test intersects`) :
un seul candidat, `+proj=unitconvert +xy_in=deg +z_in=us-ft +xy_out=deg +z_out=m`.

Avant le correctif, la crate donnait `+proj=unitconvert +z_in=us-ft +z_out=m`.

### Cause

PROJ, à l'import d'un `COMPD_CS` dont le vertical est une hauteur ellipsoïdale (datum 2002), appelle
`CompoundCRS::createLax` (crs.cpp:5396-5460), qui fait `components[0]->promoteTo3D(name, db, axe)`.
Pour un `BoundCRS` avec TOWGS84, `BoundCRS::promoteTo3D` (crs.cpp:1479-1487) promeut la base, le hub
(`hubCRS()->promoteTo3D`) et la transformation (`transf->promoteTo3D`). Le hub WGS 84 devient donc
EPSG:4979 à 3 axes. PROJ écrit ainsi `ABRIDGEDTRANSFORMATION` vers `WGS 84` 3D.

Le port ne faisait que la moitié : dans `wkt_import.rs::lax_compd_cs`, la branche 2002 ajoutait
l'axe de hauteur à la base (`add_height_axis`) mais laissait le hub à `EPSG:4326` (2D). Avec un hub 2D,
le candidat n'a pas de paire `z_in`/`z_out` sur l'étape NAD83 vers WGS 84 ; à l'export,
`pipeline.rs` annule alors les paires `xy` que PROJ fusionne (io.cpp, merge `xyz` + `xyz`). Le
correctif porte sur l'import, pas sur l'export : la sortie de `pipeline.rs` est inchangée.

### Correctif

- `src/iso19111/crs_ops.rs` : `promote_bound_hub_to_3d(crs, db)`, `pub(super)`. Pour un `BoundCRS`
  dont la transformation a des paramètres TOWGS84, promeut le hub et la transformation (même logique
  que `promote_bound`). Les autres liaisons ne sont pas touchées.
- `src/iso19111/wkt_import.rs` : `lax_compd_cs` reçoit la base de données et appelle ce helper après
  `add_height_axis` dans la branche 2002. Le `db` sert à identifier `EPSG:4979` comme dans PROJ.

Le nom du `BoundCRS` source (« NAD83 (Ellipsoid (US Feet)) ») et le `Promoted to 3D from` restent
ceux de la crate ; le test ne les vérifie pas.

## 2. `compoundCRS_to_compoundCRS_concatenated_operation_with_two_vert_transformation`

Le test est compilé hors de PROJ 9.9.0 (`#if no_longer_work_since_epsg_13_001`), la fiche
`docs/PORTING-opfac-closing-b.md` section 4 le décrit déjà. Le port reprend le comportement de 9.9.0 :
quatre candidats pour EPSG:8360 vers EPSG:7423, grilles ignorées, `PartialIntersection`.

Le test `operation_compound_to_compound_concatenated_two_vert_transformation_names` (ex-ignoré) vérifie le
nom du deuxième candidat et celui de son inverse, pour lesquels la fiche précédente notait l'écart
`Inverse of 'Inverse of E'` (crate) contre `'E'` (PROJ 9.9.0).

Constat sur la base fusionnée (`7e6f616`, avant tout changement de ce lot) : le test passe sans
modification. Les deux noms sont identiques à `projinfo` 9.9.0 :

- candidat 2 : `Inverse of 'ETRS89-SVK [SKTRF09] to Baltic 1957 height (...)' + 'ETRS89-SVK [SKTRF09] to EVRF2007 height (...)'` ;
- `inverse()` : `Inverse of 'ETRS89-SVK [SKTRF09] to EVRF2007 height (...)' + 'ETRS89-SVK [SKTRF09] to Baltic 1957 height (...)'`,
  qui est le candidat 2 de `projinfo -s EPSG:7423 -t EPSG:8360`.

Je n'ai pas identifié le commit qui a corrigé l'écart d'orientation ; je n'ai rien modifié pour lui.
Le test n'a été dé-ignoré qu'après vérification que sa sortie est celle de PROJ 9.9.0.

## Commandes de vérification

```bash
cargo test --test unit_op_vertical_bound_context -- --include-ignored
cargo test --test unit_opfactory_closing_b -- --include-ignored
```

Test temporaire utilisé pour lire la sortie de la crate (supprimé ensuite) : import du `COMPD_CS`, export
WKT2:2019 de la source, et `export_to_proj_string_with_database` des candidats.
