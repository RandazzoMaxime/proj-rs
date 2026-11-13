//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, the operation factory tests left open by the
//! earlier lots: a compound CRS with a bound VERTCON transformation, a PROJ-based pair of
//! compound CRS, and a bound CRS over a derived geographic CRS. The expected strings are copied
//! from the C++ file, or measured with `projinfo` of PROJ 9.9.0 where the C++ test is compiled out.

use proj_rs::iso19111::{
    create_from_user_input, AuthorityFactory, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, Crs, DatabaseContext, GridAvailabilityUse, ProjLines,
    SpatialCriterion, TmercExport, WktObject, WktParser,
};

/// `compound1` of the C++ test: a transverse Mercator (`lat_0=1`, `lon_0=2`, `k=3`, `x_0=4`,
/// `y_0=5`) on WGS 84, with the bound vertical CRS `vc1` to `vc2` through VERTCON `bla.gtx`.
/// The VERTCON method and its offset file carry their EPSG codes, as `createVERTCON` sets them.
const COMPOUND_1: &str = r#"COMPOUNDCRS["c1",PROJCRS["proj",BASEGEOGCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1]]]],CONVERSION["tm",METHOD["Transverse Mercator"],PARAMETER["Latitude of natural origin",1,ANGLEUNIT["degree",0.0174532925199433]],PARAMETER["Longitude of natural origin",2,ANGLEUNIT["degree",0.0174532925199433]],PARAMETER["Scale factor at natural origin",3,SCALEUNIT["unity",1]],PARAMETER["False easting",4,LENGTHUNIT["metre",1]],PARAMETER["False northing",5,LENGTHUNIT["metre",1]]],CS[Cartesian,2],AXIS["easting (E)",east],AXIS["northing (N)",north],LENGTHUNIT["metre",1]],BOUNDCRS[SOURCECRS[VERTCRS["vc1",VDATUM["v1"],CS[vertical,1],AXIS["gravity-related height (H)",up],LENGTHUNIT["metre",1]]],TARGETCRS[VERTCRS["vc2",VDATUM["v2"],CS[vertical,1],AXIS["gravity-related height (H)",up],LENGTHUNIT["metre",1]]],ABRIDGEDTRANSFORMATION["vt",METHOD["VERTCON",ID["EPSG",9658]],PARAMETERFILE["Vertical offset file","bla.gtx",ID["EPSG",8732]]]]]"#;

/// `compound2` of the C++ test: `createUTM32_WGS84()` and the vertical CRS `vc2`.
const COMPOUND_2: &str = r#"COMPOUNDCRS["c2",PROJCRS["WGS 84 / UTM zone 32N",BASEGEOGCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1]]]],CONVERSION["UTM zone 32N",METHOD["Transverse Mercator",ID["EPSG",9807]],PARAMETER["Latitude of natural origin",0,ANGLEUNIT["degree",0.0174532925199433]],PARAMETER["Longitude of natural origin",9,ANGLEUNIT["degree",0.0174532925199433]],PARAMETER["Scale factor at natural origin",0.9996,SCALEUNIT["unity",1]],PARAMETER["False easting",500000,LENGTHUNIT["metre",1]],PARAMETER["False northing",0,LENGTHUNIT["metre",1]]],CS[Cartesian,2],AXIS["easting (E)",east],AXIS["northing (N)",north],LENGTHUNIT["metre",1]],VERTCRS["vc2",VDATUM["v2"],CS[vertical,1],AXIS["gravity-related height (H)",up],LENGTHUNIT["metre",1]]]"#;

