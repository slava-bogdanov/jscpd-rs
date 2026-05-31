use std::collections::HashSet;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use anyhow::{Context, Result, bail};
use axum::body::Bytes;
use axum::extract::DefaultBodyLimit;
use axum::extract::State;
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::cli::{Options, store_warning};
use crate::detector::{DetectionResult, Fragment, Statistics};
use crate::detector::{PreparedSourceDraft, detect_prepared_drafts, prepare_source_drafts};
use crate::files::{self, SourceFile};

mod mcp;

#[derive(Clone)]
pub struct ServerService {
    state: Arc<RwLock<ServiceState>>,
}

#[derive(Clone)]
struct ServiceState {
    working_directory: PathBuf,
    options: Options,
    project_drafts: Vec<PreparedSourceDraft>,
    statistics: Option<Statistics>,
    last_scan_time: Option<String>,
    is_scanning: bool,
    snippet_counter: u64,
    mcp_counter: u64,
    mcp_sessions: HashSet<String>,
}

impl ServerService {
    pub fn new(working_directory: PathBuf, options: Options) -> Self {
        Self {
            state: Arc::new(RwLock::new(ServiceState {
                working_directory,
                options,
                project_drafts: Vec::new(),
                statistics: None,
                last_scan_time: None,
                is_scanning: false,
                snippet_counter: 0,
                mcp_counter: 0,
                mcp_sessions: HashSet::new(),
            })),
        }
    }

    pub fn initialize(&self) -> Result<()> {
        self.recheck()
    }

    pub fn recheck(&self) -> Result<()> {
        let options = {
            let mut state = self.state.write().expect("server state lock poisoned");
            if state.is_scanning {
                bail!(SCAN_IN_PROGRESS);
            }
            state.is_scanning = true;
            detection_options(&state.options)
        };

        let result = scan_project(&options);
        let mut state = self.state.write().expect("server state lock poisoned");
        state.is_scanning = false;

        let (project_drafts, detection_result) = result?;
        state.project_drafts = project_drafts;
        state.statistics = Some(detection_result.statistics);
        state.last_scan_time = Some(now_rfc3339());
        Ok(())
    }

    pub fn check_snippet(&self, request: CheckSnippetRequest) -> Result<CheckSnippetResponse> {
        if request.code.trim().is_empty() {
            bail!(FIELD_CODE_EMPTY);
        }

        let (options, project_drafts, snippet_id, working_directory) = {
            let mut state = self.state.write().expect("server state lock poisoned");
            if state.is_scanning {
                bail!(SCAN_IN_PROGRESS);
            }
            if state.statistics.is_none() {
                bail!(NOT_INITIALIZED);
            }
            let snippet_id = format!("<snippet>/snippet_{:08x}", state.snippet_counter);
            state.snippet_counter += 1;
            (
                detection_options(&state.options),
                state.project_drafts.clone(),
                snippet_id,
                state.working_directory.clone(),
            )
        };

        let total_lines = request.code.split('\n').count();
        let mut prepared_drafts = project_drafts;
        prepared_drafts.extend(prepare_source_drafts(
            vec![SourceFile {
                source_id: snippet_id.clone(),
                format: request.format,
                content: request.code,
            }],
            &options,
        ));
        let result = detect_prepared_drafts(prepared_drafts, &options);
        let duplications = result
            .clones
            .iter()
            .filter_map(|clone| {
                let snippet_is_a = clone.duplication_a.source_id == snippet_id;
                let snippet_is_b = clone.duplication_b.source_id == snippet_id;
                if snippet_is_a == snippet_is_b {
                    return None;
                }
                let (snippet, codebase) = if snippet_is_a {
                    (&clone.duplication_a, &clone.duplication_b)
                } else {
                    (&clone.duplication_b, &clone.duplication_a)
                };
                Some(SnippetDuplication {
                    snippet_location: SnippetLocation::from_fragment(snippet),
                    codebase_location: DuplicationLocation::from_fragment(
                        codebase,
                        &working_directory,
                        &result,
                    ),
                    lines_count: fragment_line_count(snippet),
                })
            })
            .collect::<Vec<_>>();
        let statistics = duplication_statistics(&duplications, total_lines);

        Ok(CheckSnippetResponse {
            duplications,
            statistics,
        })
    }

