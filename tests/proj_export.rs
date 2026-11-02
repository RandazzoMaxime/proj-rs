//! `Proj::as_wkt`, `Proj::to_projjson`, and the ISO 19111 writers behind them.
//!
//! The expected texts are PROJ 9.9.0's output, copied by script from the sources named on
//! each constant: `tests/cli_yaml/test_projinfo.yaml` (the `projinfo -o ALL EPSG:4326` case)
//! and `test/unit/test_io.cpp` (`json_import.projected_crs`). A failing test is a gap or a
//! regression in this crate, never a reason to edit one of these texts.

use proj_rs::iso19111::{create_from_user_input, DatabaseContext, JsonFormat};
use proj_rs::wkt::{WktOptions, WktVersion};
use proj_rs::Proj;

const WKT2_2015_4326: &str = r#"GEODCRS["WGS 84",
    DATUM["World Geodetic System 1984",
        ELLIPSOID["WGS 84",6378137,298.257223563,
            LENGTHUNIT["metre",1]]],
    PRIMEM["Greenwich",0,
        ANGLEUNIT["degree",0.0174532925199433]],
    CS[ellipsoidal,2],
        AXIS["geodetic latitude (Lat)",north,
            ORDER[1],
            ANGLEUNIT["degree",0.0174532925199433]],
        AXIS["geodetic longitude (Lon)",east,
            ORDER[2],
            ANGLEUNIT["degree",0.0174532925199433]],
    SCOPE["Horizontal component of 3D system."],
    AREA["World."],
    BBOX[-90,-180,90,180],
    ID["EPSG",4326]]"#;

const WKT2_2019_4326: &str = r#"GEOGCRS["WGS 84",
    ENSEMBLE["World Geodetic System 1984 ensemble",
        MEMBER["World Geodetic System 1984 (Transit)"],
        MEMBER["World Geodetic System 1984 (G730)"],
        MEMBER["World Geodetic System 1984 (G873)"],
        MEMBER["World Geodetic System 1984 (G1150)"],
        MEMBER["World Geodetic System 1984 (G1674)"],
        MEMBER["World Geodetic System 1984 (G1762)"],
        MEMBER["World Geodetic System 1984 (G2139)"],
        MEMBER["World Geodetic System 1984 (G2296)"],
        ELLIPSOID["WGS 84",6378137,298.257223563,
            LENGTHUNIT["metre",1]],
        ENSEMBLEACCURACY[2.0]],
    PRIMEM["Greenwich",0,
        ANGLEUNIT["degree",0.0174532925199433]],
    CS[ellipsoidal,2],
        AXIS["geodetic latitude (Lat)",north,
            ORDER[1],
            ANGLEUNIT["degree",0.0174532925199433]],
        AXIS["geodetic longitude (Lon)",east,
            ORDER[2],
            ANGLEUNIT["degree",0.0174532925199433]],
    USAGE[
        SCOPE["Horizontal component of 3D system."],
        AREA["World."],
        BBOX[-90,-180,90,180]],
    ID["EPSG",4326]]"#;

const PROJJSON_4326: &str = r#"{
  "$schema": "https://proj.org/schemas/v0.7/projjson.schema.json",
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
      },
      {
        "name": "World Geodetic System 1984 (G2296)",
        "id": {
          "authority": "EPSG",
          "code": 1383
        }
      }
    ],
    "ellipsoid": {
      "name": "WGS 84",
      "semi_major_axis": 6378137,
      "inverse_flattening": 298.257223563
    },
    "accuracy": "2.0",
    "id": {
      "authority": "EPSG",
      "code": 6326
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
  "scope": "Horizontal component of 3D system.",
  "area": "World.",
  "bbox": {
    "south_latitude": -90,
    "west_longitude": -180,
    "north_latitude": 90,
    "east_longitude": 180
  },
  "id": {
    "authority": "EPSG",
    "code": 4326
  }
}"#;

const PROJJSON_PROJECTED_TEST: &str = r#"{
  "$schema": "foo",
  "type": "ProjectedCRS",
  "name": "WGS 84 / UTM zone 31N",
  "base_crs": {
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
      "code": 4326
    }
  },
  "conversion": {
    "name": "UTM zone 31N",
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
        "value": 3,
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
    ]
  },
  "coordinate_system": {
    "subtype": "Cartesian",
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

const PROJJSON_GEOCENTRIC_TEST: &str = r#"{
  "$schema": "foo",
  "type": "GeodeticCRS",
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
    "subtype": "Cartesian",
    "axis": [
      {
        "name": "Geocentric X",
        "abbreviation": "X",
        "direction": "geocentricX",
        "unit": "metre"
      },
      {
        "name": "Geocentric Y",
        "abbreviation": "Y",
        "direction": "geocentricY",
        "unit": "metre"
      },
      {
        "name": "Geocentric Z",
        "abbreviation": "Z",
        "direction": "geocentricZ",
        "unit": "metre"
      }
    ]
  }
}"#;

