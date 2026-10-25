//! The arm probe: how the channel server asks Claude to arm it.
//!
//! A probe is one `notifications/claude/channel` event whose one-line
//! content asks the model to run `<exe> channel-ready --nonce <nonce>` —
//! the CLI command that sends the `arm` request back over the endpoint
//! socket (design decision D3). The probe proves the full path once per
//! process: the session flag, the org policy, the provider, and that the
//! model reads the events. The first probe goes out after
//! `notifications/initialized`; because Claude Code may register the
//! channel listener later than that, an unarmed server repeats the probe
//! on a `report` request — the same nonce, at most once per
//! [`REPEAT_PERIOD`] and at most [`MAX_REPEATS`] times per process, so a
//! flagged session whose model never runs the command sees at most four
//! probe turns, not one per minute. The repeat itself is triggered by the
//! unarmed branch of the report handler ([`super::connection`]).
//!
//! The clock behind the repeat gate is injected (epoch milliseconds), so
//! the period and the cap are testable without waiting.

use serde_json::{Value, json};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use super::rpc::{CHANNEL_NOTIFICATION_METHOD, LineWriter, RpcNotification};

/// The entropy of one nonce: 16 bytes from `/dev/urandom`, spelled as 32
/// lowercase hex characters — no new crate for that spelling.
const NONCE_BYTES: usize = 16;

/// The minimum gap between two probe emissions: a report on an unarmed
/// server repeats the probe at most once per period.
pub const REPEAT_PERIOD: Duration = Duration::from_secs(60);

/// The most probe repeats one process sends, after the first probe: at
/// most four probe turns per process, not one per minute.
pub const MAX_REPEATS: u32 = 3;

/// The stdin notification that triggers the first probe.
pub const INITIALIZED: &str = "notifications/initialized";

/// The clock the repeat gate reads: milliseconds since the Unix epoch.
pub type Clock = Box<dyn Fn() -> u64 + Send + Sync>;

/// The repeat gate: when the last probe went out, and how many repeats
/// the process has already sent.
#[derive(Debug, Default)]
struct RepeatState {
    last_emit_millis: Option<u64>,
    repeats: u32,
}

/// The probe one server emits: the nonce, the command line Claude runs,
/// and the repeat gate.
///
/// The nonce is fixed for the life of the process — the probe repeats
/// carry it too, so the `arm` request that answers any of them matches
/// the same [`crate::mcp::EndpointState`] nonce.
pub struct Probe {
    nonce: String,
    /// The shell-quoted absolute path of the running binary.
    exe: String,
    repeat: Mutex<RepeatState>,
    now: Clock,
}

impl Probe {
    /// A probe for the running binary, with a fresh nonce from
    /// `/dev/urandom` and the real clock.
    ///
    /// # Errors
    /// When the running binary's path cannot be read, or `/dev/urandom`
    /// cannot supply [`NONCE_BYTES`] bytes.
    pub fn for_current_exe() -> io::Result<Self> {
        let exe = std::env::current_exe()?;
        let nonce = fresh_nonce()?;
        Ok(Self::new(nonce, &exe, Box::new(real_now_millis)))
    }

    /// A probe with a given nonce, running-binary path and clock — the
    /// constructor for callers that already hold both, and for tests.
    pub fn new(nonce: String, exe: &Path, now: Clock) -> Self {
        Self {
            nonce,
            exe: shell_quote(&exe.to_string_lossy()),
            repeat: Mutex::new(RepeatState::default()),
            now,
        }
    }

    /// The nonce the probe carries and the `arm` request must match.
    pub fn nonce(&self) -> &str {
        &self.nonce
    }

    /// Emits the first probe through the shared writer, and stamps the
    /// clock the repeat period measures from — the first probe counts
    /// against the period, not against the repeat cap.
    ///
    /// # Errors
    /// When the shared writer fails, or its lock is poisoned.
    pub fn emit<W: io::Write>(&self, writer: &LineWriter<W>) -> io::Result<()> {
        self.stamp((self.now)())?;
        writer.write_json(&self.event())
    }

