use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::process;
use std::thread;
use std::time::Duration;

fn parse_port() -> Result<u16, String> {
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--port" || arg == "-p" {
            let val = args
                .next()
                .ok_or_else(|| format!("missing value after {arg}"))?;
            return val
                .parse::<u16>()
                .map_err(|_| format!("invalid port number: {val}"));
        }
        if let Some(val) = arg.strip_prefix("--port=") {
            return val
                .parse::<u16>()
                .map_err(|_| format!("invalid port number: {val}"));
        }
        if arg == "--help" || arg == "-h" {
            println!("Usage: trek [--port <PORT>]");
            process::exit(0);
        }
        return Err(format!("unknown argument: {arg}"));
    }

    if let Ok(val) = env::var("PORT") {
        if !val.trim().is_empty() {
            return val
                .trim()
                .parse::<u16>()
                .map_err(|_| format!("invalid PORT environment variable: {val}"));
        }
    }

    Ok(8080)
}

fn percent_decode(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return None;
            }
            let hi = (bytes[i + 1] as char).to_digit(16)?;
            let lo = (bytes[i + 2] as char).to_digit(16)?;
            let val = ((hi << 4) | lo) as u8;
            if val == 0 {
                return None;
            }
            out.push(val);
            i += 3;
        } else {
            if bytes[i] == 0 {
                return None;
            }
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn mime_type_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .as_deref()
    {
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") | Some("mjs") => "text/javascript; charset=utf-8",
        Some("wasm") => "application/wasm",
        Some("doc") | Some("txt") => "text/plain; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn resolve_web_path(web_root: &Path, raw_target: &str) -> Result<PathBuf, u16> {
    let path_only = raw_target
        .split(['?', '#'])
        .next()
        .unwrap_or("/");
    let decoded = percent_decode(path_only).ok_or(400u16)?;

    if decoded.contains('\\') {
        return Err(403);
    }

    let trimmed = decoded.trim_start_matches('/');
    let rel = if trimmed.is_empty() {
        PathBuf::from("index.html")
    } else {
        let candidate = Path::new(trimmed);
        for comp in candidate.components() {
            match comp {
                Component::Normal(_) => {}
                Component::CurDir | Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err(403);
                }
            }
        }
        candidate.to_path_buf()
    };

    let joined = web_root.join(&rel);
    let canonical_root = fs::canonicalize(web_root).map_err(|_| 404u16)?;
    let canonical_file = fs::canonicalize(&joined).map_err(|_| 404u16)?;
    if !canonical_file.starts_with(&canonical_root) {
        return Err(403);
    }
    if canonical_file.is_dir() {
        let index_path = canonical_file.join("index.html");
        if index_path.is_file() {
            return Ok(index_path);
        }
        return Err(404);
    }
    Ok(canonical_file)
}

fn write_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
    head_only: bool,
) {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-cache\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    if !head_only && !body.is_empty() {
        let _ = stream.write_all(body);
    }
    let _ = stream.flush();
}

fn write_error(stream: &mut TcpStream, code: u16, head_only: bool) {
    let (status, msg) = match code {
        400 => ("400 Bad Request", "400 Bad Request\n"),
        403 => ("403 Forbidden", "403 Forbidden\n"),
        404 => ("404 Not Found", "404 Not Found\n"),
        405 => ("405 Method Not Allowed", "405 Method Not Allowed\n"),
        _ => ("500 Internal Server Error", "500 Internal Server Error\n"),
    };
    write_response(stream, status, "text/plain; charset=utf-8", msg.as_bytes(), head_only);
}

fn handle_client(mut stream: TcpStream, web_root: PathBuf) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));

    let mut reader = match stream.try_clone() {
        Ok(cloned) => BufReader::new(cloned),
        Err(_) => return,
    };

    let mut request_line = String::new();
    if reader.read_line(&mut request_line).unwrap_or(0) == 0 {
        return;
    }

    let mut parts = request_line.split_whitespace();
    let method = match parts.next() {
        Some(m) => m,
        None => {
            write_error(&mut stream, 400, false);
            return;
        }
    };
    let target = match parts.next() {
        Some(t) => t,
        None => {
            write_error(&mut stream, 400, false);
            return;
        }
    };

    // Drain request headers until blank line.
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                if line == "\r\n" || line == "\n" {
                    break;
                }
            }
        }
    }

    let head_only = match method {
        "GET" => false,
        "HEAD" => true,
        _ => {
            write_error(&mut stream, 405, false);
            return;
        }
    };

    let file_path = match resolve_web_path(&web_root, target) {
        Ok(p) => p,
        Err(code) => {
            write_error(&mut stream, code, head_only);
            return;
        }
    };

    match fs::read(&file_path) {
        Ok(bytes) => {
            let mime = mime_type_for(&file_path);
            write_response(&mut stream, "200 OK", mime, &bytes, head_only);
        }
        Err(_) => {
            write_error(&mut stream, 404, head_only);
        }
    }
}

fn main() {
    let port = match parse_port() {
        Ok(p) => p,
        Err(err) => {
            eprintln!("Error: {err}");
            process::exit(2);
        }
    };

    let web_root = PathBuf::from("./web");
    if !web_root.is_dir() {
        eprintln!("Warning: ./web directory does not exist in current working directory");
    }

    let addr = format!("127.0.0.1:{port}");
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(err) => {
            eprintln!("Failed to bind {addr}: {err}");
            process::exit(1);
        }
    };

    println!("Trek web game listening on http://{addr}");

    for incoming in listener.incoming() {
        match incoming {
            Ok(stream) => {
                let root = web_root.clone();
                thread::spawn(move || handle_client(stream, root));
            }
            Err(err) => {
                eprintln!("Accept error: {err}");
            }
        }
    }
}
