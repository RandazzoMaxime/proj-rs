//! PROJ 9.9.0 `test/unit/test_crs.cpp`: identification of projected CRS against the database
//! (`projectedCRS_identify_db`).
//!
//! Each block of the C++ test is a `block!` here: a failing block is reported by its number, and
//! the other blocks still run.

use proj_rs::iso19111::{
    create_crs_from_proj_string, create_crs_from_proj_string_with_database, AuthorityFactory, Crs,
    DatabaseContext, ProjectedCRS, WktDialect, WktParser,
};

/// The EPSG code of a CRS, `""` when it has none, as `getEPSGCode()` reads it.
fn epsg(crs: &Crs) -> String {
    crs.authority_code("EPSG")
        .map_or_else(String::new, |identifier| identifier.code().to_owned())
}

/// A WKT string read with the database attached, as `WKTParser().attachDatabaseContext(db)`.
fn wkt_with_db(db: &DatabaseContext, wkt: &str) -> Crs {
    let mut parser = WktParser::new();
    parser.attach_database(db.clone());
    parser
        .create_from_wkt(wkt)
        .expect("WKT")
        .crs()
        .cloned()
        .expect("a CRS")
}

fn projected(crs: Crs) -> ProjectedCRS {
    match crs {
        Crs::Projected(crs) => crs,
        other => panic!("not a projected CRS: {other:?}"),
    }
}

/// `ProjectedCRS::create(props with name, sourceCRS->baseCRS(), derivingConversion, cs)`, with
/// the optional EPSG code of the C++ properties.
fn from_source(source: &ProjectedCRS, name: &str, code: Option<&str>) -> Crs {
    let mut crs = ProjectedCRS::create(
        name,
        source.base_crs().clone(),
        source.deriving_conversion().clone(),
        source.coordinate_system().clone(),
    );
    if let Some(code) = code {
        crs = crs.with_identifier("EPSG", code);
    }
    Crs::Projected(crs)
}

/// The panic message of a caught block.
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| {
            payload
                .downcast_ref::<&str>()
                .map(|text| (*text).to_owned())
        })
        .unwrap_or_default()
}

