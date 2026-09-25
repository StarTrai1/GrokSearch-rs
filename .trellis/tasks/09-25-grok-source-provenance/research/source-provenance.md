# Responses source provenance research

Date: 2026-09-25. Scope: read-only code, existing benchmark artifacts, and official documentation. No local tests, compilation, toolchain installation, gateway requests, or search probes were run. The only repository change from this research is this file.

## Conclusion

There is a confirmed source-loss mechanism in the Responses SSE decoder. It accumulates text deltas but not Responses annotation/content-part/output-item metadata. Compact terminal frames, `[DONE]`, named completion, or EOF therefore reduce an otherwise source-bearing stream to text alone. A full final response also replaces earlier event-only metadata instead of merging it. This defect can produce exactly the service-level `grok_sources_empty` result.

The four recorded benchmark outputs do **not** prove this was their live cause. They contain normalized MCP results, not raw gateway JSON/SSE, so the alternatives remain: no tool execution, upstream metadata stripping, an unrecognized citation shape, or local SSE source loss. The release can truthfully claim to preserve specified Responses source shapes; it cannot claim the configured gateway's live searches are repaired without new upstream evidence.

Recommended scope: preserve structured Responses metadata across streaming completion, accept the nested `url_citation` compatibility shape already understood by the Chat Completions adapter, retain existing inline-citation compatibility, and keep the current truthful source fallback. Do not introduce an answer-generation fallback mode, switch transport speculatively, or recognize arbitrary model-authored URLs.

## Existing evidence

The existing files `/home/victel/TEST/search-benchmark-20260925-171247/raw/S1-grok.json` through `S4-grok.json` each report:

- `fallback_reason: grok_sources_empty`
- `fallback_used: true`
- `search_provider: source_fallback`
- `sources_count: 3`
- A diagnostic saying that the original answer had no verifiable search sources and was not treated as verified.

Their source lists originate from the separate source provider. They demonstrate working retrieval fallback and the absence of sources accepted by the main-response parser. They contain no upstream response body or event history.

The parent supplied the configured gateway/model/transport: `http://localhost:8090/v1`, `grok-4.20-multi-agent-0309`, Responses. No credential files were read. The parent also reports previous SmartSearch Chat Completions text claiming live tools were unavailable. Such model text is supporting context, not authoritative transport capability evidence.

## Current request and service behavior

`src/adapters/grok_responses_request.rs:37-50` sends `tools: [{type: web_search}]` when enabled and optionally `x_search`, plus `stream: false`. It has no `tool_choice` or `include`. `require_web_search` checks local intent; the name does not mean the gateway is forced to execute a tool.

