# Publishing to crates.io

Nothing is published yet. The crates are ready: versions `1.0.0`, `publish` allowed for
`proj-rs` and `proj-rs-georust`. This page is the procedure for the maintainer, who alone holds
the token. The GitHub release (archives, checksums) is separate: it is made by
`.github/workflows/release.yml` when a `v*` tag is pushed.

## Which crates

| Crate | Published | Why |
|---|---|---|
| `proj-rs` | yes | the engine and the native API |
| `proj-rs-georust` | yes | the drop-in for the georust `proj` crate; depends on `proj-rs` |
| `proj-rs-capi` | no (`publish = false`) | it builds a shared library named `proj_rs_capi`, which `capi/tests/dist.sh` renames to `libproj.so.25`; a crate cannot do that renaming, so crates.io would offer a library nobody can link under the name of libproj. The C library is delivered by the release archives. If a Rust project needs it as a crate, remove `publish = false` and publish it after `proj-rs` |

The names `proj-rs`, `proj-rs-georust` and `proj-rs-capi` were free on crates.io on 2026-11-18.
The first publication claims a name; check again just before.

## Before publishing

1. The tree is the tag `v1.0.0`, CI is green on it, and the three manifests carry the same version
   (`proj-rs-georust` depends on `proj-rs` by `path` and `version`; both are needed to publish).
2. The package of `proj-rs` stays under the 10 MiB limit of crates.io. The `exclude` list of
   `Cargo.toml` leaves out the parity corpus (`tests/parity`, 24 MB) and the repository tooling.
   Package it with verification, which builds the extracted package, so a missing file shows:

   ```sh
   cargo package -p proj-rs --allow-dirty    # drop --allow-dirty on a clean checkout
   ls -l target/package/proj-rs-1.0.0.crate  # 7.8 MiB at 1.0.0
   ```

   The packaged crate does not run the whole test suite: the tests that read `tests/parity` need a
   checkout of the repository.
3. A dry run of the real command, which does the same checks as the publication without sending
   anything:

   ```sh
   cargo publish -p proj-rs --dry-run
   ```

   `proj-rs-georust` cannot be dry-run before `proj-rs` is on the index: cargo looks `proj-rs 1.0.0`
   up on crates.io and does not find it. Check its contents with
   `cargo package -p proj-rs-georust --list` and dry-run it after step 2 of the next section.

## Publishing

1. Log in with a token made on <https://crates.io/settings/tokens>, with the scopes `publish-new` and
   `publish-update`, limited to these crates once they exist. `cargo login` stores it in
   `~/.cargo/credentials.toml`; or export `CARGO_REGISTRY_TOKEN` for one command. The token is never
   written in the repository, in a workflow file or in a log.
2. In this order, `proj-rs` first, because the second depends on it:

   ```sh
   cargo publish -p proj-rs
   cargo publish -p proj-rs-georust --dry-run   # now it resolves
   cargo publish -p proj-rs-georust
   ```

   `cargo publish` waits until the index shows the new version, so the second command finds it.
3. After the publication: read the pages on <https://docs.rs/proj-rs> and
   <https://docs.rs/proj-rs-georust> once docs.rs has built them; turn on the `semver` job of
   `.github/workflows/ci.yml` (remove its `if: false`; it then compares each crate with the latest
   release on the registry); in `README.md`, replace "PROJ-RS is not on crates.io yet. Use it from
   git" by the `proj-rs = "1"` line.

## What cannot be undone

- A published version is permanent. It cannot be edited or overwritten, and its number cannot be
  used again; crates.io removes a crate only in the narrow cases of its deletion policy
  (<https://crates.io/policies>). A mistake is repaired by a new version (`1.0.1`).
- `cargo yank --version 1.0.0 proj-rs` only stops new projects from resolving to that version;
  projects that already have it in their `Cargo.lock` keep building. `cargo yank --undo` reverts it.
  The code stays public and downloadable.
- The licence terms of what the crate contains are published with it (`LICENSE`, `NOTICE.md`,
  `data/`). Check them before the first publication, not after.
- The crate name belongs to its owners; `cargo owner --add <user> proj-rs` changes who they are.
