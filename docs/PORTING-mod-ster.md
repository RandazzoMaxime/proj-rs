# Fiche : stéréographiques modifiées (`mod_ster`)

Référence PROJ 9.9.0 : `src/projections/mod_ster.cpp`. Port : `src/projections/mod_ster.rs`.
Projections : `mil_os`, `lee_os`, `gs48`, `alsk`, `gs50`.

## Ce que fait PROJ

Le setup remplace trois champs de l'ellipsoïde après son calcul :

- `gs48` : `a = 6370997`, `es = 0`.
- `alsk` et `gs50` sur ellipsoïde : `a = 6378206.4`, `es = 0.00676866`, `e = sqrt(es)`,
  et la série `ABe`. Sur sphère : `a = 6370997`, `es = 0`, série `ABs`.
- `mil_os`, `lee_os` : `es = 0`. `a` et `e` de l'entrée restent.

Trois détails non évidents :

1. `P->e` garde la valeur de l'entrée pour `mil_os`, `lee_os`, `gs48`. Le noyau lit `P->e`
   sur tous les chemins, donc l'excentricité d'entrée agit même si `es` vaut 0.
2. `chio` vaut `phi0` quand `es = 0`, et la conforme sinon.
3. `P->ra` garde `1/a` de l'entrée. L'inverse divise donc par l'`a` d'entrée, alors que
   l'avant multiplie par l'`a` forcé.

## Ce que fait le port

`build` pose le même ellipsoïde (`a`, `es`) dans le cadre. Le noyau reçoit `e` de
l'entrée pour les trois projections qui ne forcent que `es`. Son inverse multiplie
les coordonnées par `a_forcé / a_entrée` avant la résolution.

## Écarts fermés

35 entrées du registre (30 avant, 5 inverse), retirées de `tests/parity/known.tsv` :

| Projection | Écart avant | Cause |
|---|---|---|
| `gs48` avant | jusqu'à 10 Mm | `e` d'entrée non gardé et `a` non forcé |
| `gs48` inverse | 0,19° en latitude | `ra` d'entrée dans PROJ, `a` forcé en avant |
| `gs50` avant | ~1e-5 relatif | `a` non forcé |
| `alsk` avant | dizaines de m | `a` non forcé |
| `lee_os` avant | dizaines de km | `e` mis à zéro |
| `mil_os` avant | ~1 km | `e` mis à zéro |

## Mesure

- `cargo test --test unit_proj_mod_ster` : `5 passed; 0 failed`. Valeurs de
  `proj -f %.12f` et `proj -I` du binaire de référence, six points par projection.
- Suite de parité, avant : 187 hors seuil, 6 forme, 1 statut, 3974 identiques.
  Après : 152 hors seuil. La famille `proj-fwd` passe de 36 à 6 hors seuil, `proj-inv`
  de 11 à 6. Aucun écart nouveau.

## Écarts ouverts

Aucun dans cette famille. Reste ouvert : `ob_tran` avec `o_proj=moll`, fiche
[`docs/PORTING-ob-tran-longlat.md`](PORTING-ob-tran-longlat.md).
