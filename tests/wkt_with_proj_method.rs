//! A WKT whose method is `PROJ-based operation method: +proj=...` is read as WKT, as PROJ 9.9.0's
//! `createFromUserInput` does: WKT comes before PROJ strings (io.cpp:7780-7797). The WKT is the
//! derived projected CRS of PDAL's `CopcWriterTest.srsWkt2`; PROJ makes a `DerivedProjectedCRS`
//! (`proj_get_type` 28) named `Custom Site Calibrated CRS`.

use proj_rs::iso19111::{create_user_object, opens_as_wkt, DatabaseContext, InitRules, UserObject};
use proj_rs::unstable::Crs;

const DERIVED: &str = "DERIVEDPROJCRS[\"Custom Site Calibrated CRS\",\n    BASEPROJCRS[\"NAD83(2011) / Mississippi East (ftUS)\",\n        BASEGEOGCRS[\"NAD83(2011)\",\n            DATUM[\"NAD83 (National Spatial Reference System 2011)\",\n                ELLIPSOID[\"GRS 1980\",6378137,298.257222101,\n                    LENGTHUNIT[\"metre\",1]]],\n            PRIMEM[\"Greenwich\",0,\n                ANGLEUNIT[\"degree\",0.0174532925199433]]],\n        CONVERSION[\"SPCS83 Mississippi East zone (US Survey feet)\",\n            METHOD[\"Transverse Mercator\",\n                ID[\"EPSG\",9807]],\n            PARAMETER[\"Latitude of natural origin\",29.5,\n                ANGLEUNIT[\"degree\",0.0174532925199433],\n                ID[\"EPSG\",8801]],\n            PARAMETER[\"Longitude of natural origin\",-88.8333333333333,\n                ANGLEUNIT[\"degree\",0.0174532925199433],\n                ID[\"EPSG\",8802]],\n            PARAMETER[\"Scale factor at natural origin\",0.99995,\n                SCALEUNIT[\"unity\",1],\n                ID[\"EPSG\",8805]],\n            PARAMETER[\"False easting\",984250,\n                LENGTHUNIT[\"US survey foot\",0.304800609601219],\n                ID[\"EPSG\",8806]],\n            PARAMETER[\"False northing\",0,\n                LENGTHUNIT[\"US survey foot\",0.304800609601219],\n                ID[\"EPSG\",8807]]]],\n    DERIVINGCONVERSION[\"Affine transformation as PROJ-based\",\n        METHOD[\"PROJ-based operation method: +proj=pipeline +step +proj=unitconvert +xy_in=m +xy_out=us-ft +step +proj=affine +xoff=20 +step +proj=unitconvert +xy_in=us-ft +xy_out=m\"]],\n    CS[Cartesian,2],\n        AXIS[\"northing (Y)\",north,\n            LENGTHUNIT[\"US survey foot\",0.304800609601219]],\n        AXIS[\"easting (X)\",east,\n            LENGTHUNIT[\"US survey foot\",0.304800609601219]],\n    REMARK[\"EPSG:6507 with 20 feet offset and axis inversion\"]]";

#[test]
fn wkt_with_a_proj_based_method_is_wkt() {
    assert!(opens_as_wkt(DERIVED));
    let db = DatabaseContext::create().expect("proj.db");
    let object = create_user_object(DERIVED, Some(&db), InitRules::Iso19111).expect("WKT");
    let UserObject::Crs(Crs::DerivedProjected(crs)) = object else {
        panic!("not a derived projected CRS");
    };
    assert_eq!(crs.name(), "Custom Site Calibrated CRS");
}

#[test]
fn opens_as_wkt_reads_the_head_only() {
    assert!(opens_as_wkt("GEOGCRS [\"x\"]"));
    assert!(!opens_as_wkt("+proj=pipeline +step +proj=affine"));
    assert!(!opens_as_wkt(" GEOGCRS[\"x\"]"));
    assert!(!opens_as_wkt("EPSG:4326"));
}
