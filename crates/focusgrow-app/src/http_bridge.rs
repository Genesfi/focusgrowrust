use focusgrow_core::domain_matches;
use serde::Deserialize;
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APP};

/// Posted to `main_hwnd` when the HTTP `:8766` bridge receives a pass grant or tab event.
pub const WM_BRIDGE_EVENT: u32 = WM_APP + 13;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BridgeCommand {
    GrantPass { domain: String, minutes: u32 },
    ActiveTab { url: String, domain: String, title: String },
}

#[derive(Clone)]
pub struct BridgeShared {
    pub restricted_sites_json: String,
    pub state_json: String,
    pub is_focusing_unpaused: bool,
    pub restricted_domains: Vec<String>,
    pub active_pass_domains: Vec<(String, u32)>,
    pub incoming: VecDeque<BridgeCommand>,
}

impl Default for BridgeShared {
    fn default() -> Self {
        Self {
            restricted_sites_json: "[]".to_string(),
            state_json: "{}".to_string(),
            is_focusing_unpaused: false,
            restricted_domains: Vec::new(),
            active_pass_domains: Vec::new(),
            incoming: VecDeque::new(),
        }
    }
}

/// Lightweight local HTTP bridge (`0.0.0.0:8766`) for the YTMPX browser blocker extension
/// and Android Companion app (`/restricted-sites`, `/state`, `/grant`).
pub struct HttpBridgeServer {
    stop_flag: Arc<AtomicBool>,
    shared: Arc<Mutex<BridgeShared>>,
    _thread: Option<JoinHandle<()>>,
    pub bound_port: u16,
}

impl HttpBridgeServer {
    pub fn start(notify_hwnd: HWND) -> Self {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let shared = Arc::new(Mutex::new(BridgeShared::default()));

        let listener = TcpListener::bind("0.0.0.0:8766")
            .or_else(|_| TcpListener::bind("127.0.0.1:8766"))
            .ok();
        let bound_port = if listener.is_some() { 8766 } else { 0 };

        let hwnd_raw = notify_hwnd.0 as isize;
        let stop_clone = Arc::clone(&stop_flag);
        let shared_clone = Arc::clone(&shared);

        let handle = listener.map(|sock| {
            let _ = sock.set_nonblocking(true);
            thread::spawn(move || {
                run_server_loop(sock, hwnd_raw, stop_clone, shared_clone);
            })
        });

        Self {
            stop_flag,
            shared,
            _thread: handle,
            bound_port,
        }
    }

    pub fn update_snapshot(
        &self,
        restricted_sites_json: String,
        state_json: String,
        is_focusing_unpaused: bool,
        restricted_domains: Vec<String>,
        active_pass_domains: Vec<(String, u32)>,
    ) {
        if let Ok(mut guard) = self.shared.lock() {
            guard.restricted_sites_json = restricted_sites_json;
            guard.state_json = state_json;
            guard.is_focusing_unpaused = is_focusing_unpaused;
            guard.restricted_domains = restricted_domains;
            guard.active_pass_domains = active_pass_domains;
        }
    }

    pub fn drain_commands(&self) -> Vec<BridgeCommand> {
        let mut guard = self.shared.lock().unwrap_or_else(|e| e.into_inner());
        guard.incoming.drain(..).collect()
    }
}

impl Drop for HttpBridgeServer {
    fn drop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }
}

fn run_server_loop(
    listener: TcpListener,
    hwnd_raw: isize,
    stop: Arc<AtomicBool>,
    shared: Arc<Mutex<BridgeShared>>,
) {
    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((mut stream, _)) => {
                let _ = stream.set_read_timeout(Some(Duration::from_millis(250)));
                handle_client(&mut stream, hwnd_raw, &shared);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(60));
            }
            Err(_) => {
                thread::sleep(Duration::from_millis(200));
            }
        }
    }
}

