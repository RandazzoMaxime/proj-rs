//! `populateWithFakeEPSG` of PROJ 9.9.0 `test/unit/test_factory.cpp`: the SQL that gives an
//! empty structure the EPSG objects the factory tests use. Each statement is one `execute()`
//! call of the C++ fixture, in the same order.

use proj_rs::iso19111::DatabaseContext;

pub(crate) const STATEMENTS: [&str; 44] = [
    r#"INSERT INTO unit_of_measure VALUES('EPSG','9001','metre','length',1.0,NULL,0);"#,
    r#"INSERT INTO unit_of_measure VALUES('EPSG','9102','degree','angle',1.74532925199432781271e-02,NULL,0);"#,
    r#"INSERT INTO unit_of_measure VALUES('EPSG','9122','degree (supplier to define representation)','angle',1.74532925199432781271e-02,NULL,0);"#,
    r#"INSERT INTO extent VALUES('EPSG','1262','World','World.',-90.0,90.0,-180.0,180.0,0);"#,
    r#"INSERT INTO scope VALUES('EPSG','1024','Not known.',0);"#,
    r#"INSERT INTO prime_meridian VALUES('EPSG','8901','Greenwich',0.0,'EPSG','9102',0);"#,
    r#"INSERT INTO celestial_body VALUES('PROJ','EARTH','Earth',6378137.0);"#,
    r#"INSERT INTO ellipsoid VALUES('EPSG','7030','WGS 84','','PROJ','EARTH',6378137.0,'EPSG','9001',298.257223563,NULL,0);"#,
    r#"INSERT INTO geodetic_datum VALUES('EPSG','6326','World Geodetic System 1984','','EPSG','7030','EPSG','8901',NULL,NULL,NULL,'my anchor',NULL,0);"#,
    r#"INSERT INTO usage VALUES('EPSG','geodetic_datum_6326_usage','geodetic_datum','EPSG','6326','EPSG','1262','EPSG','1024');"#,
    r#"INSERT INTO vertical_datum VALUES('EPSG','1027','EGM2008 geoid',NULL,NULL,NULL,NULL,'my anchor',NULL,0);"#,
    r#"INSERT INTO usage VALUES('EPSG','vertical_datum_1027_usage','vertical_datum','EPSG','1027','EPSG','1262','EPSG','1024');"#,
    r#"INSERT INTO coordinate_system VALUES('EPSG','6422','ellipsoidal',2);"#,
    r#"INSERT INTO axis VALUES('EPSG','106','Geodetic latitude','Lat','north','EPSG','6422',1,'EPSG','9122');"#,
    r#"INSERT INTO axis VALUES('EPSG','107','Geodetic longitude','Lon','east','EPSG','6422',2,'EPSG','9122');"#,
    r#"INSERT INTO geodetic_crs VALUES('EPSG','4326','WGS 84',NULL,'geographic 2D','EPSG','6422','EPSG','6326',NULL,0);"#,
    r#"INSERT INTO usage VALUES('EPSG','geodetic_crs4326_usage','geodetic_crs','EPSG','4326','EPSG','1262','EPSG','1024');"#,
    r#"INSERT INTO coordinate_system VALUES('EPSG','6499','vertical',1);"#,
    r#"INSERT INTO axis VALUES('EPSG','114','Gravity-related height','H','up','EPSG','6499',1,'EPSG','9001');"#,
    r#"INSERT INTO vertical_crs VALUES('EPSG','3855','EGM2008 height',NULL,'EPSG','6499','EPSG','1027',0);"#,
    r#"INSERT INTO usage VALUES('EPSG','vertical_crs3855_usage','vertical_crs','EPSG','3855','EPSG','1262','EPSG','1024');"#,
    r#"INSERT INTO unit_of_measure VALUES('EPSG','9201','unity','scale',1.0,NULL,0);"#,
    r#"INSERT INTO extent VALUES('EPSG','1933','World - N hemisphere - 0°E to 6°E','',0.0,84.0,0.0,6.0,0);"#,
    r#"INSERT INTO conversion VALUES('EPSG','16031','UTM zone 31N',NULL,'EPSG','9807','Transverse Mercator','EPSG','8801','Latitude of natural origin',0.0,'EPSG','9102','EPSG','8802','Longitude of natural origin',3.0,'EPSG','9102','EPSG','8805','Scale factor at natural origin',0.9996,'EPSG','9201','EPSG','8806','False easting',500000.0,'EPSG','9001','EPSG','8807','False northing',0.0,'EPSG','9001',NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,0);"#,
    r#"INSERT INTO usage VALUES('EPSG','conversion16031_usage','conversion','EPSG','16031','EPSG','1933','EPSG','1024');"#,
    r#"INSERT INTO extent VALUES('EPSG','2060','World - N hemisphere - 0°E to 6°E - by country','',0.0,84.0,0.0,6.0,0);"#,
    r#"INSERT INTO coordinate_system VALUES('EPSG','4400','Cartesian',2);"#,
    r#"INSERT INTO axis VALUES('EPSG','1','Easting','E','east','EPSG','4400',1,'EPSG','9001');"#,
    r#"INSERT INTO axis VALUES('EPSG','2','Northing','N','north','EPSG','4400',2,'EPSG','9001');"#,
    r#"INSERT INTO projected_crs VALUES('EPSG','32631','WGS 84 / UTM zone 31N',NULL,'EPSG','4400','EPSG','4326','EPSG','16031',NULL,0);"#,
    r#"INSERT INTO usage VALUES('EPSG','projected_crs32631_usage','projected_crs','EPSG','32631','EPSG','2060','EPSG','1024');"#,
    r#"INSERT INTO compound_crs VALUES('EPSG','MY_COMPOUND','WGS 84 + EGM2008 geoid height',NULL,'EPSG','4326','EPSG','3855',0);"#,
    r#"INSERT INTO usage VALUES('EPSG','compound_crsMY_COMPOUND_usage','compound_crs','EPSG','MY_COMPOUND','EPSG','1262','EPSG','1024');"#,
    r#"INSERT INTO helmert_transformation VALUES('EPSG','DUMMY_HELMERT','dummy_helmert',NULL,'EPSG','9603','Geocentric translations (geog2D domain)','EPSG','4326','EPSG','4326',44.0,-143.0,-90.0,-294.0,'EPSG','9001',NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,0);"#,
    r#"INSERT INTO usage VALUES('EPSG','helmert_transformation_DUMMY_HELMERT_usage','helmert_transformation','EPSG','DUMMY_HELMERT','EPSG','1262','EPSG','1024');"#,
    r#"INSERT INTO grid_transformation VALUES('EPSG','DUMMY_GRID_TRANSFORMATION','dummy_grid_transformation',NULL,'EPSG','9615','NTv2','EPSG','4326','EPSG','4326',1.0,'EPSG','8656','Latitude and longitude difference file','nzgd2kgrid0005.gsb',NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,0);"#,
    r#"INSERT INTO usage VALUES('EPSG','grid_transformation_DUMMY_GRID_TRANSFORMATION_usage','grid_transformation','EPSG','DUMMY_GRID_TRANSFORMATION','EPSG','1262','EPSG','1024');"#,
    r#"INSERT INTO unit_of_measure VALUES('EPSG','9110','sexagesimal DMS','angle',NULL,NULL,0);"#,
    r#"INSERT INTO other_transformation VALUES('EPSG','DUMMY_OTHER_TRANSFORMATION','dummy_other_transformation',NULL,'EPSG','9601','Longitude rotation','EPSG','4326','EPSG','4326',0.0,'EPSG','8602','Longitude offset',-17.4,'EPSG','9110',NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,0);"#,
    r#"INSERT INTO usage VALUES('EPSG','other_transformation_DUMMY_OTHER_TRANSFORMATION_usage','other_transformation','EPSG','DUMMY_OTHER_TRANSFORMATION','EPSG','1262','EPSG','1024');"#,
    r#"INSERT INTO concatenated_operation VALUES('EPSG','DUMMY_CONCATENATED','dummy_concatenated',NULL,'EPSG','4326','EPSG','4326',NULL,NULL,0);"#,
    r#"INSERT INTO usage VALUES('EPSG','concatenated_operation_DUMMY_CONCATENATED_usage','concatenated_operation','EPSG','DUMMY_CONCATENATED','EPSG','1262','EPSG','1024');"#,
    r#"INSERT INTO concatenated_operation_step VALUES('EPSG','DUMMY_CONCATENATED',1,'EPSG','DUMMY_OTHER_TRANSFORMATION',NULL);"#,
    r#"INSERT INTO concatenated_operation_step VALUES('EPSG','DUMMY_CONCATENATED',2,'EPSG','DUMMY_OTHER_TRANSFORMATION',NULL);"#,
];

/// `createStructure()` then `populateWithFakeEPSG()` of the C++ fixture: an empty structure
/// with foreign keys on, filled with the fake EPSG objects.
pub(crate) fn database() -> DatabaseContext {
    let db = DatabaseContext::with_empty_structure().expect("empty structure");
    db.execute_sql("PRAGMA foreign_keys = 1;").expect("pragma");
    for sql in STATEMENTS {
        db.execute_sql(sql)
            .unwrap_or_else(|error| panic!("{sql}: {error}"));
    }
    db
}