/// The PROJJSON of the C++ test `boundCRS_to_derived_geog_with_transformation_with_source_crs_being_base_crs_of_source_crs`.
const BOUND_DERIVED_JSON: &str = r#"{
  "$schema": "foo",
  "type": "BoundCRS",
  "source_crs": {
    "type": "DerivedGeographicCRS",
    "name": "CH1903+ with height offset",
    "base_crs": {
      "type": "GeographicCRS",
      "name": "CH1903+",
      "datum": {
        "type": "GeodeticReferenceFrame",
        "name": "CH1903+",
        "ellipsoid": {
          "name": "Bessel 1841",
          "semi_major_axis": 6377397.155,
          "inverse_flattening": 299.1528128
        }
      },
      "coordinate_system": {
        "subtype": "ellipsoidal",
        "axis": [
          {
            "name": "Latitude",
            "abbreviation": "lat",
            "direction": "north",
            "unit": "degree"
          },
          {
            "name": "Longitude",
            "abbreviation": "lon",
            "direction": "east",
            "unit": "degree"
          },
          {
            "name": "Ellipsoidal height",
            "abbreviation": "h",
            "direction": "up",
            "unit": "metre"
          }
        ]
      }
    },
    "conversion": {
      "name": "Ellipsoidal to gravity related height",
      "method": {
        "name": "Geographic3D offsets",
        "id": {
          "authority": "EPSG",
          "code": 9660
        }
      },
      "parameters": [
        {
          "name": "Latitude offset",
          "value": 0,
          "unit": "degree",
          "id": {
            "authority": "EPSG",
            "code": 8601
          }
        },
        {
          "name": "Longitude offset",
          "value": 0,
          "unit": "degree",
          "id": {
            "authority": "EPSG",
            "code": 8602
          }
        },
        {
          "name": "Vertical Offset",
          "value": 10,
          "unit": "metre",
          "id": {
            "authority": "EPSG",
            "code": 8603
          }
        }
      ]
    },
    "coordinate_system": {
      "subtype": "ellipsoidal",
      "axis": [
        {
          "name": "Geodetic latitude",
          "abbreviation": "Lat",
          "direction": "north",
          "unit": "degree"
        },
        {
          "name": "Geodetic longitude",
          "abbreviation": "Lon",
          "direction": "east",
          "unit": "degree"
        },
        {
          "name": "Ellipsoidal height",
          "abbreviation": "h",
          "direction": "up",
          "unit": "metre"
        }
      ]
    }
  },
  "target_crs": {
    "type": "GeographicCRS",
    "name": "WGS 84",
    "datum_ensemble": {
      "name": "World Geodetic System 1984 ensemble",
      "members": [
        {
          "name": "World Geodetic System 1984 (Transit)",
          "id": {
            "authority": "EPSG",
            "code": 1166
          }
        },
        {
          "name": "World Geodetic System 1984 (G730)",
          "id": {
            "authority": "EPSG",
            "code": 1152
          }
        },
        {
          "name": "World Geodetic System 1984 (G873)",
          "id": {
            "authority": "EPSG",
            "code": 1153
          }
        },
        {
          "name": "World Geodetic System 1984 (G1150)",
          "id": {
            "authority": "EPSG",
            "code": 1154
          }
        },
        {
          "name": "World Geodetic System 1984 (G1674)",
          "id": {
            "authority": "EPSG",
            "code": 1155
          }
        },
        {
          "name": "World Geodetic System 1984 (G1762)",
          "id": {
            "authority": "EPSG",
            "code": 1156
          }
        },
        {
          "name": "World Geodetic System 1984 (G2139)",
          "id": {
            "authority": "EPSG",
            "code": 1309
          }
        }
      ],
      "ellipsoid": {
        "name": "WGS 84",
        "semi_major_axis": 6378137,
        "inverse_flattening": 298.257223563
      },
      "accuracy": "2.0"
    },
    "coordinate_system": {
      "subtype": "ellipsoidal",
      "axis": [
        {
          "name": "Geodetic latitude",
          "abbreviation": "Lat",
          "direction": "north",
          "unit": "degree"
        },
        {
          "name": "Geodetic longitude",
          "abbreviation": "Lon",
          "direction": "east",
          "unit": "degree"
        },
        {
          "name": "Ellipsoidal height",
          "abbreviation": "h",
          "direction": "up",
          "unit": "metre"
        }
      ]
    },
    "id": {
      "authority": "EPSG",
      "code": 4979
    }
  },
  "transformation": {
    "name": "CH1903+ to WGS 84 (1)",
    "source_crs": {
      "type": "GeographicCRS",
      "name": "CH1903+",
      "datum": {
        "type": "GeodeticReferenceFrame",
        "name": "CH1903+",
        "ellipsoid": {
          "name": "Bessel 1841",
          "semi_major_axis": 6377397.155,
          "inverse_flattening": 299.1528128
        }
      },
      "coordinate_system": {
        "subtype": "ellipsoidal",
        "axis": [
          {
            "name": "Latitude",
            "abbreviation": "lat",
            "direction": "north",
            "unit": "degree"
          },
          {
            "name": "Longitude",
            "abbreviation": "lon",
            "direction": "east",
            "unit": "degree"
          },
          {
            "name": "Ellipsoidal height",
            "abbreviation": "h",
            "direction": "up",
            "unit": "metre"
          }
        ]
      }
    },
    "method": {
      "name": "Geocentric translations (geog2D domain)",
      "id": {
        "authority": "EPSG",
        "code": 9603
      }
    },
    "parameters": [
      {
        "name": "X-axis translation",
        "value": 674.374,
        "unit": "metre",
        "id": {
          "authority": "EPSG",
          "code": 8605
        }
      },
      {
        "name": "Y-axis translation",
        "value": 15.056,
        "unit": "metre",
        "id": {
          "authority": "EPSG",
          "code": 8606
        }
      },
      {
        "name": "Z-axis translation",
        "value": 405.346,
        "unit": "metre",
        "id": {
          "authority": "EPSG",
          "code": 8607
        }
      }
    ],
    "id": {
      "authority": "EPSG",
      "code": 1676
    }
  }
}"#;

