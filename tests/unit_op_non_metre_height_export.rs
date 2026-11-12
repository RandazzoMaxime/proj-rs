//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`: a compound CRS with a non-metre height given
//! to a PROJ string CRS, and the IGNF `init=` pair under the PROJ 4 rules. Expected values are
//! copied from that file; none is taken from this crate's output.

use proj_rs::iso19111::{
    create_crs_from_proj_string, create_from_user_input, create_from_user_input_with_init_rules,
    CoordinateOperation, CoordinateOperationContext, CoordinateOperationFactory, Crs,
    DatabaseContext, InitRules, ProjLines, SpatialCriterion,
};

/// `createOperations` with the partial-intersection criterion, as the C++ tests set it.
fn partial_operations(
    db: &DatabaseContext,
    authority: &str,
    source: &Crs,
    target: &Crs,
) -> Vec<CoordinateOperation> {
    let mut ctxt = CoordinateOperationContext::create(db, authority);
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    CoordinateOperationFactory::create()
        .create_operations(source, target, &ctxt)
        .expect("operations")
}

/// `exportToPROJString` of an operation.
fn proj(db: &DatabaseContext, operation: &CoordinateOperation) -> String {
    operation
        .export_to_proj_string_with_database(db, ProjLines::Single)
        .expect("PROJ string")
}

/// `TEST(operation, compoundCRS_to_proj_string_with_non_metre_height)`
///
/// `EPSG:6318+5703` (NAD83(2011) + NAVD88 height in US survey feet) to a PROJ string
/// geographic CRS with `+vunits=us-ft` and no identifier. The first operation must write the
/// vertical unit change once, in the last `unitconvert`.
#[test]
fn operation_compound_to_proj_string_with_non_metre_height() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = create_from_user_input("EPSG:6318+5703", &db).expect("EPSG:6318+5703");
    let dst = create_crs_from_proj_string("+proj=longlat +ellps=GRS80 +vunits=us-ft +type=crs")
        .expect("PROJ string CRS");

    let list = partial_operations(&db, "EPSG", &src, &dst);
    assert!(list.len() > 1, "{} operations", list.len());
    // What is important to check here is the vertical unit conversion
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=vgridshift +grids=us_noaa_g2018u0.tif +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=us-ft"
    );
}

/// `TEST(operation, geogCRS_to_geogCRS_init_IGNF_to_init_IGNF_context)`
///
/// `+init=IGNF:NTFG` to `+init=IGNF:RGF93G` under the PROJ 4 init rules
/// (`PROJStringParser().setUsePROJ4InitRules(true)`, here `create_from_user_input_with_init_rules`
/// with `InitRules::Proj4`), operations with an empty authority name.
#[test]
fn operation_geog_crs_to_geog_crs_init_ignf_to_init_ignf_context() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = create_from_user_input_with_init_rules("+init=IGNF:NTFG", &db, InitRules::Proj4)
        .expect("+init=IGNF:NTFG");
    let dst = create_from_user_input_with_init_rules("+init=IGNF:RGF93G", &db, InitRules::Proj4)
        .expect("+init=IGNF:RGF93G");

    let ctxt = CoordinateOperationContext::create(&db, "");
    let list = CoordinateOperationFactory::create()
        .create_operations(&src, &dst, &ctxt)
        .expect("operations");
    assert_eq!(list.len(), 2, "{} operations", list.len());

    assert_eq!(
        list[0].name(),
        "NOUVELLE TRIANGULATION DE LA FRANCE (NTF) vers RGF93 (ETRS89)"
    );
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=hgridshift +grids=fr_ign_ntf_r93.tif +step \
         +proj=unitconvert +xy_in=rad +xy_out=deg"
    );
}

/// `TEST(operation, createOperation_ossfuzz_18587)` (test_operationfactory.cpp)
///
/// An extremely weird PROJ string that must be accepted as a CRS (`ASSERT_TRUE(dst != nullptr)`),
/// and on which `createOperation` must return, with a value or an error. A stack overflow of
/// the recursion would abort the test process. PROJ 9.9.0 `projinfo` reads it as a Mercator CRS
/// with a `nadgrids` bound, the quoted value of the second `proj` being a parameter.
#[test]
fn operation_create_operation_ossfuzz_18587() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = create_from_user_input("EPSG:4326", &db).expect("EPSG:4326");

    // Extremely weird string ! We should likely reject it
    let dst = create_crs_from_proj_string(
        "type=crs proj=pipeline step proj=merc vunits=m nadgrids=@x \
         proj=\"\nproj=pipeline step\n\"",
    )
    .expect("the CRS of the fuzzer string");

    // Just check that we don't go into an infinite recursion
    let _ = CoordinateOperationFactory::create().create_operation(&src, &dst);
}

