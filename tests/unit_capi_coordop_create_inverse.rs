//! PROJ 9.9.0 `test/unit/test_c_api.cpp`: `TEST_F(CApi, proj_coordoperation_create_inverse)`.
//!
//! The inverse of a PROJ pipeline (Everest to WGS 84, with a position-vector Helmert) is written
//! back with `proj_as_proj_string` in multi-line form: `MULTILINE=YES`, `INDENTATION_WIDTH=4`,
//! `MAX_LINE_LENGTH=40`. The expected text is copied from the C++ test.

use proj_rs::{Proj, ProjStringLayout};

const PIPELINE: &str = "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
     +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=push \
     +v_3 +step +proj=cart +ellps=evrst30 +step +proj=helmert \
     +x=293 +y=836 +z=318 +rx=0.5 +ry=1.6 +rz=-2.8 +s=2.1 \
     +convention=position_vector +step +inv +proj=cart \
     +ellps=WGS84 +step +proj=pop +v_3 +step +proj=unitconvert \
     +xy_in=rad +xy_out=deg +step +proj=axisswap +order=2,1";

#[test]
fn capi_proj_coordoperation_create_inverse() {
    let proj = Proj::new(PIPELINE).expect("pipeline");
    let inverse = proj.coordoperation_create_inverse().expect("inverse");

    let layout = ProjStringLayout::Multi {
        indentation_width: 4,
        max_line_length: 40,
    };
    let text = inverse.as_proj_string(layout).expect("PROJ string");
    let expected = "+proj=pipeline\n\
         \x20   +step +proj=axisswap +order=2,1\n\
         \x20   +step +proj=unitconvert +xy_in=deg\n\
         \x20         +xy_out=rad\n\
         \x20   +step +proj=push +v_3\n\
         \x20   +step +proj=cart +ellps=WGS84\n\
         \x20   +step +inv +proj=helmert +x=293\n\
         \x20         +y=836 +z=318 +rx=0.5 +ry=1.6\n\
         \x20         +rz=-2.8 +s=2.1\n\
         \x20         +convention=position_vector\n\
         \x20   +step +inv +proj=cart +ellps=evrst30\n\
         \x20   +step +proj=pop +v_3\n\
         \x20   +step +proj=unitconvert +xy_in=rad\n\
         \x20         +xy_out=deg\n\
         \x20   +step +proj=axisswap +order=2,1";
    assert_eq!(text, expected);
}
