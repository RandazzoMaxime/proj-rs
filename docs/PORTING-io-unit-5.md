# Port — `io.projparse_*` : projections non importées et noms de méthode (lot io-5)

Source : PROJ 9.9.0, `test/unit/test_io.cpp`, groupe `io.projparse_*`. Côté
crate : `tests/test_io.rs` (bloc en fin de fichier), `src/iso19111/projstring.rs`.

Ce lot porte **9 `TEST()`** et corrige la sélection de méthode à l'import d'une
chaîne PROJ. Il laisse **5 tests tentés** ouverts, avec la cause de chacun. Le
lot part de la branche `feat/pensive-allen-fkoqxo` (commit `b4dcf83`, io-4 et
triage inclus), et non de `main` : `docs/PORTING-io-unit-4.md`,
`docs/UNIT-TRIAGE.md` et `tests/docs_in_sync.rs` n'existent que sur cette
branche.

## 0. Ordre suivi

Dans l'ordre de la demande : geos, somerc et aeqd guam, cea, noms
`merc_stere`, krovak czech, merc sphérique puis Google Mercator. Les tests qui
exigent un objet lié (`+towgs84`, `+nadgrids`, `+geoidgrids`) ou `+vunits` ne
sont pas traités ici.

## 1. Portés (9)

| PROJ 9.9.0 `io.projparse_*` | PROJ-RS `tests/test_io.rs` | Ce qui est vérifié |
|---|---|---|
| `geos_sweep_y` | `io_projparse_geos_sweep_y` | `+proj=geos +h=1` nomme `Geostationary Satellite (Sweep Y)` |
| `geos_sweep_x` | `io_projparse_geos_sweep_x` | `+sweep=x` nomme `(Sweep X)` |
| `somerc` | `io_projparse_somerc` | `Hotine Oblique Mercator (variant B)` ID 9815, six paramètres, pas d'`EXTENSION` en WKT1 GDAL |
| `aeqd_guam` | `io_projparse_aeqd_guam` | `+guam` nomme `Guam Projection` |
| `cea_spherical_on_ellipsoid` | `io_projparse_cea_spherical_on_ellipsoid` | `+R_A` : `Lambert Cylindrical Equal Area (Spherical)` ID 9834, aller-retour de la chaîne |
| `cea_ellipsoidal_with_k_0` | `io_projparse_cea_ellipsoidal_with_k_0` | `+k_0=0.99` sur GRS80 donne `Latitude of 1st standard parallel` 8.1365… |
| `merc_stere` | `io_projparse_merc_stere` | `+proj=stere +lat_0=30` nomme `Stereographic`, sans ID |
| `merc_spherical_on_ellipsoid` | `io_projparse_merc_spherical_on_ellipsoid` | `+R_C` : `Mercator (Spherical)` ID 1026, aller-retour de la chaîne |
| `merc_stere_polar_k_and_lat_ts_incompatible` | `io_projparse_merc_stere_polar_k_and_lat_ts_incompatible` | `+lat_0=90 +lat_ts=70 +k=0.994` est refusé (PROJ lève `ParsingException`) |

Les noms de test Rust sont ceux de PROJ en minuscules. Les chaînes attendues
sont celles de PROJ, sans modification.

## 2. Tentés, ouverts, avec la cause

### A. Krovak « czech » (`krovak_czech`, `krovak_modified_czech`)

Le test attend `+proj=krovak +czech ...` à l'export, avec les axes
`westing` puis `southing`. L'import de `+czech` renvoie la méthode `Krovak`
(9819) avec des axes est-nord, et l'export écrit `+axis=swu`, qui vient du
`proj_aux` de la méthode.

PROJ ne distingue pas les deux par la méthode seule. Dans `krovak.cpp`,
`+czech` (`tczech`) met `easting_northing` à faux : la sortie est en
(westing, southing) sans permutation d'axes. `+axis=swu` est une permutation
d'axes après une sortie est-nord. Côté CRS, PROJ importe `+czech` avec des
axes westing/southing, et l'export choisit `+czech` d'après ces axes.

Pour passer, il faut : (1) importer `+czech` avec des axes westing/southing,
(2) à l'export, choisir `+czech` plutôt que `+axis=swu` pour une méthode
Krovak dont les axes sont westing/southing. Le point (2) touche l'export de
la conversion (`buildop.rs`, `push_aux`) et l'export du CRS projeté. Ce n'est
pas un correctif petit ni sûr sans les autres chemins d'export : laissé ouvert.

### B. Google Mercator (`merc_google_mercator`, `_non_metre_unit`, `_not_quite_google_mercator`)

Les trois tests attendent que l'aller-retour de la chaîne redonne
`+nadgrids=@null`. C'est un CRS lié (`Crs::Bound`), donc du ressort du lot qui
traite les objets liés. Sans lui, aucun des trois ne peut passer, même si le
choix de méthode était juste.

En plus, pour le premier et le troisième :

- `+proj=merc +a=6378137 +b=6378137 ... +nadgrids=@null` doit donner la méthode
  `Popular Visualisation Pseudo Mercator` (1024) et `DATUM["World Geodetic
  System 1984"]`. Dans la table (`parammap.rs`), la méthode 1024 a `proj_name`
  `webmerc`, et `+proj=merc` ne la sélectionne pas.

