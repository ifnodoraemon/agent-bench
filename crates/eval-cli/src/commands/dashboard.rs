use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::{error, info, warn};

pub async fn execute(
    port: u16,
    results_path: Option<String>,
    web_dist: Option<String>,
) -> Result<()> {
    // 1. Resolve dist directory
    let dist_dir = resolve_dist_dir(web_dist)?;
    info!("Using dashboard dist directory: {}", dist_dir.display());

    // 2. If results_path is specified, copy or update data/default_results.json
    if let Some(ref rpath) = results_path {
        let p = Path::new(rpath);
        let target_file = dist_dir.join("data").join("default_results.json");
        std::fs::create_dir_all(dist_dir.join("data"))?;

        if p.is_dir() {
            // Find eval_results JSON files
            let mut entries: Vec<PathBuf> = std::fs::read_dir(p)?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    p.extension().map(|e| e == "json").unwrap_or(false)
                        && p.file_name().and_then(|n| n.to_str()).map(|n| n.starts_with("eval_results_")).unwrap_or(false)
                })
                .collect();

            // Prefer largest file (most comprehensive benchmark run), fallback to newest
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
    if let Some(ref rp) = results_path {
        println!("║  📊 载入数据来源:   {:<46} ║", truncate_str(rp, 46));
    } else {
        println!("║  📊 载入数据来源:   默认内置基准测试样例 (1009 Cases)              ║");
    }
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
