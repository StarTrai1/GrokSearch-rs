# GitHub-only validation and personal-fork release

## Scope and evidence

Research is based on repository files only. No local tests, builds, toolchain
installation, executable probes, remote jobs, or host configuration inspection were
performed. The required destination is the user's Linux x86_64 WSL installation;
the parent agent owns commit/push, GitHub execution, installation, and MCP config.

| Evidence | Finding |
| --- | --- |
| `.github/workflows/ci.yml:3-47` | PRs and pushes to `main` run formatting, clippy for default/http, and tests for default/http. Cargo lint/test commands already use `--locked`. There is no manual or reusable-workflow trigger. |
| `.github/workflows/release.yml:3-155` | Tags `v*` or manual dispatch build seven artifact variants. The build has no dependency on the CI checks. Linux x86_64 already uses `x86_64-unknown-linux-musl`, installs musl tools on GitHub, and archives the single binary. |
| `.github/workflows/release.yml:94-112` | Tag builds rewrite the crate version and run `cargo update`; these build-time mutations weaken a simple exact-commit artifact identity. A personal-fork path does not need them. |
| `.github/workflows/release.yml:157-204` | Tagged builds create GitHub releases with archives and `SHA256SUMS`; manually dispatched builds only upload build artifacts. Checksums currently occur in the release job, after the builds. |
| `.github/workflows/release.yml:208-265` | GHCR publishing is **not hardcoded to upstream**: the image is `ghcr.io/${{ github.repository_owner }}/grok-search-rs`. It is nevertheless unnecessary for the requested stdio installation. |
| `.github/workflows/release.yml:267-441` | Npm publishing and source-manifest synchronization are tied to upstream release conventions. The sync job commits/pushes version changes back to `main`. These jobs should not run for this fork. |
| `.github/workflows/bump.yml:17-133` | The optional bump workflow rewrites Cargo/npm versions and creates/pushes `v*` tags. It is not needed for a commit-identified fork release. |
| `Cargo.toml` | Crate version is `0.1.26`. The default binary is stdio; `http` is opt-in and enables axum/network/multithread runtime. Default release uses `panic = "abort"`; the separate HTTP release profile uses unwind. |
| `npm/grok-search-rs/package.json`, `npm/platforms/linux-x64/package.json`, `npm/grok-search-rs/run.js` | The wrapper package is `grok-search-rs`, with platform packages under `@epsiekygrr_zedxx`. The executable entry is a JS launcher selecting the optional platform dependency. Publishing these names from the fork is outside scope. |
| `src/main.rs:28-39` | `--version` reports only `CARGO_PKG_VERSION`, so it alone cannot distinguish a fork commit retaining the upstream version. |

## Recommended workflow changes

Keep upstream packaging intact but separate it from the narrowly required fork
artifact. No npm renaming, npm publication, GHCR image, cross compilation to ARM,
macOS/Windows artifact, or automatic version synchronization is required.

1. Add `workflow_dispatch` and `workflow_call` to the existing CI workflow. Reuse
   its existing five commands without reducing coverage:

   ```text
   cargo fmt --check
   cargo clippy --locked --all-targets -- -D warnings
   cargo clippy --locked --all-targets --features http -- -D warnings
   cargo test --locked
   cargo test --locked --features http
   ```

   Include `${{ github.workflow }}` in its concurrency group, for example
   `ci-${{ github.workflow }}-${{ github.ref }}`. A direct CI run and a reusable
   call from the fork-release workflow can otherwise share `ci-<ref>` and cancel
   one another. Keep cancellation within repeated runs of the same workflow/ref.

2. Add `.github/workflows/fork-release.yml`, restricted at job level to
   `github.repository == 'StarTrai1/GrokSearch-rs'`, with manual dispatch and a
   distinct tag pattern such as `fork-*`. Use a `check` job calling
   `./.github/workflows/ci.yml`; make the build job depend on `check`. This gates
   the **same checked-out commit** being packaged, regardless of whether a
   separate branch CI status exists.

3. The fork build runs only on GitHub `ubuntu-latest`: install Rust stable with
   `x86_64-unknown-linux-musl`, install `musl-tools`, optionally reuse the existing
   cargo-cache action, and execute:

   ```text
   cargo build --locked --release --target x86_64-unknown-linux-musl
   ```

   Use no `--features http` for the delivered binary. Keep package version and
   lockfile unchanged; distinguish the fork by tag/commit metadata. Toolchain
   installation occurs only on the GitHub runner.

4. Package `target/x86_64-unknown-linux-musl/release/grok-search-rs` as
   `grok-search-rs_Linux_x86_64.tar.gz`. Add `build-info.json` containing repository,
   full commit SHA, ref/tag, workflow run ID, target, and `default` feature set.
   Generate `SHA256SUMS` for the archive and metadata. Do not use a `*.zip` glob:
   this path produces no ZIP and a literal unmatched argument would fail
   `sha256sum`. Upload all three files with `actions/upload-artifact@v4` and
   `if-no-files-found: error` for both manual and tagged runs.