    /// Emits the first probe when `notification` is the trigger: the stdin
    /// loop calls this for every parsed notification, and only
    /// [`INITIALIZED`] emits. Returns whether the probe went out.
    ///
    /// # Errors
    /// When the shared writer fails, or its lock is poisoned.
    pub fn emit_after_notification<W: io::Write>(
        &self,
        notification: &RpcNotification,
        writer: &LineWriter<W>,
    ) -> io::Result<bool> {
        if notification.method != INITIALIZED {
            return Ok(false);
        }
        self.emit(writer)?;
        Ok(true)
    }

    /// Emits a repeat probe when the repeat rules allow one: at most once
    /// per [`REPEAT_PERIOD`] and at most [`MAX_REPEATS`] times per
    /// process, both measured from the last probe of any kind. Returns
    /// whether the probe went out.
    ///
    /// # Errors
    /// When the shared writer fails, or its lock is poisoned.
    pub fn emit_repeat<W: io::Write>(&self, writer: &LineWriter<W>) -> io::Result<bool> {
        let allowed = {
            let mut repeat = self.lock_repeat()?;
            let now = (self.now)();
            let within_period = repeat
                .last_emit_millis
                .is_some_and(|last| now < last.saturating_add(REPEAT_PERIOD.as_millis() as u64));
            if repeat.repeats >= MAX_REPEATS || within_period {
                false
            } else {
                repeat.repeats += 1;
                repeat.last_emit_millis = Some(now);
                true
            }
        };
        if !allowed {
            return Ok(false);
        }
        writer.write_json(&self.event())?;
        Ok(true)
    }

    /// Records `now_millis` as the time the last probe went out.
    fn stamp(&self, now_millis: u64) -> io::Result<()> {
        self.lock_repeat()?.last_emit_millis = Some(now_millis);
        Ok(())
    }

    /// Locks the repeat state.
    ///
    /// # Errors
    /// Poisoning means an earlier update panicked mid-way; the next call
    /// gets a `BrokenPipe` error instead of a panic, so the caller treats
    /// it like any other emission failure.
    fn lock_repeat(&self) -> io::Result<MutexGuard<'_, RepeatState>> {
        self.repeat.lock().map_err(|_| {
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "the probe repeat state was poisoned by a panicked update",
            )
        })
    }

    /// The probe event: the one-line command in `content`, the nonce in
    /// `meta`, and nothing else — the meta keys are identifiers, as the
    /// Channels contract requires.
    fn event(&self) -> Value {
        json!({
            "jsonrpc": "2.0",
            "method": CHANNEL_NOTIFICATION_METHOD,
            "params": {
                "content": self.content(),
                "meta": {
                    "kind": "probe",
                    "nonce": self.nonce,
                },
            },
        })
    }

    /// The one-line content: run the arm command, and nothing else.
    fn content(&self) -> String {
        format!(
            "Run {} channel-ready --nonce {} and do nothing else.",
            self.exe, self.nonce,
        )
    }
}

impl std::fmt::Debug for Probe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Probe")
            .field("nonce", &self.nonce)
            .field("exe", &self.exe)
            .finish_non_exhaustive()
    }
}

/// Quotes `value` as one POSIX shell word: wrapped in single quotes, with
/// an embedded single quote spelled `'\''` — so the probe's command works
/// for any binary path.
fn shell_quote(value: &str) -> String {
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('\'');
    for character in value.chars() {
        if character == '\'' {
            quoted.push_str("'\\''");
        } else {
            quoted.push(character);
        }
    }
    quoted.push('\'');
    quoted
}

