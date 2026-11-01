//! PROJ `test/unit/test_io.cpp` checks on the CRS-level parameters of a PROJ
//! string: the linear unit (`+units`, `+to_meter`, `+vunits`), the title,
//! `+proj=geocent`, the strings PROJ refuses, and the Google Mercator
//! (`+nadgrids=@null`, EPSG 1024).
//!
//! Each function is one `TEST()` name. Expected text is copied from that test.

use proj_rs::{
    create_crs_from_proj_string, create_from_user_input, Crs, DatabaseContext, Equivalence,
    WktConvention, WktFormatter, WktLines,
};

fn proj_crs(definition: &str) -> Crs {
    create_crs_from_proj_string(definition).expect("PROJ string")
}

/// `crs->exportToWKT` with `simulCurNodeHasId()`, on one line when `lines` says so.
fn wkt2(crs: &Crs, lines: WktLines) -> String {
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2015);
    formatter.simul_cur_node_has_id();
    formatter.set_lines(lines);
    crs.export_to_wkt(&mut formatter).expect("WKT2")
}

fn contains(wkt: &str, expected: &str) {
    assert!(wkt.contains(expected), "{expected} not in {wkt}");
}

fn proj_export(crs: &Crs) -> String {
    crs.export_to_proj_string().expect("PROJ string")
}

/// `EXPECT_THROW(createFromPROJString(text), ParsingException)`.
fn refused(definition: &str) {
    assert!(
        create_crs_from_proj_string(definition).is_err(),
        "accepted: {definition}"
    );
}

#[test]
fn io_projparse_projected_units() {
    let crs = proj_crs("+proj=tmerc +x_0=0.304800609601219 +units=us-ft +type=crs");
    assert!(matches!(crs, Crs::Projected(_)));
    let wkt = wkt2(&crs, WktLines::Single);
    contains(
        &wkt,
        "PARAMETER[\"False easting\",1,LENGTHUNIT[\"US survey foot\",0.304800609601219]",
    );
    contains(
        &wkt,
        "AXIS[\"(E)\",east,ORDER[1],LENGTHUNIT[\"US survey foot\",0.304800609601219]",
    );
}

#[test]
fn io_projparse_projected_to_meter_known() {
    let crs = proj_crs("+proj=tmerc +to_meter=0.304800609601219 +type=crs");
    assert!(matches!(crs, Crs::Projected(_)));
    let wkt = wkt2(&crs, WktLines::Single);
    contains(
        &wkt,
        "PARAMETER[\"False easting\",0,LENGTHUNIT[\"US survey foot\",0.304800609601219]",
    );
    contains(
        &wkt,
        "AXIS[\"(E)\",east,ORDER[1],LENGTHUNIT[\"US survey foot\",0.304800609601219]",
    );
}

#[test]
fn io_projparse_projected_to_meter_unknown() {
    let crs = proj_crs("+proj=tmerc +to_meter=0.1234 +type=crs");
    assert!(matches!(crs, Crs::Projected(_)));
    let wkt = wkt2(&crs, WktLines::Single);
    contains(
        &wkt,
        "PARAMETER[\"False easting\",0,LENGTHUNIT[\"unknown\",0.1234]",
    );
    contains(
        &wkt,
        "AXIS[\"(E)\",east,ORDER[1],LENGTHUNIT[\"unknown\",0.1234]",
    );
}

#[test]
fn io_projparse_projected_vunits() {
    let crs = proj_crs("+proj=tmerc +vunits=ft +type=crs");
    let Crs::Projected(projected) = &crs else {
        panic!("not a projected CRS");
    };
    let axes = projected.coordinate_system().axes();
    assert_eq!(axes.len(), 3);
    assert_eq!(axes[2].unit().name(), "foot");
}