const PROJJSON_VERTICAL_TEST: &str = r#"{
  "$schema": "foo",
  "type": "VerticalCRS",
  "name": "EGM2008 height",
  "datum": {
    "type": "VerticalReferenceFrame",
    "name": "EGM2008 geoid"
  },
  "coordinate_system": {
    "subtype": "vertical",
    "axis": [
      {
        "name": "Gravity-related height",
        "abbreviation": "H",
        "direction": "up",
        "unit": "metre"
      }
    ]
  }
}"#;

const PROJJSON_COMPOUND_TEST: &str = r#"{
  "$schema": "foo",
  "type": "CompoundCRS",
  "name": "WGS 84 + EGM2008 height",
  "components": [
    {
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
        "code": 4326
      }
    },
    {
      "type": "VerticalCRS",
      "name": "EGM2008 height",
      "datum": {
        "type": "VerticalReferenceFrame",
        "name": "EGM2008 geoid"
      },
      "coordinate_system": {
        "subtype": "vertical",
        "axis": [
          {
            "name": "Gravity-related height",
            "abbreviation": "H",
            "direction": "up",
            "unit": "metre"
          }
        ]
      },
      "id": {
        "authority": "EPSG",
        "code": 3855
      }
    }
  ]
}"#;

const PROJJSON_BOUND_TEST: &str = r#"{
  "$schema": "foo",
  "type": "BoundCRS",
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
  }
}"#;

#[test]
fn wkt2_2015_of_4326_is_proj_text() {
    let proj = Proj::new("EPSG:4326").expect("crs");
    let wkt = proj.as_wkt(Some(WktVersion::Wkt2_2015), None).expect("wkt");
    assert_eq!(wkt, WKT2_2015_4326);
}

#[test]
fn wkt2_2019_of_4326_is_proj_text() {
    let proj = Proj::new("EPSG:4326").expect("crs");
    let wkt = proj.as_wkt(Some(WktVersion::Wkt2_2019), None).expect("wkt");
    assert_eq!(wkt, WKT2_2019_4326);
}

#[test]
fn wkt_defaults_to_wkt2_2019() {
    let proj = Proj::new("EPSG:4326").expect("crs");
    let default = proj.as_wkt(None, Some(WktOptions::default())).expect("wkt");
    let explicit = proj.as_wkt(Some(WktVersion::Wkt2_2019), None).expect("wkt");
    assert_eq!(default, explicit);
}

#[test]
fn projjson_of_4326_is_proj_text() {
    let proj = Proj::new("EPSG:4326").expect("crs");
    let json = proj.to_projjson(None, None, None).expect("projjson");
    assert_eq!(json, PROJJSON_4326);
}

#[test]
fn projjson_of_a_projected_crs_roundtrips_through_the_facade() {
    // PROJ's projected-CRS unit text, imported by `Proj::new` and written back.
    // The comparison drops the base CRS `"type": "GeographicCRS"` line, which
    // that unit test also drops.
    let proj = Proj::new(PROJJSON_PROJECTED_TEST).expect("projected json");
    let json = proj.to_projjson(None, None, Some("foo")).expect("export");
    let json = json.replacen("    \"type\": \"GeographicCRS\",\n", "", 1);
    assert_eq!(json, PROJJSON_PROJECTED_TEST);
}

#[test]
fn projjson_of_projected_crs_is_proj_unit_test_text() {
    // The base CRS of PROJ's test carries `"type": "GeographicCRS"`, which the test's own
    // comparison removes. The CRS is written here from WKT with the same identifiers.
    let db = DatabaseContext::create().expect("proj.db");
    let crs = create_from_user_input(PROJECTED_WKT, &db).expect("projected");
    let json = crs
        .export_to_json_with(&db, &JsonFormat::default().with_schema("foo"))
        .expect("projjson");
    let json = json.replacen("    \"type\": \"GeographicCRS\",\n", "", 1);
    assert_eq!(json, PROJJSON_PROJECTED_TEST);
}

