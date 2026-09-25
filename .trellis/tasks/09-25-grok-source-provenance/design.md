# Design

## Reconstruction seam
Keep the existing HTTP transport and response adapters. Extend the existing SSE reconstruction with bounded state for known Responses output items, text-content parts and citation events. Preserve structured web_search_call.action.sources and output_text annotations/citations. A complete terminal payload remains authoritative for text; merge earlier source metadata that the terminal omits. Existing failed/incomplete stream semantics must not be converted to success.

Support nested url_citation objects in the Responses adapter using the same recognized structure already handled by Chat Completions. Do not broaden extraction to ordinary Markdown or bare URLs. Keep the current documented [[n]] citation compatibility; it is a protocol compatibility assumption rather than independent execution proof.

## Scope and uncertainty
The loss of separately streamed source metadata is established in code. Existing MCP result archives do not contain raw gateway responses, so this mechanism is not claimed to explain every live grok_sources_empty result. If the upstream never emits citations, the correct outcome remains source fallback. No alternative synthesis mode, model switch, or gateway mutation is part of this patch.

## Release and installation
Reuse required checks via a callable CI workflow. Add a dedicated personal-fork Linux x86_64 musl stdio release, triggered by fork-* tags and gated on those checks. Avoid upstream npm publishing, container deployment and manifest synchronization on the fork. Keep crate version stable; commit/run metadata identifies the fork build.

Install the downloaded, checksum-verified binary in a versioned user-owned directory and set an absolute MCP command. Preserve the original npm installation and backed-up command for rollback; do not overwrite npm-owned files. A new MCP process/session is required to load the new executable and parameters.

## Parameter adjustment
Preserve extra sources=3, inline sources=4, enrichment concurrency=4, response budget=45000, timeout=90s, cache=256. Set fallback sources=5, enrichment per-source cap=8000, standalone fetch cap=60000. The fetch cap addresses observed loss of later documentation sections; the inline cap leaves room for summaries/metadata within the fixed response budget. More fallback candidates improve the opportunity to select useful sources but do not prove higher quality.
