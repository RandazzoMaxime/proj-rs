let meta = #{
    name: "port-remaining",
    description: "Port the remaining proj-rs operations: formulas, test grids, leftover EPSG codes, WKT and the CLIs",
    when_to_use: "After builtins.gie is green, port every still-skipped gie operation that does not need an external PROJ-data grid.",
    phases: [
        #{ title: "Port", detail: "Formula and grid families, each in its own worktree" },
        #{ title: "Integrate", detail: "Register modules and bump the gie ratchet" },
        #{ title: "Surface", detail: "Leftover EPSG codes, WKT, PROJJSON, cs2cs and cct" },
        #{ title: "Verify", detail: "Fail closed unless builtins stays green and passing.txt matches" },
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
        "passes": #{ "type": "string" },
    },
};

fn rules() {
    let p = "Repo B:\\proj-rs, branch dev. Do not commit, push, or merge to main. Do not revert uncommitted work already in the tree: data/proj.db, src/projdb.rs, Adams, Guyou, Peirce, Spilhaus, and tests/gie/passing.txt. ";
    p += "Rust edition 2021, rust-version 1.83. Prepend C:\\Users\\maxim\\.cargo\\bin to PATH. cargo --offline after any fetch. ";
    p += "build.rs compiles geodesic.c with MSVC cl through a .bat file. Do not switch that to mingw. No GPU path. A CPU path is never a production fallback for a GPU kernel. ";
    p += "Unknown +proj or a parameter is an error at creation, never a coordinate. Do not loosen .gie tolerances. Do not edit the vendored .gie files. ";
    p += "tests/gie/passing.txt equals the live pass count. Change it only after cargo test --offline --test gie_run prints failed=0 and a higher total passed. builtins.gie must stay failed=0 skipped=0. ";
    p += "The runner skips a case tagged require_grid before transforming. Leave that behavior. Do not download the PROJ-data CDN. ";
    p += "Clippy: cargo clippy --offline --all-targets --all-features -- -D warnings. pedantic is allow. No as cast without an expect and a reason. ";
    p += "Classic kernels return units of the semi-major axis. The wrapper multiplies by a. k0 belongs inside the kernel only where PROJ reads P->k0. Read src/engine.rs and src/projections/laea.rs before writing. ";
    p += "PROJ 9.9.0 sources are at C:\\Users\\maxim\\AppData\\Local\\Temp\\PROJ-9.9.0. Read the C++ before writing Rust. ";
    p
}

fn port_prompt(key, scope) {
    let p = rules();
    p += "You are in a git worktree. Port only this family: ";
    p += key;
    p += ". ";
    p += scope;
    p += " Add modules under src/projections/ or the engine. Do not edit src/projections/mod.rs, src/ported.rs, src/lib.rs, or tests/gie/passing.txt. The integrator registers them. ";
    p += "Write docs/PORTING-";
    p += key;
    p += ".md naming the PROJ source file and the quirks you copied. Write PORT-NOTES-";
    p += key;
    p += ".md at the worktree root with new paths and the mod.rs plus ported.rs lines to add. ";
    p += "Run git rev-parse --show-toplevel. cargo check --offline --tests is enough if the module is not registered yet. ";
    p += "Final message must include the worktree path and the new file paths. ";
    p
}

fn integrate_prompt(notes) {
    let p = rules();
    p += "Work in the parent workspace B:\\proj-rs. Copy new .rs files, docs/PORTING-*.md, and any small vendored test grids from the worktrees into the parent. Do not delete the proj.db work or the Adams, Guyou, Peirce, and Spilhaus modules. ";
    p += "Register every new operation in src/projections/mod.rs and src/ported.rs. Notes JSON follows. Skip a family with no notes. ";
    p += notes;
    p += " Run cargo test --offline --test gie_run -- --nocapture. If failed is 0 and total passed is higher than tests/gie/passing.txt, write that total into passing.txt. builtins.gie stays failed=0 skipped=0. ";
    p += "Run cargo fmt and cargo clippy --offline --all-targets --all-features -- -D warnings. Fix new warnings. Do not commit. ";
    p += "Final message: total passed, failed, skipped, and the families you registered. ";
    p
}

fn surface_prompt() {
    let p = rules();
    p += "Work in B:\\proj-rs, not a worktree. The integrator has just registered the new operations. ";
    p += "Finish three surfaces. ";
    p += "First, epsg_no_grid.gie still skips EPSG:4979, EPSG:7843, and EPSG:4123. Resolve them from data/proj.db through the existing rusqlite lookup. Emit a pipeline only when every step is already built. A grid step returns ProjCreateError. ";
    p += "Second, Proj::as_wkt and Proj::to_projjson currently return an error. For an EPSG code loaded from proj.db, return the WKT or PROJJSON stored in that database when the column exists. Parse a minimal WKT or PROJJSON in Proj::new only for geographic and projected CRS whose conversion method is already built. Anything else is an error. ";
    p += "Third, add binaries cs2cs and cct under src/bin that transform coordinates with the library. Cover a +proj string and one EPSG pair with tests. Do not vendor PROJ's test/cli suite. ";
    p += "Re-run cargo test --offline --test gie_run -- --nocapture and update passing.txt only under the same rule. cargo fmt and clippy -D warnings. Do not commit. ";
    p
}

