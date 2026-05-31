use axum::Json;
use axum::body::{Bytes, to_bytes};
use axum::extract::State;
use axum::http::header::{ALLOW, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::{Map, Value, json};

use super::{CheckSnippetRequest, ServerService};

const MCP_SESSION_ID: &str = "mcp-session-id";
const JSONRPC_VERSION: &str = "2.0";
const PROTOCOL_VERSION: &str = "2024-11-05";

pub(super) async fn post_mcp(
    State(service): State<ServerService>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !accepts_mcp_response(&headers) {
        return jsonrpc_error(
            StatusCode::NOT_ACCEPTABLE,
            Value::Null,
            -32000,
            "Not Acceptable: Client must accept both application/json and text/event-stream",
        );
    }

    let session_id = headers
        .get(MCP_SESSION_ID)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let has_valid_session = session_id
        .as_deref()
        .is_some_and(|session_id| service.has_mcp_session(session_id));

    if !has_json_content_type(&headers) {
        return if has_valid_session {
            unsupported_media_type_response()
        } else {
            bad_session_response()
        };
    }

    let payload = match serde_json::from_slice::<Value>(&body) {
        Ok(payload) => payload,
        Err(error) => {
            return syntax_error_response(super::json_syntax_error_message(&body, &error));
        }
    };
    let mut response = match payload {
        Value::Array(messages) => handle_mcp_batch(service, session_id.as_deref(), messages).await,
        payload => handle_mcp_request(service, session_id.as_deref(), payload).await,
    };
    if has_valid_session && response.status() != StatusCode::ACCEPTED {
        let value = HeaderValue::from_str(session_id.as_deref().expect("session id exists"))
            .expect("valid MCP session id");
        response
            .headers_mut()
            .insert(HeaderName::from_static(MCP_SESSION_ID), value);
    }
    response
}

pub(super) async fn method_not_allowed() -> Response {
    let mut response = (
        StatusCode::METHOD_NOT_ALLOWED,
        Json(json!({ "error": "Method Not Allowed" })),
    )
        .into_response();
    response
        .headers_mut()
        .insert(ALLOW, HeaderValue::from_static("POST"));
    response
}

async fn handle_mcp_request(
    service: ServerService,
    session_id: Option<&str>,
    payload: Value,
) -> Response {
    let request_id = request_id(&payload);
    let Some(method) = payload.get("method").and_then(Value::as_str) else {
        return jsonrpc_error(
            StatusCode::BAD_REQUEST,
            Value::Null,
            -32700,
            "Parse error: Invalid JSON-RPC message",
        );
    };

    if let Some(session_id) = session_id {
        if !service.has_mcp_session(session_id) {
            return bad_session_response();
        }
    } else if method != "initialize" {
        return bad_session_response();
    }

    match method {
        "initialize" => initialize(service, request_id),
        "notifications/initialized" => StatusCode::ACCEPTED.into_response(),
        "tools/list" => jsonrpc_result(request_id, tools_list_result()),
        "tools/call" => call_tool(service, request_id, payload),
        "resources/list" => jsonrpc_result(request_id, resources_list_result()),
        "resources/read" => read_resource(service, request_id, payload),
        _ => jsonrpc_error(StatusCode::OK, request_id, -32601, "Method not found"),
    }
}

async fn handle_mcp_batch(
    service: ServerService,
    session_id: Option<&str>,
    messages: Vec<Value>,
) -> Response {
    let mut results = Vec::new();
    let mut response_session_id = None;

    for message in messages {
        let response = handle_mcp_request(service.clone(), session_id, message).await;
        if response_session_id.is_none() {
            response_session_id = response
                .headers()
                .get(MCP_SESSION_ID)
                .and_then(|value| value.to_str().ok())
                .map(str::to_string);
        }
        if response.status() == StatusCode::ACCEPTED {
            continue;
        }

        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_default();
        if bytes.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_slice::<Value>(&bytes) {
            results.push(value);
        }
    }

    let mut response = match results.len() {
        0 => StatusCode::ACCEPTED.into_response(),
        1 => Json(results.pop().expect("one response")).into_response(),
        _ => Json(Value::Array(results)).into_response(),
    };
    if let Some(session_id) = response_session_id {
        let value = HeaderValue::from_str(&session_id).expect("valid MCP session id");
        response
            .headers_mut()
            .insert(HeaderName::from_static(MCP_SESSION_ID), value);
    }
    response
}

fn initialize(service: ServerService, request_id: Value) -> Response {
    let session_id = service.create_mcp_session();
    let mut response = jsonrpc_result(
        request_id,
        json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {
                "logging": {},
                "tools": {
                    "listChanged": true,
                },
                "resources": {
                    "listChanged": true,
                },
            },
            "serverInfo": {
                "name": "jscpd-server",
                "version": env!("CARGO_PKG_VERSION"),
            },
        }),
    );
    let value = HeaderValue::from_str(&session_id).expect("valid MCP session id");
    response
        .headers_mut()
        .insert(HeaderName::from_static(MCP_SESSION_ID), value);
    response
}