Pour `_non_metre_unit`, `+units=ft` est refusé par `projected_from_proj_string`
(`unsupported +units=ft`). Le test attend en plus le nom
`WGS 84 / Pseudo-Mercator (unit ft)`.

## 3. Changements dans `src`

Fichier : `src/iso19111/projstring.rs`. Seul l'import d'une chaîne PROJ change ;
l'export ne change pas.

1. **Sélection par `proj_aux`** (`aux_selects`). Une méthode dont le `proj_aux`
   est dans la chaîne (drapeau `+guam`, `+R_A`, `+R_C`, ou clé=valeur
   `+sweep=x`, `+axis=swu`) est choisie avant les autres. Une méthode qui a un
   `proj_aux` n'est plus retenue par défaut. Pourquoi : c'est la règle de
   PROJ (`buildProjectedCRS`, `proj_name_aux`). Avant, `+proj=geos +h=1` prenait
   `Sweep X`, puisque la méthode X vient en premier et que sa couverture des
   paramètres est complète : c'est le défaut `Sweep X` signalé.
2. **`stere` non polaire** : la méthode `Stereographic` (sans code EPSG) est
   choisie par son nom quand `+lat_0` n'est pas un pôle. Avant, la première
   méthode `stere` était `Polar Stereographic (variant A)`.
3. **`somerc`** est importé comme `omerc` avec `+alpha=90`, `+gamma=90` et
   `+lonc=lon_0`, comme `buildProjectedCRS` de PROJ, quand aucun de `alpha`,
   `gamma`, `lonc` n'est donné.
4. **`cea` avec `+k_0`** (sans `+lat_ts`) : `lat_ts` est calculé et `k` est
   retiré. Sur ellipsoïde, la racine de `cos(φ)/sqrt(1 - e² sin²φ) = k0` est
   trouvée par bissection (`standard_parallel_of_scale`). Avec `+R_A`,
   `lat_ts = acos(k0)`. Le résultat pour GRS80 et `k_0=0.99` est 8.136531°, ce
   que le test attend.
5. **Stéréographique polaire avec `+lat_ts` et `+k` ≠ 1** : erreur, comme PROJ.
6. L'aplatissement et la datum sont calculés avant la méthode, car `cea` en a
   besoin pour `e²`. Les erreurs de datum passent donc avant celles de méthode.

Taille : environ 140 lignes ajoutées dans `projstring.rs` (fonctions et
commentaires compris). Je n'ai pas trouvé de budget de lignes écrit dans le dépôt.

Aucun changement dans `buildop.rs`. Le nom `Geostationary Satellite (Sweep X)`
est bien écrit en dur dans `create_geostationary_satellite_sweep_x`, mais ce
constructeur n'est appelé par aucun import : la cause du défaut était le choix
de méthode de l'import (point 1), pas ce nom.

## 4. Ledger et compteurs

- `docs/UNPORTED-UNIT.md` : 9 lignes retirées de `## test_io.cpp`.
- `docs/UNIT-TRIAGE.md` : régénéré avec `UPDATE_DOCS=1 cargo test --test
  docs_in_sync`. `test_io.cpp` 273 → 264 ouverts, `core` 717 → 708.
- `README.md` et `ROADMAP.md` : non modifiés, comme demandé. Leurs compteurs
  ne correspondent donc plus au ledger (voir section 5).

## 5. Vérification

```
cargo test --test test_io io_projparse_                    # ok, 47 passed, 0 failed
cargo test --all-features --no-fail-fast                   # un échec, voir ci-dessous
cargo clippy --all-targets --all-features -- -D warnings   # ok, aucune sortie
cargo fmt --check                                          # ok
UPDATE_DOCS=1 cargo test --test docs_in_sync               # ok (triage régénéré)
git checkout README.md ROADMAP.md                          # restaurés
```

Échec restant : `tests/docs_in_sync.rs`, `counts_in_docs_match_the_ledgers`,
« README.md is out of date with the ledgers ». Ce test passait avant ce lot. Les
compteurs du README (`test_io.cpp` 89 → 107 portés, 365 → 347 ouverts, etc.)
ne sont pas à jour, puisque le README n'a pas été modifié par ce lot. Il faut
le régénérer après la réécriture du README et de la ROADMAP :
`UPDATE_DOCS=1 cargo test --test docs_in_sync`.

`cli_yaml` n'a pas changé `docs/UNPORTED-CLI.md` (aucun cas CLI ne passe de
plus).

## 6. Non tentés dans ce lot

- `io.projparse_tmerc_south_oriented` (`+axis=wsu`) : reste ouvert (groupe H et
  L de io-4). Par construction, la sélection par `proj_aux` (point 1 de la
  section 3) fait désormais choisir à `+axis=wsu` la méthode dont le `proj_aux`
  est `axis=wsu`. Le test reste ouvert : sa seconde moitié est un pipeline
  (groupe L), que ce lot ne traite pas.
- `io.projparse_merc_google_mercator_*` et `krovak_*_czech` : voir section 2.
