//! PROJ 9.9.0 values for the modified stereographic family (`src/projections/mod_ster.cpp`).
//!
//! Forward values are the `proj -f %.12f` output of the reference build; each inverse is the
//! reference `proj -I` output. Points are the parity corpus inputs (`tests/parity/cases.jsonl`).

use proj_rs::{Operation, Xyzt};

/// Tolerance for a projected value: 1 micrometre, or 1e-11 relative for large values.
fn assert_metres(got: f64, want: f64, what: &str) {
    let tol = 1e-6 + 1e-11 * want.abs();
    assert!(
        (got - want).abs() <= tol,
        "{what}: got {got}, want {want}, tolerance {tol}"
    );
}

/// Tolerance for an angle in degrees: 1e-9 degree.
fn assert_degrees(got: f64, want: f64, what: &str) {
    assert!(
        (got - want).abs() <= 1e-9,
        "{what}: got {got}, want {want} degrees"
    );
}

fn forward(definition: &str, lon_deg: f64, lat_deg: f64) -> Xyzt {
    Operation::create(definition)
        .expect("create")
        .forward(Xyzt::xy(lon_deg.to_radians(), lat_deg.to_radians()))
        .expect("forward")
}

fn inverse(definition: &str, x: f64, y: f64) -> Xyzt {
    Operation::create(definition)
        .expect("create")
        .inverse(Xyzt::xy(x, y))
        .expect("inverse")
}

#[test]
fn mil_os_forward_and_inverse_match_proj_9_9_0() {
    let definition = "+proj=mil_os +ellps=WGS84";
    let forward_points = [
        (
            15.613033565,
            -47.434144328,
            -387564.77709508885,
            -7295264.787471141,
        ),
        (
            -14.951904487,
            -45.345574306,
            -3237381.6565967286,
            -7473861.957187959,
        ),
        (
            -14.880519927,
            42.964183262,
            -2743583.31867897,
            3067288.950113749,
        ),
        (
            -126.674335762,
            58.080274043,
            -2837463.440593951,
            12243696.00691584,
        ),
        (
            -88.392414844,
            -49.159453424,
            -9911510.7816863,
            -16101729.368951939,
        ),
        (
            69.541522039,
            15.442790898,
            5241739.249592235,
            429726.6421308701,
        ),
    ];
    for (lon, lat, x, y) in forward_points {
        let got = forward(definition, lon, lat);
        assert_metres(got.x, x, "x");
        assert_metres(got.y, y, "y");
    }
    let inverse_points = [
        (
            -387564.77709508885,
            -7295264.787471141,
            15.613033565,
            -47.434144328,
        ),
        (
            -3237381.6565967286,
            -7473861.957187959,
            -14.951904487,
            -45.345574306,
        ),
        (
            -2743583.31867897,
            3067288.950113749,
            -14.880519927,
            42.964183262,
        ),
        (
            -2837463.440593951,
            12243696.00691584,
            -126.674335762,
            58.080274043,
        ),
        (
            -9911510.7816863,
            -16101729.368951939,
            -88.392414844,
            -49.159453424,
        ),
        (
            5241739.249592235,
            429726.6421308701,
            69.541522039,
            15.442790898,
        ),
    ];
    for (x, y, lon, lat) in inverse_points {
        let got = inverse(definition, x, y);
        assert_degrees(got.x.to_degrees(), lon, "lon");
        assert_degrees(got.y.to_degrees(), lat, "lat");
    }
}

#[test]
fn lee_os_forward_and_inverse_match_proj_9_9_0() {
    let definition = "+proj=lee_os +ellps=WGS84";
    let forward_points = [
        (
            -14.724623175,
            -23.002781104,
            18098461.017427757,
            -9360884.402323868,
        ),
        (
            -24.713640967,
            -35.743117097,
            11519725.293503592,
            -10388912.732141634,
        ),
        (
            -27.613384988,
            -26.591454310,
            13163558.389313884,
            -8019551.34900214,
        ),
        (
            86.579188789,
            -19.465838292,
            -11050042.562162813,
            -3454746.4200848998,
        ),
        (
            107.236418273,
            18.848335538,
            -8337723.479261216,
            2814417.9712478253,
        ),
        (
            12.405543093,
            -62.783333714,
            1137434.4869087518,
            -13587907.7790876,
        ),
    ];
    for (lon, lat, x, y) in forward_points {
        let got = forward(definition, lon, lat);
        assert_metres(got.x, x, "x");
        assert_metres(got.y, y, "y");
    }
    let inverse_points = [
        (
            18098461.017427757,
            -9360884.402323868,
            -14.724623175,
            -23.002781104,
        ),
        (
            11519725.293503592,
            -10388912.732141634,
            -24.713640967,
            -35.743117097,
        ),
        (
            13163558.389313884,
            -8019551.34900214,
            -27.613384988,
            -26.59145431,
        ),
        (
            -11050042.562162813,
            -3454746.4200848998,
            86.579188789,
            -19.465838292,
        ),
        (
            -8337723.479261216,
            2814417.9712478253,
            107.236418273,
            18.848335538,
        ),
        (
            1137434.4869087518,
            -13587907.7790876,
            12.405543093,
            -62.783333714,
        ),
    ];
    for (x, y, lon, lat) in inverse_points {
        let got = inverse(definition, x, y);
        assert_degrees(got.x.to_degrees(), lon, "lon");
        assert_degrees(got.y.to_degrees(), lat, "lat");
    }
}

