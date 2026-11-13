# Lot `io-create-user-input-final` : fermeture de `io.createFromUserInput`

Worktree `agent-a0406c184d94e2e85`, branche `worktree-agent-a0406c184d94e2e85`, base `7e6f616` (fusion de
`claude/upbeat-meitner-32qhxg`, qui porte la première partie du test). Un commit, pas de push, pas de PR.

Le test C++ est `TEST(io, createFromUserInput)`, `test/unit/test_io.cpp` 13718-14275 (PROJ 9.9.0). Les valeurs
attendues viennent du C++ quand il les donne. Sinon elles viennent du `projinfo` de référence
(`target/proj-reference/build/bin`, `PROJ_DATA` du même build). Aucune valeur n'est tirée du code porté.

## Sources PROJ lues

| PROJ 9.9.0 (`src/src/iso19111/`) | Lignes | Rôle |
|---|---|---|
| `io.cpp` `createFromUserInput` (début) | 7711-7800 | espaces, `@` (époque), JSON, WKT avant les noms |
| `io.cpp` 2 jetons `AUTH:CODE` | 7832-7888 | repli versionné, ci, `+` (`createLax`) |
| `io.cpp` 3 jetons `AUTH:CODE+AUTH:CODE` | 7889-7910 | `createLax` |
| `io.cpp` recherche par nom (`searchObject`, deux passes de compound) | 8157-8299 | noms, compounds `A + B`, quatre passes |
| `crs.cpp` `CompoundCRS::createLax` | 5396-5460 | cas géographique (équivalence, `promoteTo3D`) |
| `crs.cpp` `GeographicCRS::_exportToWKT` | 2166-2187 | WKT1 non conforme (`demoteTo2D` puis `identify` à 100) |
| `crs.cpp` `ProjectedCRS::_exportToWKT` | 4347-4362 | idem pour le projeté ; la base est seulement démotée |
| `metadata.cpp` `Identifier::isEquivalentName` | 1383-1516 | `19dd` dans les deux modes ; `zone`, `height`, `north` en mode large |
| `coordinates.cpp` `CoordinateMetadata::_exportToJSON` | 255-270 | `type`, `crs`, `coordinateEpoch` |
| `datum.cpp` `PrimeMeridian`, `Ellipsoid`, `DatumEnsemble` `_exportToJSON` | 479, 954, 2069 | `MakeObjectContext` : `type` en tête |

## Carte C++ -> Rust

| PROJ | Rust | Fichier |
|---|---|---|
| `createFromUserInput(text, db)` (routage) | `read_user_object` (renvoie `UserObject`), `create_from_user_input_with_optional_database` l'enveloppe en `Crs` | `json.rs` |
| `createFromUserInput` pour un objet | `create_object_from_user_input` -> `read_user_object` | `json.rs` |
| recherche par nom | `object_from_name`, `compound_of_names`, `crs_of_compound_name`, `search_object`, `authority_object_name`, `user_object_of` (remplacent `crs_from_name` et `crs_named_one`) | `json.rs` |
| `searchObject` : égalité de nom | `equiv::equivalent_name_with(_, _, NameComparison)` ; `NameComparison::Exact` = `isEquivalentName(a, b, false)` | `equiv.rs` |
| `CompoundCRS::createLax` (cas géographique) | `CompoundCRS::create_lax_with_database` ; `lax_geographic_pair` | `model.rs` |
| `createLax` appelé par `AUTH:CODE+CODE` | `implicit_compound(db, ...)` | `json.rs` |
| WKT1 non conforme | `nonconformant_geographic_2d`, `nonconformant_projected_2d` | `wkt.rs` |
| PROJJSON des objets | `UserObject::export_to_json` : `VerticalDatum`, `Ensemble`, `PrimeMeridian`, `Ellipsoid`, `Operation`, `CoordinateMetadata` | `json.rs` |

`CompoundCRS::create_lax` garde sa signature (il appelle `create_lax_with_database(_, _, None)`).

## Couverture, bloc par bloc

| Lignes C++ | Contenu | Test Rust |
|---|---|---|
| 13720-13745, 13764-13816, 13818-13865, 13968-13982, 13986-14117 | formes déjà portées | `tests/unit_io_create_from_user_input.rs` (inchangé) |
| 13746-13783 | URN d'opérations, datum, ensemble, méridien, ellipsoïde | `tests/unit_io_create_object_blocks.rs` (inchangé) |
| 13767 | `NN2000:2018 height` : code EPSG 5941 (lu par le nom) | `tests/unit_io_create_user_input_final.rs` `io_create_from_user_input_name_with_colon_and_id74` |
| 13867-13906 | `EPSG:4326+4326` : géographique 3D `WGS 84`, code 4979, WKT `COMPD_CS` | `io_create_from_user_input_non_conformant_geographic_compound` |
| 13907-13908 | `EPSG:4326+4258` : `InvalidCompoundCRSException` | `io_create_from_user_input_non_consistent_compound` |
| 13910-13960 | `EPSG:32631+4326` : projeté, base 3D 4979, WKT `COMPD_CS` | `io_create_from_user_input_non_conformant_projected_compound` |
| 14120-14274 | noms : `WGS 84`, `WGS84`, `UTM zone 31N`, `UTM zone 31`, `WGS84 UTM zone 31N`, `ID74`, `NGF IGN69 height`, `NGF IGN1969`, `NGF IGN69`, les composés de noms, `WGS 84 / World Mercator + EGM2008 height`, `foobar`, `foo`, `omerc`, `WGS84 PseudoMercator`, l'ensemble WGS 84, `WGS 84 UTM 31 north`, `WGS 85` | `io_create_from_user_input_names` (et `io_create_from_user_input`, `foobar`) |
| 14219-14268 | époques `@` | `unit_io_create_object_blocks.rs` (déjà porté, vérifié) |