/// The compound source of `compoundCRS_to_PROJJSON_with_non_metre_height`: NAD83(2011) / Nebraska
/// (ftUS) + NAVD88 height (ftUS) with a `GEOIDMODEL` `us_noaa_g2012bu0.tif`. The JSON is the
/// C++ string literal of `test_operationfactory.cpp:10299-10521`.
const SRC_PROJJSON: &str = r##"{
  "$schema": "https://proj.org/schemas/v0.2/projjson.schema.json",
  "type": "CompoundCRS",
  "name": "Compound CRS NAD83(2011) / Nebraska (ftUS) + North American Vertical Datum 1988 + PROJ us_noaa_g2012bu0.tif",
  "components": [
    {
      "type": "ProjectedCRS",
      "name": "NAD83(2011) / Nebraska (ftUS)",
      "base_crs": {
        "name": "NAD83(2011)",
        "datum": {
          "type": "GeodeticReferenceFrame",
          "name": "NAD83 (National Spatial Reference System 2011)",
          "ellipsoid": {
            "name": "GRS 1980",
            "semi_major_axis": 6378137,
            "inverse_flattening": 298.257222101
          }
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
            }
          ]
        },
        "id": {
          "authority": "EPSG",
          "code": 6318
        }
      },
      "conversion": {
        "name": "SPCS83 Nebraska zone (US Survey feet)",
        "method": {
          "name": "Lambert Conic Conformal (2SP)",
          "id": {
            "authority": "EPSG",
            "code": 9802
          }
        },
        "parameters": [
          {
            "name": "Latitude of false origin",
            "value": 39.8333333333333,
            "unit": "degree",
            "id": {
              "authority": "EPSG",
              "code": 8821
            }
          },
          {
            "name": "Longitude of false origin",
            "value": -100,
            "unit": "degree",
            "id": {
              "authority": "EPSG",
              "code": 8822
            }
          },
          {
            "name": "Latitude of 1st standard parallel",
            "value": 43,
            "unit": "degree",
            "id": {
              "authority": "EPSG",
              "code": 8823
            }
          },
          {
            "name": "Latitude of 2nd standard parallel",
            "value": 40,
            "unit": "degree",
            "id": {
              "authority": "EPSG",
              "code": 8824
            }
          },
          {
            "name": "Easting at false origin",
            "value": 1640416.6667,
            "unit": {
              "type": "LinearUnit",
              "name": "US survey foot",
              "conversion_factor": 0.304800609601219
            },
            "id": {
              "authority": "EPSG",
              "code": 8826
            }
          },
          {
            "name": "Northing at false origin",
            "value": 0,
            "unit": {
              "type": "LinearUnit",
              "name": "US survey foot",
              "conversion_factor": 0.304800609601219
            },
            "id": {
              "authority": "EPSG",
              "code": 8827
            }
          }
        ],
        "id": {
          "authority": "EPSG",
          "code": 15396
        }
      },
      "coordinate_system": {
        "subtype": "Cartesian",
        "axis": [
          {
            "name": "Easting",
            "abbreviation": "X",
            "direction": "east",
            "unit": {
              "type": "LinearUnit",
              "name": "US survey foot",
              "conversion_factor": 0.304800609601219
            }
          },
          {
            "name": "Northing",
            "abbreviation": "Y",
            "direction": "north",
            "unit": {
              "type": "LinearUnit",
              "name": "US survey foot",
              "conversion_factor": 0.304800609601219
            }
          }
        ]
      }
    },
    {
      "type": "VerticalCRS",
      "name": "North American Vertical Datum 1988 + PROJ us_noaa_g2012bu0.tif",
      "datum": {
        "type": "VerticalReferenceFrame",
        "name": "North American Vertical Datum 1988",
        "id": {
          "authority": "EPSG",
          "code": 5103
        }
      },
      "coordinate_system": {
        "subtype": "vertical",
        "axis": [
          {
            "name": "Gravity-related height",
            "abbreviation": "H",
            "direction": "up",
            "unit": {
              "type": "LinearUnit",
              "name": "US survey foot",
              "conversion_factor": 0.304800609601219
            }
          }
        ]
      },
      "geoid_model": {
        "name": "PROJ us_noaa_g2012bu0.tif",
        "interpolation_crs": {
          "type": "GeographicCRS",
          "name": "NAD83(2011)",
          "datum": {
            "type": "GeodeticReferenceFrame",
            "name": "NAD83 (National Spatial Reference System 2011)",
            "ellipsoid": {
              "name": "GRS 1980",
              "semi_major_axis": 6378137,
              "inverse_flattening": 298.257222101
            }
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
            "code": 6319
          }
        }
      }
    }
  ]
}"##;

