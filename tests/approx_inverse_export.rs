//! The inverse of a Helmert transformation written as `(approx. inversion)` in WKT2:2019:
//! PROJ 9.9.0 `createApproximateInverseIfPossible`, called by
//! `InverseTransformation::_exportToWKT`. The expected blocks are the text of
//! `projinfo -k operation EPSG:<code> -o WKT2:2019 -q` in PROJ 9.9.0. The approximate
//! step has no `VERSION`, keeps its EPSG identifiers and operation accuracy, and takes the
//! parameters in the units PROJ builds them with (metre, arc-second, parts per million).

use proj_rs::iso19111::{AlternativeGridNames, AuthorityFactory, DatabaseContext};
use proj_rs::{WktConvention, WktFormatter};

fn export(code: &str) -> String {
    let db = DatabaseContext::create().expect("proj.db");
    AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_operation(code, AlternativeGridNames::Official)
        .expect(code)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("WKT2:2019")
}

/// `EPSG:10755`: a full-matrix geographic 3D rotation, inverted, so its name, method and
/// parameters are those of the forward method with every value negated.
#[test]
fn full_matrix_inverse_is_approximate() {
    let wkt = export("10755");
    assert!(
        wkt.contains(concat!(
            "        COORDINATEOPERATION[\"Inverse of Saba to BES2020 Saba (1) (approx. inversion)\",\n",
            "            SOURCECRS[\n",
        )),
        "{wkt}"
    );
    assert!(
        wkt.contains(
            r#"            METHOD["Coordinate Frame rotation full matrix (geog3D)",
                ID["EPSG",1140]],
            PARAMETER["X-axis translation",-1138.7432,
                LENGTHUNIT["metre",1],
                ID["EPSG",8605]],
            PARAMETER["Y-axis translation",2064.4761,
                LENGTHUNIT["metre",1],
                ID["EPSG",8606]],
            PARAMETER["Z-axis translation",-110.7016,
                LENGTHUNIT["metre",1],
                ID["EPSG",8607]],
            PARAMETER["X-axis rotation",214.615206,
                ANGLEUNIT["arc-second",4.84813681109536E-06],
                ID["EPSG",8608]],
            PARAMETER["Y-axis rotation",-479.360036,
                ANGLEUNIT["arc-second",4.84813681109536E-06],
                ID["EPSG",8609]],
            PARAMETER["Z-axis rotation",164.703951,
                ANGLEUNIT["arc-second",4.84813681109536E-06],
                ID["EPSG",8610]],
            PARAMETER["Scale difference",402.32073,
                SCALEUNIT["parts per million",1E-06],
                ID["EPSG",8611]],
            OPERATIONACCURACY[0.05],
            ID["INVERSE(EPSG)",10646]]],"#
        ),
        "{wkt}"
    );
}

/// `EPSG:11285`: the inverse of a time-dependent position vector with rates. Its parameters
/// are rebuilt in PROJ's units, so the millimetre and milliarc-second EPSG values come out in
/// metres and arc-seconds, and the reference epoch stays. The plain inverse of the geocentric
/// translation step after it carries no `VERSION`.
#[test]
fn time_dependent_position_vector_inverse_is_approximate_in_si_units() {
    let wkt = export("11285");
    assert!(
        wkt.contains(concat!(
            "        COORDINATEOPERATION[\"Inverse of ETRF97 to ETRF2000 (2) (approx. inversion)\",\n",
            "            SOURCECRS[\n",
        )),
        "{wkt}"
    );
    assert!(
        wkt.contains(concat!(
            "        COORDINATEOPERATION[\"Inverse of ETRS89-MDA [MOLDREF99] to ETRF97 (1)\",\n",
            "            SOURCECRS[\n",
        )),
        "{wkt}"
    );
    assert!(
        wkt.contains(
            r#"            METHOD["Time-dependent Position Vector tfm (geocentric)",
                ID["EPSG",1053]],
            PARAMETER["X-axis translation",-0.0063,
                LENGTHUNIT["metre",1],
                ID["EPSG",8605]],
            PARAMETER["Y-axis translation",-0.0057,
                LENGTHUNIT["metre",1],
                ID["EPSG",8606]],
            PARAMETER["Z-axis translation",-0.0237,
                LENGTHUNIT["metre",1],
                ID["EPSG",8607]],
            PARAMETER["X-axis rotation",0.001309,
                ANGLEUNIT["arc-second",4.84813681109536E-06],
                ID["EPSG",8608]],
            PARAMETER["Y-axis rotation",0.00011,
                ANGLEUNIT["arc-second",4.84813681109536E-06],
                ID["EPSG",8609]],
            PARAMETER["Z-axis rotation",0.001622,
                ANGLEUNIT["arc-second",4.84813681109536E-06],
                ID["EPSG",8610]],
            PARAMETER["Scale difference",0.00158,
                SCALEUNIT["parts per million",1E-06],
                ID["EPSG",8611]],
            PARAMETER["Rate of change of X-axis translation",0,
                LENGTHUNIT["metres per year",3.16887651727315E-08],
                ID["EPSG",1040]],
            PARAMETER["Rate of change of Y-axis translation",-0.0006,
                LENGTHUNIT["metres per year",3.16887651727315E-08],
                ID["EPSG",1041]],
            PARAMETER["Rate of change of Z-axis translation",-0.0014,
                LENGTHUNIT["metres per year",3.16887651727315E-08],
                ID["EPSG",1042]],
            PARAMETER["Rate of change of X-axis rotation",0.000119,
                ANGLEUNIT["arc-seconds per year",1.53631468932076E-13],
                ID["EPSG",1043]],
            PARAMETER["Rate of change of Y-axis rotation",1E-05,
                ANGLEUNIT["arc-seconds per year",1.53631468932076E-13],
                ID["EPSG",1044]],
            PARAMETER["Rate of change of Z-axis rotation",0.000162,
                ANGLEUNIT["arc-seconds per year",1.53631468932076E-13],
                ID["EPSG",1045]],
            PARAMETER["Rate of change of Scale difference",1E-05,
                SCALEUNIT["parts per million per year",3.16887651727315E-14],
                ID["EPSG",1046]],
            PARAMETER["Parameter reference epoch",2000,
                TIMEUNIT["year",31556925.445],
                ID["EPSG",1047]],
            OPERATIONACCURACY[0.01],
            ID["INVERSE(EPSG)",11228]]],"#
        ),
        "{wkt}"
    );
}
