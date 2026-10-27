# Textes des pull requests

La forge de cette session est un dépôt temporaire d'agent : elle refuse
les pull requests (`repoKind=agent_temp`). Le jeton GitHub n'a pas le droit
de créer `RandazzoMaxime/proj-rs` (403).

Les branches sont poussées. Dès que le dépôt GitHub existe, les cinq
commandes ci-dessous ouvrent les PR. Le mainteneur fusionne dans l'ordre,
une à une. Pas de merge automatique.

```bash
gh repo create RandazzoMaxime/proj-rs --public --description "Portage Rust de PROJ" --source=. --remote=github
git push github cursor/base-initiale-a775 cursor/lot-0-gouvernance-a775 cursor/lot-1-ellipsoide-a775 cursor/lot-2-tmerc-a775 cursor/lot-3-lambert-a775 cursor/lot-4-simd-bench-a775 cursor/lot-5-lambert-avx-a775 cursor/lot-6-mercator-a775 cursor/lot-7-helmert-a775

gh pr create --repo RandazzoMaxime/proj-rs --base cursor/base-initiale-a775 --head cursor/lot-0-gouvernance-a775 --title "Lot 0 — Gouvernance, licence et lints" --body-file docs/pr/lot-0.md
gh pr create --repo RandazzoMaxime/proj-rs --base cursor/lot-0-gouvernance-a775 --head cursor/lot-1-ellipsoide-a775 --title "Lot 1 — Ellipsoïde, coordonnées, réduction de longitude" --body-file docs/pr/lot-1.md
gh pr create --repo RandazzoMaxime/proj-rs --base cursor/lot-1-ellipsoide-a775 --head cursor/lot-2-tmerc-a775 --title "Lot 2 — Transverse Mercator exact et UTM" --body-file docs/pr/lot-2.md
gh pr create --repo RandazzoMaxime/proj-rs --base cursor/lot-2-tmerc-a775 --head cursor/lot-3-lambert-a775 --title "Lot 3 — Lambert conforme conique et Lambert-93" --body-file docs/pr/lot-3.md
gh pr create --repo RandazzoMaxime/proj-rs --base cursor/lot-3-lambert-a775 --head cursor/lot-4-simd-bench-a775 --title "Lot 4 — Lot AVX2 du Transverse Mercator et chrono" --body-file docs/pr/lot-4.md
gh pr create --repo RandazzoMaxime/proj-rs --base cursor/lot-4-simd-bench-a775 --head cursor/lot-5-lambert-avx-a775 --title "Lot 5 — Lot AVX2 de Lambert-93" --body-file docs/pr/lot-5.md
gh pr create --repo RandazzoMaxime/proj-rs --base cursor/lot-5-lambert-avx-a775 --head cursor/lot-6-mercator-a775 --title "Lot 6 — Mercator et Web Mercator" --body-file docs/pr/lot-6.md
gh pr create --repo RandazzoMaxime/proj-rs --base cursor/lot-6-mercator-a775 --head cursor/lot-7-helmert-a775 --title "Lot 7 — Helmert à 7 paramètres" --body-file docs/pr/lot-7.md
```

La ligne à publier après relecture est `cursor/lot-4-simd-bench-a775`.
`main` sur cette forge garde le premier jet, hors de la pile.
