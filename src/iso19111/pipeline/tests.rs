//! PROJ `test/unit/test_io.cpp`, `io.projstringformatter*`: the PROJ string a formatter
//! prints from steps added with `addStep` and `addParam`, or parsed with
//! `ingestPROJString`. Each function is one `TEST()`.
//!
//! The formatter is crate-private, so these tests live beside it. A PROJ `addParam`
//! with a number is `add_number`, with a string `add_param`, and without a value
//! `add_flag`. `setCurrentStepInverted(true)` is `invert_last_step`.

use super::{PipelineFormatter, ProjLines};

fn print(fmt: PipelineFormatter) -> String {
    fmt.finish(ProjLines::Single)
}

#[test]
fn io_projstringformatter() {
    let mut fmt = PipelineFormatter::default();
    fmt.add_step("my_proj");
    assert_eq!(print(fmt), "+proj=my_proj");

    let mut fmt = PipelineFormatter::default();
    fmt.add_step("my_proj");
    fmt.invert_last_step();
    assert_eq!(print(fmt), "+proj=pipeline +step +inv +proj=my_proj");

    let mut fmt = PipelineFormatter::default();
    fmt.add_step("my_proj1");
    fmt.add_step("my_proj2");
    assert_eq!(
        print(fmt),
        "+proj=pipeline +step +proj=my_proj1 +step +proj=my_proj2"
    );

    let mut fmt = PipelineFormatter::default();
    fmt.add_step("my_proj1");
    fmt.invert_last_step();
    fmt.add_step("my_proj2");
    assert_eq!(
        print(fmt),
        "+proj=pipeline +step +inv +proj=my_proj1 +step +proj=my_proj2"
    );

    let mut fmt = PipelineFormatter::default();
    fmt.start_inversion();
    fmt.add_step("my_proj1");
    fmt.invert_last_step();
    fmt.add_step("my_proj2");
    fmt.stop_inversion();
    assert_eq!(
        print(fmt),
        "+proj=pipeline +step +inv +proj=my_proj2 +step +proj=my_proj1"
    );

    let mut fmt = PipelineFormatter::default();
    fmt.start_inversion();
    fmt.add_step("my_proj1");
    fmt.invert_last_step();
    fmt.start_inversion();
    fmt.add_step("my_proj2");
    fmt.stop_inversion();
    fmt.stop_inversion();
    assert_eq!(
        print(fmt),
        "+proj=pipeline +step +proj=my_proj2 +step +proj=my_proj1"
    );
}

#[test]
fn io_projstringformatter_helmert_3_param_noop() {
    let mut fmt = PipelineFormatter::default();
    fmt.add_step("helmert");
    fmt.add_number("x", 0.0);
    fmt.add_number("y", 0.0);
    fmt.add_number("z", 0.0);
    assert_eq!(print(fmt), "+proj=noop");
}

#[test]
fn io_projstringformatter_helmert_7_param_noop() {
    let mut fmt = PipelineFormatter::default();
    fmt.add_step("helmert");
    fmt.add_number("x", 0.0);
    fmt.add_number("y", 0.0);
    fmt.add_number("z", 0.0);
    fmt.add_number("rx", 0.0);
    fmt.add_number("ry", 0.0);
    fmt.add_number("rz", 0.0);
    fmt.add_number("s", 0.0);
    fmt.add_param("convention", "position_vector");
    assert_eq!(print(fmt), "+proj=noop");
}

