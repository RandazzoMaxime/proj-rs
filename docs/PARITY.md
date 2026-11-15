# Suite de parité contre PROJ 9.9.0

But : garder, dans le dépôt, un filet de sécurité de plusieurs milliers de
conversions comparées à PROJ 9.9.0. La phase 2 réarchitecte le calcul pour la
performance. Elle doit rester précise : la suite dit tout écart de sortie.

La suite ne dépend plus d'un PROJ installé. Le corpus et la sortie de référence
sont dans le dépôt.

## Fichiers

| Fichier | Rôle |
|---|---|
| `tools/parity/generate.py` | Tire les entrées (graine fixe) et écrit `tests/parity/cases.jsonl`. |
| `tools/parity/record.sh`, `record.py` | Lance PROJ 9.9.0 sur chaque invocation et écrit `tests/parity/expected.jsonl`. |
| `tests/parity/cases.jsonl` | Une invocation par ligne : outil, arguments, lignes d'entrée, genre de chaque champ. |
| `tests/parity/expected.jsonl` | Code de sortie et sortie standard de la référence, ligne à ligne. |
| `tests/parity/known.tsv` | Écarts connus : clé `id:ligne`, cause courte, renvoi à une fiche. |
| `tests/parity.rs` | Relance nos binaires `cs2cs`, `proj`, `cct` et compare. |

## Commandes

Depuis la racine du dépôt, après `cargo build --bins` :

```sh
# Régénérer le corpus (pas besoin de PROJ pour lire le résultat ; la référence sert aux entrées)
PROJ_REFERENCE_DIR=/chemin/proj-reference python3 tools/parity/generate.py

# Réenregistrer la référence
PROJ_REFERENCE_DIR=/chemin/proj-reference tools/parity/record.sh

# Lancer la suite, avec le résumé et le rapport détaillé
PARITY_REPORT=/tmp/parity.tsv cargo test --test parity -- --nocapture
```

`PROJ_REFERENCE_DIR` désigne un répertoire contenant `build/bin` et `build/data`
de PROJ 9.9.0. Sans cette variable, c'est `target/proj-reference`.

Le rapport `PARITY_REPORT` contient une ligne par écart : clé, famille, classe,
connu ou non, détail, et la commande de reproduction.

La suite tourne en environ deux minutes en debug sur quatre cœurs.

## Ce qui est comparé

Chaque ligne de sortie est comparée à la ligne attendue, champ par champ.

| Classe | Règle | Échec ? |
|---|---|---|
| Identique | Texte identique. | Non (compté) |
| Dans le seuil | Texte différent, valeurs dans le seuil. | Non (compté) |
| Erreur des deux côtés | `*`, `inf` ou `nan` des deux côtés. | Non (compté) |
| Statut | Erreur d'un seul côté, ou code de sortie nul d'un seul côté. | Oui |
| Forme | Nombre de champs ou de lignes différent. | Oui |
| Valeur | Champ numérique hors seuil. | Oui |

Seuils, par genre de champ (`kinds` dans `cases.jsonl`) :

- mètres (`m`) : 5e-4 m, soit 0,5 mm. C'est le défaut de `gie`.
- degrés (`deg`) : 5e-9 degré, soit environ 0,5 mm à l'équateur.
- temps (`t`) : 5e-9, en années décimales.
- facteurs d'échelle de `proj -S` (`fac`) : 1e-5 relatif. `proj -S` imprime
  avec `%g` (six chiffres). Un arrondi du sixième chiffre donne un écart relatif
  d'environ 7e-6. Le seuil du plan (1e-6) échouerait donc sur de l'arrondi.
  Près de zéro, le seuil devient absolu (1e-5) : la distorsion angulaire vaut
  0 d'un côté et 1e-6 degré de l'autre quand le bruit des différences finies de
  `pj_deriv` tombe autrement (`krovak`, `omerc`).
- texte (`text`) : la ligne entière, à l'identique. Sert aux listes de candidats
  de `projinfo --summary` (famille `projinfo-candidates` du gros corpus).

`cs2cs` écrit toujours trois valeurs (z = 0 sans hauteur), quatre avec le temps.
Le corpus en tient compte.

## Registre des écarts

`known.tsv` liste les écarts connus. Règles :

- un écart listé ne fait pas échouer la suite ;
- un écart nouveau fait échouer la suite ;
- un écart listé qui a disparu fait échouer la suite : retirer sa ligne.

