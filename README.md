# PROJ-RS

Portage open source de [PROJ](https://github.com/OSGeo/PROJ) vers Rust.
Le dépôt vise le même comportement observable que PROJ 9.4 sur les conversions
portées, avec une API Rust (`Result`, enums, pas de codes d'erreur ni de `NaN`
sentinel sur le chemin scalaire).

Le premier périmètre est celui qui sert en France métropolitaine :

- **UTM** — Transverse Mercator exact de Poder / Engsager, le défaut
  `tmerc_default_algo = poder_engsager` de PROJ 9.4.
- **Lambert-93** — Lambert conforme conique ellipsoïdal sur GRS80.

Le reste (base EPSG, grilles NTv2, WKT, pipelines, les autres projections)
est listé dans [ROADMAP.md](ROADMAP.md) et s'ajoute par pull request.

## État

| Lot | Contenu | Branche |
|---|---|---|
| 0 | Gouvernance, licence, CI, clippy | `cursor/lot-0-gouvernance-a775` |
| 1 | Ellipsoïde, erreurs, angles | `cursor/lot-1-ellipsoide-a775` |
| 2 | Transverse Mercator et UTM | `cursor/lot-2-tmerc-a775` |
| 3 | Lambert conforme et Lambert-93 | `cursor/lot-3-lambert-a775` |
| 4 | Lot AVX2 du Transverse Mercator et chrono | `cursor/lot-4-simd-bench-a775` |
| 5 | Lot AVX2 de Lambert-93 | `cursor/lot-5-lambert-avx-a775` |
| 6 | Mercator et Web Mercator | `cursor/lot-6-mercator-a775` |
| 7 | Helmert à 7 paramètres | `cursor/lot-7-helmert-a775` |

Les lots 1 à 7 sont des pull requests empilées. On les relit dans l'ordre.
**Seul le mainteneur fusionne.** Les textes de PR sont dans
[docs/LOTS.md](docs/LOTS.md) : la forge temporaire de l'agent ne peut pas
les ouvrir, et le jeton GitHub de la session n'a pas le droit de créer
`RandazzoMaxime/proj-rs`.

## Lancer

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

Le lot 4 compare le crate à `libproj` (`libproj-dev`, `proj-bin`) :

```bash
cargo test --all-features
cargo run --release --features bench --bin proj-rs-bench
```

Rust 1.83 ou plus récent. Les lints viennent de
[BeautifulRust](https://github.com/morganlinton/BeautifulRust).

## Contribuer

Toute modification passe par une pull request. La procédure, le format d'une PR
de lot, et la règle de fusion sont dans [CONTRIBUTING.md](CONTRIBUTING.md).

## Licence

MIT. Les formules reprises de PROJ restent sous la licence MIT de PROJ.
Voir [LICENSE](LICENSE).
