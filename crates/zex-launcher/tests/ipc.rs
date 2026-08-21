use std::sync::mpsc;

use zex_launcher::ipc::{Answer, Bridge, Demand, Dial, Hit, Mode, Request};

/// Pump the daemon queue, answering each request, until the socket disappears
fn pump_queue(receiver: flume::Receiver<Demand>) -> mpsc::Receiver<()> {
    let (done_tx, done_rx) = mpsc::channel();
    std::thread::spawn(move || {
        while let Ok(demand) = receiver.recv() {
            let answer = match demand.request {
                Request::Show(_) => Answer::Done,
                Request::Hide => Answer::Done,
                Request::Toggle(_) => Answer::Done,
                Request::Query { text, .. } => Answer::Hits(vec![Hit {
                    kind: "app".into(),
                    title: text,
                    note: Some("mock".into()),
                }]),
                Request::Run(name) if name == "boom" => Answer::Done,
                Request::Quit | Request::Reload | Request::Run(_) => Answer::Done,
                Request::OpenWindow(_) | Request::ToggleWindow(_) | Request::CloseWindow(_) => {
                    Answer::WindowResult("ok".into())
                }
            };
            if demand.reply.send(answer).is_err() {
                break;
            }
        }
        let _ = done_tx.send(());
    });
    done_rx
}

#[test]
fn full_roundtrip_over_the_socket() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (queue, receiver) = flume::unbounded();
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("ipc.sock");

    let bridge = runtime
        .block_on(Bridge::start_at(queue, &socket))
        .expect("bridge binds");
    let done = pump_queue(receiver);

    runtime.block_on(async {
        assert!(zex_launcher::ipc::is_listening(&socket));

        let dial = Dial::open_at(&socket).await.expect("dial connects");
        dial.toggle(Some(vec![Mode::Emojis]))
            .await
            .expect("toggle ok");
        dial.hide().await.expect("hide ok");

        let hits = dial.query("firefox", 5).await.expect("query ok");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "firefox");
        assert_eq!(hits[0].kind, "app");

        dial.quit().await.expect("quit ok");
    });

    drop(bridge);
    runtime.block_on(async {
        tokio::task::yield_now().await;
    });
    assert!(!zex_launcher::ipc::is_listening(&socket));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let done_signal = loop {
        runtime.block_on(async {
            tokio::task::yield_now().await;
        });
        match done.recv_timeout(std::time::Duration::from_millis(100)) {
            std::result::Result::Ok(signal) => break Ok(signal),
            std::result::Result::Err(_e) if std::time::Instant::now() < deadline => continue,
            std::result::Result::Err(e) => break Err(e),
        }
    };
    assert!(
        done_signal.is_ok(),
        "queue pump finished, got {done_signal:?}"
    );
}

#[test]
fn refused_requests_surface_as_faults() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (queue, receiver) = flume::unbounded();
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("ipc.sock");

    let _bridge = runtime
        .block_on(Bridge::start_at(queue, &socket))
        .expect("bridge binds");

    std::thread::spawn(move || {
        while let Ok(demand) = receiver.recv() {
            let _ = demand.reply.send(Answer::Done);
        }
    });

    runtime.block_on(async {
        let dial = Dial::open_at(&socket).await.expect("dial connects");
        dial.quit().await.expect("quit ok");
    });
}

#[test]
fn mode_and_fault_round_trip_over_the_wire() {
    let modes = vec![Mode::Combined, Mode::Clipboard, Mode::Windows];
    let raw = serde_json::to_string(&modes).unwrap();
    assert_eq!(raw, r#"["combined","clipboard","windows"]"#);
    let back: Vec<Mode> = serde_json::from_str(&raw).unwrap();
    assert_eq!(back, modes);

    let fault = zex_launcher::ipc::Fault::Rejected("nope".into());
    let raw = serde_json::to_string(&fault).unwrap();
    let back: zex_launcher::ipc::Fault = serde_json::from_str(&raw).unwrap();
    assert_eq!(back, fault);
}

#[test]
fn running_bridge_on_the_same_socket_is_refused() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (queue, _receiver) = flume::unbounded();
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("ipc.sock");

    let first = runtime
        .block_on(Bridge::start_at(queue.clone(), &socket))
        .expect("first bridge binds");
    let second = runtime.block_on(Bridge::start_at(queue, &socket));
    assert!(second.is_err());
    drop(first);
}
