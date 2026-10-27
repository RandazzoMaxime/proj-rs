# Port — Helmert à 7 paramètres

Source : PROJ 9.4, `+proj=helmert`. Les valeurs de référence viennent de `cct`.

## Correspondance d'API

| PROJ | PROJ-RS |
|---|---|
| `+x +y +z` (mètres) | `Helmert::try_new`, translations |
| `+rx +ry +rz` (secondes d'arc) | rotations |
| `+s` (ppm) | `scale_ppm` |
| `+convention=position_vector` | `HelmertConvention::PositionVector` |
| `+convention=coordinate_frame` | `HelmertConvention::CoordinateFrame` |

## Correspondance de tests

| `cct` | Test |
|---|---|
| translation (1, −2, 3) | `proj_translation` |
| `rx = 1"` position vector | `proj_rotation_position_vector` |
| même rotation, coordinate frame | `proj_rotation_coordinate_frame_flips_sign` |
| `s = 1` ppm | `proj_scale_is_parts_per_million` |
| translation, trois rotations et échelle | `proj_combined_matches_cct` |

## Écarts volontaires

- Pas de grille, pas de pas de temps. Uniquement les 7 paramètres constants.
- L'échec d'un paramètre non fini est `Err`, pas un crash de `cct`.

## Quirks conservés

- La rotation est la matrice linéarisée de PROJ, pas une rotation exacte.
  `rz = 3600"` (1°) sur `(1, 0, 0)` donne `y = π/180`, pas `sin(1°)`.
- Le facteur d'échelle est en parties par million : `1 + s · 10⁻⁶`.
- `coordinate_frame` inverse seulement le signe des trois rotations.

## Vérification

```bash
cargo test --all-features helmert
cargo clippy --all-targets --all-features -- -D warnings
```

Tolérance 1e-6 m sur les sorties `cct -d 8`.
