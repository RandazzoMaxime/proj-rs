# Contribuer à PROJ-RS

PROJ-RS est ouvert : les issues et les pull requests sont les bienvenues.
La fusion dans la branche principale reste entre les mains du mainteneur.

## Qui fusionne

**Maxime Randazzo ([@RandazzoMaxime](https://github.com/RandazzoMaxime)) est le
seul à fusionner.** Une revue approuvée, une CI verte ou un accord dans
l'issue ne fusionnent pas la PR toutes seules. Il n'y a pas de merge
automatique.

Personne ne pousse directement sur `main`. Même un correctif d'une ligne
passe par une branche et une pull request.

Deux rulesets GitHub, `protect-main` et `protect-dev`, appliquent ces règles :

- pas de push direct : toute modification passe par une pull request ;
- pas de force-push ni de suppression de la branche ;
- historique linéaire, fusion par squash ou par rebase ;
- aucune approbation tierce exigée, le mainteneur fusionne seul.

`CODEOWNERS` désigne `@RandazzoMaxime` sur tout le dépôt. GitHub s'en sert
pour la revue ; ça ne donne à personne d'autre le droit de fusionner.

## Proposer un lot

Un lot est une pull request qui porte un seul sujet. Les sujets ouverts sont
dans [ROADMAP.md](ROADMAP.md). Ouvrir une issue « Je prends le lot N » avant
de coder évite deux ports du même module.

La branche part de `main`. Le nom indique le lot : `lot-8-cart`.

## Ce qu'une PR de port doit contenir

Le port suit le skill
[rust-port](https://github.com/morganlinton/BeautifulRust/tree/main/skills/rust-port) :

1. Lire le source PROJ et lister le comportement observable, y compris les
   échecs et les cas limites.
2. Écrire d'abord les tests qui caractérisent ce comportement.
3. Dessiner l'API Rust (`Result`, `Option`, enums). Ne pas translittérer les
   codes d'erreur, les `NULL` ni les paramètres booléens.
4. Garder l'algorithme. Un changement d'algorithme se justifie dans la PR.
5. Joindre un `docs/PORTING-<module>.md` : correspondance des fonctions, des
   tests, écarts volontaires, quirks conservés, commandes lancées.
6. Ajouter les cas du lot à `src/bin/bench.rs` et une ligne à
   `bench/history.jsonl` via `bench/record.sh`. Ne jamais renommer un cas
   existant : l'historique est indexé par nom.
7. Si le lot retire des lignes de `docs/UNPORTED-UNIT.md` ou
   `docs/UNPORTED-CLI.md`, lancer `UPDATE_DOCS=1 cargo test --test docs_in_sync`.
   Les chiffres du `README.md` et du `ROADMAP.md` et le tri
   `docs/UNIT-TRIAGE.md` en sont calculés, et le test échoue s'ils dérivent.

Le modèle de pull request rappelle ces sections. Le remplir fait partie de
la revue.

## Barre technique

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

Pas de `unsafe` sans commentaire `// SAFETY:` et sans besoin mesuré ou FFI.
Pas de paramètre `bool` dans une signature publique. Pas de `unwrap()` hors
des tests. Les lints sont ceux de BeautifulRust (`clippy.toml` et `[lints]`
dans `Cargo.toml`).

## Revue

Le mainteneur regarde, dans l'ordre : le comportement face à PROJ, les écarts
annoncés dans la fiche `docs/PORTING-<module>.md`, l'API, puis le style. Une PR qui mélange deux
lots ou qui élargit le périmètre sans le dire retourne à l'auteur.
