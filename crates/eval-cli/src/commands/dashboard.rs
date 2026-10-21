use anyhow::{bail, Context, Result};
use chrono::Local;
use eval_agent::VerificationAgent;
use eval_core::dataset::{Dataset, DatasetLoader, TestCase};
use eval_core::model::{create_client, ApiProtocol, ModelConfig};
use eval_core::verifier::{get_all_baselines, ModelVerifier, STANDARD_PROBES};
use eval_suites::{BenchmarkOrchestrator, RunControl};
use percent_encoding::percent_decode_str;
use rand::RngCore;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{RwLock, Semaphore};
use tracing::{error, warn};

const MAX_HEADER_BYTES: usize = 64 * 1024;
const MAX_BODY_BYTES: usize = 1024 * 1024;
const MAX_CONNECTIONS: usize = 64;
const COOKIE_NAME: &str = "ab_token";
const CONFIG_CANDIDATES: &[&str] = &[
    "eval_config.toml",
    "agent_bench.toml",
    "eval_gpustack.toml",
    "eval_config.example.toml",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunMeta {
    pub filename: String,
    pub timestamp: String,
    pub size_bytes: u64,
    pub size_human: String,
    pub models: Vec<String>,
    pub total_cases: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveBenchmarkState {
    pub status: String,
    pub model_name: String,
    pub total_cases: usize,
    pub completed_cases: usize,
    pub passed_cases: usize,
    pub current_case: Option<String>,
    pub live_cases: Vec<LiveCaseItem>,
    pub started_at: Option<String>,
    pub elapsed_secs: u64,
    pub result_file: Option<String>,
    pub message: Option<String>,
}

impl Default for LiveBenchmarkState {
    fn default() -> Self {
        Self {
            status: "idle".to_string(),
            model_name: "Mock-Pro-v1".to_string(),
            total_cases: 0,
            completed_cases: 0,
            passed_cases: 0,
            current_case: None,
            live_cases: Vec::new(),
            started_at: None,
            elapsed_secs: 0,
            result_file: None,
            message: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveCaseItem {
    pub id: String,
    pub category: String,
    pub passed: bool,
    pub score: f64,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkRunRequest {
    pub category: Option<String>,
    pub difficulty: Option<String>,
    pub limit: Option<usize>,
    pub model: Option<String>,
    pub concurrency: Option<usize>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub protocol: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyApiRequest {
    pub model: Option<String>,
    pub target: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub protocol: Option<String>,
}

type RunsCache = Arc<Mutex<HashMap<String, (u64, SystemTime, RunMeta)>>>;

#[derive(Clone)]
struct ServerState {
    dist_dir: PathBuf,
    results_dir: PathBuf,
    live_state: Arc<RwLock<LiveBenchmarkState>>,
    control: Arc<Mutex<Option<Arc<RunControl>>>>,
    token: Arc<String>,
    allow_custom_endpoints: bool,
    default_result_file: Option<PathBuf>,
    runs_cache: RunsCache,
}

struct HttpRequest {
    method: String,
    path: String,
    query: HashMap<String, String>,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl HttpRequest {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(|s| s.as_str())
    }
}

struct Response {
    status: u16,
    content_type: String,
    body: Vec<u8>,
    set_cookie: Option<String>,
    location: Option<String>,
}

impl Response {
    fn json<T: Serialize + ?Sized>(status: u16, value: &T) -> Self {
        Self {
            status,
            content_type: "application/json; charset=utf-8".to_string(),
            body: serde_json::to_vec(value)
                .unwrap_or_else(|_| b"{\"error\":\"serialize\"}".to_vec()),
            set_cookie: None,
            location: None,
        }
    }

    fn error(status: u16, message: impl Into<String>) -> Self {
        Self::json(status, &json!({ "error": message.into() }))
    }

    fn text(status: u16, message: impl Into<String>) -> Self {
        Self {
            status,
            content_type: "text/plain; charset=utf-8".to_string(),
            body: message.into().into_bytes(),
            set_cookie: None,
            location: None,
        }
    }

    fn bytes(status: u16, content_type: impl Into<String>, body: Vec<u8>) -> Self {
        Self {
            status,
            content_type: content_type.into(),
            body,
            set_cookie: None,
            location: None,
        }
    }

    fn empty(status: u16) -> Self {
        Self {
            status,
            content_type: "text/plain; charset=utf-8".to_string(),
            body: Vec::new(),
            set_cookie: None,
            location: None,
        }
    }

    fn redirect(location: impl Into<String>, cookie: impl Into<String>) -> Self {
        Self {
            status: 302,
            content_type: "text/plain; charset=utf-8".to_string(),
            body: Vec::new(),
            set_cookie: Some(cookie.into()),
            location: Some(location.into()),
        }
    }
}

pub async fn execute(
    port: u16,
    host: String,
    token: Option<String>,
    results_path: Option<String>,
    web_dist: Option<String>,
    allow_custom_endpoints: bool,
) -> Result<()> {
    let dist_dir = resolve_dist_dir(web_dist)?;
    let results_dir = resolve_results_dir(results_path.as_deref());
    let token = token
        .filter(|t| !t.trim().is_empty())
        .or_else(|| std::env::var("AGENT_BENCH_TOKEN").ok())
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(generate_token);

    let default_result_file = results_path.as_deref().and_then(|p| {
        let path = Path::new(p);
        if path.is_file() {
            path.canonicalize().ok()
        } else {
            None
        }
    });

    let host = if host.trim().is_empty() {
        "127.0.0.1".to_string()
    } else {
        host
    };
    let display_host = if host == "0.0.0.0" || host == "::" {
        "127.0.0.1".to_string()
    } else {
        host.clone()
    };
    let addr = format!("{host}:{port}");
    let listener = TcpListener::bind(&addr)
        .await
        .with_context(|| format!("Failed to bind dashboard server to {addr}"))?;

    let state = ServerState {
        dist_dir,
        results_dir,
        live_state: Arc::new(RwLock::new(LiveBenchmarkState::default())),
        control: Arc::new(Mutex::new(None)),
        token: Arc::new(token.clone()),
        allow_custom_endpoints,
        default_result_file,
        runs_cache: Arc::new(Mutex::new(HashMap::new())),
    };
    let connection_limit = Arc::new(Semaphore::new(MAX_CONNECTIONS));

    println!("\n╔════════════════════════════════════════════════════════════════════╗");
    println!("║      🦀  agent-bench 可视化大屏服务已成功启动!                     ║");
    println!("╠════════════════════════════════════════════════════════════════════╣");
    println!("║  打开此带一次性令牌的地址（浏览器会写入 HttpOnly Cookie）:          ║");
    println!("║  http://{}:{}/?token={:<40}║", display_host, port, token);
    println!("║  令牌也可用 AGENT_BENCH_TOKEN 环境变量或 --token 指定。           ║");
    println!("║  默认只监听 127.0.0.1；如改 --host 请自行承担网络暴露风险。      ║");
    println!("╚════════════════════════════════════════════════════════════════════╝\n");

    loop {
        let (socket, _peer) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("Error accepting connection: {e}");
                continue;
            }
        };
        let permit = match Arc::clone(&connection_limit).acquire_owned().await {
            Ok(p) => p,
            Err(_) => continue,
        };
        let state = state.clone();
        tokio::spawn(async move {
            let _permit = permit;
            if let Err(e) = handle_connection(socket, state).await {
                tracing::debug!("dashboard connection error: {e}");
            }
        });
    }
}

async fn handle_connection(mut socket: TcpStream, state: ServerState) -> Result<()> {
    let req = match read_request(&mut socket).await {
        Ok(req) => req,
        Err(e) => {
            let response = Response::text(400, format!("Bad Request: {e}"));
            let _ = write_response(&mut socket, response, false).await;
            return Ok(());
        }
    };
    let is_head = req.method == "HEAD";
    let response = handle_request(&req, &state).await;
    write_response(&mut socket, response, is_head).await
}

async fn read_request(socket: &mut TcpStream) -> Result<HttpRequest> {
    let inner = async {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 8192];
        let mut parsed: Option<ParsedHead> = None;

        loop {
            if let Some(ref p) = parsed {
                if buf.len() >= p.header_end + 4 + p.content_length {
                    break;
                }
            }
            if buf.len() > MAX_HEADER_BYTES + MAX_BODY_BYTES {
                bail!("request too large");
            }
            let n = socket
                .read(&mut chunk)
                .await
                .context("failed to read request")?;
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
            if parsed.is_none() {
                if let Some(end) = find_subsequence(&buf, b"\r\n\r\n") {
                    if end > MAX_HEADER_BYTES {
                        bail!("request headers too large");
                    }
                    let head = parse_head(&buf[..end])?;
                    if head.content_length > MAX_BODY_BYTES {
                        bail!("request body too large");
                    }
                    parsed = Some(head);
                }
            }
        }

        let head = parsed.ok_or_else(|| anyhow::anyhow!("incomplete HTTP request"))?;
        if head.content_length > MAX_BODY_BYTES {
            bail!("request body too large");
        }
        if buf.len() < head.header_end + 4 {
            bail!("incomplete HTTP headers");
        }
        let body_bytes = &buf[head.header_end + 4..];
        if body_bytes.len() < head.content_length {
            bail!("incomplete HTTP body");
        }
        let body = body_bytes[..head.content_length].to_vec();
        let (path, query) = split_target(&head.target)?;
        Ok(HttpRequest {
            method: head.method,
            path,
            query,
            headers: head.headers,
            body,
        })
    };

    tokio::time::timeout(Duration::from_secs(15), inner)
        .await
        .context("request read timeout")?
}

struct ParsedHead {
    method: String,
    target: String,
    headers: HashMap<String, String>,
    content_length: usize,
    header_end: usize,
}

fn parse_head(head: &[u8]) -> Result<ParsedHead> {
    let text = std::str::from_utf8(head).context("request head is not UTF-8")?;
    let mut lines = text.split("\r\n");
    let first = lines
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing request line"))?;
    let mut parts = first.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing method"))?
        .to_ascii_uppercase();
    let target = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing request target"))?
        .to_string();
    if !target.starts_with('/') {
        bail!("only origin-form request targets are accepted");
    }

    let mut headers = HashMap::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| anyhow::anyhow!("malformed header"))?;
        headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
    }
    let content_length = match headers.get("content-length") {
        Some(v) => v
            .parse::<usize>()
            .context("invalid Content-Length header")?,
        None => 0,
    };
    let header_end = find_subsequence(head, b"\r\n\r\n").unwrap_or(head.len());

    Ok(ParsedHead {
        method,
        target,
        headers,
        content_length,
        header_end,
    })
}

fn split_target(target: &str) -> Result<(String, HashMap<String, String>)> {
    let (path, query_str) = match target.split_once('?') {
        Some((p, q)) => (p.to_string(), q),
        None => (target.to_string(), ""),
    };
    if !path.starts_with('/') || path.contains('\0') {
        bail!("invalid request path");
    }
    let mut query = HashMap::new();
    for pair in query_str.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let Ok(k) = percent_decode_str(k).decode_utf8() else {
            continue;
        };
        let Ok(v) = percent_decode_str(v).decode_utf8() else {
            continue;
        };
        query.insert(k.to_string(), v.to_string());
    }
    Ok((path, query))
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

async fn write_response(socket: &mut TcpStream, response: Response, head_only: bool) -> Result<()> {
    let reason = status_reason(response.status);
    let mut head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\nX-Frame-Options: DENY\r\nReferrer-Policy: no-referrer\r\nCache-Control: no-store\r\n",
        response.status,
        reason,
        response.content_type,
        response.body.len()
    );
    if let Some(cookie) = response.set_cookie {
        head.push_str(&format!("Set-Cookie: {cookie}\r\n"));
    }
    if let Some(location) = response.location {
        head.push_str(&format!("Location: {location}\r\n"));
    }
    head.push_str("\r\n");
    socket.write_all(head.as_bytes()).await?;
    if !head_only {
        socket.write_all(&response.body).await?;
    }
    socket.flush().await?;
    Ok(())
}

fn status_reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        204 => "No Content",
        302 => "Found",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        413 => "Payload Too Large",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        _ => "Error",
    }
}

