use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::{error, info, warn};

#[derive(Debug, Serialize, Deserialize)]
pub struct RunMeta {
    pub filename: String,
    pub timestamp: String,
    pub size_bytes: u64,
    pub size_human: String,
    pub models: Vec<String>,
    pub total_cases: usize,
}

pub async fn execute(
    port: u16,
    results_path: Option<String>,
    web_dist: Option<String>,
) -> Result<()> {
    // 1. Resolve dist directory
    let dist_dir = resolve_dist_dir(web_dist)?;
    info!("Using dashboard dist directory: {}", dist_dir.display());

    // 2. Resolve results directory
    let results_dir = resolve_results_dir(results_path.as_deref());
    info!("Using results directory: {}", results_dir.display());

    // 3. If results_path is specified, copy or update data/default_results.json
    if let Some(ref rpath) = results_path {
        let p = Path::new(rpath);
        let target_file = dist_dir.join("data").join("default_results.json");
        std::fs::create_dir_all(dist_dir.join("data"))?;

        if p.is_dir() {
            let mut entries: Vec<PathBuf> = std::fs::read_dir(p)?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    p.extension().map(|e| e == "json").unwrap_or(false)
                        && p.file_name().and_then(|n| n.to_str()).map(|n| n.starts_with("eval_results_")).unwrap_or(false)
                })
                .collect();

            // Prefer largest file (most comprehensive benchmark run)
            entries.sort_by_key(|p| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0));

            if let Some(chosen) = entries.last() {
                info!("Copying results file {} to {}", chosen.display(), target_file.display());
                std::fs::copy(chosen, &target_file)
                    .with_context(|| format!("Failed to copy {} to default_results.json", chosen.display()))?;
            } else {
                warn!("No JSON result files found in directory {}", p.display());
            }
        } else if p.is_file() {
            info!("Copying results file {} to {}", p.display(), target_file.display());
            std::fs::copy(p, &target_file)
                .with_context(|| format!("Failed to copy {} to default_results.json", p.display()))?;
        }
    }

    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await
        .with_context(|| format!("Failed to bind dashboard server to {}", addr))?;

    println!("\n╔════════════════════════════════════════════════════════════════════╗");
    println!("║      🦀  agent-bench 可视化大屏服务已成功启动!                     ║");
    println!("╠════════════════════════════════════════════════════════════════════╣");
    println!("║  🌐 本地浏览器访问: http://localhost:{:<5}                        ║", port);
    println!("║  📁 静态资源目录:   {:<46} ║", truncate_str(&dist_dir.display().to_string(), 46));
    println!("║  📊 历史评测目录:   {:<46} ║", truncate_str(&results_dir.display().to_string(), 46));
    println!("║  🔌 开放 API 接口:  /api/runs (评测列表) | /api/runs/:file (流式数据) ║");
    println!("║  ⌨️  退出服务:       按 Ctrl + C 即可终止进程                      ║");
    println!("╚════════════════════════════════════════════════════════════════════╝\n");

    loop {
        let (mut socket, _peer) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("Error accepting connection: {}", e);
                continue;
            }
        };

        let base_dir = dist_dir.clone();
        let r_dir = results_dir.clone();

        tokio::spawn(async move {
            let mut buf = [0u8; 4096];
            let n = match socket.read(&mut buf).await {
                Ok(n) if n > 0 => n,
                _ => return,
            };

            let req_str = String::from_utf8_lossy(&buf[..n]);
            let first_line = req_str.lines().next().unwrap_or("");
            let parts: Vec<&str> = first_line.split_whitespace().collect();

            if parts.len() < 2 {
                return;
            }

            let method = parts[0];
            let is_head = method == "HEAD";
            if method != "GET" && !is_head {
                let _ = socket.write_all(b"HTTP/1.1 405 Method Not Allowed\r\nContent-Length: 0\r\n\r\n").await;
                return;
            }

            let mut req_path = parts[1];
            if let Some(pos) = req_path.find('?') {
                req_path = &req_path[..pos];
            }

            // 1. API route: /api/runs
            if req_path == "/api/runs" {
                let runs = scan_runs_meta(&r_dir);
                let json_bytes = serde_json::to_vec(&runs).unwrap_or_else(|_| b"[]".to_vec());
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\n\r\n",
                    json_bytes.len()
                );
                let _ = socket.write_all(header.as_bytes()).await;
                if !is_head {
                    let _ = socket.write_all(&json_bytes).await;
                }
                return;
            }

            // 2. API route: /api/runs/:filename
            if let Some(filename) = req_path.strip_prefix("/api/runs/") {
                let safe_filename = filename.trim_matches('/');
                let target_file = r_dir.join(safe_filename);
                if target_file.is_file() {
                    match tokio::fs::read(&target_file).await {
                        Ok(bytes) => {
                            let header = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\n\r\n",
                                bytes.len()
                            );
                            let _ = socket.write_all(header.as_bytes()).await;
                            if !is_head {
                                let _ = socket.write_all(&bytes).await;
                            }
                        }
                        Err(_) => {
                            let _ = socket.write_all(b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n").await;
                        }
                    }
                } else {
                    let _ = socket.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n").await;
                }
                return;
            }

            // 3. API route: /api/system
            if req_path == "/api/system" {
                let info = serde_json::json!({
                    "engine": "agent-bench (Rust Tokio)",
                    "version": "v2.4.0",
                    "results_dir": r_dir.display().to_string(),
                    "status": "ready"
                });
                let bytes = serde_json::to_vec(&info).unwrap();
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\n\r\n",
                    bytes.len()
                );
                let _ = socket.write_all(header.as_bytes()).await;
                if !is_head {
                    let _ = socket.write_all(&bytes).await;
                }
                return;
            }

            // 4. Static files
            let clean_path = req_path.trim_start_matches('/');
            let target_path = if clean_path.is_empty() {
                base_dir.join("index.html")
            } else {
                base_dir.join(clean_path)
            };

            // SPA fallback: if file doesn't exist, serve index.html
            let (file_path, content_type) = if target_path.is_file() {
                let ct = get_content_type(&target_path);
                (target_path, ct)
            } else {
                (base_dir.join("index.html"), "text/html; charset=utf-8")
            };

            match tokio::fs::read(&file_path).await {
                Ok(bytes) => {
                    let header = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\n\r\n",
                        content_type,
                        bytes.len()
                    );
                    let _ = socket.write_all(header.as_bytes()).await;
                    if !is_head {
                        let _ = socket.write_all(&bytes).await;
                    }
                }
                Err(_) => {
                    let msg = b"404 Not Found";
                    let header = format!(
                        "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        msg.len()
                    );
                    let _ = socket.write_all(header.as_bytes()).await;
                    if !is_head {
                        let _ = socket.write_all(msg).await;
                    }
                }
            }
        });
    }
}

