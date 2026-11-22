# GRASS GIS contre la bibliothèque C de proj-rs

GRASS GIS est un SIG complet (raster, vecteur, séries temporelles). Sa bibliothèque `libgis`
appelle libproj pour décrire et convertir les projections. `g.proj`, `r.proj`, `v.proj` et
`r.latlong` passent par elle. `m.proj` lance en plus le programme `cs2cs`.

## Méthode

`tools/clients/grass.sh` clone GRASS au tag `8.5.0` (commit épinglé). Il le compile une fois
contre PROJ 9.9.0 (la référence de `tools/build-reference.sh`), sans interface graphique. Il lance
ensuite la suite `pytest` de GRASS deux fois :

- avec PROJ 9.9.0 et son `cs2cs` ;
- avec notre `libproj`, chargée à sa place par `LD_LIBRARY_PATH` (même soname), et notre
  `cs2cs` sur le `PATH`.

Les tests marqués `needs_solo_run` tournent seuls, après les autres. Le script compare le
résultat de chaque test (réussi, échoué, erreur, ignoré) entre les deux passes.

## Résultat (GRASS 8.5.0 et GDAL 3.8.4, le 2026-11-22)

| | PROJ 9.9.0 | proj-rs |
|---|---:|---:|
| Tests réussis | 744 sur 818 | 744 sur 818 |
| Tests qui échouent ou ne démarrent pas | 65 | les mêmes 65 |
| Tests ignorés | 9 | les mêmes 9 |

Chacun des 818 tests a le même résultat avec les deux bibliothèques. Les 65 échecs communs
viennent de l'environnement de test, pas de libproj :

- 28 tests demandent `r.surf.fractal`, qui exige FFTW. Le script compile GRASS sans FFTW.
- 35 tests de séries temporelles échouent dès `t.create`. Les modules de GRASS tournent avec le
  `python3` du système, et le `psycopg2` de Debian ne s'y charge pas
  (`No module named 'psycopg2._psycopg'`).
- `g.extension` compile une extension téléchargée et échoue sur la page d'aide.
- `r.pack` attend un refus d'écriture qui n'arrive pas, car les tests tournent en root.

## Écart trouvé et corrigé

Au premier passage, un seul test différait :
`raster/r.latlong/tests/test_r_latlong.py::test_latlong_projected_epsg_3358`. `r.latlong`
appelle `proj_create` avec `+proj=pipeline +step +inv +proj=lcc ... +type=crs`. PROJ en fait une
opération, et proj-rs la refusait (`unsupported +proj=pipeline for a CRS`). Deux écarts étaient
en cause :

1. **Pipeline refusé.** PROJ lit un pipeline marqué `+type=crs` comme un CRS seulement s'il en
   a la forme. Sinon, c'est une opération (`io.cpp:12924-12974`). proj-rs fait désormais pareil.
2. **Définition simplifiée à tort.** `proj_pj_info` réduisait tout pipeline d'une étape à cette
   étape. PROJ garde le pipeline quand l'étape est inversée, porte `omit_inv` ou `omit_fwd`, ou
   quand le pipeline a un paramètre global (`io.cpp:10045-10050`).

Le détail est dans `docs/PORTING-creation-refusal.md`. Les deux cas ont leurs tests, avec les
valeurs de PROJ 9.9.0, dans `tests/pipeline_type_crs.rs`.
