//! `test/unit/test_defmodel.cpp`: `defmodel.basic`, `defmodel.full`,
//! `defmodel.error_cases` and `defmodel.ISO8601ToDecimalYear`, through
//! `proj_rs::defmodel`. Each `EXPECT_THROW` is an `Err(Parsing)`, each
//! `EXPECT_EQ` an `assert_eq!`. The expected values are PROJ 9.9.0's.

#[path = "common/defmodel_json.rs"]
mod defmodel_json;

use defmodel_json::{
    arr, clear, full_valid, min_valid, n, remove, s, set, Json, IDX_CONSTANT, IDX_EXPONENTIAL,
    IDX_PIECEWISE, IDX_REVERSE_STEP, IDX_STEP, IDX_VELOCITY, MODEL_MAX_X, MODEL_MAX_Y, MODEL_MIN_X,
    MODEL_MIN_Y,
};
use proj_rs::defmodel::{
    decimal_year_from_iso8601, Bound, DefmodelError, Displacement, MasterFile, TimeFunction,
};

fn is_parsing_error(j: &Json) -> bool {
    matches!(MasterFile::parse(&j.dump()), Err(DefmodelError::Parsing(_)))
}

fn is_parsing_error_text(text: &str) -> bool {
    matches!(MasterFile::parse(text), Err(DefmodelError::Parsing(_)))
}

#[test]
fn defmodel_basic() {
    assert!(is_parsing_error_text("foo"));
    assert!(is_parsing_error_text("null"));
    assert!(is_parsing_error_text("{}"));

    let j_min_valid = min_valid();
    {
        let mf = MasterFile::parse(&j_min_valid.dump()).expect("valid");
        assert_eq!(mf.file_type(), "GeoTIFF");
        assert_eq!(mf.format_version(), "1.0");
        assert_eq!(mf.source_crs(), "EPSG:4959");
        assert_eq!(mf.target_crs(), "EPSG:7907");
        assert_eq!(mf.definition_crs(), "EPSG:4959");
        assert_eq!(mf.extent().minx(), MODEL_MIN_X);
        assert_eq!(mf.extent().miny(), MODEL_MIN_Y);
        assert_eq!(mf.extent().maxx(), MODEL_MAX_X);
        assert_eq!(mf.extent().maxy(), MODEL_MAX_Y);
        assert_eq!(mf.time_extent().first().as_str(), "1900-01-01T00:00:00Z");
        assert_eq!(mf.time_extent().last().as_str(), "2050-01-01T00:00:00Z");
    }

    // Removing one of each required key causes an exception.
    for key in j_min_valid.keys() {
        let mut jcopy = j_min_valid.clone();
        remove(&mut jcopy, &[&key]);
        assert!(is_parsing_error(&jcopy), "removing {key} must fail");
    }

    {
        let mut jcopy = j_min_valid.clone();
        set(&mut jcopy, &["definition_crs"], s("EPSG:4326"));
        assert!(is_parsing_error(&jcopy));
    }
    {
        let mut jcopy = j_min_valid.clone();
        set(&mut jcopy, &["file_type"], n(1.0));
        assert!(is_parsing_error(&jcopy));
    }
    {
        let mut jcopy = j_min_valid.clone();
        remove(&mut jcopy, &["extent", "type"]);
        assert!(is_parsing_error(&jcopy));
    }
    {
        let mut jcopy = j_min_valid.clone();
        remove(&mut jcopy, &["extent", "parameters"]);
        assert!(is_parsing_error(&jcopy));
    }
    {
        let mut jcopy = j_min_valid.clone();
        clear(&mut jcopy, &["extent", "parameters"]);
        assert!(is_parsing_error(&jcopy));
    }
    {
        let mut jcopy = j_min_valid.clone();
        remove(&mut jcopy, &["extent", "parameters", "bbox"]);
        assert!(is_parsing_error(&jcopy));
    }
    {
        let mut jcopy = j_min_valid.clone();
        set(&mut jcopy, &["extent", "parameters", "bbox"], s("foo"));
        assert!(is_parsing_error(&jcopy));
    }
    {
        let mut jcopy = j_min_valid.clone();
        set(
            &mut jcopy,
            &["extent", "parameters", "bbox"],
            arr(vec![n(0.0), n(1.0), n(2.0)]),
        );
        assert!(is_parsing_error(&jcopy));
    }
    {
        let mut jcopy = j_min_valid.clone();
        set(
            &mut jcopy,
            &["extent", "parameters", "bbox"],
            arr(vec![n(0.0), n(1.0), n(2.0), s("foo")]),
        );
        assert!(is_parsing_error(&jcopy));
    }
    {
        let mut jcopy = j_min_valid.clone();
        set(&mut jcopy, &["time_extent"], s("foo"));
        assert!(is_parsing_error(&jcopy));
    }
    {
        let mut jcopy = j_min_valid.clone();
        remove(&mut jcopy, &["time_extent", "first"]);
        assert!(is_parsing_error(&jcopy));
    }
    {
        let mut jcopy = j_min_valid.clone();
        remove(&mut jcopy, &["time_extent", "last"]);
        assert!(is_parsing_error(&jcopy));
    }
}