async fn handle_request(req: &HttpRequest, state: &ServerState) -> Response {
    if req.method == "OPTIONS" {
        // No CORS headers: same-origin browser requests do not need preflight.
        return Response::empty(204);
    }
    if !same_origin(req) {
        return Response::error(403, "cross-origin POST requests are denied");
    }

    let query_token = req.query.get("token").map(|s| s.as_str());
    let has_cookie = cookie_token(req).as_deref() == Some(state.token.as_str());

    if req.path == "/" && query_token == Some(state.token.as_str()) {
        return Response::redirect("/", cookie_header(state.token.as_str()));
    }

    if !authorized(req, state.token.as_str()) {
        return if req.path.starts_with("/api/") {
            Response::error(
                401,
                "unauthorized: open the tokenized URL printed by the dashboard",
            )
        } else {
            Response::text(
                401,
                "Unauthorized. Open the URL printed by `agent-bench dashboard` (it contains ?token=...).",
            )
        };
    }

    let mut response = route(req, state).await;
    if query_token == Some(state.token.as_str()) && !has_cookie {
        response.set_cookie = Some(cookie_header(state.token.as_str()));
    }
    response
}

async fn route(req: &HttpRequest, state: &ServerState) -> Response {
    let path = req.path.as_str();

    if path == "/api/runs" {
        if req.method != "GET" && req.method != "HEAD" {
            return method_not_allowed();
        }
        let runs = scan_runs_meta_cached(state).await;
        return Response::json(200, &runs);
    }

    if let Some(rest) = path.strip_prefix("/api/runs/") {
        if req.method != "GET" && req.method != "HEAD" {
            return method_not_allowed();
        }
        return serve_run_file(rest, state).await;
    }

    if path == "/api/system" {
        if req.method != "GET" && req.method != "HEAD" {
            return method_not_allowed();
        }
        return Response::json(
            200,
            &json!({
                "engine": "agent-bench (Rust Tokio)",
                "version": env!("CARGO_PKG_VERSION"),
                "results_dir": state.results_dir.display().to_string(),
                "status": "ready",
                "auth": "cookie/header token",
            }),
        );
    }

    if path == "/api/benchmark/status" {
        if req.method != "GET" && req.method != "HEAD" {
            return method_not_allowed();
        }
        let state_guard = state.live_state.read().await;
        return Response::json(200, &*state_guard);
    }

    if path == "/api/benchmark/run" {
        if req.method != "POST" {
            return method_not_allowed();
        }
        let body: BenchmarkRunRequest = match parse_body(req) {
            Ok(body) => body,
            Err(e) => return Response::error(400, format!("invalid JSON body: {e}")),
        };
        let model = body.model.clone().unwrap_or_default();
        if model.trim().is_empty() {
            return Response::error(400, "model is required");
        }
        if let Err(e) = build_model_config(
            &model,
            body.base_url.clone(),
            body.api_key.clone(),
            body.protocol.clone(),
            state.allow_custom_endpoints,
        ) {
            return Response::error(400, e.to_string());
        }
        {
            let live = state.live_state.read().await;
            if live.status == "running" {
                return Response::error(409, "a benchmark is already running");
            }
        }
        let control = Arc::new(RunControl::new());
        if let Ok(mut guard) = state.control.lock() {
            *guard = Some(Arc::clone(&control));
        }
        let task_state = state.clone();
        tokio::spawn(async move {
            run_live_benchmark_task(task_state, body, control).await;
        });
        return Response::json(
            200,
            &json!({ "status": "started", "message": "实时评测已成功启动" }),
        );
    }

    if path == "/api/benchmark/stop" {
        if req.method != "POST" {
            return method_not_allowed();
        }
        if let Ok(mut guard) = state.control.lock() {
            if let Some(control) = guard.take() {
                control.cancel();
            }
        }
        let mut live = state.live_state.write().await;
        live.status = "idle".to_string();
        live.message = Some("已手动终止评测".to_string());
        return Response::json(200, &json!({ "status": "stopped" }));
    }

    if path == "/api/verify/probes" {
        if req.method != "GET" && req.method != "HEAD" {
            return method_not_allowed();
        }
        return Response::json(200, &STANDARD_PROBES);
    }

    if path == "/api/baselines" {
        if req.method != "GET" && req.method != "HEAD" {
            return method_not_allowed();
        }
        return Response::json(200, get_all_baselines());
    }

    if path == "/api/verify" {
        if req.method != "POST" {
            return method_not_allowed();
        }
        let body: VerifyApiRequest = match parse_body(req) {
            Ok(body) => body,
            Err(e) => return Response::error(400, format!("invalid JSON body: {e}")),
        };
        let model = body.model.clone().unwrap_or_default();
        if model.trim().is_empty() {
            return Response::error(400, "model is required");
        }
        let cfg = match build_model_config(
            &model,
            body.base_url.clone(),
            body.api_key.clone(),
            body.protocol.clone(),
            state.allow_custom_endpoints,
        ) {
            Ok(cfg) => cfg,
            Err(e) => return Response::error(400, e.to_string()),
        };
        let target = body
            .target
            .clone()
            .unwrap_or_else(|| cfg.model_name.clone());
        let client = match create_client(cfg) {
            Ok(client) => client,
            Err(e) => return Response::error(500, format!("failed to create client: {e}")),
        };
        let crosscheck_cases = load_crosscheck_cases();
        return match ModelVerifier::verify_with_benchmark(
            &*client,
            Some(&target),
            &crosscheck_cases,
        )
        .await
        {
            Ok(report) => Response::json(200, &report),
            Err(e) => Response::error(502, format!("verification failed: {e}")),
        };
    }

    if path == "/api/verify/agentic" {
        if req.method != "POST" {
            return method_not_allowed();
        }
        let body: VerifyApiRequest = match parse_body(req) {
            Ok(body) => body,
            Err(e) => return Response::error(400, format!("invalid JSON body: {e}")),
        };
        let model = body.model.clone().unwrap_or_default();
        if model.trim().is_empty() {
            return Response::error(400, "model is required");
        }
        let cfg = match build_model_config(
            &model,
            body.base_url.clone(),
            body.api_key.clone(),
            body.protocol.clone(),
            state.allow_custom_endpoints,
        ) {
            Ok(cfg) => cfg,
            Err(e) => return Response::error(400, e.to_string()),
        };
        let target = body
            .target
            .clone()
            .unwrap_or_else(|| cfg.model_name.clone());
        let client = match create_client(cfg) {
            Ok(client) => client,
            Err(e) => return Response::error(500, format!("failed to create client: {e}")),
        };
        let auditor = resolve_judge_config().and_then(|cfg| create_client(cfg).ok());
        let crosscheck_cases = load_crosscheck_cases();
        return match VerificationAgent::audit_with_benchmark(
            &*client,
            auditor.as_deref(),
            Some(&target),
            &crosscheck_cases,
        )
        .await
        {
            Ok(report) => Response::json(200, &report),
            Err(e) => Response::error(502, format!("agentic audit failed: {e}")),
        };
    }

    if path.starts_with("/api/") {
        return Response::error(404, "unknown API route");
    }

    if path == "/data/default_results.json" {
        if req.method != "GET" && req.method != "HEAD" {
            return method_not_allowed();
        }
        return serve_default_results(state).await;
    }

    if req.method != "GET" && req.method != "HEAD" {
        return method_not_allowed();
    }
    serve_static(req, state).await
}