fn resolve_dist_dir(custom_path: Option<String>) -> Result<PathBuf> {
    if let Some(p) = custom_path {
        let path = PathBuf::from(p);
        if path.join("index.html").exists() {
            return Ok(path);
        }
    }

    let candidates = [
        PathBuf::from("web/dist"),
        PathBuf::from("../web/dist"),
        PathBuf::from("../../web/dist"),
    ];

    for c in &candidates {
        if c.join("index.html").exists() {
            return Ok(c.canonicalize().unwrap_or_else(|_| c.clone()));
        }
    }

    anyhow::bail!(
        "无法定位前端 dist 目录！请先在 web/ 目录下执行 `npm run build` 生成构建产物，或通过 `--web-dist` 参数指定目录。"
    )
}

fn resolve_results_dir(custom_path: Option<&str>) -> PathBuf {
    if let Some(p) = custom_path {
        let path = PathBuf::from(p);
        if path.is_dir() {
            return path.canonicalize().unwrap_or(path);
        } else if let Some(parent) = path.parent() {
            if parent.is_dir() {
                return parent.to_path_buf().canonicalize().unwrap_or(parent.to_path_buf());
            }
        }
    }

    let candidates = [
        PathBuf::from("results"),
        PathBuf::from("../results"),
        PathBuf::from("../../results"),
    ];

    for c in &candidates {
        if c.is_dir() {
            return c.canonicalize().unwrap_or_else(|_| c.clone());
        }
    }

    PathBuf::from("results")
}

fn scan_runs_meta(results_dir: &Path) -> Vec<RunMeta> {
    let mut metas = Vec::new();
    let entries = match std::fs::read_dir(results_dir) {
        Ok(e) => e,
        Err(_) => return metas,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            if !filename.starts_with("eval_results_") {
                continue;
            }

            let meta = match std::fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };

            let size_bytes = meta.len();
            let size_human = format_size(size_bytes);

            // Format timestamp from filename (e.g. eval_results_20260821_185648.json)
            let timestamp = format_timestamp_from_filename(&filename);

            // Quick extraction of models & total cases
            let (models, total_cases) = quick_inspect_json(&path);

            metas.push(RunMeta {
                filename,
                timestamp,
                size_bytes,
                size_human,
                models,
                total_cases,
            });
        }
    }

    // Sort by filename descending (newest timestamp first)
    metas.sort_by(|a, b| b.filename.cmp(&a.filename));
    metas
}

fn format_timestamp_from_filename(filename: &str) -> String {
    // eval_results_YYYYMMDD_HHMMSS.json
    let parts: Vec<&str> = filename.trim_end_matches(".json").split('_').collect();
    if parts.len() >= 4 {
        let date = parts[2];
        let time = parts[3];
        if date.len() == 8 && time.len() == 6 {
            return format!(
                "{}-{}-{} {}:{}:{}",
                &date[0..4], &date[4..6], &date[6..8],
                &time[0..2], &time[2..4], &time[4..6]
            );
        }
    }
    filename.to_string()
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn quick_inspect_json(path: &Path) -> (Vec<String>, usize) {
    if let Ok(file) = std::fs::File::open(path) {
        let reader = std::io::BufReader::new(file);
        if let Ok(val) = serde_json::from_reader::<_, serde_json::Value>(reader) {
            if let Some(arr) = val.as_array() {
                let mut models = Vec::new();
                let mut max_cases = 0;
                for item in arr {
                    if let Some(m) = item.get("model_name").and_then(|v| v.as_str()) {
                        models.push(m.to_string());
                    } else if let Some(m) = item.get("model_id").and_then(|v| v.as_str()) {
                        models.push(m.to_string());
                    }
                    if let Some(cnt) = item.get("total_cases").and_then(|v| v.as_u64()) {
                        if cnt as usize > max_cases {
                            max_cases = cnt as usize;
                        }
                    }
                }
                return (models, max_cases);
            }
        }
    }
    (Vec::new(), 0)
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

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let prefix: String = s.chars().take(max_len - 3).collect();
        format!("{}...", prefix)
    }
}