fn call_tool(service: ServerService, request_id: Value, payload: Value) -> Response {
    let params = match payload.get("params").and_then(Value::as_object) {
        Some(params) => params,
        None => {
            return jsonrpc_error(
                StatusCode::OK,
                request_id,
                -32602,
                "Invalid params: params must be an object",
            );
        }
    };
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return jsonrpc_error(
            StatusCode::OK,
            request_id,
            -32602,
            "Invalid params: name must be a string",
        );
    };
    let arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let result = match name {
        "check_duplication" => check_duplication_tool(service, arguments),
        "get_statistics" => get_statistics_tool(service),
        "check_current_directory" => check_current_directory_tool(service),
        _ => Err(format!("MCP error -32602: Tool {name} not found")),
    };

    match result {
        Ok(result) => jsonrpc_result(request_id, result),
        Err(message) => jsonrpc_result(request_id, tool_error(message)),
    }
}

fn check_duplication_tool(
    service: ServerService,
    arguments: Map<String, Value>,
) -> Result<Value, String> {
    let code = string_argument(&arguments, "code", "check_duplication")?;
    let format = string_argument(&arguments, "format", "check_duplication")?;
    let recheck = bool_argument(&arguments, "recheck", "check_duplication")?.unwrap_or(false);

    if recheck {
        service
            .recheck()
            .map_err(|error| format!("Error checking duplication: {error}"))?;
    }
    let response = service
        .check_snippet(CheckSnippetRequest { code, format })
        .map_err(|error| format!("Error checking duplication: {error}"))?;
    Ok(text_content(
        serde_json::to_string_pretty(&response)
            .map_err(|error| format!("Error checking duplication: {error}"))?,
    ))
}

fn get_statistics_tool(service: ServerService) -> Result<Value, String> {
    let statistics = service.statistics();
    Ok(text_content(
        serde_json::to_string_pretty(&statistics)
            .map_err(|error| format!("Error getting statistics: {error}"))?,
    ))
}

fn check_current_directory_tool(service: ServerService) -> Result<Value, String> {
    service
        .recheck()
        .map_err(|error| format!("Error starting recheck: {error}"))?;
    let statistics = service.statistics();
    Ok(text_content(serde_json::to_string(&statistics).map_err(
        |error| format!("Error starting recheck: {error}"),
    )?))
}

