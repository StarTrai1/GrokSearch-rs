use reqwest::{Client, Response};
use serde_json::Value;
use std::collections::BTreeMap;
use std::time::Duration;

use crate::error::{GrokSearchError, Result};

/// Build a tuned `reqwest::Client`. The same client is shared across providers
/// so TLS sessions and keep-alive connections can be reused between providers
/// that hit different hosts. Falls back to a bare `Client::new()` if the
/// builder errors (preserves prior behavior for tests that construct providers
/// without env-driven config).
pub fn build_client(timeout: Duration) -> Client {
    Client::builder()
        .timeout(timeout)
        .gzip(true)
        .pool_idle_timeout(Some(Duration::from_secs(90)))
        .tcp_keepalive(Some(Duration::from_secs(60)))
        .tcp_nodelay(true)
        .build()
        .unwrap_or_else(|_| Client::new())
}

/// True if `ip` is a globally-routable public address. Rejects loopback,
/// private, link-local (incl. cloud metadata 169.254.169.254), CGNAT,
/// unspecified, multicast, broadcast, and documentation ranges — for both IPv4
/// and IPv6 (including IPv4-mapped IPv6). Used by the public HTTP transport's
/// SSRF guard; gated to that build so it is never dead code in the stdio build.
#[cfg(feature = "http")]
pub(crate) fn is_public_ip(ip: &std::net::IpAddr) -> bool {
    use std::net::IpAddr;
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            !(v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_multicast()
                || v4.is_documentation()
                // CGNAT 100.64.0.0/10
                || (o[0] == 100 && (o[1] & 0xc0) == 64))
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_public_ip(&IpAddr::V4(v4));
            }
            let seg0 = v6.segments()[0];
            !(v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                // unique local fc00::/7
                || (seg0 & 0xfe00) == 0xfc00
                // link-local fe80::/10
                || (seg0 & 0xffc0) == 0xfe80)
        }
    }
}

/// Shared builder for the restricted (SSRF-aware) HTTP-transport client: the
/// tuning knobs plus a redirect policy that rejects redirects to non-public
/// targets. Callers finish with `.build()`, optionally after pinning DNS.
#[cfg(feature = "http")]
fn restricted_client_builder(timeout: Duration) -> reqwest::ClientBuilder {
    use reqwest::redirect::Policy;
    Client::builder()
        .timeout(timeout)
        .gzip(true)
        .pool_idle_timeout(Some(Duration::from_secs(90)))
        .tcp_keepalive(Some(Duration::from_secs(60)))
        .tcp_nodelay(true)
        .redirect(Policy::custom(|attempt| {
            use std::net::ToSocketAddrs;
            if attempt.previous().len() > 5 {
                return attempt.error("too many redirects");
            }
            let Some(host) = attempt.url().host_str() else {
                return attempt.follow();
            };
            // Reject redirects that target — or resolve to — a non-public
            // address. Covers IP literals AND hostnames, closing redirect-based
            // SSRF / DNS-rebinding (a public URL that 30x's to an internal name).
            let ips: Vec<std::net::IpAddr> = match host.parse::<std::net::IpAddr>() {
                Ok(ip) => vec![ip],
                Err(_) => {
                    let port = attempt.url().port_or_known_default().unwrap_or(443);
                    match (host, port).to_socket_addrs() {
                        Ok(addrs) => addrs.map(|addr| addr.ip()).collect(),
                        Err(_) => return attempt.error("redirect host did not resolve"),
                    }
                }
            };
            if ips.iter().any(|ip| !is_public_ip(ip)) {
                return attempt.error("redirect to a non-public address is blocked");
            }
            attempt.follow()
        }))
}

/// Like [`build_client`] but rejects redirects to private/loopback IP-literal
/// targets (defense-in-depth against redirect-based SSRF) and caps redirect
/// depth. Used only by the public HTTP transport.
#[cfg(feature = "http")]
pub fn build_restricted_client(timeout: Duration) -> Client {
    restricted_client_builder(timeout)
        .build()
        .unwrap_or_else(|_| Client::new())
}

