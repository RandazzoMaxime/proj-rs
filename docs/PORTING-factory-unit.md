# Port — tests unitaires `test_factory.cpp` (lot 2)

Source : PROJ 9.9.0, `test/unit/test_factory.cpp`, et `src/iso19111/factory.cpp`,
`metadata.cpp` (`Identifier::isEquivalentName`), `crs.cpp`
(`SingleCRS::baseIsEquivalentTo`), `coordinatesystem.cpp`
(`CoordinateSystemAxis::_isEquivalentTo`), `datum.cpp` (`Ellipsoid`).

Ce lot porte 12 `TEST()` de `test_factory.cpp`. Il ne touche pas aux
conversions vers chaîne PROJ des transformations (`createCoordinateOperation`
suivi de `exportToPROJString`), ni à `buildop.rs` / `projstring.rs` pour
l'export des `Transformation`.

## Correspondance

| PROJ 9.9.0 `factory.*` | PROJ-RS `tests/test_factory.rs` |
|---|---|
| `getMetadata` | `factory_get_metadata` |
| `ogc_crs` | `factory_ogc_crs` |
| `AuthorityFactory_getDescriptionText` | `factory_authority_factory_get_description_text` |
| `AuthorityFactory_EPSG_4326_approximate_equivalent_to_builtin` | `factory_authority_factory_epsg_4326_approximate_equivalent_to_builtin` |
| `AuthorityFactory_test_uom_9110` | `factory_authority_factory_test_uom_9110` |
| `AuthorityFactory_getAvailableGeoidmodels` | `factory_authority_factory_get_available_geoidmodels` |
| `listAreaOfUseFromName` | `factory_list_area_of_use_from_name` |
| `getCRSInfoList` | `factory_get_crs_info_list` |
| `getUnitList` | `factory_get_unit_list` |
| `getCelestialBodyList` | `factory_get_celestial_body_list` |
| `toWGS84AutocorrectWrongValues` | `factory_to_wgs84_autocorrect_wrong_values` |
| `getOfficialNameFromAlias` | `factory_get_official_name_from_alias` |

Restent ouverts dans `docs/UNPORTED-UNIT.md` :

- `ogc_timecrs` : PROJ crée des `TemporalCRS` (`AnsiDate`, `JulianDate`,
  `UnixTime`). Le modèle Rust n'a pas de variante temporelle dans `Crs`.
- `AuthorityFactory_createDynamicVerticalDatum` : dans PROJ, le `TEST()` est
  sous `#ifdef no_more_dynamic_vertical_datum` (`test_factory.cpp`, l. 320-330).
  Il n'est pas compilé dans les tests PROJ 9.9.0, donc il n'est pas porté. La
  base 9.9 n'a pas de `frame_reference_epoch` pour `EPSG:1096`, ce qui
  confirme que l'assertion n'est plus vraie.

## API ajoutée

| PROJ 9.9.0 | PROJ-RS |
|---|---|
| `AuthorityFactory::getGeoidModels` | `AuthorityFactory::get_geoid_models(code) -> Vec<String>` |
| `AuthorityFactory::getUnitList` | `AuthorityFactory::get_unit_list() -> Vec<UnitInfo>` |
| `AuthorityFactory::getCelestialBodyList` | `AuthorityFactory::get_celestial_body_list() -> Vec<CelestialBodyInfo>` |
| `AuthorityFactory::getOfficialNameFromAlias` | `AuthorityFactory::official_name_from_alias(alias, table, source, NameSpelling) -> Option<OfficialName>` |
| `DatabaseContext::toWGS84AutocorrectWrongValues` | `DatabaseContext::to_wgs84_autocorrect_wrong_values(&mut [f64; 7]) -> Result<bool, _>` |
| `CrsInfo::type` (`ObjectType`) | `CrsInfo::object_type() -> ObjectType` |

Le paramètre `tryEquivalentNameSpelling` (booléen) devient l'enum
`NameSpelling { Exact, Equivalent }`. Les paramètres de sortie
(`outTableName`, `outAuthName`, `outCode`) sont dans `OfficialName`.

`toWGS84AutocorrectWrongValues` prend les sept valeurs TOWGS84 dans l'ordre
`tx, ty, tz, rx, ry, rz, scale_difference` et modifie le tableau en place.

## Écarts volontaires

- `CrsInfo::type_name` garde la chaîne de la base (`other` pour
  `IAU_2015:19902`). `object_type()` donne le `ObjectType` que PROJ
  reporte (`GEODETIC_CRS`). `projinfo --type` continue à filtrer sur
  `type_name`, comme avant. Les deux ne sont pas alignés : `projinfo` ne liste
  pas les CRS `other` sous `geodetic`, alors que PROJ les liste. C'est un écart
  connu de `projinfo`, pas traité ici.
- `equivalent_name` (`isEquivalentName`) traduit les lettres accentuées
  minuscules par leur équivalent ASCII avec la table de PROJ
  (`map_utf8_to_lower` : á ä ě è é ë í ô ö ç). Ici la traduction s'applique
  aussi aux majuscules accentuées, que PROJ ne traduit pas. Les règles
  `biggerDifferencesAllowed` (« zone », « height », north/south, `19dd`) ne
  sont pas portées.
- `get_crs_info_list` avec une autorité vide liste toutes les autorités
  (`hasAuthorityRestriction` faux). Auparavant le filtre était toujours
  appliqué, donc la liste était vide.
- Équivalence non stricte d'un CRS géographique : le datum est comparé après
  `asDatum` (un ensemble devient son premier membre), et les axes ne comparent
  que la direction et l'unité, comme `CoordinateSystemAxis::_isEquivalentTo`.
  Le critère strict et le matching d'identification (`identify`) gardent
  l'ancien comportement.
- `Ellipsoid::export_to_proj_string` (et `+ellps` des datums) utilise la table
  complète `pj_ellps` de PROJ, désormais dans `src/ellipsoid.rs::PJ_ELLPS` et
  partagée avec `engine.rs::named_ellipsoid`. Avant, seuls six ellipsoïdes
  avaient un nom `+ellps` ; `EPSG:2172` donnait `+a +rf` au lieu de
  `+ellps=krass`.
- `OGC:84` est résolu comme `OGC:CRS84` dans
  `create_coordinate_reference_system`, comme PROJ.

## Commandes

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
PATH=<proj-ref>/bin:$PATH PROJ_CS2CS_REQUIRED=1 PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES \
  cargo test --all-features --no-fail-fast
```

`test_factory` : 58 / 58. Suite complète : aucun test en échec.
