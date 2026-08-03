use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{bail, Context, Result};

use super::PendingRequest;
use crate::control::{ControlRequest, ControlResponse};

const IO_TIMEOUT: Duration = Duration::from_millis(750);
const MAX_REQUEST_BYTES: u64 = 1024 * 1024;

pub(crate) struct ControlServer {
    path: PathBuf,
    requests: mpsc::Receiver<PendingRequest>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl ControlServer {
    pub(crate) fn bind(path: PathBuf) -> Result<Self> {
        prepare_path(&path)?;
        let listener = UnixListener::bind(&path)
            .with_context(|| format!("bind control socket at {}", path.display()))?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        listener.set_nonblocking(true)?;
        let (send, requests) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let worker = thread::Builder::new()
            .name("polypty-control".into())
            .spawn(move || serve(listener, send, worker_stop))
            .context("spawn polypty control server")?;
        Ok(Self {
            path,
            requests,
            stop,
            worker: Some(worker),
        })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn try_recv(&self) -> Option<PendingRequest> {
        self.requests.try_recv().ok()
    }
}

impl Drop for ControlServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        remove_owned_socket(&self.path);
    }
}

fn serve(listener: UnixListener, requests: mpsc::Sender<PendingRequest>, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::Acquire) {
        match listener.accept() {
            Ok((stream, _)) => handle_connection(stream, &requests),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(8));
            }
            Err(_) => thread::sleep(Duration::from_millis(20)),
        }
    }
}

fn handle_connection(mut stream: UnixStream, requests: &mpsc::Sender<PendingRequest>) {
    let response = read_request(&stream).and_then(|request| {
        let (reply, receive) = mpsc::sync_channel(1);
        requests
            .send(PendingRequest { request, reply })
            .map_err(|_| "polypty event loop stopped".to_string())?;
        receive
            .recv_timeout(Duration::from_secs(3))
            .map_err(|_| "polypty event loop did not respond".to_string())
    });
    let response = response.unwrap_or_else(ControlResponse::error);
    if let Ok(mut json) = serde_json::to_vec(&response) {
        json.push(b'\n');
        let _ = stream.set_write_timeout(Some(IO_TIMEOUT));
        let _ = stream.write_all(&json);
    }
}

fn read_request(stream: &UnixStream) -> std::result::Result<ControlRequest, String> {
    stream
        .set_read_timeout(Some(IO_TIMEOUT))
        .map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    stream
        .take(MAX_REQUEST_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_REQUEST_BYTES {
        return Err("control request is too large".into());
    }
    serde_json::from_slice(&bytes).map_err(|error| format!("invalid control request: {error}"))
}

fn prepare_path(path: &Path) -> Result<()> {
    let Some(parent) = path.parent() else {
        bail!("control socket path has no parent: {}", path.display());
    };
    if !parent.is_dir() {
        bail!(
            "control socket directory does not exist: {}",
            parent.display()
        );
    }
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.uid() != unsafe { libc::geteuid() } || !metadata.file_type().is_socket() {
            bail!(
                "refusing to replace unsafe control socket: {}",
                path.display()
            );
        }
        fs::remove_file(path)?;
    }
    Ok(())
}

fn remove_owned_socket(path: &Path) {
    if fs::symlink_metadata(path).is_ok_and(|metadata| {
        metadata.uid() == unsafe { libc::geteuid() } && metadata.file_type().is_socket()
    }) {
        let _ = fs::remove_file(path);
    }
}
