#!/usr/bin/env python3
"""Write src/conversions.rs: the proj_create_conversion_* functions of proj.h.

usage: gen_conversions.py OUTPUT          (run from the repository root)

Each C function takes its angles and lengths in the units it is given. The model takes degrees
and metres, so the generated body converts them with `Units` and calls the creator of the model
(`Conversion::create_*`), or `Conversion::create_by_name` for a method that has no creator of
its own. The order of the arguments is the one of proj.h, which is the order of the creators.
"""
import sys

sys.path.insert(0, "capi/tools")
from sigs import signatures  # noqa: E402

DIRECT = {  # suffix -> creator of the model
    "transverse_mercator": "create_transverse_mercator",
    "gauss_schreiber_transverse_mercator": "create_gauss_schreiber_transverse_mercator",
    "transverse_mercator_south_oriented": "create_transverse_mercator_south_oriented",
    "two_point_equidistant": "create_two_point_equidistant",
    "tunisia_mining_grid": "create_tunisia_mining_grid",
    "albers_equal_area": "create_albers_equal_area",
    "lambert_conic_conformal_1sp": "create_lambert_conic_conformal_1sp",
    "lambert_conic_conformal_1sp_variant_b": "create_lambert_conic_conformal_1sp_variant_b",
    "lambert_conic_conformal_2sp": "create_lambert_conic_conformal_2sp",
    "lambert_conic_conformal_2sp_michigan": "create_lambert_conic_conformal_2sp_michigan",
    "lambert_conic_conformal_2sp_belgium": "create_lambert_conic_conformal_2sp_belgium",
    "azimuthal_equidistant": "create_azimuthal_equidistant",
    "guam_projection": "create_guam_projection",
    "bonne": "create_bonne",
    "lambert_cylindrical_equal_area_spherical": "create_lambert_cylindrical_equal_area_spherical",
    "lambert_cylindrical_equal_area": "create_lambert_cylindrical_equal_area",
    "cassini_soldner": "create_cassini_soldner",
    "equidistant_conic": "create_equidistant_conic",
    "equidistant_cylindrical": "create_equidistant_cylindrical",
    "equidistant_cylindrical_spherical": "create_equidistant_cylindrical_spherical",
    "gall": "create_gall",
    "goode_homolosine": "create_goode_homolosine",
    "interrupted_goode_homolosine": "create_interrupted_goode_homolosine",
    "geostationary_satellite_sweep_x": "create_geostationary_satellite_sweep_x",
    "geostationary_satellite_sweep_y": "create_geostationary_satellite_sweep_y",
    "gnomonic": "create_gnomonic",
    "hotine_oblique_mercator_variant_a": "create_hotine_oblique_mercator_variant_a",
    "hotine_oblique_mercator_variant_b": "create_hotine_oblique_mercator_variant_b",
    "hotine_oblique_mercator_two_point_natural_origin": "create_hotine_oblique_mercator_two_point_natural_origin",
    "laborde_oblique_mercator": "create_laborde_oblique_mercator",
    "international_map_world_polyconic": "create_international_map_world_polyconic",
    "krovak_north_oriented": "create_krovak_north_oriented",
    "krovak": "create_krovak",
    "lambert_azimuthal_equal_area": "create_lambert_azimuthal_equal_area",
    "miller_cylindrical": "create_miller_cylindrical",
    "mercator_variant_a": "create_mercator_variant_a",
    "mercator_variant_b": "create_mercator_variant_b",
    "popular_visualisation_pseudo_mercator": "create_popular_visualisation_pseudo_mercator",
    "mollweide": "create_mollweide",
    "polar_stereographic_variant_a": "create_polar_stereographic_variant_a",
    "polar_stereographic_variant_b": "create_polar_stereographic_variant_b",
}
ECKERT = {"eckert_i": 1, "eckert_ii": 2, "eckert_iii": 3, "eckert_iv": 4, "eckert_v": 5, "eckert_vi": 6}
BY_NAME = {
    "tunisia_mapping_grid": "Tunisia Mapping Grid",
    "new_zealand_mapping_grid": "New Zealand Map Grid",
    "oblique_stereographic": "Oblique Stereographic",
    "orthographic": "Orthographic",
    "local_orthographic": "Local Orthographic",
    "american_polyconic": "American Polyconic",
    "robinson": "Robinson",
    "sinusoidal": "Sinusoidal",
    "stereographic": "Stereographic",
    "van_der_grinten": "Van der Grinten",
    "wagner_i": "Wagner I",
    "wagner_ii": "Wagner II",
    "wagner_iii": "Wagner III",
    "wagner_iv": "Wagner IV",
    "wagner_v": "Wagner V",
    "wagner_vi": "Wagner VI",
    "wagner_vii": "Wagner VII",
    "quadrilateralized_spherical_cube": "Quadrilateralized Spherical Cube",
    "spherical_cross_track_height": "Spherical Cross-Track Height",
    "equal_earth": "Equal Earth",
    "vertical_perspective": "Vertical Perspective",
    "pole_rotation_grib_convention": "Pole rotation (GRIB convention)",
    "pole_rotation_netcdf_cf_convention": "Pole rotation (netCDF CF convention)",
}


