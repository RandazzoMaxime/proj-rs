# Port : `projinfo_lib` (API de bibliothèque `proj_rs::projinfo`)

Lot : déplacer la logique du binaire `projinfo` dans la bibliothèque, pour exposer
l'équivalent de `projinfo()` de PROJ 9.9.0, et porter `TEST(projinfo_lib, simple)` et
`TEST(projinfo_lib, error)` (le second, ajouté ensuite, ne change pas la lib : le port
donnait déjà la sortie de PROJ).
Audit des cinq `TEST()` : [AUDIT-projinfo-lib.md](AUDIT-projinfo-lib.md).

## Carte

| C++ (`src/apps/projinfo_lib.cpp`, `projapps_lib.h`) | Rust |
|---|---|
| `int projinfo(ctx, argc, argv, cb, user_data)`, contexte nul | `proj_rs::projinfo::run(&[&str]) -> Report` |
| `int projinfo(ctx, argc, argv, cb, user_data)`, contexte donné | `proj_rs::projinfo::run_with_context(&Context, &[&str]) -> Report` |
| `Streamer` : `INFO` / `WARN` / `ERR` vers `cb` | `Report::output` / `Report::warnings` / `Report::errors` |
| retour 0 ou 1 | `Report::status`, `Status::Success` ou `Status::Failure` |
| `main_projinfo` | `execute(args, warnings, context)` |
| `buildObject` | `build_object`, `read_wkt`, `load_named`, `named_object` |
| `outputObject` | `render` (un bloc par dialecte) |
| `outputOperations` | `candidates` |
| `usage(strm, is_error)` | `usage()` : sortie pour `-h`, `errors` après une erreur d'option |

`src/bin/projinfo.rs` n'est plus qu'un appel de `run` : il écrit les avertissements sur
stderr, puis le texte sur stdout, ou les erreurs sur stderr, et sort avec 0 ou 1.

## Écarts volontaires

1. **Forme de l'API.** `run` et `run_with_context` prennent un `&[&str]` et renvoient des
   valeurs. Le callback est absent : la bibliothèque n'écrit ni sur stdout ni sur stderr
   (règle `print_stdout` du projet). Le contexte (`ctx`) est un paramètre de
   `run_with_context` depuis le lot `use_ctx` ci-dessous ; `run` garde le contexte nul.
2. **Ordre des messages.** Les avertissements sont poussés dans `warnings` dans l'ordre
   où ils surviennent. Le binaire les écrit avant la sortie ou les erreurs : c'est l'ordre
   d'avant le lot, sur stderr.
3. **Pile du thread.** Le thread de 64 MiB (`STACK_SIZE`) passe du binaire dans `run`,
   pour que les appelants de la bibliothèque aient la même pile. Motif inchangé :
   `test_projinfo.yaml#62`.
4. **Panique.** PROJ attrape les exceptions dans `projinfo()` et renvoie 1 (`pj_log`).
   Le port ne les attrape pas : une panique du thread est relancée par `run`
   (`resume_unwind`), et le binaire sort alors avec 101, comme avant le lot.
5. **Texte d'usage gardé tel quel.** `usage()` dit encore « Other dialects (WKT1, WKT2:2015,
   SQL, all) are an error », alors que `WKT1:GDAL`, `WKT1:ESRI`, `WKT2:2015` et `SQL` sont
   écrits. Ce texte est la sortie du binaire ; le corriger change la sortie, donc il est
   laissé à un lot qui change la sortie. L'en-tête du module, lui, est corrigé.
6. **Type interne renommé.** L'enum d'échec `Report` (`Usage`, `Plain`) devient `Failure`,
   pour laisser le nom `Report` à la valeur publique.

## Vérification

Sortie du binaire avant et après le lot, sur 42 cas (`stdout`, `stderr`, code de retour),
dont les avertissements (`WARNING`, `Warning:`, `Grammar error:`, `Error when exporting`),
les erreurs d'option et l'usage : **identiques**.

Commandes lancées (depuis la racine du worktree) :

```bash
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
cargo test --test unit_projinfo_lib          # 4 tests, verts (rouge avant l'API)
cargo test --test cli_projinfo_text          # 3 tests, verts
cargo test --test cli_yaml                   # 14 échecs, tous test_cs2cs_ignf.yaml (CDN)
git checkout docs/UNPORTED-CLI.md            # restauré après cli_yaml
cargo +1.100.0 clippy --all-targets --all-features -- -D warnings   # propre
cargo fmt --check
```

Référence PROJ 9.9.0 pour `simple` :

```bash
R=target/proj-reference/build
PROJ_DATA=$R/data LD_LIBRARY_PATH=$R/lib $R/bin/projinfo EPSG:25832 -o WKT1_GDAL
```

