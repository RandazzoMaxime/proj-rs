# Port — `io.projparse_*` : familles de projections (lot io-6)

Source : PROJ 9.9.0, `test/unit/test_io.cpp`, groupe `io.projparse_*` des
familles UTM, `etmerc`/`tmerc`, `laea`, `eqc`, `ortho` et `peirce_q`, plus
`ob_tran`. Côté crate : `tests/test_io_projfam.rs` (nouveau),
`src/iso19111/projfam.rs` (nouveau), et quatre retouches de
`src/iso19111/projstring.rs` et `src/iso19111/buildop.rs` (section 3).

Ce lot porte **17 `TEST()`** et laisse **6 tests tentés** ouverts, avec la
cause de chacun. Les chaînes attendues sont celles de PROJ, sans modification.

## 0. Périmètre et base

- La base de ce lot n'est pas `main` (`ad1451a`), mais `feat/pensive-allen-fkoqxo`
  (`cd23575`). `docs/UNIT-TRIAGE.md`, `tests/docs_in_sync.rs` et la commande
  `UPDATE_DOCS=1` n'existent que là. Sur `main`, la commande de régénération
  ne se lance pas. Les commits io-4, io-5, bound-import et triage de cette
  base ne sont pas l'objet de ce lot : le diff du lot est la seule différence
  avec `cd23575`.
- Les tests `io.projparse_*` de io-5 (`proj_aux`, `+R_A`, `+sweep=x`) sont
  présents dans cette base. Les règles de choix de méthode du lot s'y ajoutent.
- Aucune valeur attendue n'a été modifiée. Les constantes `EPSG_CODE_METHOD_*`
  de PROJ ne sont pas dans les en-têtes publics accessibles ; les codes sont
  ceux de la table EPSG de la crate (`src/iso19111/parammap.rs`), qui sont les
  mêmes nombres. Une première version du test `ortho_with_alpha` contenait
  `9833` pour `Local Orthographic` : c'était une erreur de transcription, que
  la table EPSG corrige. `9833` est `Hyperbolic Cassini-Soldner` ; `Local
  Orthographic` est `1130`. Le test a été corrigé à `1130` avant tout passage.

## 1. Portés (17)

| PROJ 9.9.0 `io.projparse_*` | PROJ-RS `tests/test_io_projfam.rs` | Ce qui est vérifié |
|---|---|---|
| `utm` | `io_projparse_utm` | `UTM zone 1N`, TM 9807, longitude d'origine -177, faux nord 0 |
| `utm_south` | `io_projparse_utm_south` | `UTM zone 1S`, faux nord 10000000 |
| `etmerc` | `io_projparse_etmerc` | méthode TM 9807, chaîne `+proj=tmerc ...`, pas d'`EXTENSION PROJ4` en WKT1 |
| `laea_north_pole` | `io_projparse_laea_north_pole` | axes `(E)` et `(N)` `south` à `lat_0=90` |
| `laea_south_pole` | `io_projparse_laea_south_pole` | axes `north` à `lat_0=-90` |
| `laea_ellipsoidal` | `io_projparse_laea_ellipsoidal` | méthode 9820 |
| `laea_spherical_on_ellipsoid` | `io_projparse_laea_spherical_on_ellipsoid` | méthode 1027 (`+R_A`), aller-retour exact |
| `eqc_ellipsoidal` | `io_projparse_eqc_ellipsoidal` | méthode 1028 |
| `ortho_ellipsoidal` | `io_projparse_ortho_ellipsoidal` | méthode 9840, aller-retour exact |
| `ortho_with_alpha` | `io_projparse_ortho_with_alpha` | `Local Orthographic` 1130, aller-retour exact |
| `ortho_with_scale` | `io_projparse_ortho_with_scale` | `Local Orthographic` 1130, `+k=0.9`, aller-retour exact |
| `ortho_spherical_on_ellipsoid` | `io_projparse_ortho_spherical_on_ellipsoid` | nom `Orthographic (Spherical)`, aller-retour exact |
| `ortho_spherical_on_sphere` | `io_projparse_ortho_spherical_on_sphere` | `+R=`, aller-retour exact |
| `peirce_q_square` | `io_projparse_peirce_q_square` | `+lat_0=90` par défaut, `+k_0=1`, chaîne exacte |
| `peirce_q_diamond` | `io_projparse_peirce_q_diamond` | idem, `+shape=diamond` |
| `peirce_q_invalid_lat_0` | `io_projparse_peirce_q_invalid_lat_0` | `+lat_0=0` : l'export échoue |
| `peirce_q_invalid_k_0` | `io_projparse_peirce_q_invalid_k_0` | `+k_0=0.5` : l'export échoue |

