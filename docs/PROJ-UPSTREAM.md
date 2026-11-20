# Suivre les versions de PROJ

proj-rs embarque la base `proj.db` de PROJ 9.9.0. Cette base porte le registre EPSG : les CRS, les
datums, les transformations et leurs zones d'usage. L'EPSG publie des corrections plusieurs fois
par an, et PROJ les livre dans ses versions (une mineure par semestre, des correctifs entre les
deux). Une base qui ne bouge plus se périme : une transformation corrigée ou ajoutée par l'EPSG
n'arrive jamais chez l'utilisateur. Ce document dit comment la base suit PROJ.

## Le partage : automatique pour la base, à la main pour le code

Régénérer la base est mécanique : `data/proj.db` est exactement la base qu'écrit la compilation
de PROJ à sa version (vérifié pour 9.9.0, table par table). Une machine le fait mieux qu'une
personne, et le refait chaque semaine sans oubli.

Porter le code d'une nouvelle version ne l'est pas. Un correctif de calcul, un changement de
sélection des opérations ou un nouveau test `gie` demande de lire PROJ et de juger. Ce travail
reste un lot, comme le lot 8 qui a déplacé la référence vers 9.9 (`ROADMAP.md`). L'automatisation
ouvre ce lot au bon moment, avec un rapport, au lieu de le faire.

## Ce que fait le workflow (`.github/workflows/proj-upstream.yml`)

Chaque lundi, il :

1. Lit la version de PROJ d'où vient `data/proj.db` (clé `PROJ.VERSION` de la table `metadata`,
   `tools/proj-db/meta.py`) et la dernière version finale de PROJ (tags git de
   `OSGeo/PROJ`, sans les RC).
2. S'arrête si la version publiée n'est pas plus récente, ou si une issue `proj-upstream` existe
   déjà pour elle.
3. Sinon, compile PROJ à ce tag (`tools/proj-db/update.sh`, qui appelle
   `tools/build-reference.sh` avec TIFF et curl, comme les distributions) et remplace
   `data/proj.db` par la base qu'il écrit.
4. Lance toute la suite de tests, avec les binaires de la nouvelle version sur `PATH` : les tests
   différentiels (`agree_cs2cs`, les corpus de parité, les cas des outils) comparent donc à la
   nouvelle version, pas à 9.9.0.
5. Pousse la base sur la branche `auto/proj-db-<tag>`.
6. Ouvre une issue `PROJ <tag> : <état>`, étiquetée `proj-upstream`, avec le rapport de
   `tools/proj-db/report.sh` :
   - les métadonnées qui changent (EPSG, schéma) et, table par table, les lignes ajoutées et
     retirées ;
   - le résultat des tests et la liste de ceux qui échouent ;
   - les fichiers `.gie` de PROJ qui diffèrent de `tests/gie/` ;
   - le lien pour ouvrir la PR, et la liste de ce qui reste à faire.

Si une étape casse (compilation de PROJ, dépendance absente), l'issue le dit et pointe vers
l'exécution. Si la **version majeure du schéma** change (`DATABASE.LAYOUT.VERSION.MAJOR`), la base
n'est pas remplacée : proj-rs lit l'ancien schéma, il faut d'abord porter le code de lecture.
L'issue dit « bloqué ».

Autres déclenchements :

- **Lancement manuel** (onglet Actions, « PROJ upstream », champ `tag`) : essaie ce tag. Sur un
  tag qui n'est pas plus récent, c'est un essai à blanc : il ne pousse rien et n'ouvre pas
  d'issue. Le rapport reste dans le résumé de l'exécution.
- **Pull request** qui touche le workflow ou `tools/proj-db/` : essai à blanc sur la version
  déjà embarquée, pour vérifier la chaîne sans rien publier.

Le workflow n'ouvre pas la PR lui-même : une PR ouverte par le jeton du workflow ne déclenche
pas la CI. L'issue donne le lien ; une personne l'ouvre et la CI tourne normalement.

## À la réception d'une issue `proj-upstream`

Un lot par version de PROJ, dans cet ordre :

