//! `+init=file:section` of a dictionary with several sections, as `data/ITRF2008` is. PROJ 9.9.0's
//! `get_init_string` (`src/init.cpp`) ends a section at the first `<` after its header, which
//! starts the next section, and drops the `#` comments of each line (`pj_chomp`). pyproj builds
//! `+init=ITRF2008:ITRF2000` this way. Expected values: `cct -d 6` of PROJ 9.9.0 on the same file.

use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use proj_rs::ProjBuilder;

const DICTIONARY: &str = "# A dictionary of several sections, as data/ITRF2008 is
<metadata> +version=1.0.0 +origin=test

<FIRST> +proj=helmert +x=1 +y=2 +z=3 # a comment after the parameters

<SECOND> +proj=helmert +x=-0.0019 +y=-0.0017 +z=-0.0105 +s=0.00134  +dx=0.0001 +dy=0.0001 +dz=-0.0018 +ds=0.00008  +t_epoch=2000.0 +convention=position_vector

<LAST> +proj=helmert +x=10
  +y=20 # the section goes on, and ends with the file
";

const INPUT: (f64, f64, f64, f64) = (4_000_000.0, 100_000.0, 4_800_000.0, 2010.0);

/// Written once: the tests run in parallel and must not truncate the file another one reads.
fn dictionary_dir() -> PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir =
            std::env::temp_dir().join(format!("proj-rs-init-sections-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("dictionary dir");
        fs::write(dir.join("multi_dic"), DICTIONARY).expect("write dictionary");
        dir
    })
    .clone()
}

fn convert(section: &str) -> (f64, f64, f64) {
    let mut builder = ProjBuilder::new();
    builder
        .set_search_paths(dictionary_dir())
        .expect("search path");
    let proj = builder
        .proj(&format!("+init=multi_dic:{section}"))
        .unwrap_or_else(|error| panic!("{section}: {error}"));
    let (x, y, z, _) = proj
        .convert_xyzt(INPUT.0, INPUT.1, INPUT.2, INPUT.3)
        .expect(section);
    (x, y, z)
}

fn assert_near(got: (f64, f64, f64), expected: (f64, f64, f64)) {
    let close = |a: f64, b: f64| (a - b).abs() <= 1e-6;
    assert!(
        close(got.0, expected.0) && close(got.1, expected.1) && close(got.2, expected.2),
        "{got:?} vs {expected:?}"
    );
}

#[test]
fn a_section_ends_where_the_next_one_starts() {
    assert_near(convert("FIRST"), (4_000_001.0, 100_002.0, 4_800_003.0));
    assert_near(
        convert("SECOND"),
        (4_000_000.007_660, 99_999.999_514, 4_799_999.981_772),
    );
}

#[test]
fn the_last_section_ends_with_the_file_and_loses_its_comments() {
    assert_near(convert("LAST"), (4_000_010.0, 100_020.0, 4_800_000.0));
}
