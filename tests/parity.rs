//! Suite de parité contre PROJ 9.9.0 (voir docs/PARITY.md).
//!
//! `tests/parity/cases.jsonl` liste les invocations (outil, arguments, lignes
//! d'entrée) ; `tests/parity/expected.jsonl` donne la sortie de la référence.
//! Chaque invocation est relancée avec notre binaire et comparée ligne à ligne.
//! Les écarts connus sont dans `tests/parity/known.tsv`.

mod common;
#[cfg(feature = "network")]
#[path = "net_cdn/mod.rs"]
mod net_cdn;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_yaml::Value;

const TOLERANCE_METRE: f64 = 5e-4;
const TOLERANCE_DEGREE: f64 = 5e-9;
const TOLERANCE_TIME: f64 = 5e-9;
const TOLERANCE_FACTOR: f64 = 1e-5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Metre,
    Degree,
    Time,
    Factor,
    /// A whole line compared as text: the candidate list of `projinfo --summary`.
    Text,
    /// A degrees-minutes-seconds field of `geod`, compared as text.
    Dms,
}

impl Kind {
    fn parse(text: &str) -> Kind {
        match text {
            "m" => Kind::Metre,
            "deg" => Kind::Degree,
            "t" => Kind::Time,
            "fac" => Kind::Factor,
            "text" => Kind::Text,
            "dms" => Kind::Dms,
            other => panic!("genre de champ inconnu dans cases.jsonl : {other}"),
        }
    }

    fn within(self, expected: f64, actual: f64) -> bool {
        let diff = (expected - actual).abs();
        match self {
            Kind::Metre => diff <= TOLERANCE_METRE,
            Kind::Degree => diff <= TOLERANCE_DEGREE,
            Kind::Time => diff <= TOLERANCE_TIME,
            // Relative, but absolute near zero: an angular distortion of 0 against 1e-6 degree
            // is the finite-difference noise of `pj_deriv`, printed with six digits.
            Kind::Factor => diff <= TOLERANCE_FACTOR * expected.abs().max(actual.abs()).max(1.0),
            Kind::Text | Kind::Dms => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    Status,
    Shape,
    Value,
}

impl Class {
    fn label(self) -> &'static str {
        match self {
            Class::Status => "statut",
            Class::Shape => "forme",
            Class::Value => "valeur",
        }
    }
}

enum Outcome {
    Identical,
    Within,
    BothError,
    Deviation(Class, String),
}

/// Where the binaries get their grids, and whether the network is on.
#[derive(Clone, Copy)]
enum Source<'a> {
    /// The repository fixtures, network off.
    Local,
    /// The fixtures plus a grid directory (docs/PARITY.md, grid corpus), network off.
    Grids(&'a str),
    /// The grid files of cdn.proj.org, network on, a new empty user directory per invocation.
    #[cfg_attr(
        not(feature = "network"),
        expect(dead_code, reason = "only the network corpus builds it")
    )]
    Network,
}

/// An empty user-writable directory for one invocation, removed when dropped: no cache survives
/// from a previous run, so each invocation fetches its grid chunks itself.
struct FreshUserDir(PathBuf);

impl FreshUserDir {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let index = NEXT.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("proj-rs-parity-net-{}-{index}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("créer le dossier utilisateur de la suite réseau");
        Self(path)
    }
}

impl Drop for FreshUserDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct Case {
    id: String,
    family: String,
    tool: String,
    args: Vec<String>,
    input: Vec<String>,
    kinds: Vec<Kind>,
}

struct Expected {
    rc: i64,
    out: Vec<String>,
}

struct Deviation {
    key: String,
    family: String,
    class: Class,
    detail: String,
    repro: String,
}

#[derive(Default)]
struct Stats {
    invocations: usize,
    conversions: usize,
    identical: usize,
    within: usize,
    both_error: usize,
    outside: usize,
    status: usize,
    shape: usize,
}