/// The target of `compoundCRS_to_PROJJSON_with_non_metre_height`: NAD83(2011) / UTM zone 14N with
/// an ellipsoidal height in US survey feet (`test_operationfactory.cpp:10534-10680`).
const DST_PROJJSON: &str = r##"{
  "$schema": "https://proj.org/schemas/v0.2/projjson.schema.json",
  "type": "ProjectedCRS",
  "name": "Projected CRS NAD83(2011) / UTM zone 14N with ellipsoidal NAD83(2011) height",
  "base_crs": {
    "name": "NAD83(2011)",
    "datum": {
      "type": "GeodeticReferenceFrame",
      "name": "NAD83 (National Spatial Reference System 2011)",
      "ellipsoid": {
        "name": "GRS 1980",
        "semi_major_axis": 6378137,
        "inverse_flattening": 298.257222101
      },
      "id": {
        "authority": "EPSG",
        "code": 1116
      }
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
          "unit": {
            "type": "LinearUnit",
            "name": "US survey foot",
            "conversion_factor": 0.304800609601219
          }
        }
      ]
    }
  },
  "conversion": {
    "name": "UTM zone 14N",
    "method": {
      "name": "Transverse Mercator",
      "id": {
        "authority": "EPSG",
        "code": 9807
      }
    },
    "parameters": [
      {
        "name": "Latitude of natural origin",
        "value": 0,
        "unit": "degree",
        "id": {
          "authority": "EPSG",
          "code": 8801
        }
      },
      {
        "name": "Longitude of natural origin",
        "value": -99,
        "unit": "degree",
        "id": {
          "authority": "EPSG",
          "code": 8802
        }
      },
      {
        "name": "Scale factor at natural origin",
        "value": 0.9996,
        "unit": "unity",
        "id": {
          "authority": "EPSG",
          "code": 8805
        }
      },
      {
        "name": "False easting",
        "value": 500000,
        "unit": "metre",
        "id": {
          "authority": "EPSG",
          "code": 8806
        }
      },
      {
        "name": "False northing",
        "value": 0,
        "unit": "metre",
        "id": {
          "authority": "EPSG",
          "code": 8807
        }
      }
    ],
    "id": {
      "authority": "EPSG",
      "code": 16014
    }
  },
  "coordinate_system": {
    "subtype": "Cartesian",
    "axis": [
      {
        "name": "Easting",
        "abbreviation": "E",
        "direction": "east",
        "unit": {
          "type": "LinearUnit",
          "name": "US survey foot",
          "conversion_factor": 0.304800609601219
        }
      },
      {
        "name": "Northing",
        "abbreviation": "N",
        "direction": "north",
        "unit": {
          "type": "LinearUnit",
          "name": "US survey foot",
          "conversion_factor": 0.304800609601219
        }
      },
      {
        "name": "Ellipsoidal height",
        "abbreviation": "h",
        "direction": "up",
        "unit": {
          "type": "LinearUnit",
          "name": "US survey foot",
          "conversion_factor": 0.304800609601219
        }
      }
    ]
  }
}"##;

/// `TEST(operation, compoundCRS_to_PROJJSON_with_non_metre_height)`
///
/// The compound of `SRC_PROJJSON` (a `GEOIDMODEL` vertical CRS in US survey feet) to the projected
/// 3D CRS `DST_PROJJSON`. The vertical unit conversion must be in the expected `unitconvert` steps;
/// the first operation is checked.
#[test]
fn operation_compound_to_projjson_with_non_metre_height() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = create_from_user_input(SRC_PROJJSON, &db).expect("compound PROJJSON");
    let dst = create_from_user_input(DST_PROJJSON, &db).expect("projected PROJJSON");

    let list = partial_operations(&db, "EPSG", &src, &dst);
    assert!(list.len() > 1, "{} operations", list.len());
    // What is important to check here is the vertical unit conversion
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline \
         +step +proj=unitconvert +xy_in=us-ft +xy_out=m \
         +step +inv +proj=lcc +lat_0=39.8333333333333 +lon_0=-100 +lat_1=43 \
         +lat_2=40 +x_0=500000.00001016 +y_0=0 +ellps=GRS80 \
         +step +proj=unitconvert +z_in=us-ft +z_out=m \
         +step +proj=vgridshift +grids=us_noaa_g2012bu0.tif +multiplier=1 \
         +step +proj=utm +zone=14 +ellps=GRS80 \
         +step +proj=unitconvert +xy_in=m +z_in=m +xy_out=us-ft +z_out=us-ft"
    );
}