fn method_not_allowed() -> Response {
    Response::error(405, "method not allowed")
}

async fn serve_run_file(name: &str, state: &ServerState) -> Response {
    let decoded = match percent_decode_str(name).decode_utf8() {
        Ok(s) => s.into_owned(),
        Err(_) => return Response::error(400, "invalid run filename"),
    };
    if !is_run_filename(&decoded) {
        return Response::error(400, "invalid run filename");
    }
    let candidate = state.results_dir.join(&decoded);
    let Ok(canonical) = candidate.canonicalize() else {
        return Response::error(404, "run not found");
    };
    if !canonical.starts_with(&state.results_dir) {
        return Response::error(403, "path outside results directory");
    }
    if !canonical.is_file() {
        return Response::error(404, "run not found");
    }
    match tokio::fs::read(&canonical).await {
        Ok(bytes) => Response::bytes(200, "application/json; charset=utf-8", bytes),
        Err(e) => Response::error(500, format!("failed to read run: {e}")),
    }
}

fn is_run_filename(name: &str) -> bool {
    !name.is_empty()
        && name.len() < 256
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains("..")
        && name.starts_with("eval_results_")
        && name.ends_with(".json")
}

async fn serve_static(req: &HttpRequest, state: &ServerState) -> Response {
    let decoded = match percent_decode_str(&req.path).decode_utf8() {
        Ok(s) => s.into_owned(),
        Err(_) => return Response::error(400, "invalid path encoding"),
    };
    let mut rel = decoded.trim_start_matches('/').to_string();
    if rel.is_empty() {
        rel = "index.html".to_string();
    }
    if rel.contains('\0') || rel.contains('\\') || rel.split('/').any(|c| c == "..") {
        return Response::error(400, "invalid path");
    }
    let candidate = state.dist_dir.join(&rel);
    if candidate.is_file() {
        let Ok(canonical) = candidate.canonicalize() else {
            return Response::error(404, "not found");
        };
        if !canonical.starts_with(&state.dist_dir) {
            return Response::error(403, "path outside web root");
        }
        return match tokio::fs::read(&canonical).await {
            Ok(bytes) => {
                let ct = get_content_type(Path::new(&rel));
                Response::bytes(200, ct, bytes)
            }
            Err(e) => Response::error(500, format!("failed to read asset: {e}")),
        };
    }
    if Path::new(&rel).extension().is_some() {
        return Response::error(404, "not found");
    }
    let index = state.dist_dir.join("index.html");
    match tokio::fs::read(&index).await {
        Ok(bytes) => Response::bytes(200, "text/html; charset=utf-8", bytes),
        Err(e) => Response::error(500, format!("index.html missing: {e}")),
    }
}