#[test]
fn defmodel_full() {
    let mf = MasterFile::parse(&full_valid().dump()).expect("valid");
    assert_eq!(mf.name(), "name");
    assert_eq!(mf.version(), "version");
    assert_eq!(mf.publication_date(), "2018-07-01T00:00:00Z");
    assert_eq!(mf.license(), "license");
    assert_eq!(mf.description(), "description");
    assert_eq!(mf.authority().name, "authority_name");
    assert_eq!(mf.authority().url, "authority_url");
    assert_eq!(mf.authority().address, "authority_address");
    assert_eq!(mf.authority().email, "authority_email");
    assert_eq!(mf.links().len(), 1);
    assert_eq!(mf.links()[0].href, "href");
    assert_eq!(mf.links()[0].rel, "rel");
    assert_eq!(mf.links()[0].type_, "type");
    assert_eq!(mf.links()[0].title, "title");
    assert_eq!(mf.reference_epoch(), "2000-01-01T00:00:00Z");
    assert_eq!(mf.uncertainty_reference_epoch(), "2018-12-15T00:00:00Z");
    assert_eq!(mf.horizontal_offset_unit(), "metre");
    assert_eq!(mf.vertical_offset_unit(), "metre");
    assert_eq!(
        mf.horizontal_uncertainty_type(),
        "circular 95% confidence limit"
    );
    assert_eq!(mf.horizontal_uncertainty_unit(), "metre");
    assert_eq!(mf.vertical_uncertainty_type(), "95% confidence limit");
    assert_eq!(mf.vertical_uncertainty_unit(), "metre");
    assert_eq!(mf.horizontal_offset_method(), "addition");
    let components = mf.components();
    assert_eq!(components.len(), 6);
    {
        let comp = &components[IDX_CONSTANT];
        assert_eq!(comp.description(), "description");
        assert_eq!(comp.displacement_type(), "horizontal");
        assert_eq!(comp.displacement(), Displacement::Horizontal);
        assert_eq!(comp.uncertainty_type(), "none");
        assert_eq!(comp.horizontal_uncertainty(), 0.01);
        assert_eq!(comp.vertical_uncertainty(), 0.02);
        assert_eq!(comp.extent().minx(), MODEL_MIN_X);
        assert_eq!(comp.extent().miny(), MODEL_MIN_Y);
        assert_eq!(comp.extent().maxx(), MODEL_MAX_X);
        assert_eq!(comp.extent().maxy(), MODEL_MAX_Y);
        assert_eq!(comp.spatial_model().model_type(), "GeoTIFF");
        assert_eq!(comp.spatial_model().interpolation_method(), "bilinear");
        assert_eq!(comp.spatial_model().filename(), "nzgd2000-ndm-grid02.tif");
        assert_eq!(
            comp.spatial_model().md5_checksum(),
            "49fce8ab267be2c8d00d43683060a032"
        );
        assert_eq!(comp.time_function().type_name(), "constant");
    }
    {
        let comp = &components[IDX_VELOCITY];
        assert_eq!(comp.time_function().type_name(), "velocity");
        let TimeFunction::Velocity { reference_epoch } = comp.time_function() else {
            panic!("velocity expected");
        };
        assert_eq!(reference_epoch.as_str(), "2000-01-01T00:00:00Z");
    }
    {
        let comp = &components[IDX_STEP];
        assert_eq!(comp.time_function().type_name(), "step");
        let TimeFunction::Step { step_epoch } = comp.time_function() else {
            panic!("step expected");
        };
        assert_eq!(step_epoch.as_str(), "2000-01-01T00:00:00Z");
    }
    {
        let comp = &components[IDX_REVERSE_STEP];
        assert_eq!(comp.time_function().type_name(), "reverse_step");
        let TimeFunction::ReverseStep { step_epoch } = comp.time_function() else {
            panic!("reverse_step expected");
        };
        assert_eq!(step_epoch.as_str(), "2000-01-01T00:00:00Z");
    }
    {
        let comp = &components[IDX_PIECEWISE];
        assert_eq!(comp.time_function().type_name(), "piecewise");
        let TimeFunction::Piecewise {
            before_first,
            after_last,
            model,
        } = comp.time_function()
        else {
            panic!("piecewise expected");
        };
        assert_eq!(*before_first, Bound::Zero);
        assert_eq!(*after_last, Bound::Constant);
        assert_eq!(model.len(), 4);
        assert_eq!(model[0].epoch().as_str(), "2016-01-01T00:00:00Z");
        assert_eq!(model[0].scale_factor(), 0.5);
    }
    {
        let comp = &components[IDX_EXPONENTIAL];
        assert_eq!(comp.time_function().type_name(), "exponential");
        let TimeFunction::Exponential {
            reference_epoch,
            end_epoch,
            relaxation_constant,
            before_scale_factor,
            initial_scale_factor,
            final_scale_factor,
        } = comp.time_function()
        else {
            panic!("exponential expected");
        };
        assert_eq!(reference_epoch.as_str(), "2000-01-01T00:00:00Z");
        assert_eq!(
            end_epoch.as_ref().map(|epoch| epoch.as_str()),
            Some("2001-01-01T00:00:00Z")
        );
        assert_eq!(*relaxation_constant, 2.0);
        assert_eq!(*before_scale_factor, 0.0);
        assert_eq!(*initial_scale_factor, 1.0);
        assert_eq!(*final_scale_factor, 3.0);
    }
}

