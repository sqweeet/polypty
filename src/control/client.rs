use std::path::Path;

use anyhow::Result;

use super::{ControlRequest, ControlResponse};

#[cfg(unix)]
pub(super) fn exchange(path: &Path, request: &ControlRequest) -> Result<ControlResponse> {
    use std::io::Read;
    use std::net::Shutdown;
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    use anyhow::Context;

    let mut stream = UnixStream::connect(path).with_context(|| {
        format!(
            "no running polypty session at {} (start interactive `polypty` first)",
            path.display()
        )
    })?;
    let timeout = Some(Duration::from_secs(4));
    stream.set_read_timeout(timeout)?;
    stream.set_write_timeout(timeout)?;
    serde_json::to_writer(&mut stream, request).context("encode control request")?;
    stream.shutdown(Shutdown::Write)?;
    let mut bytes = Vec::new();
    stream
        .take(16 * 1024 * 1024)
        .read_to_end(&mut bytes)
        .context("read polypty control response")?;
    serde_json::from_slice(&bytes).context("decode polypty control response")
}

#[cfg(not(unix))]
pub(super) fn exchange(_: &Path, _: &ControlRequest) -> Result<ControlResponse> {
    anyhow::bail!("polypty control sessions require Unix sockets")
}
