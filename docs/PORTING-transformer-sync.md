# Transformer : Send + Sync

Lot : rendre `Transformer` `Send + Sync`, pour qu'une application convertisse un lot en parallèle
avec un seul `&Transformer` au lieu de huit copies. Fichiers touchés : `src/iso19111/alternatives.rs`,
`src/api/transformer.rs` (doc et test unitaire), `tests/transformer_sync.rs`, `docs/API-v1.md`,
`CHANGELOG.md`.

## Cause

`Transformer` n'était pas `Sync` à cause d'un seul type : `Alternatives` (l'ensemble de candidats
d'une paire, `src/iso19111/alternatives.rs`). Il gardait entre deux points quatre états dans des
`Cell` et `OnceCell` :

| champ | type avant | rôle |
|---|---|---|
| `warn` | `Cell<bool>` | `warnIfBestTransformationNotAvailable` : vrai jusqu'au premier échec sous `Warn` |
| `current` | `Cell<Option<usize>>` | `P->iCurCoordOp` : le candidat du dernier point |
| `last_failure` | `Cell<Option<&'static str>>` | texte du dernier échec, pour la ligne de log du retry |
| `forward_index`, `inverse_index` | `OnceCell<Option<Box<BoxIndex>>>` | index des boîtes, construit au premier point |

Le diagnostic est `assert_send_sync::<Transformer>()` dans `tests/transformer_sync.rs` : avant le
changement, le compilateur donne 12 erreurs E0277, toutes sur ces quatre champs (`Cell<bool>`,
`Cell<Option<usize>>`, `Cell<Option<&str>>`, `OnceCell<...>`). `Operation` (le moteur), `Context` et
`Crs` ne posaient pas de problème : `Operation` est déjà `Sync` (ses étapes gardent leurs paramètres
dans des atomiques, `src/engine.rs`).

## Conception

- `warn` : `AtomicBool`. Lu une fois au début de chaque point, pris par `swap(false)` au premier
  échec : de plusieurs threads qui échouent ensemble, un seul écrit l'avertissement.
- `current` : `AtomicUsize`, `usize::MAX` pour « aucun point ». Écrit seulement quand le candidat
  change, comme avant.
- `last_failure` : `Mutex<Option<&'static str>>`. Il n'est écrit que sur un candidat qui échoue,
  jamais sur le chemin qui réussit. Verrou empoisonné lu quand même (la valeur est un `&'static str`).
- `forward_index`, `inverse_index` : `OnceLock`, même `get_or_init`.
- Ordre mémoire : `Relaxed` partout. Ces états ne publient aucune autre donnée ; ce sont des
  indications de choix et de log.
- Pas de `unsafe`, pas de `unsafe impl Sync`.
- `Route::Single` (une seule opération) n'utilise pas `Alternatives` : son chemin chaud est inchangé
  dans le code. `Transformer` reste `Clone` et `Debug`, et la taille de `Route` ne change pas (le
  `Box<Alternatives>` est déjà sur le tas).
- `#[inline(never)]` sur `Alternatives::transform` : voir « Performance ».

`last_used_operation()` garde sa sémantique documentée : l'opération du dernier point. Sous plusieurs
threads, c'est celle du dernier point qui a écrit, et un point en cours peut montrer un candidat qu'il
a tenté avant celui qu'il garde (`current` est écrit avant l'essai, et un échec passe au suivant).

## Écarts vers PROJ

PROJ n'est pas thread-safe au niveau d'un `PJ` : un objet de transformation va à un thread à la fois.
Le port partage donc entre threads ce que `proj_trans` garde par `PJ`. Trois conséquences :