1. **Ouvrir la PR de la base** depuis `auto/proj-db-<tag>` (lien dans l'issue).
2. **Trier les tests en échec.** Un échec n'est pas forcément un bug de proj-rs : un test qui fige
   une sortie de PROJ 9.9.0 (une liste de candidats, un texte de `projinfo`) change légitimement
   quand l'EPSG change. Comparer avec la sortie de la nouvelle version, que
   `~/.cache/proj-rs/proj-reference-<tag>/build/bin` contient après `tools/proj-db/update.sh <tag>`. Une
   valeur attendue se reprend de PROJ, jamais du code testé (`CLAUDE.md`).
3. **Lire le NEWS de PROJ.** Un correctif de calcul ou de sélection se porte dans un lot à part,
   avec ses tests d'abord.
4. **Revendoriser les fichiers `.gie`** que l'issue liste, et mettre à jour
   `tests/gie/passing.txt`.
5. **Déplacer la référence** vers la nouvelle version, dans le même lot que la base :
   - `tools/build-reference.sh` : la valeur par défaut de `TAG` ;
   - `.github/workflows/ci.yml` : `proj=<tag>` de conda-forge (deux endroits), les clés de cache
     `proj-grids-…-<tag>` et `proj-reference-<tag>`, la branche du clone de `capi-windows` ;
   - `capi/include/*.h` : les en-têtes de la nouvelle version (`PROJ_VERSION_*` compris), si
     l'API C change ;
   - les constantes `RELEASE` des outils (`grep -rn 'const RELEASE' src`) ;
   - les mentions de la version dans `README.md`, `ROADMAP.md`, `CLAUDE.md`, `NOTICE.md`
     (`grep -rn '9\.9\.0' --include=*.md .`).
6. **Version de proj-rs** : une base plus récente change des résultats visibles. C'est une
   version mineure (`CHANGELOG.md`).

## En local

```bash
tools/proj-db/update.sh 9.10.0                 # compile PROJ 9.10.0, remplace data/proj.db
REF=~/.cache/proj-rs/proj-reference-9.10.0     # hors du dépôt : voir plus bas
cat $REF/proj-db-diff.md
PATH=$REF/build/bin:$PATH PROJ_CS2CS_REQUIRED=1 PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES \
  cargo test --workspace --all-features --no-fail-fast 2>&1 | tee tests.log
tools/proj-db/report.sh 9.10.0 9.9.0 $REF 0 tests.log
```

La référence se compile hors du dépôt : les tests qui comparent à `cs2cs` ignorent les dossiers de
`PATH` situés dans le dépôt, où cargo met le `cs2cs` de proj-rs lui-même. Une référence compilée
dans `target/` ne serait donc pas vue, et ces tests échoueraient (« no PROJ cs2cs is on PATH »).

Compiler PROJ demande cmake, un compilateur C++17 et les en-têtes de SQLite (`build-reference.sh`).
`tools/proj-db/diff.py <ancienne> <nouvelle>` compare deux bases quelconques ; il sort avec le code
0 si elles ont le même contenu, 1 sinon, comme `diff`. Régénérer la même version ne touche pas
`data/proj.db` : le fichier ne différerait qu'en octets.

## Essai de la chaîne

PROJ 9.9.0 est la dernière version publiée au moment d'écrire ce document. La chaîne a donc été
essayée à l'envers, en régénérant depuis PROJ 9.8.1 (`tools/proj-db/update.sh 9.8.1`), puis en
lançant toute la suite contre les binaires 9.8.1 :

- Le rapport voit le schéma qui change : `DATABASE.LAYOUT.VERSION.MINOR` passe de 7 à 6, et la
  table `derived_projected_crs` est retirée. Il voit aussi l'EPSG qui recule (v13.102 → v12.029),
  et des milliers de lignes ajoutées et retirées.
- La suite donne 1 328 tests réussis et 1 287 en échec. Les messages les plus fréquents du
  rapport en donnent la cause en une ligne : proj-rs refuse une base dont le schéma est plus ancien
  que le sien (« MINOR = 6 alors que ≥ 7 est attendu »), exactement comme PROJ 9.9.0. C'est un effet
  du sens inverse : une version plus récente porte un `MINOR` égal ou plus grand, que proj-rs
  accepte, comme PROJ.
- Le rapport liste aussi les fichiers `.gie` qui diffèrent (`builtins.gie`, `epsg_grid.gie`).

Dans le sens normal, ce qu'on attend d'une nouvelle version est un `MINOR` qui monte, éventuellement
des colonnes ou des tables nouvelles que le code de lecture ignore, et des tests qui figent une
sortie de 9.9.0 (listes de candidats, texte de `projinfo`) à reprendre de la nouvelle version.
