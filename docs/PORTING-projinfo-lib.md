# Port : `projinfo_lib` (API de bibliothèque `proj_rs::projinfo`)

Lot : déplacer la logique du binaire `projinfo` dans la bibliothèque, pour exposer
l'équivalent de `projinfo()` de PROJ 9.9.0, et porter `TEST(projinfo_lib, simple)` et
`TEST(projinfo_lib, error)` (le second, ajouté ensuite, ne change pas la lib : le port
donnait déjà la sortie de PROJ).
Audit des cinq `TEST()` : [AUDIT-projinfo-lib.md](AUDIT-projinfo-lib.md).

## Carte

| C++ (`src/apps/projinfo_lib.cpp`, `projapps_lib.h`) | Rust |
|---|---|
| `int projinfo(ctx, argc, argv, cb, user_data)` | `proj_rs::projinfo::run(&[&str]) -> Report` |
| `Streamer` : `INFO` / `WARN` / `ERR` vers `cb` | `Report::output` / `Report::warnings` / `Report::errors` |
| retour 0 ou 1 | `Report::status`, `Status::Success` ou `Status::Failure` |
| `main_projinfo` | `execute(args, warnings)` |
| `buildObject` | `build_object`, `read_wkt`, `load_named`, `named_object` |
| `outputObject` | `render` (un bloc par dialecte) |
| `outputOperations` | `candidates` |
| `usage(strm, is_error)` | `usage()` : sortie pour `-h`, `errors` après une erreur d'option |

`src/bin/projinfo.rs` n'est plus qu'un appel de `run` : il écrit les avertissements sur
stderr, puis le texte sur stdout, ou les erreurs sur stderr, et sort avec 0 ou 1.

## Écarts volontaires

1. **Forme de l'API.** `run` prend un `&[&str]` et renvoie des valeurs. Le contexte
   (`ctx`) et le callback sont absents : la bibliothèque n'écrit ni sur stdout ni sur
   stderr (règle `print_stdout` du projet), et `simple` passe un contexte nul, qui vaut la
   base par défaut. Le contexte viendra avec `use_ctx` (voir l'audit, écart réel).
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

## Hors de ce lot

- `use_ctx` : écart réel (pas de contexte dans l'API, trois sites `DatabaseContext::create()`).
- `works_without_cb` : pas d'équivalent Rust (pas de callback).
- `docs/UNPORTED-UNIT.md`, `README.md`, `ROADMAP.md`, `docs/UNIT-TRIAGE.md` : non modifiés ici,
  ils sont régénérés avec `UPDATE_DOCS=1 cargo test --test docs_in_sync`.
