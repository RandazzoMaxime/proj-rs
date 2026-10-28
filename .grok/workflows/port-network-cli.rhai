let meta = #{
    name: "port-network-cli",
    description: "Port PROJ network grid download, then cs2cs -d, projinfo, and the unit and CLI suites",
    when_to_use: "After the gie suite, add on-demand CDN grids and the PROJ apps and tests that exercise them.",
    phases: [
        #{ title: "Network", detail: "Download a missing grid from the CDN when PROJ_NETWORK is on" },
        #{ title: "Apps", detail: "cs2cs -d, a fuller cct, and projinfo, each in its own worktree" },
        #{ title: "Tests", detail: "Port test/unit and test/cli onto those binaries and the library" },
        #{ title: "Integrate", detail: "Copy the worktrees in and keep builtins green" },
        #{ title: "Verify", detail: "Fail closed unless network, -d, and projinfo actually run" },
    ],
};

let port_schema = #{
    "type": "object",
    "properties": #{
        "worktree": #{ "type": "string" },
        "summary": #{ "type": "string" },
    },
};

let verify_schema = #{
    "type": "object",
    "properties": #{
        "ok": #{ "type": "boolean" },
        "evidence": #{ "type": "string" },
    },
};

fn rules() {
    let p = "Repo B:\\proj-rs, branch dev. Do not commit, push, or merge. Do not revert existing work. ";
    p += "Rust edition 2021, rust-version 1.83. Prepend C:\\Users\\maxim\\.cargo\\bin to PATH. ";
    p += "No GPU path. A CPU path is never a production fallback for a GPU kernel. ";
    p += "Unknown +proj is an error at creation, never a coordinate. Do not loosen a .gie tolerance. Do not edit vendored expected coordinates. ";
    p += "builtins.gie must stay failed=0 skipped=0. Do not change tests/gie/passing.txt unless a green gie run prints a new total with failed=0. ";
    p += "Network download stays off unless PROJ_NETWORK is ON, YES, or TRUE, or the caller enables it. A test must not hit the public CDN. ";
    p += "Clippy: cargo clippy --offline --all-targets --all-features -- -D warnings. ";
    p += "PROJ 9.9.0 sources are at C:\\Users\\maxim\\AppData\\Local\\Temp\\PROJ-9.9.0. Read the C++ before writing Rust. ";
    p
}

fn network_prompt() {
    let p = rules();
    p += "Port PROJ's on-demand grid download. Read data/proj.ini, src/filemanager.cpp around pj_load_ini and the grid open path, and src/networkfilemanager.cpp. ";
    p += "Default is network = off and cdn_endpoint = https://cdn.proj.org. PROJ_NETWORK and PROJ_NETWORK_ENDPOINT override the ini, same as PROJ 9.9. ";
    p += "When a grid file is missing and the network is on, download endpoint + / + filename, write it into a local cache, and open that file. When the network is off, keep today's error. Never invent a coordinate. ";
    p += "Use proj.db grid_alternatives when PROJ looks up a short name such as alaska before calling the CDN. ";
    p += "ProjBuilder::enable_network, set_url_endpoint, get_url_endpoint, network_enabled, grid_cache_enable, and set_search_paths must drive this. They must not always return an error. ";
    p += "Pick an HTTP client that builds on rustc 1.83. Cache under a directory the tests can set. ";
    p += "Port the behavior of test/unit/test_network.cpp with a local HTTP server or a fixture. Do not call https://cdn.proj.org from the test. ";
    p += "cargo fmt and cargo clippy --offline --all-targets --all-features -- -D warnings. Do not commit. ";
    p += "Final message: the function that decides to download, and the test name that proves a missing file is fetched only when the network is on. ";
    p
}

fn app_prompt(key, scope) {
    let p = rules();
    p += "You are in a git worktree. Port only this app: ";
    p += key;
    p += ". ";
    p += scope;
    p += " Read the matching C++ under src/apps and the yaml under test/cli. ";
    p += "Do not edit src/lib.rs or tests/gie/passing.txt. Write PORT-NOTES-";
    p += key;
    p += ".md at the worktree root with new file paths. ";
    p += "Run git rev-parse --show-toplevel and cargo check --offline --bins. ";
    p += "Final message must include the worktree path. ";
    p
}