fn crs(text: &str) -> Crs {
    match WktParser::new().create_from_wkt(text).expect("WKT") {
        WktObject::Crs(crs) => crs,
        other => panic!("not a CRS: {other:?}"),
    }
}

fn epsg(db: &DatabaseContext, code: &str) -> Crs {
    AuthorityFactory::create(db, "EPSG")
        .create_coordinate_reference_system(code)
        .expect(code)
}

/// `CoordinateOperationContext::create` with the grids taken as available and
/// `SpatialCriterion::PARTIAL_INTERSECTION`, the setting of the C++ tests that list operations.
fn partial_ignore_grids(db: &DatabaseContext, authority: &str) -> CoordinateOperationContext {
    let mut ctxt = CoordinateOperationContext::create(db, authority);
    ctxt.set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    ctxt
}

/// `exportToPROJString(PROJStringFormatter::create())`, with the database for the grid names.
fn proj(db: &DatabaseContext, operation: &CoordinateOperation) -> String {
    operation
        .export_to_proj_string_with_database(db, ProjLines::Single)
        .expect("PROJ string")
}

/// `TEST(operation, compoundCRS_to_compoundCRS_with_vertical_transform)`.
#[test]
fn operation_compound_to_compound_with_vertical_transform() {
    let db = DatabaseContext::create().expect("proj.db");
    let compound1 = crs(COMPOUND_1);
    let compound2 = crs(COMPOUND_2);
    let factory = CoordinateOperationFactory::create();
    let op = factory
        .create_operation(&compound1, &compound2)
        .expect("operation")
        .expect("an operation");
    let export = |operation: &CoordinateOperation, tmerc| {
        operation
            .export_to_proj_string_with_tmerc(&db, ProjLines::Single, tmerc)
            .expect("PROJ string")
    };
    assert_eq!(
        export(&op, TmercExport::Exact),
        "+proj=pipeline +step +inv +proj=tmerc +lat_0=1 +lon_0=2 +k=3 \
         +x_0=4 +y_0=5 +ellps=WGS84 +step \
         +proj=vgridshift +grids=bla.gtx +multiplier=0.001 +step \
         +proj=utm +zone=32 +ellps=WGS84"
    );
    assert_eq!(
        export(&op, TmercExport::Approximate),
        "+proj=pipeline +step +inv +proj=tmerc +approx +lat_0=1 +lon_0=2 \
         +k=3 +x_0=4 +y_0=5 +ellps=WGS84 +step \
         +proj=vgridshift +grids=bla.gtx +multiplier=0.001 +step \
         +proj=utm +approx +zone=32 +ellps=WGS84"
    );
    assert_eq!(
        export(&op.inverse(), TmercExport::Approximate),
        "+proj=pipeline +step +inv +proj=utm +approx +zone=32 +ellps=WGS84 \
         +step +inv +proj=vgridshift +grids=bla.gtx \
         +multiplier=0.001 +step +proj=tmerc +approx +lat_0=1 +lon_0=2 \
         +k=3 +x_0=4 +y_0=5 +ellps=WGS84"
    );

    let op_inverse = factory
        .create_operation(&compound2, &compound1)
        .expect("operation")
        .expect("an operation");
    assert_eq!(
        export(&op_inverse.inverse(), TmercExport::Exact),
        export(&op, TmercExport::Exact)
    );
}