fn read_resource(service: ServerService, request_id: Value, payload: Value) -> Response {
    let uri = payload
        .get("params")
        .and_then(Value::as_object)
        .and_then(|params| params.get("uri"))
        .and_then(Value::as_str);
    match uri {
        Some("jscpd://statistics") => {
            let statistics = service.statistics();
            match serde_json::to_string_pretty(&statistics) {
                Ok(text) => jsonrpc_result(
                    request_id,
                    json!({
                        "contents": [{
                            "uri": "jscpd://statistics",
                            "text": text,
                        }],
                    }),
                ),
                Err(error) => jsonrpc_error(
                    StatusCode::OK,
                    request_id,
                    -32603,
                    format!("Error getting statistics resource: {error}"),
                ),
            }
        }
        Some(uri) => jsonrpc_error(
            StatusCode::OK,
            request_id,
            -32602,
            format!("MCP error -32602: Resource {uri} not found"),
        ),
        None => jsonrpc_error(
            StatusCode::OK,
            request_id,
            -32602,
            "Invalid params: uri must be a string",
        ),
    }
}

fn tools_list_result() -> Value {
    json!({
        "tools": [
            {
                "name": "check_duplication",
                "description": "Check code snippet for duplications against the codebase",
                "inputSchema": {
                    "$schema": "http://json-schema.org/draft-07/schema#",
                    "type": "object",
                    "properties": {
                        "code": {
                            "type": "string",
                            "description": "Source code snippet to check for duplications",
                        },
                        "format": {
                            "type": "string",
                            "description": "Format of the code (e.g., \"javascript\", \"typescript\", \"python\")",
                        },
                        "recheck": {
                            "type": "boolean",
                            "description": "Trigger a re-scan of the current working directory before checking",
                        },
                    },
                    "required": ["code", "format"],
                },
                "execution": {
                    "taskSupport": "forbidden",
                },
            },
            {
                "name": "get_statistics",
                "description": "Get overall project duplication statistics",
                "inputSchema": {
                    "$schema": "http://json-schema.org/draft-07/schema#",
                    "type": "object",
                    "properties": {},
                },
                "execution": {
                    "taskSupport": "forbidden",
                },
            },
            {
                "name": "check_current_directory",
                "description": "Trigger a re-scan of the current working directory for duplications",
                "inputSchema": {
                    "$schema": "http://json-schema.org/draft-07/schema#",
                    "type": "object",
                    "properties": {},
                },
                "execution": {
                    "taskSupport": "forbidden",
                },
            },
        ],
    })
}

fn resources_list_result() -> Value {
    json!({
        "resources": [{
            "uri": "jscpd://statistics",
            "name": "statistics",
            "description": "Get overall project duplication statistics",
            "mimeType": "application/json",
        }],
    })
}

fn accepts_mcp_response(headers: &HeaderMap) -> bool {
    let Some(accept) = headers.get("accept").and_then(|value| value.to_str().ok()) else {
        return false;
    };
    accept.contains("application/json") && accept.contains("text/event-stream")
}

fn has_json_content_type(headers: &HeaderMap) -> bool {
    let Some(content_type) = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    let mime = content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    mime == "application/json" || mime.ends_with("+json")
}

fn unsupported_media_type_response() -> Response {
    jsonrpc_error(
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
        Value::Null,
        -32000,
        "Unsupported Media Type: Content-Type must be application/json",
    )
}

fn syntax_error_response(message: String) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({
            "error": "SyntaxError",
            "message": message,
            "statusCode": 400,
        })),
    )
        .into_response()
}

fn string_argument(
    arguments: &Map<String, Value>,
    name: &str,
    tool_name: &str,
) -> Result<String, String> {
    let Some(value) = arguments.get(name) else {
        return Err(input_validation_error(
            tool_name,
            "string",
            name,
            "undefined",
        ));
    };
    let Some(value) = value.as_str() else {
        return Err(input_validation_error(
            tool_name,
            "string",
            name,
            received_type(value),
        ));
    };
    Ok(value.to_string())
}

fn bool_argument(
    arguments: &Map<String, Value>,
    name: &str,
    tool_name: &str,
) -> Result<Option<bool>, String> {
    let Some(value) = arguments.get(name) else {
        return Ok(None);
    };
    value
        .as_bool()
        .map(Some)
        .ok_or_else(|| input_validation_error(tool_name, "boolean", name, received_type(value)))
}