#[test]
fn defmodel_error_cases() {
    let j_full_valid = full_valid();
    let rejects = |edit: &dyn Fn(&mut Json)| {
        let mut jcopy = j_full_valid.clone();
        edit(&mut jcopy);
        is_parsing_error(&jcopy)
    };
    let accepts = |edit: &dyn Fn(&mut Json)| {
        let mut jcopy = j_full_valid.clone();
        edit(&mut jcopy);
        MasterFile::parse(&jcopy.dump()).is_ok()
    };
    assert!(rejects(&|j| set(
        j,
        &["horizontal_offset_method"],
        s("unsupported")
    )));
    assert!(rejects(&|j| set(
        j,
        &["horizontal_offset_unit"],
        s("unsupported")
    )));
    assert!(rejects(&|j| set(
        j,
        &["vertical_offset_unit"],
        s("unsupported")
    )));
    assert!(rejects(&|j| set(
        j,
        &["components", "0", "spatial_model", "interpolation_method"],
        s("unsupported")
    )));
    assert!(rejects(&|j| set(
        j,
        &["components", "0", "displacement_type"],
        s("unsupported")
    )));
    assert!(rejects(&|j| set(
        j,
        &["components", "4", "time_function", "parameters", "model"],
        s("foo")
    )));
    assert!(rejects(&|j| set(
        j,
        &[
            "components",
            "4",
            "time_function",
            "parameters",
            "before_first"
        ],
        s("illegal")
    )));
    assert!(rejects(&|j| set(
        j,
        &[
            "components",
            "4",
            "time_function",
            "parameters",
            "after_last"
        ],
        s("illegal")
    )));
    assert!(rejects(&|j| set(
        j,
        &["components", "0", "time_function", "type"],
        s("unknown")
    )));

    // Unsupported combination: only the degree unit with the geocentric method
    // is refused by the parser.
    assert!(accepts(&|j| set(
        j,
        &["horizontal_offset_method"],
        s("geocentric")
    )));
    assert!(accepts(&|j| set(
        j,
        &["horizontal_offset_unit"],
        s("degree")
    )));
    assert!(rejects(&|j| {
        set(j, &["horizontal_offset_method"], s("geocentric"));
        set(j, &["horizontal_offset_unit"], s("degree"));
    }));

    // Unsupported combination: a geocentric bilinear interpolation with the
    // degree unit.
    assert!(accepts(&|j| set(
        j,
        &["components", "1", "spatial_model", "interpolation_method"],
        s("geocentric_bilinear")
    )));
    assert!(accepts(&|j| set(
        j,
        &["horizontal_offset_unit"],
        s("degree")
    )));
    assert!(rejects(&|j| {
        set(
            j,
            &["components", "1", "spatial_model", "interpolation_method"],
            s("geocentric_bilinear"),
        );
        set(j, &["horizontal_offset_unit"], s("degree"));
    }));
}

#[test]
fn defmodel_iso8601_to_decimal_year() {
    let year = |text: &str| decimal_year_from_iso8601(text).expect("valid date");
    assert_eq!(year("2000-01-01T00:00:00Z"), 2000.0);
    assert_eq!(
        year("2000-02-29T12:00:00Z"),
        2000.0 + ((31.0 + 28.0) * 86400.0 + 12.0 * 3600.0) / (366.0 * 86400.0)
    );
    assert_eq!(
        year("2000-12-31T23:59:59Z"),
        2000.0 + (366.0 * 86400.0 - 1.0) / (366.0 * 86400.0)
    );
    assert_eq!(year("2001-01-01T00:00:00Z"), 2001.0);
    assert_eq!(
        year("2001-12-31T23:59:59Z"),
        2001.0 + (365.0 * 86400.0 - 1.0) / (365.0 * 86400.0)
    );
    for invalid in [
        "",
        "0000-01-01T00:00:00Z",
        "2001-02-29T00:00:00Z",
        "2000-13-01T00:00:00Z",
        "2000-01-32T00:00:00Z",
        "2000-01-01T24:00:00Z",
        "2000-01-01T00:60:00Z",
        "2000-01-01T00:00:61Z",
    ] {
        assert!(
            decimal_year_from_iso8601(invalid).is_err(),
            "{invalid} must be rejected"
        );
    }
}
