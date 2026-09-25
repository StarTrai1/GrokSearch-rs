# Quality Guidelines

> Code quality standards for backend development.

## Responses stream provenance

- Preserve structured citation and search-source metadata across compact completion, `[DONE]`, named completion, and EOF. Complete final text takes precedence; snapshots and deltas must not duplicate answer text.
- Keep source metadata cumulative while replacing text by output/content index. A failed or incomplete terminal event remains an error even after usable text or citations arrived.
- Support the existing structured and numbered-inline citation contracts. Ordinary Markdown links and bare URLs do not establish search provenance; source-free answers retain the explicit source fallback.
- Cover these boundaries through adapter tests and the fake SSE server in `tests/sse_bug_repro.rs`. When local execution is prohibited, run formatting, clippy, and tests in GitHub CI and record the exact SHA; static review alone is not a passing test result.


## Live gateway source diagnosis

- Correlate the same request's ingress tools, selected outbound protocol/body, upstream citations and client response before attributing missing sources to a parser. A Responses-to-Chat conversion can discard a hosted search tool before the model sees it.
- Check actual structured citations and tool events. A gateway may report `num_sources_used=0` even when it returns valid citation objects; that counter alone cannot establish source absence.
- Keep search candidate URLs distinct from the citations used by an answer when evaluating relevance. A larger source list is not evidence of higher answer quality.

---

## Overview

<!--
Document your project's quality standards here.

Questions to answer:
- What patterns are forbidden?
- What linting rules do you enforce?
- What are your testing requirements?
- What code review standards apply?
-->

(To be filled by the team)

---

## Forbidden Patterns

<!-- Patterns that should never be used and why -->

(To be filled by the team)

---

## Required Patterns

<!-- Patterns that must always be used -->

(To be filled by the team)

---

## Testing Requirements

<!-- What level of testing is expected -->

(To be filled by the team)

---

## Code Review Checklist

<!-- What reviewers should check -->

(To be filled by the team)