#[test]
fn io_projstringformatter_merge_consecutive_helmert_3_param() {
    let mut fmt = PipelineFormatter::default();
    fmt.add_step("helmert");
    fmt.add_number("x", 10.0);
    fmt.add_number("y", 20.0);
    fmt.add_number("z", 30.0);
    fmt.add_step("helmert");
    fmt.add_number("x", -1.0);
    fmt.add_number("y", -2.0);
    fmt.add_number("z", -3.0);
    assert_eq!(print(fmt), "+proj=helmert +x=9 +y=18 +z=27");

    let mut fmt = PipelineFormatter::default();
    fmt.add_step("helmert");
    fmt.add_number("x", 10.0);
    fmt.add_number("y", 20.0);
    fmt.add_number("z", 30.0);
    fmt.add_step("helmert");
    fmt.invert_last_step();
    fmt.add_number("x", -10.0);
    fmt.add_number("y", -20.0);
    fmt.add_number("z", -30.0);
    assert_eq!(print(fmt), "+proj=helmert +x=20 +y=40 +z=60");

    let mut fmt = PipelineFormatter::default();
    fmt.add_step("helmert");
    fmt.invert_last_step();
    fmt.add_number("x", 10.0);
    fmt.add_number("y", 20.0);
    fmt.add_number("z", 30.0);
    fmt.add_step("helmert");
    fmt.add_number("x", -10.0);
    fmt.add_number("y", -20.0);
    fmt.add_number("z", -30.0);
    assert_eq!(print(fmt), "+proj=helmert +x=-20 +y=-40 +z=-60");
}

#[test]
fn io_projstringformatter_merge_consecutive_helmert_3_param_noop() {
    let mut fmt = PipelineFormatter::default();
    fmt.add_step("helmert");
    fmt.add_number("x", 10.0);
    fmt.add_number("y", 20.0);
    fmt.add_number("z", 30.0);
    fmt.add_step("helmert");
    fmt.add_number("x", -10.0);
    fmt.add_number("y", -20.0);
    fmt.add_number("z", -30.0);
    assert_eq!(print(fmt), "+proj=noop");

    let mut fmt = PipelineFormatter::default();
    fmt.add_step("helmert");
    fmt.invert_last_step();
    fmt.add_number("x", 10.0);
    fmt.add_number("y", 20.0);
    fmt.add_number("z", 30.0);
    fmt.add_step("helmert");
    fmt.invert_last_step();
    fmt.add_number("x", -10.0);
    fmt.add_number("y", -20.0);
    fmt.add_number("z", -30.0);
    assert_eq!(print(fmt), "+proj=noop");
}

#[test]
fn io_projstringformatter_merge_consecutive_helmert_7_param_noop() {
    let mut fmt = PipelineFormatter::default();
    fmt.add_step("helmert");
    fmt.add_number("x", 10.0);
    fmt.add_number("y", 20.0);
    fmt.add_number("z", 30.0);
    fmt.add_number("rx", 1.0);
    fmt.add_number("ry", 2.0);
    fmt.add_number("rz", 3.0);
    fmt.add_number("s", 4.0);
    fmt.add_param("convention", "position_vector");
    fmt.add_step("helmert");
    fmt.add_number("x", -10.0);
    fmt.add_number("y", -20.0);
    fmt.add_number("z", -30.0);
    fmt.add_number("rx", -1.0);
    fmt.add_number("ry", -2.0);
    fmt.add_number("rz", -3.0);
    fmt.add_number("s", -4.0);
    fmt.add_param("convention", "position_vector");
    assert_eq!(print(fmt), "+proj=noop");

    let mut fmt = PipelineFormatter::default();
    fmt.add_step("helmert");
    fmt.invert_last_step();
    fmt.add_number("x", 10.0);
    fmt.add_number("y", 20.0);
    fmt.add_number("z", 30.0);
    fmt.add_number("rx", 1.0);
    fmt.add_number("ry", 2.0);
    fmt.add_number("rz", 3.0);
    fmt.add_number("s", 4.0);
    fmt.add_param("convention", "position_vector");
    fmt.add_step("helmert");
    fmt.invert_last_step();
    fmt.add_number("x", -10.0);
    fmt.add_number("y", -20.0);
    fmt.add_number("z", -30.0);
    fmt.add_number("rx", -1.0);
    fmt.add_number("ry", -2.0);
    fmt.add_number("rz", -3.0);
    fmt.add_number("s", -4.0);
    fmt.add_param("convention", "position_vector");
    assert_eq!(print(fmt), "+proj=noop");
}

