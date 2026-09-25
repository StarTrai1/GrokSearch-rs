# Design

## Responsible boundary

Actual request/execution pairs establish that AxonHub's implicit Chat endpoint converts Responses requests and drops the hosted web_search tool. The installed GrokSearch client already sends the correct tool and can parse the native response shape. No further source patch is justified for this failure.

## Configuration correction (original proposal)

Add a custom endpoint `{api_format: openai/responses, transport: http}` to the existing Grok channel using the supported admin UI/API. Leave URL/path empty to inherit the configured base and `/responses`. AxonHub merges it with implicit defaults, preserving Chat endpoints; incoming Responses chooses the native matching endpoint. Six configured models are Grok variants. The change applies to Responses requests on that channel, and the current selected model has been directly validated.

The supported save API validates and refreshes routing caches. Do not write database rows directly or restart the managed service. The task has a model-use API key, not an authenticated admin session, so the user must save this concrete endpoint entry in the existing UI. No passwords, upstream keys or JWTs should be pasted into chat.

## Evidence

- Native streaming upstream Responses: HTTP200, completed in32.3s, web_search_call events, official Python3.12 structured url_citation.
- Native nonstreaming upstream Responses: HTTP200, completed in20.9s, search calls and official citation. This validates the installed client's existing stream=false contract.
- Both retain the current model and provider; no prompt-only source invention or ordinary-link promotion.
- Existing source SHA3e202342 and its passed hosted CI/release remain the installed artifact. A configuration-only correction does not require a new build or expanded developer test run.

## Rollback

For the actual user-saved configuration, restore the channel type to its recorded original `openai` value with endpoints=[] through the supported admin UI/API. Removing a custom endpoint does not apply because none was added. The original proposal would instead have rolled back by removing its additional endpoint. The old npm package was retained until acceptance and then removed; routing rollback is independent of the installed binary and client credentials.

## Applied configuration

The user saved channel type `openai_responses` rather than adding a custom endpoint to the former Chat channel. The database records this type with endpoints=[] at 2026-09-25T12:14:40Z. Treat this explicit user configuration as the actual correction. The original proposal's preservation of implicit Chat endpoints does not describe this saved change; other protocol behavior was not claimed or tested.

Installed-MCP acceptance subsequently used native Responses with pass_through_applied=1, preserved web_search intent and official url_citation end to end. The old npm package was removed only after this evidence was checked. No source patch was needed for the live incident.