Un test Rust de plus, hors ledger, n'est pas un `TEST()` de PROJ :
`io_projfam_sphere_selects_spherical_method` vérifie le choix de méthode
d'un `laea` et d'un `eqc` sur sphère (`+R=`), qui n'est couvert par aucun
autre test ; il ne compte pas dans `docs/UNPORTED-UNIT.md`.

## 2. Tentés, ouverts, avec la cause

Chaque test ouvert a été lancé et son échec relevé avant de rester hors du
fichier. Le fichier ne contient que les tests qui passent.

### A. `tmerc_approx` (`+proj=tmerc +approx`)

L'import refuse `approx` : `unsupported +proj=tmerc for a CRS`, parce que la clé
n'appartient à aucune méthode (`covers`, `projected_from_proj_string`). Le
modèle `ProjectedCRS` ne garde pas le drapeau. `Conversion::export_to_proj_string_approx_tmerc`
existe (`buildop.rs`), mais `ProjectedCRS::export_with` ne l'appelle pas.
Il faut un drapeau `approx` dans le modèle du CRS (`model.rs`) : trop large
pour ce lot.

### B. `laea_spherical` et `eqc_spherical`

La partie méthode passe après le correctif de la section 3 : `laea +R=` est
la méthode 1027, `eqc +R=` la méthode 1029. Reste l'assertion
`isEquivalentTo(EQUIVALENT)` entre ce CRS et un CRS construit avec la méthode
ellipsoïdale. `ProjectedCRS::is_equivalent_to` (`src/iso19111/equiv.rs:249`)
est le seul critère, `Strict` : il exige le même nom et la même méthode. Il
n'existe pas de critère `Equivalent` pour un CRS projeté, et le porter est
plus qu'un petit correctif. Le test est donc retiré ; la partie méthode est
couverte par `io_projfam_sphere_selects_spherical_method`.

### C. `peirce_q_horizontal` (`+shape=horizontal`)

L'import refuse : `unsupported +proj=peirce_q for a CRS`. La table n'a que les
méthodes `Peirce Quincuncial (Square)` et `(Diamond)` (`parammap.rs`, autour
des lignes 2846 et 2890). La chaîne attendue n'a ni `+lat_0` ni `+k_0`, forme
que le modèle ne sait pas exporter.

### D. `ob_tran_longlat`

