# Restore verifiable Grok sources on the actual gateway

## Goal

Restore genuine searchable, source-bearing answers through the configured GrokSearch-rs and AxonHub path, using actual recorded failure evidence.

## Requirements

- Stop the incomplete benchmark after S1/S2; do not run its remaining cases unless needed as explicit repair acceptance.
- Inspect AxonHub request/execution records read-only. Keep credentials and raw production evidence outside the public repository.
- Diagnose the responsible layer before changing code or configuration. Do not turn ordinary generated links or independent fallback sources into claimed model search provenance.
- Reuse the existing personal GitHub fork and local checkout. Any source changes must be reviewed, committed, validated and built in GitHub CI/CD, then install its verified artifact.
- No local build/test toolchain installation, formatting-tool execution, development tests or compilation. Targeted live diagnostic and repair-acceptance calls are now authorized by the user's retest/repair requests.
- Preserve current credentials and unrelated changes. Keep the old npm installation until the repaired runtime path is demonstrated working.

## Acceptance Criteria

- [x] Link the actual failure to recorded ingress, outbound request, upstream response and client-facing response.
- [x] Apply the smallest supported correction at the responsible layer, with an explicit source provenance contract.
- [x] No source change was needed for the live incident; reuse the existing installed SHA and its passed hosted CI/release.
- [x] A bounded live acceptance request returns real source evidence; merely suppressing grok_sources_empty is not success.
- [x] Document the evidence, limitations and installed configuration. Remove the old installation only after acceptance meets the user's condition.
