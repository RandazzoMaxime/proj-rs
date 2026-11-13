//! The lists `proj -l` and `geod -l` print, as `proj.cpp` and `geod.cpp` format them.

use proj_rs::listing::{linear_units, ELLIPSOIDS};

/// The ellipsoids of `proj_list_ellps`: id, `a=`, `rf=` or `b=`, and name.
pub(crate) fn ellipsoids() -> String {
    let mut out = String::new();
    for entry in ELLIPSOIDS {
        out.push_str(&format!(
            "{:>9} {:<16} {:<16} {}\n",
            entry.id, entry.major, entry.ell, entry.name
        ));
    }
    out
}

/// The linear units of proj.db that have a PROJ short name: short name, factor to metres, and
/// name. Nothing when proj.db cannot be read, as `proj_get_units_from_database` gives `NULL`.
pub(crate) fn units() -> String {
    let mut out = String::new();
    for unit in linear_units().unwrap_or_default() {
        let factor = crate::dms::format_number("%.15g", unit.conv_factor).unwrap_or_default();
        out.push_str(&format!(
            "{:>12} {:<20} {}\n",
            unit.short_name, factor, unit.name
        ));
    }
    out
}
