//! PROJ 9.9.0 `test/unit/test_crs.cpp` and `test/unit/test_io.cpp` cases of the
//! crs-io-core lot.
//!
//! Each function is one `TEST()` name, in snake case, with the C++ name in its
//! comment. Expected text is copied from those files without change.

use proj_rs::iso19111::{create_object_from_json, JsonFormat, JsonObject, ProjLines};
use proj_rs::{
    AlternativeGridNames, AuthorityFactory, BoundCRS, CoordinateOperationFactory, Crs,
    DatabaseContext, GeographicCRS,
};

/// `crs.boundCRS_nadcon_to_PROJ_string_usePROJAlternativeGridNames_false`
#[test]
fn crs_bound_crs_nadcon_to_proj_string_use_proj_alternative_grid_names_false() {
    let db = DatabaseContext::create().expect("proj.db");
    let operation = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_operation("15851", AlternativeGridNames::Official)
        .expect("EPSG:15851");
    let bound = BoundCRS::create(
        Crs::Geographic(GeographicCRS::epsg_4267()),
        Crs::Geographic(GeographicCRS::epsg_4326()),
        operation,
    );
    assert_eq!(
        Crs::Bound(bound)
            .export_to_proj_string_with_database(&db)
            .expect("proj"),
        "+proj=longlat +ellps=clrk66 +nadgrids=us_noaa_conus.tif +no_defs +type=crs"
    );
}

/// `json_import.bound_crs_with_source_crs_in_transformation`
#[test]
fn json_import_bound_crs_with_source_crs_in_transformation() {
    let json = r#"{
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
    let db = DatabaseContext::create().expect("proj.db");
    let Ok(JsonObject::Crs(bound @ Crs::Bound(_))) = create_object_from_json(json, None) else {
        panic!("expected a BoundCRS");
    };
    let format = JsonFormat::default().with_schema("foo");
    assert_eq!(
        bound.export_to_json_with(&db, &format).expect("PROJJSON"),
        json
    );
}

/// `crs.normalizeForVisualization_bound`
#[test]
fn crs_normalize_for_visualization_bound() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    // NTF (Paris)
    let crs_4807 = factory
        .create_coordinate_reference_system("4807")
        .expect("EPSG:4807");
    let bound = crs_4807.create_bound_crs_to_wgs84_if_possible(&db);
    // C++ `EXPECT_NE` on the two pointers: the binding produced a new CRS object.
    assert!(matches!(bound, Crs::Bound(_)));

    let normalized_proj_string = "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
        +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=push +v_3 +step \
        +proj=cart +ellps=WGS84 +step +proj=helmert +x=168 +y=60 +z=-320 \
        +step +inv +proj=cart +ellps=clrk80ign +step +proj=pop +v_3 +step \
        +proj=longlat +ellps=clrk80ign +pm=paris +step +proj=unitconvert \
        +xy_in=rad +xy_out=grad";
    let orig_proj_string = format!("{normalized_proj_string} +step +proj=axisswap +order=2,1");
    let wgs84 = Crs::Geographic(GeographicCRS::epsg_4326());
    let op = CoordinateOperationFactory::create()
        .create_operation(&wgs84, &bound)
        .expect("operation")
        .expect("some operation");
    assert_eq!(
        op.export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ"),
        orig_proj_string
    );

    let normalized = bound.normalize_for_visualization();
    let Crs::Bound(normalized_bound) = &normalized else {
        panic!("expected a bound CRS");
    };
    let Crs::Geographic(source) = normalized_bound.source_crs() else {
        panic!("expected a geographic source CRS");
    };
    let axes = source.coordinate_system().axes();
    assert_eq!(axes.len(), 2);
    assert_eq!(axes[0].direction(), "east");
    assert_eq!(axes[1].direction(), "north");

    let op_normalized = CoordinateOperationFactory::create()
        .create_operation(&wgs84, &normalized)
        .expect("operation")
        .expect("some operation");
    assert_eq!(
        op_normalized
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ"),
        normalized_proj_string
    );
}
