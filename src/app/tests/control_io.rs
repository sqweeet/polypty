use std::sync::{Arc, Mutex};

use anyhow::Result;

use crate::{
    control::{ControlRequest, ControlResponse},
    info::TabInfo,
    session::{SessionFactory, TerminalSession},
};

use super::{App, EmptyClipboard};

struct IoFactory {
    writes: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl SessionFactory for IoFactory {
    fn spawn(&mut self, id: u64, _: u64, cols: u16, rows: u16) -> Result<Box<dyn TerminalSession>> {
        let mut parser = vt100::Parser::new(rows, cols, 0);
        parser.process(b"hello\r\nworld");
        Ok(Box::new(IoSession {
            id,
            parser,
            writes: Arc::clone(&self.writes),
        }))
    }
}

struct IoSession {
    id: u64,
    parser: vt100::Parser,
    writes: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl TerminalSession for IoSession {
    fn id(&self) -> u64 {
        self.id
    }
    fn info(&self) -> &TabInfo {
        static INFO: std::sync::LazyLock<TabInfo> = std::sync::LazyLock::new(TabInfo::default);
        &INFO
    }
    fn screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }
    fn poll(&mut self) -> Result<bool> {
        Ok(false)
    }
    fn last_poll_bytes(&self) -> usize {
        0
    }
    fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        self.parser.screen_mut().set_size(rows, cols);
        Ok(())
    }
    fn write_all(&mut self, data: &[u8]) -> Result<()> {
        self.writes.lock().unwrap().push(data.to_vec());
        Ok(())
    }
    fn try_reap(&mut self) -> bool {
        false
    }
    fn is_alive(&self) -> bool {
        true
    }
    fn kill(&mut self) {}
    fn is_dirty(&self) -> bool {
        true
    }
    fn mark_rendered(&mut self) {}
}

#[test]
fn control_capture_and_send_keys_address_a_live_pane() {
    let writes = Arc::new(Mutex::new(Vec::new()));
    let factory = IoFactory {
        writes: Arc::clone(&writes),
    };
    let mut app = App::with_services(80, 24, Box::new(EmptyClipboard), Box::new(factory)).unwrap();
    let capture = app.handle_control(ControlRequest::CapturePane {
        tab: Some("1".into()),
        pane: Some(1),
    });
    assert!(matches!(
        capture,
        ControlResponse::Capture { capture } if capture.text == "hello\nworld"
    ));

    let sent = app.handle_control(ControlRequest::SendKeys {
        tab: Some("@1".into()),
        pane: Some(1),
        text: "echo ok".into(),
        enter: true,
    });
    assert!(matches!(sent, ControlResponse::Ack { .. }));
    assert_eq!(*writes.lock().unwrap(), vec![b"echo ok\r".to_vec()]);
}
