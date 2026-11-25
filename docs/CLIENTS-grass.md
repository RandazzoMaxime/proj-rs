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
résultat de chaque test (réussi, échoué, erreur, ignoré) entre les deux passes, et échoue s'il en
diffère un seul.

### L'environnement que la suite attend

Le script met en place lui-même ce dont la suite a besoin, pour qu'un échec parle de libproj et
non de la machine :

- **FFTW** (`--with-fftw`, paquet `libfftw3-dev`) : `r.surf.fractal` l'exige, et beaucoup de tests
  génèrent leurs données avec lui. Le script vérifie que `configure` l'a trouvé.
- **Le Python du venv pour les modules aussi** : son `bin` passe en tête du `PATH`, donc les
  modules Python de GRASS (`t.create`, `g.extension`...) tournent avec lui. Il porte
  `psycopg2-binary` (le cadre temporel l'importe), `IPython` et `ipywidgets` (les tests de
  `grass.jupyter`, ignorés sans eux).
- **Un utilisateur sans privilège** : lancé en root, le script crée l'utilisateur système
  `grass-bench` et fait tourner `pytest` sous son identité (`setpriv`). Le test
  `r_pack_test.py::test_output_dir_is_not_writable` vérifie que `r.pack` refuse d'écrire dans un
  dossier en lecture seule, ce que root peut toujours faire. Si le paquet de certificats de
  l'environnement (`SSL_CERT_FILE`, `GIT_SSL_CAINFO`...) est illisible pour cet utilisateur, le
  script en pose une copie lisible.
- **Le réseau** : `g.extension` télécharge et compile les extensions `r.stream.distance` et
  `r.lake.series` depuis `github.com/OSGeo/grass-addons`.
- Les paquets système (flex, bison, GDAL, SQLite, FFTW...) sont installés par `apt-get` quand le
  script tourne en root sous Debian ou Ubuntu.

### Quelle libproj chaque passe charge

`tools/clients/libproj_proof.sh` le prouve, pour chaque passe :

- `ldd` résout `libproj.so.25` de `g.proj`, `r.proj` et `libgrass_gproj.so` vers le fichier de la
  passe, et aucun objet de leur chaîne n'a de `DT_RPATH` (cherché avant `LD_LIBRARY_PATH`) qui
  mènerait à une autre `libproj.so.25` ;
- un vrai appel (`grass --tmp-project EPSG:3358 --exec g.proj -jf`) tourne sous
  `LD_DEBUG=libs` : chaque `libproj` initialisée est le fichier de la passe ;
- l'intercepteur `tools/clients/proj_count.c` reste préchargé pendant toute la passe : chaque
  processus qui charge une libproj, au démarrage ou par `dlopen`, note son chemin, et la passe
  échoue si un seul en a chargé une autre.

## Résultat (GRASS 8.5.0, GDAL 3.8.4, Python 3.13.18, pytest 9.2.0, le 2026-11-25)

| | PROJ 9.9.0 | proj-rs |
|---|---:|---:|
| Tests réussis | 818 sur 825 | 818 sur 825 |
| Tests qui échouent ou ne démarrent pas | 0 | 0 |
| Tests ignorés | 7 | les mêmes 7 |

Chacun des 825 tests a le même résultat avec les deux bibliothèques.

Les 7 tests ignorés le sont par GRASS lui-même, sans condition :
`raster/r.mapcalc/tests/r_mapcalc_nprocs_test.py::test_rand_no_explicit_seed_setting`, pour ses
7 valeurs de `nprocs`, porte `@pytest.mark.skip(reason="Waiting for a fix to work on all
platforms for any nprocs")`. Aucun environnement ne les fait tourner.

Preuves de la dernière passe :

    ref: ldd g.proj: libproj.so.25 => .../grass/ref-prefix/lib/libproj.so.25
    ref: ldd r.proj: libproj.so.25 => .../grass/ref-prefix/lib/libproj.so.25
    ref: ldd libgrass_gproj.so: libproj.so.25 => .../grass/ref-prefix/lib/libproj.so.25
    ref: no DT_RPATH that could load another libproj
    ref: grass: +proj=lcc +lat_0=33.75 +lon_0=-79 +lat_1=36.1666666666667 +lat_2=34.3333333333333 +x_0=609601.22 +y_0=0 +ellps=GRS80 +units=m +no_defs +type=crs
    ref: LD_DEBUG=libs: calling init: .../grass/ref-prefix/lib/libproj.so.25
    ref: interposer: .../grass/ref-prefix/lib/libproj.so.25.9.9.0, proj_create_from_wkt=1/0
    ref: interposer: 5433 processes mapped a libproj at start, 129 called into it, all .../grass/ref-prefix/lib/libproj.so.25.9.9.0
    ours: ldd g.proj: libproj.so.25 => .../grass/ours/lib/libproj.so.25
    ours: ldd r.proj: libproj.so.25 => .../grass/ours/lib/libproj.so.25
    ours: ldd libgrass_gproj.so: libproj.so.25 => .../grass/ours/lib/libproj.so.25
    ours: no DT_RPATH that could load another libproj
    ours: grass: +proj=lcc +lat_0=33.75 +lon_0=-79 +lat_1=36.1666666666667 +lat_2=34.3333333333333 +x_0=609601.22 +y_0=0 +ellps=GRS80 +units=m +no_defs +type=crs
    ours: LD_DEBUG=libs: calling init: .../grass/ours/lib/libproj.so.25
    ours: interposer: .../grass/ours/lib/libproj.so.25, proj_create_from_wkt=1/0
    ours: interposer: 5416 processes mapped a libproj at start, 119 called into it, all .../grass/ours/lib/libproj.so.25

Le nombre de processus `g.proj` qui appellent `proj_create_from_database` varie d'un passage à
l'autre (56 avec PROJ et 54 avec proj-rs ici, 54 et 56 au passage précédent du même script) : `pytest-xdist` répartit les tests sur 8 processus selon leur durée, et
chaque processus qui reçoit un test d'un module refait la fixture de ce module, qui crée un projet
avec `g.proj`. La passe PROJ compte en plus 8 processus `cs2cs` : `m.proj` lance le `cs2cs` de
PROJ, lié à libproj ; celui de proj-rs est un binaire Rust qui ne charge pas libproj.

## Écart trouvé et corrigé

`raster/r.latlong/tests/test_r_latlong.py::test_latlong_projected_epsg_3358` différait au premier
passage du banc. `r.latlong` appelle `proj_create` avec
`+proj=pipeline +step +inv +proj=lcc ... +type=crs`. PROJ en fait une opération, et proj-rs la
refusait (`unsupported +proj=pipeline for a CRS`). Deux écarts étaient en cause :

1. **Pipeline refusé.** PROJ lit un pipeline marqué `+type=crs` comme un CRS seulement s'il en
   a la forme. Sinon, c'est une opération (`io.cpp:12924-12974`). proj-rs fait désormais pareil.
2. **Définition simplifiée à tort.** `proj_pj_info` réduisait tout pipeline d'une étape à cette
   étape. PROJ garde le pipeline quand l'étape est inversée, porte `omit_inv` ou `omit_fwd`, ou
   quand le pipeline a un paramètre global (`io.cpp:10045-10050`).

Le détail est dans `docs/PORTING-creation-refusal.md`. Les deux cas ont leurs tests, avec les
valeurs de PROJ 9.9.0, dans `tests/pipeline_type_crs.rs`.