/// Like [`build_restricted_client`] but pins DNS for `host` to `addrs` (already
/// validated as public) so the connection cannot re-resolve the hostname to an
/// internal address between the SSRF check and the request (DNS-rebinding).
/// Used for caller-supplied Grok gateways (`X-Grok-Base-Url`) on the HTTP path.
#[cfg(feature = "http")]
pub fn build_restricted_client_pinned(
    timeout: Duration,
    host: &str,
    addrs: &[std::net::SocketAddr],
) -> Client {
    use reqwest::redirect::Policy;
    restricted_client_builder(timeout)
        .resolve_to_addrs(host, addrs)
        // The pin only covers the ORIGINAL host; a 3xx to another host would be
        // re-resolved unpinned (redirect-level DNS-rebinding SSRF). A Grok
        // gateway's /v1/responses endpoint never legitimately redirects, so
        // refuse to follow redirects at all on the pinned client.
        .redirect(Policy::none())
        .build()
        .unwrap_or_else(|_| Client::new())
}

/// Failure from [`post_json_with_status`]. `status` is the upstream HTTP
/// status when the request reached the server and came back non-2xx; `None`
/// for transport, timeout, body-read, and parse failures. Lets callers make
/// status-driven retry decisions (e.g. API-key rotation) without parsing
/// error strings.
pub struct HttpFailure {
    pub status: Option<u16>,
    pub error: GrokSearchError,
}

impl HttpFailure {
    fn transport(error: GrokSearchError) -> Self {
        Self {
            status: None,
            error,
        }
    }
}

/// Issue an authenticated JSON POST and normalize transport / status / parse
/// errors into `GrokSearchError`. `label` appears in error messages to
/// distinguish upstream providers (e.g. "Tavily", "Firecrawl", "Grok Responses").
pub async fn post_json(
    client: &Client,
    endpoint: &str,
    api_key: &str,
    body: &Value,
    label: &str,
) -> Result<Value> {
    post_json_with_status(client, endpoint, api_key, body, label)
        .await
        .map_err(|failure| failure.error)
}

/// Status-aware variant of [`post_json`]: identical behavior, but non-2xx
/// responses carry their HTTP status alongside the normalized error.
pub async fn post_json_with_status(
    client: &Client,
    endpoint: &str,
    api_key: &str,
    body: &Value,
    label: &str,
) -> std::result::Result<Value, HttpFailure> {
    send_json(client.post(endpoint).bearer_auth(api_key).json(body), label).await
}

/// Like [`post_json`], but authenticated with a provider-specific header
/// (e.g. TinyFish's `X-API-Key`) instead of a bearer `Authorization`.
pub async fn post_json_with_header_auth(
    client: &Client,
    endpoint: &str,
    header: (&str, &str),
    body: &Value,
    label: &str,
) -> Result<Value> {
    send_json(
        client.post(endpoint).header(header.0, header.1).json(body),
        label,
    )
    .await
    .map_err(|failure| failure.error)
}

/// Authenticated JSON GET with query parameters and a provider-specific auth
/// header, for GET-style search APIs (e.g. TinyFish). Same error
/// normalization as [`post_json`].
pub async fn get_json_with_header_auth(
    client: &Client,
    endpoint: &str,
    query: &[(&str, String)],
    header: (&str, &str),
    label: &str,
) -> Result<Value> {
    send_json(
        client.get(endpoint).query(query).header(header.0, header.1),
        label,
    )
    .await
    .map_err(|failure| failure.error)
}