/// `TEST(crs, projectedCRS_identify_db)`
#[test]
fn crs_projected_crs_identify_db() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory_epsg = AuthorityFactory::create(&db, "EPSG");
    let factory_ignf = AuthorityFactory::create(&db, "IGNF");
    let factory_anonymous = AuthorityFactory::create(&db, "");
    let factory_all = AuthorityFactory::create(&db, "");
    let mut failures: Vec<String> = Vec::new();

    macro_rules! block {
        ($name:literal, $body:block) => {
            if let Err(payload) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $body)) {
                failures.push(format!("{}: {}", $name, panic_message(&*payload)));
            }
        };
    }

    block!("identify by existing code", {
        let crs = factory_epsg.create_projected_crs("2172").expect("2172");
        let res = Crs::Projected(crs.clone()).identify_with(Some(&factory_epsg));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "2172");
        assert_eq!(res[0].1, 100);
        assert_eq!(
            Crs::Projected(crs.clone())
                .identify_with(Some(&factory_anonymous))
                .len(),
            1
        );
        assert_eq!(
            Crs::Projected(crs).identify_with(Some(&factory_ignf)).len(),
            0
        );
    });

    block!("identify by existing code, IGNF source", {
        let crs = factory_ignf
            .create_projected_crs("ETRS89UTM28")
            .expect("ETRS89UTM28");
        let res = Crs::Projected(crs).identify_with(Some(&factory_epsg));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "25828");
        assert_eq!(res[0].1, 70);
    });

    block!("non-existing code", {
        let source = factory_epsg.create_projected_crs("2172").expect("2172");
        let crs = from_source(&source, "Pulkovo 1942(58) / Poland zone II", Some("1"));
        let res = crs.identify_with(Some(&factory_epsg));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "2172");
        assert_eq!(res[0].1, 70);
    });

    block!("existing code, not matching content", {
        let source = factory_epsg.create_projected_crs("2172").expect("2172");
        let crs = from_source(&source, "Pulkovo 1942(58) / Poland zone II", Some("32631"));
        let res = crs.identify_with(Some(&factory_epsg));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "2172");
        assert_eq!(res[0].1, 70);
    });

    block!("identify by exact name", {
        let source = factory_epsg.create_projected_crs("2172").expect("2172");
        let crs = from_source(&source, "Pulkovo 1942(58) / Poland zone II", None);
        let res = crs.identify_with(Some(&factory_epsg));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "2172");
        assert_eq!(res[0].1, 100);
    });

    block!("identify by equivalent name", {
        let source = factory_epsg.create_projected_crs("2172").expect("2172");
        let crs = from_source(&source, "Pulkovo_1942_58_Poland_zone_II", None);
        let res = crs.identify_with(Some(&factory_epsg));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "2172");
        assert_eq!(res[0].1, 90);
    });

    block!("identify by properties", {
        let source = factory_epsg.create_projected_crs("2172").expect("2172");
        let crs = from_source(&source, "i am a faked name", None);
        let res = crs.identify_with(Some(&factory_epsg));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "2172");
        assert_eq!(res[0].1, 70);
    });

    block!("name, but objects aren't equivalent", {
        let source = factory_epsg.create_projected_crs("3375").expect("3375");
        let crs = from_source(&source, "Pulkovo 1942(58) / Poland zone II", None);
        let res = crs.identify_with(Some(&factory_epsg));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "2172");
        assert_eq!(res[0].1, 25);
    });

    block!("PROJ string, omerc with the pipeline", {
        let crs = create_crs_from_proj_string(
            "+type=crs +proj=pipeline +step +proj=axisswap +order=2,1 +step \
             +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=omerc \
             +no_uoff +lat_0=4 +lonc=102.25 +alpha=323.025796466667 \
             +gamma=323.130102361111 +k=0.99984 +x_0=804671 +y_0=0 \
             +ellps=GRS80",
        )
        .expect("PROJ string");
        let res = crs.identify_with(Some(&factory_epsg));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "3375");
        assert_eq!(res[0].1, 70);
    });

    block!("PROJ string, omerc with the wrong axis order", {
        let crs = create_crs_from_proj_string(
            "+proj=omerc +no_uoff +lat_0=4 +lonc=102.25 \
             +alpha=323.025796466667 +gamma=323.130102361111 +k=0.99984 \
             +x_0=804671 +y_0=0 +ellps=GRS80 +type=crs",
        )
        .expect("PROJ string");
        let res = crs.identify_with(Some(&factory_epsg));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "3375");
        assert_eq!(res[0].1, 70);
    });

    block!("WKT1 with explicit correct axis order", {
        let crs = wkt_with_db(
            &db,
            "PROJCS[\"ETRS89 / UTM zone 32N (N-E)\",GEOGCS[\"ETRS89\",\
             DATUM[\"European_Terrestrial_Reference_System_1989\",\
             SPHEROID[\"GRS 1980\",6378137,298.257222101]],\
             PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433]],\
             PROJECTION[\"Transverse_Mercator\"],\
             PARAMETER[\"latitude_of_origin\",0],\
             PARAMETER[\"central_meridian\",9],\
             PARAMETER[\"scale_factor\",0.9996],\
             PARAMETER[\"false_easting\",500000],\
             PARAMETER[\"false_northing\",0],\
             UNIT[\"metre\",1],\
             AXIS[\"Northing\",NORTH],AXIS[\"Easting\",EAST]]",
        );
        let res = projected(crs).identify_with(Some(&factory_epsg));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "3044");
        assert_eq!(res[0].1, 100);
    });

    block!("WKT1 with wrong axis order", {
        let crs = wkt_with_db(
            &db,
            "PROJCS[\"ETRS89 / UTM zone 32N (N-E)\",GEOGCS[\"ETRS89\",\
             DATUM[\"European_Terrestrial_Reference_System_1989\",\
             SPHEROID[\"GRS 1980\",6378137,298.257222101]],\
             PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433]],\
             PROJECTION[\"Transverse_Mercator\"],\
             PARAMETER[\"latitude_of_origin\",0],\
             PARAMETER[\"central_meridian\",9],\
             PARAMETER[\"scale_factor\",0.9996],\
             PARAMETER[\"false_easting\",500000],\
             PARAMETER[\"false_northing\",0],\
             UNIT[\"metre\",1],\
             AXIS[\"Easting\",EAST], AXIS[\"Northing\",NORTH]]",
        );
        let res = projected(crs).identify_with(Some(&factory_epsg));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "3044");
        assert_eq!(res[0].1, 50);
    });

    block!("WKT1 without explicit axis", {
        let crs = wkt_with_db(
            &db,
            "PROJCS[\"ETRS89 / UTM zone 32N (N-E)\",GEOGCS[\"ETRS89\",\
             DATUM[\"European_Terrestrial_Reference_System_1989\",\
             SPHEROID[\"GRS 1980\",6378137,298.257222101]],\
             PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433]],\
             PROJECTION[\"Transverse_Mercator\"],\
             PARAMETER[\"latitude_of_origin\",0],\
             PARAMETER[\"central_meridian\",9],\
             PARAMETER[\"scale_factor\",0.9996],\
             PARAMETER[\"false_easting\",500000],\
             PARAMETER[\"false_northing\",0],\
             UNIT[\"metre\",1]]",
        );
        let res = projected(crs).identify_with(Some(&factory_epsg));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "3044");
        assert_eq!(res[0].1, 100);
    });

    block!("WKT ESRI with bad PROJCS and GEOGCS names", {
        let crs = wkt_with_db(
            &db,
            "PROJCS[\"Lambert Conformal Conic\",GEOGCS[\"grs80\",\
             DATUM[\"D_North_American_1983\",\
             SPHEROID[\"Geodetic_Reference_System_1980\",\
             6378137,298.257222101]],PRIMEM[\"Greenwich\",0],\
             UNIT[\"Degree\",0.017453292519943295]],\
             PROJECTION[\"Lambert_Conformal_Conic\"],\
             PARAMETER[\"standard_parallel_1\",34.33333333333334],\
             PARAMETER[\"standard_parallel_2\",36.16666666666666],\
             PARAMETER[\"latitude_of_origin\",33.75],\
             PARAMETER[\"central_meridian\",-79],\
             PARAMETER[\"false_easting\",609601.22],\
             PARAMETER[\"false_northing\",0],UNIT[\"Meter\",1]]",
        );
        let res = projected(crs).identify_with(Some(&factory_epsg));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "32119");
        assert_eq!(res[0].1, 70);
    });

    block!("no equivalent CRS in the result set", {
        let crs = create_crs_from_proj_string("+proj=tmerc +datum=WGS84 +type=crs").expect("PROJ");
        let res = projected(crs).identify_with(Some(&factory_epsg));
        assert_eq!(res.len(), 0);
    });

    block!("ESRI:103729 as a PROJ string", {
        let crs = create_crs_from_proj_string_with_database(
            "+proj=lcc +lat_0=43.5 +lon_0=-93.95 +lat_1=43.5666666666667 \
             +lat_2=43.8 +x_0=152400.30480061 +y_0=30480.0609601219 \
             +a=6378521.049 +rf=298.257222100883 +units=us-ft +type=crs",
            &db,
        )
        .expect("PROJ string");
        let res = projected(crs).identify_with(Some(&factory_all));
        assert!(!res.is_empty());
        let found = res
            .iter()
            .find(|(crs, _)| crs.identifiers()[0].code() == "103729")
            .expect("ESRI:103729");
        assert_eq!(found.1, 70);
    });

    block!("EPSG:2327 as a PROJ string", {
        let crs = create_crs_from_proj_string_with_database(
            "+proj=tmerc +lat_0=0 +lon_0=75 +k=1 +x_0=13500000 +y_0=0 \
             +a=6378140 +b=6356755.288157528 +units=m +type=crs",
            &db,
        )
        .expect("PROJ string");
        let res = projected(crs).identify_with(Some(&factory_epsg));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "2327");
        assert_eq!(res[0].1, 70);
    });

    block!("EPSG:6646 as a PROJ string with clrk80", {
        let crs = create_crs_from_proj_string_with_database(
            "+proj=tmerc +lat_0=29.02626833333333 +lon_0=46.5 \
             +k=0.9994 +x_0=800000 +y_0=0 +ellps=clrk80 \
             +units=m +type=crs",
            &db,
        )
        .expect("PROJ string");
        let res = projected(crs).identify_with(Some(&factory_epsg));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "6646");
        assert_eq!(res[0].1, 70);
    });

    block!("WKT ESRI that has the name of ESRI:102039 in US feet", {
        let crs = wkt_with_db(
            &db,
            "PROJCS[\"USA_Contiguous_Albers_Equal_Area_Conic_USGS_version\",\
             GEOGCS[\"GCS_North_American_1983\",\
             DATUM[\"D_North_American_1983\",SPHEROID[\"GRS_1980\",\
             6378137.0,298.257222101]],PRIMEM[\"Greenwich\",0.0],\
             UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"Albers\"],\
             PARAMETER[\"False_Easting\",0.0],\
             PARAMETER[\"False_Northing\",0.0],\
             PARAMETER[\"Central_Meridian\",-96.0],\
             PARAMETER[\"Standard_Parallel_1\",29.5],\
             PARAMETER[\"Standard_Parallel_2\",45.5],\
             PARAMETER[\"Latitude_Of_Origin\",23.0],\
             UNIT[\"Foot_US\",0.3048006096012192]]",
        );
        let res = projected(crs).identify_with(Some(&factory_all));
        assert!(!res.is_empty());
        let found = res
            .iter()
            .find(|(crs, _)| crs.identifiers()[0].code() == "102039")
            .expect("ESRI:102039");
        assert_eq!(found.1, 50);
    });

    block!(
        "ESRI WKT where the EPSG system has Northing/Easting order",
        {
            let crs = wkt_with_db(
                &db,
                "PROJCS[\"NZGD2000_New_Zealand_Transverse_Mercator_2000\",\
             GEOGCS[\"GCS_NZGD2000\",\
             DATUM[\"New_Zealand_Geodetic_Datum_2000\",\
             SPHEROID[\"GRS 1980\",6378137,298.2572221010042,\
             AUTHORITY[\"EPSG\",\"7019\"]],\
             AUTHORITY[\"EPSG\",\"6167\"]],\
             PRIMEM[\"Greenwich\",0],\
             UNIT[\"degree\",0.0174532925199433]],\
             PROJECTION[\"Transverse_Mercator\"],\
             PARAMETER[\"latitude_of_origin\",0],\
             PARAMETER[\"central_meridian\",173],\
             PARAMETER[\"scale_factor\",0.9996],\
             PARAMETER[\"false_easting\",1600000],\
             PARAMETER[\"false_northing\",10000000],\
             UNIT[\"metre\",1,\
             AUTHORITY[\"EPSG\",\"9001\"]]]",
            );
            let res = projected(crs).identify_with(Some(&factory_all));
            assert_eq!(res.len(), 1);
            assert_eq!(epsg(&res[0].0), "2193");
            assert_eq!(res[0].1, 90);
        }
    );

    block!(
        "NAD_1983_HARN_StatePlane_Colorado_North_FIPS_0501 in US feet",
        {
            let crs = wkt_with_db(
                &db,
                "PROJCS[\"NAD_1983_HARN_StatePlane_Colorado_North_FIPS_0501\",\
             GEOGCS[\"GCS_North_American_1983_HARN\",\
             DATUM[\"D_North_American_1983_HARN\",SPHEROID[\"GRS_1980\",\
             6378137.0,298.257222101]],PRIMEM[\"Greenwich\",0.0],\
             UNIT[\"Degree\",0.0174532925199433]],\
             PROJECTION[\"Lambert_Conformal_Conic\"],\
             PARAMETER[\"False_Easting\",3000000.000316083],\
             PARAMETER[\"False_Northing\",999999.999996],\
             PARAMETER[\"Central_Meridian\",-105.5],\
             PARAMETER[\"Standard_Parallel_1\",39.71666666666667],\
             PARAMETER[\"Standard_Parallel_2\",40.78333333333333],\
             PARAMETER[\"Latitude_Of_Origin\",39.33333333333334],\
             UNIT[\"Foot_US\",0.3048006096012192]]",
            );
            let res = projected(crs).identify_with(Some(&factory_all));
            assert_eq!(res.len(), 1);
            assert_eq!(epsg(&res[0].0), "2876");
            assert_eq!(res[0].1, 100);
        }
    );

    block!(
        "JGD2011_Japan_Zone_2 with the ESRI alias JGD_2011_Japan_Zone_2",
        {
            let crs = wkt_with_db(
                &db,
                "PROJCS[\"JGD2011_Japan_Zone_2\",GEOGCS[\"GCS_JGD_2011\",\
             DATUM[\"D_JGD_2011\",\
             SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],\
             PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],\
             PROJECTION[\"Transverse_Mercator\"],\
             PARAMETER[\"False_Easting\",0.0],\
             PARAMETER[\"False_Northing\",0.0],\
             PARAMETER[\"Central_Meridian\",131],\
             PARAMETER[\"Scale_Factor\",0.9999],\
             PARAMETER[\"Latitude_Of_Origin\",33],UNIT[\"Meter\",1.0]]",
            );
            let res = projected(crs).identify_with(Some(&factory_epsg));
            assert_eq!(res.len(), 1);
            assert_eq!(epsg(&res[0].0), "6670");
            assert_eq!(res[0].1, 90);
        }
    );

    block!("NAD_1983_CSRS_Prince_Edward_Island with ESRI aliases", {
        let crs = wkt_with_db(
            &db,
            "PROJCS[\"NAD_1983_CSRS_Prince_Edward_Island\",\
             GEOGCS[\"GCS_North_American_1983_CSRS\",\
             DATUM[\"D_North_American_1983_CSRS\",\
             SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],\
             PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],\
             PROJECTION[\"Double_Stereographic\"],\
             PARAMETER[\"False_Easting\",400000.0],\
             PARAMETER[\"False_Northing\",800000.0],\
             PARAMETER[\"Central_Meridian\",-63.0],\
             PARAMETER[\"Scale_Factor\",0.999912],\
             PARAMETER[\"Latitude_Of_Origin\",47.25],UNIT[\"Meter\",1.0]]",
        );
        let res = projected(crs).identify_with(Some(&factory_all));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "2954");
        assert_eq!(res[0].1, 100);
    });

    block!("LCC 2SP with switched standard parallels", {
        let crs = wkt_with_db(
            &db,
            "PROJCS[\"foo\",\n\
             GEOGCS[\"RGF93\",\n\
             DATUM[\"Reseau_Geodesique_Francais_1993\",\n\
             SPHEROID[\"GRS 1980\",6378137,298.257222101]],\n\
             PRIMEM[\"Greenwich\",0],\n\
             UNIT[\"degree\",0.0174532925199433]],\n\
             PROJECTION[\"Lambert_Conformal_Conic_2SP\"],\n\
             PARAMETER[\"latitude_of_origin\",46.5],\n\
             PARAMETER[\"central_meridian\",3],\n\
             PARAMETER[\"standard_parallel_1\",44],\n\
             PARAMETER[\"standard_parallel_2\",49],\n\
             PARAMETER[\"false_easting\",700000],\n\
             PARAMETER[\"false_northing\",6600000],\n\
             UNIT[\"metre\",1]]",
        );
        let res = projected(crs).identify_with(Some(&factory_epsg));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "2154");
        assert_eq!(res[0].1, 70);
    });

    block!("LKS92_Latvia_TM", {
        let crs = wkt_with_db(
            &db,
            "PROJCS[\"LKS92_Latvia_TM\",GEOGCS[\"GCS_LKS92\",\
             DATUM[\"D_Latvia_1992\",\
             SPHEROID[\"GRS_1980\",6378137,298.257222101]],\
             PRIMEM[\"Greenwich\",0],UNIT[\"Degree\",0.017453292519943295]],\
             PROJECTION[\"Transverse_Mercator\"],\
             PARAMETER[\"latitude_of_origin\",0],\
             PARAMETER[\"central_meridian\",24],\
             PARAMETER[\"scale_factor\",0.9996],\
             PARAMETER[\"false_easting\",500000],\
             PARAMETER[\"false_northing\",-6000000],\
             UNIT[\"Meter\",1]]",
        );
        let res = projected(crs).identify_with(Some(&factory_epsg));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "3059");
        assert_eq!(res[0].1, 90);
    });

    block!("everything but the datum name matches", {
        let crs = wkt_with_db(
            &db,
            "PROJCS[\"WGS_1984_UTM_Zone_31N\",GEOGCS[\"GCS_WGS_1984\",\
             DATUM[\"wrong_datum_name\",\
             SPHEROID[\"WGS_1984\",6378137.0,298.257223563]],\
             PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],\
             PROJECTION[\"Transverse_Mercator\"],\
             PARAMETER[\"False_Easting\",500000.0],\
             PARAMETER[\"False_Northing\",0.0],\
             PARAMETER[\"Central_Meridian\",3.0],\
             PARAMETER[\"Scale_Factor\",0.9996],\
             PARAMETER[\"Latitude_Of_Origin\",0.0],\
             UNIT[\"Meter\",1.0]]",
        );
        let res = projected(crs).identify_with(Some(&factory_epsg));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "32631");
        assert_eq!(res[0].1, 60);
    });

    block!("ETRS89_LAEA_Europe through the EPSG alias", {
        let crs = wkt_with_db(
            &db,
            "PROJCS[\"ETRS89_LAEA_Europe\",\
             GEOGCS[\"GCS_ETRS_1989\",DATUM[\"D_ETRS_1989\",\
             SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],\
             PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],\
             PROJECTION[\"Lambert_Azimuthal_Equal_Area\"],\
             PARAMETER[\"false_easting\",4321000.0],\
             PARAMETER[\"false_northing\",3210000.0],\
             PARAMETER[\"central_meridian\",10.0],\
             PARAMETER[\"latitude_of_origin\",52.0],\
             UNIT[\"Meter\",1.0]]",
        );
        let res = projected(crs).identify_with(Some(&factory_all));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "3035");
        assert_eq!(res[0].1, 90);
    });

    block!("RGF93_Lambert_93 (QGIS issue 32255)", {
        let crs = wkt_with_db(
            &db,
            "PROJCS[\"RGF93_Lambert_93\",GEOGCS[\"GCS_RGF_1993\",\
             DATUM[\"D_RGF_1993\",\
             SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],\
             PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],\
             PROJECTION[\"Lambert_Conformal_Conic\"],\
             PARAMETER[\"False_Easting\",700000.0],\
             PARAMETER[\"False_Northing\",6600000.0],\
             PARAMETER[\"Central_Meridian\",3.0],\
             PARAMETER[\"Standard_Parallel_1\",44.0],\
             PARAMETER[\"Standard_Parallel_2\",49.0],\
             PARAMETER[\"Latitude_Of_Origin\",46.5],UNIT[\"Meter\",1.0]]",
        );
        let res = projected(crs).identify_with(Some(&factory_all));
        let found = res
            .iter()
            .find(|(crs, _)| epsg(crs) == "2154")
            .expect("EPSG:2154");
        assert_eq!(found.1, 90);
    });

    block!("UTM 31N with a datum ensemble", {
        let wkt = "PROJCRS[\"WGS 84 / UTM zone 31N\",\
            BASEGEOGCRS[\"WGS 84\",\
            ENSEMBLE[\"World Geodetic System 1984 ensemble\",\
            MEMBER[\"World Geodetic System 1984 (Transit)\",ID[\"EPSG\",1166]],\
            MEMBER[\"World Geodetic System 1984 (G730)\",ID[\"EPSG\",1152]],\
            MEMBER[\"World Geodetic System 1984 (G873)\",ID[\"EPSG\",1153]],\
            MEMBER[\"World Geodetic System 1984 (G1150)\",ID[\"EPSG\",1154]],\
            MEMBER[\"World Geodetic System 1984 (G1674)\",ID[\"EPSG\",1155]],\
            MEMBER[\"World Geodetic System 1984 (G1762)\",ID[\"EPSG\",1156]],\
            ELLIPSOID[\"WGS 84\",6378137,298.257223563,\
            LENGTHUNIT[\"metre\",1,ID[\"EPSG\",9001]]],\
            ENSEMBLEACCURACY[2]]],\
            CONVERSION[\"UTM zone 31N\",\
            METHOD[\"Transverse Mercator\",ID[\"EPSG\",9807]],\
            PARAMETER[\"Latitude of natural origin\",0,\
            ANGLEUNIT[\"degree\",0.0174532925199433,ID[\"EPSG\",9102]]],\
            PARAMETER[\"Longitude of natural origin\",3,\
            ANGLEUNIT[\"degree\",0.0174532925199433,ID[\"EPSG\",9102]]],\
            PARAMETER[\"Scale factor at natural origin\",0.9996,\
            SCALEUNIT[\"unity\",1,ID[\"EPSG\",9201]]],\
            PARAMETER[\"False easting\",500000,\
            LENGTHUNIT[\"metre\",1,ID[\"EPSG\",9001]]],\
            PARAMETER[\"False northing\",0,\
            LENGTHUNIT[\"metre\",1,ID[\"EPSG\",9001]]]],\
            CS[Cartesian,2],\
            AXIS[\"Easting (E)\",east,ORDER[1]],\
            AXIS[\"Northing (N)\",north,ORDER[2]],\
            LENGTHUNIT[\"metre\",1,ID[\"EPSG\",9001]]]";
        let crs = WktParser::new()
            .create_from_wkt(wkt)
            .expect("WKT")
            .crs()
            .cloned()
            .expect("a CRS");
        let res = projected(crs).identify_with(Some(&factory_all));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "32631");
        assert_eq!(res[0].1, 100);
    });

    block!("UTM 31N with a datum ensemble and an unknown name", {
        let wkt = "PROJCRS[\"unknown\",\
            BASEGEOGCRS[\"unknown\",\
            ENSEMBLE[\"World Geodetic System 1984 ensemble\",\
            MEMBER[\"World Geodetic System 1984 (Transit)\",ID[\"EPSG\",1166]],\
            MEMBER[\"World Geodetic System 1984 (G730)\",ID[\"EPSG\",1152]],\
            MEMBER[\"World Geodetic System 1984 (G873)\",ID[\"EPSG\",1153]],\
            MEMBER[\"World Geodetic System 1984 (G1150)\",ID[\"EPSG\",1154]],\
            MEMBER[\"World Geodetic System 1984 (G1674)\",ID[\"EPSG\",1155]],\
            MEMBER[\"World Geodetic System 1984 (G1762)\",ID[\"EPSG\",1156]],\
            ELLIPSOID[\"WGS 84\",6378137,298.257223563,\
            LENGTHUNIT[\"metre\",1,ID[\"EPSG\",9001]]],\
            ENSEMBLEACCURACY[2]]],\
            CONVERSION[\"UTM zone 31N\",\
            METHOD[\"Transverse Mercator\",ID[\"EPSG\",9807]],\
            PARAMETER[\"Latitude of natural origin\",0,\
            ANGLEUNIT[\"degree\",0.0174532925199433,ID[\"EPSG\",9102]]],\
            PARAMETER[\"Longitude of natural origin\",3,\
            ANGLEUNIT[\"degree\",0.0174532925199433,ID[\"EPSG\",9102]]],\
            PARAMETER[\"Scale factor at natural origin\",0.9996,\
            SCALEUNIT[\"unity\",1,ID[\"EPSG\",9201]]],\
            PARAMETER[\"False easting\",500000,\
            LENGTHUNIT[\"metre\",1,ID[\"EPSG\",9001]]],\
            PARAMETER[\"False northing\",0,\
            LENGTHUNIT[\"metre\",1,ID[\"EPSG\",9001]]]],\
            CS[Cartesian,2],\
            AXIS[\"Easting (E)\",east,ORDER[1]],\
            AXIS[\"Northing (N)\",north,ORDER[2]],\
            LENGTHUNIT[\"metre\",1,ID[\"EPSG\",9001]]]";
        let crs = WktParser::new()
            .create_from_wkt(wkt)
            .expect("WKT")
            .crs()
            .cloned()
            .expect("a CRS");
        let res = projected(crs).identify_with(Some(&factory_epsg));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "32631");
        assert!(res[0].1 >= 70);
    });

    block!("ESRI WKT where the datum name does not start with D_", {
        let wkt = "PROJCS[\"S-JTSK_[JTSK03]_Krovak_East_North\",\
            GEOGCS[\"S-JTSK_[JTSK03]\",\
            DATUM[\"S-JTSK_[JTSK03]\",\
            SPHEROID[\"Bessel_1841\",6377397.155,299.1528128]],\
            PRIMEM[\"Greenwich\",0.0],\
            UNIT[\"Degree\",0.0174532925199433]],\
            PROJECTION[\"Krovak\"],\
            PARAMETER[\"False_Easting\",0.0],\
            PARAMETER[\"False_Northing\",0.0],\
            PARAMETER[\"Pseudo_Standard_Parallel_1\",78.5],\
            PARAMETER[\"Scale_Factor\",0.9999],\
            PARAMETER[\"Azimuth\",30.2881397527778],\
            PARAMETER[\"Longitude_Of_Center\",24.8333333333333],\
            PARAMETER[\"Latitude_Of_Center\",49.5],\
            PARAMETER[\"X_Scale\",-1.0],\
            PARAMETER[\"Y_Scale\",1.0],\
            PARAMETER[\"XY_Plane_Rotation\",90.0],\
            UNIT[\"Meter\",1.0]]";
        assert_eq!(WktParser::guess_dialect(wkt), WktDialect::Wkt1Esri);
        let crs = wkt_with_db(&db, wkt);
        let res = projected(crs).identify_with(Some(&factory_all));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "8353");
        assert_eq!(res[0].1, 100);
    });

    block!(
        "ESRI WKT with an AUTHORITY node that points to another object",
        {
            let crs = wkt_with_db(
                &db,
                "PROJCS[\"ETRS_1989_UTM_Zone_32N_6Stellen\",\
             GEOGCS[\"GCS_ETRS_1989\",DATUM[\"D_ETRS_1989\",\
             SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],\
             PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],\
             PROJECTION[\"Transverse_Mercator\"],\
             PARAMETER[\"False_Easting\",500000.0],\
             PARAMETER[\"False_Northing\",0.0],\
             PARAMETER[\"Central_Meridian\",9.0],\
             PARAMETER[\"Scale_Factor\",0.9996],\
             PARAMETER[\"Latitude_Of_Origin\",0.0],\
             UNIT[\"Meter\",1.0],\
             AUTHORITY[\"ESRI\",\"102328\"]]",
            );
            let res = projected(crs).identify_with(Some(&factory_all));
            assert!(!res.is_empty());
            assert_eq!(epsg(&res[0].0), "25832");
            assert_eq!(res[0].1, 70);
        }
    );

    block!(
        "projected CRS whose base is a spherical geodetic CRS (IAU Mercury)",
        {
            let crs = wkt_with_db(
                &db,
                "PROJCRS[\"unknown\",\n\
             BASEGEODCRS[\"Mercury (2015) / Ocentric\",\n\
             DATUM[\"Mercury (2015)\",\n\
             ELLIPSOID[\"Mercury (2015)\",2440530,1075.12334801762,\n\
             LENGTHUNIT[\"metre\",1]],\n\
             ANCHOR[\"Hun Kal: 20 W.0\"]],\n\
             PRIMEM[\"Reference Meridian\",0,\n\
             ANGLEUNIT[\"degree\",0.0174532925199433]],\n\
             ID[\"IAU\",19902,2015]],\n\
             CONVERSION[\"Equirectangular, clon = 0\",\n\
             METHOD[\"Equidistant Cylindrical\",\n\
             ID[\"EPSG\",1028]],\n\
             PARAMETER[\"Latitude of 1st standard parallel\",0,\n\
             ANGLEUNIT[\"degree\",0.0174532925199433],\n\
             ID[\"EPSG\",8823]],\n\
             PARAMETER[\"Longitude of natural origin\",0,\n\
             ANGLEUNIT[\"degree\",0.0174532925199433],\n\
             ID[\"EPSG\",8802]],\n\
             PARAMETER[\"False easting\",0,\n\
             LENGTHUNIT[\"metre\",1],\n\
             ID[\"EPSG\",8806]],\n\
             PARAMETER[\"False northing\",0,\n\
             LENGTHUNIT[\"metre\",1],\n\
             ID[\"EPSG\",8807]]],\n\
             CS[Cartesian,2],\n\
             AXIS[\"(E)\",east,\n\
             ORDER[1],\n\
             LENGTHUNIT[\"metre\",1]],\n\
             AXIS[\"(N)\",north,\n\
             ORDER[2],\n\
             LENGTHUNIT[\"metre\",1]]]",
            );
            let res = projected(crs).identify_with(Some(&factory_all));
            assert!(!res.is_empty());
            assert_eq!(res[0].0.identifiers()[0].code(), "19912");
            assert_eq!(res[0].0.identifiers()[0].code_space(), "IAU_2015");
            assert_eq!(res[0].1, 90);
        }
    );

    block!(
        "ESRI WKT with a deprecated ESRI name (Korea 2000 Central Belt 2010)",
        {
            let crs = wkt_with_db(
                &db,
                "PROJCS[\"Korea_2000_Korea_Central_Belt_2010\",\
             GEOGCS[\"GCS_Korea_2000\",DATUM[\"D_Korea_2000\",\
             SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],\
             PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],\
             PROJECTION[\"Transverse_Mercator\"],\
             PARAMETER[\"False_Easting\",200000.0],\
             PARAMETER[\"False_Northing\",600000.0],\
             PARAMETER[\"Central_Meridian\",127.0],\
             PARAMETER[\"Scale_Factor\",1.0],\
             PARAMETER[\"Latitude_Of_Origin\",38.0],\
             UNIT[\"Meter\",1.0]]",
            );
            let res = projected(crs).identify_with(Some(&factory_all));
            assert!(!res.is_empty());
            assert_eq!(res[0].0.identifiers()[0].code(), "5186");
            assert_eq!(res[0].0.identifiers()[0].code_space(), "EPSG");
            assert_eq!(res[0].1, 100);
        }
    );

    block!(
        "ESRI WKT with a deprecated ESRI name (Arizona Central, US feet)",
        {
            let crs = wkt_with_db(
                &db,
                "PROJCS[\"NAD_1983_StatePlane_Arizona_Central_FIPS_0202_IntlFeet\",\
             GEOGCS[\"GCS_North_American_1983\",\
             DATUM[\"D_North_American_1983\",\
             SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],\
             PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],\
             PROJECTION[\"Transverse_Mercator\"],\
             PARAMETER[\"False_Easting\",700000.0],\
             PARAMETER[\"False_Northing\",0.0],\
             PARAMETER[\"Central_Meridian\",-111.9166666666667],\
             PARAMETER[\"Scale_Factor\",0.9999],\
             PARAMETER[\"Latitude_Of_Origin\",31.0],UNIT[\"Foot\",0.3048]]",
            );
            let res = projected(crs).identify_with(Some(&factory_all));
            assert!(!res.is_empty());
            assert_eq!(res[0].0.identifiers()[0].code(), "2223");
            assert_eq!(res[0].0.identifiers()[0].code_space(), "EPSG");
            assert_eq!(res[0].1, 70);
        }
    );

    block!(
        "ETRS89 / Portugal TM06 after the ETRS89 -> ETRS89-PRT [1995] change",
        {
            let crs = wkt_with_db(
                &db,
                "PROJCS[\"ETRS89 / Portugal TM06\",\
             GEOGCS[\"ETRS89\",\
             DATUM[\"European Terrestrial Reference System 1989\",\
             SPHEROID[\"GRS 1980\", 6378137.0, 298.257222101,\
             AUTHORITY[\"EPSG\",\"7019\"]],\
             AUTHORITY[\"EPSG\",\"6258\"]],\
             PRIMEM[\"Greenwich\", 0.0, AUTHORITY[\"EPSG\",\"8901\"]],\
             UNIT[\"degree\", 0.017453292519943295],\
             AXIS[\"Geodetic latitude\", NORTH],\
             AXIS[\"Geodetic longitude\", EAST],\
             AUTHORITY[\"EPSG\",\"4258\"]],\
             PROJECTION[\"Transverse_Mercator\",\
             AUTHORITY[\"EPSG\",\"9807\"]],\
             PARAMETER[\"central_meridian\", -8.133108333333334],\
             PARAMETER[\"latitude_of_origin\", 39.66825833333334],\
             PARAMETER[\"scale_factor\", 1.0],\
             PARAMETER[\"false_easting\", 0.0],\
             PARAMETER[\"false_northing\", 0.0],\
             UNIT[\"m\", 1.0],\
             AXIS[\"Easting\", EAST],\
             AXIS[\"Northing\", NORTH],\
             AUTHORITY[\"EPSG\",\"3763\"]]",
            );
            let res = projected(crs).identify_with(Some(&factory_epsg));
            assert!(!res.is_empty());
            assert_eq!(res[0].0.identifiers()[0].code(), "3763");
            assert_eq!(res[0].0.identifiers()[0].code_space(), "EPSG");
            assert_eq!(res[0].1, 100);
        }
    );

    block!(
        "inverse flattening far from every ellipsoid (Bessel 299.15281310608)",
        {
            let crs = wkt_with_db(
                &db,
                "PROJCRS[\"Transverse Mercator\",\n\
             BASEGEOGCRS[\"DE_DHDN (whole country, 2001) to ETRS89\",\n\
             DATUM[\"DE_DHDN (whole country, 2001) to ETRS89\",\n\
             ELLIPSOID[\"Bessel\",6377397.155,299.15281310608,\n\
             LENGTHUNIT[\"metre\",1,\n\
             ID[\"EPSG\",9001]]]],\n\
             PRIMEM[\"Greenwich\",0,\n\
             ANGLEUNIT[\"degree\",0.0174532925199433,\n\
             ID[\"EPSG\",9122]]]],\n\
             CONVERSION[\"Transverse Mercator\",\n\
             METHOD[\"Transverse Mercator\",\n\
             ID[\"EPSG\",9807]],\n\
             PARAMETER[\"Latitude of natural origin\",0,\n\
             ANGLEUNIT[\"degree\",0.0174532925199433],\n\
             ID[\"EPSG\",8801]],\n\
             PARAMETER[\"Longitude of natural origin\",12,\n\
             ANGLEUNIT[\"degree\",0.0174532925199433],\n\
             ID[\"EPSG\",8802]],\n\
             PARAMETER[\"Scale factor at natural origin\",1,\n\
             SCALEUNIT[\"unity\",1],\n\
             ID[\"EPSG\",8805]],\n\
             PARAMETER[\"False easting\",4500000,\n\
             LENGTHUNIT[\"metre\",1],\n\
             ID[\"EPSG\",8806]],\n\
             PARAMETER[\"False northing\",0,\n\
             LENGTHUNIT[\"metre\",1],\n\
             ID[\"EPSG\",8807]]],\n\
             CS[Cartesian,2],\n\
             AXIS[\"easting\",east,\n\
             ORDER[1],\n\
             LENGTHUNIT[\"meters\",1]],\n\
             AXIS[\"northing\",north,\n\
             ORDER[2],\n\
             LENGTHUNIT[\"meters\",1]]]",
            );
            let res = projected(crs).identify_with(Some(&factory_all));
            assert_eq!(res.len(), 13);
        }
    );

    assert!(
        failures.is_empty(),
        "failing blocks:\n{}",
        failures.join("\n")
    );
}
