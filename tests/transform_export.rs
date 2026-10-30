//! PROJ strings of transformations, checked against PROJ 9.9.0.
//!
//! The expected text is `projinfo -k operation EPSG:<code> -o PROJ -q` from PROJ 9.9.0.
//! Each test loads one EPSG operation, exports it, and compares the text.

use proj_rs::{
    AlternativeGridNames, AuthorityFactory, CoordinateOperation, DatabaseContext, ProjLines,
};

fn export(code: &str) -> Result<String, String> {
    let db = DatabaseContext::create().map_err(|error| error.to_string())?;
    let factory = AuthorityFactory::create(&db, "EPSG");
    let operation = factory
        .create_coordinate_operation(code, AlternativeGridNames::Official)
        .map_err(|error| error.to_string())?;
    match operation {
        CoordinateOperation::Transformation(transformation) => transformation
            .export_to_proj_string(ProjLines::Multi)
            .map_err(|error| error.to_string()),
        CoordinateOperation::Concatenated(concatenated) => concatenated
            .export_to_proj_string(ProjLines::Multi)
            .map_err(|error| error.to_string()),
        CoordinateOperation::Conversion(_) => Err(format!("EPSG:{code} is a conversion")),
    }
}

/// As [`export`], with the database of the context, so grid names read as PROJ writes them.
fn export_with_database(code: &str) -> Result<String, String> {
    let db = DatabaseContext::create().map_err(|error| error.to_string())?;
    let factory = AuthorityFactory::create(&db, "EPSG");
    let operation = factory
        .create_coordinate_operation(code, AlternativeGridNames::Official)
        .map_err(|error| error.to_string())?;
    operation
        .export_to_proj_string_with_database(&db, ProjLines::Multi)
        .map_err(|error| error.to_string())
}

#[test]
fn geog2d_geocentric_translations_9603() {
    let text = export("1055").expect("export");
    assert_eq!(
        text,
        r#"+proj=pipeline
  +step +proj=axisswap +order=2,1
  +step +proj=unitconvert +xy_in=deg +xy_out=rad
  +step +proj=push +v_3
  +step +proj=cart +ellps=intl
  +step +proj=helmert +x=-145.7 +y=-249.1 +z=1.5
  +step +inv +proj=cart +ellps=WGS84
  +step +proj=pop +v_3
  +step +proj=unitconvert +xy_in=rad +xy_out=deg
  +step +proj=axisswap +order=2,1"#
    );
}

#[test]
fn position_vector_geog2d_9606_with_geog_source() {
    let text = export("1074").expect("export");
    assert_eq!(
        text,
        r#"+proj=pipeline
  +step +proj=axisswap +order=2,1
  +step +proj=unitconvert +xy_in=deg +xy_out=rad
  +step +inv +proj=longlat +a=6378300.789 +b=6356566.435
  +step +proj=push +v_3
  +step +proj=cart +a=6378300.789 +b=6356566.435
  +step +proj=helmert +x=-275.7224 +y=94.7824 +z=340.8944 +rx=-8.001 +ry=-4.42
        +rz=-11.821 +s=1 +convention=position_vector
  +step +inv +proj=cart +ellps=WGS84
  +step +proj=pop +v_3
  +step +proj=unitconvert +xy_in=rad +xy_out=deg
  +step +proj=axisswap +order=2,1"#
    );
}

#[test]
fn coordinate_frame_geog2d_9607_push_v3() {
    let text = export("1024").expect("export");
    assert_eq!(
        text,
        r#"+proj=pipeline
  +step +proj=axisswap +order=2,1
  +step +proj=unitconvert +xy_in=deg +xy_out=rad
  +step +proj=push +v_3
  +step +proj=cart +ellps=bessel
  +step +proj=helmert +x=601.705 +y=84.263 +z=485.227 +rx=-4.7354 +ry=-1.3145
        +rz=-5.393 +s=-2.3887 +convention=coordinate_frame
  +step +inv +proj=cart +ellps=GRS80
  +step +proj=pop +v_3
  +step +proj=unitconvert +xy_in=rad +xy_out=deg
  +step +proj=axisswap +order=2,1"#
    );
}

