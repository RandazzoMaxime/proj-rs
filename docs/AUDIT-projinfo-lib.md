# Audit : `test/unit/test_projinfo_lib.cpp` (PROJ 9.9.0)

Sources lues (lecture seule) :

- `target/proj-reference/src/test/unit/test_projinfo_lib.cpp` (140 lignes, 5 `TEST()`) ;
- `target/proj-reference/src/src/apps/projinfo_lib.cpp` (2246 lignes, lu en entier) ;
- `target/proj-reference/src/src/apps/projapps_lib.h` (la signature C de `projinfo()`).

Le fichier de test n'avait jamais été audité. Les lots successifs portent `simple`, `error`,
`warning`, puis `use_ctx`.

## Ce que fait la fonction testée

`int projinfo(PJ_CONTEXT *ctx, int argc, char **argv, projinfo_cb_t cb, void *user_data)`
(`projinfo_lib.cpp`, fin du fichier) :

- `ctx` sert à ouvrir la base (`DatabaseContext::create(mainDBPath, auxDBPath, ctx)`) ;
- la sortie n'est jamais écrite directement : `Streamer` la transmet à `cb` par niveau
  (`INFO` pour stdout, `WARN` et `ERR` pour stderr). Si `cb` est nul, `Stream::doit`
  n'écrit rien ;
- la valeur de retour est 0, ou 1 après une erreur ; une exception non gérée devient
  `pj_log(ctx, PJ_LOG_ERROR, ...)` et 1.

## Tableau des 5 `TEST()`