/// `TEST(operation, boundCRS_to_derived_geog_with_transformation_with_source_crs_being_base_crs_of_source_crs)`.
#[test]
fn operation_bound_to_derived_geog_with_transformation_with_source_crs_being_base_crs() {
    let db = DatabaseContext::create().expect("proj.db");
    let bound = create_from_user_input(BOUND_DERIVED_JSON, &db).expect("PROJJSON");
    let target = epsg(&db, "4979");
    let ctxt = partial_ignore_grids(&db, "EPSG");
    let list = CoordinateOperationFactory::create()
        .create_operations(&bound, &target, &ctxt)
        .expect("operations");
    assert_eq!(list.len(), 1);
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m \
         +step +inv +proj=geogoffset +dlat=0 +dlon=0 +dh=10 \
         +step +proj=cart +ellps=bessel \
         +step +proj=helmert +x=674.374 +y=15.056 +z=405.346 \
         +step +inv +proj=cart +ellps=WGS84 \
         +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m \
         +step +proj=axisswap +order=2,1"
    );
}

/// `TEST(operation, compoundCRS_to_compoundCRS_concatenated_operation_with_two_vert_transformation)`.
///
/// PROJ 9.9.0 compiles this test out (`#if no_longer_work_since_epsg_13_001`, pull 4803). The
/// expected values are what PROJ 9.9.0 gives for the same pair, measured with
/// `projinfo -s EPSG:8360 -t EPSG:7423 --grid-check none --spatial-test intersects`: four
/// candidates, the Czechia vertical offset first, then the two Slovak grid candidates. The grids
/// are not installed, so they are taken as available.
#[test]
fn operation_compound_to_compound_concatenated_two_vert_transformation() {
    let db = DatabaseContext::create().expect("proj.db");
    // ETRS89 + Baltic 1957 height (now deprecated)
    let source = epsg(&db, "8360");
    // ETRS89 + EVRF2007 height
    let target = epsg(&db, "7423");
    let ctxt = partial_ignore_grids(&db, "");
    let list = CoordinateOperationFactory::create()
        .create_operations(&source, &target, &ctxt)
        .expect("operations");
    assert_eq!(list.len(), 4);
    // For Czechia
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=vertoffset +lat_0=49.9166666666667 \
         +lon_0=15.25 +dh=0.13 +slope_lat=0.026 +slope_lon=0 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
    assert_eq!(list[0].name(), "Baltic 1957 height to EVRF2007 height (1)");

    // For Slovakia
    assert_eq!(
        proj(&db, &list[1]),
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=vgridshift +grids=sk_gku_Slovakia_ETRS89h_to_Baltic1957.tif +multiplier=1 \
         +step +inv +proj=vgridshift +grids=sk_gku_Slovakia_ETRS89h_to_EVRF2007.tif +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}

/// The names of the second candidate of `operation_compound_to_compound_concatenated_two_vert_transformation`,
/// as PROJ 9.9.0 names them (`projinfo -s EPSG:8360 -t EPSG:7423`, operation 2, and its reverse,
/// operation 2 of `projinfo -s EPSG:7423 -t EPSG:8360`). The Rust names are the same strings.
#[test]
fn operation_compound_to_compound_concatenated_two_vert_transformation_names() {
    let db = DatabaseContext::create().expect("proj.db");
    let source = epsg(&db, "8360");
    let target = epsg(&db, "7423");
    let ctxt = partial_ignore_grids(&db, "");
    let list = CoordinateOperationFactory::create()
        .create_operations(&source, &target, &ctxt)
        .expect("operations");
    assert_eq!(
        list[1].name(),
        "Inverse of 'ETRS89-SVK [SKTRF09] to Baltic 1957 height (from ETRS89-SVK [SKTRF09] \
         to ETRS89-SVK [SKTRF09] + Baltic 1957 height (1))' + 'ETRS89-SVK [SKTRF09] to EVRF2007 \
         height (from ETRS89-SVK [SKTRF09] to ETRS89-SVK [SKTRF09] + EVRF2007 height (1))'"
    );
    // The inverse is the reverse candidate of PROJ 9.9.0 (`projinfo -s EPSG:7423 -t EPSG:8360`,
    // operation 2), which has the same pipeline in the other direction.
    assert_eq!(
        list[1].inverse().name(),
        "Inverse of 'ETRS89-SVK [SKTRF09] to EVRF2007 height (from ETRS89-SVK [SKTRF09] \
         to ETRS89-SVK [SKTRF09] + EVRF2007 height (1))' + 'ETRS89-SVK [SKTRF09] to Baltic 1957 \
         height (from ETRS89-SVK [SKTRF09] to ETRS89-SVK [SKTRF09] + Baltic 1957 height (1))'"
    );
}