fn parse_body<T: DeserializeOwned>(req: &HttpRequest) -> Result<T> {
    if req.body.is_empty() {
        bail!("empty request body");
    }
    serde_json::from_slice(&req.body).context("invalid JSON")
}

fn same_origin(req: &HttpRequest) -> bool {
    if req.method != "POST" {
        return true;
    }
    let Some(origin) = req.header("origin") else {
        return true;
    };
    let Some(host) = req.header("host") else {
        return false;
    };
    origin.eq_ignore_ascii_case(&format!("http://{host}"))
        || origin.eq_ignore_ascii_case(&format!("https://{host}"))
}

fn cookie_token(req: &HttpRequest) -> Option<String> {
    let cookie = req.header("cookie")?;
    for part in cookie.split(';') {
        let Some((name, value)) = part.split_once('=') else {
            continue;
        };
        if name.trim() == COOKIE_NAME {
            return Some(value.trim().to_string());
        }
    }
    None
}

fn header_token(req: &HttpRequest) -> Option<String> {
    if let Some(token) = req.header("x-auth-token") {
        return Some(token.trim().to_string());
    }
    let value = req.header("authorization")?;
    value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))
        .map(|s| s.trim().to_string())
}

fn authorized(req: &HttpRequest, token: &str) -> bool {
    if cookie_token(req).as_deref() == Some(token) {
        return true;
    }
    if header_token(req).as_deref() == Some(token) {
        return true;
    }
    req.query.get("token").map(|s| s.as_str()) == Some(token)
}

