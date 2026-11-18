# Port — chemin par lots (`forward_batch`) hors AVX2

Lot G. Fichiers : `src/tmerc.rs` (`forward_batch`), `src/tmerc/batch.rs` (repli portable),
`src/lcc.rs` (`forward_batch`, inchangé), `src/simd.rs`, `src/lib.rs` (`batch_path`).

## Constat

La documentation disait : AVX2 sur x86-64 Linux glibc et Windows, boucle scalaire ailleurs.
C'est inexact. `batch_path()` prend AVX2 sur tout x86-64 qui a `avx2` et `fma`. Seules les
transcendantes changent : libmvec sous Linux glibc, sinon le CRT voie par voie (`map1`).
La boucle scalaire tourne donc sur les autres architectures : aarch64 (Mac Apple Silicon,
Linux ARM).

## Mesure

Machine : Linux x86-64, Intel Xeon 2,1 GHz, AVX2 et FMA, 4 cœurs partagés (charge 2 à 3
pendant les mesures). Build `--release`.

Protocole :

- 1 000 000 points, mêmes grilles que `src/bin/bench.rs` (graines 1, 2, 3 et leurs boîtes).
- Une série = médiane de 5 passes chronométrées, après une passe d'échauffement.
- Séries répétées jusqu'à deux médianes consécutives à moins de 3 % (10 séries au plus).
- Processus épinglé (`taskset -c 2`).
- (a) boucle scalaire : `forward` point par point, comme le repli de `forward_batch`.
  (b) repli portable : `Tmerc::forward_batch_portable`. (c) `forward_batch` avec AVX2.

Résultat du passage final, sur le code livré :

| Cas | (a) scalaire ns/pt | (b) portable ns/pt | (c) AVX2 ns/pt | gain (b) vs (a) |
|---|---:|---:|---:|---:|
| UTM 31, zone centre | 111,6 | 83,8 | 40,4 | −25,0 % |
| UTM 31, bord de zone | 112,3 | 74,2 | 38,7 | −33,9 % |
| Lambert-93 | 82,8 | non livré | 25,4 | (c) −69,3 % vs (a) |

La série (b) « zone centre » de ce passage oscille entre 75 et 118 ns/pt sur six séries.
Les deux dernières (74,8 et 75,0) se recoupent. La médiane des six, 83,8, est celle retenue.

Passages précédents, même protocole, médianes en ns/pt :

| Passage | UTM centre (a → b) | UTM bord (a → b) | Lambert-93, variante Lcc portable (a → b) |
|---|---|---|---|
| 2 | 111,0 → 77,0 (−30,7 %) | 110,0 → 73,6 (−33,2 %) | 78,9 → 75,8 (−4,0 %) |
| 3 | 110,8 → 77,9 (−29,7 %) | 113,2 → 75,0 (−33,7 %) | 82,5 → 83,6 (+1,4 %) |

Le premier passage a été écarté. Sa ligne (a) appelait `forward` à travers un `&dyn Fn`
par point, ce qui empêchait l'inlining et gonflait le gain. La mesure est refaite avec
une boucle monomorphe avant les passages 2 et 3.

Gain sur UTM : entre −25 % et −34 % sur les trois passages retenus. Gain sur Lambert-93 : entre
−4 % et +1 %, dans le bruit.

## Parité

Référence : `Operation::forward` sur 10 000 points de chaque grille (valeurs est et nord,
soit 20 000 valeurs par cas). Cette référence est égale à `proj` 9.9.0 pour les quatre
points testés (Tour Eiffel, (3 ; 48), (0,2 ; 43,5), (5,8 ; 49,9)), à neuf décimales près.

