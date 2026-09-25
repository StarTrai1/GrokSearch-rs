# Native Responses acceptance and old package removal

Date: 2026-09-25. Scope: one focused installed-MCP search after the user changed the AxonHub channel configuration. The user cancelled the remaining full benchmark; no new aggregate benchmark score is claimed.

## Runtime evidence

- Installed executable: source SHA `3e20234220c4b4899b72fc7f4e26588466af281d`, binary SHA256 `63c008279946c2c613394018c73ec9f719be7c4eb89e482103ee2e954da32cc0`.
- The active MCP processes resolve to this versioned executable, not the npm launcher.
- User-saved channel: type `openai_responses`, endpoints=[]; database updated at 2026-09-25T12:14:40.8291144Z.
- Query: `Use web_search to find the official Python 3.12 asyncio.timeout documentation. In one sentence state where TimeoutError is caught and give the source citation.`
- MCP call used query only and server defaults; wall time 43.492s.
- Result: `search_provider=grok_responses`, `fallback_used=false`, `fallback_reason=null`, `truncated=false`.
- Answer correctly places TimeoutError handling outside the asyncio.timeout context manager and cites https://docs.python.org/3.12/library/asyncio-task.html.
- Exact database correlation: request `45141`, execution `162067`, same requested/upstream Grok model. Both ingress and outbound use `openai/responses`, stream=false and tools=[web_search]; pass_through_applied=1.
- Upstream and client-facing response both contain completed status, web_search_call items and the same structured official Python `url_citation`. This establishes end-to-end provenance preservation, not just an HTTP success.
- The response contained 23 tool-search candidate URLs, one separately cited official URL and three supplemental sources (27 total). Candidate count is not citation quality or independent verification; source ordering and the entire benchmark were not rescored.
- Upstream num_sources_used remained zero despite explicit citations. The acceptance uses actual source objects/events, not that counter.

## Installation cleanup

After acceptance, `npm uninstall -g --ignore-scripts grok-search-rs` removed the old 0.1.26 package and its platform package. The old nvm launcher and all matching global npm package manifests are absent.

`~/.local/bin/grok-search-rs` now points to the installed versioned executable. Both PATH resolution and the Codex MCP absolute command resolve to that same file, whose checksum is unchanged. Current working MCP processes already run the new file.

No local toolchains, developer tests, formatting tools or compilation were installed/run. No source or production database rows were modified by the assistant. The user saved the routing configuration through AxonHub's UI. Sensitive/raw request evidence remains in the private local state directory; only this sanitized summary is committed.