fn cookie_header(token: &str) -> String {
    format!("{COOKIE_NAME}={token}; HttpOnly; SameSite=Strict; Path=/; Max-Age=86400")
}

fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn build_model_config(
    model: &str,
    base_url: Option<String>,
    api_key: Option<String>,
    protocol: Option<String>,
    allow_custom: bool,
) -> Result<ModelConfig> {
    let base_url = base_url.filter(|s| !s.trim().is_empty());
    let api_key = api_key.filter(|s| !s.trim().is_empty());
    let protocol = protocol.filter(|s| !s.trim().is_empty());
    let custom = base_url.is_some() || api_key.is_some() || protocol.is_some();
    if custom && !allow_custom {
        bail!(
            "custom endpoints are disabled. Start the dashboard with --allow-custom-endpoints \
             or use a server-side model profile."
        );
    }
    if let Some(ref key) = api_key {
        if key.starts_with("env:") {
            bail!("web requests may not reference server environment variables as API keys");
        }
    }

    let mut cfg = if model.starts_with("mock") {
        if custom {
            bail!("mock models cannot be combined with a custom endpoint");
        }
        let mut cfg = ModelConfig::new(model, "mock", model);
        cfg.protocol = ApiProtocol::Mock;
        cfg
    } else if let Some(profile) = resolve_profile(model) {
        profile
    } else {
        // Explicit ad-hoc endpoint: only when the operator enabled it and
        // both URL and key are supplied by the request. Never reuse a
        // server-side key for an arbitrary URL.
        if !allow_custom || base_url.is_none() || api_key.is_none() {
            bail!(
                "model '{model}' is not defined in a server-side config file; \
                 enable --allow-custom-endpoints and provide both base_url and api_key"
            );
        }
        let protocol_name = protocol.as_deref().unwrap_or("openai_chat");
        let mut cfg = ModelConfig::new(model, protocol_name, model);
        cfg.protocol = ApiProtocol::from_str(protocol_name)
            .with_context(|| format!("invalid protocol '{protocol_name}'"))?;
        cfg
    };

    if base_url.is_some()
        && api_key.is_none()
        && cfg
            .api_key
            .as_deref()
            .map(|k| !k.is_empty() && k != "none")
            .unwrap_or(false)
    {
        bail!(
            "a custom base_url requires an explicit api_key; refusing to forward a configured \
             server-side secret to an arbitrary URL"
        );
    }
    if let Some(url) = base_url {
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            bail!("base_url must use http:// or https://");
        }
        cfg.base_url = Some(url);
    }
    if let Some(key) = api_key {
        cfg.api_key = Some(key);
    }
    if let Some(protocol) = protocol {
        cfg.protocol = ApiProtocol::from_str(&protocol)
            .with_context(|| format!("invalid protocol '{protocol}'"))?;
    }
    if let Some(ref key) = cfg.api_key {
        if key.starts_with("env:") {
            bail!("configured API key environment variable is missing: {key}");
        }
    }
    Ok(cfg)
}

