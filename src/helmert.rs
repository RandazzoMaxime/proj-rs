//! Transformation de Helmert à 7 paramètres.
//!
//! Portage de la forme linéarisée de PROJ (`+proj=helmert`). Les rotations
//! sont en secondes d'arc, le facteur d'échelle en parties par million.
//! Voir `docs/PORTING-helmert.md`.

use crate::ProjectionError;

/// Convention de signe des rotations.
///
/// `position_vector` est la convention EPSG 1032. `coordinate_frame` en
/// inverse le signe, comme `+convention` dans PROJ.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HelmertConvention {
    /// Rotations du vecteur position.
    PositionVector,
    /// Rotations du repère. Signe opposé à [`Self::PositionVector`].
    CoordinateFrame,
}

/// Paramètres bruts, unités de PROJ : mètres, secondes d'arc, ppm.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HelmertParams {
    /// Translation en X, mètres.
    pub translation_x: f64,
    /// Translation en Y, mètres.
    pub translation_y: f64,
    /// Translation en Z, mètres.
    pub translation_z: f64,
    /// Rotation autour de X, secondes d'arc.
    pub rotation_x_arcsec: f64,
    /// Rotation autour de Y, secondes d'arc.
    pub rotation_y_arcsec: f64,
    /// Rotation autour de Z, secondes d'arc.
    pub rotation_z_arcsec: f64,
    /// Échelle, parties par million.
    pub scale_ppm: f64,
    /// Signe des rotations.
    pub convention: HelmertConvention,
}

/// Helmert préparé. Les rotations sont déjà en radians.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Helmert {
    tx: f64,
    ty: f64,
    tz: f64,
    rx: f64,
    ry: f64,
    rz: f64,
    scale: f64,
}

impl Helmert {
    /// `translations_*` en mètres, `rotation_*_arcsec` en secondes d'arc,
    /// `scale_ppm` en parties par million.
    ///
    /// # Errors
    ///
    /// [`ProjectionError::NonFinite`] si un paramètre ne l'est pas.
    pub fn try_new(params: HelmertParams) -> Result<Self, ProjectionError> {
        let numbers = [
            params.translation_x,
            params.translation_y,
            params.translation_z,
            params.rotation_x_arcsec,
            params.rotation_y_arcsec,
            params.rotation_z_arcsec,
            params.scale_ppm,
        ];
        if !numbers.iter().all(|value| value.is_finite()) {
            return Err(ProjectionError::NonFinite);
        }
        let sign = match params.convention {
            HelmertConvention::PositionVector => 1.0,
            HelmertConvention::CoordinateFrame => -1.0,
        };
        let arcsec = std::f64::consts::PI / (180.0 * 3600.0);
        Ok(Self {
            tx: params.translation_x,
            ty: params.translation_y,
            tz: params.translation_z,
            rx: sign * params.rotation_x_arcsec * arcsec,
            ry: sign * params.rotation_y_arcsec * arcsec,
            rz: sign * params.rotation_z_arcsec * arcsec,
            scale: 1.0 + params.scale_ppm * 1e-6,
        })
    }

    /// Applique `(1 + s) R x + t` avec la matrice de rotation linéarisée de PROJ.
    #[must_use]
    pub fn apply(&self, x: f64, y: f64, z: f64) -> (f64, f64, f64) {
        let rotated_x = x - self.rz * y + self.ry * z;
        let rotated_y = self.rz * x + y - self.rx * z;
        let rotated_z = -self.ry * x + self.rx * y + z;
        (
            self.scale * rotated_x + self.tx,
            self.scale * rotated_y + self.ty,
            self.scale * rotated_z + self.tz,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[expect(
        clippy::too_many_arguments,
        reason = "test helper passes the seven PROJ parameters plus the convention"
    )]
    #[expect(
        clippy::similar_names,
        reason = "rotation_x/y/z_arcsec are the PROJ +rx +ry +rz names"
    )]
    fn params(
        translation_x: f64,
        translation_y: f64,
        translation_z: f64,
        rotation_x_arcsec: f64,
        rotation_y_arcsec: f64,
        rotation_z_arcsec: f64,
        scale_ppm: f64,
        convention: HelmertConvention,
    ) -> HelmertParams {
        HelmertParams {
            translation_x,
            translation_y,
            translation_z,
            rotation_x_arcsec,
            rotation_y_arcsec,
            rotation_z_arcsec,
            scale_ppm,
            convention,
        }
    }

    fn close(got: (f64, f64, f64), expect: (f64, f64, f64)) {
        for (value, reference) in [got.0, got.1, got.2]
            .into_iter()
            .zip([expect.0, expect.1, expect.2])
        {
            assert!((value - reference).abs() < 1e-6, "{value} vs {reference}");
        }
    }

    #[test]
    fn proj_translation() {
        let h = Helmert::try_new(params(
            1.0,
            -2.0,
            3.0,
            0.0,
            0.0,
            0.0,
            0.0,
            HelmertConvention::PositionVector,
        ))
        .expect("helmert");
        close(h.apply(100.0, 200.0, 300.0), (101.0, 198.0, 303.0));
    }

    #[test]
    fn proj_rotation_position_vector() {
        let h = Helmert::try_new(params(
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            0.0,
            0.0,
            HelmertConvention::PositionVector,
        ))
        .expect("helmert");
        close(
            h.apply(100.0, 200.0, 300.0),
            (100.0, 199.998_545_56, 300.000_969_63),
        );
    }

    #[test]
    fn proj_rotation_coordinate_frame_flips_sign() {
        let h = Helmert::try_new(params(
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            0.0,
            0.0,
            HelmertConvention::CoordinateFrame,
        ))
        .expect("helmert");
        close(
            h.apply(100.0, 200.0, 300.0),
            (100.0, 200.001_454_44, 299.999_030_37),
        );
    }

    #[test]
    fn proj_scale_is_parts_per_million() {
        let h = Helmert::try_new(params(
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
            HelmertConvention::PositionVector,
        ))
        .expect("helmert");
        close(
            h.apply(100.0, 200.0, 300.0),
            (100.000_1, 200.000_2, 300.000_3),
        );
    }

    #[test]
    fn proj_combined_matches_cct() {
        let h = Helmert::try_new(params(
            0.5,
            -1.0,
            2.0,
            -0.5,
            0.25,
            1.5,
            2.5,
            HelmertConvention::PositionVector,
        ))
        .expect("helmert");
        close(
            h.apply(1.0, 2.0, 3.0),
            (1.499_991_59, 1.000_019_54, 5.000_001_44),
        );
    }
}
