# Port — CLI `proj` : CRS projeté et facteurs d'échelle

Source : PROJ 9.9.0, `src/apps/proj.cpp` (`main`, `process`), `src/factors.cpp`
(`pj_factors`, `proj_factors`), `src/deriv.cpp` (`pj_deriv`),
`src/aasincos.cpp` (`aasin`). Le binaire de référence est `proj` 9.9.0.

Ce lot porte les deux cas ouverts de `tests/cli_yaml/test_proj.yaml`
(`#3` : `EPSG:27571 -S`, `#4` : `EPSG:5972 -S`). Il ne porte pas le reste des
options de `proj` (voir « Hors périmètre »).

## Correspondance d'API

| PROJ 9.9.0 | PROJ-RS (`src/bin/proj.rs`) |
|---|---|
| argument CRS sans `+proj` (`proj_create` + `PJ_TYPE_PROJECTED_CRS`) | `projected_definition` : `create_from_user_input` puis `Crs::Projected` |
| CRS composé, premier sous-CRS projeté (`proj_crs_get_sub_crs(P, 0)`) | `Crs::Compound` → `components().first()` |
| `proj_create_crs_to_crs_from_pj(geogNorm, P)` | `Operation::create` sur `ProjectedCRS::export_to_proj_string` |
| ordre des axes du CRS (sortie) | `Job::north_first` : le nord est imprimé d'abord |
| `proj_factors` / `pj_factors` | `Job::factors` |
| `pj_deriv` (différences centrées, h = 1e-5 rad) | `Job::derivatives` |
| `aasin` (borne à ±π/2, sans erreur) | `aasin` |
| `%g` de `limited_fprintf` / `printf` | `format_g` (format C : `8.53878e-07`) |
| `emess(3, …)` | sortie avec le code 3 |
| `proj_errno` partagé (contexte par défaut) | `error_seen` dans `main` |

## Correspondance de tests

| Comportement PROJ | Cas Rust |
|---|---|
| `EPSG:27571 -S` : pm de Paris, facteurs 0,999877 | `tests/cli_yaml`, `test_proj.yaml#3` |
| `EPSG:5972 -S` : premier sous-CRS projeté d'un composé | `tests/cli_yaml`, `test_proj.yaml#4` |
| `EPSG:32620 -S` (non-régression) | `tests/cli_yaml`, `test_proj.yaml#2` |
| `%g` de C : six chiffres, zéros retirés, exposant à deux chiffres | `format_g_matches_c_printf` |
| zéros de fin retirés, entiers conservés | `trim_zeros_keeps_integers` |

## Écarts volontaires

- **CRS : pas de transformation de datum.** L'ancienne version construisait
  `EPSG:4326 → CRS`, ce qui appliquait le décalage WGS 84 → datum du CRS
  (`499999.41` au lieu de `500000.00` pour `EPSG:5972`) et échouait pour
  `EPSG:27571` (« crs is not built »). PROJ part d'un géographique normalisé
  qui reprend le datum du CRS, ou seulement son ellipsoïde quand le méridien
  d'origine n'est pas Greenwich. Ici, l'opération est donc la projection
  seule, avec son `+pm` : l'entrée est en longitude Greenwich.
- **Ordre des axes.** Pour un CRS dont le premier axe est le nord
  (EPSG:3035, 2169, 2180, 2193), PROJ imprime le nord d'abord. Le lot fait
  de même. Les autres ordres (ouest, sud) ne sont pas traités.
- **Dérivées sur l'opération complète.** La différentiation passe par
  `Operation::forward` (le pipeline entier), pas par les étapes internes de
  PROJ. L'arithmétique n'est donc pas identique bit à bit. Conséquence : la
  partie `omega` de l'ordre de 1e-6 degré, qui est du bruit de différences
  finies, peut différer dans ses derniers chiffres (par exemple `1.20786e-06`
  chez PROJ, `0` ici). `test_proj.yaml#3` le traite déjà par `sub`.
- **Codes de sortie.** Une erreur d'initialisation (CRS non parsable, non
  projeté, `+proj` inconnu, absence de projection) sort avec 3, comme
  `emess(3)`. Une option inconnue reste à 1. Sans argument, l'usage est
  affiché et la sortie est 0, comme PROJ.

## Quirks conservés

- **Erreur collante pour `-S`.** PROJ partage un seul état d'erreur entre
  les calculs : après un échec de projection ou de facteurs, chaque ligne
  suivante avec `-S` affiche `<* * * * * *>`, même si son point est valide.
  `main` le reproduit avec `error_seen`. Vérifié avec
  `+proj=ortho +lon_0=0 +lat_0=0 +ellps=WGS84 -S` sur `10 0`, `100 0`,
  `20 10`, `100 0` : la référence et le binaire donnent la même sortie.
- **Échec de projection avec `-S`.** La ligne est `*\t*\t<* * * * * *>`, et
  `-E` recopie l'entrée avant `*\t*`.
- **Facteurs non finis = échec.** Une valeur NaN ou infinie dans les facteurs
  est imprimée comme `* * * * * *`. C'est ce que fait PROJ pour `EPSG:5514`
  à `10 60` ; ce port obtient des NaN à cet endroit et les traite de la même
  façon. La cause côté PROJ n'est pas identifiée.

## Hors périmètre (ouvert)

- Options `-I`, `-r`, `-s`, `-t`, `-i`, `-o`, `-b`, `-m`, `-d`, `-e`, `-w`,
  `-W`, `-V`, `-l` : non portées, elles sortent avec « unknown option ».
- Entrées DMS (`proj_dmstor`, par exemple `2d20'E 49d30'N`) : seules les
  décimales sont lues.
- Lignes de commentaire (`#`) : PROJ les recopie sur la sortie, ce port les
  ignore.
- **Facteurs NZTM loin du méridien central** (`EPSG:2193` à `9 0`, à 164°) :
  PROJ affiche `* * * * * *` parce qu'une dérivée échoue, alors que ce port
  calcule des valeurs aberrantes (`315619`). Les quatre points de dérivée
  sont identiques dans les deux programmes et aucun n'échoue côté PROJ
  (vérifié). Je n'ai pas identifié la condition d'échec de PROJ, donc je ne
  l'ai pas reproduite. Le même écart apparaît avec `+proj=tmerc +lon_0=173`
  sur `9 0`.

## Vérification

- `cargo fmt --check` : propre.
- `cargo clippy --all-targets --all-features -- -D warnings` : propre.
- `cargo test --bin proj` : 2 tests unitaires verts.
- `cargo test --test cli_yaml` avec le harnais (`PATH` sur la référence,
  `PROJ_CS2CS_REQUIRED=1`) : `test_proj.yaml` passe entièrement.
- Comparaison à la référence sur 21 CRS, 8 points, avec et sans `-S`
  (336 cas) : coordonnées identiques dans tous les cas hors NZTM à 164°.
  Restent les différences de bruit de `omega` et le cas NZTM ci-dessus.
