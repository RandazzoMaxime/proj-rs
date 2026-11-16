//! `substitutePROJAlternativeGridNames` for the methods whose grid is a velocity or deformation
//! file (`singleoperation.cpp:2639-2990`): the replacement drops the version, writes the
//! identifiers as `DERIVED_FROM(EPSG)` and keeps the order of the parameters.
//!
//! Expected values: PROJ 9.9.0 `projinfo -k operation EPSG:<code> -o WKT2:2019 -q`, reduced to the
//! lines at the operation level that name a method, a parameter, a version or an identifier.

use std::process::Command;

fn operation_lines(code: &str) -> Vec<String> {
    let output = Command::new(env!("CARGO_BIN_EXE_projinfo"))
        .args([
            "-k",
            "operation",
            &format!("EPSG:{code}"),
            "-o",
            "WKT2:2019",
            "-q",
        ])
        .output()
        .expect("run projinfo");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("utf-8")
        .lines()
        .filter(|line| {
            ["METHOD[", "PARAMETER[", "PARAMETERFILE[", "VERSION[", "ID["]
                .iter()
                .any(|prefix| {
                    line.strip_prefix("    ")
                        .is_some_and(|rest| rest.starts_with(prefix))
                })
        })
        .map(str::to_owned)
        .collect()
}

/// Method 1143: the velocity grid file is replaced where it stands, after the other parameters.
#[test]
fn operation_10813_position_vector_with_velocity_grid() {
    assert_eq!(
        operation_lines("10813"),
        [
            r#"    METHOD["Position Vector (geocen) & Geocen translations NEU velocities (gtg)","#,
            r#"    PARAMETER["X-axis translation",0.03054,"#,
            r#"    PARAMETER["Y-axis translation",0.04606,"#,
            r#"    PARAMETER["Z-axis translation",-0.07944,"#,
            r#"    PARAMETER["X-axis rotation",1.41958,"#,
            r#"    PARAMETER["Y-axis rotation",0.15132,"#,
            r#"    PARAMETER["Z-axis rotation",1.50337,"#,
            r#"    PARAMETER["Scale difference",3.002,"#,
            r#"    PARAMETER["Source epoch",2000,"#,
            r#"    PARAMETER["Target epoch",1999.5,"#,
            r#"    PARAMETERFILE["Point motion velocity grid file","eur_nkg_nkgrf17vel.tif"],"#,
            r#"    ID["DERIVED_FROM(EPSG)",10813],"#,
        ]
    );
}

/// Method 1142: both files are replaced, the translation file keeps its name.
#[test]
fn operation_10811_two_grid_files() {
    assert_eq!(
        operation_lines("10811"),
        [
            r#"    METHOD["Geocen translations by grid (gtg) & Geocen translations NEU velocities (gtg)","#,
            r#"    PARAMETERFILE["Geocentric translation file","no_kv_NKGETRF14_EPSG7922_2000.tif"],"#,
            r#"    PARAMETERFILE["Point motion velocity grid file","eur_nkg_nkgrf17vel.tif"],"#,
            r#"    PARAMETER["Source epoch",2000,"#,
            r#"    PARAMETER["Target epoch",1995,"#,
            r#"    ID["DERIVED_FROM(EPSG)",10811],"#,
        ]
    );
}

/// Method 1144.
#[test]
fn operation_10809_geocentric_translations_with_velocity_grid() {
    assert_eq!(
        operation_lines("10809"),
        [
            r#"    METHOD["Geocentric translations using NEU velocity grid (gtg)","#,
            r#"    PARAMETERFILE["Point motion velocity grid file","eur_nkg_nkgrf17vel.tif"],"#,
            r#"    PARAMETER["Target epoch",2000,"#,
            r#"    ID["DERIVED_FROM(EPSG)",10809],"#,
        ]
    );
}

/// Method 1079, the `gridTransformations` table: the version `LINZ-Nzl 2018-07-01` is dropped.
#[test]
fn operation_9090_new_zealand_deformation_model() {
    assert_eq!(
        operation_lines("9090"),
        [
            r#"    METHOD["New Zealand Deformation Model","#,
            r#"    PARAMETERFILE["Point motion velocity grid file","nz_linz_nzgd2000-20180701.json"],"#,
            r#"    ID["DERIVED_FROM(EPSG)",9090],"#,
        ]
    );
}

/// Method 1113, the vertical offset by velocity grid: the version `EPSG-Can` is dropped.
#[test]
fn operation_10542_vertical_offset_by_velocity_grid() {
    assert_eq!(
        operation_lines("10542"),
        [
            r#"    METHOD["Vertical Offset using NEU velocity grid (NTv2_Vel)","#,
            r#"    PARAMETERFILE["Point motion velocity grid file","ca_nrc_NAD83v70VG.tif"],"#,
            r#"    ID["DERIVED_FROM(EPSG)",10542],"#,
        ]
    );
}

/// Method 1114, the geographic 3D offset by velocity grid.
#[test]
fn operation_10119_geographic_3d_offset_by_velocity_grid() {
    assert_eq!(
        operation_lines("10119"),
        [
            r#"    METHOD["Geographic3D Offset using NEU velocity grid (NTv2_Vel)","#,
            r#"    PARAMETERFILE["Point motion velocity grid file","ca_nrc_NAD83v70VG.tif"],"#,
            r#"    ID["DERIVED_FROM(EPSG)",10119]]"#,
        ]
    );
}