/// Send a prepared JSON request and normalize transport / status / parse
/// failures into `HttpFailure`. Shared by the bearer- and header-auth helpers
/// so every provider gets identical error handling (including the SSE path).
async fn send_json(
    request: reqwest::RequestBuilder,
    label: &str,
) -> std::result::Result<Value, HttpFailure> {
    let mut response = request.send().await.map_err(|err| {
        HttpFailure::transport(if err.is_timeout() {
            GrokSearchError::Timeout(format!("{label} request timed out: {err}"))
        } else {
            GrokSearchError::Provider(format!("{label} request failed: {err}"))
        })
    })?;

    let status = response.status();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if status.is_success() && content_type.starts_with("text/event-stream") {
        return read_sse_json(&mut response, label)
            .await
            .map_err(HttpFailure::transport);
    }

    let bytes = response.bytes().await.map_err(|err| {
        HttpFailure::transport(GrokSearchError::Provider(format!(
            "{label} body read failed: {err}"
        )))
    })?;

    if !status.is_success() {
        let text = String::from_utf8_lossy(&bytes);
        return Err(HttpFailure {
            status: Some(status.as_u16()),
            error: GrokSearchError::Provider(format!("{label} returned HTTP {status}: {text}")),
        });
    }

    serde_json::from_slice(&bytes).map_err(|err| {
        HttpFailure::transport(GrokSearchError::Parse(format!(
            "invalid {label} JSON: {err}"
        )))
    })
}

async fn read_sse_json(response: &mut Response, label: &str) -> Result<Value> {
    let mut buffer = Vec::new();
    let mut responses = ResponsesStreamState::default();
    let mut chat_content = String::new();
    let mut last_json = None;
    let mut chat_metadata = None;

    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|err| GrokSearchError::Provider(format!("{label} stream read failed: {err}")))?
    {
        buffer.extend_from_slice(&chunk);

        while let Some((event, rest)) = split_sse_event(&buffer) {
            let event = event.to_vec();
            buffer = rest.to_vec();
            if let Some(value) = process_sse_event(
                &event,
                label,
                &mut last_json,
                &mut chat_metadata,
                &mut responses,
                &mut chat_content,
            )? {
                return Ok(value);
            }
        }
    }

    if !buffer.is_empty() {
        let event = std::mem::take(&mut buffer);
        if let Some(value) = process_sse_event(
            &event,
            label,
            &mut last_json,
            &mut chat_metadata,
            &mut responses,
            &mut chat_content,
        )? {
            return Ok(value);
        }
    }

    finish_sse_json(label, last_json, chat_metadata, responses, chat_content)
}

/// Text snapshots replace their own part; later deltas extend that snapshot.
/// Source metadata is cumulative because compact final responses may omit it.
#[derive(Default)]
struct ResponsesStreamState {
    seen: bool,
    text_parts: BTreeMap<(u64, u64), String>,
    citations: Vec<Value>,
}

impl ResponsesStreamState {
    fn collect(&mut self, value: &Value) {
        let kind = value.get("type").and_then(Value::as_str).unwrap_or("");
        if !kind.starts_with("response.")
            && value.get("output").is_none()
            && value.get("output_text").is_none()
        {
            return;
        }
        self.seen = true;
        let output_index = value
            .get("output_index")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let content_index = value
            .get("content_index")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        self.collect_citations(value.get("citations"));

        match kind {
            "response.output_text.delta" => {
                if let Some(delta) = value.get("delta").and_then(Value::as_str) {
                    self.text_parts
                        .entry((output_index, content_index))
                        .or_default()
                        .push_str(delta);
                }
            }
            "response.output_text.done" => {
                self.set_text(output_index, content_index, value.get("text"));
            }
            "response.output_text.annotation.added" => {
                self.collect_citations(value.get("annotation"));
            }
            "response.content_part.added" | "response.content_part.done" => {
                if let Some(part) = value.get("part") {
                    self.collect_part(part, output_index, content_index);
                }
            }
            "response.output_item.added" | "response.output_item.done" => {
                if let Some(item) = value.get("item") {
                    self.collect_item(item, output_index);
                }
            }
            _ => {}
        }

        let snapshot = value.get("response").unwrap_or(value);
        self.collect_citations(snapshot.get("citations"));
        if let Some(output) = snapshot.get("output").and_then(Value::as_array) {
            for (index, item) in output.iter().enumerate() {
                self.collect_item(item, index as u64);
            }
        }
        if !self.has_text() {
            self.set_text(0, 0, snapshot.get("output_text"));
        }
    }