fn input_validation_error(tool_name: &str, expected: &str, field: &str, received: &str) -> String {
    format!(
        "MCP error -32602: Input validation error: Invalid arguments for tool {tool_name}: [\n  {{\n    \"expected\": \"{expected}\",\n    \"code\": \"invalid_type\",\n    \"path\": [\n      \"{field}\"\n    ],\n    \"message\": \"Invalid input: expected {expected}, received {received}\"\n  }}\n]"
    )
}

fn received_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn request_id(payload: &Value) -> Value {
    payload.get("id").cloned().unwrap_or(Value::Null)
}

fn jsonrpc_result(id: Value, result: Value) -> Response {
    Json(json!({
        "jsonrpc": JSONRPC_VERSION,
        "id": id,
        "result": result,
    }))
    .into_response()
}

fn jsonrpc_error(status: StatusCode, id: Value, code: i64, message: impl Into<String>) -> Response {
    (
        status,
        Json(json!({
            "jsonrpc": JSONRPC_VERSION,
            "id": id,
            "error": {
                "code": code,
                "message": message.into(),
            },
        })),
    )
        .into_response()
}

fn bad_session_response() -> Response {
    jsonrpc_error(
        StatusCode::BAD_REQUEST,
        Value::Null,
        -32000,
        "Bad Request: No valid session ID provided",
    )
}

fn text_content(text: String) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": text,
        }],
    })
}

