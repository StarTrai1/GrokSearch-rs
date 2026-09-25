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

## Pending

Implementation green, independent review and tagged release evidence will be recorded after the corresponding GitHub jobs finish. Existing benchmark data does not establish the live gateway's original raw response shape.
