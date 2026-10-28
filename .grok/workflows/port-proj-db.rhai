let meta = #{
    name: "port-proj-db",
    description: "Vendor PROJ 9.9 proj.db through rusqlite and port the next skipped projections",
    when_to_use: "Continue proj-rs: internal proj.db, then the most-skipped gie projections.",
    phases: [
        #{ title: "Database", detail: "Build proj.db from PROJ 9.9 SQL and open it with rusqlite" },
        #{ title: "Port", detail: "One projection family per agent, in its own worktree" },
        #{ title: "Integrate", detail: "Register the new modules and bump the gie ratchet" },
        #{ title: "Verify", detail: "Fail closed unless the database and builtins stay honest" },
    ],
};

let port_schema = #{
    "type": "object",
    "properties": #{
        "worktree": #{ "type": "string" },
        "summary": #{ "type": "string" },
        "gie_file": #{ "type": "string" },
    },
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
    let p = "Repo B:\\proj-rs, branch dev. Do not commit, push, or merge to main. ";
    p += "Rust edition 2021, rust-version 1.83. Prepend C:\\Users\\maxim\\.cargo\\bin to PATH. Use cargo --offline after dependencies are fetched. ";
    p += "build.rs compiles Karney geodesic.c with MSVC cl via a .bat file. Do not switch that to mingw. Do not add a GPU path. A CPU path is never a production fallback for a GPU kernel. ";
    p += "Unknown +proj or an operation this crate cannot build is an error at creation, never a coordinate. Do not loosen any .gie tolerance. ";
    p += "tests/gie/passing.txt stays equal to the live passing count. Update it only after a green cargo test --offline --test gie_run whose total passed rose and whose failed count is 0. ";
    p += "builtins.gie must stay at failed=0 and skipped=0. ";
    p += "Clippy is cargo clippy --offline --all-targets --all-features -- -D warnings. pedantic is allow in Cargo.toml. Do not use as casts without an expect and a reason. ";
    p += "Classic kernels return units of the semi-major axis. The wrapper multiplies by a. Apply k0 only inside kernels that read it. Read src/engine.rs and one existing module such as src/projections/laea.rs before writing. ";
    p
}

fn db_prompt() {
    let p = rules();
    p += "Task: give proj-rs an internal SQLite proj.db read with rusqlite. Take PROJ's database, do not hand-write EPSG rows. ";
    p += "The PROJ 9.9.0 source tree is C:\\Users\\maxim\\AppData\\Local\\Temp\\PROJ-9.9.0. Its data/sql directory plus data/sql_filelist.cmake is how PROJ builds proj.db: concatenate those SQL files in the listed order and load them with sqlite3. ";
    p += "OSGeo4W at C:\\Users\\maxim\\AppData\\Local\\Programs\\OSGeo4W has sqlite3 and a proj.db, but that binary is PROJ 9.7.1. Use it only as a tool. The database file in the repo must be built from the 9.9.0 SQL. Do not git commit. ";
    p += "Write the result to B:\\proj-rs\\data\\proj.db and keep PROJ's MIT notice beside it. ";
    p += "Add rusqlite with the bundled feature, choosing a crate version whose rust-version is 1.83 or older. Embed the bytes so a downstream crate does not need the file on disk. Open them with rusqlite deserialize if that feature exists. Otherwise copy the bytes to a temp file once and open that path. ";
    p += "Replace the hand-coded EPSG table used by Proj::new_known_crs with lookups in this database. Read the SQL schema before writing queries. Emit a PROJ pipeline only when every step is an operation this crate already builds. ";
    p += "Longitude, latitude order for geographic CRS. Easting, northing for projected CRS. ";
    p += "EPSG:4326 to EPSG:2154 must still match Lcc::lambert93 on the same lon/lat. Keep 4258, 4978, 3857, and UTM 32601-32660 and 32701-32760 working. ";
    p += "A code whose operation needs a grid, a deformation model, or a method this crate does not build returns ProjCreateError. Never return a coordinate for that case. ";
    p += "Network and grid methods on ProjBuilder still return an error. ";
    p += "Run cargo test --offline --test proj_facade and cargo test --offline --test gie_run. Fix failures you caused. Leave passing.txt unchanged if the pass count did not rise. ";
    p += "In your final message, state the proj.db byte size, the rusqlite version, and one SQL query you actually ran. ";
    p
}

