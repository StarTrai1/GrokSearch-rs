# Preserve structured source provenance

## Problem
A compatible gateway may stream a search answer and structured citations separately, then finish with a compact completion payload. The current MCP can lose the structured source records and incorrectly report a source-free primary answer. Existing live results establish repeated missing parsed sources, but do not prove whether the upstream omitted them or the client discarded them.

## Required behavior
- Retain supported structured Responses source metadata throughout stream reconstruction and parsing, including compatible nested citation objects.
- Preserve complete final-response text precedence, successful tool-source records, stable source ordering/deduplication, and existing documented inline citation compatibility.
- Do not promote arbitrary URLs, ordinary Markdown links, or uncited model claims into independently verified search evidence.
- A genuinely source-free upstream answer must continue using the existing explicit fallback/error behavior.
- Deliver the change through the user's GitHub fork, with CI-only regression tests, quality checks, build and downloadable Linux artifact, then install the checked artifact on WSL.
- Preserve unrelated configuration and user credentials. Remove the separately requested Smart Search installation and active skills, keeping a private configuration backup.

## Constraints
The user has already authorized fork, clone, implementation, commits, pushes, GitHub CI/CD, and WSL installation. No additional planning/commit confirmation is needed. All source tests, executable smoke checks, builds and toolchain installation run on GitHub only. WSL may edit/review source, operate Git, inspect existing evidence and verify artifact files/checksums; no local runtime probes or test compilation. No API keys or private gateway access are provided to CI.

## Acceptance
1. New regression cases fail at their intended assertions against pre-fix code on GitHub.
2. The fix passes format, clippy and tests for default and HTTP feature configurations on GitHub.
3. Known Responses citation events survive compact terminal and permitted end-of-stream forms, without duplicating or regressing complete final text.
4. Source-free/bare-link behavior remains explicitly unverified; no synthetic provenance is introduced.
5. A fork-only Linux release is gated by checks and identifies the exact commit, run and checksum.
6. Installed file hashes and configured command match that release. Runtime behavior against the user's gateway is not claimed as tested under the local-test prohibition.