#[test]
fn io_projstringformatter_merge_inverted_helmert_with_opposite_conventions() {
    let mut fmt = PipelineFormatter::default();
    fmt.add_step("helmert");
    fmt.add_number("x", 10.0);
    fmt.add_number("y", 20.0);
    fmt.add_number("z", 30.0);
    fmt.add_number("rx", 1.0);
    fmt.add_number("ry", 2.0);
    fmt.add_number("rz", 3.0);
    fmt.add_number("s", 4.0);
    fmt.add_param("convention", "position_vector");
    fmt.add_step("helmert");
    fmt.invert_last_step();
    fmt.add_number("x", 10.0);
    fmt.add_number("y", 20.0);
    fmt.add_number("z", 30.0);
    fmt.add_number("rx", -1.0);
    fmt.add_number("ry", -2.0);
    fmt.add_number("rz", -3.0);
    fmt.add_number("s", 4.0);
    fmt.add_param("convention", "coordinate_frame");
    assert_eq!(print(fmt), "+proj=noop");

    let mut fmt = PipelineFormatter::default();
    fmt.add_step("helmert");
    fmt.invert_last_step();
    fmt.add_number("x", 10.0);
    fmt.add_number("y", 20.0);
    fmt.add_number("z", 30.0);
    fmt.add_number("rx", 1.0);
    fmt.add_number("ry", 2.0);
    fmt.add_number("rz", 3.0);
    fmt.add_number("s", 4.0);
    fmt.add_param("convention", "position_vector");
    fmt.add_step("helmert");
    fmt.add_number("x", 10.0);
    fmt.add_number("y", 20.0);
    fmt.add_number("z", 30.0);
    fmt.add_number("rx", -1.0);
    fmt.add_number("ry", -2.0);
    fmt.add_number("rz", -3.0);
    fmt.add_number("s", 4.0);
    fmt.add_param("convention", "coordinate_frame");
    assert_eq!(print(fmt), "+proj=noop");

    let mut fmt = PipelineFormatter::default();
    fmt.add_step("helmert");
    fmt.invert_last_step();
    fmt.add_number("x", 10.0);
    fmt.add_number("y", 20.0);
    fmt.add_number("z", 30.0);
    fmt.add_number("rx", 1.0);
    fmt.add_number("ry", 2.0);
    fmt.add_number("rz", 3.0);
    fmt.add_number("s", 4.0);
    fmt.add_param("convention", "coordinate_frame");
    fmt.add_step("helmert");
    fmt.add_number("x", 10.0);
    fmt.add_number("y", 20.0);
    fmt.add_number("z", 30.0);
    fmt.add_number("rx", -1.0);
    fmt.add_number("ry", -2.0);
    fmt.add_number("rz", -3.0);
    fmt.add_number("s", 4.0);
    fmt.add_param("convention", "position_vector");
    assert_eq!(print(fmt), "+proj=noop");

    // Cannot be optimized: the second step is not inverted.
    let mut fmt = PipelineFormatter::default();
    fmt.add_step("helmert");
    fmt.add_number("x", 10.0);
    fmt.add_number("y", 20.0);
    fmt.add_number("z", 30.0);
    fmt.add_number("rx", 1.0);
    fmt.add_number("ry", 2.0);
    fmt.add_number("rz", 3.0);
    fmt.add_number("s", 4.0);
    fmt.add_param("convention", "position_vector");
    fmt.add_step("helmert");
    fmt.add_number("x", 10.0);
    fmt.add_number("y", 20.0);
    fmt.add_number("z", 30.0);
    fmt.add_number("rx", -1.0);
    fmt.add_number("ry", -2.0);
    fmt.add_number("rz", -3.0);
    fmt.add_number("s", 4.0);
    fmt.add_param("convention", "coordinate_frame");
    assert_ne!(print(fmt), "+proj=noop");

    // Cannot be optimized: the conventions match, but the second step is inverted.
    let mut fmt = PipelineFormatter::default();
    fmt.add_step("helmert");
    fmt.add_number("x", 10.0);
    fmt.add_number("y", 20.0);
    fmt.add_number("z", 30.0);
    fmt.add_number("rx", 1.0);
    fmt.add_number("ry", 2.0);
    fmt.add_number("rz", 3.0);
    fmt.add_number("s", 4.0);
    fmt.add_param("convention", "position_vector");
    fmt.add_step("helmert");
    fmt.invert_last_step();
    fmt.add_number("x", 10.0);
    fmt.add_number("y", 20.0);
    fmt.add_number("z", 30.0);
    fmt.add_number("rx", -1.0);
    fmt.add_number("ry", -2.0);
    fmt.add_number("rz", -3.0);
    fmt.add_number("s", 4.0);
    fmt.add_param("convention", "position_vector");
    assert_ne!(print(fmt), "+proj=noop");

    // Cannot be optimized: the translations differ in sign.
    let mut fmt = PipelineFormatter::default();
    fmt.add_step("helmert");
    fmt.add_number("x", 10.0);
    fmt.add_number("y", 20.0);
    fmt.add_number("z", 30.0);
    fmt.add_number("rx", 1.0);
    fmt.add_number("ry", 2.0);
    fmt.add_number("rz", 3.0);
    fmt.add_number("s", 4.0);
    fmt.add_param("convention", "position_vector");
    fmt.add_step("helmert");
    fmt.invert_last_step();
    fmt.add_number("x", -10.0);
    fmt.add_number("y", 20.0);
    fmt.add_number("z", 30.0);
    fmt.add_number("rx", -1.0);
    fmt.add_number("ry", -2.0);
    fmt.add_number("rz", -3.0);
    fmt.add_number("s", 4.0);
    fmt.add_param("convention", "coordinate_frame");
    assert_ne!(print(fmt), "+proj=noop");

    // Cannot be optimized: the rotations differ.
    let mut fmt = PipelineFormatter::default();
    fmt.add_step("helmert");
    fmt.add_number("x", 10.0);
    fmt.add_number("y", 20.0);
    fmt.add_number("z", 30.0);
    fmt.add_number("rx", 1.0);
    fmt.add_number("ry", 3.0);
    fmt.add_number("rz", 2.0);
    fmt.add_number("s", 4.0);
    fmt.add_param("convention", "position_vector");
    fmt.add_step("helmert");
    fmt.invert_last_step();
    fmt.add_number("x", 10.0);
    fmt.add_number("y", 20.0);
    fmt.add_number("z", 30.0);
    fmt.add_number("rx", 1.0);
    fmt.add_number("ry", -2.0);
    fmt.add_number("rz", -3.0);
    fmt.add_number("s", 4.0);
    fmt.add_param("convention", "coordinate_frame");
    assert_ne!(print(fmt), "+proj=noop");
}