fn handle_client(stream: &mut TcpStream, hwnd_raw: isize, shared: &Arc<Mutex<BridgeShared>>) {
    let mut buf = [0u8; 8192];
    let n = match stream.read(&mut buf) {
        Ok(n) if n > 0 => n,
        _ => return,
    };
    let req = String::from_utf8_lossy(&buf[..n]);
    let first_line = req.lines().next().unwrap_or("");

    if first_line.starts_with("OPTIONS ") {
        let resp = "HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nContent-Length: 0\r\n\r\n";
        let _ = stream.write_all(resp.as_bytes());
        return;
    }

    if first_line.starts_with("GET /restricted-sites") {
        let body = shared
            .lock()
            .map(|s| s.restricted_sites_json.clone())
            .unwrap_or_else(|_| "[]".to_string());
        write_json_response(stream, 200, "OK", &body);
        return;
    }

    if first_line.starts_with("GET /state") {
        let body = shared
            .lock()
            .map(|s| s.state_json.clone())
            .unwrap_or_else(|_| "{}".to_string());
        write_json_response(stream, 200, "OK", &body);
        return;
    }

    let body = extract_http_body(&req);

    if first_line.starts_with("POST /grant") {
        if let Some((domain, minutes)) = parse_grant_body(body) {
            queue_command(
                hwnd_raw,
                shared,
                BridgeCommand::GrantPass { domain, minutes },
            );
        }
        write_empty_200(stream);
        return;
    }

    // Browser extension tab check / pass grant
    if let Some(check) = parse_tab_check_body(body) {
        if check.grant_pass == Some(true) && !check.domain.is_empty() {
            queue_command(
                hwnd_raw,
                shared,
                BridgeCommand::GrantPass {
                    domain: check.domain.clone(),
                    minutes: check.minutes.unwrap_or(5),
                },
            );
        }
        if !check.domain.is_empty() {
            queue_command(
                hwnd_raw,
                shared,
                BridgeCommand::ActiveTab {
                    url: check.url.unwrap_or_default(),
                    domain: check.domain.clone(),
                    title: check.title.unwrap_or_default(),
                },
            );
        }

        let (status, remaining_sec) = shared
            .lock()
            .map(|s| evaluate_domain_status(&s, &check.domain))
            .unwrap_or(("ok", 0));
        let resp_json = serde_json::json!({
            "status": status,
            "domain": check.domain,
            "remainingSec": remaining_sec,
        })
        .to_string();
        write_json_response(stream, 200, "OK", &resp_json);
        return;
    }

    write_json_response(stream, 200, "OK", r#"{"status":"ok"}"#);
}

fn queue_command(hwnd_raw: isize, shared: &Arc<Mutex<BridgeShared>>, cmd: BridgeCommand) {
    if let Ok(mut guard) = shared.lock() {
        guard.incoming.push_back(cmd);
    }
    unsafe {
        let _ = PostMessageW(Some(HWND(hwnd_raw as *mut _)), WM_BRIDGE_EVENT, WPARAM(0), LPARAM(0));
    }
}

fn evaluate_domain_status(shared: &BridgeShared, domain: &str) -> (&'static str, u32) {
    if !shared.is_focusing_unpaused || domain.is_empty() {
        return ("ok", 0);
    }
    let low = domain.trim().to_ascii_lowercase();
    for (pass_dom, rem) in &shared.active_pass_domains {
        if *rem > 0 && domain_matches(&low, pass_dom) {
            return ("ok", *rem);
        }
    }
    for restricted in &shared.restricted_domains {
        if domain_matches(&low, restricted) {
            return ("blocked", 0);
        }
    }
    ("ok", 0)
}

fn write_json_response(stream: &mut TcpStream, code: u16, reason: &str, body: &str) {
    let header = format!(
        "HTTP/1.1 {code} {reason}\r\nAccess-Control-Allow-Origin: *\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body.as_bytes());
}

fn write_empty_200(stream: &mut TcpStream) {
    let resp = "HTTP/1.1 200 OK\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: 0\r\n\r\n";
    let _ = stream.write_all(resp.as_bytes());
}

fn extract_http_body(req: &str) -> &str {
    req.split_once("\r\n\r\n")
        .or_else(|| req.split_once("\n\n"))
        .map(|(_, body)| body.trim())
        .unwrap_or("")
}

#[derive(Deserialize)]
struct GrantPayload {
    action: Option<String>,
    domain: Option<String>,
    minutes: Option<u32>,
}

fn parse_grant_body(body: &str) -> Option<(String, u32)> {
    let p: GrantPayload = serde_json::from_str(body).ok()?;
    if p.action.as_deref() != Some("grantPass") {
        return None;
    }
    let domain = p.domain?.trim().to_string();
    if domain.is_empty() {
        return None;
    }
    Some((domain, p.minutes.unwrap_or(5).clamp(1, 30)))
}

#[derive(Deserialize)]
struct TabCheckPayload {
    url: Option<String>,
    domain: Option<String>,
    title: Option<String>,
    #[serde(rename = "grantPass")]
    grant_pass: Option<bool>,
    minutes: Option<u32>,
}

struct ParsedTabCheck {
    url: Option<String>,
    domain: String,
    title: Option<String>,
    grant_pass: Option<bool>,
    minutes: Option<u32>,
}

fn parse_tab_check_body(body: &str) -> Option<ParsedTabCheck> {
    let p: TabCheckPayload = serde_json::from_str(body).ok()?;
    let domain = p.domain.unwrap_or_default().trim().to_string();
    if domain.is_empty() && p.url.is_none() {
        return None;
    }
    Some(ParsedTabCheck {
        url: p.url,
        domain,
        title: p.title,
        grant_pass: p.grant_pass,
        minutes: p.minutes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_grant_pass_json_and_evaluates_blocked_domain() {
        let req = "POST /grant HTTP/1.1\r\nHost: localhost\r\n\r\n{\"action\":\"grantPass\",\"domain\":\"youtube.com\",\"minutes\":5}";
        let body = extract_http_body(req);
        assert_eq!(parse_grant_body(body), Some(("youtube.com".to_string(), 5)));

        let mut shared = BridgeShared::default();
        shared.is_focusing_unpaused = true;
        shared.restricted_domains = vec!["youtube.com".to_string()];
        assert_eq!(evaluate_domain_status(&shared, "www.youtube.com"), ("blocked", 0));

        shared.active_pass_domains.push(("youtube.com".to_string(), 240));
        assert_eq!(evaluate_domain_status(&shared, "youtube.com"), ("ok", 240));

        // Regression test: netflix.com must never be blocked by x.com substring match
        shared.restricted_domains.push("x.com".to_string());
        assert_eq!(evaluate_domain_status(&shared, "netflix.com"), ("ok", 0));
        assert_eq!(evaluate_domain_status(&shared, "www.netflix.com"), ("ok", 0));
        assert_eq!(evaluate_domain_status(&shared, "x.com"), ("blocked", 0));
        assert_eq!(evaluate_domain_status(&shared, "mobile.x.com"), ("blocked", 0));
    }
}

