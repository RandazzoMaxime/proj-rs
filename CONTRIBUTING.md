# Contributing to PROJ-RS

PROJ-RS is open: issues and pull requests are welcome. Merging into the main
branch stays with the maintainer.

## Who merges

**Maxime Randazzo ([@RandazzoMaxime](https://github.com/RandazzoMaxime)) is the
only one who merges.** An approved review, a green CI or an agreement in the
issue does not merge a PR on its own. There is no automatic merge.

Nobody pushes directly to `main`. Even a one-line fix goes through a branch and
a pull request.

Two GitHub rulesets, `protect-main` and `protect-dev`, enforce these rules:

- no direct push: every change goes through a pull request;
- no force-push and no deletion of the branch;
- linear history, squash or rebase merge;
- no third-party approval required, the maintainer merges alone.

`CODEOWNERS` names `@RandazzoMaxime` on the whole repository. GitHub uses it
for review; it gives nobody else the right to merge.

## Proposing a lot

A lot is a pull request that carries one topic. The open topics are in
[ROADMAP.md](ROADMAP.md). Opening an issue "I take lot N" before coding avoids
two ports of the same module.

The branch starts from `main`. Its name says the lot: `lot-8-cart`.

## What a port PR contains

The port follows the
[rust-port](https://github.com/morganlinton/BeautifulRust/tree/main/skills/rust-port)
skill:

1. Read the PROJ source and list the observable behavior, failures and edge
   cases included.
2. Write the tests that characterize that behavior first. Expected values come
   from PROJ 9.9.0 (its sources, its tests, its CLI yaml files), never from the
   code under test.
3. Design the Rust API (`Result`, `Option`, enums). Do not transliterate error
   codes, `NULL`s or boolean parameters.
4. Keep the algorithm. A change of algorithm is justified in the PR.
5. Attach a `docs/PORTING-<module>.md`: map of functions and tests, deliberate
   deviations, quirks kept, commands run.
6. Add the lot's cases to `src/bin/bench.rs` and a line to
   `bench/history.jsonl` with `bench/record.sh`. Never rename an existing case:
   the history is indexed by name.
7. If the lot removes lines from `docs/UNPORTED-UNIT.md` or
   `docs/UNPORTED-CLI.md`, run `UPDATE_DOCS=1 cargo test --test docs_in_sync`.
   The counts in `README.md` and `ROADMAP.md` and the `docs/UNIT-TRIAGE.md`
   triage are computed from them, and the test fails if they drift.

There is no pull request template: the PR description covers the sections
above (behavior against PROJ, cases closed and reopened, deviations, commands
run).

## Working a lot without regressions

Several attempts failed for the same reasons. These rules come from them.

- **Measure before and after.** Run `cargo test --test cli_yaml` before any
  change. It runs our binaries against the outputs PROJ 9.9.0 wrote in
  `tests/cli_yaml/*.yaml`, offline, with no PROJ install. It rewrites
  `docs/UNPORTED-CLI.md` and fails when a case that used to pass breaks. It
  compares with the ledger file on disk, so restore it (`git checkout
  docs/UNPORTED-CLI.md`) before measuring after an experiment.
- **Compare with the real PROJ.** `tools/build-reference.sh` builds `cs2cs` and
  `projinfo` 9.9.0 from the tagged sources in about two minutes. Check the
  pair you change and a few neighbours against it, not only the yaml cases;
  when an existing test disagrees with it, say so in the PR instead of
  changing the test silently.
- **Zero reopened case.** A lot may close cases; it never reopens one, not even
  for a larger gain elsewhere.
- **One criterion at a time.** Change one thing (a ranking criterion, a lookup,
  a branch of the factory), then measure. Changing several at once hides which
  one broke what.
- **Port the C++ function whole.** Keep PROJ's branches, order and recursion.
  A simplified "one-level" version of a recursive function has regressed other
  cases each time it was tried. If a needed piece is missing, stop at a clean
  boundary and say so.
- **Two attempts, then a status sheet.** After two attempts that still regress,
  revert the code and commit only `docs/PORTING-<lot>.md` as a status sheet:
  what PROJ does, what the port does, the cause, the next step. That sheet is
  useful work: the next attempt starts from it.
- **Count failures exactly.** Read the totals of `cargo test --all-features`
  (passed, failed, ignored); do not trust a truncated or filtered output.

## Technical bar

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

No `unsafe` without a `// SAFETY:` comment and without a measured need or FFI.
No boolean parameter in a public signature. No `unwrap()` outside tests. The
lints are BeautifulRust's (`clippy.toml` and `[lints]` in `Cargo.toml`).

## Review

The maintainer looks, in this order, at: the behavior against PROJ, the
deviations stated in the `docs/PORTING-<module>.md` sheet, the API, then the
style. A PR that mixes two lots or widens its scope without saying so goes
back to its author.