    fn set_text(&mut self, output_index: u64, content_index: u64, value: Option<&Value>) {
        if let Some(text) = value
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
        {
            self.text_parts
                .insert((output_index, content_index), text.to_string());
        }
    }

    fn collect_part(&mut self, part: &Value, output_index: u64, content_index: u64) {
        self.set_text(output_index, content_index, part.get("text"));
        self.collect_citations(part.get("annotations"));
        self.collect_citations(part.get("citations"));
    }

    fn collect_item(&mut self, item: &Value, output_index: u64) {
        if item.get("type").and_then(Value::as_str) == Some("web_search_call") {
            self.collect_citations(item.pointer("/action/sources"));
        }
        if let Some(parts) = item.get("content").and_then(Value::as_array) {
            // A nonempty item snapshot replaces earlier text for that item,
            // including any old content parts absent from the new snapshot.
            if parts.iter().any(|part| has_nonempty_text(part.get("text"))) {
                self.text_parts
                    .retain(|(index, _), _| *index != output_index);
            }
            for (index, part) in parts.iter().enumerate() {
                self.collect_part(part, output_index, index as u64);
            }
        }
    }

    fn collect_citations(&mut self, value: Option<&Value>) {
        match value {
            Some(Value::Array(items)) => {
                for item in items {
                    self.collect_citations(Some(item));
                }
            }
            Some(item)
                if (item.is_object() || item.is_string()) && !self.citations.contains(item) =>
            {
                self.citations.push(item.clone());
            }
            _ => {}
        }
    }

    fn has_text(&self) -> bool {
        self.text_parts.values().any(|text| !text.trim().is_empty())
    }

    fn into_json(self, last_json: Option<Value>) -> Value {
        let mut raw = last_json
            .and_then(|value| {
                value
                    .get("response")
                    .filter(|response| response.is_object())
                    .cloned()
                    .or_else(|| {
                        (value.get("output").is_some()
                            || value.get("output_text").is_some()
                            || value.get("citations").is_some())
                        .then_some(value)
                    })
            })
            .unwrap_or_else(|| serde_json::json!({}));

        // Preserve the complete terminal answer. Only compact terminal objects
        // need their missing text rebuilt; never append snapshots to deltas.
        if !response_has_text(&raw) {
            let text = self
                .text_parts
                .values()
                .map(|text| text.trim())
                .filter(|text| !text.is_empty())
                .collect::<Vec<_>>()
                .join("\n");
            if !text.is_empty() {
                raw["output_text"] = Value::String(text);
            }
        }
        if !self.citations.is_empty() {
            if let Some(object) = raw.as_object_mut() {
                // Terminal metadata comes first, so downstream URL dedupe
                // retains its title/order while adding event-only evidence.
                if let Some(existing) = object.get_mut("citations") {
                    if !existing.is_array() {
                        *existing = Value::Array(vec![existing.take()]);
                    }
                }
                append_json_array(object, "citations", &Value::Array(self.citations));
            }
        }
        raw
    }
}

fn has_nonempty_text(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_str)
        .is_some_and(|text| !text.trim().is_empty())
}

fn response_has_text(value: &Value) -> bool {
    has_nonempty_text(value.get("output_text"))
        || value
            .get("output")
            .and_then(Value::as_array)
            .is_some_and(|items| {
                items.iter().any(|item| {
                    item.get("content")
                        .and_then(Value::as_array)
                        .is_some_and(|parts| {
                            parts.iter().any(|part| has_nonempty_text(part.get("text")))
                        })
                })
            })
}

struct SseEvent {
    name: Option<String>,
    data: Option<String>,
}

