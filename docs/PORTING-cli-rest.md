# Port — CLI `cs2cs`, `proj`, `geod` et `projinfo` : yaml restants et listes

Sources : PROJ 9.9.0 `src/apps/proj.cpp`, `geod.cpp`, `geod_set.cpp`, `cs2cs.cpp` ;
`src/init.cpp`, `src/trans.cpp`, `src/grids.cpp`, `src/ellps.cpp`, `src/list.cpp`, `src/pr_list.cpp`, `src/param.cpp`, `src/ell_set.cpp`, `src/datum_set.cpp`, `src/datums.cpp`, `src/apps/utils.cpp` (`limited_fprintf_for_number`),
`src/projections/*.cpp` (`PROJ_HEAD`) ; `src/iso19111/c_api.cpp` et `factory.cpp`
(`getUnitList`) ; `src/test/cli/` (yaml et `test_projinfo.sh`). Audit :
[AUDIT-cli-rest.md](AUDIT-cli-rest.md).

## Carte

| Fichier | Rôle |
|---|---|
| `src/crs.rs` | `section_words` : une section `<n> ... <>` d'un fichier `+init=` est lue en entier ; `ignf_definition` : `+init=IGNF:CODE` depuis proj.db. |
| `src/engine.rs` | `split_quoted` : un mot entre guillemets reste un mot (`+nadgrids="dir with space/myconus"`). `Params` : la liste de jetons de `paralist`, avec le marquage « lu ». `Operation::parameters`, `ParamUse`. |
| `src/projections/legacy.rs` | Le nom de grille d'un `+datum=` avec espaces est cité dans la chaîne reconstruite. |
| `src/projections/gridshift/ntv1.rs` | Lecteur NTv1 (`.dat`), port de `NTv1Grid::open` et `valueAt`. |
| `src/projdb.rs` | `PROJ_AUX_DB` : bases auxiliaires attachées et lignes copiées dans les tables. |
| `src/listing.rs` | Données publiques : `PROJECTIONS` (192), `ELLIPSOIDS` (46), `linear_units()` (proj.db). Générées depuis les sources de PROJ ; la table est la donnée, pas le générateur. |
| `src/bin/cs2cs.rs` | `init_side_as_code` (IGNF seulement), `init_authority`, `geoid_without_height`, `raw_words`, réunion des arguments `+` (`parse`), `check_epochs`. |
| `src/bin/support/proj_cli.rs` | `-d -e -m -r -s -i -o -b -l -v -V`, contrôles `fwd_prepare` de `pj_fwd`, `break_cs2cs_recursion` ajouté à la définition. |
| `src/bin/support/proj_monitor.rs` | `-v` : `pj_pr_list` et `pr_list` (72 colonnes). `-V` : figure de l'ellipsoïde, `vprocess` (un bloc par point). |
| `src/bin/support/geod_cli.rs` | `-l`, `-le`, `-lu`, `+units=`. |
| `src/bin/support/lists.rs` | Affichage des listes d'ellipsoïdes et d'unités, comme `proj.cpp` et `geod.cpp`. |
| `tests/cli_yaml.rs` | `FILES` : +6 yaml (argv, datumfile, epoch, ntv2, ignf, flaky) ; `projinfo_aux_script`. |
| `tests/cli_yaml/` | `test_cs2cs_*.yaml`, `test_projinfo_out.dist`. |
| `data/tests/dir with space/myconus` | Copie de `conus`, comme la règle CMake de PROJ (`src/data/CMakeLists.txt`). |

## Corrections de fond

1. **`+init=FILE:SECTION` lit toute la section.** `dictionary_section` ne prenait que la
   ligne `<n>` : `+init=nad83:5002` devenait `proj=tmerc datum=NAD83`, et tous les
   défauts (`lon_0`, `lat_0`, `k`, `x_0`) s'appliquaient. Corrige les 225 cas nad83.