Sortie identique à celle du port (`PROJCS["ETRS89 / UTM zone 32N"`, code 0).

Test `TEST(projinfo_lib, warning)` (ajouté ensuite, sans changement de la lib) : le port
donnait déjà la sortie de PROJ. Deux tests Rust (`warning_returns_zero`,
`warning_found_error_when_exporting_wkt1_gdal`) sur `EPSG:4937 -o WKT1_GDAL`, qui écrit
`Error when exporting to WKT1:GDAL: WKT1 does not support Geographic 3D CRS.` sur la voie WARN
et sort avec 0. Référence PROJ 9.9.0, stdout et stderr comparés séparément au binaire du port :
identiques (stdout `WKT1:GDAL string:`, stderr la ligne d'avertissement).

```bash
R=target/proj-reference/build
PROJ_DATA=$R/data LD_LIBRARY_PATH=$R/lib $R/bin/projinfo EPSG:4937 -o WKT1_GDAL
# stderr : Error when exporting to WKT1:GDAL: WKT1 does not support Geographic 3D CRS.
# code de retour 0
```

Référence PROJ 9.9.0 pour `error` (le même `argv` que le test, un seul argument) :

```bash
R=target/proj-reference/build
PROJ_DATA=$R/data LD_LIBRARY_PATH=$R/lib $R/bin/projinfo doesnotwork
# stderr : input string: parsing of 'doesnotwork' failed: unrecognized format / unknown name
# code de retour 1, stdout vide ; le binaire du port donne la même chose, octet pour octet
```

## Lot `use_ctx`

`TEST(projinfo_lib, use_ctx)` : deux appels de `projinfo` sur un même contexte, `EPSG:25832`
puis `EPSG:25833`, chacun avec `res == 0` et un INFO qui contient `PROJCS["ETRS89 / UTM zone 3`.
Le test Rust (`use_ctx`, `tests/unit_projinfo_lib.rs`) ajoute les noms complets
(`ETRS89 / UTM zone 32N`, `ETRS89 / UTM zone 33N`).

Changements : `run_with_context` copie le `Context` dans le thread de la commande.
`database(context: Option<&Context>)` ouvre `Context::database()` si un contexte est donné,
et `DatabaseContext::create()` sinon. Le contexte passe par `command`, `execute`,
`complete_shell`, `candidates` et `describe`. `run` passe `None` : il appelle la même
fonction qu'avant le lot, donc son comportement ne change pas.

Écarts restants :

- Le réseau (`--grid-check` par défaut, `--remote-data`) et `searchpaths` lisent encore le
  contexte par défaut (`Proj::new`). PROJ les lit sur `ctx`. Hors de l'assertion de `use_ctx`.
- Le test ne voit pas un contexte ignoré : la base par défaut est la même. Un contexte à base
  absente n'est pas une sonde : avec PROJ 9.9.0, `proj_context_set_database_path` échoue
  (`Open of /nonexistent/dir/proj.db failed`) et laisse le contexte sur sa base, si bien que
  `projinfo` rend encore 0 et la ligne attendue.

Référence PROJ 9.9.0 : `projinfo EPSG:25832 -o WKT1_GDAL` rend `PROJCS["ETRS89 / UTM zone 32N"`,
et `EPSG:25833` rend `PROJCS["ETRS89 / UTM zone 33N"`, tous deux avec le code 0. Programme C
compilé contre `libproj.so` (`projinfo(ctx, ...)` deux fois sur un même `PJ_CONTEXT*`) : `res=0`
et la ligne trouvée pour les deux codes.

```bash
cargo test --test unit_projinfo_lib     # 7 tests, verts (use_ctx : E0432 avant l'API)
cargo test --lib projinfo               # 7 tests, verts (describe reçoit None dans le test)
```

## Hors de ce lot

- `works_without_cb` : non porté, par choix. Son assertion (`res == 0` pour
  `EPSG:25832 -o WKT1_GDAL`, `cb` et `user_data` nuls) est déjà `simple_returns_zero`, avec le
  même `argv` et le même attendu. Le cas « cb nul » n'a pas de forme Rust : `run` ne prend pas de
  callback, et un paramètre `Option<callback>` ajouté pour ce seul test créerait une deuxième voie
  de sortie à côté de `Report`. Un changement d'API ne se justifie que si un usage réel demande
  un callback.
  Référence PROJ 9.9.0 : un programme C qui appelle `projinfo(NULL, 3, argv, NULL, NULL)`, compilé
  contre `libproj.so` (`PROJ_DATA` = données de la référence), rend `res=0` et sort normalement.
- `docs/UNPORTED-UNIT.md`, `README.md`, `ROADMAP.md`, `docs/UNIT-TRIAGE.md` : non modifiés ici,
  ils sont régénérés avec `UPDATE_DOCS=1 cargo test --test docs_in_sync`.
