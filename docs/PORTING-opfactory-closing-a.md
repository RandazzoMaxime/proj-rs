# Fiche : fermeture des tests `test_operationfactory.cpp` (lot opfactory-closing-a)

Source : PROJ 9.9.0, `test/unit/test_operationfactory.cpp`. Le comportement de référence est celui de
`libproj.so` 9.9.0 compilé depuis le tag, appelé par son API C++ (`createOperations`, `name()`,
`inverse()`, `exportToPROJString`), et non celui du code testé. Les valeurs écrites dans les tests
Rust viennent de ces mesures ou des littéraux du fichier C++.

## Bilan

| Test PROJ | Fichier Rust | État |
|---|---|---|
| `operation.geog3DCRS_to_geog2DCRS_plus_vertCRS_context` (dans `#if no_longer_work_since_epsg_13_001`, non compilé en 9.9.0) | `tests/unit_op_vertical_bound_context.rs` | fermé, passe |
| `operation.nadgrids_with_pm` (EPSG:4326, EPSG:4258, round trip WKT2_2019 vers 4258) | `tests/unit_op_nadgrids_with_pm.rs` | fermé, passe |
| `operation.boundCRS_to_geogCRS_hubCRS_and_targetCRS_same_but_baseCRS_not` | `tests/unit_op_vertical_bound_context.rs` | ouvert, `#[ignore]` (cause ci-dessous) |

Ligne retirée de `docs/UNPORTED-UNIT.md` : les deux premiers. Le troisième reste.

## 1. `geog3DCRS_to_geog2DCRS_plus_vertCRS_context`

Le test C++ est désactivé en 9.9.0 : ses noms (`ETRS89 to Baltic 1957 height (2)`) sont ceux d'anciennes
versions d'EPSG. Les valeurs mesurées en 9.9.0 (4937 vers 8360, contexte « toutes autorités »,
`PARTIAL_INTERSECTION`, grilles non filtrées) :

- candidat 0 : `ETRS89-CZE [2007] to Baltic 1957 height (2)`, inverse `Inverse of ETRS89-CZE [2007] to Baltic 1957 height (2)` ;
- candidat 1 : `'ETRS89-SVK [SKTRF09] to Baltic 1957 height (from ETRS89-SVK [SKTRF09] to ETRS89-SVK [SKTRF09] + Baltic 1957 height (1))'`
  (guillemets inclus), inverse `Inverse of '...'`.

Cause du nom faux du candidat 1 (`Inverse of 'Inverse of '...''`) : la sortie de `PROJBasedOperation`
(`createHorizVerticalHorizPROJBased`) porte un nom calculé à partir des étapes (`computed_name`). Son
`inverse()` le reconstruisait donc à partir des étapes inversées. PROJ n'agit pas ainsi : son
`PROJBasedOperation::inverse()` applique toujours `createPropertiesForInverse` au nom (oputils.cpp:231-316).

Correction : `ConcatenatedOperation::inverse` (`src/iso19111/buildop.rs`) traite d'abord le cas
`proj_based`. Le nom du `computed_name` ne sert plus qu'aux opérations sans `proj_based`.

## 2. `nadgrids_with_pm`

Le test EPSG:4326 et le test EPSG:4258 passaient déjà ; le round trip WKT2_2019 manquait (il n'était
pas écrit, et non ignoré). Le round trip donnait un pas `+step +proj=longlat +ellps=intl +pm=lisbon` en trop.

Cause : `BOUNDCRS` lu depuis WKT avec un `ABRIDGEDTRANSFORMATION` sans `SOURCECRS` prenait pour source de
la transformation la base géographique elle-même (méridien de Lisbonne). PROJ construit cette source par
`createBoundCRSSourceTransformationCRS` (crs.cpp:5989-6010) : une copie sur le méridien de Greenwich,
nommée `... (with Greenwich prime meridian)`, quand le méridien de la base n'est pas Greenwich
(`buildTransformationForBoundCRS`, io.cpp:5361-5376).

Avec la mauvaise source, `intermediate_operation` (`bound_ops.rs`) jugeait la base et la source
équivalentes, n'ajoutait pas l'opération Lisbonne vers Greenwich (`inv longlat +pm=lisbon`), qui aurait
annulé le `longlat +pm=lisbon` de la première jambe.

Correction : `bound_import::transformation_source_of` (même logique que `createBoundCRSSourceTransformationCRS`,
réutilisée par `bind_nadgrids`), appliquée dans `wkt_import.rs::build_bound` quand la transformation abrégée
n'a pas de source.

## 3. `boundCRS_to_geogCRS_hubCRS_and_targetCRS_same_but_baseCRS_not` (ouvert)

Attendu : `+proj=unitconvert +xy_in=deg +z_in=us-ft +xy_out=deg +z_out=m`. Obtenu :
`+proj=unitconvert +z_in=us-ft +z_out=m`. Les noms des candidats sont identiques ; seule l'écriture diffère.

Mesures (9.9.0) :

- le candidat `bound -> 4979` a pour cible `WGS 84` à 3 axes, et la transformation `Transformation from NAD83 to WGS84`
  s'écrit `+proj=noop` seule. Dans la concaténation, PROJ écrit ses étapes `push/cart/helmert/cart/pop` avec les
  paires `unitconvert` `z_in=m z_out=m` ;
- le port fixe la cible de cette transformation à `EPSG:4326` (2D) dans `bound_import.rs::bind_towgs84`, et
  n'écrit donc pas les paires `z`. Dans l'export, le simplificateur (`pipeline.rs`) annule alors les paires `xy`
  alors que PROJ les fusionne (`xy_in=deg ... xy_out=deg`).

Ce qui reste à faire : que l'import COMPD_CS (base 2D, hauteur ellipsoïdale) donne une cible 3D comme PROJ,
puis vérifier l'ordre de fusion de `unitconvert` dans `PipelineFormatter::merge_adjacent` contre
`PROJStringFormatter::toString` (io.cpp:8877-9400). Non traité dans ce lot : ce sont deux sujets
distincts (import des BOUNDCRS et simplification), à ne pas mélanger.

## Commandes de vérification

```bash
cargo test --test unit_op_vertical_bound_context
cargo test --test unit_op_nadgrids_with_pm
```