| TEST | Assertions C++ | Statut dans ce lot | Cause exacte (après lecture du C++) |
|---|---|---|---|
| `simple` | `EXPECT_EQ(res, 0)` ; `EXPECT_EQ(found, true)`, où `found` devient vrai si un message `INFO` contient `PROJCS["ETRS89 / UTM zone 32N"` | **Porté** : `simple_returns_zero`, `simple_found_etrs89_utm_zone_32n` (`tests/unit_projinfo_lib.rs`) | Aucun écart. Les deux assertions tiennent sur le binaire d'avant le lot : `EPSG:25832 -o WKT1_GDAL` rend le même texte que PROJ 9.9.0 (sortie comparée octet par octet). Le contexte `nullptr` de PROJ est la base par défaut, qui est celle de `run`. Le bloc WKT est écrit par un seul `<<` (`outputObject`, bloc WKT1:GDAL), donc un seul message `INFO` : tester le texte entier de `output` revient à tester le message. |
| `error` | `EXPECT_EQ(res, 1)` ; un message `ERR` contient `unrecognized format / unknown name` | **Porté** : `error_returns_one`, `error_found_unrecognized_format` (`tests/unit_projinfo_lib.rs`) | Aucun écart. Sortie comparée à PROJ 9.9.0 sur `doesnotwork` : même statut 1, même ligne sur stderr (`input string: parsing of 'doesnotwork' failed: unrecognized format / unknown name`), stdout vide des deux côtés. Le texte est celui de `ParsingException("unrecognized format / unknown name")` (`src/iso19111/io.cpp`, l. 8301), que le port reprend dans `create_default`. Pas d'usage : l'erreur n'est pas une erreur d'option, donc `errors` ne reçoit que ce message. |
| `warning` | `EXPECT_EQ(res, 0)` ; un message `WARN` contient `Error when exporting to WKT1:GDAL` | **Porté** : `warning_returns_zero`, `warning_found_error_when_exporting_wkt1_gdal` (`tests/unit_projinfo_lib.rs`) | Aucun écart. `EPSG:4937 -o WKT1_GDAL` : statut 0 et la ligne `Error when exporting to WKT1:GDAL: WKT1 does not support Geographic 3D CRS.` sur stderr des deux côtés, stdout identique (`WKT1:GDAL string:`) ; sortie comparée octet par octet à PROJ 9.9.0. Le port la range dans `warnings` (`render`). `Streamer::doit` envoie chaque `<<` séparément : `"Error when exporting to WKT1:GDAL: "` est un seul `<<`, donc la sous-chaîne tient dans un seul message de PROJ, comme dans le port. |
| `use_ctx` | Deux appels avec le même `PJ_CONTEXT*` (`EPSG:25832`, puis `EPSG:25833`), chacun `res == 0` et un `INFO` contenant `PROJCS["ETRS89 / UTM zone 3` | **Porté** : `use_ctx` (`tests/unit_projinfo_lib.rs`), sur `run_with_context(&Context, args)`. Le test vérifie aussi le nom complet, `ETRS89 / UTM zone 32N` puis `ETRS89 / UTM zone 33N`. | L'écart était l'absence de contexte dans l'API. `database()` et `candidates` (`src/projinfo.rs`) ouvrent maintenant `Context::database()` quand un contexte est donné ; `run` garde `DatabaseContext::create()` (contexte nul). Valeurs vérifiées avec `projinfo` 9.9.0 et avec un programme C contre `libproj.so` : `projinfo(ctx, ...)` deux fois sur un même `PJ_CONTEXT*` rend `res=0` et la ligne `PROJCS["ETRS89 / UTM zone 3...` pour `EPSG:25832` et `EPSG:25833`. Limite, comme en C++ : le test ne voit pas un contexte ignoré, car la base par défaut est la même. Restent hors de ce test les usages réseau de `ctx` (`proj_context_is_network_enabled` pour `--grid-check`, `proj_context_get_url_endpoint` pour `--remote-data`) et `searchpaths`, qui lisent encore le contexte par défaut. |
| `works_without_cb` | `EXPECT_EQ(res, 0)` pour `EPSG:25832 -o WKT1_GDAL` avec `cb == nullptr` et `user_data == nullptr` ; « ne plante pas » | Non porté, **par choix : pas de test Rust**. Aucun `#[test]` ne peut porter l'état « callback nul », car `run(&[&str]) -> Report` n'a pas de callback à omettre. L'assertion restante, `res == 0` pour le même `argv`, est déjà `simple_returns_zero` (même `argv`, même attendu). | Cause exacte : `Stream::doit` (`projinfo_lib.cpp`, l. 101) n'écrit rien quand `callback` est nul ; le port n'a pas de voie de sortie à omettre. Ajouter un paramètre `Option<callback>` pour ce seul test créerait une deuxième voie de sortie à côté de `Report`, contraire à la règle du port (valeurs renvoyées, rien n'est écrit). Vérifié avec PROJ 9.9.0 : un programme C appelant `projinfo(NULL, 3, argv, NULL, NULL)` contre `libproj.so` rend `res=0` et sort normalement (pas de plantage). Un test Rust `works_without_cb` ne ferait que doubler `simple_returns_zero` : il n'est pas ajouté. |

## Résumé

- 4 `TEST()` sur 5 sont entièrement portés : `simple`, `error`, `warning` (2 assertions
  chacun, 2 tests Rust chacun) et `use_ctx` (2 appels, 2 assertions chacun, 1 test Rust).
- `works_without_cb` : pas d'équivalent, puisque l'API n'a pas de callback ; son statut est déjà couvert.
  Il reste le seul `TEST()` non porté de ce fichier.

## Points relevés en lisant le C++

- `Streamer::doit` appelle le callback une fois par `<<`, avec le texte de ce seul `<<`. Un test
  qui cherche une sous-chaîne doit donc la trouver dans un seul message. C'est vrai pour
  `PROJCS[...]` (un seul `<<`), mais pas pour un texte coupé en plusieurs `<<`.
- `usage()` écrit sur `ERR` (`is_error == true`) ; `-h` écrit sur `INFO` avec le code 0.
  Le port garde la même séparation : `usage()` est dans `errors` après une erreur d'option.
