# Actual gateway diagnosis

Read-only records captured on 2026-09-25 from the running Windows AxonHub SQLite database using the already-installed Windows Python sqlite3 module, URI mode=ro and PRAGMA query_only=ON. No credentials or request headers are included in this repository. Private raw request/response projections are stored outside Git under the user's local state directory.

## Reproduction

The restarted MCP process was verified to execute the installed source SHA `3e20234220c4b4899b72fc7f4e26588466af281d`. Original benchmark S1 and S2 still returned `grok_sources_empty` and five fallback sources. The user stopped the remaining benchmark and requested repair; no score was extrapolated and the old npm installation remains installed.

## Confirmed mechanism

| Case | AxonHub request | Execution | Ingress | Outbound | Upstream reported usage |
| --- | --- | --- | --- | --- | --- |
| S1 | 45058 | 161984 | Responses, tools=[web_search] | Chat Completions, no tools | server-side tools=0, sources=0 |
| S2 | 45060 | 161986 | Responses, tools=[web_search] | Chat Completions, no tools | server-side tools=0, sources=0 |

Both upstream responses contain ordinary answer text and ordinary Markdown/plain URLs, without structured citations. Client-facing Responses annotations are empty. Therefore the installed GrokSearch parser is not discarding existing citations in these observed requests; truthful source fallback is expected.

The selected AxonHub channel is configured as OpenAI Chat with no custom endpoints. AxonHub's Chat converter filters every non-function tool (`llm/transformer/openai/outbound_convert.go`). Its native Responses transformer can preserve `web_search`. Adding a native Responses endpoint is a supported configuration operation that augments implicit defaults, but upstream capability must first be verified. This changes routing for all Responses requests on the channel, so unrelated model compatibility matters.

## Validation boundary

A native upstream Responses call initially met Cloudflare 1010 under Python's default User-Agent. A repeat using the recorded outbound User-Agent timed out before response headers after 100 seconds; this proves neither endpoint support nor source-bearing search. A narrower native streaming probe then succeeded in32.3s with actual web-search events and a structured official Python3.12 citation. The matching nonstreaming probe succeeded in20.9s with search calls and the same official citation; stream=false is supported. The upstream num_sources_used counter remained zero despite actual citation objects, so citation availability must be checked in the response structure, not inferred from that counter alone. No production database write or server restart has occurred. Do not change production routing based only on static capability declarations, and do not promote ordinary links to verified retrieval sources.

## YCE gates

- GrokSearch result: /tmp/yce-results/yce-search-20260925T114840-1108701-146ef4.xml; exit0, gate=true; bytes34312, SHA256 592f6c6102f7161e94ef6782c1dd5ab26cf00264f72edcb280d59f19ea1d30d1; full EOF read.
- AxonHub result: /tmp/yce-results/yce-search-20260925T115540-1145114-12fa18.xml; exit0, gate=true; bytes13505, SHA256 8ade31cf6d94242cfb60c8e6fc9ca3e9083a9ed15ea0b99af2d2e090c5989040; full EOF read.
