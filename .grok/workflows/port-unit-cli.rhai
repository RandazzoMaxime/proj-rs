let meta = #{
    name: "port-unit-cli",
    description: "Make every PROJ 9.9 unit test and the 258 remaining CLI cases pass",
    when_to_use: "After the gie suite, port docs/UNPORTED-UNIT.md and docs/UNPORTED-CLI.md until both lists are empty.",
    phases: [
        #{ title: "Model", detail: "ISO 19111 objects, WKT dialects, and the proj.db factory" },
        #{ title: "Units", detail: "Port each unported unit file until its TEST names run" },
        #{ title: "CLI", detail: "Port the 258 yaml cases, including projinfo WKT and the proj binary" },
        #{ title: "Verify", detail: "Fail closed unless both unported lists are empty and the tests pass" },
    ],
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
    p += "Unknown +proj is an error, never a coordinate. Do not loosen a .gie tolerance or edit an expected coordinate. ";
    p += "builtins.gie must stay failed=0 skipped=0. Do not change tests/gie/passing.txt unless a green gie run prints a new total with failed=0. ";
    p += "PROJ 9.9.0 sources are at C:\\Users\\maxim\\AppData\\Local\\Temp\\PROJ-9.9.0. The native tests are the oracle. ";
    p += "A ported test must assert the same values as the C++ test. Do not delete a test, skip it, or weaken it to make a list shorter. ";
    p += "When a TEST name from docs/UNPORTED-UNIT.md or a row from docs/UNPORTED-CLI.md passes, remove that entry. Leave the files, with their headers, empty of entries only when nothing remains. ";
    p += "Clippy: cargo clippy --offline --all-targets --all-features -- -D warnings. ";
    p
}

fn model_prompt() {
    let p = rules();
    p += "Build the library surface the unported unit tests call. Read test/unit/test_crs.cpp, test_factory.cpp, and test_io.cpp under the PROJ tree, and src/iso19111 plus src/iso19111/factory.cpp. ";
    p += "Expose geographic, projected, bound, and compound CRS objects, datum ensembles, coordinate systems, and WKT export for WKT2:2019, WKT2:2015, their simplified forms, WKT1_GDAL, and WKT1_ESRI. ";
    p += "A factory must read data/proj.db the way PROJ's AuthorityFactory and CoordinateOperationFactory do for the cases those tests hit. ";
    p += "Put the types where tests can use them from the crate root. Document the module in docs/CRS-API.md. ";
    p += "Add one Rust test that builds EPSG:4326 from the database and checks one WKT2:2019 fragment the C++ test checks. ";
    p += "Do not commit. Final message: the type names and the test that passed. ";
    p
}

fn unit_prompt(files) {
    let p = rules();
    p += "Port these PROJ unit files so every TEST name in them runs and passes: ";
    p += files;
    p += ". They live under C:\\Users\\maxim\\AppData\\Local\\Temp\\PROJ-9.9.0\\test\\unit. ";
    p += "Use the CRS, WKT, and factory types already added in this workspace. If one is missing, add it. ";
    p += "Each new test goes in tests/proj_unit.rs or a file under tests/ named after the C++ file. ";
    p += "Remove the matching entries from docs/UNPORTED-UNIT.md only after cargo test --offline shows that test passing. ";
    p += "Do not commit. Final message: how many TEST names from these files still remain in the unported list. ";
    p
}

fn cli_prompt(scope) {
    let p = rules();
    p += "Port the CLI cases still listed in docs/UNPORTED-CLI.md. ";
    p += scope;
    p += " The yaml files are under C:\\Users\\maxim\\AppData\\Local\\Temp\\PROJ-9.9.0\\test\\cli and tests/cli_yaml. ";
    p += "Extend src/bin/cs2cs.rs, src/bin/cct.rs, and src/bin/projinfo.rs. Add src/bin/proj.rs for the four proj cases. ";
    p += "cs2cs must accept DMS, grads, a third axis, and a prime meridian the way the yaml expects. projinfo must write the WKT dialects those cases compare. ";
    p += "Drive them with the existing yaml runner in tests/cli_yaml.rs. Remove a row from docs/UNPORTED-CLI.md only when that case passes. ";
    p += "Do not commit. Final message: how many CLI rows remain. ";
    p
}

fn verify_prompt() {
    let p = "Check B:\\proj-rs. Use read_file, grep, and cargo. Do not edit files. ";
    p += "docs/UNPORTED-UNIT.md must contain no TEST name entries. docs/UNPORTED-CLI.md must contain no table rows. ";
    p += "Run cargo test --offline --test proj_unit --test cli_yaml -- --nocapture. Every test must pass. ";
    p += "Run cargo test --offline --test gie_run -- --nocapture and confirm builtins.gie failed=0 skipped=0. ";
    p += "ok is true only if you saw an empty unported list and passing tests. Quote the test result lines in evidence. Otherwise ok is false and name what remains. ";
    p
}

phase("Model");
let model = agent(model_prompt(), #{
    label: "crs-wkt-factory",
    capability_mode: "all",
});
if model == () || model.success != true {
    complete(#{
        summary: "CRS model failed",
        ok: false,
        evidence: "",
    });
}

phase("Units");
let unit_files = [
    "test_crs.cpp and test_factory.cpp",
    "test_io.cpp",
    "test_operation.cpp and test_operationfactory.cpp",
    "test_c_api.cpp",
];
for files in unit_files {
    let result = agent(unit_prompt(files), #{
        label: "unit",
        capability_mode: "all",
    });
    if result == () || result.success != true {
        log("unit slice failed");
    }
}

phase("CLI");
let cli = agent(cli_prompt("Cover all 258 rows: 132 cs2cs, 111 projinfo, 11 cct, and 4 proj."), #{
    label: "cli-258",
    capability_mode: "all",
});
if cli == () || cli.success != true {
    log("cli slice failed");
}

phase("Verify");
let verdict = agent(verify_prompt(), #{
    label: "verify-empty",
    capability_mode: "execute",
    output_schema: verify_schema,
});

let ok = false;
let evidence = "";
if verdict != () && verdict.success == true && type_of(verdict.output) == "map" {
    if verdict.output.ok == true && verdict.output.evidence != () {
        evidence = verdict.output.evidence;
        if evidence.contains("0 failed") {
            ok = true;
        }
    }
}

let summary = "unit and CLI lists are not empty";
if ok {
    summary = "every listed unit test and CLI case passed";
}
complete(#{
    summary: summary,
    ok: ok,
    evidence: evidence,
});