fn resolve_profile(model: &str) -> Option<ModelConfig> {
    for path in CONFIG_CANDIDATES {
        let path_ref = Path::new(path);
        if !path_ref.is_file() {
            continue;
        }
        if let Ok(cfg) = crate::config::ConfigFile::load_from_file(path_ref) {
            if let Some(profile) = cfg
                .models
                .into_iter()
                .find(|m| m.id == model || m.model_name == model)
            {
                return Some(profile.to_model_config());
            }
        }
    }
    None
}

fn resolve_judge_config() -> Option<ModelConfig> {
    for path in CONFIG_CANDIDATES {
        let path_ref = Path::new(path);
        if !path_ref.is_file() {
            continue;
        }
        if let Ok(cfg) = crate::config::ConfigFile::load_from_file(path_ref) {
            if let Some(profile) = cfg.judge {
                let model = profile.to_model_config();
                if model
                    .api_key
                    .as_deref()
                    .map(|k| k.starts_with("env:"))
                    .unwrap_or(false)
                {
                    warn!("judge API key environment variable is missing; judge disabled");
                    continue;
                }
                return Some(model);
            }
        }
    }
    None
}

fn control_is_current(state: &ServerState, control: &Arc<RunControl>) -> bool {
    state
        .control
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|current| Arc::ptr_eq(current, control)))
        .unwrap_or(false)
}

async fn fail_live_for(state: &ServerState, control: &Arc<RunControl>, message: String) {
    if !control_is_current(state, control) {
        return;
    }
    let mut live = state.live_state.write().await;
    live.status = "error".to_string();
    live.message = Some(message);
}

async fn clear_control(state: &ServerState, control: &Arc<RunControl>) {
    if let Ok(mut guard) = state.control.lock() {
        if guard
            .as_ref()
            .map(|current| Arc::ptr_eq(current, control))
            .unwrap_or(false)
        {
            *guard = None;
        }
    }
}