#[test]
fn io_projstringformatter_cart_grs80_wgs84() {
    let mut fmt = PipelineFormatter::default();
    fmt.add_step("cart");
    fmt.add_param("ellps", "WGS84");
    fmt.add_step("cart");
    fmt.invert_last_step();
    fmt.add_param("ellps", "GRS80");
    assert_eq!(print(fmt), "+proj=noop");
}

#[test]
fn io_projstringformatter_axisswap_unitconvert_axisswap() {
    let mut fmt = PipelineFormatter::default();
    fmt.add_step("axisswap");
    fmt.add_param("order", "2,1");
    fmt.add_step("unitconvert");
    fmt.add_param("xy_in", "rad");
    fmt.add_param("xy_out", "deg");
    fmt.add_step("axisswap");
    fmt.add_param("order", "2,1");
    assert_eq!(print(fmt), "+proj=unitconvert +xy_in=rad +xy_out=deg");
}

#[test]
fn io_projstringformatter_axisswap_one_minus_two_inv() {
    let mut fmt = PipelineFormatter::default();
    fmt.ingest_proj_string("+proj=pipeline +step +inv +proj=axisswap +order=1,-2");
    assert_eq!(print(fmt), "+proj=axisswap +order=1,-2");
}

#[test]
fn io_projstringformatter_axisswap_two_one_followed_two_minus_one() {
    let mut fmt = PipelineFormatter::default();
    fmt.ingest_proj_string(
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=axisswap +order=2,-1",
    );
    assert_eq!(print(fmt), "+proj=axisswap +order=1,-2");
}