#[test]
fn io_projparse_longlat_title() {
    let crs = proj_crs(
        "+title=Ile d'Amsterdam 1963 +proj=longlat \
         +towgs84=109.7530,-528.1330,-362.2440 \
         +a=6378388.0000 +rf=297.0000000000000 +units=m \
         +no_defs +type=crs",
    );
    let Crs::Bound(bound) = &crs else {
        panic!("not a bound CRS");
    };
    let Crs::Geographic(base) = bound.source_crs() else {
        panic!("base is not geographic");
    };
    assert_eq!(base.name(), "Ile d'Amsterdam 1963");
    assert_eq!(base.datum().expect("datum").name(), "Ile d'Amsterdam 1963");
    assert_eq!(
        proj_export(&crs),
        "+proj=longlat +ellps=intl +towgs84=109.753,-528.133,-362.244,0,0,0,0 \
         +no_defs +type=crs"
    );
}

#[test]
fn io_projparse_projected_title() {
    let crs = proj_crs(
        "+title=Amsterdam 1963 +proj=tmerc \
         +towgs84=109.7530,-528.1330,-362.2440 +a=6378388.0000 \
         +rf=297.0000000000000 +lat_0=0.000000000 +lon_0=75.000000000 \
         +k_0=0.99960000 +x_0=500000.000 +y_0=10000000.000 +units=m +no_defs \
         +type=crs",
    );
    let Crs::Bound(bound) = &crs else {
        panic!("not a bound CRS");
    };
    let Crs::Projected(base) = bound.source_crs() else {
        panic!("base is not projected");
    };
    assert_eq!(base.name(), "Amsterdam 1963");
    assert_eq!(base.base_crs().name(), "unknown");
    assert_eq!(
        proj_export(&crs),
        "+proj=utm +zone=43 +south +ellps=intl \
         +towgs84=109.753,-528.133,-362.244,0,0,0,0 +units=m +no_defs +type=crs"
    );
}

#[test]
fn io_projparse_longlat_errors() {
    refused("+proj=longlat +datum=unknown +type=crs");
    refused("+proj=longlat +ellps=unknown +type=crs");
    refused("+proj=longlat +a=invalid +b=1 +type=crs");
    refused("+proj=longlat +a=1 +b=invalid +type=crs");
    refused("+proj=longlat +a=invalid +rf=1 +type=crs");
    refused("+proj=longlat +a=1 +rf=invalid +type=crs");
    refused("+proj=longlat +a=1 +f=invalid +type=crs");
    refused("+proj=longlat +R=invalid +type=crs");
    refused("+proj=longlat +b=1 +type=crs");
    refused("+proj=longlat +rf=1 +type=crs");
    refused("+proj=longlat +f=0 +type=crs");
    refused("+proj=longlat +pm=unknown +type=crs");
    refused("+proj=longlat +ellps=GRS80 +towgs84=1.2,2,3,4,5,6,invalid +type=crs");
    refused("+proj=longlat +axis=foo +type=crs");
    refused(
        "+proj=pipeline +step +proj=longlat +ellps=GRS80 +step \
         +proj=unitconvert +xy_in=rad +xy_out=foo",
    );
    refused("+proj=pipeline +step +proj=longlat +ellps=GRS80 +step +proj=axisswap");
    refused(
        "+proj=pipeline +step +proj=longlat +ellps=GRS80 +step \
         +proj=axisswap +order=0,0",
    );
    // PROJ only checks that this returns: it must not loop forever.
    let _ = create_crs_from_proj_string("+=x;proj=pipeline step proj=push +type=crs");
}

#[test]
fn io_projparse_projected_errors() {
    refused("+proj=tmerc +units=foo +type=crs");
    refused("+proj=tmerc +x_0=foo +type=crs");
    refused("+proj=tmerc +lat_0=foo +type=crs");
    // Inconsistent pm values between geogCRS and projectedCRS
    refused(
        "+type=crs +proj=pipeline +step +proj=longlat +ellps=WGS84 \
         +step +proj=tmerc +ellps=WGS84 +lat_0=0 +pm=paris",
    );
}

#[test]
fn io_projparse_errors() {
    refused("");
    refused("foo");
    refused("inv");
    refused("step");
    refused("proj=unknown +type=crs");
    refused("proj=pipeline step proj=unitconvert step proj=longlat a=invalid");
    refused("proj=pipeline step proj=pipeline");
    refused("proj=pipeline step init=epsg:4326 init=epsg:4326");
    refused("proj=\tinit= +type=crs");
}