fn read_jsonl(path: &Path) -> Vec<Value> {
    let text = fs::read_to_string(path).unwrap_or_else(|error| {
        panic!(
            "{} illisible ({error}) : régénérer avec tools/parity/generate.py et record.sh",
            path.display()
        )
    });
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_yaml::from_str(line).expect("ligne JSON de la suite de parité"))
        .collect()
}

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn strings(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_sequence)
        .map(|items| {
            items
                .iter()
                .map(|item| item.as_str().unwrap_or_default().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn load_cases(path: &Path) -> Vec<Case> {
    read_jsonl(path)
        .iter()
        .map(|value| Case {
            id: text(value, "id"),
            family: text(value, "family"),
            tool: text(value, "tool"),
            args: strings(value, "args"),
            input: strings(value, "input"),
            kinds: strings(value, "kinds")
                .iter()
                .map(|kind| Kind::parse(kind))
                .collect(),
        })
        .collect()
}

fn load_expected(path: &Path) -> BTreeMap<String, Expected> {
    read_jsonl(path)
        .iter()
        .map(|value| {
            let rc = value.get("rc").and_then(Value::as_i64).unwrap_or(-1);
            let out = strings(value, "out");
            (text(value, "id"), Expected { rc, out })
        })
        .collect()
}

fn tokens(line: &str) -> Vec<String> {
    line.replace(['<', '>'], " ")
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

fn is_error(tokens: &[String]) -> bool {
    tokens
        .iter()
        .any(|token| matches!(token.as_str(), "*" | "inf" | "-inf" | "nan" | "-nan"))
}

/// A `dms` field that differs, also on a line holding an error token (a NaN azimuth).
fn dms_mismatch(want: &[String], got: &[String], kinds: &[Kind]) -> Option<String> {
    if want.len() != got.len() {
        return None;
    }
    for (index, (x, y)) in want.iter().zip(got).enumerate() {
        if x != y && kinds.get(index) == Some(&Kind::Dms) {
            return Some(format!("champ {} (dms) : «{x}» contre «{y}»", index + 1));
        }
    }
    None
}

fn compare(expected: &str, actual: &str, kinds: &[Kind]) -> Outcome {
    if kinds == [Kind::Text] {
        return if expected == actual {
            Outcome::Identical
        } else {
            Outcome::Deviation(
                Class::Value,
                format!("attendu «{expected}», obtenu «{actual}»"),
            )
        };
    }
    let want = tokens(expected);
    let got = tokens(actual);
    let (want_error, got_error) = (is_error(&want), is_error(&got));
    if want_error != got_error {
        let detail = format!("sortie attendue «{expected}», obtenue «{actual}»");
        return Outcome::Deviation(Class::Status, detail);
    }
    if want_error {
        if want == got {
            return Outcome::Identical;
        }
        return match dms_mismatch(&want, &got, kinds) {
            Some(detail) => Outcome::Deviation(Class::Value, detail),
            None => Outcome::BothError,
        };
    }
    if want.len() != got.len() {
        let detail = format!("{} champs attendus, {} obtenus", want.len(), got.len());
        return Outcome::Deviation(Class::Shape, detail);
    }
    let mut exact = true;
    for (index, (x, y)) in want.iter().zip(&got).enumerate() {
        if x == y {
            continue;
        }
        exact = false;
        if kinds.get(index) == Some(&Kind::Dms) {
            let detail = format!("champ {} (dms) : «{x}» contre «{y}»", index + 1);
            return Outcome::Deviation(Class::Value, detail);
        }
        let (Ok(expected_value), Ok(actual_value)) = (x.parse::<f64>(), y.parse::<f64>()) else {
            let detail = format!("champ {} : «{x}» contre «{y}»", index + 1);
            return Outcome::Deviation(Class::Shape, detail);
        };
        let Some(kind) = kinds.get(index) else {
            let detail = format!("champ {} sans genre dans cases.jsonl", index + 1);
            return Outcome::Deviation(Class::Shape, detail);
        };
        if !kind.within(expected_value, actual_value) {
            let detail = format!(
                "champ {} ({kind:?}) : {x} contre {y}, écart {:e}",
                index + 1,
                (expected_value - actual_value).abs()
            );
            return Outcome::Deviation(Class::Value, detail);
        }
    }
    if exact {
        Outcome::Identical
    } else {
        Outcome::Within
    }
}

fn shell_quote(arg: &str) -> String {
    format!("'{}'", arg.replace('\'', "'\\''"))
}

fn repro(case: &Case, source: Source<'_>) -> String {
    let input: Vec<String> = case.input.iter().map(|line| shell_quote(line)).collect();
    let args: Vec<String> = case.args.iter().map(|arg| shell_quote(arg)).collect();
    let environment = match source {
        Source::Local => {
            "PROJ_DATA=data:data/tests PROJ_NETWORK=OFF \
                          PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES"
        }
        Source::Grids(_) => {
            "PROJ_DATA=data:data/tests:$PARITY_GRIDS_DIR PROJ_NETWORK=OFF \
                             PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES"
        }
        Source::Network => {
            "PROJ_DATA=data PROJ_NETWORK=ON \
                            PROJ_USER_WRITABLE_DIRECTORY=$(mktemp -d)"
        }
    };
    format!(
        "printf '%s\\n' {} | {environment} PROJ_ONLY_BEST_DEFAULT=NO target/debug/{} {}",
        input.join(" "),
        case.tool,
        args.join(" ")
    )
}

fn binary(tool: &str) -> &'static str {
    match tool {
        "cs2cs" => env!("CARGO_BIN_EXE_cs2cs"),
        "proj" => env!("CARGO_BIN_EXE_proj"),
        "cct" => env!("CARGO_BIN_EXE_cct"),
        "projinfo" => env!("CARGO_BIN_EXE_projinfo"),
        "geod" => env!("CARGO_BIN_EXE_geod"),
        "invgeod" => env!("CARGO_BIN_EXE_invgeod"),
        other => panic!("outil inconnu dans cases.jsonl : {other}"),
    }
}

fn run(case: &Case, source: Source<'_>) -> (i64, Vec<String>) {
    let mut data = match source {
        Source::Network => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .display()
            .to_string(),
        Source::Local | Source::Grids(_) => common::fixture_search_path(),
    };
    if let Source::Grids(dir) = source {
        data.push(if cfg!(windows) { ';' } else { ':' });
        data.push_str(dir);
    }
    let user_dir = matches!(source, Source::Network).then(FreshUserDir::new);
    let mut command = Command::new(binary(&case.tool));
    command
        .args(&case.args)
        // Recorded with build/data/proj.ini (only_best_default = off), which this search path lacks.
        .env("PROJ_ONLY_BEST_DEFAULT", "NO")
        .env("PROJ_DATA", data);
    match &user_dir {
        Some(dir) => {
            command
                .env("PROJ_NETWORK", "ON")
                .env("PROJ_USER_WRITABLE_DIRECTORY", &dir.0);
        }
        None => {
            command
                .env("PROJ_NETWORK", "OFF")
                .env("PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY", "YES");
        }
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("lancer le binaire de la suite de parité");
    if let Some(mut stdin) = child.stdin.take() {
        // record.py writes nothing for an empty input: sending a newline would add a blank line.
        let mut text = case.input.join("\n");
        if !case.input.is_empty() {
            text.push('\n');
        }
        // A binary that exits before reading (an error case) closes the pipe: its exit code and
        // output are what get compared, so a broken pipe is not a failure here.
        match stdin.write_all(text.as_bytes()) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => {}
            Err(error) => panic!("écrire l'entrée : {error}"),
        }
    }
    let output = child.wait_with_output().expect("sortie du binaire");
    let code = output.status.code().map_or(-1, i64::from);
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    let lines = stdout.lines().map(str::to_owned).collect();
    (code, lines)
}

impl Stats {
    fn add(&mut self, other: &Stats) {
        self.invocations += other.invocations;
        self.conversions += other.conversions;
        self.identical += other.identical;
        self.within += other.within;
        self.both_error += other.both_error;
        self.outside += other.outside;
        self.status += other.status;
        self.shape += other.shape;
    }
}

fn check_invocation(
    case: &Case,
    expected: &Expected,
    source: Source<'_>,
    deviations: &mut Vec<Deviation>,
    stats: &mut Stats,
) {
    let (code, got) = run(case, source);
    let lines = expected.out.len().max(got.len());
    stats.invocations += 1;
    stats.conversions += lines;
    let mut deviate = |key: String, class: Class, detail: String| {
        deviations.push(Deviation {
            key,
            family: case.family.clone(),
            class,
            detail,
            repro: repro(case, source),
        });
    };
    if (expected.rc != 0) != (code != 0) {
        let detail = format!("code de sortie attendu {}, obtenu {code}", expected.rc);
        stats.status += 1;
        deviate(format!("{}:rc", case.id), Class::Status, detail);
    }
    for index in 0..lines {
        let key = format!("{}:{}", case.id, index + 1);
        let (Some(want), Some(have)) = (expected.out.get(index), got.get(index)) else {
            let detail = format!(
                "{} lignes attendues, {} obtenues",
                expected.out.len(),
                got.len()
            );
            stats.shape += 1;
            deviate(key, Class::Shape, detail);
            continue;
        };
        match compare(want, have, &case.kinds) {
            Outcome::Identical => stats.identical += 1,
            Outcome::Within => stats.within += 1,
            Outcome::BothError => stats.both_error += 1,
            Outcome::Deviation(class, detail) => {
                match class {
                    Class::Status => stats.status += 1,
                    Class::Shape => stats.shape += 1,
                    Class::Value => stats.outside += 1,
                }
                deviate(key, class, detail);
            }
        }
    }
}

fn read_known(path: &Path) -> BTreeMap<String, (String, String)> {
    let text = fs::read_to_string(path).unwrap_or_default();
    text.lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .map(|line| {
            let mut columns = line.split('\t');
            let key = columns.next().unwrap_or_default().to_owned();
            let cause = columns.next().unwrap_or_default().to_owned();
            let sheet = columns.next().unwrap_or("-").to_owned();
            (key, (cause, sheet))
        })
        .collect()
}

fn write_known(path: &Path, known: &BTreeMap<String, (String, String)>) {
    let mut out = String::from(
        "# Écarts connus de la suite de parité (tests/parity.rs).\n\
         # Colonnes : clé « id:ligne », cause courte, renvoi à une fiche.\n\
         # Un écart listé ne fait pas échouer ; un écart nouveau ou disparu oui.\n",
    );
    for (key, (cause, sheet)) in known {
        out.push_str(&format!("{key}\t{cause}\t{sheet}\n"));
    }
    fs::write(path, out).expect("écrire known.tsv");
}

fn write_report(path: &Path, deviations: &[Deviation], known: &BTreeMap<String, (String, String)>) {
    let mut out = String::from("clé\tfamille\tclasse\tconnu\tdétail\treproduction\n");
    for deviation in deviations {
        let flag = if known.contains_key(&deviation.key) {
            "oui"
        } else {
            "non"
        };
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\n",
            deviation.key,
            deviation.family,
            deviation.class.label(),
            flag,
            deviation.detail.replace(['\t', '\n'], " "),
            deviation.repro
        ));
    }
    fs::write(path, out).expect("écrire le rapport de parité");
}

fn print_summary(cases: usize, stats: &BTreeMap<String, Stats>) {
    let mut total = Stats::default();
    for family in stats.values() {
        total.invocations += family.invocations;
        total.conversions += family.conversions;
        total.identical += family.identical;
        total.within += family.within;
        total.both_error += family.both_error;
        total.outside += family.outside;
        total.status += family.status;
        total.shape += family.shape;
    }
    println!(
        "parité contre PROJ 9.9.0 : {cases} invocations, {} conversions",
        total.conversions
    );
    println!(
        "  identiques (texte) {} · dans le seuil {} · erreur des deux côtés {}",
        total.identical, total.within, total.both_error
    );
    println!(
        "  hors seuil {} · statut {} · forme {}",
        total.outside, total.status, total.shape
    );
    println!("  famille : invocations conversions identiques dans-seuil hors-seuil statut forme");
    for (name, family) in stats {
        println!(
            "  {name} : {} {} {} {} {} {} {}",
            family.invocations,
            family.conversions,
            family.identical,
            family.within,
            family.outside,
            family.status,
            family.shape
        );
    }
}

#[test]
fn parity_against_proj_9_9_0() {
    check_corpus("tests/parity", Source::Local);
}

/// The wide corpus (about 50 000 conversions, see docs/PARITY.md): several minutes, so it runs
/// on demand with `cargo test --test parity -- --ignored`.
#[test]
#[ignore = "corpus large : cargo test --test parity -- --ignored"]
fn parity_wide_against_proj_9_9_0() {
    check_corpus("tests/parity/wide", Source::Local);
}

/// `geod` and `invgeod` on their own corpus (docs/PARITY.md, section geod).
#[test]
fn parity_geod_against_proj_9_9_0() {
    check_corpus("tests/parity/geod", Source::Local);
}

/// The export corpus: CRS, operations, datums and ellipsoids written by `projinfo` in every
/// export format (about 9 000 invocations, see docs/PARITY.md). About 9 minutes in debug.
#[test]
#[ignore = "corpus d'exports : cargo test --test parity parity_export -- --ignored"]
fn parity_export_against_proj_9_9_0() {
    check_corpus("tests/parity/export", Source::Local);
}

/// The grid corpus (about 10 000 conversions, see docs/PARITY.md): its grid files are in
/// PARITY_GRIDS_DIR, made by tools/parity/fetch-grids.sh from tests/parity/grids/grids.txt.
#[test]
#[ignore = "corpus avec grilles : PARITY_GRIDS_DIR=... cargo test --test parity -- --ignored"]
fn parity_grids_against_proj_9_9_0() {
    let dir = std::env::var("PARITY_GRIDS_DIR").unwrap_or_else(|_| {
        panic!(
            "PARITY_GRIDS_DIR absent : lancer tools/parity/fetch-grids.sh <dossier>, puis \
             PARITY_GRIDS_DIR=<dossier> cargo test --test parity -- --ignored"
        )
    });
    assert!(
        Path::new(&dir).is_dir(),
        "PARITY_GRIDS_DIR n'est pas un dossier : {dir}"
    );
    check_corpus("tests/parity/grids", Source::Grids(&dir));
}

/// The CDN corpus (docs/PARITY.md, section réseau): every pair uses a grid of cdn.proj.org, the
/// network is on and each invocation starts from an empty user directory. The CDN probe comes
/// first: without it the corpus would compare nothing, so the test fails instead of passing.
#[cfg(feature = "network")]
#[test]
#[ignore = "réseau : cargo test --features network --test parity -- --ignored"]
fn parity_network_against_proj_9_9_0() {
    assert!(
        net_cdn::cdn_reachable(),
        "cdn.proj.org ne répond pas (sonde tests/net_cdn/mod.rs : GET avec Range, 206 attendu). \
         Le corpus réseau ne peut pas être comparé : vérifier la connexion ou HTTPS_PROXY."
    );
    check_corpus("tests/parity/network", Source::Network);
}

/// A corpus drawn at random from a seed by `tools/parity/fuzz_diff.py` (docs/FUZZ.md):
/// PARITY_RANDOM_DIR holds its `cases.jsonl` and `expected.jsonl`. It has no `known.tsv` unless
/// the tool's operator wrote one, so every deviation fails and goes to PARITY_REPORT.
#[test]
#[ignore = "corpus aléatoire : PARITY_RANDOM_DIR=... cargo test --test parity -- --ignored"]
fn parity_random_against_proj_9_9_0() {
    let dir = std::env::var("PARITY_RANDOM_DIR").unwrap_or_else(|_| {
        panic!(
            "PARITY_RANDOM_DIR absent : lancer tools/parity/fuzz_diff.py --seed N, qui l'écrit \
             et lance ce test"
        )
    });
    assert!(
        Path::new(&dir).is_absolute(),
        "PARITY_RANDOM_DIR doit être un chemin absolu : {dir}"
    );
    check_corpus(&dir, Source::Local);
}

fn check_corpus(dir: &str, source: Source<'_>) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(dir);
    let cases = load_cases(&root.join("cases.jsonl"));
    let expected = load_expected(&root.join("expected.jsonl"));
    let known_path = root.join("known.tsv");
    let workers = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .clamp(1, 8);
    let chunk = cases.len().div_ceil(workers).max(1);
    let per_case: Vec<(Vec<Deviation>, Stats)> = std::thread::scope(|scope| {
        let handles: Vec<_> = cases
            .chunks(chunk)
            .map(|part| {
                let expected = &expected;
                scope.spawn(move || {
                    part.iter()
                        .map(|case| {
                            let reference = expected
                                .get(&case.id)
                                .unwrap_or_else(|| panic!("{} absent de expected.jsonl", case.id));
                            let mut found = Vec::new();
                            let mut stats = Stats::default();
                            check_invocation(case, reference, source, &mut found, &mut stats);
                            (found, stats)
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("travailleur de la suite de parité"))
            .collect()
    });
    let mut deviations = Vec::new();
    let mut stats: BTreeMap<String, Stats> = BTreeMap::new();
    for (case, (found, part)) in cases.iter().zip(per_case) {
        deviations.extend(found);
        stats.entry(case.family.clone()).or_default().add(&part);
    }
    print_summary(cases.len(), &stats);

    let mut known = read_known(&known_path);
    // The expected values come from PROJ 9.9.0 on Linux (glibc). Another libm rounds otherwise,
    // and the ill-conditioned points (near-antipodal geodesics, huge coordinates) move past the
    // thresholds: their deviations are named in `known-<os>.tsv`, read only off glibc.
    if !cfg!(all(target_os = "linux", target_env = "gnu")) {
        known.extend(read_known(
            &root.join(format!("known-{}.tsv", std::env::consts::OS)),
        ));
    }
    let observed: BTreeSet<String> = deviations.iter().map(|d| d.key.clone()).collect();
    if let Ok(path) = std::env::var("PARITY_REPORT") {
        write_report(Path::new(&path), &deviations, &known);
    }
    if std::env::var("PARITY_UPDATE_KNOWN").as_deref() == Ok("1") {
        let mut updated = BTreeMap::new();
        for deviation in &deviations {
            let entry = known.get(&deviation.key).cloned().unwrap_or_else(|| {
                (
                    format!("à classer ({})", deviation.class.label()),
                    "-".to_owned(),
                )
            });
            updated.insert(deviation.key.clone(), entry);
        }
        write_known(&known_path, &updated);
        return;
    }
    let new: Vec<String> = deviations
        .iter()
        .filter(|d| !known.contains_key(&d.key))
        .map(|d| format!("{} [{}] {} · {}", d.key, d.class.label(), d.detail, d.repro))
        .collect();
    let gone: Vec<String> = known
        .keys()
        .filter(|key| !observed.contains(*key))
        .cloned()
        .collect();
    assert!(
        new.is_empty() && gone.is_empty(),
        "{} écart(s) nouveau(x) :\n{}\n{} écart(s) connu(s) disparu(s), à retirer de known.tsv :\n{}",
        new.len(),
        new.join("\n"),
        gone.len(),
        gone.join("\n")
    );
}