#[test]
fn io_projstringformatter_axisswap_minus_two_one_followed_two_one() {
    let mut fmt = PipelineFormatter::default();
    fmt.ingest_proj_string(
        "+proj=pipeline +step +proj=axisswap +order=-2,1 +step +proj=axisswap +order=2,1",
    );
    assert_eq!(print(fmt), "+proj=axisswap +order=1,-2");
}

#[test]
fn io_projstringformatter_axisswap_two_minus_one_followed_minus_two_one() {
    let mut fmt = PipelineFormatter::default();
    fmt.ingest_proj_string(
        "+proj=pipeline +step +proj=axisswap +order=2,-1 +step +proj=axisswap +order=-2,1",
    );
    assert_eq!(print(fmt), "+proj=noop");
}

#[test]
fn io_projstringformatter_axisswap_two_minus_one_followed_one_minus_two() {
    let mut fmt = PipelineFormatter::default();
    fmt.ingest_proj_string(
        "+proj=pipeline +step +proj=axisswap +order=2,-1 +step +proj=axisswap +order=1,-2",
    );
    assert_eq!(print(fmt), "+proj=axisswap +order=2,1");
}

#[test]
fn io_projstringformatter_unitconvert() {
    // +step +proj=unitconvert +xy_in=X1 +xy_out=X2
    // +step +proj=unitconvert +xy_in=X2 +z_in=Z1 +xy_out=X1 +z_out=Z2
    // ==> +step +proj=unitconvert +z_in=Z1 +z_out=Z2
    let mut fmt = PipelineFormatter::default();
    fmt.ingest_proj_string(
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=ft",
    );
    assert_eq!(print(fmt), "+proj=unitconvert +z_in=m +z_out=ft");

    // +step +proj=unitconvert +xy_in=X1 +z_in=Z1 +xy_out=X2 +z_out=Z2
    // +step +proj=unitconvert +z_in=Z2 +z_out=Z3
    // ==> +step +proj=unitconvert +xy_in=X1 +z_in=Z1 +xy_out=X2 +z_out=Z3
    let mut fmt = PipelineFormatter::default();
    fmt.ingest_proj_string(
        "+proj=pipeline \
         +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=ft \
         +step +proj=unitconvert +z_in=ft +z_out=us-ft",
    );
    assert_eq!(
        print(fmt),
        "+proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=us-ft"
    );

    // +step +proj=unitconvert +z_in=Z1 +z_out=Z2
    // +step +proj=unitconvert +xy_in=X1 +z_in=Z2 +xy_out=X2 +z_out=Z3
    // ==> +step +proj=unitconvert +xy_in=X1 +z_in=Z1 +xy_out=X2 +z_out=Z3
    let mut fmt = PipelineFormatter::default();
    fmt.ingest_proj_string(
        "+proj=pipeline \
         +step +proj=unitconvert +z_in=ft +z_out=m \
         +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=us-ft ",
    );
    assert_eq!(
        print(fmt),
        "+proj=unitconvert +xy_in=deg +z_in=ft +xy_out=rad +z_out=us-ft"
    );

    // +step +proj=unitconvert +xy_in=X1 +z_in=Z1 +xy_out=X2 +z_out=Z2
    // +step +proj=unitconvert +xy_in=X2 +xy_out=X3
    // ==> +step +proj=unitconvert +xy_in=X1 +z_in=Z1 +xy_out=X3 +z_out=Z2
    let mut fmt = PipelineFormatter::default();
    fmt.ingest_proj_string(
        "+proj=pipeline \
         +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=ft \
         +step +proj=unitconvert +xy_in=rad +xy_out=grad",
    );
    assert_eq!(
        print(fmt),
        "+proj=unitconvert +xy_in=deg +z_in=m +xy_out=grad +z_out=ft"
    );

    // +step +proj=unitconvert +xy_in=X1 +z_in=Z1 +xy_out=X2 +z_out=Z2
    // +step +proj=unitconvert +xy_in=X2 +z_in=Z3 +xy_out=X3 +z_out=Z3
    // ==> +step +proj=unitconvert +xy_in=X1 +z_in=Z1 +xy_out=X3 +z_out=Z2
    let mut fmt = PipelineFormatter::default();
    fmt.ingest_proj_string(
        "+proj=pipeline \
         +step +proj=unitconvert +xy_in=deg +z_in=ft +xy_out=rad +z_out=us-ft \
         +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=grad +z_out=m",
    );
    assert_eq!(
        print(fmt),
        "+proj=unitconvert +xy_in=deg +z_in=ft +xy_out=grad +z_out=us-ft"
    );

    // +step +proj=unitconvert +xy_in=X1 +z_in=Z1 +xy_out=X2 +z_out=Z1
    // +step +proj=unitconvert +xy_in=X2 +z_in=Z2 +xy_out=X3 +z_out=Z3
    // ==> +step +proj=unitconvert +xy_in=X1 +z_in=Z2 +xy_out=X3 +z_out=Z3
    let mut fmt = PipelineFormatter::default();
    fmt.ingest_proj_string(
        "+proj=pipeline \
         +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m \
         +step +proj=unitconvert +xy_in=rad +z_in=ft +xy_out=grad +z_out=us-ft",
    );
    assert_eq!(
        print(fmt),
        "+proj=unitconvert +xy_in=deg +z_in=ft +xy_out=grad +z_out=us-ft"
    );
}