#[test]
fn io_projparse_geocent() {
    let crs = proj_crs("+proj=geocent +ellps=WGS84 +type=crs");
    assert!(matches!(crs, Crs::Geodetic(_)));
    assert_eq!(
        wkt2(&crs, WktLines::Single),
        "GEODCRS[\"unknown\",DATUM[\"Unknown based on WGS 84 \
         ellipsoid\",ELLIPSOID[\"WGS 84\",6378137,298.257223563,LENGTHUNIT[\"metre\",1]]],\
         PRIMEM[\"Greenwich\",0,ANGLEUNIT[\"degree\",0.0174532925199433]],CS[Cartesian,3],\
         AXIS[\"(X)\",geocentricX,ORDER[1],LENGTHUNIT[\"metre\",1]],\
         AXIS[\"(Y)\",geocentricY,ORDER[2],LENGTHUNIT[\"metre\",1]],\
         AXIS[\"(Z)\",geocentricZ,ORDER[3],LENGTHUNIT[\"metre\",1]]]"
    );
}

#[test]
fn io_projparse_geocent_towgs84() {
    let crs = proj_crs("+proj=geocent +ellps=WGS84 +towgs84=1,2,3 +type=crs");
    assert!(matches!(crs, Crs::Bound(_)));
    let wkt = wkt2(&crs, WktLines::Single);
    contains(&wkt, "METHOD[\"Geocentric translations (geocentric domain)");
    contains(&wkt, "PARAMETER[\"X-axis translation\",1");
    contains(&wkt, "PARAMETER[\"Y-axis translation\",2");
    contains(&wkt, "PARAMETER[\"Z-axis translation\",3");
    assert_eq!(
        proj_export(&crs),
        "+proj=geocent +ellps=WGS84 +towgs84=1,2,3,0,0,0,0 +units=m +no_defs +type=crs"
    );
}

#[test]
fn io_projparse_non_earth_ellipsoid() {
    let input = "+proj=merc +lon_0=0 +k=1 +x_0=0 +y_0=0 +R=1 +units=m +no_defs +type=crs";
    let crs = proj_crs(input);
    assert!(matches!(crs, Crs::Projected(_)));
    assert_eq!(proj_export(&crs), input);
}

#[test]
fn io_projparse_merc_google_mercator() {
    let proj_string = "+proj=merc +a=6378137 +b=6378137 +lat_ts=0 +lon_0=0 +x_0=0 +y_0=0 \
                       +k=1 +units=m +nadgrids=@null +no_defs +type=crs";
    let crs = proj_crs(proj_string);
    assert!(matches!(crs, Crs::Projected(_)));
    let wkt = wkt2(&crs, WktLines::Single);
    contains(
        &wkt,
        "METHOD[\"Popular Visualisation Pseudo Mercator\",ID[\"EPSG\",1024]",
    );
    contains(&wkt, "DATUM[\"World Geodetic System 1984\"");
    assert_eq!(proj_export(&crs).replace(" +wktext", ""), proj_string);
}