1. **Le choix d'un point peut dépendre des autres points, sous plusieurs threads.** L'avertissement
   `warn` change le mode du choix (`Any` contre `InstantiableOnly`) pour les points qui suivent le
   premier échec. Un seul thread voit l'ordre des points ; plusieurs threads ne l'ordonnent pas.
   Un point qui tourne pendant le premier échec peut prendre le choix d'avant. Le cas où ce mode
   change le résultat, et non seulement le log, n'a **pas été atteint** par les tests de ce lot
   (voir « Ce qui n'est pas vérifié ») : il est argumenté, pas mesuré.
2. **`last_used_operation()` peut montrer un candidat tenté puis abandonné** (voir ci-dessus).
3. **Les lignes de log `Using coordinate operation` et l'avertissement peuvent être dupliquées ou
   absentes** sous concurrence, parce que le log de `proj_trans` est par point et que son état est
   partagé. Seul le log change, jamais le résultat. L'avertissement unique reste unique grâce à `swap`.

Le comportement à un seul thread est celui d'avant : mêmes champs, mêmes écritures, même ordre.

## Tests (tests/transformer_sync.rs), écrits avant le changement

- `transformer_is_send_and_sync` : `assert_send_sync::<Transformer>()`. Échoue à la compilation avant
  le changement (12 erreurs E0277, sortie dans la PR).
- `shared_transformer_matches_one_thread_bit_for_bit_single_candidate` : EPSG:4326 vers EPSG:32631,
  8 threads partagent `&Transformer`, chacun convertit 10 000 points par `forward` puis par
  `forward_slice`. Chaque résultat doit égaler, bit pour bit (`f64::to_bits`), celui d'un transformeur
  neuf sur un seul thread. Erreurs comparées par leur texte.
- `shared_transformer_matches_one_thread_bit_for_bit_several_candidates` : EPSG:4267 vers EPSG:4269,
  même protocole, plusieurs candidats.
- `shared_transformer_agrees_with_cs2cs_on_a_few_points` : valeurs de `cs2cs` 9.9.0
  (`target/proj-reference/build/bin/cs2cs -f %.9f`, `PROJ_DATA=data`) : trois points en UTM 31N, trois
  points NAD27 vers NAD83. Tolérance 1e-6 m et 1e-8 degré.
- `last_used_operation_reads_while_points_convert` : 4 threads convertissent, 4 threads lisent
  `last_used_operation()` 10 000 fois. Pas de panique ; chaque nom lu est un nom que voit un thread
  seul, ou un candidat de `operations()` (c'est l'ensemble des candidats tentés, voir plus haut).
  Après les écritures, la valeur lue est un candidat de la paire.

Les valeurs attendues viennent de `cs2cs` 9.9.0 et d'un transformeur à un seul thread, jamais du
code testé seul.

## Performance

Méthode. Exemple temporaire, non commité : un `Transformer` par paire, 10 000 points à seed fixe
(lat 35..70, lon -10..20 pour la paire simple ; lat 25..49, lon -125..-66 pour EPSG:4267 vers
EPSG:4269). Par passe : `forward`, un appel par point, temps total / 10 000 ; `forward_slice`, un
slice de 10 000 points, copie préparée hors chronométrage, temps / 10 000 ; `clone`, 1 000 clones,
temps / 1 000. 21 passes par mesure ; on donne la médiane et le minimum des passes.
- Référence : `origin/main` (91f92e7) exporté par `git archive` dans un répertoire temporaire (pas de
  worktree). Changement : la branche, même exemple, même profil `release` (LTO thin, une unité de
  génération). Deux répertoires cibles distincts, sinon cargo réutilise le binaire du premier.
- Machine : 4 vCPU, `taskset -c 2`, séries alternées référence et changement, une paire par série.
  Le bruit est réel : sur une même série, la médiane d'une passe varie de plusieurs dizaines de
  pourcents d'une série à l'autre ; les minimums sont plus stables. Les écarts ci-dessous sont
  donc les médianes et les minimums de 16 séries (32 binaires exécutés).

Résultats, en ns (référence, changement, écart), configuration livrée (`#[inline(never)]` gardé) :

| mesure | médiane réf. | médiane chg. | écart médiane | minimum réf. | minimum chg. | écart minimum |
|---|---|---|---|---|---|---|
| `forward` simple | 286,7 | 281,7 | −1,7 % | 262,8 | 262,6 | −0,1 % |
| `forward_slice`, par point, simple | 49,0 | 49,2 | +0,2 % | 46,8 | 46,8 | +0,1 % |
| `clone` simple | 25 972 | 26 028 | +0,2 % | 23 377 | 23 428 | +0,2 % |
| `forward` plusieurs | 507,9 | 505,9 | −0,4 % | 455,8 | 451,6 | −0,9 % |
| `forward_slice`, par point, plusieurs | 399,9 | 399,4 | −0,1 % | 362,2 | 366,7 | +1,2 % |
| `clone` plusieurs | 11 416 | 11 878 | +4,0 % | 10 200 | 10 495 | +2,9 % |

Lecture.
- Les chemins par point (`forward` et `forward_slice`, simple et plusieurs candidats) restent sous
  le seuil de 2 % sur la médiane et sur le minimum. Le plus grand écart est −1,7 % (médiane de
  `forward` simple, dans le sens plus rapide) ; le plus grand écart lent est +1,2 % (minimum de
  `forward_slice` plusieurs).
- `#[inline(never)]` sur `Alternatives::transform` est justifié par une mesure. Sans lui,
  `forward_slice` du chemin simple, qui n'utilise pas `Alternatives`, est plus lent de 3,0 % sur la
  médiane et de 1,1 % sur le minimum (24 séries contre la référence). Avec lui, l'écart est de
  +0,2 % et +0,1 % (tableau ci-dessus).
- `clone` d'un transformeur à plusieurs candidats est plus lent : +4,0 % sur la médiane, +2,9 % sur le
  minimum. C'est au-dessus de 2 %, mais ce n'est pas un chemin par point (un clone coûte 10 à 26 µs).
  La cause n'a pas été isolée ; le verrou de `last_failure` copié par `clone` est une piste, non
  vérifiée. Le clone d'un transformeur simple reste à +0,2 %.

Le bench existant (`proj-rs-bench`, `bench/record.sh`) ne mesure pas `Transformer` (0 occurrence dans
`src/bin/bench.rs`, il passe par la façade `Proj`). Il n'a pas été lancé : `record.sh` réécrit
`README.md`, `ROADMAP.md` et les graphiques, et ajoute une ligne à `bench/history.jsonl`. Ce lot ne le fait pas.

## Commandes lancées

```text
PROJ_DATA=$PWD/data cargo +1.100.0 test --test transformer_sync          # rouge avant, vert après
git archive origin/main | tar -x -C <dir-ref>                            # référence, sans worktree
CARGO_TARGET_DIR=<cible-ref>  cargo +1.100.0 build --release --example <mesure>   # référence
CARGO_TARGET_DIR=<cible-new>  cargo +1.100.0 build --release --example <mesure>   # changement
taskset -c 2 <binaire>                                                   # séries alternées
target/proj-reference/build/bin/cs2cs -f %.9f EPSG:4326 EPSG:32631       # valeurs de référence
```

## Ce qui n'est pas vérifié

- **Le chemin où l'avertissement change le résultat n'est pas atteint.** Sur EPSG:4267 vers 4269,
  EPSG:4230 vers 4326 et EPSG:4277 vers 4326, sur des points de la zone et hors zone (-80..80 de
  latitude, -180..180 de longitude), avec le contexte par défaut comme avec le contexte hermétique,
  aucun point n'échoue : aucun candidat ne tombe sur une grille absente. Seul EPSG:4326 vers 32631
  échoue (`OutsideDomain`, une paire à un candidat, qui n'a pas de choix). Le comportement concurrent
  de `warn` est donc argumenté et non mesuré. Pour le mesurer, il faudrait un contexte où une grille
  de candidat manque au point, ou un candidat qui échoue sur des points de sa boîte.
- Les tests de concurrence ne prouvent pas l'absence de course ; ils prouvent l'identité bit à bit
  sur 80 000 points par paire, lancée une fois, sur cette machine.
- `OnlyBest::Yes` et `OnlyBest::No` ne sont pas testés sous concurrence.
- Une seule plateforme (Linux x86_64). Les atomiques et `Mutex` de la std sont portables.
- Le MSRV 1.83 : `OnceLock` (1.70) et les atomiques sont dans la std ; `cargo +1.83.0 check` est
  lancé avant la PR.
