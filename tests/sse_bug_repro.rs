// 复现/回归：上游返回 SSE，并且发完 completed 后不主动关闭连接。

use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use grok_search_rs::adapters::chat_completions_response::parse_chat_completions;
use grok_search_rs::adapters::grok_responses_response::parse_grok_responses;
use grok_search_rs::error::Result;
use grok_search_rs::providers::http::{build_client, post_json};
use serde_json::{json, Value};

async fn spawn_sse_server(expected_stream: bool, chunks: Vec<Vec<u8>>) -> String {
    spawn_sse_server_with_mode(expected_stream, chunks, true).await
}

async fn spawn_closing_sse_server(expected_stream: bool, chunks: Vec<Vec<u8>>) -> String {
    spawn_sse_server_with_mode(expected_stream, chunks, false).await
}

async fn spawn_sse_server_with_mode(
    expected_stream: bool,
    chunks: Vec<Vec<u8>>,
    keep_open: bool,
) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            let chunks = chunks.clone();
            let (mut sock, _) = match listener.accept().await {
                Ok(s) => s,
                Err(_) => return,
            };
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                let n = sock.read(&mut buf).await.unwrap_or(0);
                let request = String::from_utf8_lossy(&buf[..n]);
                let expected = format!(r#""stream":{}"#, expected_stream);
                if !request.contains(&expected) {
                    let _ = sock
                        .write_all(
                            b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        )
                        .await;
                    return;
                }

                let resp = "HTTP/1.1 200 OK\r\n\
                     Content-Type: text/event-stream\r\n\
                     Connection: keep-alive\r\n\r\n";
                let _ = sock.write_all(resp.as_bytes()).await;
                for chunk in chunks {
                    let _ = sock.write_all(&chunk).await;
                }

                if keep_open {
                    // 不 shutdown：服务端保持连接，客户端必须在 completed 后主动结束读取。
                    let mut one = [0u8; 1];
                    let _ = sock.read(&mut one).await;
                } else {
                    let _ = sock.shutdown().await;
                }
            });
        }
    });

    format!("http://{}", addr)
}

fn responses_chunks() -> Vec<Vec<u8>> {
    vec![b"event: response.created\n\
data: {\"type\":\"response.created\"}\n\n\
event: response.output_text.delta\n\
data: {\"type\":\"response.output_text.delta\",\"delta\":\"streamed answer\"}\n\n\
event: response.completed\n\
data: {\"type\":\"response.completed\"}\n\n"
        .to_vec()]
}

#[tokio::test]
async fn post_json_returns_when_stream_true_sse_completed_without_connection_close() {
    let base = spawn_sse_server(true, responses_chunks()).await;
    let client = build_client(Duration::from_secs(5));

    let raw = post_json(
        &client,
        &format!("{}/v1/responses", base),
        "dummy-key",
        &json!({"model": "grok-4-fast", "input": "test", "stream": true}),
        "Grok Responses",
    )
    .await
    .expect("SSE stream should be normalized into JSON");

    assert_eq!(raw["output_text"], "streamed answer");
}

#[tokio::test]
async fn post_json_returns_when_stream_false_still_gets_sse() {
    let base = spawn_sse_server(false, responses_chunks()).await;
    let client = build_client(Duration::from_secs(5));

    let raw = post_json(
        &client,
        &format!("{}/v1/responses", base),
        "dummy-key",
        &json!({"model": "grok-4-fast", "input": "test", "stream": false}),
        "Grok Responses",
    )
    .await
    .expect("SSE stream should be normalized even when stream:false is ignored");

    assert_eq!(raw["output_text"], "streamed answer");
}

#[tokio::test]
async fn post_json_returns_when_done_event_has_no_data() {
    let chunks = vec![b"event: response.output_text.delta\n\
data: {\"type\":\"response.output_text.delta\",\"delta\":\"done marker\"}\n\n\
event: done\n\n"
        .to_vec()];
    let base = spawn_sse_server(false, chunks).await;
    let client = build_client(Duration::from_secs(5));

    let raw = post_json(
        &client,
        &format!("{}/v1/responses", base),
        "dummy-key",
        &json!({"model": "grok-4-fast", "input": "test", "stream": false}),
        "Grok Responses",
    )
    .await
    .expect("event: done without data should terminate the SSE stream");

    assert_eq!(raw["output_text"], "done marker");
}

