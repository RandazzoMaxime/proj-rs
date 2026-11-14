# Audit : `test/unit/test_projinfo_lib.cpp` (PROJ 9.9.0)

Sources lues (lecture seule) :

- `target/proj-reference/src/test/unit/test_projinfo_lib.cpp` (140 lignes, 5 `TEST()`) ;
- `target/proj-reference/src/src/apps/projinfo_lib.cpp` (2246 lignes, lu en entier) ;
- `target/proj-reference/src/src/apps/projapps_lib.h` (la signature C de `projinfo()`).

Le fichier de test n'avait jamais été audité. Le lot ne porte que `simple`.

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
| `use_ctx` | Deux appels avec le même `PJ_CONTEXT*` (`EPSG:25832`, puis `EPSG:25833`), chacun `res == 0` et un `INFO` contenant `PROJCS["ETRS89 / UTM zone 3` | Non porté. **Écart réel.** | L'API n'a pas de paramètre de contexte. `database()` (`src/projinfo.rs`, `fn database`) et `candidates` (`DatabaseContext::create()`) ouvrent la base par défaut et ignorent tout contexte. Les valeurs sont justes (vérifié pour `EPSG:25833` contre la référence). Pour porter le test : une fonction `run_with_context(&Context, args)`, et `Context::database()` (`src/context.rs`, ligne 196) aux trois sites. Le C++ utilise aussi `ctx` pour le réseau (`proj_context_is_network_enabled`, valeur par défaut de `--grid-check`, et `proj_context_get_url_endpoint` pour `--remote-data`) : ces usages ne sont pas dans `simple`. |
| `works_without_cb` | `EXPECT_EQ(res, 0)` pour `EPSG:25832 -o WKT1_GDAL` avec `cb == nullptr` et `user_data == nullptr` ; « ne plante pas » | Non porté. **Pas d'équivalent Rust.** | L'assertion porte sur le callback C nul : `Stream::doit` ne fait rien quand `callback` est nul. L'API Rust n'a pas de callback et renvoie toujours un `Report`, donc l'état « sans callback » n'existe pas. La partie vérifiable, le statut 0 pour le même `argv`, est déjà couverte par `simple_returns_zero`. |

## Résumé

- 3 `TEST()` sur 5 sont entièrement portés : `simple`, `error` et `warning` (2 assertions
  chacun, 2 tests Rust chacun).
- `use_ctx` : écart réel, dû à l'absence de contexte dans l'API (trois sites `DatabaseContext::create()`).
- `works_without_cb` : pas d'équivalent, puisque l'API n'a pas de callback ; son statut est déjà couvert.

## Points relevés en lisant le C++

- `Streamer::doit` appelle le callback une fois par `<<`, avec le texte de ce seul `<<`. Un test
  qui cherche une sous-chaîne doit donc la trouver dans un seul message. C'est vrai pour
  `PROJCS[...]` (un seul `<<`), mais pas pour un texte coupé en plusieurs `<<`.
- `usage()` écrit sur `ERR` (`is_error == true`) ; `-h` écrit sur `INFO` avec le code 0.
  Le port garde la même séparation : `usage()` est dans `errors` après une erreur d'option.