#[test]
fn io_projparse_merc_google_mercator_non_metre_unit() {
    let proj_string = "+proj=merc +a=6378137 +b=6378137 +lat_ts=0 +lon_0=0 +x_0=0 +y_0=0 \
                       +k=1 +units=ft +nadgrids=@null +no_defs +type=crs";
    let crs = proj_crs(proj_string);
    let Crs::Projected(projected) = &crs else {
        panic!("not a projected CRS");
    };
    assert_eq!(crs.name(), "WGS 84 / Pseudo-Mercator (unit ft)");
    assert_eq!(
        projected.deriving_conversion().method().name(),
        "Popular Visualisation Pseudo Mercator"
    );
    assert_eq!(proj_export(&crs).replace(" +wktext", ""), proj_string);

    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2019);
    formatter.set_lines(WktLines::Multi);
    let wkt = crs.export_to_wkt(&mut formatter).expect("WKT2");
    let expected_wkt = "PROJCRS[\"WGS 84 / Pseudo-Mercator (unit ft)\",\n\
        \x20   BASEGEOGCRS[\"WGS 84\",\n\
        \x20       DATUM[\"World Geodetic System 1984\",\n\
        \x20           ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n\
        \x20               LENGTHUNIT[\"metre\",1]]],\n\
        \x20       PRIMEM[\"Greenwich\",0,\n\
        \x20           ANGLEUNIT[\"degree\",0.0174532925199433]],\n\
        \x20       ID[\"EPSG\",4326]],\n\
        \x20   CONVERSION[\"unknown\",\n\
        \x20       METHOD[\"Popular Visualisation Pseudo Mercator\",\n\
        \x20           ID[\"EPSG\",1024]],\n\
        \x20       PARAMETER[\"Latitude of natural origin\",0,\n\
        \x20           ANGLEUNIT[\"degree\",0.0174532925199433],\n\
        \x20           ID[\"EPSG\",8801]],\n\
        \x20       PARAMETER[\"Longitude of natural origin\",0,\n\
        \x20           ANGLEUNIT[\"degree\",0.0174532925199433],\n\
        \x20           ID[\"EPSG\",8802]],\n\
        \x20       PARAMETER[\"False easting\",0,\n\
        \x20           LENGTHUNIT[\"metre\",1],\n\
        \x20           ID[\"EPSG\",8806]],\n\
        \x20       PARAMETER[\"False northing\",0,\n\
        \x20           LENGTHUNIT[\"metre\",1],\n\
        \x20           ID[\"EPSG\",8807]]],\n\
        \x20   CS[Cartesian,2],\n\
        \x20       AXIS[\"(E)\",east,\n\
        \x20           ORDER[1],\n\
        \x20           LENGTHUNIT[\"foot\",0.3048,\n\
        \x20               ID[\"EPSG\",9002]]],\n\
        \x20       AXIS[\"(N)\",north,\n\
        \x20           ORDER[2],\n\
        \x20           LENGTHUNIT[\"foot\",0.3048,\n\
        \x20               ID[\"EPSG\",9002]]]]";
    assert_eq!(wkt, expected_wkt);

    let db = DatabaseContext::create().expect("proj.db");
    let from_wkt = create_from_user_input(&wkt, &db).expect("WKT2 of the CRS");
    let Crs::Projected(from_projected) = &from_wkt else {
        panic!("WKT2 is not a projected CRS");
    };
    assert_eq!(from_projected.name(), "WGS 84 / Pseudo-Mercator (unit ft)");
    assert!(projected.is_equivalent_to_with(from_projected, Equivalence::Equivalent));
}

#[test]
fn io_projparse_merc_not_quite_google_mercator() {
    let proj_string = "+proj=merc +a=6378137 +b=6378137 +lat_ts=0 +lon_0=10 +x_0=0 +y_0=0 \
                       +k=1 +units=m +nadgrids=@null +no_defs +type=crs";
    let crs = proj_crs(proj_string);
    assert!(matches!(crs, Crs::Projected(_)));
    let wkt = wkt2(&crs, WktLines::Single);
    contains(
        &wkt,
        "METHOD[\"Popular Visualisation Pseudo Mercator\",ID[\"EPSG\",1024]",
    );
    contains(&wkt, "DATUM[\"unknown using nadgrids=@null\",");
    assert_eq!(proj_export(&crs).replace(" +wktext", ""), proj_string);
}

#[test]
fn io_projparse_longlat_wktext() {
    let crs = proj_crs("+proj=longlat +foo +wktext +type=crs");
    assert!(matches!(crs, Crs::Geographic(_)));
    assert_eq!(
        proj_export(&crs),
        "+proj=longlat +datum=WGS84 +no_defs +type=crs"
    );
}

#[test]
fn io_projparse_geocent_wktext() {
    let crs = proj_crs("+proj=geocent +foo +wktext +type=crs");
    assert!(matches!(crs, Crs::Geodetic(_)));
    assert_eq!(
        proj_export(&crs),
        "+proj=geocent +datum=WGS84 +units=m +no_defs +type=crs"
    );
}

#[test]
fn io_projparse_projected_wktext() {
    let crs = proj_crs("+proj=merc +foo +wktext +type=crs");
    assert!(matches!(crs, Crs::Projected(_)));
    assert_eq!(
        proj_export(&crs),
        "+proj=merc +lon_0=0 +k=1 +x_0=0 +y_0=0 +datum=WGS84 +units=m +no_defs +type=crs"
    );
}
