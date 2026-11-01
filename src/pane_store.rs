//! The pane-write seam: one owner for every write to an agent pane's
//! `@pane_*` options. A caller builds a [`PaneWrites`] list and hands it to a
//! [`PaneStore`]; the store applies the whole list as one batch, so the
//! sidebar's poll reads the pane state from before the batch or from after
//! it, never from between. Two adapters implement the trait: the tmux
//! adapter in [`crate::tmux::pane_store`] for production, and
//! [`MemPaneStore`] here for tests — the replacement of the old
//! `cfg(test)` `tmux::test_mock`.
//!
//! The write rules themselves (the status priority, the subagent guard, the
//! not-reported rule) live in the transition function built on this seam, not
//! here; this module only owns the shape of a write list and the two ways of
//! applying one.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::cli::sanitize_tmux_value;

/// One entry of a write list: set `key` to `value`, or unset it. The keys
/// are the `&'static str` option-name constants of `tmux::options`, not a
/// typed field enum — the schema table that would generate one waits for
/// another change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaneWrite {
    /// Store `value` under `key`, overwriting whatever the option held.
    Set(&'static str, String),
    /// Remove `key` from the pane's options.
    Unset(&'static str),
}

/// An ordered list of [`PaneWrite`] entries applied as one batch. The order
/// is the write order: a later entry wins over an earlier one for the same
/// key.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PaneWrites {
    entries: Vec<PaneWrite>,
}

impl PaneWrites {
    /// An empty write list.
    pub fn new() -> Self {
        PaneWrites {
            entries: Vec::new(),
        }
    }

    /// Append a `Set` entry.
    pub fn set(&mut self, key: &'static str, value: impl Into<String>) {
        self.entries.push(PaneWrite::Set(key, value.into()));
    }

    /// Append an `Unset` entry.
    pub fn unset(&mut self, key: &'static str) {
        self.entries.push(PaneWrite::Unset(key));
    }

    /// Whether nothing would be written; an empty list applies as a no-op.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// The entries in write order.
    pub fn iter(&self) -> impl Iterator<Item = &PaneWrite> {
        self.entries.iter()
    }

    /// Append every entry of `other`, keeping its order after the entries
    /// already present. A later key wins over an earlier one at apply time.
    pub fn merge(&mut self, other: &PaneWrites) {
        self.entries.extend(other.entries.iter().cloned());
    }
}

/// The pane options the write rules consult, read with one call. An absent
/// option is `None`; an option set to an empty value reads as `None` too,
/// matching [`PaneStore::get`]'s empty-for-unset convention.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PaneSnapshot {
    /// `@pane_status` — the high-level status (`idle` / `running` / ...).
    pub status: Option<String>,
    /// `@pane_bg_cmd` — the live background shell command.
    pub bg_cmd: Option<String>,
    /// `@pane_subagents` — the active subagent list.
    pub subagents: Option<String>,
    /// `@pane_started_at` — the current run's start timestamp.
    pub started_at: Option<String>,
}

/// The store seam every agent pane option access goes through: one read of
/// the rule-owned options, one read of any other option, and one batched
/// apply of a write list. The store is created once at a command's entry and
/// passed down as an argument; there is no global.
pub trait PaneStore {
    /// Read the options the write rules consult from `pane` in one call.
    fn snapshot(&self, pane: &str) -> PaneSnapshot;

    /// Read one option of `pane`. An unset option reads as the empty
    /// string, matching what the raw helpers returned before the seam.
    fn get(&self, pane: &str, key: &str) -> String;

    /// Apply `writes` to `pane` as one batch. An empty list is a no-op that
    /// touches nothing. Hook callers ignore the error, as they ignored tmux
    /// errors before the seam.
    fn apply(&self, pane: &str, writes: &PaneWrites) -> Result<(), String>;
}

/// The in-memory adapter: options held in a map keyed by `(pane, key)`,
/// with the same observable semantics tests relied on from the old
/// `tmux::test_mock` — `get` returns the empty string for an unset key, and
/// a pre-populated value stays exactly as inserted. `apply` encodes `Set`
/// values with [`sanitize_tmux_value`] like every other adapter, so a value
/// written through the store is always one field. The map sits behind a
/// `RefCell` because the trait's `apply` takes `&self` — the tmux adapter
/// needs no mutability, and the seam is threaded as a shared reference.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemPaneStore {
    options: RefCell<HashMap<(String, String), String>>,
}

impl MemPaneStore {
    /// An empty store.
    pub fn new() -> Self {
        MemPaneStore {
            options: RefCell::new(HashMap::new()),
        }
    }

    /// Pre-populate one option, before or between applies. Test setup only —
    /// the value is stored raw, un-encoded.
    pub fn insert(&self, pane: &str, key: &str, value: &str) {
        self.options
            .borrow_mut()
            .insert((pane.to_string(), key.to_string()), value.to_string());
    }

    /// Whether `key` is present for `pane`, even when its value is empty.
    pub fn contains(&self, pane: &str, key: &str) -> bool {
        self.options
            .borrow()
            .contains_key(&(pane.to_string(), key.to_string()))
    }

    fn field(&self, pane: &str, key: &str) -> Option<String> {
        let value = self.get(pane, key);
        (!value.is_empty()).then_some(value)
    }
}

