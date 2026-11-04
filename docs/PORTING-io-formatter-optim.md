# Port — `io.projstringformatter_*` : réécritures du formateur PROJ-string

Source : PROJ 9.9.0, `test/unit/test_io.cpp` (groupe `io.projstringformatter_*`) et
`src/iso19111/io.cpp`, `PROJStringFormatter::Private::toString` : les trois passes
d'optimisation (lignes ~9655 à ~9990 de `io.cpp`, 9.9.0).

## Cas portés (4)

| C++ TEST | Test Rust |
|---|---|
| `projstringformatter_optim_hgridshift_vgridshift_hgridshift_inv` | `io_projstringformatter_optim_hgridshift_vgridshift_hgridshift_inv` |
| `projstringformatter_optim_as_uc_vgridshift_uc_as_push_as_uc` | `io_projstringformatter_optim_as_uc_vgridshift_uc_as_push_as_uc` |
| `projstringformatter_krovak_to_krovak_east_north` | `io_projstringformatter_krovak_to_krovak_east_north` |
| `projstringformatter_krovak_east_north_to_krovak` | `io_projstringformatter_krovak_east_north_to_krovak` |

Les tests sont dans `src/iso19111/pipeline/tests.rs`, à côté du formateur qui est
`pub(crate)`, comme le reste des cas `io.projstringformatter_*`.

## Changements dans `src/iso19111/pipeline.rs`

- `stop_inversion` échange `omit_fwd` et `omit_inv` des pas inversés (C++ : paramètres
  `omit_fwd` / `omit_inv` inversés à l'inversion du pipeline).
- `finish` lance `rewrite_patterns` après `merge_adjacent`. `rewrite_once` applique, à la
  première position qui correspond, trois motifs de C++ :
  1. `rewrite_krovak_swu` : deux pas Krovak de même nom, de sens opposé, dont l'un porte
     `axis=swu` en tête et dont les autres paramètres sont identiques, deviennent
     `axisswap order=-2,-1`.
  2. `rewrite_grid_push_pop` : `hgridshift` / `vgridshift` / `hgridshift` inverse
     deviennent `push v_1 v_2`, `hgridshift +omit_inv`, `vgridshift`,
     `hgridshift +omit_fwd`, `pop v_1 v_2`.
  3. `rewrite_push_axes_units` : deux motifs autour de `push`, avec `axisswap order=2,1`
     et `unitconvert` deg/rad autour d'un `vgridshift`.

## Écarts et limites

- Les passes C++ sont des boucles sur une liste avec `continue`. Ici, `rewrite_once` relance
  la recherche depuis le début après chaque réécriture. Le résultat est le même pour les
  motifs portés, car chaque réécriture supprime des pas ou les marque et ne recrée pas de
  motif.
- Les motifs de `io.cpp` sur `set` (fusion d'époques) ne sont pas portés. Ils ne sont
  testés par aucun cas du lot IO.
- `docs/PORTING-io-core-a.md`, `docs/PORTING-io-unit-8.md` et
  `docs/PORTING-io-capi-core-2.md` décrivent encore ces quatre cas comme non portés.
  Ce document les remplace.

## Commandes et résultats

- `cargo test --lib io_projstringformatter` : 20 passés, 0 échec.
- `cargo fmt --check` : OK.
- `cargo clippy --all-targets --all-features -- -D warnings` : OK.
- `PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast` :
  1376 passés, 0 échec, 26 ignorés.