fn tests_prompt() {
    let p = rules();
    p += "You are in a git worktree. Port PROJ test/unit and test/cli onto proj-rs. ";
    p += "Sources: C:\\Users\\maxim\\AppData\\Local\\Temp\\PROJ-9.9.0\\test\\unit and test\\cli. ";
    p += "Add a Rust runner for the CLI yaml files test_cct.yaml, test_cs2cs_various.yaml, test_projinfo.yaml, and test_proj.yaml. Run our binaries. Skip a yaml case whose app is proj, geod, gie, or projsync, and list it in docs/UNPORTED-CLI.md. ";
    p += "Port these unit files as Rust tests against Operation and Proj: pj_phi2_test.cpp, proj_angular_io_test.cpp, proj_context_test.cpp, proj_errno_string_test.cpp, test_grids.cpp, test_defmodel.cpp, test_tinshift.cpp, test_polyhedral.cpp, test_tmerczoned.cpp, test_misc.cpp, test_util.cpp. ";
    p += "From test_crs.cpp, test_factory.cpp, test_io.cpp, test_operation.cpp, test_operationfactory.cpp, and test_c_api.cpp, port only the cases that build a pipeline or a projection we already have. List every other TEST() name in docs/UNPORTED-UNIT.md. Do not drop a name silently. ";
    p += "Write PORT-NOTES-tests.md at the worktree root. cargo test --offline the new tests. Do not commit. ";
    p
}

fn integrate_prompt(notes) {
    let p = rules();
    p += "Work in B:\\proj-rs. The network port is already in this workspace. Copy new files from the worktrees. Do not delete the network code. Notes JSON follows. ";
    p += notes;
    p += " Register new binaries in Cargo.toml if the worktrees did not. ";
    p += "Run cargo test --offline --test gie_run -- --nocapture. builtins.gie must be failed=0 skipped=0. Update passing.txt only if failed is 0 and the total changed. ";
    p += "cargo fmt and cargo clippy --offline --all-targets --all-features -- -D warnings. Do not commit. ";
    p
}

fn verify_prompt() {
    let p = "Check B:\\proj-rs. Use read_file, grep, and cargo. Do not edit files. ";
    p += "Confirm network download is off by default and turns on for PROJ_NETWORK=ON. Name the function. ";
    p += "Run the new network test and quote its pass line. ";
    p += "Run our cs2cs with -d 4 on a +proj pipeline and quote one numeric line. ";
    p += "Run projinfo and quote its first line. ";
    p += "Confirm docs/UNPORTED-UNIT.md and docs/UNPORTED-CLI.md exist if any case was left out. ";
    p += "Run cargo test --offline --test gie_run -- --nocapture and confirm builtins.gie failed=0 skipped=0. ";
    p += "ok is true only if you saw those command results. Put them in evidence. Otherwise ok is false. ";
    p
}

phase("Network");
let net = agent(network_prompt(), #{
    label: "network",
    capability_mode: "all",
});
if net == () || net.success != true {
    complete(#{
        summary: "network port failed",
        ok: false,
        evidence: "",
    });
}

phase("Apps");
let apps = [
    #{
        key: "cs2cs",
        scope: "Extend src/bin/cs2cs.rs. Implement -d digits, and the other flags used by test/cli/test_cs2cs_various.yaml and test_cs2cs_ntv2.yaml. -d 8 must change the printed precision. A +to split must still work.",
    },
    #{
        key: "cct",
        scope: "Extend src/bin/cct.rs so test/cli/test_cct.yaml cases for -d, -z, -t, -o, and a +proj pipeline match the yaml stdout. Keep the binary name cct.",
    },
    #{
        key: "projinfo",
        scope: "Add src/bin/projinfo.rs. Cover the options exercised by test/cli/test_projinfo.yaml: --version, -o PROJ, -o WKT2:2019, and -o PROJJSON for one EPSG code that proj.db can describe. Return an error for a dialect we cannot write.",
    },
];
let jobs = [];
for app in apps {
    jobs.push(#{
        prompt: app_prompt(app.key, app.scope),
        label: "app:" + app.key,
        capability_mode: "all",
        isolation_worktree: true,
        output_schema: port_schema,
    });
}
jobs.push(#{
    prompt: tests_prompt(),
    label: "unit-cli-tests",
    capability_mode: "all",
    isolation_worktree: true,
    output_schema: port_schema,
});
let ported = parallel(jobs);

let notes = [];
let dropped = 0;
for slot in ported {
    if slot != () && slot.success == true && type_of(slot.output) == "map" {
        notes.push(slot.output);
    } else {
        dropped += 1;
    }
}
if dropped > 0 {
    log("slots dropped: " + dropped.to_string());
}

phase("Integrate");
let integrated = agent(integrate_prompt(json_encode(notes)), #{
    label: "integrate",
    capability_mode: "all",
});
if integrated == () || integrated.success != true {
    complete(#{
        summary: "integrate failed",
        ok: false,
        evidence: "",
    });
}

phase("Verify");
let verdict = agent(verify_prompt(), #{
    label: "verify",
    capability_mode: "execute",
    output_schema: verify_schema,
});

let ok = false;
let evidence = "";
if verdict != () && verdict.success == true && type_of(verdict.output) == "map" {
    if verdict.output.ok == true && verdict.output.evidence != () {
        evidence = verdict.output.evidence;
        if evidence != "" {
            ok = true;
        }
    }
}

let summary = "network and CLI port did not verify";
if ok {
    summary = "network download, cs2cs -d, and projinfo verified";
}
complete(#{
    summary: summary,
    ok: ok,
    evidence: evidence,
});