fn verify_prompt() {
    let p = "Check B:\\proj-rs. Use read_file, grep, and cargo. Do not edit files. ";
    p += "Run cargo test --offline --test gie_run -- --nocapture. builtins.gie must show failed=0 and skipped=0. tests/gie/passing.txt must equal the printed total passed, and failed must be 0. ";
    p += "Read the new modules and confirm unitconvert, hourglass, interrupted, eqearth, affine, hgridshift or vgridshift, and tinshift or defmodel exist as operations. ";
    p += "Confirm a missing grid file or an unknown +proj is an error, not a coordinate. Name the function you read. ";
    p += "Set ok true only if you saw the gie command output yourself. Put the totals in evidence. Otherwise ok false. ";
    p
}

phase("Port");
let families = [
    #{
        key: "conversions",
        scope: "Port +proj=unitconvert, +proj=noop, +proj=push, +proj=pop, and +proj=set from PROJ 9.9 conversions. tests/gie/unitconvert.gie and the push/pop/set cases in tests/gie/4D-API_cs2cs-style.gie are the oracle.",
    },
    #{
        key: "hourglass-interrupted",
        scope: "Port +proj=hourglass and +proj=interrupted. Oracles: tests/gie/hourglass.gie and tests/gie/interrupted.gie. Interrupted bases sinu, tmerc, cass, and poly are already built. Reuse them.",
    },
    #{
        key: "sphere-ops",
        scope: "Port +proj=eqearth and +proj=bertin1953. Oracles are the eqearth and bertin1953 blocks in tests/gie/more_builtins.gie.",
    },
    #{
        key: "datum-ops",
        scope: "Port +proj=molodensky, +proj=molobadekas, and Helmert +exact. Also accept +proj=webmerc as the spherical Mercator alias and +lon_wrap. Oracles are those blocks in tests/gie/more_builtins.gie and tests/gie/GDA.gie. Do not invent a grid shift.",
    },
    #{
        key: "affine-offsets",
        scope: "Port +proj=affine, +proj=geogoffset, and +proj=vertoffset, including omit_fwd and omit_inv on a pipeline step. Oracles are those blocks in tests/gie/more_builtins.gie and tests/gie/4D-API_cs2cs-style.gie.",
    },
    #{
        key: "spheroid",
        scope: "Port +proj=geoc and the spherification radii R_A, R_V, R_h, R_lat_a, and R_lat_g. R_C already exists and must keep ignoring lat_0. Oracle: the remaining skips in tests/gie/ellipsoid.gie and the geoc block in tests/gie/more_builtins.gie.",
    },
    #{
        key: "gridshift",
        scope: "Port +proj=hgridshift, +proj=vgridshift, and +proj=gridshift for the small GeoTIFF and NTv2 files that already exist under C:\\Users\\maxim\\AppData\\Local\\Temp\\PROJ-9.9.0. Vendor those test files next to the gie tests. If a named grid is not in that tree, creation returns an error and the case may stay skipped. Do not download PROJ-data. Oracles: tests/gie/gridshift.gie, tests/gie/geotiff_grids.gie, and the hgridshift/vgridshift blocks in more_builtins.gie.",
    },
    #{
        key: "models",
        scope: "Port +proj=deformation, +proj=defmodel, and +proj=tinshift. Vendor only the small JSON and model files the gie cases name when they exist in the PROJ 9.9.0 tree. Missing files stay an error. Oracles: tests/gie/deformation.gie, tests/gie/defmodel.gie, tests/gie/tinshift.gie, tests/gie/tinshift_gpkg.gie.",
    },
    #{
        key: "nkg",
        scope: "Port the NKG coordinate-operation URNs used by tests/gie/nkg.gie. Read them from proj.db or from the PROJ 9.9.0 source. A URN whose steps are built operations must transform. A URN that needs a grid returns an error.",
    },
];

let jobs = [];
for family in families {
    jobs.push(#{
        prompt: port_prompt(family.key, family.scope),
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
if integrated == () || integrated.success != true {
    complete(#{
        summary: "integrate failed",
        ok: false,
        evidence: "",
        families: 9,
    });
}

phase("Surface");
let surface = agent(surface_prompt(), #{
    label: "surface",
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
if surface == () || surface.success != true {
    ok = false;
}

let summary = "remaining port did not verify";
if ok {
    summary = "remaining operations verified";
}
complete(#{
    summary: summary,
    ok: ok,
    evidence: evidence,
    families: 9,
});