The xAI [Web Search documentation](https://docs.x.ai/developers/tools/web-search) shows the same basic Responses tool payload. The [Citations documentation](https://docs.x.ai/developers/tools/citations) states that Responses inline citations are enabled by default; `include: ["inline_citations"]` is the Python SDK/gRPC opt-in, not a missing Responses requirement. It also documents flat `output_text.annotations` with `type: url_citation`, `url`, offsets, and a numeric citation label. Existing JSON parsing already handles that documented flat shape and discards junk numeric titles.

Therefore absence of `include: ["inline_citations"]` is not an established defect. Forcing tools or adding provider-specific include options should wait for a demonstrated gateway contract; neither manufactures provenance when a relay lacks tool execution. Official xAI behavior cannot be assumed for an arbitrary compatible gateway.

`src/service.rs:778-783` runs the AI request concurrently with separate retrieval; the AI request receives an empty extra-source slice. Supplementary source results are consequently not evidence that the AI answer used those documents. `src/service.rs:804-807` correctly gates before merging the supplementary results. `src/service.rs:2150-2157` identifies nonempty text with no accepted sources as `grok_sources_empty`.

`src/service.rs:972-1089` then retains actual source-provider provenance, omits the unverified original prose, labels the result `source_fallback`, caches the sources, and fails loudly if no usable sources survive. Preserve this behavior. Relabeling supplementation as Grok citations would be false provenance.

## Confirmed adapter and stream gaps

| Location | Confirmed behavior | Consequence |
| --- | --- | --- |
| `src/providers/http.rs:236-239` | SSE decoding runs even though the request set `stream: false`. | The streaming defect remains relevant to this configuration. |
| `src/providers/http.rs:263-268`, `396-397` | State retains text, last JSON, and Chat metadata, but has no Responses metadata accumulator. | Annotation/content-part/output-item events are overwritten as `last_json` advances. |
| `src/providers/http.rs:406-415` | `response.completed.response` is returned unchanged; absent that object, accumulated text becomes `{output_text: ...}`. | Earlier structured evidence disappears; a compact response object can also displace accumulated text. |
| `src/providers/http.rs:598-615` | Nonempty Responses text wins over all other retained JSON on generic termination. | `[DONE]`, named completion, and EOF discard metadata even when some was present in earlier frames. |
| `src/adapters/grok_responses_response.rs:78-90` | Structured objects must expose `url` or `uri` directly. | A compatible gateway's `{type: url_citation, url_citation: {url, title}}` is silently dropped. |
| `src/adapters/chat_completions_response.rs:125-127` | Chat parsing already unwraps that nested shape. | Reusing the supported shape for Responses is a bounded compatibility fix. |

Event paths requiring preservation are `response.output_text.annotation.added` (`annotation`), `response.content_part.added/done` (`part.annotations` / `part.citations`), and `response.output_item.added/done` (`item.content[*].annotations/citations` and `web_search_call.action.sources`). Terminal response snapshots may repeat some or all of them.

These are statically established loss paths; they are not evidence that the configured gateway actually emits those event types. The official flat, complete, non-streaming xAI JSON path is already supported.

Responses also ignores top-level `search_sources`, whereas Chat explicitly documents its use by some compatible gateways. No benchmark evidence shows that shape on this Responses endpoint, so it need not be added in this change. Avoid a recursive scan of arbitrary JSON fields for URLs.

## Merge design

Keep the implementation within the existing HTTP SSE normalization and response-adapter seams. A small Responses accumulator should retain structured source metadata and enough text snapshots to reconstruct a coherent answer; it need not become a general protocol emulator.

1. Accumulate only known protocol source locations. Preserve annotation objects and web-search source objects as metadata, not URLs harvested from prose, tool arguments, or error text.
2. Retain the latest item/part snapshot by output/content index or stable item identity. `added` and `done` often repeat the same material. Do not append their complete text to already accumulated deltas.
3. At all completion exits, use one finalization path. Prefer a complete terminal response's text. Otherwise use available completed item/part text, falling back to text deltas where snapshots are absent. A compact terminal response must not erase previously received text.
4. Merge terminal structured sources with previously streamed evidence, including sources absent from the final snapshot. Prefer terminal metadata when a URL occurs in both. Retain unique tool-action sources and event-only annotations.
5. Preserve each URL once, with structured metadata before existing inline extraction. Avoid replacing a real title with the numeric annotation label. Existing first-source ordering and title enrichment should keep their established contracts.
6. Return a shape the normal Responses adapter understands: for example coherent `output_text` plus structured `citations`, or a normalized `output` tree. If both `output_text` and output content are populated, ensure parser behavior cannot repeat the same answer in differently chunked forms.
7. Keep terminal `response.failed` / `response.incomplete` as errors even if text and sources arrived earlier. Keep the existing early return after logical stream completion so a server that leaves the socket open cannot stall the request.
8. Do not merge Responses and Chat text into one response. The Chat normalization path has existing metadata-preservation coverage and should continue unchanged.

For the JSON parser, unwrap a nested `url_citation` object at the existing structured-source seam, while retaining flat `url`/`uri`, titles, descriptions, dates, and URL deduplication. This is a citation-shape compatibility change, not a transport fallback.

## Inline citations and truthful claims

The existing adapters deliberately accept `[[n]](http[s]://...)` and existing tests require this behavior. Official xAI documents that exact visible citation format. Preserve this compatibility for the current task, including structured-source priority and deduplication. Do not add ordinary Markdown links or bare-URL extraction to make the fallback disappear.

That format alone cannot independently prove tool execution for an arbitrary gateway: a model can reproduce it as text. The current schema does not distinguish an inline-only source from a structured citation. Consequently, retain the compatibility contract while describing sources as provider-reported citations, and do not claim that the parser independently verifies their retrieval or factual support. A stricter provenance policy would be a separate behavior change requiring explicit scope and a representation for that distinction.

## CI regression cases

Use the existing fake HTTP/SSE seam in `tests/sse_bug_repro.rs` and normal adapter tests. Write these regression cases for GitHub CI; do not run them locally under the user's prohibition.

| Case | Required assertion |
| --- | --- |
| Text delta, annotation-added event, compact `response.completed` | Answer survives; annotation URL/title survives; no fallback-causing empty list. |
| Event-only annotation plus final complete response with different/final text | Final text wins exactly once; annotation absent from final snapshot still survives. |
| `web_search_call.action.sources` on output-item events | Tool sources survive and retain provider label. |
| Content-part/output-item snapshot annotations | Both supported event shapes preserve citations; repeated added/done snapshots do not duplicate sources or text. |
| Repeated citation in event and final response | One URL; terminal/richer structured metadata is retained. |
| The same provenance stream ending with `[DONE]`, named completion, or EOF | Every supported termination route preserves sources. A small parameterized set is sufficient. |
| Compact `response.completed.response` with no usable text | Previously accumulated text and sources are retained. |
| Annotation/source metadata with no answer text | Parser preserves the source list; service continues the existing `grok_content_empty` behavior and keeps real citations. |
| Failed/incomplete terminal after text and annotations | Error is returned; collected partial evidence never turns failure into normal success. |
| Nested structured `url_citation` in non-streaming Responses JSON | URL/title is extracted with no arbitrary URL scanning. |
| Existing flat xAI annotations, top-level citations, and inline `[[n]]` | Compatibility remains unchanged. |
| Text with ordinary Markdown/bare URLs only | Does not gain authoritative sources; existing source fallback remains truthful. |

Existing coverage already tests multiple SSE delimiters, BOM, UTF-8 chunk boundaries, ignored `stream: false`, logical completion without connection close, Chat metadata, fallback source counts, caching, and `include_content: false`. Reuse those checks; do not add redundant copies.

The existing `.github/workflows/ci.yml` runs formatting, clippy, and tests for both default and `http` builds with locked dependencies. Those are the authorized execution environment for validation. Release/installation is outside this research subtask.

## Validation limits and remaining evidence

No tests or builds were executed. All code findings are from inspection. Official docs were read using the available fetch tool after native web search failed; this was documentation retrieval only, not a gateway or search capability probe.

The exact benchmark root cause remains unresolved because raw upstream responses were not retained and local probes are prohibited. A future authorized capture should preserve response Content-Type and redacted JSON/SSE event shapes, plus whether the gateway returned tool-call records and citation fields; it should not capture authorization headers or API keys. No new raw-body logging, model switch, transport switch, or runtime capture is required to deliver the bounded parser fix.

If the gateway genuinely returns no search citations, the corrected decoder must still report `grok_sources_empty` and use real source fallback. That is a correct, informative result, not a reason to invent citations or attach unrelated retrieved pages to the original answer.