/// A fresh nonce: [`NONCE_BYTES`] bytes from `/dev/urandom`, as 32
/// lowercase hex characters.
///
/// # Errors
/// When `/dev/urandom` cannot be opened, or cannot supply the bytes.
fn fresh_nonce() -> io::Result<String> {
    let mut bytes = [0u8; NONCE_BYTES];
    File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// The real clock: milliseconds since the Unix epoch.
fn real_now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    const NONCE: &str = "00112233445566778899aabbccddeeff";

    /// A stdout recorder the test inspects through clones of its shared
    /// buffer, since the shared [`LineWriter`] keeps the writer private.
    #[derive(Clone, Default)]
    struct Recorder(Arc<Mutex<Vec<u8>>>);

    impl io::Write for Recorder {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn stdout_of(recorder: &Recorder) -> String {
        String::from_utf8_lossy(&recorder.0.lock().unwrap()).to_string()
    }

    /// A probe with a fixed nonce and exe whose clock reads `cell`.
    fn probe_with_clock(exe: &Path, cell: Arc<Mutex<u64>>) -> Probe {
        Probe::new(
            NONCE.to_string(),
            exe,
            Box::new(move || *cell.lock().unwrap()),
        )
    }

    /// The probe event the writer received, as JSON.
    fn probe_event(recorder: &Recorder) -> Value {
        let stdout = stdout_of(recorder);
        assert_eq!(stdout.matches('\n').count(), 1, "one line: {stdout:?}");
        serde_json::from_str(stdout.trim_end()).unwrap()
    }

    // Row 4.1: the "Probe after initialized" scenario.

    #[test]
    fn initialized_emits_one_probe_with_a_fresh_nonce() {
        let probe = probe_with_clock(Path::new("/usr/bin/pmux-explorer"), Arc::new(Mutex::new(0)));
        let recorder = Recorder::default();
        let writer = LineWriter::new(recorder.clone());

        let emitted = probe
            .emit_after_notification(
                &RpcNotification {
                    method: "notifications/initialized".to_string(),
                    params: Value::Null,
                },
                &writer,
            )
            .unwrap();
        assert!(emitted);

        let event = probe_event(&recorder);
        assert_eq!(event["method"], CHANNEL_NOTIFICATION_METHOD);
        assert_eq!(event["params"]["meta"]["kind"], "probe");
        assert_eq!(event["params"]["meta"]["nonce"], NONCE);
        assert_eq!(
            event["params"]["content"],
            format!(
                "Run '/usr/bin/pmux-explorer' channel-ready --nonce {NONCE} and do nothing else."
            ),
        );
    }

    #[test]
    fn another_notification_emits_nothing() {
        let probe = probe_with_clock(Path::new("/usr/bin/pmux-explorer"), Arc::new(Mutex::new(0)));
        let recorder = Recorder::default();
        let writer = LineWriter::new(recorder.clone());

        let emitted = probe
            .emit_after_notification(
                &RpcNotification {
                    method: "notifications/cancelled".to_string(),
                    params: Value::Null,
                },
                &writer,
            )
            .unwrap();

        assert!(!emitted);
        assert_eq!(stdout_of(&recorder), "");
    }

    // Row 4.1: the "Probe names the running binary" scenario.

    #[test]
    fn the_probe_names_the_shell_quoted_running_binary() {
        let probe = probe_with_clock(Path::new("/opt/px/pmux-explorer"), Arc::new(Mutex::new(0)));
        assert!(
            probe.content().contains(
                format!("'/opt/px/pmux-explorer' channel-ready --nonce {NONCE}").as_str()
            ),
            "content: {}",
            probe.content(),
        );
        assert!(!probe.content().contains('\n'), "one line");
    }

    #[test]
    fn a_binary_path_with_a_quote_is_quoted_whole() {
        let probe = Probe::new(NONCE.to_string(), Path::new("/op't/px"), Box::new(|| 0));
        assert_eq!(
            probe.content(),
            format!("Run '/op'\\''t/px' channel-ready --nonce {NONCE} and do nothing else."),
        );
    }

    // Row 4.1: the nonce is 16 bytes of entropy as 32 hex characters.

    #[test]
    fn a_fresh_nonce_is_32_hex_characters_and_differs_per_process() {
        let first = fresh_nonce().unwrap();
        let second = fresh_nonce().unwrap();
        for nonce in [&first, &second] {
            assert_eq!(nonce.len(), 2 * NONCE_BYTES);
            assert!(
                nonce
                    .bytes()
                    .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')),
            );
        }
        assert_ne!(first, second);
    }

    // Row 4.1: the arm nonce and the probe nonce are one and the same.

    #[test]
    fn the_probe_reports_its_nonce_to_the_arm_state() {
        let probe = probe_with_clock(Path::new("/usr/bin/pmux-explorer"), Arc::new(Mutex::new(0)));
        assert_eq!(probe.nonce(), NONCE);
    }

    // Row 4.3: the repeat gate, at the gate level; the report-triggered
    // scenarios are pinned in `connection.rs`.

    #[test]
    fn a_repeat_within_the_period_is_refused_and_after_it_is_allowed() {
        let cell = Arc::new(Mutex::new(0u64));
        let probe = probe_with_clock(Path::new("/usr/bin/pmux-explorer"), cell.clone());
        let recorder = Recorder::default();
        let writer = LineWriter::new(recorder.clone());

        probe.emit(&writer).unwrap();
        // 10 seconds after the first probe: no repeat.
        *cell.lock().unwrap() = 10_000;
        assert!(!probe.emit_repeat(&writer).unwrap());
        // 90 seconds after the first probe: one repeat, same nonce.
        *cell.lock().unwrap() = 90_000;
        assert!(probe.emit_repeat(&writer).unwrap());

        // Two probe lines on stdout, the repeat carrying the same nonce.
        let stdout = stdout_of(&recorder);
        assert_eq!(stdout.matches('\n').count(), 2);
        let last = stdout.lines().last().unwrap();
        let repeated: Value = serde_json::from_str(last).unwrap();
        assert_eq!(
            repeated["params"]["meta"],
            json!({"kind": "probe", "nonce": NONCE})
        );
    }

    #[test]
    fn the_lifetime_cap_stops_the_repeats_at_three() {
        let cell = Arc::new(Mutex::new(0u64));
        let probe = probe_with_clock(Path::new("/usr/bin/pmux-explorer"), cell.clone());
        let recorder = Recorder::default();
        let writer = LineWriter::new(recorder.clone());

        // The first probe, then three repeats spaced beyond the period.
        probe.emit(&writer).unwrap();
        for minute in 1..=MAX_REPEATS {
            *cell.lock().unwrap() = u64::from(minute) * 70_000;
            assert!(probe.emit_repeat(&writer).unwrap(), "repeat {minute}");
        }
        // The fourth repeat is refused, however late it arrives.
        *cell.lock().unwrap() = 600_000;
        assert!(!probe.emit_repeat(&writer).unwrap());

        // Four probe lines in total: the first probe and 3 repeats.
        let stdout = stdout_of(&recorder);
        assert_eq!(stdout.matches('\n').count(), 4);
    }

    #[test]
    fn a_failed_repeat_write_still_counts_as_sent() {
        // The repeat gate updates before the write, so a broken stdout
        // consumes the repeat rather than retrying it every report.
        struct Failing;
        impl io::Write for Failing {
            fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
                Err(io::Error::new(io::ErrorKind::BrokenPipe, "gone"))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let cell = Arc::new(Mutex::new(0u64));
        let probe = probe_with_clock(Path::new("/usr/bin/pmux-explorer"), cell.clone());
        let writer = LineWriter::new(Failing);

        assert!(probe.emit_repeat(&writer).is_err());
        // The failed write consumed its repeat: the next three gate
        // passes are repeats 2 and 3, and then the cap closes the gate —
        // had the failures not been consumed, this last call would still
        // be repeat 3 and would fail again.
        *cell.lock().unwrap() = 90_000;
        assert!(probe.emit_repeat(&writer).is_err());
        *cell.lock().unwrap() = 180_000;
        assert!(probe.emit_repeat(&writer).is_err());
        *cell.lock().unwrap() = 270_000;
        assert!(!probe.emit_repeat(&writer).unwrap());
    }

    // The production constructor.

    #[test]
    fn the_probe_of_the_running_binary_carries_a_real_path() {
        let probe = Probe::for_current_exe().unwrap();
        assert_eq!(probe.nonce().len(), 2 * NONCE_BYTES);
        let exe = std::env::current_exe().unwrap();
        assert!(
            probe
                .content()
                .starts_with(&format!("Run '{}", exe.display())),
            "content: {}",
            probe.content(),
        );
    }

    // The shell quoting, directly.

    #[test]
    fn shell_quote_wraps_and_escapes_single_quotes() {
        assert_eq!(
            shell_quote("/opt/px/pmux-explorer"),
            "'/opt/px/pmux-explorer'"
        );
        assert_eq!(shell_quote("op't"), "'op'\\''t'");
        assert_eq!(shell_quote(""), "''");
    }

    // The probe is shareable across the accept and stdin threads.

    #[test]
    fn the_probe_moves_between_threads() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Probe>();
        let probe = probe_with_clock(Path::new("/usr/bin/pmux-explorer"), Arc::new(Mutex::new(0)));
        thread::spawn(move || drop(probe)).join().unwrap();
    }
}
