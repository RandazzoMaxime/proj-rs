# Audit du lot cli-rest

Sources lues (lecture seule) : `target/proj-reference/src/test/cli/` (`CMakeLists.txt`,
`test_cs2cs_datumfile.yaml`, `test_cs2cs_ntv2.yaml`, `test_cs2cs_ignf.yaml`,
`test_cs2cs_flaky.yaml`, `test_projinfo.sh`, `test_projinfo_out.dist`), `src/apps/`
(`proj.cpp`, `geod.cpp`, `geod_set.cpp`, `cs2cs.cpp`, `utils.cpp`), `src/init.cpp`,
`src/trans.cpp`, `src/grids.cpp`, `src/ellps.cpp`, `src/list.cpp`, `src/iso19111/c_api.cpp`
(`proj_get_units_from_database`), `src/iso19111/factory.cpp` (`getUnitList`).

Erratum du lot cli-geod : `docs/AUDIT-cli-geod.md` disait que `docs/UNPORTED-CLI.md` ne
contenait plus aucune ligne et que `test_proj_nad83.yaml` passait. C'était faux : 225 cas
de nad83 étaient listés et échouaient ; le test passait parce que la liste les excusait.
Cause : `+init=nad83:5002` ne lisait que la première ligne de la section `<5002>`
(`dictionary_section`). Corrigé dans ce lot (`src/crs.rs`).

## Fichiers de test de PROJ 9.9.0

| Source PROJ | Cas | Test Rust | Résultat |
|---|---|---|---|
| `test_proj_nad83.yaml` | 225 | `tests/cli_yaml.rs` (`FILES`) | passe (225 corrigés, `UNPORTED-CLI` vide) |
| `test_proj_nad27.yaml`, `test_invproj.yaml` | 265, 1 | idem | passe |
| `test_cs2cs_ntv2.yaml` | 6 | idem | passe (NTv1 `ntv1_can.dat` : cas 2) |
| `test_cs2cs_flaky.yaml` | 2 | idem | passe |
| `test_cs2cs_ignf.yaml` | 34 | idem | 32 passent ; cas 9 et 17 ouverts (voir PORTING) |
| `test_cs2cs_datumfile.yaml` | 12 | idem | 11 passent ; cas 3 (`--bbox`) ouvert |
| `test_projinfo.sh` + `test_projinfo_out.dist` | script | `tests/cli_yaml.rs::projinfo_aux_script` | passe (`PROJ_AUX_DB` porté) |
| `test_cs2cs_locale.sh` | 6 | `tests/cli_yaml.rs::cs2cs_locale_script` | passe (inchangé) |

Cas ouverts, avec la cause exacte :

| Cas | Cause |
|---|---|
| (aucun) | `test_cs2cs_ignf.yaml#9`, `#17` et `test_cs2cs_datumfile.yaml#3` sont portés : les causes relevées ici (candidat `towgs84` du CRS lié ; chemin `facade` et `--area`) étaient fausses. Voir `docs/PORTING-cs2cs-ignf-area.md`. |

## Écarts de `proj`, `geod` et `projinfo` relevés

| Écart | Statut |
|---|---|
| `proj -l`, `-lp`, `-lP`, `-l=ID`, `-le`, `-lu` ; `geod -l`, `-le`, `-lu` | Porté, sortie identique octet à octet au binaire de référence. |
| `geod +units=` (distance) | Porté. La clé est `units` : `pj_param(..., "sunits")` lit la clé `units`. |
| `proj -d -e -m -r -s -i -o -b` | Porté (`docs/PORTING-cli-rest.md`). Comparé au binaire de référence sur 27 combinaisons. |
| `proj -S` omega à la latitude -80 | Écart d'environ 5e-7 rad, reste ouvert (voir PORTING). |
| `proj -b/-i/-o` : derniers bits | Écart d'un ou deux ulps, reste ouvert (voir PORTING). |
| `proj -v`, `-V` (dump du moniteur) | Non porté : sortie interne de PROJ (`pj_pr_list`, `vprocess`). |
| Texte d'erreur (`emess`) | Non comparé par les yaml ; le format court de ce port diffère. |