fn port_prompt(key, gie_names, hint) {
    let p = rules();
    p += "You are in a git worktree. Port only the projection family named ";
    p += key;
    p += ". Find the PROJ 9.9.0 C++ under C:\\Users\\maxim\\AppData\\Local\\Temp\\PROJ-9.9.0\\src by searching for the +proj names. Read that C++ before writing Rust. ";
    p += "Vendored expectations are already in B:\\proj-rs\\tests\\gie\\ for these files: ";
    p += gie_names;
    p += ". ";
    p += hint;
    p += " Add a new module under src/projections/. Do not edit src/projections/mod.rs, src/ported.rs, src/lib.rs, or tests/gie/passing.txt. The integrator registers the module. ";
    p += "Write docs/PORTING-";
    p += key;
    p += ".md with the PROJ source file name and the numerical quirks you copied on purpose. ";
    p += "Also write PORT-NOTES-";
    p += key;
    p += ".md at the worktree root with the new file paths and the exact mod.rs and ported.rs lines the integrator should add. ";
    p += "Run git rev-parse --show-toplevel and cargo test --offline --test gie_run if the module is wired enough to compile. If it is not registered yet, cargo check --offline --tests is enough, and say so. ";
    p += "Final message must include the worktree path and the new .rs paths. ";
    p
}

fn integrate_prompt(notes) {
    let p = rules();
    p += "The parent workspace B:\\proj-rs already has uncommitted proj.db work. Do not revert it. ";
    p += "Port agents worked in separate worktrees. Copy only their new projection .rs files and docs/PORTING-*.md into B:\\proj-rs. Register each module in src/projections/mod.rs and add a building definition in src/ported.rs. ";
    p += "Their notes follow as JSON. Ignore a family whose notes are missing. ";
    p += notes;
    p += " Then run cargo test --offline --test gie_run -- --nocapture. ";
    p += "If failed is 0 and total passed is higher than tests/gie/passing.txt, set passing.txt to that live total. If anything failed, fix the port or leave passing.txt unchanged. builtins.gie must remain failed=0 skipped=0. ";
    p += "Run cargo fmt and cargo clippy --offline --all-targets --all-features -- -D warnings. Fix new warnings. Do not commit. ";
    p += "Final message: the new total passed, failed, and skipped, and which families you registered. ";
    p
}

fn verify_prompt() {
    let p = "Check the proj-rs workspace at B:\\proj-rs. Use read_file, grep, and cargo. Do not edit files. ";
    p += "Confirm data/proj.db exists and is a SQLite database built for this port, and that rusqlite opens it from the library rather than from a hand-written EPSG table. ";
    p += "Confirm Proj::new_known_crs for EPSG:4326 to EPSG:2154 still agrees with Lcc::lambert93. ";
    p += "Confirm a grid-backed or unknown EPSG code returns an error and not a coordinate. Read the code path. Name the function. ";
    p += "Run cargo test --offline --test gie_run -- --nocapture. builtins.gie must show failed=0 and skipped=0. tests/gie/passing.txt must equal the printed total passed, and failed must be 0. ";
    p += "Set ok true only if you saw those command results yourself. Put the totals in evidence. Otherwise ok false. ";
    p
}

phase("Database");
let db = agent(db_prompt(), #{
    label: "proj-db",
    capability_mode: "all",
});
if db == () || db.success != true {
    complete(#{
        summary: "proj.db step failed",
        ok: false,
    });
}
log("proj.db step returned");

phase("Port");
let families = [
    #{ key: "adams", gie: "adams_hemi.gie, adams_ws1.gie, adams_ws2.gie", hint: "These three Adams projections share one PROJ source. Port all three." },
    #{ key: "guyou", gie: "guyou.gie", hint: "Port +proj=guyou." },
    #{ key: "peirce", gie: "peirce_q.gie", hint: "Port +proj=peirce_q." },
    #{ key: "spilhaus", gie: "spilhaus.gie", hint: "Port +proj=spilhaus." },
];
let jobs = [];
for family in families {
    jobs.push(#{
        prompt: port_prompt(family.key, family.gie, family.hint),
        label: "port:" + family.key,
        capability_mode: "all",
        isolation_worktree: true,
        output_schema: port_schema,
    });
}
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
    log("port slots dropped: " + dropped.to_string());
}

phase("Integrate");
let integrated = agent(integrate_prompt(json_encode(notes)), #{
    label: "integrate",
    capability_mode: "all",
});

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
if integrated == () || integrated.success != true {
    ok = false;
}

let summary = "proj.db and projection port did not verify";
if ok {
    summary = "proj.db is internal and the new projections verified";
}
complete(#{
    summary: summary,
    ok: ok,
    evidence: evidence,
    families: 4,
});
