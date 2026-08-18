use std::io::{Read, Write};
use std::net::TcpListener;

use zex_launcher::chat::{Event, Fault, Profile, Role, Turn, answer, stream};

fn profile(port: u16) -> Profile {
    Profile {
        endpoint: format!("http://127.0.0.1:{port}"),
        model: "Qwythos-9B-v2:latest".to_string(),
        temperature: 0.7,
        max_tokens: 128,
        system_prompt: "You are a test assistant.".to_string(),
    }
}

fn turns() -> Vec<Turn> {
    vec![Turn {
        role: Role::User,
        text: "hi".to_string(),
    }]
}

/// One-shot HTTP server that answers a single request with `body`.
/// Returns the port it listened on.
fn serve_once(status: &str, body: &str) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let status = status.to_string();
    let body = body.to_string();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = Vec::new();
        let mut buffer = [0u8; 4096];
        loop {
            let read = stream.read(&mut buffer).unwrap();
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
            let text = String::from_utf8_lossy(&request);
            if let Some(header) = text.split("\r\n\r\n").next() {
                if header.contains("Content-Length") {
                    let length: usize = header
                        .lines()
                        .find_map(|line| line.strip_prefix("Content-Length: "))
                        .and_then(|value| value.trim().parse().ok())
                        .unwrap_or(0);
                    if text.len() >= text.find("\r\n\r\n").unwrap() + 4 + length {
                        break;
                    }
                }
            }
        }
        // sanity: the request reached with the expected payload shape
        let body_text = String::from_utf8_lossy(&request);
        assert!(body_text.contains("Qwythos-9B-v2:latest"), "model missing");

        let reply = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(reply.as_bytes());
    });
    port
}

#[test]
fn answer_returns_the_full_reply() {
    let body = r#"{"message":{"role":"assistant","content":"hello there"},"done":true}"#;
    let port = serve_once("200 OK", body);
    let text = answer(&profile(port), &turns()).unwrap();
    assert_eq!(text, "hello there");
}

#[test]
fn stream_yields_tokens_in_order_then_done() {
    let body = concat!(
        r#"{"message":{"content":"foo"},"done":false}"#,
        "\n",
        r#"{"message":{"content":"bar"},"done":false}"#,
        "\n",
        r#"{"message":{"content":""},"done":true}"#,
        "\n",
    );
    let port = serve_once("200 OK", body);
    let events = stream(&profile(port), turns()).collect();
    assert_eq!(
        events,
        vec![
            Event::Token("foo".into()),
            Event::Token("bar".into()),
            Event::Done,
        ]
    );
}

#[test]
fn upstream_status_is_reported_as_a_fault() {
    let port = serve_once("404 Not Found", r#"{"error":"model not found"}"#);
    let err = answer(&profile(port), &turns()).unwrap_err();
    assert!(matches!(&err, Fault::Denied(why) if why.contains("404")));
    assert!(err.to_string().contains("model not found"));
}

#[test]
fn mid_stream_error_line_appears_as_a_fault() {
    let body = concat!(
        r#"{"message":{"content":"partial"},"done":false}"#,
        "\n",
        r#"{"error":"context overflow"}"#,
        "\n",
    );
    let port = serve_once("200 OK", body);
    let events = stream(&profile(port), turns()).collect();
    eprintln!("DEBUG events: {events:?}");
    assert!(events.contains(&Event::Fault(Fault::Denied("context overflow".into()))));
    assert_eq!(events.last(), Some(&Event::Done));
}

#[test]
fn empty_text_answer_is_malformed() {
    let body = r#"{"message":{"role":"assistant","content":""},"done":true}"#;
    let port = serve_once("200 OK", body);
    let err = answer(&profile(port), &turns()).unwrap_err();
    assert!(matches!(err, Fault::Malformed(_)));
}

trait CollectExt {
    fn collect(self) -> Vec<Event>;
}

impl CollectExt for flume::Receiver<Event> {
    fn collect(self) -> Vec<Event> {
        let mut events = Vec::new();
        while let Ok(event) = self.recv_timeout(std::time::Duration::from_secs(5)) {
            if matches!(&event, Event::Done) {
                events.push(event);
                break;
            }
            events.push(event);
        }
        events
    }
}