#[test]
fn io_projstringformatter_unmodified() {
    for text in [
        "+proj=pipeline +step +proj=axisswap +order=2,-1 +step +proj=axisswap +order=2,1",
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=axisswap +order=-2,1",
        "+proj=pipeline +step +inv +proj=axisswap +order=-2,1 +step +proj=axisswap +order=2,1",
    ] {
        let mut fmt = PipelineFormatter::default();
        fmt.ingest_proj_string(text);
        assert_eq!(print(fmt), text);
    }
}

/// TEST(io, projstringformatter_optim_hgridshift_vgridshift_hgridshift_inv)
#[test]
fn io_projstringformatter_optim_hgridshift_vgridshift_hgridshift_inv() {
    {
        let mut fmt = PipelineFormatter::default();
        fmt.add_step("hgridshift");
        fmt.add_param("grids", "foo");
        fmt.add_step("vgridshift");
        fmt.add_param("grids", "bar");
        fmt.start_inversion();
        fmt.add_step("hgridshift");
        fmt.add_param("grids", "foo");
        fmt.stop_inversion();
        assert_eq!(
            print(fmt),
            "+proj=pipeline \
             +step +proj=push +v_1 +v_2 \
             +step +proj=hgridshift +grids=foo +omit_inv \
             +step +proj=vgridshift +grids=bar \
             +step +inv +proj=hgridshift +grids=foo +omit_fwd \
             +step +proj=pop +v_1 +v_2"
        );
    }
    {
        let mut fmt = PipelineFormatter::default();
        fmt.start_inversion();
        fmt.ingest_proj_string("+proj=hgridshift +grids=foo +omit_fwd");
        fmt.stop_inversion();
        assert_eq!(
            print(fmt),
            "+proj=pipeline +step +inv +proj=hgridshift +grids=foo +omit_inv"
        );
    }
    {
        let mut fmt = PipelineFormatter::default();
        fmt.start_inversion();
        fmt.ingest_proj_string("+proj=hgridshift +grids=foo +omit_inv");
        fmt.stop_inversion();
        assert_eq!(
            print(fmt),
            "+proj=pipeline +step +inv +proj=hgridshift +grids=foo +omit_fwd"
        );
    }
    {
        let mut fmt = PipelineFormatter::default();
        fmt.start_inversion();
        fmt.add_step("hgridshift");
        fmt.add_param("grids", "foo");
        fmt.stop_inversion();
        fmt.add_step("vgridshift");
        fmt.add_param("grids", "bar");
        fmt.add_step("hgridshift");
        fmt.add_param("grids", "foo");
        assert_eq!(
            print(fmt),
            "+proj=pipeline \
             +step +proj=push +v_1 +v_2 \
             +step +inv +proj=hgridshift +grids=foo +omit_inv \
             +step +proj=vgridshift +grids=bar \
             +step +proj=hgridshift +grids=foo +omit_fwd \
             +step +proj=pop +v_1 +v_2"
        );
    }
    {
        let mut fmt = PipelineFormatter::default();
        fmt.add_step("hgridshift");
        fmt.add_param("grids", "foo");
        fmt.add_step("vgridshift");
        fmt.add_param("grids", "bar");
        fmt.start_inversion();
        fmt.add_step("hgridshift");
        fmt.add_param("grids", "foo2");
        fmt.stop_inversion();
        assert_eq!(
            print(fmt),
            "+proj=pipeline +step +proj=hgridshift +grids=foo \
             +step +proj=vgridshift +grids=bar \
             +step +inv +proj=hgridshift +grids=foo2"
        );
    }
    {
        let mut fmt = PipelineFormatter::default();
        fmt.add_step("hgridshift");
        fmt.add_param("grids", "foo");
        fmt.add_step("vgridshift");
        fmt.add_param("grids", "bar");
        fmt.add_step("hgridshift");
        fmt.add_param("grids", "foo");
        assert_eq!(
            print(fmt),
            "+proj=pipeline +step +proj=hgridshift +grids=foo \
             +step +proj=vgridshift +grids=bar \
             +step +proj=hgridshift +grids=foo"
        );
    }
}