5. For a `fork-*` tag only, publish those files to a GitHub release of the fork,
   using an explicit tag/target SHA and a release body linking the exact workflow
   run and source commit. Prefer a prerelease for a personal fork. Only this job
   needs `contents: write`; check/build jobs need `contents: read`. No npm token,
   GitHub package permission, or OIDC permission is necessary. A manual branch
   run can stop at a downloadable Actions artifact.

6. Add an explicit upstream repository guard to inherited `release.yml`'s root
   `build-release` job so `v*` tags/manual dispatch on the fork cannot start the
   old matrix. Also make npm publish/image/sync job guards explicitly upstream
   only, preserving their existing event conditions. Their current dependencies
   would skip them when the root build is skipped, but explicit publication
   guards make the ownership boundary clear. Guard `bump.yml`'s sole job to
   upstream too, since this fork uses commit-identified `fork-*` tags and has no
   reason to mutate upstream npm manifests automatically.

This is a repository-specific release path, not a generalized release framework.
Avoid copying the complete CI command list into the new workflow, renaming npm
packages, or introducing a local release script/toolchain.

## GitHub-only regression red/green evidence

The least ambiguous TDD sequence uses a test-first commit:

1. Commit the focused regression tests and the CI manual/reusable trigger change,
   leaving affected production behavior unchanged. The tests must use existing
   public APIs so the old code compiles and reaches the intended assertions.
2. Push this commit to the authorized fork branch and manually dispatch CI on
   that branch. Manual dispatch must be available from the repository's default
   branch; if it is not yet available there, first land the workflow-only change
   on `main`, or use the existing PR trigger for the test-first branch. Do not
   assume a non-main branch push triggers the current CI.
3. Inspect GitHub logs for the exact new failing test names, assertions, and
   fixture values. A formatting, compilation, dependency, or runner failure is
   not a demonstrated regression red result. Correct such setup issues first.
   The red test should assert the intended behavior, not expect an error merely
   to manufacture a failure.
4. Implement the behavior in a later commit, push, and require all existing CI
   commands to pass. Save both commit SHAs and both run URLs in task evidence.
5. Tag the successful implementation SHA with a unique `fork-*` tag and let the
   fork release rerun the shared checks before building. This final run supplies
   artifact provenance even if the branch later moves.

If test-first commits are operationally unsuitable, a GitHub job can check out a
fixed pre-fix commit, apply only the new test fixture/test patch, run the named
tests, and assert that they fail for the expected assertions. That is more
workflow machinery and is unnecessary when a normal test-first commit can run.
Never treat arbitrary nonzero Cargo status as proof of the original defect.

The current CI stops at the first failure. A red default-feature regression is
sufficient evidence of the defect; the final green run still must include both
default and `http` feature variants. Running optional binary `--version` or MCP
initialize smoke checks belongs on the GitHub runner under this user's ban on
local tests, not on WSL.

## Installation and evidence allowed on WSL

After the exact release workflow succeeds, the parent can download the archive,
metadata, and checksum from that explicit tag/run. Verify the release commit and
metadata SHA against the intended commit, run `sha256sum -c SHA256SUMS`, inspect
the archive member list, and extract to a temporary installation directory.
These are artifact-integrity/file checks; they do not compile or execute tests.

Install into a dedicated, versioned user-owned location such as
`~/.local/lib/grok-search-rs/<full-sha>/grok-search-rs`, set executable permission,
and point the MCP command at its absolute path (or atomically move an owned
`current` symlink). Preserve the previous configured command and rollback target.
Do not overwrite the npm-managed launcher at
`/home/victel/.nvm/versions/node/v24.14.1/bin/grok-search-rs`: a future npm update
could replace it and obscure which binary is active. Do not copy or print keys.

Verify installed file hashes, symlink targets, mode, and configured executable
path without launching the program. `--version` cannot establish the fork SHA
anyway; metadata plus checksum and exact GitHub run are the provenance evidence.
Respect the strict local-test prohibition: no local Cargo command, test harness,
MCP smoke script, Rust installation, or binary execution as an installation test.
Normal subsequent use of the MCP remains distinct from development validation;
do not claim live gateway behavior was validated by CI fixtures or file checks.

## Remaining prerequisites and limits

- The parent must verify that fork Actions is enabled and that its token can push
  commits/tags and create releases. No remote settings or credentials were read
  in this subtask.
- The WSL architecture must match the requested x86_64 artifact. The task states
  x86_64; this research did not probe the host.
- The precise regression assertions depend on the code/source-provenance
  investigation. This note supplies their execution path, not root-cause proof.
- CI tests must remain offline fixtures or local mock servers on GitHub; no
  private gateway or API credentials should be injected merely to validate this
  fix. CI cannot establish what an external gateway omitted before delivery.
- A checksum verifies artifact integrity, while matching tag, commit, run, and
  build metadata establishes provenance. It is not an independent signature.
- Project backend quality documents are placeholders and add no extra command
  requirements; the existing CI is the concrete required check set.