fn tool_error(message: String) -> Value {
    json!({
        "isError": true,
        "content": [{
            "type": "text",
            "text": message,
        }],
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use axum::body::to_bytes;
    use serde_json::json;

    use crate::cli::Options;

    use super::*;

    fn fixture_project() -> PathBuf {
        let mut path = std::env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        path.push(format!("jscpd-rs-mcp-{stamp}"));
        fs::create_dir_all(&path).expect("create temp project");
        let content = "const alpha = 1;\nconst beta = 2;\nconst gamma = alpha + beta;\n";
        fs::write(path.join("a.js"), content).expect("write a.js");
        fs::write(path.join("b.js"), content).expect("write b.js");
        path
    }

    fn service_for(path: &Path) -> ServerService {
        let options = Options {
            paths: vec![path.to_path_buf()],
            min_tokens: 5,
            min_lines: 2,
            max_size_bytes: 1024 * 1024,
            ..Options::default()
        };
        let service = ServerService::new(path.to_path_buf(), options);
        service.initialize().expect("initialize");
        service
    }

    async fn response_json(response: Response) -> (StatusCode, HeaderMap, Value) {
        let (parts, body) = response.into_parts();
        let bytes = to_bytes(body, usize::MAX).await.expect("response body");
        let value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).expect("json body")
        };
        (parts.status, parts.headers, value)
    }

    fn mcp_headers(session_id: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            "accept",
            HeaderValue::from_static("application/json, text/event-stream"),
        );
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        if let Some(session_id) = session_id {
            headers.insert(
                HeaderName::from_static(MCP_SESSION_ID),
                HeaderValue::from_str(session_id).expect("session header"),
            );
        }
        headers
    }

    fn mcp_headers_with_content_type(session_id: Option<&str>, content_type: &str) -> HeaderMap {
        let mut headers = mcp_headers(session_id);
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_str(content_type).expect("content-type header"),
        );
        headers
    }

    #[tokio::test]
    async fn mcp_initialize_creates_session() {
        let path = fixture_project();
        let service = service_for(&path);

        let response = handle_mcp_request(
            service.clone(),
            None,
            json!({
                "jsonrpc": "2.0",
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": { "name": "test-client", "version": "1.0.0" },
                },
                "id": 1,
            }),
        )
        .await;
        let (status, headers, body) = response_json(response).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["jsonrpc"], "2.0");
        assert_eq!(body["id"], 1);
        assert_eq!(body["result"]["serverInfo"]["name"], "jscpd-server");
        assert_eq!(body["result"]["capabilities"]["tools"]["listChanged"], true);
        assert_eq!(
            body["result"]["capabilities"]["resources"]["listChanged"],
            true
        );
        let session_id = headers
            .get(MCP_SESSION_ID)
            .and_then(|value| value.to_str().ok())
            .expect("session id");
        assert!(service.has_mcp_session(session_id));
        fs::remove_dir_all(path).ok();
    }

    #[tokio::test]
    async fn mcp_tools_list_matches_upstream_schema_shape() {
        let path = fixture_project();
        let service = service_for(&path);
        let session_id = service.create_mcp_session();

        let response = handle_mcp_request(
            service,
            Some(&session_id),
            json!({
                "jsonrpc": "2.0",
                "method": "tools/list",
                "id": 2,
            }),
        )
        .await;
        let (status, _headers, body) = response_json(response).await;

        assert_eq!(status, StatusCode::OK);
        for tool in body["result"]["tools"].as_array().expect("tools list") {
            assert_eq!(
                tool["inputSchema"]["$schema"],
                "http://json-schema.org/draft-07/schema#"
            );
            assert_eq!(tool["execution"]["taskSupport"], "forbidden");
        }
        fs::remove_dir_all(path).ok();
    }

    #[tokio::test]
    async fn mcp_batch_requests_match_upstream_sdk_shape() {
        let path = fixture_project();
        let service = service_for(&path);
        let session_id = service.create_mcp_session();

        let response = post_mcp(
            State(service.clone()),
            mcp_headers(Some(&session_id)),
            Bytes::from(
                json!([{
                    "jsonrpc": "2.0",
                    "method": "tools/list",
                    "id": 5,
                }])
                .to_string(),
            ),
        )
        .await;
        let (status, headers, body) = response_json(response).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["id"], 5);
        assert!(body["result"]["tools"].is_array());
        assert_eq!(
            headers
                .get(MCP_SESSION_ID)
                .and_then(|value| value.to_str().ok()),
            Some(session_id.as_str())
        );

        let response = post_mcp(
            State(service.clone()),
            mcp_headers(Some(&session_id)),
            Bytes::from(
                json!([
                    {
                        "jsonrpc": "2.0",
                        "method": "tools/list",
                        "id": 6,
                    },
                    {
                        "jsonrpc": "2.0",
                        "method": "resources/list",
                        "id": 7,
                    }
                ])
                .to_string(),
            ),
        )
        .await;
        let (status, _headers, body) = response_json(response).await;

        assert_eq!(status, StatusCode::OK);
        let responses = body.as_array().expect("batch responses");
        assert_eq!(responses.len(), 2);
        assert_eq!(responses[0]["id"], 6);
        assert_eq!(responses[1]["id"], 7);
        assert!(responses[1]["result"]["resources"].is_array());

        let response = post_mcp(
            State(service),
            mcp_headers(Some(&session_id)),
            Bytes::from(json!([]).to_string()),
        )
        .await;
        let (status, headers, body) = response_json(response).await;

        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(body, Value::Null);
        assert!(
            headers.get(MCP_SESSION_ID).is_none(),
            "upstream does not echo session IDs on accepted notification-only batches"
        );
        fs::remove_dir_all(path).ok();
    }

    #[tokio::test]
    async fn mcp_initialized_notification_omits_session_echo() {
        let path = fixture_project();
        let service = service_for(&path);
        let session_id = service.create_mcp_session();

        let response = post_mcp(
            State(service),
            mcp_headers(Some(&session_id)),
            Bytes::from(
                json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/initialized",
                })
                .to_string(),
            ),
        )
        .await;
        let (status, headers, body) = response_json(response).await;

        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(body, Value::Null);
        assert!(
            headers.get(MCP_SESSION_ID).is_none(),
            "upstream does not echo session IDs on accepted notifications"
        );
        fs::remove_dir_all(path).ok();
    }

    #[tokio::test]
    async fn mcp_rejects_unsupported_content_type_like_upstream_sdk() {
        let path = fixture_project();
        let service = service_for(&path);
        let initialize_payload = Bytes::from(
            json!({
                "jsonrpc": "2.0",
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": { "name": "test-client", "version": "1.0.0" },
                },
                "id": 1,
            })
            .to_string(),
        );

        let response = post_mcp(
            State(service.clone()),
            mcp_headers_with_content_type(None, "text/plain"),
            initialize_payload,
        )
        .await;
        let (status, _headers, body) = response_json(response).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"]["code"], -32000);
        assert_eq!(
            body["error"]["message"],
            "Bad Request: No valid session ID provided"
        );

        let session_id = service.create_mcp_session();
        let response = post_mcp(
            State(service),
            mcp_headers_with_content_type(Some(&session_id), "text/plain"),
            Bytes::from(
                json!({
                    "jsonrpc": "2.0",
                    "method": "tools/list",
                    "id": 2,
                })
                .to_string(),
            ),
        )
        .await;
        let (status, headers, body) = response_json(response).await;

        assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
        assert_eq!(body["id"], Value::Null);
        assert_eq!(body["error"]["code"], -32000);
        assert_eq!(
            body["error"]["message"],
            "Unsupported Media Type: Content-Type must be application/json"
        );
        assert!(
            headers.get(MCP_SESSION_ID).is_none(),
            "upstream SDK does not echo session IDs on unsupported media type errors"
        );
        fs::remove_dir_all(path).ok();
    }

    #[tokio::test]
    async fn mcp_rejects_non_initialize_without_session() {
        let path = fixture_project();
        let service = service_for(&path);

        let response = handle_mcp_request(
            service,
            None,
            json!({
                "jsonrpc": "2.0",
                "method": "tools/list",
                "id": 2,
            }),
        )
        .await;
        let (status, _headers, body) = response_json(response).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"]["code"], -32000);
        fs::remove_dir_all(path).ok();
    }

    #[tokio::test]
    async fn mcp_check_duplication_tool_returns_content() {
        let path = fixture_project();
        let service = service_for(&path);
        let session_id = service.create_mcp_session();

        let response = handle_mcp_request(
            service,
            Some(&session_id),
            json!({
                "jsonrpc": "2.0",
                "method": "tools/call",
                "params": {
                    "name": "check_duplication",
                    "arguments": {
                        "code": "const alpha = 1;\nconst beta = 2;\nconst gamma = alpha + beta;\n",
                        "format": "javascript",
                        "recheck": true,
                    },
                },
                "id": 3,
            }),
        )
        .await;
        let (status, _headers, body) = response_json(response).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["id"], 3);
        let content = body["result"]["content"][0]["text"]
            .as_str()
            .expect("text content");
        assert!(content.contains("duplications"));
        assert!(content.contains("totalDuplications"));
        fs::remove_dir_all(path).ok();
    }

    #[tokio::test]
    async fn mcp_statistics_resource_returns_stats() {
        let path = fixture_project();
        let service = service_for(&path);
        let session_id = service.create_mcp_session();

        let response = handle_mcp_request(
            service,
            Some(&session_id),
            json!({
                "jsonrpc": "2.0",
                "method": "resources/read",
                "params": { "uri": "jscpd://statistics" },
                "id": 4,
            }),
        )
        .await;
        let (status, _headers, body) = response_json(response).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["id"], 4);
        assert_eq!(body["result"]["contents"][0]["uri"], "jscpd://statistics");
        assert!(
            body["result"]["contents"][0].get("mimeType").is_none(),
            "upstream resource reads omit content mimeType"
        );
        let content = body["result"]["contents"][0]["text"]
            .as_str()
            .expect("text content");
        assert!(content.contains("statistics"));
        fs::remove_dir_all(path).ok();
    }
}