    pub fn statistics(&self) -> StatsResponse {
        let state = self.state.read().expect("server state lock poisoned");
        StatsResponse {
            statistics: state.statistics.clone(),
            timestamp: state.last_scan_time.clone().unwrap_or_else(now_rfc3339),
        }
    }

    pub fn health(&self) -> HealthResponse {
        let state = self.state.read().expect("server state lock poisoned");
        HealthResponse {
            status: if state.is_scanning {
                "initializing"
            } else {
                "ready"
            },
            working_directory: state.working_directory.display().to_string(),
            last_scan_time: state.last_scan_time.clone(),
        }
    }

    pub(crate) fn create_mcp_session(&self) -> String {
        let mut state = self.state.write().expect("server state lock poisoned");
        let timestamp = OffsetDateTime::now_utc().unix_timestamp_nanos();
        let session_id = format!("{timestamp:x}-{:08x}", state.mcp_counter);
        state.mcp_counter += 1;
        state.mcp_sessions.insert(session_id.clone());
        session_id
    }

    pub(crate) fn has_mcp_session(&self, session_id: &str) -> bool {
        let state = self.state.read().expect("server state lock poisoned");
        state.mcp_sessions.contains(session_id)
    }
}

fn detection_options(options: &Options) -> Options {
    let mut options = options.clone();
    options.reporters = vec!["json".to_string()];
    options.silent = true;
    options.no_tips = true;
    options
}

fn scan_project(options: &Options) -> Result<(Vec<PreparedSourceDraft>, DetectionResult)> {
    let files = files::discover(options)?;
    let project_drafts = prepare_source_drafts(files, options);
    let result = detect_prepared_drafts(project_drafts.clone(), options);
    Ok((project_drafts, result))
}

pub fn create_router(service: ServerService) -> Router {
    Router::new()
        .route("/", get(api_info))
        .route("/api/check", post(check_snippet).fallback(not_found))
        .route("/api/recheck", post(recheck).fallback(not_found))
        .route("/api/stats", get(stats).fallback(not_found))
        .route("/api/health", get(health).fallback(not_found))
        .route(
            "/mcp",
            post(mcp::post_mcp)
                .get(mcp::method_not_allowed)
                .fallback(not_found),
        )
        .fallback(not_found)
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024))
        .with_state(service)
}

pub async fn serve(options: Options, host: &str, port: u16) -> Result<()> {
    if let Some(warning) = store_warning(&options) {
        eprintln!("{warning}");
    }
    let working_directory = server_working_directory(&options);
    let service = ServerService::new(working_directory, options);
    service.initialize()?;
    let app = create_router(service);
    let address = server_bind_address(host, port)?;
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .with_context(|| format!("failed to bind server address {address}"))?;
    println!("JSCPD server running on {}", server_display_url(host, port));
    axum::serve(listener, app).await.context("server failed")
}

fn server_bind_address(host: &str, port: u16) -> Result<SocketAddr> {
    let bind_host = if host == "true" { "0.0.0.0" } else { host };
    (bind_host, port)
        .to_socket_addrs()
        .with_context(|| format!("failed to resolve server address {host}:{port}"))?
        .next()
        .with_context(|| format!("failed to resolve server address {host}:{port}"))
}

fn server_display_url(host: &str, port: u16) -> String {
    format!("http://{host}:{port}")
}