#[tokio::test]
async fn post_json_returns_when_done_event_has_empty_data() {
    let chunks = vec![b"event: response.output_text.delta\n\
data: {\"type\":\"response.output_text.delta\",\"delta\":\"empty data done\"}\n\n\
event: done\n\
data:\n\n"
        .to_vec()];
    let base = spawn_sse_server(false, chunks).await;
    let client = build_client(Duration::from_secs(5));

    let raw = post_json(
        &client,
        &format!("{}/v1/responses", base),
        "dummy-key",
        &json!({"model": "grok-4-fast", "input": "test", "stream": false}),
        "Grok Responses",
    )
    .await
    .expect("event: done with empty data should terminate the SSE stream");

    assert_eq!(raw["output_text"], "empty data done");
}

#[tokio::test]
async fn post_json_returns_when_done_event_has_json_payload() {
    let chunks = vec![b"event: response.output_text.delta\n\
data: {\"type\":\"response.output_text.delta\",\"delta\":\"json done\"}\n\n\
event: done\n\
data: {}\n\n"
        .to_vec()];
    let base = spawn_sse_server(false, chunks).await;
    let client = build_client(Duration::from_secs(5));

    let raw = post_json(
        &client,
        &format!("{}/v1/responses", base),
        "dummy-key",
        &json!({"model": "grok-4-fast", "input": "test", "stream": false}),
        "Grok Responses",
    )
    .await
    .expect("event: done with JSON payload should terminate the SSE stream");

    assert_eq!(raw["output_text"], "json done");
}

#[tokio::test]
async fn post_json_splits_cr_only_sse_events() {
    let chunks = vec![b"event: response.output_text.delta\r\
data: {\"type\":\"response.output_text.delta\",\"delta\":\"cr done\"}\r\r\
event: done\r\r"
        .to_vec()];
    let base = spawn_sse_server(false, chunks).await;
    let client = build_client(Duration::from_secs(5));

    let raw = post_json(
        &client,
        &format!("{}/v1/responses", base),
        "dummy-key",
        &json!({"model": "grok-4-fast", "input": "test", "stream": false}),
        "Grok Responses",
    )
    .await
    .expect("CR-only SSE frame delimiters should be recognized");

    assert_eq!(raw["output_text"], "cr done");
}

#[tokio::test]
async fn post_json_uses_earliest_mixed_sse_delimiter() {
    let chunks = vec![
        b"data: {\"type\":\"response.output_text.delta\",\"delta\":\"first\"}\r\n\r\n\
data: {\"type\":\"response.output_text.delta\",\"delta\":\" second\"}\n\n\
event: done\n\n"
            .to_vec(),
    ];
    let base = spawn_sse_server(false, chunks).await;
    let client = build_client(Duration::from_secs(5));

    let raw = post_json(
        &client,
        &format!("{}/v1/responses", base),
        "dummy-key",
        &json!({"model": "grok-4-fast", "input": "test", "stream": false}),
        "Grok Responses",
    )
    .await
    .expect("mixed SSE delimiters should split at the earliest event boundary");

    assert_eq!(raw["output_text"], "first second");
}

#[tokio::test]
async fn post_json_strips_initial_sse_bom() {
    let chunks = vec![
        "\u{feff}data: {\"type\":\"response.output_text.delta\",\"delta\":\"bom ok\"}\n\n\
event: done\n\n"
            .as_bytes()
            .to_vec(),
    ];
    let base = spawn_sse_server(false, chunks).await;
    let client = build_client(Duration::from_secs(5));

    let raw = post_json(
        &client,
        &format!("{}/v1/responses", base),
        "dummy-key",
        &json!({"model": "grok-4-fast", "input": "test", "stream": false}),
        "Grok Responses",
    )
    .await
    .expect("initial SSE BOM should not hide the first data field");

    assert_eq!(raw["output_text"], "bom ok");
}