#[test]
fn projjson_single_line_has_no_newline_and_no_space_after_colon() {
    let proj = Proj::new("EPSG:4326").expect("crs");
    let json = proj.to_projjson(Some(false), None, None).expect("projjson");
    assert!(!json.contains('\n'), "{json}");
    assert!(
        json.starts_with("{\"$schema\":\"https://proj.org/schemas/v0.7/projjson.schema.json\","),
        "{json}"
    );
    assert!(json.ends_with("}}") || json.ends_with('}'), "{json}");
}

#[test]
fn projjson_indentation_width_and_schema_are_honoured() {
    let proj = Proj::new("EPSG:4326").expect("crs");
    let json = proj
        .to_projjson(None, Some(4), Some("foo"))
        .expect("projjson");
    assert!(
        json.starts_with("{\n    \"$schema\": \"foo\",\n    \"type\": \"GeographicCRS\""),
        "{json}"
    );
    let no_schema = proj.to_projjson(None, None, Some("")).expect("projjson");
    assert!(!no_schema.contains("$schema"), "{no_schema}");
}

#[test]
fn transform_between_two_crs_exports_the_operation() {
    // PROJ 9.8.1 `proj_create_crs_to_crs` + `proj_as_projjson` / `proj_as_wkt`
    // (WKT2:2019) / `proj_pj_info`, pyproj 3.8.0's libproj. The bytes of the
    // JSON and the WKT are those exports (10 958 and 6 238 bytes).
    let proj = Proj::new_known_crs("EPSG:2230", "EPSG:26946", None).expect("pair");
    let definition = proj.def().expect("definition");
    assert_eq!(
        definition,
        "proj=pipeline step proj=unitconvert xy_in=us-ft xy_out=m step inv proj=lcc lat_0=32.1666666666667 lon_0=-116.25 lat_1=33.8833333333333 lat_2=32.7833333333333 x_0=2000000.0001016 y_0=500000.0001016 ellps=GRS80 step proj=lcc lat_0=32.1666666666667 lon_0=-116.25 lat_1=33.8833333333333 lat_2=32.7833333333333 x_0=2000000 y_0=500000 ellps=GRS80"
    );
    let info = proj.proj_info();
    assert_eq!(info.id.as_deref(), Some("pipeline"));
    assert_eq!(
        info.description.as_deref(),
        Some("Inverse of SPCS83 California zone 6 (US survey foot) + SPCS83 California zone 6 (meter)")
    );
    let json = proj.to_projjson(None, None, None).expect("projjson");
    let wkt = proj.as_wkt(None, None).expect("wkt");
    assert_eq!(json.len(), 10958, "{json}");
    assert_eq!(wkt.len(), 6238, "{wkt}");
    assert!(
        json.contains("\"type\": \"ConcatenatedOperation\""),
        "{json}"
    );
    assert!(wkt.starts_with(
        "CONCATENATEDOPERATION[\"Inverse of SPCS83 California zone 6 (US survey foot) + SPCS83 California zone 6 (meter)\""
    ));
}

const GEOCENTRIC_WKT: &str = r#"GEODCRS["WGS 84",
    DATUM["World Geodetic System 1984",
        ELLIPSOID["WGS 84",6378137,298.257223563,
            LENGTHUNIT["metre",1]]],
    CS[Cartesian,3],
        AXIS["Geocentric X (X)",geocentricX,
            ORDER[1],
            LENGTHUNIT["metre",1]],
        AXIS["Geocentric Y (Y)",geocentricY,
            ORDER[2],
            LENGTHUNIT["metre",1]],
        AXIS["Geocentric Z (Z)",geocentricZ,
            ORDER[3],
            LENGTHUNIT["metre",1]]]"#;

const VERTICAL_WKT: &str = r#"VERTCRS["EGM2008 height",
    VDATUM["EGM2008 geoid"],
    CS[vertical,1],
        AXIS["Gravity-related height (H)",up,
            LENGTHUNIT["metre",1]]]"#;

const COMPOUND_WKT: &str = r#"COMPOUNDCRS["WGS 84 + EGM2008 height",
    GEOGCRS["WGS 84",
        DATUM["World Geodetic System 1984",
            ELLIPSOID["WGS 84",6378137,298.257223563,
                LENGTHUNIT["metre",1]]],
        CS[ellipsoidal,2],
            AXIS["Geodetic latitude (Lat)",north,
                ANGLEUNIT["degree",0.0174532925199433]],
            AXIS["Geodetic longitude (Lon)",east,
                ANGLEUNIT["degree",0.0174532925199433]],
        ID["EPSG",4326]],
    VERTCRS["EGM2008 height",
        VDATUM["EGM2008 geoid"],
        CS[vertical,1],
            AXIS["Gravity-related height (H)",up,
                LENGTHUNIT["metre",1]],
        ID["EPSG",3855]]]"#;