Le test `io_create_from_user_input_projjson_of_objects` couvre l'export PROJJSON des six genres d'objets.

## Valeurs vérifiées contre le `projinfo` de référence

- `EPSG:4326+4326` et `EPSG:32631+4326` : le WKT1 GDAL de `projinfo` est identique au `COMPD_CS` du C++
  (les deux parties démotées, codes 4326 et 32631).
- `EPSG:4326+4258` : même message que `projinfo` : « The 'vertical' geographic CRS is not equivalent to the
  geographic CRS of the horizontal part ».
- `NN2000:2018 height` : `VERT_CS`, code EPSG 5941. `ID74` : `GEOGCS`, code EPSG 4238.
- `UTM zone 31N` : `projinfo` le lit comme une conversion (EPSG 16031), pas comme un CRS. Le C++ rend donc un
  objet ; `create_from_user_input` le refuse et `create_object_from_user_input` renvoie `UserObject::Operation`.
- `NGF IGN69` : `projinfo` donne `NGF-IGN 1969`, comme le C++. Le résultat dépend du mode d'égalité
  (`19dd`), corrigé dans cette passe.
- PROJJSON : l'`ensemble`, le méridien, l'ellipsoïde et la datum verticale ont les champs que `projinfo -o PROJJSON`
  écrit pour EPSG:4326 et EPSG:5703 (blocs `datum_ensemble`, `prime_meridian`, `ellipsoid`, `datum`), avec `type`
  en tête pour un objet de premier niveau.

## Écarts

1. **Erreurs typées.** `ParsingException` (nom inconnu, format inconnu) donne un `Err` (`FactoryError::Format`
   ou `Invalid`, selon le texte). `InvalidCompoundCRSException` donne `FactoryError::Invalid` avec le message
   du C++. Le test ne vérifie que `is_err()` pour les noms inconnus.
2. **`create_from_user_input` ne renvoie que des CRS.** Un nom qui désigne un datum, un ensemble, un ellipsoïde ou
   une opération (`World Geodetic System 1984 ensemble`, `UTM zone 31N`) donne
   `FactoryError::Invalid("...: not a coordinate reference system")`. Le C++ rend un `BaseObject`. Le même texte
   passé à `create_object_from_user_input` donne l'objet.
3. **Égalité des noms.** `equiv::equivalent_name` (inchangé, pour les autres appelants) ne lit pas `19dd`. La
   recherche utilise `equivalent_name_with(_, _, Exact)`, qui suit `isEquivalentName(a, b, false)`. Le mode
   `BiggerDifferences` est celui qui existait ; il n'a pas changé.
4. **Export WKT1 non conforme.** Le WKT d'un CRS 3D non conforme écrit maintenant les parties démotées et
   identifiées (`EPSG:4326` et `EPSG:32631`), comme `crs.cpp`. Avant, il écrivait le code 3D (4979). Les
   appelants qui exportent un composé `A+B` non conforme voient ce changement.
5. **`createLax` géographique.** `EPSG:4326+4326` est maintenant un géographique 3D, et `EPSG:4326+4258` une
   erreur typée. Avant, la seconde donnait « a compound CRS cannot contain two horizontal CRS ».
6. **PROJJSON autonome.** Une datum verticale exportée seule écrit `scope`, `area`, `bbox` et `id` (membres
   complets, comme le C++ pour un objet de premier niveau). Dans un CRS, `projinfo` écrit la forme abrégée
   (`name`). Aucun outil de référence n'exporte un objet seul, donc ce point n'est pas comparé à une sortie
   réelle. `CoordinateMetadata` suit `coordinates.cpp` 255-270 : pas de sortie de référence non plus.
7. **Dead code retiré.** `DatabaseContext::crs_codes_named` (`factory.rs`) n'avait plus d'appelant.

## Appelants à risque

- La recherche par nom n'est plus filtrée par `:` ni `+` (sans espace). Un texte comme `NN2000:2018 height` ou
  `WGS84+EGM96` est maintenant cherché par nom au lieu d'être refusé. Les textes avec `[` restent WKT.
- `create_object_from_user_input` passe par le même routage que `create_from_user_input` : un nom d'objet non
  CRS devient un `UserObject` au lieu d'une erreur.
- Les appelants de `create_lax` et de `implicit_compound` (`AUTH:CODE+CODE`) voient le cas géographique. Le
  `WKT1_GDAL` d'un composé non conforme change (écart 4).

Fichiers touchés par cette passe : `src/iso19111/json.rs`, `src/iso19111/model.rs`, `src/iso19111/wkt.rs`,
`src/iso19111/equiv.rs`, `src/iso19111/factory.rs` (suppression), `tests/unit_io_create_user_input_final.rs`
(nouveau), `docs/UNPORTED-UNIT.md` (une entrée retirée).

## Ce qui reste ouvert

Rien dans `io.createFromUserInput` : chaque assertion du C++ est couverte par un test Rust, ou vérifiée contre le
`projinfo` de référence quand le C++ ne donne pas la valeur. `README.md`, `ROADMAP.md` et `docs/UNIT-TRIAGE.md`
ne sont pas régénérés (consigne du lot) : `docs_in_sync` échoue jusqu'à `UPDATE_DOCS=1`, à lancer par le
coordinateur.