#[tokio::test]
async fn post_json_preserves_metadata_only_chat_stream() {
    let chunks = vec![
        b"data: {\"choices\":[{\"delta\":{\"annotations\":[{\"type\":\"url_citation\",\"url\":\"https://example.com/meta\",\"title\":\"Meta\"}]}}],\"search_sources\":[{\"url\":\"https://example.com/source\",\"title\":\"Source\"}]}\n\n\
data: {\"choices\":[{\"finish_reason\":\"stop\"}]}\n\n\
event: done\n\n"
            .to_vec(),
    ];
    let base = spawn_sse_server(true, chunks).await;
    let client = build_client(Duration::from_secs(5));

    let raw = post_json(
        &client,
        &format!("{}/v1/chat/completions", base),
        "dummy-key",
        &json!({"model": "grok-4-fast", "messages": [], "stream": true}),
        "OpenAI-compatible",
    )
    .await
    .expect("metadata-only chat SSE should preserve source provenance");

    let parsed = parse_chat_completions(&raw).expect("metadata-only chat response has sources");
    assert!(parsed.content.is_empty());
    let urls: Vec<_> = parsed
        .sources
        .iter()
        .map(|source| source.url.as_str())
        .collect();
    assert!(urls.contains(&"https://example.com/meta"));
    assert!(urls.contains(&"https://example.com/source"));
}

#[tokio::test]
async fn post_json_parses_trailing_sse_event_at_eof() {
    let chunks = vec![
        b"data: {\"type\":\"response.output_text.delta\",\"delta\":\"trailing eof\"}".to_vec(),
    ];
    let base = spawn_closing_sse_server(false, chunks).await;
    let client = build_client(Duration::from_secs(5));

    let raw = post_json(
        &client,
        &format!("{}/v1/responses", base),
        "dummy-key",
        &json!({"model": "grok-4-fast", "input": "test", "stream": false}),
        "Grok Responses",
    )
    .await
    .expect("EOF should flush the final SSE event even without blank-line delimiter");

    assert_eq!(raw["output_text"], "trailing eof");
}