/// TEST(io, projstringformatter_optim_as_uc_vgridshift_uc_as_push_as_uc)
#[test]
fn io_projstringformatter_optim_as_uc_vgridshift_uc_as_push_as_uc() {
    let mut fmt = PipelineFormatter::default();
    fmt.add_step("axisswap");
    fmt.add_param("order", "2,1");
    fmt.add_step("unitconvert");
    fmt.add_param("xy_in", "deg");
    fmt.add_param("xy_out", "rad");
    fmt.add_step("vgridshift");
    fmt.add_param("grids", "foo");
    fmt.add_step("unitconvert");
    fmt.add_param("xy_in", "rad");
    fmt.add_param("xy_out", "deg");
    fmt.add_step("axisswap");
    fmt.add_param("order", "2,1");
    fmt.add_step("push");
    fmt.add_flag("v_1");
    fmt.add_flag("v_2");
    fmt.add_step("axisswap");
    fmt.add_param("order", "2,1");
    fmt.add_step("unitconvert");
    fmt.add_param("xy_in", "deg");
    fmt.add_param("xy_out", "rad");
    assert_eq!(
        print(fmt),
        "+proj=pipeline \
         +step +proj=push +v_1 +v_2 \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=vgridshift +grids=foo"
    );
}

/// TEST(io, projstringformatter_krovak_to_krovak_east_north)
#[test]
fn io_projstringformatter_krovak_to_krovak_east_north() {
    let mut fmt = PipelineFormatter::default();
    fmt.ingest_proj_string(
        "+proj=pipeline \
         +step +inv +proj=krovak +axis=swu +lat_0=49.5 \
         +lon_0=24.8333333333333 \
         +alpha=30.2881397527778 +k=0.9999 +x_0=0 +y_0=0 +ellps=bessel \
         +step +proj=krovak +lat_0=49.5 +lon_0=24.8333333333333 \
         +alpha=30.2881397527778 +k=0.9999 +x_0=0 +y_0=0 +ellps=bessel",
    );
    assert_eq!(print(fmt), "+proj=axisswap +order=-2,-1");

    let mut fmt = PipelineFormatter::default();
    fmt.ingest_proj_string(
        "+proj=pipeline \
         +step +inv +proj=krovak +axis=swu +lat_0=49.5 \
         +lon_0=24.8333333333333 \
         +alpha=30.2881397527778 +k=0.9999 +x_0=0 +y_0=0 +ellps=bessel \
         +step +proj=krovak +lat_0=49.5 +lon_0=24.8333333333333 \
         +alpha=30.2881397527778 +k=0.9999 +x_0=0 +y_0=0",
    );
    assert_ne!(print(fmt), "+proj=axisswap +order=-2,-1");

    let mut fmt = PipelineFormatter::default();
    fmt.ingest_proj_string(
        "+proj=pipeline \
         +step +inv +proj=krovak +axis=swu +lat_0=49.5 \
         +lon_0=24.8333333333333 \
         +alpha=30.2881397527778 +k=0.9999 +x_0=0 +y_0=0 +ellps=bessel \
         +step +proj=krovak +lat_0=FOO +lon_0=24.8333333333333 \
         +alpha=30.2881397527778 +k=0.9999 +x_0=0 +y_0=0 +ellps=bessel",
    );
    assert_ne!(print(fmt), "+proj=axisswap +order=-2,-1");
}

