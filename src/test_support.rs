//! Test-only helpers shared across the crate's unit tests.
//!
//! One copy of each helper, so a fix in one place cannot drift from the
//! rest: the probe with an injected clock, the stdout recorder the
//! shared [`LineWriter`] is inspected through, the writer whose every
//! write fails, and the detached endpoint serve scaffold. Compiled only
//! under `cfg(test)`, so the integration tests in `tests/` never see it.

use std::io;
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::mcp::{EndpointState, LineWriter, Probe, accept_loop};

/// A probe with the given nonce and exe whose clock reads `cell`, so a
/// test drives the repeat period and cap by moving `cell`.
pub(crate) fn probe_with_clock(nonce: &str, exe: &Path, cell: Arc<Mutex<u64>>) -> Probe {
    Probe::new(
        nonce.to_string(),
        exe,
        Box::new(move || *cell.lock().unwrap()),
    )
}

/// A stdout recorder whose state a test inspects through clones of its
/// shared buffers, since the shared [`LineWriter`] keeps the writer
/// itself private. `flushes` counts the flushes, so a test can pin the
/// one-flush-per-line rule.
#[derive(Clone, Default)]
pub(crate) struct Recorder {
    buf: Arc<Mutex<Vec<u8>>>,
    flushes: Arc<Mutex<usize>>,
}

impl Recorder {
    /// The bytes the writer received, as one string.
    pub(crate) fn stdout(&self) -> String {
        String::from_utf8_lossy(&self.buf.lock().unwrap()).to_string()
    }

    /// How many times the writer was flushed.
    pub(crate) fn flushes(&self) -> usize {
        *self.flushes.lock().unwrap()
    }
}

impl io::Write for Recorder {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        *self.flushes.lock().unwrap() += 1;
        Ok(())
    }
}

/// A writer whose every write fails with `BrokenPipe`, so a test pins
/// how a caller reports a failed stdout write.
pub(crate) struct Failing;

impl io::Write for Failing {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "gone"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Serves an already-bound endpoint on a detached thread, writing its
/// emissions to a sink: a test's client exchange runs against a real
/// socket without the test thread blocking on accept. The thread ends
/// with the process, since a blocking accept has no other stop.
pub(crate) fn serve_detached(
    listener: UnixListener,
    state: Arc<EndpointState>,
    read_timeout: Duration,
) {
    std::thread::spawn(move || {
        let _ = accept_loop(listener, state, LineWriter::new(io::sink()), read_timeout);
    });
}