#[test]
fn gs48_forward_and_inverse_match_proj_9_9_0() {
    let definition = "+proj=gs48 +ellps=WGS84";
    let forward_points = [
        (
            17.533443259,
            -34.114234426,
            79177334.77975246,
            -514306489.2132275,
        ),
        (
            -16.504148709,
            36.039741232,
            6027838.9566362575,
            3497730.9826107775,
        ),
        (
            15.526352587,
            49.651168162,
            5875454.626060571,
            3458734.018780878,
        ),
        (
            -83.613572161,
            -73.484848675,
            41493769.16195077,
            -138871077.44807696,
        ),
        (
            -116.706515167,
            47.010214092,
            -1558972.1011840147,
            1046962.5196096986,
        ),
        (
            -7.785714123,
            73.663413026,
            2916385.8731926167,
            5817242.081887147,
        ),
    ];
    for (lon, lat, x, y) in forward_points {
        let got = forward(definition, lon, lat);
        assert_metres(got.x, x, "x");
        assert_metres(got.y, y, "y");
    }
    let inverse_points = [
        (
            6478645.603178534,
            -16715581.005374622,
            -85.191741800237,
            -40.665913753095,
        ),
        (
            7116452.619126545,
            872390.0148178161,
            -29.84788499256,
            23.843208184236,
        ),
        (
            6679730.157305005,
            3030326.835393941,
            -19.599852257733,
            31.676237427947,
        ),
        (
            -76950952.92332587,
            64464132.31011996,
            29.51607660448,
            6.462145490838,
        ),
        (
            -183624.83565927175,
            -8943660.544864757,
            -96.694851790018,
            -21.755412907447,
        ),
    ];
    for (x, y, lon, lat) in inverse_points {
        let got = inverse(definition, x, y);
        assert_degrees(got.x.to_degrees(), lon, "lon");
        assert_degrees(got.y.to_degrees(), lat, "lat");
    }
}

#[test]
fn alsk_forward_and_inverse_match_proj_9_9_0() {
    let definition = "+proj=alsk +ellps=WGS84";
    let forward_points = [
        (
            21.445286392,
            48.091663163,
            -3500439.3100842256,
            21319702.812195525,
        ),
        (
            14.382573633,
            46.532475949,
            5917116.710544554,
            23447165.757043865,
        ),
        (
            0.133988951,
            -38.969599594,
            -403896104763.16345,
            -314692647692.1299,
        ),
        (
            56.786228500,
            -6.223405849,
            3779570203.386713,
            -1469089383.5834115,
        ),
        (
            -117.918585694,
            -65.812989493,
            -7063156073.712292,
            -23977158044.51613,
        ),
        (
            76.626755636,
            4.117776993,
            81987090.83025995,
            703142290.2929976,
        ),
    ];
    for (lon, lat, x, y) in forward_points {
        let got = forward(definition, lon, lat);
        assert_metres(got.x, x, "x");
        assert_metres(got.y, y, "y");
    }
    let inverse_points = [
        (
            -3500439.3100842256,
            21319702.812195525,
            21.445276401665,
            48.091533676432,
        ),
        (
            5917116.710544554,
            23447165.757043865,
            14.38259919644,
            46.532349473385,
        ),
    ];
    for (x, y, lon, lat) in inverse_points {
        let got = inverse(definition, x, y);
        assert_degrees(got.x.to_degrees(), lon, "lon");
        assert_degrees(got.y.to_degrees(), lat, "lat");
    }
}

#[test]
fn gs50_forward_and_inverse_match_proj_9_9_0() {
    let definition = "+proj=gs50 +ellps=WGS84";
    let forward_points = [
        (
            -6.761967269,
            -8.221634812,
            -26889703373.65039,
            -19532652860.93059,
        ),
        (
            10.240595960,
            32.620163482,
            -340304389.2447726,
            -897308214.8156712,
        ),
        (
            -6.964807799,
            8.087578844,
            3512076166.7391386,
            -1704126061.6659565,
        ),
        (
            36.169181299,
            -52.442590258,
            -5.1600252015847464e+16,
            6.9828643491669576e+16,
        ),
        (
            48.222590270,
            59.202824419,
            -101060132.73753963,
            -5530747.294570852,
        ),
        (
            -145.867686514,
            -63.547695309,
            -23943378616.870907,
            -2288002354.108588,
        ),
    ];
    for (lon, lat, x, y) in forward_points {
        let got = forward(definition, lon, lat);
        assert_metres(got.x, x, "x");
        assert_metres(got.y, y, "y");
    }
    let inverse_points = [
        (0.0, 0.0, -120.0, 45.0),
        (
            1672352.4021247425,
            -347454.74772436864,
            -99.999800675552,
            39.999927827988,
        ),
        (
            -641128.7280539884,
            1131469.9335915945,
            -130.000131580761,
            55.000104066612,
        ),
        (
            -1478184.8218436693,
            -1504439.4183410546,
            -135.000124874687,
            29.999827465714,
        ),
        (
            553939.832203789,
            1714721.7840343388,
            -109.999848679077,
            60.000151089142,
        ),
    ];
    for (x, y, lon, lat) in inverse_points {
        let got = inverse(definition, x, y);
        assert_degrees(got.x.to_degrees(), lon, "lon");
        assert_degrees(got.y.to_degrees(), lat, "lat");
    }
}