async fn run_live_benchmark_task(
    state: ServerState,
    req: BenchmarkRunRequest,
    control: Arc<RunControl>,
) {
    let model_choice = match req.model.clone() {
        Some(m) if !m.trim().is_empty() => m,
        _ => {
            fail_live_for(&state, &control, "model is required".to_string()).await;
            clear_control(&state, &control).await;
            return;
        }
    };

    {
        let mut live = state.live_state.write().await;
        live.status = "running".to_string();
        live.model_name = model_choice.clone();
        live.completed_cases = 0;
        live.passed_cases = 0;
        live.live_cases.clear();
        live.started_at = Some(Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
        live.elapsed_secs = 0;
        live.result_file = None;
        live.message = Some("正在加载测试集与初始化隔离沙箱...".to_string());
    }

    if control.is_cancelled() {
        if control_is_current(&state, &control) {
            let mut live = state.live_state.write().await;
            live.status = "idle".to_string();
            live.message = Some("已取消".to_string());
        }
        clear_control(&state, &control).await;
        return;
    }

    let model_config = match build_model_config(
        &model_choice,
        req.base_url.clone(),
        req.api_key.clone(),
        req.protocol.clone(),
        state.allow_custom_endpoints,
    ) {
        Ok(cfg) => cfg,
        Err(e) => {
            fail_live_for(&state, &control, format!("创建模型配置失败: {e}")).await;
            clear_control(&state, &control).await;
            return;
        }
    };

    let limit = req.limit.unwrap_or(10).clamp(1, 10_000);
    let dataset = match load_live_dataset(req.category.as_deref(), req.difficulty.as_deref(), limit)
    {
        Ok(ds) => ds,
        Err(e) => {
            fail_live_for(&state, &control, format!("加载测试集失败: {e}")).await;
            clear_control(&state, &control).await;
            return;
        }
    };
    let total_cases = dataset.test_cases.len();
    if total_cases == 0 {
        fail_live_for(&state, &control, "过滤后测试集为空".to_string()).await;
        clear_control(&state, &control).await;
        return;
    }
    {
        let mut live = state.live_state.write().await;
        live.total_cases = total_cases;
        live.message = Some(format!(
            "开始评测，共 {} 题 (并发: {})",
            total_cases,
            req.concurrency.unwrap_or(4).clamp(1, 32)
        ));
    }

    let client = match create_client(model_config) {
        Ok(c) => c,
        Err(e) => {
            fail_live_for(&state, &control, format!("创建模型客户端失败: {e}")).await;
            clear_control(&state, &control).await;
            return;
        }
    };

    let mut orchestrator = BenchmarkOrchestrator::new(req.concurrency.unwrap_or(4).clamp(1, 32))
        .with_control(Arc::clone(&control));
    if let Some(judge_cfg) = resolve_judge_config() {
        if let Ok(judge) = create_client(judge_cfg) {
            orchestrator = orchestrator.with_judge(judge);
        }
    }

    let started = Instant::now();
    let summary_res = orchestrator.run_benchmark(client, &dataset).await;

    if control.is_cancelled() {
        if control_is_current(&state, &control) {
            let mut live = state.live_state.write().await;
            live.status = "idle".to_string();
            live.message = Some("已手动终止评测（未写入结果文件）".to_string());
            live.elapsed_secs = started.elapsed().as_secs();
        }
        clear_control(&state, &control).await;
        return;
    }

    match summary_res {
        Ok(summary) => {
            if !control_is_current(&state, &control) {
                return;
            }
            let filename = format!("eval_results_{}.json", Local::now().format("%Y%m%d_%H%M%S"));
            let out_path = state.results_dir.join(&filename);
            let summaries = vec![summary.clone()];
            match serde_json::to_string_pretty(&summaries)
                .map_err(anyhow::Error::from)
                .and_then(|json| std::fs::write(&out_path, json).map_err(anyhow::Error::from))
            {
                Ok(()) => {}
                Err(e) => {
                    fail_live_for(&state, &control, format!("结果写入失败: {e}")).await;
                    clear_control(&state, &control).await;
                    return;
                }
            }

            let mut live = state.live_state.write().await;
            live.status = "completed".to_string();
            live.completed_cases = summary.total_cases;
            live.passed_cases = summary.passed_cases;
            live.elapsed_secs = started.elapsed().as_secs();
            live.result_file = Some(filename.clone());
            live.message = Some(format!(
                "评测完成! 准确率: {:.1}%, 结果已保存至 {}",
                summary.overall_accuracy * 100.0,
                filename
            ));
            live.live_cases = summary
                .case_results
                .iter()
                .take(30)
                .map(|c| LiveCaseItem {
                    id: c.test_case_id.clone(),
                    category: c.category.as_str().to_string(),
                    passed: c.passed,
                    score: c.score,
                    latency_ms: c.latency_ms,
                })
                .collect();
        }
        Err(e) => {
            fail_live_for(&state, &control, format!("评测执行出错: {e}")).await;
        }
    }
    clear_control(&state, &control).await;
}

fn load_crosscheck_cases() -> Vec<TestCase> {
    let root = resolve_datasets_dir();
    let mut cases = Vec::new();
    for rel in ["foundation/math_gsm8k.jsonl", "domain/mmlu_domain.jsonl"] {
        if let Ok(ds) = DatasetLoader::load_from_jsonl(root.join(rel)) {
            cases.extend(ds.test_cases.into_iter().take(8));
        }
    }
    cases
}

fn load_live_dataset(
    category: Option<&str>,
    difficulty: Option<&str>,
    limit: usize,
) -> Result<Dataset> {
    let mut files = Vec::new();
    collect_jsonl_files(&resolve_datasets_dir(), &mut files)?;
    let mut combined = Dataset::new("live_benchmark");
    for file in files {
        if let Ok(ds) = DatasetLoader::load_from_jsonl(&file) {
            combined.test_cases.extend(ds.test_cases);
        }
    }
    if let Some(cat_str) = category {
        if !cat_str.is_empty() && cat_str != "all" {
            combined = combined.filter_by_group(cat_str);
        }
    }
    if let Some(diff) = difficulty {
        if !diff.is_empty() && diff != "all" {
            combined = combined.filter_by_difficulty(diff);
        }
    }
    if limit < combined.test_cases.len() {
        combined.test_cases.truncate(limit);
    }
    Ok(combined)
}

fn collect_jsonl_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_file() {
        if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            files.push(path.to_path_buf());
        }
    } else if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let child_path = entry.path();
            if child_path.is_dir() {
                collect_jsonl_files(&child_path, files)?;
            } else if child_path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                files.push(child_path);
            }
        }
    }
    Ok(())
}

fn resolve_datasets_dir() -> PathBuf {
    for candidate in [
        PathBuf::from("datasets"),
        PathBuf::from("../datasets"),
        PathBuf::from("../../datasets"),
    ] {
        if candidate.is_dir() {
            return candidate.canonicalize().unwrap_or(candidate);
        }
    }
    PathBuf::from("datasets")
}

fn resolve_dist_dir(custom: Option<String>) -> Result<PathBuf> {
    if let Some(path) = custom {
        let path = PathBuf::from(path);
        if path.join("index.html").is_file() {
            return path
                .canonicalize()
                .context("failed to canonicalize web dist");
        }
        bail!("web dist directory has no index.html: {}", path.display());
    }
    for candidate in [
        PathBuf::from("web/dist"),
        PathBuf::from("../web/dist"),
        PathBuf::from("../../web/dist"),
    ] {
        if candidate.join("index.html").is_file() {
            return candidate
                .canonicalize()
                .context("failed to canonicalize web dist");
        }
    }
    bail!("无法定位前端 dist 目录！请先执行 `npm run build` 或传 --web-dist")
}

