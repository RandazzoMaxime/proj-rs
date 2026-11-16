#![allow(
    clippy::print_stdout,
    reason = "an example prints its result for the reader"
)]

//! Convert a point between two coordinate reference systems.
//!
//! ```bash
//! cargo run --example quickstart
//! ```

use proj_rs::{Context, Crs, Point, Transformer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // EPSG codes resolve from the embedded PROJ 9.9 database. No libproj.
    let ctx = Context::default();
    let wgs84 = Crs::from_user_input(&ctx, "EPSG:4326")?;
    let lambert93 = Crs::from_user_input(&ctx, "EPSG:2154")?;

    // The Eiffel Tower. Without `normalize_axis_order`, points are in the axis order of the
    // CRS: EPSG:4326 is latitude, longitude.
    let to_lambert93 = Transformer::builder(&wgs84, &lambert93).build()?;
    let p = to_lambert93.forward(Point::new(48.8584, 2.2945))?;
    println!("Lambert-93 (EPSG:2154): {:.3} E  {:.3} N", p.x, p.y);

    // `normalize_axis_order` reads longitude, latitude, as a GIS does.
    let gis = Transformer::builder(&wgs84, &lambert93)
        .normalize_axis_order()
        .build()?;
    let p = gis.forward(Point::new(2.2945, 48.8584))?;
    println!("Same, longitude first:  {:.3} E  {:.3} N", p.x, p.y);

    // A PROJ string is a CRS too.
    let utm31 = Crs::from_user_input(&ctx, "+proj=utm +zone=31 +ellps=WGS84 +type=crs")?;
    let to_utm = Transformer::builder(&wgs84, &utm31).build()?;
    let p = to_utm.forward(Point::new(48.8584, 2.2945))?;
    println!("UTM 31N (PROJ string):  {:.3} E  {:.3} N", p.x, p.y);

    // Anything this crate cannot do is an error, never a wrong number.
    let refused = Crs::from_user_input(&ctx, "+proj=not_a_real_projection +type=crs");
    println!("Unknown projection:     {}", refused.is_err());

    Ok(())
}