`+proj=pipeline ...` n'est pas importable comme CRS : `unsupported +proj=pipeline
for a CRS`. PROJ en fait un `DerivedGeographicCRS` ; la crate n'a que
`DerivedProjectedCRS`.

### E. `ob_tran_rhealpix`

`+proj=ob_tran +o_proj=rhealpix ...` n'est pas importable : `unsupported
+proj=ob_tran for a CRS`. Aucune méthode `rhealpix` n'est dans la table CRS.
La projection existe côté opérations (`src/projections/ob_tran.rs`,
`healpix.rs`), pas côté CRS.

## 3. Changements dans `src`

Chaque changement répond à un test du lot. Les quatre retouches restent
locales, et la logique est dans un module nouveau.

1. **`src/iso19111/projfam.rs`** (nouveau) : `utm_projected`, `spherical_method_code`,
   `projected_cs`, `default_value`, `check_peirce_q_export`.
   - UTM : `+proj=utm +zone=N [+south]` devient une TM (`tmerc`) pour le datum,
     et la conversion est `Conversion::create_utm_north` ou `_south` (déjà
     présents dans `buildop.rs`, `UTM zone NN<N|S>`).
   - Méthode sphérique : un ellipsoïde sphérique (aplatissement 0) prend
     `laea` 1027 et `eqc` 1029.
   - Axes polaires : `laea` à `lat_0=±90` a ses axes `south` (pôle nord) ou
     `north` (pôle sud).
   - `peirce_q` : `+lat_0` vaut 90 par défaut.
   - Export `peirce_q` : refuse une latitude de centre qui n'est pas ±90, ou
     une échelle qui n'est pas 1.
2. **`src/iso19111/mod.rs`** : `mod projfam;`.
3. **`src/iso19111/projstring.rs`** :
   - `projected_from_proj_string` passe en `pub(super)` et renvoie
     `projfam::utm_projected` pour `+proj=utm` (en premier, avant les unités).
   - `method_spelling` : `etmerc` devient `tmerc`.
   - Choix de méthode : `projfam::spherical_method_code` passe avant
     `forced_method_code`, après `proj_aux`.
   - Système de coordonnées : `projfam::projected_cs` au lieu de
     `CartesianCS::create_easting_northing` (axes polaires `laea`).
   - `parameter_default` : premier appel à `projfam::default_value`.
   - Import : `CartesianCS` ne sert plus, retiré.
4. **`src/iso19111/buildop.rs`**, `export_to_proj_inner` : appel de
   `check_peirce_q_export` avant la branche `9808`.

Effets de bord voulus, à relire :

- `+proj=laea +R=...` sort la méthode 1027 (avant : 9820). `+proj=eqc +R=...`
  sort 1029 (avant : 1028). Aucun test existant ne le contredisait.
- `+proj=laea +lat_0=±90` a des axes `south` ou `north` (avant : `east`,
  `north`).
- `+proj=peirce_q` sans `+lat_0` a `+lat_0=90` (avant : 0).
- Un `peirce_q` dont l'échelle est absente du modèle vaut 0 et refuse l'export.
  C'est le même chemin que pour un `+k_0` nul.

## 4. Ledger et compteurs

- `docs/UNPORTED-UNIT.md` : 17 lignes retirées de `## test_io.cpp`.
- `docs/UNIT-TRIAGE.md` : régénéré (`UPDATE_DOCS=1 cargo test --test docs_in_sync`).
  `test_io.cpp` 254 → 237 ouverts, `core` 698 → 681.
- `docs/progress.svg` : régénéré (`tests/progress_svg.rs`).
- `README.md` et `ROADMAP.md` : **non modifiés**, comme demandé. Leurs
  compteurs ne correspondent plus au ledger. Le test
  `counts_in_docs_match_the_ledgers` échoue donc tant qu'ils ne sont pas
  régénérés (voir section 5).

## 5. Vérification

```
UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg   # ok (unit_triage, progress_chart)
git checkout README.md ROADMAP.md                                  # restaurés
cargo test --all-features --test test_io_projfam                   # ok, 18 passed (17 PROJ + 1 complément)
cargo test --all-features --no-fail-fast                           # un échec : counts_in_docs_match_the_ledgers
cargo clippy --all-targets --all-features -- -D warnings           # ok
cargo fmt --check                                                  # ok
```

Échec restant : `counts_in_docs_match_the_ledgers` (`tests/docs_in_sync.rs:285`).
Il passait quand `UPDATE_DOCS=1` avait réécrit les compteurs de `README.md`.
Le restaurer, comme demandé, rend le compteur du README inexact par rapport au
ledger. Pour le rétablir : régénérer `README.md` et `ROADMAP.md` dans un lot de
doc à part, avec `UPDATE_DOCS=1 cargo test --test docs_in_sync`.

Dans le reste de la suite, tous les binaires passent. `cli_yaml` n'a pas eu de
« Broken pipe ». `docs/UNPORTED-CLI.md` n'a pas changé : aucun cas CLI ne passe
de plus, et aucun cas qui passait ne régresse.