fn resolve_results_dir(custom: Option<&str>) -> PathBuf {
    if let Some(p) = custom {
        let path = PathBuf::from(p);
        if path.is_dir() {
            return path.canonicalize().unwrap_or(path);
        }
        if path.is_file() {
            if let Some(parent) = path.parent() {
                return parent
                    .canonicalize()
                    .unwrap_or_else(|_| parent.to_path_buf());
            }
        }
        let _ = std::fs::create_dir_all(&path);
        return path.canonicalize().unwrap_or(path);
    }
    for candidate in [
        PathBuf::from("results"),
        PathBuf::from("../results"),
        PathBuf::from("../../results"),
    ] {
        if candidate.is_dir() {
            return candidate.canonicalize().unwrap_or(candidate);
        }
    }
    let fallback = PathBuf::from("results");
    let _ = std::fs::create_dir_all(&fallback);
    fallback.canonicalize().unwrap_or(fallback)
}

fn latest_result_file(results_dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(results_dir).ok()?;
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|p| {
            p.extension().and_then(|s| s.to_str()) == Some("json")
                && p.file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s.starts_with("eval_results_"))
                    .unwrap_or(false)
        })
        .filter(|p| p.metadata().map(|m| m.is_file()).unwrap_or(false))
        .max_by_key(|p| {
            std::fs::metadata(p)
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH)
        })
}

async fn serve_default_results(state: &ServerState) -> Response {
    let file = state
        .default_result_file
        .clone()
        .or_else(|| latest_result_file(&state.results_dir));
    let Some(file) = file else {
        // Static preview fallback shipped with the frontend.
        let fallback = state.dist_dir.join("data/default_results.json");
        return match tokio::fs::read(&fallback).await {
            Ok(bytes) => Response::bytes(200, "application/json; charset=utf-8", bytes),
            Err(_) => Response::error(404, "no result files available"),
        };
    };
    if let Ok(canonical) = file.canonicalize() {
        if canonical.starts_with(&state.results_dir) || state.default_result_file.is_some() {
            return match tokio::fs::read(&canonical).await {
                Ok(bytes) => Response::bytes(200, "application/json; charset=utf-8", bytes),
                Err(e) => Response::error(500, format!("failed to read results: {e}")),
            };
        }
    }
    Response::error(403, "result file outside results directory")
}

async fn scan_runs_meta_cached(state: &ServerState) -> Vec<RunMeta> {
    let results_dir = state.results_dir.clone();
    let cache = Arc::clone(&state.runs_cache);
    tokio::task::spawn_blocking(move || scan_runs_meta_blocking(&results_dir, &cache))
        .await
        .unwrap_or_default()
}

fn scan_runs_meta_blocking(results_dir: &Path, cache: &RunsCache) -> Vec<RunMeta> {
    let mut metas = Vec::new();
    let Ok(entries) = std::fs::read_dir(results_dir) else {
        return metas;
    };

    for entry in entries.flatten().take(500) {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        if !filename.starts_with("eval_results_") {
            continue;
        }
        let Ok(meta) = std::fs::metadata(&path) else {
            continue;
        };
        if meta.len() > 64 * 1024 * 1024 {
            continue;
        }
        let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        if let Ok(guard) = cache.lock() {
            if let Some((size, cached_mtime, cached_meta)) = guard.get(&filename) {
                if *size == meta.len() && *cached_mtime == mtime {
                    metas.push(cached_meta.clone());
                    continue;
                }
            }
        }

        let (models, total_cases) = quick_inspect_json(&path);
        let run_meta = RunMeta {
            timestamp: format_timestamp_from_filename(&filename),
            filename: filename.clone(),
            size_bytes: meta.len(),
            size_human: format_size(meta.len()),
            models,
            total_cases,
        };
        if let Ok(mut guard) = cache.lock() {
            guard.insert(filename, (meta.len(), mtime, run_meta.clone()));
        }
        metas.push(run_meta);
    }

    metas.sort_by(|a, b| b.filename.cmp(&a.filename));
    metas
}

fn quick_inspect_json(path: &Path) -> (Vec<String>, usize) {
    let Ok(bytes) = std::fs::read(path) else {
        return (Vec::new(), 0);
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return (Vec::new(), 0);
    };
    let arr = value
        .as_array()
        .cloned()
        .or_else(|| value.get("summaries").and_then(|s| s.as_array()).cloned())
        .unwrap_or_default();
    let mut models = Vec::new();
    let mut max_cases = 0;
    for item in arr {
        if let Some(m) = item
            .get("model_name")
            .and_then(|v| v.as_str())
            .or_else(|| item.get("model_id").and_then(|v| v.as_str()))
        {
            models.push(m.to_string());
        }
        if let Some(cnt) = item.get("total_cases").and_then(|v| v.as_u64()) {
            max_cases = max_cases.max(cnt as usize);
        }
    }
    (models, max_cases)
}

fn format_timestamp_from_filename(filename: &str) -> String {
    let parts: Vec<&str> = filename.trim_end_matches(".json").split('_').collect();
    if parts.len() >= 4 {
        let date = parts[2];
        let time = parts[3];
        if date.len() == 8
            && time.len() == 6
            && date.bytes().all(|b| b.is_ascii_digit())
            && time.bytes().all(|b| b.is_ascii_digit())
        {
            return format!(
                "{}-{}-{} {}:{}:{}",
                &date[0..4],
                &date[4..6],
                &date[6..8],
                &time[0..2],
                &time[2..4],
                &time[4..6]
            );
        }
    }
    filename.to_string()
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn get_content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        _ => "application/octet-stream",
    }
}