impl PaneStore for MemPaneStore {
    fn snapshot(&self, pane: &str) -> PaneSnapshot {
        PaneSnapshot {
            status: self.field(pane, crate::tmux::PANE_STATUS),
            bg_cmd: self.field(pane, crate::tmux::PANE_BG_CMD),
            subagents: self.field(pane, crate::tmux::PANE_SUBAGENTS),
            started_at: self.field(pane, crate::tmux::PANE_STARTED_AT),
        }
    }

    fn get(&self, pane: &str, key: &str) -> String {
        self.options
            .borrow()
            .get(&(pane.to_string(), key.to_string()))
            .cloned()
            .unwrap_or_default()
    }

    fn apply(&self, pane: &str, writes: &PaneWrites) -> Result<(), String> {
        let mut options = self.options.borrow_mut();
        for write in writes.iter() {
            match write {
                PaneWrite::Set(key, value) => {
                    options.insert(
                        (pane.to_string(), key.to_string()),
                        sanitize_tmux_value(value),
                    );
                }
                PaneWrite::Unset(key) => {
                    options.remove(&(pane.to_string(), key.to_string()));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tmux::{
        PANE_ATTENTION, PANE_BG_CMD, PANE_STARTED_AT, PANE_STATUS, PANE_SUBAGENTS, PANE_WAIT_REASON,
    };

    const PANE: &str = "%SEAM";

    #[test]
    fn pane_writes_build_and_read_back_in_order() {
        let mut writes = PaneWrites::new();
        writes.set(PANE_STATUS, "running");
        writes.unset(PANE_STARTED_AT);
        writes.set(PANE_WAIT_REASON, "permission");
        writes.unset(PANE_ATTENTION);

        assert_eq!(writes.len(), 4);
        assert!(!writes.is_empty());
        let collected: Vec<(&str, Option<&str>)> = writes
            .iter()
            .map(|write| match write {
                PaneWrite::Set(key, value) => (*key, Some(value.as_str())),
                PaneWrite::Unset(key) => (*key, None),
            })
            .collect();
        assert_eq!(
            collected,
            vec![
                (PANE_STATUS, Some("running")),
                (PANE_STARTED_AT, None),
                (PANE_WAIT_REASON, Some("permission")),
                (PANE_ATTENTION, None),
            ]
        );
    }

    #[test]
    fn an_empty_write_list_reports_empty() {
        let writes = PaneWrites::new();
        assert!(writes.is_empty());
        assert_eq!(writes.len(), 0);
        assert_eq!(writes.iter().count(), 0);
    }

    #[test]
    fn mem_store_applies_a_batch_and_reads_it_back() {
        let store = MemPaneStore::new();
        let mut writes = PaneWrites::new();
        writes.set(PANE_STATUS, "running");
        writes.set(crate::tmux::PANE_PROMPT, "text");
        writes.unset(PANE_ATTENTION);
        store.apply(PANE, &writes).unwrap();

        assert_eq!(store.get(PANE, PANE_STATUS), "running");
        assert_eq!(store.get(PANE, PANE_ATTENTION), "");
        assert!(store.contains(PANE, PANE_STATUS));
        assert!(!store.contains(PANE, PANE_ATTENTION));
    }

    #[test]
    fn mem_store_applies_an_unset_over_an_earlier_set() {
        let store = MemPaneStore::new();
        let mut writes = PaneWrites::new();
        writes.set(PANE_WAIT_REASON, "permission");
        writes.unset(PANE_WAIT_REASON);
        store.apply(PANE, &writes).unwrap();

        assert_eq!(store.get(PANE, PANE_WAIT_REASON), "");
        assert!(!store.contains(PANE, PANE_WAIT_REASON));
    }

    #[test]
    fn mem_store_applies_an_empty_list_as_a_no_op() {
        let store = MemPaneStore::new();
        store.insert(PANE, PANE_STATUS, "running");
        let before = store.clone();

        store.apply(PANE, &PaneWrites::new()).unwrap();

        assert_eq!(store, before);
    }

    #[test]
    fn mem_store_get_returns_empty_for_unset_keys() {
        let store = MemPaneStore::new();
        assert_eq!(store.get(PANE, PANE_STATUS), "");
        assert!(!store.contains(PANE, PANE_STATUS));
    }

    #[test]
    fn mem_store_encodes_newline_carriage_return_and_pipe_into_one_field() {
        let store = MemPaneStore::new();
        let mut writes = PaneWrites::new();
        writes.set(PANE_WAIT_REASON, "line1\nline2|a\rb");
        store.apply(PANE, &writes).unwrap();

        assert_eq!(store.get(PANE, PANE_WAIT_REASON), "line1 line2 a b");
    }

    #[test]
    fn mem_store_snapshot_reads_the_four_rule_options() {
        let store = MemPaneStore::new();
        store.insert(PANE, PANE_STATUS, "waiting");
        store.insert(PANE, PANE_BG_CMD, "npm run dev");
        store.insert(PANE, PANE_SUBAGENTS, "Explore:sub-1");
        store.insert(PANE, PANE_STARTED_AT, "1700");
        store.insert(PANE, PANE_ATTENTION, "notification");

        assert_eq!(
            store.snapshot(PANE),
            PaneSnapshot {
                status: Some("waiting".into()),
                bg_cmd: Some("npm run dev".into()),
                subagents: Some("Explore:sub-1".into()),
                started_at: Some("1700".into()),
            }
        );
    }

    #[test]
    fn mem_store_snapshot_reads_unset_options_as_none() {
        let store = MemPaneStore::new();
        store.insert(PANE, PANE_STATUS, "");
        assert_eq!(store.snapshot(PANE), PaneSnapshot::default());
    }
}