def kind(name: str) -> str:
    if name.startswith("scale") or name == "ellipsoid_scaling_factor":
        return "scale"
    if any(word in name for word in ("lat", "long", "azimuth", "heading", "axis_rotation", "angle_from")):
        return "angle"
    return "length"


def main() -> int:
    sigs = signatures()
    out = [
        "//! The `proj_create_conversion_*` functions of `proj.h`.",
        "//!",
        "//! Generated by `capi/tools/gen_conversions.py`: do not edit. Angles and lengths come in the",
        "//! units of the call and go to the model in degrees and metres.",
        "",
        "use std::ffi::c_char;",
        "",
        "use proj_rs::iso19111::{Conversion, OpProps, ParameterValue};",
        "",
        "use crate::conversion_support::{finish, Units};",
        "use crate::ffi::*;",
        "use crate::util::guard;",
        "",
    ]
    count = 0
    for name, (params, _ret) in sigs.items():
        if not name.startswith("proj_create_conversion_"):
            continue
        suffix = name[len("proj_create_conversion_") :]
        if suffix == "utm":
            continue
        floats = [n for n, t in params if t == "f64" and "conv_factor" not in n]
        ctypes = ", ".join(
            f"{n}: {t.replace('::core::ffi::', '')}" for n, t in params
        )
        names = [n for n, _ in params]
        ang = "ang_unit_name, ang_unit_conv_factor" if "ang_unit_name" in names else "std::ptr::null(), 0.0"
        lin = (
            "linear_unit_name, linear_unit_conv_factor"
            if "linear_unit_name" in names
            else "std::ptr::null(), 0.0"
        )
        units = f"Units::new({ang}, {lin})"
        values = []
        for n in floats:
            values.append((n, kind(n)))
        if suffix in DIRECT or suffix in ECKERT:
            args = ", ".join(f"units.{k}({n})" for n, k in values)
            if suffix in ECKERT:
                call = f"Conversion::create_eckert(&OpProps::new(), {ECKERT[suffix]}, {args})"
            else:
                call = f"Conversion::{DIRECT[suffix]}(&OpProps::new(), {args})"
        else:
            vals = ", ".join(f"ParameterValue::{k}(units.{k}({n}))" for n, k in values)
            call = f'Conversion::create_by_name(&OpProps::new(), "{BY_NAME[suffix]}", vec![{vals}])'
        out += [
            "#[unsafe(no_mangle)]",
            f'pub extern "C" fn {name}({ctypes}) -> *mut PJ {{',
            "    guard(std::ptr::null_mut(), || {",
            f"        let units = {units};",
            f"        finish(ctx, {call}, \"{name}\")",
            "    })",
            "}",
            "",
        ]
        count += 1
    open(sys.argv[1], "w", encoding="utf-8").write("\n".join(out))
    print(f"{count} functions", file=sys.stderr)
    return 0


sys.exit(main())
