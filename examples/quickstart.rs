#![allow(
    clippy::print_stdout,
    reason = "an example prints its result for the reader"
)]

//! Convert a point between two coordinate reference systems.
//!
//! ```bash
//! cargo run --example quickstart
//! ```

use proj_rs::Proj;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The Eiffel Tower, in WGS 84 longitude and latitude (degrees).
    let eiffel = (2.2945_f64, 48.8584_f64);

    // EPSG codes resolve from the embedded PROJ 9.9 database. No libproj.
    let to_lambert93 = Proj::new_known_crs("EPSG:4326", "EPSG:2154", None)?;
    let (east, north) = to_lambert93.convert(eiffel)?;
    println!("Lambert-93 (EPSG:2154): {east:.3} E  {north:.3} N");

    // A plain PROJ string works too.
    let to_utm = Proj::new("+proj=utm +zone=31 +ellps=WGS84")?;
    let radians = (eiffel.0.to_radians(), eiffel.1.to_radians());
    let (east, north) = to_utm.project(radians, false)?;
    println!("UTM 31N (PROJ string):  {east:.3} E  {north:.3} N");

    // Anything this crate cannot do is an error at creation, never a wrong number.
    let refused = Proj::new("+proj=not_a_real_projection");
    println!("Unknown projection:     {}", refused.is_err());

    Ok(())
}