/// TEST(io, projstringformatter_krovak_east_north_to_krovak)
#[test]
fn io_projstringformatter_krovak_east_north_to_krovak() {
    let mut fmt = PipelineFormatter::default();
    fmt.ingest_proj_string(
        "+proj=pipeline \
         +step +inv +proj=krovak +lat_0=49.5 +lon_0=24.8333333333333 \
         +alpha=30.2881397527778 +k=0.9999 +x_0=0 +y_0=0 +ellps=bessel \
         +step +proj=krovak +axis=swu +lat_0=49.5 +lon_0=24.8333333333333 \
         +alpha=30.2881397527778 +k=0.9999 +x_0=0 +y_0=0 +ellps=bessel",
    );
    assert_eq!(print(fmt), "+proj=axisswap +order=-2,-1");

    let mut fmt = PipelineFormatter::default();
    fmt.ingest_proj_string(
        "+proj=pipeline \
         +step +inv +proj=krovak +lat_0=49.5 +lon_0=24.8333333333333 \
         +alpha=30.2881397527778 +k=0.9999 +x_0=0 +y_0=0 +ellps=bessel \
         +step +proj=krovak +axis=swu +lat_0=FOO +lon_0=24.8333333333333 \
         +alpha=30.2881397527778 +k=0.9999 +x_0=0 +y_0=0",
    );
    assert_ne!(print(fmt), "+proj=axisswap +order=-2,-1");

    let mut fmt = PipelineFormatter::default();
    fmt.ingest_proj_string(
        "+proj=pipeline \
         +step +inv +proj=krovak +lat_0=49.5 +lon_0=24.8333333333333 \
         +alpha=30.2881397527778 +k=0.9999 +x_0=0 +y_0=0 +ellps=bessel \
         +step +proj=krovak +axis=swu +lat_0=FOO +lon_0=24.8333333333333 \
         +alpha=30.2881397527778 +k=0.9999 +x_0=0 +y_0=0 +ellps=bessel",
    );
    assert_ne!(print(fmt), "+proj=axisswap +order=-2,-1");
}
