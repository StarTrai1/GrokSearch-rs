# Execution

- [x] Uninstall Smart Search CLI and active Codex/Claude skills; preserve private backup.
- [x] Fork upstream to StarTrai1/GrokSearch-rs, clone under ~/chat and pass/read the YCE code gate.
- [x] Inspect source and release contracts; record the confirmed metadata-loss mechanism and live-evidence limit.
- [x] Apply the bounded MCP parameter changes without changing credentials.
- [ ] Add focused regression tests only, plus CI/fork-release workflow support; review the diff.
- [ ] Commit/push the test-first state to the authorized fork and confirm named assertion failures in GitHub CI, not setup/format/build failures.
- [ ] Implement Responses reconstruction/adapter corrections with no local tests or builds.
- [ ] Run independent standards/spec reviews; incorporate justified findings.
- [ ] Commit/push the fix and require all default/http CI checks on that exact SHA.
- [ ] Tag the green SHA with fork-*; require check-gated GitHub artifact release, checksums and build metadata.
- [ ] Download and inspect archive, compare SHA256 and commit/run identity, install at a versioned absolute path, update MCP command.
- [ ] Preserve red/green/release evidence and update task records; report required MCP restart and runtime-verification boundary.

Rollback uses the saved original MCP command and npm package plus the configuration backup. No local Cargo, Rust installation, test runner, binary version execution or MCP smoke is authorized. Package parsing, diff review, archive listing and checksum checks are file-integrity operations only.
