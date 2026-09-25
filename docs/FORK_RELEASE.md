# Personal fork releases

`StarTrai1/GrokSearch-rs` publishes a Linux x86_64 musl **stdio** binary through
the **Fork Release** workflow. The inherited upstream version-bump and release
workflows are disabled on this fork, including npm publication, container
publication, and automatic manifest synchronization.

## Validate and publish

Push reviewed source to the fork. **CI** runs formatting plus clippy/tests with
both default and `http` features. It also supports manual dispatch. For a
regression fix, preserve a test-first GitHub run showing the intended assertion
failure, then a passing run for the implementation commit. An unrelated setup,
formatting, or compilation failure is not regression evidence.

Create a unique `fork-*` tag on the intended commit and push it to the fork.
**Fork Release** reruns the same CI workflow on that exact SHA, then builds only
`x86_64-unknown-linux-musl` with default features and `--locked`. Binary version
and credential-free MCP initialization smoke checks run on GitHub. No real API
keys are needed, and these checks do not contact a search gateway.

Successful tagged runs publish a GitHub prerelease with:

- `grok-search-rs_Linux_x86_64.tar.gz`: the executable.
- `build-info.json`: repository, commit, ref, run identity, target, package version,
  feature set, and executable SHA256.
- `SHA256SUMS`: archive and metadata checksums.

Manual dispatch on a branch runs the same checks/build and uploads the
`fork-linux-x86_64` Actions artifact, without creating a release. Dispatch on a
`fork-*` tag can also publish its release. Crate/npm version manifests remain
unchanged; the commit and hashes identify each fork build.

## Install on WSL without local tests

Download all three files from the explicit successful tag/run. Compare the
release target and `build-info.json` commit with the intended source SHA. In the
download directory, check artifact integrity and inspect archive contents:

```sh
sha256sum -c SHA256SUMS
tar -tzf grok-search-rs_Linux_x86_64.tar.gz
```

The archive contains only `grok-search-rs`. Extract it into a temporary directory,
compare its SHA256 with `binary_sha256` in `build-info.json`, and install it with
mode `0755` in a versioned user-owned directory such as
`~/.local/lib/grok-search-rs/<commit>/grok-search-rs`. Retain the metadata and
checksums alongside the installation. Set the MCP command to the expanded
absolute executable path; leave credentials and unrelated settings unchanged.

Preserve the previous configured command and npm installation for rollback.
Do not replace the npm-managed launcher or its platform package. Verify the
installed file hash and configured path without executing the binary. All
tests, smoke checks, builds, and build-toolchain installation belong on GitHub;
do not run Cargo, `--version`, or an MCP smoke harness locally.

Restart the MCP process/client session to load the new command. Roll back by
restoring the previous command and restarting it. CI fixtures and startup smoke
checks establish software behavior and artifact viability; they do not verify
the user's gateway responses.