`PARITY_UPDATE_KNOWN=1 cargo test --test parity` réécrit le registre avec les
écarts observés. À n'utiliser qu'en connaissance de cause : les causes non
renseignées s'écrivent « à classer ».

## Seuil d'alerte

Si plus de 5 % des conversions sont hors seuil, s'arrêter et rapporter avant de
figer un nouveau corpus. Le premier passage est à 4,4 %.

## Ajouter une famille

1. Ajouter une fonction `fam_*` dans `tools/parity/generate.py` et l'appeler dans `build`.
2. Donner à chaque invocation le genre de chaque champ de sortie (`kinds`).
3. Régénérer `cases.jsonl`, réenregistrer `expected.jsonl`, relancer la suite.
4. Classer chaque nouvel écart : grille absente d'un côté, bug du port, écart
   volontaire documenté dans `docs/PORTING-*.md`, ou bruit d'arrondi.

Le générateur écarte les paires qui exigent une grille absente (`projinfo` :
« grid missing ») et les invocations sans résultat exploitable.

## Premier passage

Corpus de graine `20261115` : 710 invocations, 4236 conversions.

| Classe | Conversions |
|---|---|
| Identique (texte) | 3974 |
| Dans le seuil | 68 |
| Erreur des deux côtés | 1 |
| Hors seuil | 187 (4,4 %) |
| Statut (code de sortie) | 1 invocation |
| Forme (lignes absentes) | 6 |

Les 194 clés du registre (187 valeurs, 6 formes, 1 statut) portent chacune une
cause. Le port n'est pas corrigé dans ce lot : c'est la phase de correction.

## État actuel

Après les lots de correction (P1 à P4, voir `docs/PORTING-cs2cs-pivot-candidates.md`,
`docs/PORTING-mod-ster.md`, `docs/PORTING-cli-rest.md`) : 4171 identiques (texte),
64 dans le seuil, 1 erreur des deux côtés, 0 hors seuil, 0 statut, 0 forme. Le registre
`known.tsv` ne contient plus aucun écart.

Les 64 « dans le seuil » sont des derniers chiffres : le plus grand écart est sous 1e-9 m,
six ordres de grandeur sous le seuil de 5e-4 m. Ils ne font pas échouer le test.

## Gros corpus (`tests/parity/wide`)

Graine `20261116`, échelle 6, 12 points par invocation, 15 % des points tirés dans le
monde entier (hors emprise du CRS compris), 1000 paires `projinfo --summary` :

```sh
python3 tools/parity/generate.py --seed 20261116 --scale 6 --points 12 --world 0.15 \
  --projinfo 1000 --out tests/parity/wide/cases.jsonl
tools/parity/record.sh --cases tests/parity/wide/cases.jsonl --out tests/parity/wide/expected.jsonl
cargo test --test parity -- --ignored --nocapture      # une dizaine de minutes en debug
```

4274 invocations : 48 551 conversions (`cs2cs`, `proj`, `cct`) et 1000 listes de candidats,
soit 50 928 lignes comparées. Le test `parity_wide_against_proj_9_9_0` est ignoré par défaut
(il apparaît « 1 ignored », jamais « passed » sans avoir tourné).

Premier passage : 355 écarts. Corrigés depuis : la chaîne composé → géocentrique sans emprise
ni précision, l'emprise des ballparks, l'inverse de Cassini, `proj -S` sur `s2`, `alsk`,
`gs50`. État actuel : 50 219 identiques, 560 dans le seuil, aucun écart de valeur sur les
conversions. Le registre `tests/parity/wide/known.tsv` garde 134 lignes, toutes de 38 listes
`projinfo` : ensemble de candidats différent entre CRS composés, verticaux et géocentriques (111),
texte d'un candidat (17), emprise « World » contre « World. » (6). Aucune ne change les
coordonnées de `cs2cs` sur le corpus.

Repli de `proj_trans` reproduit tel quel : pour un point hors de toutes les emprises, PROJ prend
le premier candidat sans grille, sans nouvel essai, même si sa chaîne PROJ n'a pas pu être
construite. `EPSG:10644 → EPSG:11069` : le premier candidat convertit deux fois les degrés en
radians (bug de PROJ, « Mismatched units between step 1 and 2 », reproduit à l'identique par le
port), donc `* * inf` hors de Saba (`tests/cs2cs_fallback_broken_candidate.rs`). Candidat à une
correction en phase 2.