fn split_sse_event(buffer: &[u8]) -> Option<(&[u8], &[u8])> {
    let delimiter = [
        b"\n\n".as_slice(),
        b"\r\n\r\n".as_slice(),
        b"\r\r".as_slice(),
    ]
    .into_iter()
    .filter_map(|delimiter| find_bytes(buffer, delimiter).map(|index| (index, delimiter.len())))
    .min_by_key(|(index, _)| *index)?;
    Some((&buffer[..delimiter.0], &buffer[delimiter.0 + delimiter.1..]))
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn parse_sse_event(event: &[u8], label: &str) -> Result<SseEvent> {
    let event = std::str::from_utf8(event)
        .map_err(|err| GrokSearchError::Parse(format!("invalid {label} SSE UTF-8: {err}")))?;
    let event = event.strip_prefix('\u{feff}').unwrap_or(event);
    let mut name = None;
    let mut lines = Vec::new();
    for line in event.split(['\n', '\r']) {
        if let Some(event_name) = line.strip_prefix("event:") {
            name = Some(event_name.trim_start().to_string());
            continue;
        }
        if let Some(data) = line.strip_prefix("data:") {
            lines.push(data.trim_start());
        }
    }

    Ok(SseEvent {
        name,
        data: if lines.is_empty() {
            None
        } else {
            Some(lines.join("\n"))
        },
    })
}

fn is_completion_event_name(name: &str) -> bool {
    matches!(
        name,
        "done" | "end" | "complete" | "completed" | "response.completed"
    )
}

fn process_sse_event(
    event: &[u8],
    label: &str,
    last_json: &mut Option<Value>,
    chat_metadata: &mut Option<Value>,
    responses: &mut ResponsesStreamState,
    chat_content: &mut String,
) -> Result<Option<Value>> {
    let event = parse_sse_event(event, label)?;
    let named_completion = event.name.as_deref().is_some_and(is_completion_event_name);
    let data = event.data.as_deref().map(str::trim);

    if named_completion && data.map(str::is_empty).unwrap_or(true) {
        return finish_sse_state(label, last_json, chat_metadata, responses, chat_content)
            .map(Some);
    }

    let Some(data) = data else {
        return Ok(None);
    };
    if data.is_empty() {
        return Ok(None);
    }
    if data == "[DONE]" {
        return finish_sse_state(label, last_json, chat_metadata, responses, chat_content)
            .map(Some);
    }

    let value: Value = serde_json::from_str(data)
        .map_err(|err| GrokSearchError::Parse(format!("invalid {label} SSE JSON: {err}")))?;
    responses.collect(&value);
    collect_chat_delta(&value, chat_content);
    accumulate_chat_metadata(chat_metadata, &value);

    if let Some(kind) = response_terminal_error_type(&value) {
        return Err(GrokSearchError::Provider(format!(
            "{label} stream ended with {kind}: {}",
            response_terminal_error_detail(&value)
        )));
    }

    if named_completion || value.get("type").and_then(Value::as_str) == Some("response.completed") {
        *last_json = Some(value);
        return finish_sse_state(label, last_json, chat_metadata, responses, chat_content)
            .map(Some);
    }

    *last_json = Some(value);
    Ok(None)
}

fn finish_sse_state(
    label: &str,
    last_json: &mut Option<Value>,
    chat_metadata: &mut Option<Value>,
    responses: &mut ResponsesStreamState,
    chat_content: &mut String,
) -> Result<Value> {
    finish_sse_json(
        label,
        last_json.take(),
        chat_metadata.take(),
        std::mem::take(responses),
        std::mem::take(chat_content),
    )
}

fn response_terminal_error_type(value: &Value) -> Option<&str> {
    match value.get("type").and_then(Value::as_str) {
        Some("response.failed" | "response.incomplete") => {
            value.get("type").and_then(Value::as_str)
        }
        _ => None,
    }
}

fn response_terminal_error_detail(value: &Value) -> String {
    value
        .pointer("/error/message")
        .or_else(|| value.pointer("/response/error/message"))
        .or_else(|| value.get("error"))
        .map(|detail| match detail {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        })
        .unwrap_or_else(|| value.to_string())
}

fn accumulate_chat_metadata(acc: &mut Option<Value>, value: &Value) {
    if value.get("choices").is_none()
        && value.get("citations").is_none()
        && value.get("search_sources").is_none()
    {
        return;
    }

    if acc.is_none() {
        *acc = Some(serde_json::json!({}));
    }
    let Some(raw) = acc.as_mut().and_then(Value::as_object_mut) else {
        return;
    };

    for key in ["citations", "search_sources"] {
        if let Some(items) = value.get(key) {
            append_json_array(raw, key, items);
        }
    }

    for pointer in [
        "/choices/0/message/annotations",
        "/choices/0/message/citations",
        "/choices/0/delta/annotations",
        "/choices/0/delta/citations",
    ] {
        let Some(items) = value.pointer(pointer) else {
            continue;
        };
        let key = pointer.rsplit('/').next().unwrap_or_default();
        let choices = raw
            .entry("choices".to_string())
            .or_insert_with(|| serde_json::json!([{ "delta": {} }]));
        if choices.as_array().map(Vec::is_empty).unwrap_or(true) {
            *choices = serde_json::json!([{ "delta": {} }]);
        }
        if let Some(delta) = choices
            .pointer_mut("/0/delta")
            .and_then(Value::as_object_mut)
        {
            append_json_array(delta, key, items);
        }
    }
}

fn append_json_array(map: &mut serde_json::Map<String, Value>, key: &str, value: &Value) {
    let entry = map
        .entry(key.to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if !entry.is_array() {
        *entry = Value::Array(Vec::new());
    }
    let Some(out) = entry.as_array_mut() else {
        return;
    };
    match value {
        Value::Array(items) => out.extend(items.iter().cloned()),
        other => out.push(other.clone()),
    }
}

fn synthesize_chat_json(
    last_json: Option<Value>,
    chat_metadata: Option<Value>,
    chat_content: String,
) -> Value {
    let mut raw = chat_metadata
        .or(last_json)
        .unwrap_or_else(|| serde_json::json!({}));
    if !raw.is_object() {
        raw = serde_json::json!({});
    }

    let message = raw
        .pointer("/choices/0/message")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let delta = raw
        .pointer("/choices/0/delta")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    let mut message = match message {
        Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };
    if let Value::Object(delta) = delta {
        for key in ["annotations", "citations"] {
            if !message.contains_key(key) {
                if let Some(value) = delta.get(key) {
                    message.insert(key.to_string(), value.clone());
                }
            }
        }
    }
    message.insert("content".to_string(), Value::String(chat_content));

    let choice = serde_json::json!({ "message": Value::Object(message) });
    if let Some(object) = raw.as_object_mut() {
        object.insert("choices".to_string(), Value::Array(vec![choice]));
        raw
    } else {
        serde_json::json!({ "choices": [choice] })
    }
}

fn collect_chat_delta(value: &Value, chat_content: &mut String) {
    if let Some(content) = value.pointer("/choices/0/delta/content") {
        match content {
            Value::String(text) => chat_content.push_str(text),
            Value::Array(parts) => {
                for part in parts {
                    if let Some(text) = part
                        .get("text")
                        .or_else(|| part.get("content"))
                        .and_then(Value::as_str)
                    {
                        chat_content.push_str(text);
                    }
                }
            }
            _ => {}
        }
    }
}

fn finish_sse_json(
    label: &str,
    last_json: Option<Value>,
    chat_metadata: Option<Value>,
    responses: ResponsesStreamState,
    chat_content: String,
) -> Result<Value> {
    if responses.has_text()
        || !responses.citations.is_empty()
        || (responses.seen && chat_content.is_empty() && chat_metadata.is_none())
    {
        return Ok(responses.into_json(last_json));
    }
    if !chat_content.is_empty() {
        return Ok(synthesize_chat_json(last_json, chat_metadata, chat_content));
    }
    if chat_metadata.is_some() {
        return Ok(synthesize_chat_json(last_json, chat_metadata, chat_content));
    }
    last_json.ok_or_else(|| {
        GrokSearchError::Parse(format!("{label} SSE stream ended without JSON data"))
    })
}
