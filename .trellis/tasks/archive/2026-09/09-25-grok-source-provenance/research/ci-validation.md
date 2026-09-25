# GitHub validation evidence

All compilation, formatting-tool execution, tests and executable smoke checks are remote GitHub Actions operations. No such commands run on WSL.

## Regression red

- Test-first commit: `4761ff9f8a85d7c81d09c9efe0597baa5bc141a5`.
- Initial run [36127298432](https://github.com/StarTrai1/GrokSearch-rs/actions/runs/36127298432) stopped on fixture formatting only. It is not regression-red evidence.
- Formatting correction: `36af416e128a9fb763d425ea5f444102f105063c`; production source remained unchanged from upstream.
- Red run [36127592489](https://github.com/StarTrai1/GrokSearch-rs/actions/runs/36127592489) passed formatting and both default/http Clippy commands, then failed the default test suite in exactly the two relevant integration targets.
- `parses_nested_url_citation_annotations`: expected one source, got zero.
- `responses_sources_survive_compact_completion` and `responses_sources_survive_other_stream_endings`: expected structured tool sources, got no sources.
- `responses_final_text_wins_and_keeps_event_only_sources`: terminal sources survived but earlier tool/source metadata was lost.
- `responses_completed_parts_supply_text_without_deltas` and `responses_metadata_only_stream_retains_sources`: parser reported no text or sources.
- Ordinary-link and failed-terminal guard cases passed. Other default-feature test targets passed. The HTTP test step was skipped after the expected default-test failure; final green verification must run both variants.

## Implementation green

- Implementation `96e91a8483de7fb4eee46f38b9ba369ed7749ad0` passed both independent static reviews after scoped body-precedence and inline-citation compatibility follow-ups.
- Hosted formatting run `36129166759` requested four line-wrap changes. Hosted run `36129383482` then requested one collapsed match guard. These were applied manually; no local format/lint executable ran.
- Green SHA: `3e20234220c4b4899b72fc7f4e26588466af281d`.
- [CI run 36129597940](https://github.com/StarTrai1/GrokSearch-rs/actions/runs/36129597940): formatting, Clippy default/http, and tests default/http all passed. All six original failing regressions and additional text/compatibility guards passed in both test configurations.
- Immutable tag `fork-0.1.26-sources.1` points to that SHA. [Fork Release run 36129794292](https://github.com/StarTrai1/GrokSearch-rs/actions/runs/36129794292) reruns the shared check workflow before build/publication.

## Release and installation

- [Fork Release run 36129794292](https://github.com/StarTrai1/GrokSearch-rs/actions/runs/36129794292) succeeded: repeated full CI, Linux x86_64 musl release build, version check, credential-free MCP initialization, artifact packaging and publication.
- [Release fork-0.1.26-sources.1](https://github.com/StarTrai1/GrokSearch-rs/releases/tag/fork-0.1.26-sources.1) targets `3e20234220c4b4899b72fc7f4e26588466af281d`.
- The green main-branch run reports 338 passed / 2 ignored for default features and 353 passed / 2 ignored for HTTP features. The ignored tests are existing real Chat API E2E tests, not the new regressions.
- WSL downloaded only the release archive, metadata and checksum manifest. Release API digests, manifest checksums, build repository/SHA/ref/run/target/features and binary checksum matched. The archive contained one regular `grok-search-rs` file; its header matched Linux x86_64 ELF.
- Installed under `~/.local/lib/grok-search-rs/3e20234220c4b4899b72fc7f4e26588466af281d/grok-search-rs`, mode 0755, without executing it locally.
- Binary SHA256: `63c008279946c2c613394018c73ec9f719be7c4eb89e482103ee2e954da32cc0`.
- The Codex MCP command now names this absolute path. Structural configuration comparison/readback confirmed only the command changed during installation; tuned parameters and credentials were preserved. Prior npm launcher remains available for rollback.
- Smart Search global package, active config and active Codex/Claude skills were removed. Private rollback material remains under `~/.local/state/search-migration/20260925-184638` and is not committed.
- No build/test toolchain was installed locally; no local formatting/test/build/binary probe was run. A new MCP/client session is required to load the new command.

## Runtime boundary

CI establishes the source-handling correction and artifact startup, not a successful request through the user's actual gateway. Existing benchmark data does not establish the live gateway's original raw response shape.
