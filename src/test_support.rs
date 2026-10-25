//! Test-only helpers shared across the crate's unit tests.
//!
//! One copy of each helper, so a fix in one place cannot drift from the
//! rest: the uid the test filesystem artifacts own, the probe with an
//! injected clock, and the stdout recorder the shared [`LineWriter`] is
//! inspected through. Compiled only under `cfg(test)`, so the integration
//! tests in `tests/` never see it.

use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::mcp::Probe;

/// The current user's id: the uid every test filesystem artifact owns,
/// and the uid the endpoint's peer-uid gate expects. Tests inject a
/// different value to force a refusal.
pub(crate) fn own_uid() -> u32 {
    // SAFETY: `geteuid` has no failure mode and no preconditions.
    unsafe { libc::geteuid() }
}

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