#[test]
fn coordinate_frame_geog2d_9607_second() {
    let text = export("1056").expect("export");
    assert_eq!(
        text,
        r#"+proj=pipeline
  +step +proj=axisswap +order=2,1
  +step +proj=unitconvert +xy_in=deg +xy_out=rad
  +step +proj=push +v_3
  +step +proj=cart +ellps=intl
  +step +proj=helmert +x=-85.645 +y=-273.077 +z=-79.708 +rx=-2.289 +ry=1.421
        +rz=-2.532 +s=3.194 +convention=coordinate_frame
  +step +inv +proj=cart +ellps=WGS84
  +step +proj=pop +v_3
  +step +proj=unitconvert +xy_in=rad +xy_out=deg
  +step +proj=axisswap +order=2,1"#
    );
}

#[test]
fn geocentric_translations_1031() {
    let text = export("7817").expect("export");
    assert_eq!(text, r#"+proj=helmert +x=24.322 +y=-121.372 +z=-75.847"#);
}

#[test]
fn position_vector_geocentric_1033() {
    let text = export("6936").expect("export");
    assert_eq!(
        text,
        r#"+proj=helmert +x=-0.214 +y=0.119 +z=0.156 +rx=-0.01182 +ry=0.00811 +rz=-0.01677
      +s=-0.0059 +convention=position_vector"#
    );
}

#[test]
fn coordinate_frame_geocentric_1032_is_noop() {
    let text = export("7666").expect("export");
    assert_eq!(text, r#"+proj=noop"#);
}

#[test]
fn time_dependent_position_vector_1053() {
    let text = export("5900").expect("export");
    assert_eq!(
        text,
        r#"+proj=helmert +x=0.056 +y=0.048 +z=-0.037 +rx=0 +ry=0 +rz=0 +s=0 +dx=0 +dy=0
      +dz=0 +drx=5.4e-05 +dry=0.000518 +drz=-0.000781 +ds=0 +t_epoch=1989
      +convention=position_vector"#
    );
}

#[test]
fn time_dependent_coordinate_frame_1056() {
    let text = export("6276").expect("export");
    assert_eq!(
        text,
        r#"+proj=helmert +x=-0.08468 +y=-0.01942 +z=0.03201 +rx=-0.0004254 +ry=0.0022578
      +rz=0.0024015 +s=0.00971 +dx=0.00142 +dy=0.00134 +dz=0.0009 +drx=0.0015461
      +dry=0.001182 +drz=0.0011551 +ds=0.000109 +t_epoch=1994
      +convention=coordinate_frame"#
    );
}

#[test]
fn full_matrix_geog2d_1133() {
    let text = export("10676").expect("export");
    assert_eq!(
        text,
        r#"+proj=pipeline
  +step +proj=axisswap +order=2,1
  +step +proj=unitconvert +xy_in=deg +xy_out=rad
  +step +proj=push +v_3
  +step +proj=cart +ellps=intl
  +step +proj=helmert +exact +x=1138.7432 +y=-2064.4761 +z=110.7016
        +rx=-214.615206 +ry=479.360036 +rz=-164.703951 +s=-402.32073
        +convention=coordinate_frame
  +step +inv +proj=cart +ellps=WGS84
  +step +proj=pop +v_3
  +step +proj=unitconvert +xy_in=rad +xy_out=deg
  +step +proj=axisswap +order=2,1"#
    );
}

#[test]
fn full_matrix_geog3d_1140() {
    let text = export("10646").expect("export");
    assert_eq!(
        text,
        r#"+proj=pipeline
  +step +proj=axisswap +order=2,1
  +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m
  +step +proj=cart +ellps=intl
  +step +proj=helmert +exact +x=1138.7432 +y=-2064.4761 +z=110.7016
        +rx=-214.615206 +ry=479.360036 +rz=-164.703951 +s=-402.32073
        +convention=coordinate_frame
  +step +inv +proj=cart +ellps=GRS80
  +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m
  +step +proj=axisswap +order=2,1"#
    );
}

#[test]
fn coordinate_frame_geog3d_to_compound_1149() {
    let text = export("10905").expect("export");
    assert_eq!(
        text,
        r#"+proj=pipeline
  +step +proj=axisswap +order=2,1
  +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m
  +step +proj=cart +ellps=GRS80
  +step +proj=helmert +x=-646.6552 +y=-165.0859 +z=-437.6858 +rx=4.77773
        +ry=-0.39139 +rz=-1.07485 +s=2.0025 +convention=coordinate_frame
  +step +inv +proj=cart +ellps=bessel
  +step +proj=unitconvert +xy_in=rad +xy_out=deg
  +step +proj=axisswap +order=2,1"#
    );
}

#[test]
fn similarity_9621() {
    let text = export("3929").expect("export");
    assert_eq!(
        text,
        r#"+proj=affine +xoff=-378.752 +s11=1.00001267735232 +s12=1.71860386844725e-05
      +yoff=493.395 +s21=-1.71860386844725e-05 +s22=1.00001267735232"#
    );
}

#[test]
fn affine_9624() {
    let text = export("10087").expect("export");
    assert_eq!(
        text,
        r#"+proj=affine +xoff=82357.457 +s11=0.304794369 +s12=1.5417425e-05 +yoff=28091.324
      +s21=-1.5417425e-05 +s22=0.304794369"#
    );
}

#[test]
fn cartesian_grid_offsets_9656() {
    let text = export("6724").expect("export");
    assert_eq!(text, r#"+proj=affine +xoff=550015 +yoff=8780001"#);
}

#[test]
fn concatenated_helmert_with_pm() {
    let text = export("3966").expect("export");
    assert_eq!(
        text,
        r#"+proj=pipeline
  +step +proj=axisswap +order=2,1
  +step +proj=unitconvert +xy_in=deg +xy_out=rad
  +step +inv +proj=longlat +ellps=bessel +pm=-17.6627833333333
  +step +proj=push +v_3
  +step +proj=cart +ellps=bessel
  +step +proj=helmert +x=682 +y=-203 +z=480
  +step +inv +proj=cart +ellps=WGS84
  +step +proj=pop +v_3
  +step +proj=unitconvert +xy_in=rad +xy_out=deg
  +step +proj=axisswap +order=2,1"#
    );
}

#[test]
fn molodensky_badekas_step_is_unimplemented() {
    let error = export("5192").expect_err("a Molodensky-Badekas step is not written");
    assert!(error.contains("Unimplemented"), "{error}");
}

// Grid and deformation methods: `projinfo -k operation EPSG:<code> -o PROJ -q` from PROJ 9.9.0.

#[test]
fn geoid_1048_5656() {
    let text = export_with_database("5656").expect("export");
    assert_eq!(
        text,
        r#"+proj=pipeline
  +step +proj=axisswap +order=2,1
  +step +proj=unitconvert +xy_in=deg +xy_out=rad
  +step +inv +proj=vgridshift +grids=au_ga_AUSGeoid09_V1.01.tif +multiplier=1
  +step +proj=unitconvert +xy_in=rad +xy_out=deg
  +step +proj=axisswap +order=2,1"#
    );
}

#[test]
fn geoid_1048_8451() {
    let text = export_with_database("8451").expect("export");
    assert_eq!(
        text,
        r#"+proj=pipeline
  +step +proj=axisswap +order=2,1
  +step +proj=unitconvert +xy_in=deg +xy_out=rad
  +step +inv +proj=vgridshift +grids=au_ga_AUSGeoid2020_20180201.tif
        +multiplier=1
  +step +proj=unitconvert +xy_in=rad +xy_out=deg
  +step +proj=axisswap +order=2,1"#
    );
}

#[test]
fn geoid_1103_9708() {
    let text = export_with_database("9708").expect("export");
    assert_eq!(
        text,
        r#"+proj=pipeline
  +step +proj=axisswap +order=2,1
  +step +proj=unitconvert +xy_in=deg +xy_out=rad
  +step +inv +proj=vgridshift +grids=us_nga_egm96_15.tif +multiplier=1
  +step +proj=unitconvert +xy_in=rad +xy_out=deg
  +step +proj=axisswap +order=2,1"#
    );
}

#[test]
fn geoid_1072_9916() {
    let text = export_with_database("9916").expect("export");
    assert_eq!(
        text,
        r#"+proj=pipeline
  +step +proj=axisswap +order=2,1
  +step +proj=unitconvert +xy_in=deg +xy_out=rad
  +step +inv +proj=vgridshift +grids=uk_os_OSGM15_Belfast.tif +multiplier=1
  +step +proj=unitconvert +xy_in=rad +xy_out=deg
  +step +proj=axisswap +order=2,1"#
    );
}

#[test]
fn vertical_1084_9312() {
    let text = export_with_database("9312").expect("export");
    assert_eq!(
        text,
        r#"+proj=vgridshift +grids=nz_linz_auckht1946-nzvd2016.tif +multiplier=1"#
    );
}

#[test]
fn vertical_1085_9553() {
    let text = export_with_database("9553").expect("export");
    assert_eq!(
        text,
        r#"+proj=vgridshift +grids=pt_2019z.asc +multiplier=1"#
    );
}

#[test]
fn vertical_1101_9662() {
    let text = export_with_database("9662").expect("export");
    assert_eq!(
        text,
        r#"+proj=vgridshift +grids=gugik-evrf2007.txt +multiplier=1"#
    );
}

#[test]
fn vertcon_9658_7969() {
    let text = export_with_database("7969").expect("export");
    assert_eq!(
        text,
        r#"+proj=vgridshift +grids=us_noaa_vertconw.tif +multiplier=1"#
    );
}

#[test]
fn vertical_1080_9275() {
    let text = export_with_database("9275").expect("export");
    assert_eq!(
        text,
        r#"+proj=vgridshift +grids=at_bev_GV_Hoehengrid_V1.tif +multiplier=1"#
    );
}

#[test]
fn nrcan_1126_10518() {
    let text = export_with_database("10518").expect("export");
    assert_eq!(
        text,
        r#"+proj=vgridshift +grids=ca_nrc_HT2_1997_CGG2013a.tif +multiplier=-1"#
    );
}

#[test]
fn defmodel_1079_9084() {
    let text = export_with_database("9084").expect("export");
    assert_eq!(
        text,
        r#"+proj=pipeline
  +step +proj=axisswap +order=2,1
  +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m
  +step +inv +proj=defmodel +model=nz_linz_nzgd2000-20000101.json
  +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m
  +step +proj=axisswap +order=2,1"#
    );
}

#[test]
fn hgridshift_9615_1295() {
    let text = export_with_database("1295").expect("export");
    assert_eq!(
        text,
        r#"+proj=pipeline
  +step +proj=axisswap +order=2,1
  +step +proj=unitconvert +xy_in=deg +xy_out=rad
  +step +proj=hgridshift +grids=RGNC1991_NEA74Noumea.gsb
  +step +proj=unitconvert +xy_in=rad +xy_out=deg
  +step +proj=axisswap +order=2,1"#
    );
}

#[test]
fn hgridshift_9614_1312() {
    let text = export_with_database("1312").expect("export");
    assert_eq!(
        text,
        r#"+proj=pipeline
  +step +proj=axisswap +order=2,1
  +step +proj=unitconvert +xy_in=deg +xy_out=rad
  +step +proj=hgridshift +grids=ca_nrc_ntv1_can.tif
  +step +proj=unitconvert +xy_in=rad +xy_out=deg
  +step +proj=axisswap +order=2,1"#
    );
}

#[test]
fn tinshift_1138_10703() {
    let text = export_with_database("10703").expect("export");
    assert_eq!(
        text,
        r#"+proj=pipeline
  +step +proj=axisswap +order=2,1
  +step +proj=tinshift +file=fi_nls_ykj_etrs35fin.json"#
    );
}

#[test]
fn tinshift_1145_10854() {
    let text = export_with_database("10854").expect("export");
    assert_eq!(
        text,
        r#"+proj=pipeline
  +step +proj=axisswap +order=2,1
  +step +proj=tinshift +file=no_kv_ETRS89NO_NGO48_TIN.json
  +step +proj=axisswap +order=2,1"#
    );
}