2. **`+init=IGNF:CODE`** est lu dans proj.db comme `+init=epsg:` (`pj_get_init_items`).
3. **`cs2cs` et les CRS de base.** Un côté `+init=IGNF:CODE` passe à la sélection ISO sous
   la forme `IGNF:CODE`. Un côté `+init=epsg:CODE` passe à la sélection ISO sous la forme
   de son CRS de la base, avec les axes forcés (`io.cpp` 12514-12640, règles PROJ 4 que
   `cs2cs` active). Il reste sur le chemin des chaînes PROJ s'il porte un paramètre autre
   que `over`, `no_defs`, `wktext`, `type=crs`, ou s'il est une projection sud-orientée
   (`init_side_south_oriented`). Passer les `epsg:` sans cet override a changé l'ordre des
   axes de `test_cs2cs_various.yaml#59`, `#67`, `#68` (régression mesurée, annulée) ; avec
   l'override ces cas restent verts. Fiche : `PORTING-cs2cs-init-towgs84.md`.
4. **`+geoidgrids` en destination** : la destination est un CRS composé (`instantiate_crs`),
   donc des nombres bruts, pas du DMS. Sans `--3d`, une source sans hauteur ne reçoit pas le
   géoïde (la source 2D n'a pas de hauteur à corriger) ; une source 3D (`+init=epsg:4979`) ou
   `--3d` le garde.
5. **Guillemets dans les chaînes PROJ.** `pj_mkparam_ws` garde les espaces d'une valeur
   entre guillemets. L'analyseur coupait sur les espaces ; il découpe maintenant hors des
   guillemets. `cs2cs` joint les arguments `+` par un blanc, sans les citer (`cs2cs.cpp`,
   690-700) : `"+proj=longlat +datum=WGS84"` en un argument vaut deux mots. Il ne passe pas
   une chaîne citée par la sélection ISO (son réimport coupe les espaces).
6. **NTv1.** Entête en big-endian, degrés, enregistrements d'est en ouest ; même convention
   de longitude que NTv2 (`compensateNTConvention`).
7. **Facteurs `proj -S` : dérivée du noyau unitaire.** `pj_factors` dérivait la sortie en
   mètres de `forward`, puis la divisait par `a`. Résultat : oméga faux de 5e-7 rad à
   `100 -80` (Mercator GRS80), et facteurs faux avec `+units=ft` (1,52 attendu, 4,99 donné à
   `2 49`, car la CLI n'appliquait pas `to_meter`). Le port dérive maintenant le noyau
   brut, `P->fwd` de `deriv.cpp`, via `UnitForward` : même pré-traitement (`lam0`, latitude
   géocentrique, pôle), même ordre des différences, sans `to_meter` ni `a`. `Job::factors`
   appelle `Operation::factors`. Le noyau de Mercator suit l'expression de `merc.cpp`
   (`asinh(sin/cos) - e atanh(e sin)`), et le méridien central d'UTM celle de `build_utm`.
   Mesure (`tests/cli_proj_factors.rs`, 8 cas écrits par la référence) : Mercator, LCC,
   stéréographique et Robinson sont identiques à la référence sur 88 points pris un par un.
   Le reste est dans « Écarts ouverts ».
8. **Époques sur un CRS statique.** `--s_epoch` et `--t_epoch` créent une métadonnée de
   coordonnées de leur côté (`check_epochs`, `src/bin/cs2cs.rs`). Un CRS statique la refuse
   (code 3, « Coordinate epoch should not be provided for a static CRS ») sauf si son CRS
   géodésique a des opérations de mouvement ponctuel. WGS 84 est dynamique : `EPSG:4326` et
   `EPSG:4978` acceptent une époque. Même règle que `CoordinateMetadata::create`.

9. **`proj -v` suit la liste de PROJ.** `Params` garde chaque jeton dans l'ordre, doublons
   compris. Une lecture marque le premier jeton de sa clé, comme `pj_param_exists`. Les
   ajouts vont en fin de liste : `+datum=` ajoute `ellps=` puis `towgs84=` (`datum_set.cpp`),
   et `ellps=GRS80` sans ellipsoïde (`append_default_ellipsoid_to_paralist`). Un jeton
   ajouté seulement pour la liste est caché des lectures. `pj_datum_set` lit `nadgrids`, sinon
   `towgs84`. `pj_ellipsoid` ne lit pas `ellps` quand `R` est là. La mesure : `tests/cli_proj_verbose.rs`.
10. **Facteurs `proj -S` des projections sphériques.** `pj_factors` lit `P->es`. Le setup
    de PROJ met `P->es = 0` pour les projections « Sph ». Le port gardait l'ellipsoïde de
    `+ellps=` : écart de 0,1 à 1 % sur le facteur d'aire. `Frame::factor_es` porte le `es` lu
    par `pj_factors` ; `projections::forces_sphere_factors` donne la liste (103 noms).
    Inventaire : `grep -n "es = 0" src/projections/*.cpp` dans les sources de PROJ 9.9.0,
    puis, pour chaque `PJ_PROJECTION`, le graphe d'appels vers ce setup. Un setup partagé
    (`sts`, `eck3`, `moll`, `sconics`, `gn_sinu`, `adams`, `mod_ster`, `aitoff`, `urmfps`,
    `nsper`) sert plusieurs projections. Exceptions : `sinu` (`Sph&Ell`) pose `es = 0` dans
    la branche `P->es == 0` seulement, donc reste ellipsoïdal ; `krovak`, `alsk`, `gs50`
    posent une constante non nulle ; `peirce_q` et `bertin1953` posent `factor_es` à 0 dans
    leur module (`peirce.rs`, `sphere_ops.rs`), donc hors liste. `gnom`, `gs48`, `lee_os`,
    `mil_os` ont une forward ellipsoïdale ou un rayon fixe, mais leur `es = 0` est
    inconditionnel : ils restent dans la liste. Un test « factors ignorent l'ellipsoïde »
    (`proj +ellps` contre `+R=a`) ne suffit pas pour ces quatre-là : il faut lire le setup.
    Mesure (`tests/cli_proj_factors.rs`, 93 cas de 3 points, `tests/cli_proj_factors/sphere.tsv`,
    écrits par `proj -S -f %.12f +ellps=WGS84` de la référence) : avant la liste, 91 sur 93
    rouges ; après, tous verts, les x et y à 1e-6 m près (écart `mbtfpp`, voir « Écarts ouverts »).
    Témoins ellipsoïdaux (`cea`, `eqc`, `healpix`, `laea`, hors liste) : leurs facteurs
    restent ellipsoïdaux et passent.
    Parité : le corpus n'a aucun cas `-S` sur ces projections ; rapport identique avant et après
    (4117 identiques, 64 dans le seuil, 1 des deux côtés, 54 hors seuil).
11. **rHEALPix ellipsoïdal : rayon authalique.** `healpix.cpp` pose `P->a = a · sqrt(qp / 2)`
    (lignes 623 et 667). Le port gardait `a` et multipliait le noyau par ce facteur : les
    dérivées étaient divisées par `a`, donc les facteurs étaient trop grands de `a / R_A`,
    soit 0,11 %. Le cadre porte maintenant le rayon authalique, et le noyau sort l'unité de
    PROJ. Mesure : facteurs `rhealpix` identiques (`tests/cli_proj_factors.rs`) ; `proj-fwd`
    `healpix` et `rhealpix` (6 conversions) deviennent identiques au texte. Parité : 6 écarts
    fermés.
12. **`geocent` et `cart` dans `proj`.** Le port construisait `geocent` comme `cart` :
    `proj +proj=geocent` rendait X, Y en mètres. Dans l'app `proj`, PROJ n'ajoute pas l'étape
    cartésienne : la définition porte `break_cs2cs_recursion` et `cs2cs_emulation_setup` sort
    tôt (`create.cpp:65`). `geocent` reste alors l'identité sur les radians (`geocent.cpp`), et
    la sortie passe par `fwd_finalize` (× `fr_meter`). Le port suit ce chemin quand le drapeau
    est là (`build_geocent`, `GeocentIdentity`). `cs2cs` et `cct` gardent la conversion 3D : sans
    le drapeau, PROJ l'ajoute par l'émulation. Facteurs `-S` : `pj_deriv` dérive `P->fwd` brut.
    Pour `cart`, ce brut est en mètres, sans division par `a` (`CartStep` expose
    `UnitForward`). Mesure : `tests/cli_proj_cart_geocent.rs` (9 cas écrits par la référence,
    comparaison exacte). Registre parité : 14 écarts retirés (`proj-fwd-0045`,
    `proj-inv-0035`, `proj-factors-0003` lignes 1 et 2). Hors seuil : `proj-fwd` 36 → 30,
    `proj-inv` 11 → 5, `proj-factors` 80 → 78.

## Écarts volontaires vers PROJ

1. **`+sunits=` n'existe pas dans PROJ** : `geod_set` lit `pj_param(..., "sunits")`, et
   `pj_param` prend la première lettre comme type (`s`) : la clé lue est `units`. Ce port lit
   `+units=`, comme PROJ. L'énoncé du lot disait `+sunits=` ; la sortie de référence avec
   `+sunits=` ignore l'unité.
2. **`PROJ_AUX_DB` par copie de lignes.** PROJ attache la base et ses vues interrogent les
   deux. Ici les lignes de la base auxiliaire sont copiées dans les tables principales
   (une ligne auxiliaire remplace la ligne de même clé). Une vue temporaire par table ne
   suffit pas : les vues de la base principale lisent les tables principales. `usage` est
   copiée en dernier (son déclencheur `usage_insert_trigger` vérifie les objets). `metadata`
   n'est pas copiée : chaque base contrôle son niveau de schéma.
3. **Contrôles `fwd_prepare` dans la CLI seulement.** `pj_fwd` refuse une latitude au-delà du
   pôle et une longitude au-delà de 10 rad. Ici seul `proj` le fait (`Job::project`) ;
   `Operation::forward` ne le fait pas. `cs2cs` et `cct` ont donc le même écart.
4. **Texte d'erreur d'`emess`.** Le port écrit `<prog>: message` ; PROJ écrit en plus l'en-tête
   `Rel. 9.9.0 ...` et la fin `program abnormally terminated`. Seul le code de sortie est
   comparé.

5. **Clés inconnues acceptées par `proj`.** PROJ accepte `+foo=bar` et le liste « non
   utilisé ». `Operation::create` refuse toujours une clé inconnue (`UnknownParameter`). La CLI
   passe par `create_projection_keeping_unknown` : `proj +foo=1` réussit, comme dans PROJ.
   Avant ce lot, la CLI refusait.

## Écarts ouverts

| Écart | Cause exacte |
|---|---|
| `proj -S` oméga au bit près, exact Tmerc et UTM zone 31 : 2 points sur 88 (tmerc `-120 60`, `-60 -85`), 3 sur 88 (UTM) | PROJ affiche un bruit de 1e-6 là où le port affiche 0, ou l'inverse : écart d'un ulp dans la dérivée numérique. Cause exacte non identifiée. Les coefficients de `setup_exact` (`pj_auxlat_coeffs`, `Qn`, `Zb`) ne sont pas comparés octet à octet ; `pj_auxlat_convert` avec l'ordre de PROJ n'a rien changé (86 et 85 avant et après). |
| `cs2cs +nadgrids=dir with space/myconus` en un argument sans guillemets (`-111 44`) | PROJ lit `with` et `space/myconus` comme paramètres sans `+`, ils restent inutilisés ; `dir` n'est pas trouvé : `*	* inf`, code 0. Le port rejette `+with` (`check_keys`, `src/engine.rs`) : code 3. Même cause que `+foo=1` sur `+proj=utm` : PROJ ignore une clé inconnue sur ce chemin, le port la refuse. Corriger `check_keys` touche tout le moteur : lot à part, avec un test par projection. |
| `proj -v` avec `+proj=pipeline` | Non porté : la liste d'un pipeline est vide ici, PROJ liste une projection. |
| `proj +datum=… +R=…` : valeur | PROJ prend la sphère de `R`. Ici la branche `datum` de `earth_from_params` prend l'ellipsoïde du datum : `merc +datum=WGS84 +R=6371000` donne 222638.98, PROJ 222389.85. La liste, elle, suit PROJ (`R` lu). Écart de valeur antérieur à ce lot. |
| `proj +no_defs` seul, `+rf=` sans `+a=` | PROJ refuse : `Must specify ellipsoid or sphere`. Ici GRS80 par défaut. Écart de validation antérieur à ce lot. |
| `gnom +ellps=` (forward, `-f %.12f`) | x diffère de 5e-10 m sur 2 points sur 6 (`proj-factors-0007`, lignes 2 et 6). Les facteurs sont identiques. Le forward ellipsoïdal (`gnom_e`, `geod`) n'est pas porté au bit près ; cause non identifiée. |
| `mbtfpp +ellps=WGS84` (forward, `-f %.12f`, `tests/cli_proj_factors/sphere.tsv`) | y diffère de 4,7e-10 m sur 1 point sur 3 ; les facteurs sont identiques. Ordre des opérations de la forward, cause non identifiée. La mesure tolère 1e-6 m sur x et y. |
| `aeqd +ellps=WGS84` (forward, `-f %.12f`) | x diffère de 4,6e-10 m sur 1 point sur 3 ; les facteurs sont identiques. Projection ellipsoïdale, hors liste sphérique : écart de forward non traité ici. Retirée des témoins de `cli_proj_factors`. |

## Sortie binaire `-b`, `-i`, `-o`

Mesure : `proj -b` et `proj -I -b` contre le binaire de référence, sur une grille de 101 × 51
points par projection, dans les deux sens, pour 11 projections (Mercator ellipsoïdal et sphérique,
`tmerc`, `utm` zone 31 nord et sud, `tmerc +approx`, `lcc`, `aea`, `laea`, `eqc`, `stere`) :
0 octet différent à l'époque de la mesure, avant `fmath` (voir `docs/PORTING-simd.md`). Avant : jusqu'à 22528 ulps (`utm` inverse), 1024 (`tmerc` inverse), 195 (`lcc`), 21 (Mercator).
Les grands nombres d'ulps tombent près de zéro, où un ulp est très petit.
`tests/cli_binary.rs` fige 16 cas et relit leurs doubles à la tolérance de `docs/PARITY.md` (1e-6 m, 1e-13 rad) : `fmath` s'écarte d'un ou deux ULP de la libm.

Causes corrigées :

- `atanh` : `f64::atanh` de Rust diffère de la libm sur environ 34 % des entrées. `projmath::atanh`
  appelle la libm C, comme PROJ, sauf sous 0,1 où `fmath::atanh_small` (série) la remplace.
- Aller-retour degrés / radians supprimé : Mercator, `tmerc`, `utm`, `tmerc +approx` et `lcc`
  passent par le cadre `Classic` (`fwd_prepare`, `fwd_finalize`).
- Aplatissement troisième `n = tan²(asin(e) / 2)` comme `pj_ell_set`, et non `f / (2 − f)`.
- Méridien central UTM `(zone + 0,5) π / 30 − π` comme `PJ_PROJECTION(utm)`, et non des degrés.
- Séries `tmerc` : `pj_auxlat_coeffs` et `pj_auxlat_convert` de `projmath` ; inverse à trois arguments.
- Newton de `sinhpsi2tanphi` : `e2m * (tau * tau)`, même association que PROJ.
- `aea` : pas de clamp sur `asin(qs / qp)`. PROJ rend NaN hors de [−1, 1].

Reste ouvert :

- Chemin par paquets (`forward_batch`, AVX2) : ses opérations vectorielles ne suivent pas PROJ
  bit à bit (mesure : [`PORTING-batch-path.md`](PORTING-batch-path.md)). La CLI ne l'emploie pas.
- `+algo=auto`, `tmerc` sphérique et `+over` : non mesurés.
- `sinhpsi2tanphi` non convergent : PROJ rend la dernière valeur et pose `errno`, ce port
  renvoie `OutsideDomain`. Non rencontré dans la grille.

## Commandes

```bash
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
R=/chemin/vers/target/proj-reference
PROJ_RS_LIBPROJ_DIR=$R/build/lib cargo test --test cli_yaml     # yaml et projinfo_aux_script
PROJ_RS_LIBPROJ_DIR=$R/build/lib cargo test --test cli_proj_verbose  # proj -v, -V (sorties de référence)
```

Comparaison à la main avec le binaire de référence, sur les mêmes arguments :

```bash
PROJ_DATA=$R/build/data:$PWD/data:$PWD/data/tests LD_LIBRARY_PATH=$R/build/lib \
  $R/build/bin/proj -l
```