#[test]
fn projjson_of_geocentric_crs_is_proj_unit_test_text() {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = create_from_user_input(GEOCENTRIC_WKT, &db).expect("geocentric");
    let json = crs
        .export_to_json_with(&db, &JsonFormat::default().with_schema("foo"))
        .expect("projjson");
    assert_eq!(json, PROJJSON_GEOCENTRIC_TEST);
}

#[test]
fn projjson_of_vertical_crs_is_proj_unit_test_text() {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = create_from_user_input(VERTICAL_WKT, &db).expect("vertical");
    let json = crs
        .export_to_json_with(&db, &JsonFormat::default().with_schema("foo"))
        .expect("projjson");
    assert_eq!(json, PROJJSON_VERTICAL_TEST);
}

#[test]
fn projjson_of_compound_crs_is_proj_unit_test_text() {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = create_from_user_input(COMPOUND_WKT, &db).expect("compound");
    let json = crs
        .export_to_json_with(&db, &JsonFormat::default().with_schema("foo"))
        .expect("projjson");
    assert_eq!(json, PROJJSON_COMPOUND_TEST);
}

const BOUND_WKT: &str = r#"BOUNDCRS[
    SOURCECRS[GEOGCRS["unknown",
        DATUM["Unknown based on GRS80 ellipsoid",
            ELLIPSOID["GRS 1980",6378137,298.257222101,
                LENGTHUNIT["metre",1],
                ID["EPSG",7019]]],
        CS[ellipsoidal,2],
            AXIS["Longitude (lon)",east,
                ORDER[1],
                ANGLEUNIT["degree",0.0174532925199433]],
            AXIS["Latitude (lat)",north,
                ORDER[2],
                ANGLEUNIT["degree",0.0174532925199433]]]],
    TARGETCRS[GEOGCRS["WGS 84",
        DATUM["World Geodetic System 1984",
            ELLIPSOID["WGS 84",6378137,298.257223563,
                LENGTHUNIT["metre",1]]],
        CS[ellipsoidal,2],
            AXIS["Latitude (lat)",north,
                ORDER[1],
                ANGLEUNIT["degree",0.0174532925199433]],
            AXIS["Longitude (lon)",east,
                ORDER[2],
                ANGLEUNIT["degree",0.0174532925199433]],
        ID["EPSG",4326]]],
    ABRIDGEDTRANSFORMATION["unknown to WGS84",
        METHOD["NTv2",
            ID["EPSG",9615]],
        PARAMETERFILE["Latitude and longitude difference file","@foo",
            ID["EPSG",8656]]]]"#;

#[test]
fn projjson_of_bound_crs_is_proj_unit_test_text() {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = create_from_user_input(BOUND_WKT, &db).expect("bound");
    let json = crs
        .export_to_json_with(&db, &JsonFormat::default().with_schema("foo"))
        .expect("projjson");
    assert_eq!(json, PROJJSON_BOUND_TEST);
}

const PROJECTED_WKT: &str = r#"PROJCRS["WGS 84 / UTM zone 31N",
    BASEGEOGCRS["WGS 84",
        DATUM["World Geodetic System 1984",
            ELLIPSOID["WGS 84",6378137,298.257223563,
                LENGTHUNIT["metre",1]]],
        PRIMEM["Greenwich",0,
            ANGLEUNIT["degree",0.0174532925199433]],
        ID["EPSG",4326]],
    CONVERSION["UTM zone 31N",
        METHOD["Transverse Mercator",
            ID["EPSG",9807]],
        PARAMETER["Latitude of natural origin",0,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",8801]],
        PARAMETER["Longitude of natural origin",3,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",8802]],
        PARAMETER["Scale factor at natural origin",0.9996,
            SCALEUNIT["unity",1],
            ID["EPSG",8805]],
        PARAMETER["False easting",500000,
            LENGTHUNIT["metre",1],
            ID["EPSG",8806]],
        PARAMETER["False northing",0,
            LENGTHUNIT["metre",1],
            ID["EPSG",8807]]],
    CS[Cartesian,2],
        AXIS["easting (E)",east,
            ORDER[1],
            LENGTHUNIT["metre",1]],
        AXIS["northing (N)",north,
            ORDER[2],
            LENGTHUNIT["metre",1]]]"#;
