//! PROJ 9.9.0 `test/unit/test_io.cpp`, lot `io-remaining`: PROJJSON import and export
//! cases that were still open in `docs/UNPORTED-UNIT.md`.
//!
//! Each function keeps the C++ `TEST()` name in its doc comment. The PROJJSON texts are
//! PROJ's, byte for byte, and each test compares the export with the input.

use proj_rs::iso19111::{
    create_object_from_json, create_object_from_user_input, Crs, DatabaseContext, JsonFormat,
    JsonObject, UserObject,
};

fn export_foo(object: &JsonObject) -> String {
    let db = DatabaseContext::create().expect("proj.db");
    object
        .export_to_json_with(&db, &JsonFormat::default().with_schema("foo"))
        .expect("PROJJSON export")
}

/// `TEST(json_import, several_usages)`
#[test]
fn json_import_several_usages() {
    let json = r#"{
  "$schema": "foo",
  "type": "GeodeticReferenceFrame",
  "name": "World Geodetic System 1984",
  "ellipsoid": {
    "name": "WGS 84",
    "semi_major_axis": 6378137,
    "inverse_flattening": 298.257223563
  },
  "usages": [
    {
      "area": "World",
      "bbox": {
        "south_latitude": -90,
        "west_longitude": -180,
        "north_latitude": 90,
        "east_longitude": 180
      }
    },
    {
      "scope": "my_scope",
      "area": "my_area"
    }
  ]
}"#;
    let object = create_object_from_json(json, None).expect("PROJJSON");
    assert!(matches!(object, JsonObject::GeodeticReferenceFrame(_)));
    assert_eq!(export_foo(&object), json);
}

/// `TEST(json_import, bound_crs_with_name_and_usage)`
#[test]
fn json_import_bound_crs_with_name_and_usage() {
    let json = r#"{
  "$schema": "foo",
  "type": "BoundCRS",
  "name": "my bound crs",
  "source_crs": {
    "type": "GeographicCRS",
    "name": "unknown",
    "datum": {
      "type": "GeodeticReferenceFrame",
      "name": "Unknown based on GRS80 ellipsoid",
      "ellipsoid": {
        "name": "GRS 1980",
        "semi_major_axis": 6378137,
        "inverse_flattening": 298.257222101,
        "id": {
          "authority": "EPSG",
          "code": 7019
        }
      }
    },
    "coordinate_system": {
      "subtype": "ellipsoidal",
      "axis": [
        {
          "name": "Longitude",
          "abbreviation": "lon",
          "direction": "east",
          "unit": "degree"
        },
        {
          "name": "Latitude",
          "abbreviation": "lat",
          "direction": "north",
          "unit": "degree"
        }
      ]
    }
  },
  "target_crs": {
    "type": "GeographicCRS",
    "name": "WGS 84",
    "datum": {
      "type": "GeodeticReferenceFrame",
      "name": "World Geodetic System 1984",
      "ellipsoid": {
        "name": "WGS 84",
        "semi_major_axis": 6378137,
        "inverse_flattening": 298.257223563
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
        }
      ]
    },
    "id": {
      "authority": "EPSG",
      "code": 4326
    }
  },
  "transformation": {
    "name": "unknown to WGS84",
    "method": {
      "name": "NTv2",
      "id": {
        "authority": "EPSG",
        "code": 9615
      }
    },
    "parameters": [
      {
        "name": "Latitude and longitude difference file",
        "value": "@foo",
        "id": {
          "authority": "EPSG",
          "code": 8656
        }
      }
    ]
  },
  "scope": "Example only (fictitious).",
  "area": "Description of the extent of the CRS.",
  "bbox": {
    "south_latitude": -90,
    "west_longitude": -180,
    "north_latitude": 90,
    "east_longitude": 180
  },
  "id": {
    "authority": "foo",
    "code": 1234
  }
}"#;
    let object = create_object_from_json(json, None).expect("PROJJSON");
    assert!(matches!(object, JsonObject::Crs(Crs::Bound(_))));
    assert_eq!(export_foo(&object), json);
}

/// `TEST(json_import, engineering_crs_affine_CS)`
#[test]
fn json_import_engineering_crs_affine_cs() {
    let json = r#"{
  "$schema": "foo",
  "type": "EngineeringCRS",
  "name": "myEngCRS",
  "datum": {
    "name": "myEngDatum"
  },
  "coordinate_system": {
    "subtype": "affine",
    "axis": [
      {
        "name": "Easting",
        "abbreviation": "E",
        "direction": "east",
        "unit": "metre"
      },
      {
        "name": "Northing",
        "abbreviation": "N",
        "direction": "north",
        "unit": "metre"
      }
    ]
  }
}"#;
    let db = DatabaseContext::create().expect("proj.db");
    let object = create_object_from_user_input(json, &db).expect("PROJJSON");
    assert!(matches!(object, UserObject::Crs(Crs::Engineering(_))));
    let format = JsonFormat::default().with_schema("foo");
    assert_eq!(object.export_to_json(&db, &format).expect("export"), json);
}

/// `TEST(json_import, multiple_ids)`
#[test]
fn json_import_multiple_ids() {
    let json = r#"{
  "$schema": "foo",
  "type": "Ellipsoid",
  "name": "WGS 84",
  "semi_major_axis": 6378137,
  "inverse_flattening": 298.257223563,
  "ids": [
    {
      "authority": "EPSG",
      "code": 4326
    },
    {
      "authority": "FOO",
      "code": "BAR"
    }
  ]
}"#;
    let object = create_object_from_json(json, None).expect("PROJJSON");
    assert!(matches!(object, JsonObject::Ellipsoid(_)));
    assert_eq!(export_foo(&object), json);
}