| Chemin | UTM centre : valeurs différentes au bit | UTM bord | Lambert-93 |
|---|---|---|---|
| (a) boucle scalaire | 0 | 0 | — |
| (b) repli portable | 0 | 0 | — |
| (c) AVX2 | 15 545 / 20 000 ; 29 ulps max ; 2,8 nm max | 15 543 / 20 000 ; 33 ulps ; 3,7 nm | 11 344 / 20 000 ; 32 ulps ; 7,5 nm |

Depuis `fmath` (`docs/PORTING-simd.md`), la boucle scalaire elle-même n'est plus bit à bit avec PROJ (au plus 7,5e-9 m) ; le repli portable reste bit à bit avec elle, car il appelle les mêmes fonctions de `fmath`.

Le chemin AVX2 n'est donc pas bit à bit avec PROJ ni avec la boucle scalaire. Son écart reste
sous 10 nm, la borne du test `batch_matches_scalar_within_ten_nanometres`.

Tests de parité du repli, dans `src/tmerc/batch.rs` : 1 610 points par zone (UTM 31 N et
UTM 1 S), avec NaN, infinis, pôles, longitudes hors domaine, et des points refusés au milieu
des blocs de quatre. Le test vérifie aussi qu'il y a plus de 1 000 sorties finies et au moins
un refus. Le repli est ensuite comparé au bit près à `Tmerc::forward`. Un écart de 1e-9 m
injecté dans le repli fait échouer le test (vérifié).

## Décision

- **Tmerc (UTM) : repli portable livré.** Critère atteint : gain ≥ 15 % sur les deux cas UTM,
  et 0 bit d'écart avec la boucle scalaire. `batch_path()` rend `portable` hors AVX2.
- **Lcc (Lambert-93) : boucle scalaire conservée.** Gain de −4 % à +1 %, sous le seuil.
  La variante testée (quatre voies en tableaux, transcendantes scalaires) n'est pas livrée.
  Dans les deux chemins, `tsfn` (`atanh`, `exp`, `powf`) reste un appel par point.
- **Limite.** Toutes les mesures sont faites sur x86-64 Linux, avec glibc. Le gain sur
  aarch64 n'est pas mesuré. Il dépend du libm et du cœur.

## Suite

Lot 7 : le chemin par lots sert aussi `cart`, `helmert`, `push`/`pop` (`docs/PORTING-batch-datum.md`)
et `sterea`, `somerc`, `laea` (`docs/PORTING-batch-oblique.md`).

## Ce qu'il faudrait pour aarch64

- libmvec AdvSIMD : `_ZGVnN2v_sin` (et `cos`, `log`, `exp`, `atanh`, `atan2` en `vv`) sous
  glibc ≥ 2.38. Chemin NEON à deux voies, à mesurer contre le repli portable.
- Accelerate `vvsin` (et voisins) sous macOS, pour les Mac Apple Silicon des runners de l'équipe.
- Non testable ici : pas de qemu ni de linker croisé. La compilation seule est vérifiée.

## Commandes

```sh
# Compilation aarch64 (ne linke pas). La compilation C croisée de sqlite3 et de geodesic
# exige des en-têtes aarch64 absents de cette machine : voir la note ci-dessous.
rustup target add aarch64-unknown-linux-gnu
cargo check --target aarch64-unknown-linux-gnu --lib

# Tests du repli et du chemin par lots
cargo test --lib tmerc::batch

# Non-régression
cargo test --test cli_binary --test cli_proj_factors --test georust_parity
cargo test --lib
```

Note : sur cette machine, `cargo check` aarch64 compile les unités C du dépôt (sqlite3
bundled et geodesic) avec clang, en remplaçant ces deux unités par des objets vides. Le
check ne linke pas, donc seule la vérification de types Rust est visée. Les en-têtes
OpenSSL sont pris dans un dossier local. Ce contournement n'est pas dans le dépôt.

La boucle de mesure (a), (b), (c) était un test `#[ignore]` jetable. Elle n'est pas
commitée. Le protocole est décrit plus haut : un test `--release` avec `taskset`, 1e6 points,
séries de médianes.