#[tokio::test]
async fn post_json_errors_on_response_failed_terminal_event() {
    let chunks = vec![b"event: response.failed\n\
data: {\"type\":\"response.failed\",\"error\":{\"message\":\"upstream failed\"}}\n\n"
        .to_vec()];
    let base = spawn_sse_server(false, chunks).await;
    let client = build_client(Duration::from_secs(5));

    let err = post_json(
        &client,
        &format!("{}/v1/responses", base),
        "dummy-key",
        &json!({"model": "grok-4-fast", "input": "test", "stream": false}),
        "Grok Responses",
    )
    .await
    .expect_err("terminal response.failed event should surface as provider error");

    assert!(
        err.to_string().contains("response.failed") && err.to_string().contains("upstream failed"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn post_json_decodes_sse_utf8_after_full_event_boundary() {
    let body = "event: response.output_text.delta\n\
data: {\"type\":\"response.output_text.delta\",\"delta\":\"你好\"}\n\n\
event: response.completed\n\
data: {\"type\":\"response.completed\"}\n\n";
    let bytes = body.as_bytes();
    let split = bytes
        .windows("你好".len())
        .position(|window| window == "你好".as_bytes())
        .expect("test body contains multibyte text")
        + 1;
    let chunks = vec![bytes[..split].to_vec(), bytes[split..].to_vec()];
    let base = spawn_sse_server(false, chunks).await;
    let client = build_client(Duration::from_secs(5));

    let raw = post_json(
        &client,
        &format!("{}/v1/responses", base),
        "dummy-key",
        &json!({"model": "grok-4-fast", "input": "test", "stream": false}),
        "Grok Responses",
    )
    .await
    .expect("SSE UTF-8 should be decoded after full event buffering");

    assert_eq!(raw["output_text"], "你好");
}

#[tokio::test]
async fn post_json_preserves_chat_stream_metadata() {
    let chunks = vec![
        b"data: {\"choices\":[{\"delta\":{\"content\":\"answer\",\"annotations\":[{\"type\":\"url_citation\",\"url\":\"https://example.com/a\",\"title\":\"A\"}]}}],\"search_sources\":[{\"url\":\"https://example.com/b\",\"title\":\"B\"}]}\n\n\
data: {\"choices\":[{\"finish_reason\":\"stop\"}]}\n\n\
event: done\n\n"
            .to_vec(),
    ];
    let base = spawn_sse_server(true, chunks).await;
    let client = build_client(Duration::from_secs(5));

    let raw = post_json(
        &client,
        &format!("{}/v1/chat/completions", base),
        "dummy-key",
        &json!({"model": "grok-4-fast", "messages": [], "stream": true}),
        "OpenAI-compatible",
    )
    .await
    .expect("chat SSE should be normalized into a metadata-preserving response");

    let parsed = parse_chat_completions(&raw).expect("chat response should preserve sources");
    assert_eq!(parsed.content, "answer");
    let urls: Vec<_> = parsed
        .sources
        .iter()
        .map(|source| source.url.as_str())
        .collect();
    assert!(urls.contains(&"https://example.com/a"));
    assert!(urls.contains(&"https://example.com/b"));
}

fn response_event(value: Value) -> Vec<u8> {
    format!("data: {value}\n\n").into_bytes()
}

async fn read_responses_stream(chunks: Vec<Vec<u8>>, keep_open: bool) -> Result<Value> {
    let base = spawn_sse_server_with_mode(false, chunks, keep_open).await;
    let client = build_client(Duration::from_secs(5));
    post_json(
        &client,
        &format!("{base}/v1/responses"),
        "dummy-key",
        &json!({"model": "grok-4-fast", "input": "test", "stream": false}),
        "Grok Responses",
    )
    .await
}

// Each source exists in a different structured event. Later snapshots omit
// earlier citations, as a compact gateway may do; text snapshots repeat deltas.
fn responses_provenance_chunks() -> Vec<Vec<u8>> {
    vec![
        response_event(json!({
            "type": "response.output_item.added",
            "output_index": 0,
            "item": {"id": "search-1", "type": "web_search_call", "status": "in_progress"}
        })),
        response_event(json!({
            "type": "response.output_item.done",
            "output_index": 0,
            "item": {
                "id": "search-1",
                "type": "web_search_call",
                "status": "completed",
                "action": {
                    "type": "search",
                    "sources": [{"url": "https://example.com/tool", "title": "Tool source"}]
                }
            }
        })),
        response_event(json!({
            "type": "response.content_part.added",
            "output_index": 1,
            "item_id": "message-1",
            "content_index": 0,
            "part": {"type": "output_text", "text": "", "annotations": []}
        })),
        response_event(json!({
            "type": "response.output_text.delta",
            "output_index": 1,
            "item_id": "message-1",
            "content_index": 0,
            "delta": "streamed "
        })),
        response_event(json!({
            "type": "response.output_text.delta",
            "output_index": 1,
            "item_id": "message-1",
            "content_index": 0,
            "delta": "answer"
        })),
        response_event(json!({
            "type": "response.output_text.annotation.added",
            "output_index": 1,
            "item_id": "message-1",
            "content_index": 0,
            "annotation_index": 0,
            "annotation": {
                "type": "url_citation",
                "url": "https://example.com/annotation",
                "title": "Stream annotation"
            }
        })),
        response_event(json!({
            "type": "response.content_part.done",
            "output_index": 1,
            "item_id": "message-1",
            "content_index": 0,
            "part": {
                "type": "output_text",
                "text": "streamed answer",
                "annotations": [{"url": "https://example.com/part", "title": "Part source"}]
            }
        })),
        response_event(json!({
            "type": "response.output_item.done",
            "output_index": 1,
            "item": {
                "id": "message-1",
                "type": "message",
                "role": "assistant",
                "status": "completed",
                "content": [{
                    "type": "output_text",
                    "text": "streamed answer",
                    "citations": [{"url": "https://example.com/item", "title": "Item source"}]
                }]
            }
        })),
    ]
}

fn assert_streamed_sources(parsed: &grok_search_rs::model::search::SearchResponse) {
    for url in [
        "https://example.com/tool",
        "https://example.com/annotation",
        "https://example.com/part",
        "https://example.com/item",
    ] {
        assert!(
            parsed.sources.iter().any(|source| source.url == url),
            "structured stream source was lost: {url}; got {:?}",
            parsed.sources
        );
    }
    assert!(parsed
        .sources
        .iter()
        .all(|source| source.provider == "grok_responses"));
}

#[tokio::test]
async fn responses_sources_survive_compact_completion() {
    let mut chunks = responses_provenance_chunks();
    chunks.push(response_event(json!({"type": "response.completed"})));
    let raw = read_responses_stream(chunks, true).await.expect("SSE JSON");
    let parsed = parse_grok_responses(&raw).expect("answer with streamed sources");

    assert_eq!(parsed.content, "streamed answer", "text must appear once");
    assert_streamed_sources(&parsed);
    assert_eq!(parsed.sources.len(), 4);
}

#[tokio::test]
async fn responses_sources_survive_other_stream_endings() {
    for (name, terminal, keep_open) in [
        ("DONE marker", b"data: [DONE]\n\n".to_vec(), true),
        (
            "named completion",
            b"event: done\ndata: {}\n\n".to_vec(),
            true,
        ),
        ("EOF", Vec::new(), false),
        (
            "compact response object",
            response_event(json!({
                "type": "response.completed",
                "response": {"id": "response-1", "status": "completed", "output": []}
            })),
            true,
        ),
    ] {
        let mut chunks = responses_provenance_chunks();
        chunks.push(terminal);
        let raw = read_responses_stream(chunks, keep_open)
            .await
            .unwrap_or_else(|err| panic!("{name}: {err}"));
        let parsed = parse_grok_responses(&raw)
            .unwrap_or_else(|err| panic!("{name} lost the accumulated answer: {err}"));

        assert_eq!(parsed.content, "streamed answer", "{name}");
        assert_streamed_sources(&parsed);
        assert_eq!(parsed.sources.len(), 4, "{name}");
    }
}

#[tokio::test]
async fn responses_final_text_wins_and_keeps_event_only_sources() {
    let mut chunks = responses_provenance_chunks();
    chunks.push(response_event(json!({
        "type": "response.completed",
        "response": {
            "id": "response-1",
            "status": "completed",
            "output": [{
                "id": "message-1",
                "type": "message",
                "content": [{
                    "type": "output_text",
                    "text": "Final answer.",
                    "annotations": [
                        {"url": "https://example.com/annotation", "title": "Final annotation"},
                        {"url": "https://example.com/final", "title": "Final source"}
                    ]
                }]
            }]
        }
    })));
    let raw = read_responses_stream(chunks, true).await.expect("SSE JSON");
    let parsed = parse_grok_responses(&raw).expect("complete final response");

    assert_eq!(
        parsed.content, "Final answer.",
        "final text is authoritative"
    );
    assert_streamed_sources(&parsed);
    assert_eq!(
        parsed.sources.len(),
        5,
        "repeated URLs must be deduplicated"
    );
    assert_eq!(parsed.sources[0].url, "https://example.com/annotation");
    assert_eq!(parsed.sources[0].title.as_deref(), Some("Final annotation"));
    assert_eq!(parsed.sources[1].url, "https://example.com/final");
}

#[tokio::test]
async fn responses_completed_parts_supply_text_without_deltas() {
    let mut chunks = Vec::new();
    for (content_index, text) in [(0, "First paragraph."), (1, "Second paragraph.")] {
        chunks.push(response_event(json!({
            "type": "response.content_part.done",
            "output_index": 0,
            "item_id": "message-1",
            "content_index": content_index,
            "part": {
                "type": "output_text",
                "text": text,
                "annotations": [{"url": "https://example.com/part", "title": "Part source"}]
            }
        })));
    }
    chunks.push(response_event(json!({"type": "response.completed"})));
    let raw = read_responses_stream(chunks, true).await.expect("SSE JSON");
    let parsed = parse_grok_responses(&raw).expect("completed parts contain the answer");

    assert_eq!(parsed.content, "First paragraph.\nSecond paragraph.");
    assert_eq!(parsed.sources.len(), 1);
    assert_eq!(parsed.sources[0].url, "https://example.com/part");
}

#[tokio::test]
async fn responses_metadata_only_stream_retains_sources() {
    let chunks = vec![
        response_event(json!({
            "type": "response.output_text.annotation.added",
            "output_index": 0,
            "content_index": 0,
            "annotation_index": 0,
            "annotation": {"type": "url_citation", "url": "https://example.com/metadata"}
        })),
        response_event(json!({"type": "response.completed"})),
    ];
    let raw = read_responses_stream(chunks, true).await.expect("SSE JSON");
    let parsed = parse_grok_responses(&raw).expect("metadata must survive without text");

    assert!(parsed.content.is_empty());
    assert_eq!(parsed.sources.len(), 1);
    assert_eq!(parsed.sources[0].url, "https://example.com/metadata");
}

#[tokio::test]
async fn responses_terminal_failure_overrides_accumulated_text_and_sources() {
    for terminal in ["response.failed", "response.incomplete"] {
        let mut chunks = responses_provenance_chunks();
        chunks.push(response_event(json!({
            "type": terminal,
            "response": {"error": {"message": "upstream stopped"}}
        })));
        let err = read_responses_stream(chunks, true)
            .await
            .expect_err("partial provenance cannot turn a failed response into success");

        assert!(
            err.to_string().contains(terminal),
            "unexpected error: {err}"
        );
        assert!(err.to_string().contains("upstream stopped"), "{err}");
    }
}

#[tokio::test]
async fn responses_partial_snapshot_extends_with_deltas_without_repeating_text() {
    let chunks = vec![
        response_event(json!({
            "type": "response.content_part.added",
            "output_index": 0,
            "content_index": 0,
            "part": {
                "type": "output_text",
                "text": "Snapshot ",
                "annotations": [{"url": "https://example.com/snapshot"}]
            }
        })),
        response_event(json!({
            "type": "response.output_text.delta",
            "output_index": 0,
            "content_index": 0,
            "delta": "continued."
        })),
        response_event(json!({"type": "response.completed"})),
    ];
    let raw = read_responses_stream(chunks, true).await.expect("SSE JSON");
    let parsed = parse_grok_responses(&raw).expect("snapshot and deltas form one answer");

    assert_eq!(parsed.content, "Snapshot continued.");
    assert_eq!(parsed.sources.len(), 1);
    assert_eq!(parsed.sources[0].url, "https://example.com/snapshot");
}

#[tokio::test]
async fn responses_text_follows_output_and_content_indices() {
    let mut chunks = Vec::new();
    for (output_index, content_index, text) in [
        (3, 0, "Later output."),
        (1, 1, "Second part."),
        (1, 0, "First part."),
    ] {
        chunks.push(response_event(json!({
            "type": "response.output_text.delta",
            "output_index": output_index,
            "content_index": content_index,
            "delta": text
        })));
    }
    chunks.push(response_event(json!({"type": "response.completed"})));
    let raw = read_responses_stream(chunks, true).await.expect("SSE JSON");
    let parsed = parse_grok_responses(&raw).expect("ordered output parts");

    assert_eq!(parsed.content, "First part.\nSecond part.\nLater output.");
    assert!(
        parsed.sources.is_empty(),
        "text ordering adds no provenance"
    );
}
