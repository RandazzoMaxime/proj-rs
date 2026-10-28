let meta = #{
    name: "port-external-grids",
    description: "Drive the vendored PROJ 9.9 gie suite to failed=0 and skipped=0",
    when_to_use: "Finish every remaining skipped .gie case, including external grids.",
    phases: [
        #{ title: "Census", detail: "List every case the gie runner still skips" },
        #{ title: "Close", detail: "Port or fetch until the suite reports skipped=0" },
        #{ title: "Verify", detail: "Accept only failed=0 and skipped=0" },
    ],
};

let verify_schema = #{
    "type": "object",
    "properties": #{
        "ok": #{ "type": "boolean" },
        "evidence": #{ "type": "string" },
        "passes": #{ "type": "string" },
    },
};

fn rules() {
    let p = "Repo B:\\proj-rs, branch dev. Do not commit, push, or merge. Do not revert existing work. ";
    p += "The finish line is one line from cargo test --offline --test gie_run -- --nocapture: total passed=N failed=0 skipped=0. ";
    p += "builtins.gie must stay failed=0 skipped=0. tests/gie/passing.txt must equal N. ";
    p += "A skipped case is unfinished work. Do not leave it skipped. Do not delete a .gie case. Do not loosen a tolerance. Do not edit an expected coordinate. ";
    p += "Unknown +proj or a missing file is an error at creation, never a coordinate, until you have built the operation or obtained the file. Then the case must run. ";
    p += "Rust edition 2021, rust-version 1.83. Prepend C:\\Users\\maxim\\.cargo\\bin to PATH. ";
    p += "No GPU path. A CPU path is never a production fallback for a GPU kernel. ";
    p += "Clippy: cargo clippy --offline --all-targets --all-features -- -D warnings. ";
    p += "Downloaded grids stay in B:\\proj-rs\\data\\grids\\ which is gitignored. Do not commit them. ";
    p
}

fn census_prompt() {
    let p = rules();
    p += "Run cargo test --offline --test gie_run -- --nocapture. Do not edit files. ";
    p += "From the output, list every .gie file whose skipped count is not 0, and for each file the operation or require_grid line that caused the skip. ";
    p += "Group them into: missing external grid, unbuilt +proj or parameter, EPSG or URN the database did not turn into a ported pipeline, and anything else. ";
    p += "Final message must include the total passed, failed, and skipped line. ";
    p
}

fn close_prompt(census) {
    let p = rules();
    p += "Drive tests/gie to failed=0 and skipped=0. A previous census follows. If it is empty, run the gie test yourself and use that. ";
    p += census;
    p += " For a missing grid, download the file. https://cdn.proj.org/<filename> 404 is not the end: search https://github.com/OSGeo/PROJ-data for that filename and also the PROJ 9.9 grid_alternatives name. ";
    p += "These CDN names 404 at the root and still need a real file: BETA2007.gsb, egm96_15.gtx, ntf_r93.gsb. ntf_r93.gsb is already under tests/gie/tests. ";
    p += "Save downloads in data/grids under the exact filename the .gie file uses. ";
    p += "require_grid must skip only when that file is absent. When the file is present, the case runs. ";
    p += "For an unbuilt operation, port it from C:\\Users\\maxim\\AppData\\Local\\Temp\\PROJ-9.9.0 and register it. ";
    p += "For an EPSG or URN code, resolve it from data/proj.db into steps this crate builds. A grid step needs the grid file on disk, then it runs. ";
    p += "Repeat cargo test --offline --test gie_run -- --nocapture until the total line is failed=0 skipped=0. Then set tests/gie/passing.txt to that passed count. ";
    p += "cargo fmt and cargo clippy --offline --all-targets --all-features -- -D warnings. Do not commit. ";
    p += "Final message must quote the total passed, failed, and skipped line. ";
    p
}

fn verify_prompt() {
    let p = "Check B:\\proj-rs. Use read_file, grep, and cargo. Do not edit files. ";
    p += "Run cargo test --offline --test gie_run -- --nocapture. ";
    p += "ok is true only if you saw total passed=N failed=0 skipped=0, builtins.gie failed=0 skipped=0, and tests/gie/passing.txt equal to N. ";
    p += "If any file still skips, ok is false. Put the total line in evidence exactly as printed. ";
    p
}

phase("Census");
let census = agent(census_prompt(), #{
    label: "census",
    capability_mode: "execute",
});
let notes = "";
if census != () && census.success == true && census.output != () {
    notes = json_encode(census.output);
}

phase("Close");
let closed = agent(close_prompt(notes), #{
    label: "close-skips",
    capability_mode: "all",
});
if closed == () || closed.success != true {
    complete(#{
        summary: "could not reach skipped=0",
        ok: false,
        evidence: "",
    });
}

phase("Verify");
let verdict = agent(verify_prompt(), #{
    label: "verify-zero-skips",
    capability_mode: "execute",
    output_schema: verify_schema,
});

let ok = false;
let evidence = "";
if verdict != () && verdict.success == true && type_of(verdict.output) == "map" {
    if verdict.output.ok == true && verdict.output.evidence != () {
        evidence = verdict.output.evidence;
        if evidence.contains("skipped=0") && evidence.contains("failed=0") {
            ok = true;
        }
    }
}

let summary = "gie suite still skips cases";
if ok {
    summary = "gie suite has failed=0 and skipped=0";
}
complete(#{
    summary: summary,
    ok: ok,
    evidence: evidence,
});
