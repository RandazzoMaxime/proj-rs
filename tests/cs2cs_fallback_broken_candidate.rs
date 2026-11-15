//! `cs2cs EPSG:10644 +to EPSG:11069` (BES2020 Saba + Saba height to ETRS89-CZE [2007]), PROJ 9.9.0.
//! The first candidate's PROJ string converts degrees to radians twice, so PROJ cannot build it
//! (« Mismatched units between step 1 and 2 »); the second is `+proj=noop`. A point inside Saba
//! retries onto the second candidate (`trans.cpp`, `iRetry`). A point outside every area takes the
//! first candidate without grid, the broken one, with no retry: PROJ prints `* * inf`.

use std::io::Write;
use std::process::{Command, Stdio};

const INPUT: &str = "17.697206796547 -63.296817134000 186.223194172606\n\
                     -79.227095248346 20.060492464000 2801.461373206228\n\
                     17.692167501547 -63.270271875000 1413.126271679066\n";

/// `cs2cs -d 12 EPSG:10644 +to EPSG:11069` of PROJ 9.9.0 on [`INPUT`].
const PROJ_OUTPUT: &str = "17.697206796547\t-63.296817134000 186.223194172606\n\
                           *\t* inf\n\
                           17.692167501547\t-63.270271875000 1413.126271679066\n";

#[test]
fn point_outside_every_area_falls_back_to_the_first_candidate_even_broken() {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/data");
    let mut child = Command::new(env!("CARGO_BIN_EXE_cs2cs"))
        .args(["-d", "12", "EPSG:10644", "+to", "EPSG:11069"])
        .env("PROJ_DATA", data)
        .env("PROJ_NETWORK", "OFF")
        .env("PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY", "YES")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("cs2cs");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(INPUT.as_bytes())
        .expect("write");
    let output = child.wait_with_output().expect("wait");
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), PROJ_OUTPUT);
}
