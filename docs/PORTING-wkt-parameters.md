# Port : paramètres et identifiants WKT (`tests/test_io_wkt2.rs`, six `#[ignore]`)

Référence : PROJ 9.9.0, `test/unit/test_io.cpp` (groupe `wkt_parse`) et
`src/iso19111/io.cpp`, `operation/oputils.cpp`, `operation/parammappings.cpp`.
Les valeurs attendues sont celles du C++ et des tests Rust existants ; aucune
assertion n'a été touchée.

## Bilan

Six tests étaient marqués `#[ignore = "open: ..."]`. Sur la base de départ
(`ead2c4c`), ils passent tous tels quels : la raison écrite dans l'ignore
était périmée. Aucun changement de `src/` n'a été nécessaire.

Un septième cas, `wkt_parse_boundcrs_with_interpolation_as_parameter`, échouait
pour une raison différente : l'entrée WKT du test n'était pas celle de PROJ
(voir plus bas). Son entrée a été corrigée ; l'import était déjà conforme.

| Test (`tests/test_io_wkt2.rs`) | Ligne | Raison d'origine (ignore) | Constat |
|---|---|---|---|
| `wkt_parse_wkt2_spherical_planetocentric` | 227 | version d'ID repliée dans l'autorité (`IAU_2015`) | passe : sans base, `ID["IAU",19902,2015]` est gardé (`build_id`, version conservée si `versioned_authority` ne résout rien) |
| `wkt_parse_wkt2_2019_projected_3d` | 603 | `Longitude of origin` sans ID non lu, `lon_0` = 0 | passe : retrouvé par nom équivalent (`are_equivalent_parameters`, `buildop.rs`), comme `getMapping` / `areEquivalentParameters` |
| `wkt_parse_wkt2_2019_projected_utm_3d` | 616 | idem + zone UTM non reconnue | passe : `+proj=utm +zone=31` |
| `wkt_parse_wkt2_2019_eqdc_non_epsg` | 680 | codes 8801, 8802, 8806, 8807 lus comme 0 (Equidistant Conic) | passe : les codes EPSG des paramètres sont lus depuis l'entrée |
| `wkt_parse_wkt2_2019_projected_with_base_geocentric` | 629 | `BASEGEODCRS` EPSG:4978 importé géographique | passe avec `parse_db` : la CRS de base est géocentrique |
| `wkt_parse_boundcrs_with_interpolation_as_parameter` | 1322 | « EPSG code for Interpolation CRS » perdu, un paramètre au lieu de deux sans base | l'import est conforme ; l'entrée du test était fausse (voir ci-dessous) |

## Cause du test d'interpolation

PROJ lit une `BOUNDCRS` ainsi (`io.cpp`, `buildBoundCRS`, l. 5405) :
`consumeParameters(abridgedNode, ...)` ne lit que les enfants de
`ABRIDGEDTRANSFORMATION`. `dealWithEPSGCodeForInterpolationCRSParameter`
(l. 3684) ne retire le paramètre que si une base est attachée.

Dans `test_io.cpp` (l. 5299), `PARAMETER["EPSG code for Interpolation CRS",7886]`
est enfant de `ABRIDGEDTRANSFORMATION`, après `PARAMETERFILE` (`...]],`). Dans
la copie Rust, `ID["EPSG",8666]]],` fermait la transformation une parenthèse
trop tôt : le `PARAMETER` remontait sous `BOUNDCRS`, où PROJ ne le lit pas. Le
parseur Rust ignorait donc bien ce paramètre, et n'en gardait qu'un.

Correctif : entrée du test remise à la structure de PROJ (`ID["EPSG",8666]],`
puis `PARAMETER[...]]]`). Les assertions sont inchangées.

Avec cette entrée, sans base : interpolation `None`, 2 paramètres ; avec base :
interpolation `Some`, 1 paramètre, et l'export contient
`PARAMETER["EPSG code for Interpolation CRS",7886,`. Conforme à PROJ.

## Écarts

Aucun écart introduit. Le test d'interpolation ne couvre que l'entrée
corrigée ; le cas « paramètre sous `BOUNDCRS` » n'est pas un cas PROJ.

## Commandes

```bash
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
PATH=/home/user/proj-rs/target/proj-reference/build/bin:$PATH \
PROJ_DATA=/home/user/proj-rs/target/proj-reference/build/data \
  cargo test --test test_io_wkt2 -- --ignored      # avant : 20 ok, 1 FAILED (interpolation)
PATH=... PROJ_DATA=... cargo test --test test_io_wkt2   # après : 26 passed, 15 ignored
```