pub fn server_working_directory(options: &Options) -> PathBuf {
    options
        .paths
        .first()
        .cloned()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

async fn api_info() -> Json<ApiInfoResponse> {
    Json(ApiInfoResponse {
        name: "jscpd-server",
        version: env!("CARGO_PKG_VERSION"),
        endpoints: [
            ("POST /api/check", "Check code snippet for duplications"),
            ("GET /api/stats", "Get overall project statistics"),
            ("GET /api/health", "Server health check"),
            ("POST /api/recheck", "Trigger recheck of the directory"),
            ("POST /mcp", "MCP Protocol endpoint"),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect(),
        documentation: "https://github.com/kucherenko/jscpd",
    })
}

async fn check_snippet(
    State(service): State<ServerService>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let request = match parse_check_payload(&headers, &body) {
        Ok(request) => request,
        Err(CheckPayloadError::Validation(message)) => {
            return error_response("ValidationError", message, 400);
        }
        Err(CheckPayloadError::Syntax(message)) => {
            return error_response("SyntaxError", message, 400);
        }
    };
    match service.check_snippet(request) {
        Ok(response) => Json(response).into_response(),
        Err(error) => error_response("Error", error.to_string(), 400),
    }
}

async fn recheck(State(service): State<ServerService>) -> Response {
    match service.recheck() {
        Ok(()) => Json(RecheckResponse {
            message: "Recheck started",
        })
        .into_response(),
        Err(error) => error_response("Error", error.to_string(), 400),
    }
}

async fn stats(State(service): State<ServerService>) -> Response {
    let response = service.statistics();
    if response.statistics.is_none() {
        return error_response(
            "NotReady",
            "Statistics not available yet. Server is still initializing.",
            503,
        );
    }
    Json(response).into_response()
}

async fn health(State(service): State<ServerService>) -> Json<HealthResponse> {
    Json(service.health())
}

async fn not_found(method: Method, uri: Uri) -> Response {
    error_response(
        "NotFound",
        format!("Route {method} {} not found", uri.path()),
        404,
    )
}

fn error_response(error: &str, message: impl Into<String>, status_code: u16) -> Response {
    let status = StatusCode::from_u16(status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (
        status,
        Json(ErrorResponse {
            error: error.to_string(),
            message: message.into(),
            status_code,
        }),
    )
        .into_response()
}

fn parse_check_payload(
    headers: &HeaderMap,
    body: &[u8],
) -> std::result::Result<CheckSnippetRequest, CheckPayloadError> {
    let content_type = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if content_type.starts_with("application/x-www-form-urlencoded") {
        return parse_check_form(body).map_err(CheckPayloadError::Validation);
    }
    let payload = serde_json::from_slice(body)
        .map_err(|error| CheckPayloadError::Syntax(json_syntax_error_message(body, &error)))?;
    parse_check_request(payload).map_err(CheckPayloadError::Validation)
}

fn parse_check_form(body: &[u8]) -> std::result::Result<CheckSnippetRequest, String> {
    let fields = form_urlencoded::parse(body)
        .into_owned()
        .collect::<Vec<_>>();
    let code = required_form_field(&fields, "code")?;
    if code.trim().is_empty() {
        return Err(FIELD_CODE_EMPTY.to_string());
    }
    let format = required_form_field(&fields, "format")?;
    if format.trim().is_empty() {
        return Err(FIELD_FORMAT_EMPTY.to_string());
    }
    Ok(CheckSnippetRequest { code, format })
}

fn parse_check_request(payload: Value) -> std::result::Result<CheckSnippetRequest, String> {
    let Some(object) = payload.as_object() else {
        return Err("Request body must be an object".to_string());
    };
    let code = required_string_field(object, "code")?;
    if code.trim().is_empty() {
        return Err(FIELD_CODE_EMPTY.to_string());
    }
    let format = required_string_field(object, "format")?;
    if format.trim().is_empty() {
        return Err(FIELD_FORMAT_EMPTY.to_string());
    }
    Ok(CheckSnippetRequest { code, format })
}

fn required_string_field(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> std::result::Result<String, String> {
    let Some(value) = object.get(field) else {
        return Err(format!("Missing required field: {field}"));
    };
    let Some(value) = value.as_str() else {
        return Err(format!("Field \"{field}\" must be a string"));
    };
    Ok(value.to_string())
}

fn required_form_field(
    fields: &[(String, String)],
    field: &str,
) -> std::result::Result<String, String> {
    fields
        .iter()
        .find_map(|(name, value)| (name == field).then(|| value.clone()))
        .ok_or_else(|| format!("Missing required field: {field}"))
}

fn json_syntax_error_message(body: &[u8], error: &serde_json::Error) -> String {
    let body = String::from_utf8_lossy(body);
    let trimmed = body.trim_start();
    if let Some(first) = trimmed.chars().next()
        && !matches!(first, '{' | '[' | '"' | '-' | '0'..='9' | 't' | 'f' | 'n')
    {
        let preview = if trimmed.chars().count() > 20 {
            format!("{}...", trimmed.chars().take(17).collect::<String>())
        } else {
            trimmed.to_string()
        };
        return format!("Unexpected token '{first}', \"{preview}\" is not valid JSON");
    }
    error.to_string()
}

fn duplication_statistics(
    duplications: &[SnippetDuplication],
    total_lines: usize,
) -> DuplicationStatistics {
    let mut duplicated = Vec::<usize>::new();
    for duplication in duplications {
        duplicated.extend(
            duplication.snippet_location.start_line..=duplication.snippet_location.end_line,
        );
    }
    duplicated.sort_unstable();
    duplicated.dedup();
    let duplicated_lines = duplicated.len();
    DuplicationStatistics {
        total_duplications: duplications.len(),
        duplicated_lines,
        total_lines,
        percentage_duplicated: percentage(total_lines, duplicated_lines),
    }
}

fn percentage(total: usize, duplicated: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        ((duplicated as f64 * 10000.0) / total as f64).round() / 100.0
    }
}

fn relative_source_id(path: &str, working_directory: &Path) -> String {
    let path_ref = Path::new(path);
    path_ref
        .strip_prefix(working_directory)
        .ok()
        .and_then(|relative| relative.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| path.to_string())
}

fn slice_fragment(result: &DetectionResult, fragment: &Fragment) -> Option<String> {
    result
        .source_contents
        .get(&fragment.source_id)
        .and_then(|content| content.get(fragment.range[0]..fragment.range[1]))
        .map(str::to_string)
}

fn fragment_line_count(fragment: &Fragment) -> usize {
    fragment.end.line.saturating_sub(fragment.start.line) + 1
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

const SCAN_IN_PROGRESS: &str = "Please wait for initial scan to complete";
const NOT_INITIALIZED: &str = "Server not initialized. Please wait for initial scan to complete.";
const FIELD_CODE_EMPTY: &str = "Field \"code\" cannot be empty";
const FIELD_FORMAT_EMPTY: &str = "Field \"format\" cannot be empty";

enum CheckPayloadError {
    Validation(String),
    Syntax(String),
}

#[derive(Clone, Debug, Deserialize)]
pub struct CheckSnippetRequest {
    pub code: String,
    pub format: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckSnippetResponse {
    pub duplications: Vec<SnippetDuplication>,
    pub statistics: DuplicationStatistics,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetDuplication {
    pub snippet_location: SnippetLocation,
    pub codebase_location: DuplicationLocation,
    pub lines_count: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetLocation {
    pub start_line: usize,
    pub end_line: usize,
    pub start_column: usize,
    pub end_column: usize,
}

impl SnippetLocation {
    fn from_fragment(fragment: &Fragment) -> Self {
        Self {
            start_line: fragment.start.line,
            end_line: fragment.end.line,
            start_column: fragment.start.column,
            end_column: fragment.end.column,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicationLocation {
    pub file: String,
    pub start_line: usize,
    pub end_line: usize,
    pub start_column: usize,
    pub end_column: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fragment: Option<String>,
}

impl DuplicationLocation {
    fn from_fragment(
        fragment: &Fragment,
        working_directory: &Path,
        result: &DetectionResult,
    ) -> Self {
        Self {
            file: relative_source_id(&fragment.source_id, working_directory),
            start_line: fragment.start.line,
            end_line: fragment.end.line,
            start_column: fragment.start.column,
            end_column: fragment.end.column,
            fragment: slice_fragment(result, fragment),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicationStatistics {
    pub total_duplications: usize,
    pub duplicated_lines: usize,
    pub total_lines: usize,
    pub percentage_duplicated: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct StatsResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statistics: Option<Statistics>,
    pub timestamp: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub status: &'static str,
    pub working_directory: String,
    pub last_scan_time: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ApiInfoResponse {
    pub name: &'static str,
    pub version: &'static str,
    pub endpoints: std::collections::BTreeMap<String, String>,
    pub documentation: &'static str,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub status_code: u16,
}

#[derive(Clone, Debug, Serialize)]
pub struct RecheckResponse {
    pub message: &'static str,
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use axum::body::{Body, to_bytes};
    use axum::http::header::CONTENT_TYPE;
    use axum::http::{Request, StatusCode};
    use serde_json::Value;
    use tower::ServiceExt;

    use crate::cli::Options;

    use super::*;

    fn fixture_project() -> PathBuf {
        let mut path = std::env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        path.push(format!("jscpd-rs-server-{stamp}"));
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
        ServerService::new(path.to_path_buf(), options)
    }

    #[test]
    fn server_initialization_populates_stats_and_health() {
        let path = fixture_project();
        let service = service_for(&path);

        service.initialize().expect("initialize");

        let stats = service.statistics();
        assert!(stats.statistics.is_some());
        assert!(stats.timestamp.contains('T'));
        let health = service.health();
        assert_eq!(health.status, "ready");
        assert_eq!(health.working_directory, path.display().to_string());
        assert!(health.last_scan_time.is_some());
        fs::remove_dir_all(path).ok();
    }

    #[test]
    fn server_host_binding_preserves_upstream_display_host() {
        let true_addr = server_bind_address("true", 3000).expect("true host bind");
        assert_eq!(true_addr.ip().to_string(), "0.0.0.0");
        assert_eq!(true_addr.port(), 3000);
        assert_eq!(server_display_url("true", 3000), "http://true:3000");
        assert_eq!(
            server_display_url("localhost", 3001),
            "http://localhost:3001"
        );
    }

    #[test]
    fn server_check_snippet_reports_codebase_duplications() {
        let path = fixture_project();
        let service = service_for(&path);
        service.initialize().expect("initialize");

        let response = service
            .check_snippet(CheckSnippetRequest {
                code: "const alpha = 1;\nconst beta = 2;\nconst gamma = alpha + beta;\n"
                    .to_string(),
                format: "javascript".to_string(),
            })
            .expect("check snippet");

        assert!(!response.duplications.is_empty());
        assert_eq!(
            response.statistics.total_duplications,
            response.duplications.len()
        );
        assert!(response.statistics.duplicated_lines > 0);
        assert!(
            response
                .duplications
                .iter()
                .all(|duplication| !duplication.codebase_location.file.starts_with("<snippet>"))
        );
        fs::remove_dir_all(path).ok();
    }

    #[test]
    fn server_check_snippet_rejects_empty_code() {
        let path = fixture_project();
        let service = service_for(&path);
        service.initialize().expect("initialize");

        let error = service
            .check_snippet(CheckSnippetRequest {
                code: "   ".to_string(),
                format: "javascript".to_string(),
            })
            .expect_err("empty code should fail");

        assert_eq!(error.to_string(), FIELD_CODE_EMPTY);
        fs::remove_dir_all(path).ok();
    }

    #[test]
    fn server_recheck_refreshes_statistics() {
        let path = fixture_project();
        let service = service_for(&path);
        service.initialize().expect("initialize");
        let before = service
            .statistics()
            .statistics
            .expect("stats before")
            .total
            .sources;
        fs::write(path.join("c.js"), "const unique = 1;\n").expect("write c.js");

        service.recheck().expect("recheck");

        let after = service
            .statistics()
            .statistics
            .expect("stats after")
            .total
            .sources;
        assert!(after > before);
        fs::remove_dir_all(path).ok();
    }

    #[tokio::test]
    async fn server_check_snippet_accepts_form_urlencoded_body() {
        let path = fixture_project();
        let service = service_for(&path);
        service.initialize().expect("initialize");
        let app = create_router(service);
        let body = form_urlencoded::Serializer::new(String::new())
            .append_pair(
                "code",
                "const alpha = 1;\nconst beta = 2;\nconst gamma = alpha + beta;\n",
            )
            .append_pair("format", "javascript")
            .finish();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/check")
                    .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(body))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let body: Value = serde_json::from_slice(&body).expect("json body");
        assert!(body["duplications"].is_array());
        assert_eq!(
            body["statistics"]["totalDuplications"].as_u64(),
            body["duplications"]
                .as_array()
                .map(|items| items.len() as u64)
        );
        fs::remove_dir_all(path).ok();
    }

    #[tokio::test]
    async fn server_check_snippet_invalid_json_matches_upstream_error() {
        let path = fixture_project();
        let service = service_for(&path);
        service.initialize().expect("initialize");
        let app = create_router(service);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/check")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from("invalid-json"))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let body: Value = serde_json::from_slice(&body).expect("json body");
        assert_eq!(body["error"], "SyntaxError");
        assert_eq!(
            body["message"],
            "Unexpected token 'i', \"invalid-json\" is not valid JSON"
        );
        assert_eq!(body["statusCode"], 400);
        fs::remove_dir_all(path).ok();
    }

    #[tokio::test]
    async fn server_check_snippet_rejects_non_string_format_like_upstream() {
        let path = fixture_project();
        let service = service_for(&path);
        service.initialize().expect("initialize");
        let app = create_router(service);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/check")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"code":"console.log(1);","format":123}"#))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let body: Value = serde_json::from_slice(&body).expect("json body");
        assert_eq!(body["error"], "ValidationError");
        assert_eq!(body["message"], "Field \"format\" must be a string");
        assert_eq!(body["statusCode"], 400);
        fs::remove_dir_all(path).ok();
    }

    #[tokio::test]
    async fn server_uninitialized_api_matches_upstream_error_shapes() {
        let path = fixture_project();
        let service = service_for(&path);
        let app = create_router(service);

        let check_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/check")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"code":"console.log(\"test\");","format":"javascript"}"#,
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(check_response.status(), StatusCode::BAD_REQUEST);
        let body = to_bytes(check_response.into_body(), usize::MAX)
            .await
            .expect("body");
        let body: Value = serde_json::from_slice(&body).expect("json body");
        assert_eq!(body["error"], "Error");
        assert_eq!(body["message"], NOT_INITIALIZED);
        assert_eq!(body["statusCode"], 400);

        let stats_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/stats")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(stats_response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = to_bytes(stats_response.into_body(), usize::MAX)
            .await
            .expect("body");
        let body: Value = serde_json::from_slice(&body).expect("json body");
        assert_eq!(body["error"], "NotReady");
        assert_eq!(
            body["message"],
            "Statistics not available yet. Server is still initializing."
        );
        assert_eq!(body["statusCode"], 503);

        let health_response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/health")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(health_response.status(), StatusCode::OK);
        let body = to_bytes(health_response.into_body(), usize::MAX)
            .await
            .expect("body");
        let body: Value = serde_json::from_slice(&body).expect("json body");
        assert!(matches!(
            body["status"].as_str(),
            Some("ready" | "initializing")
        ));
        assert_eq!(body["workingDirectory"], path.display().to_string());
        assert_eq!(body["lastScanTime"], Value::Null);
        fs::remove_dir_all(path).ok();
    }

    #[tokio::test]
    async fn server_unknown_routes_return_upstream_style_json_error() {
        let path = fixture_project();
        let service = service_for(&path);
        let app = create_router(service);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/unknown?ignored=true")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let body: Value = serde_json::from_slice(&body).expect("json body");
        assert_eq!(body["error"], "NotFound");
        assert_eq!(body["message"], "Route GET /api/unknown not found");
        assert_eq!(body["statusCode"], 404);
        fs::remove_dir_all(path).ok();
    }

    #[tokio::test]
    async fn server_wrong_api_methods_return_upstream_style_not_found() {
        let path = fixture_project();
        let service = service_for(&path);
        let app = create_router(service);

        for (method, uri) in [
            ("GET", "/api/check"),
            ("GET", "/api/recheck"),
            ("POST", "/api/stats"),
            ("POST", "/api/health"),
            ("PUT", "/api/check"),
            ("DELETE", "/api/stats"),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(uri)
                        .body(Body::empty())
                        .expect("request"),
                )
                .await
                .expect("response");

            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            let body = to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body");
            let body: Value = serde_json::from_slice(&body).expect("json body");
            assert_eq!(body["error"], "NotFound");
            assert_eq!(body["message"], format!("Route {method} {uri} not found"));
            assert_eq!(body["statusCode"], 404);
        }
        fs::remove_dir_all(path).ok();
    }

    #[tokio::test]
    async fn server_unsupported_mcp_methods_return_upstream_style_not_found() {
        let path = fixture_project();
        let service = service_for(&path);
        let app = create_router(service);

        for method in ["DELETE", "OPTIONS"] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri("/mcp")
                        .body(Body::empty())
                        .expect("request"),
                )
                .await
                .expect("response");

            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            let body = to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body");
            let body: Value = serde_json::from_slice(&body).expect("json body");
            assert_eq!(body["error"], "NotFound");
            assert_eq!(body["message"], format!("Route {method} /mcp not found"));
            assert_eq!(body["statusCode"], 404);
        }
        fs::remove_dir_all(path).ok();
    }
}
