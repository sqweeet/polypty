use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

use anyhow::Result;

use crate::{
    info::TabInfo,
    platform::clipboard::{Clipboard, ClipboardKind},
    session::{SessionFactory, TerminalSession},
};

use super::App;

struct EmptyClipboard;

impl Clipboard for EmptyClipboard {
    fn read(&self, _: ClipboardKind) -> Option<String> {
        None
    }
}

struct FakeFactory {
    spawns: Arc<Mutex<Vec<(u64, u16, u16)>>>,
    kills: Option<Arc<AtomicUsize>>,
}

impl SessionFactory for FakeFactory {
    fn spawn(&mut self, id: u64, cols: u16, rows: u16) -> Result<Box<dyn TerminalSession>> {
        self.spawns.lock().unwrap().push((id, cols, rows));
        Ok(Box::new(FakeSession::new(
            id,
            cols,
            rows,
            self.kills.clone(),
        )))
    }
}

struct FakeSession {
    id: u64,
    info: TabInfo,
    parser: vt100::Parser,
    alive: bool,
    dirty: bool,
    kills: Option<Arc<AtomicUsize>>,
}

impl FakeSession {
    fn new(id: u64, cols: u16, rows: u16, kills: Option<Arc<AtomicUsize>>) -> Self {
        Self {
            id,
            info: TabInfo::default(),
            parser: vt100::Parser::new(rows, cols, 0),
            alive: true,
            dirty: true,
            kills,
        }
    }
}

impl TerminalSession for FakeSession {
    fn id(&self) -> u64 {
        self.id
    }
    fn info(&self) -> &TabInfo {
        &self.info
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
    fn write_all(&mut self, _: &[u8]) -> Result<()> {
        Ok(())
    }
    fn try_reap(&mut self) -> bool {
        !self.alive
    }
    fn is_alive(&self) -> bool {
        self.alive
    }
    fn kill(&mut self) {
        if self.alive {
            if let Some(kills) = &self.kills {
                kills.fetch_add(1, Ordering::Relaxed);
            }
        }
        self.alive = false;
    }
    fn is_dirty(&self) -> bool {
        self.dirty
    }
    fn mark_rendered(&mut self) {
        self.dirty = false;
    }
}

#[test]
fn application_spawns_sessions_through_the_injected_factory() {
    let spawns = Arc::new(Mutex::new(Vec::new()));
    let factory = FakeFactory {
        spawns: Arc::clone(&spawns),
        kills: None,
    };
    let mut app = App::with_services(100, 30, Box::new(EmptyClipboard), Box::new(factory)).unwrap();
    app.spawn_workspace().unwrap();

    assert_eq!(*spawns.lock().unwrap(), vec![(1, 82, 30), (2, 82, 30)]);
    assert_eq!(app.book.len(), 2);
}

#[test]
fn dropping_application_kills_live_sessions() {
    let kills = Arc::new(AtomicUsize::new(0));
    let factory = FakeFactory {
        spawns: Arc::new(Mutex::new(Vec::new())),
        kills: Some(Arc::clone(&kills)),
    };
    let app = App::with_services(100, 30, Box::new(EmptyClipboard), Box::new(factory)).unwrap();

    drop(app);

    assert_eq!(kills.load(Ordering::Relaxed), 1);
}
