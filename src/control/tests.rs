use super::{args, ControlRequest, ControlResponse};

fn words(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn tmux_style_aliases_map_to_control_requests() {
    let list = args::parse(&words(&["list-windows", "--json"])).unwrap();
    assert_eq!(list.request, ControlRequest::ListTabs);
    assert!(list.json);

    let select = args::parse(&words(&["select-window", "-t", "@42"])).unwrap();
    assert_eq!(
        select.request,
        ControlRequest::SelectTab {
            target: "@42".into()
        }
    );
}

#[test]
fn send_keys_keeps_targets_text_and_enter() {
    let parsed = args::parse(&words(&[
        "send-keys",
        "-t",
        "2",
        "-p",
        "%9",
        "--enter",
        "--",
        "cargo",
        "test",
    ]))
    .unwrap();
    assert_eq!(
        parsed.request,
        ControlRequest::SendKeys {
            tab: Some("2".into()),
            pane: Some(9),
            text: "cargo test".into(),
            enter: true,
        }
    );
}

#[test]
fn explicit_tab_uses_its_active_pane_not_the_callers_pane() {
    let mut request = ControlRequest::CapturePane {
        tab: Some("2".into()),
        pane: None,
    };
    super::cli::apply_context(&mut request, Some("@1".into()), Some(9));
    assert_eq!(
        request,
        ControlRequest::CapturePane {
            tab: Some("2".into()),
            pane: None
        }
    );
}

#[test]
fn protocol_round_trips_as_tagged_json() {
    let response = ControlResponse::Pong { pid: 123 };
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"response\":\"pong\""));
    assert_eq!(
        serde_json::from_str::<ControlResponse>(&json).unwrap(),
        response
    );
}

#[cfg(unix)]
#[test]
fn unix_socket_exchanges_a_request_and_cleans_up() {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, Instant};

    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "mux-control-test-{}-{}.sock",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let server = super::ControlServer::bind(path.clone()).unwrap();
    let client_path = path.clone();
    let client = std::thread::spawn(move || {
        super::client::exchange(&client_path, &ControlRequest::Ping).unwrap()
    });
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(pending) = server.try_recv() {
            pending.respond_with(|request| {
                assert_eq!(request, ControlRequest::Ping);
                ControlResponse::Pong { pid: 42 }
            });
            break;
        }
        assert!(Instant::now() < deadline, "control request timed out");
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(client.join().unwrap(), ControlResponse::Pong { pid: 42 });
    drop(server);
    assert!(!path.exists());
}
