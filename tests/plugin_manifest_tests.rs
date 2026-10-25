//! The plugin wiring for the MCP channel server: the inline
//! `mcpServers.pmux` entry in `.claude-plugin/plugin.json` and the
//! `mcp.sh` wrapper beside `hook.sh`.
//!
//! The manifest half is a drift test: when the entry is renamed, moved
//! into a root `.mcp.json`, or given a `channels` key, the plugin stops
//! loading as a channel and this test fails. The wrapper half pins the
//! two behaviors the manifest cannot: the binary it execs, and the loud
//! exit 1 when no binary is anywhere (D6 — a silent exit 0 would look
//! like a server that closed).

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const PLUGIN_JSON_PATH: &str = ".claude-plugin/plugin.json";
const MCP_SH_PATH: &str = "mcp.sh";
/// The binary lookup `mcp.sh` sources; copied beside every wrapper copy,
/// since the lookup resolves the binary relative to itself.
const LOOKUP_SH_PATH: &str = "bin-lookup.sh";

/// A host tool located on the test runner's own `PATH`, to symlink into
/// a stripped-down `PATH` for the wrapper children: the script needs
/// only `bash` (its shebang resolves through `env` searching `PATH`)
/// and `dirname`; everything else it runs is a bash builtin.
fn host_tool(name: &str) -> PathBuf {
    let path = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!("host tool {name} not found on the test runner's PATH");
}

/// A `PATH` without the binary: a temp directory holding symlinks to
/// just the host tools the wrapper needs, so `command -v pmux-explorer`
/// fails deterministically on any machine.
fn stripped_path(dir: &Path) -> PathBuf {
    let bin = dir.join("bin");
    fs::create_dir(&bin).unwrap();
    for tool in ["bash", "dirname"] {
        std::os::unix::fs::symlink(host_tool(tool), bin.join(tool))
            .unwrap_or_else(|e| panic!("symlink {tool} into the stripped PATH: {e}"));
    }
    bin
}

/// Copies the checkout's `mcp.sh` and its sourced `bin-lookup.sh` under
/// `dir`, with `mcp.sh` made executable.
fn wrapper_copy(dir: &Path) -> PathBuf {
    let raw = fs::read_to_string(MCP_SH_PATH)
        .unwrap_or_else(|e| panic!("failed to read {MCP_SH_PATH}: {e}"));
    let lookup = fs::read_to_string(LOOKUP_SH_PATH)
        .unwrap_or_else(|e| panic!("failed to read {LOOKUP_SH_PATH}: {e}"));
    let script = dir.join(MCP_SH_PATH);
    fs::write(&script, raw).unwrap();
    fs::write(dir.join(LOOKUP_SH_PATH), lookup).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    script
}

#[test]
fn the_manifest_declares_the_pmux_server_inline() {
    let raw = fs::read_to_string(PLUGIN_JSON_PATH)
        .unwrap_or_else(|e| panic!("failed to read {PLUGIN_JSON_PATH}: {e}"));
    let manifest: serde_json::Value = serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("{PLUGIN_JSON_PATH} is not valid JSON: {e}"));

    let command = manifest
        .get("mcpServers")
        .and_then(|v| v.get("pmux"))
        .and_then(|v| v.get("command"))
        .and_then(|v| v.as_str())
        .expect("the manifest must declare mcpServers.pmux.command");
    assert_eq!(command, "${CLAUDE_PLUGIN_ROOT}/mcp.sh");

    // The channel server needs no configuration dialog, so the manifest
    // carries no `channels` entry (D6).
    assert!(
        manifest.get("channels").is_none(),
        "the manifest must not declare a channels key"
    );
}

#[test]
fn the_wrapper_execs_the_binary_it_finds_with_mcp() {
    let tmp = tempfile::tempdir().unwrap();
    // The plugin dir with a binary in the second lookup location, as a
    // checkout build leaves it. The stub prints its arguments, so the
    // test sees exactly how the wrapper invoked the binary.
    let plugin = tmp.path().join("plugin");
    fs::create_dir_all(plugin.join("target/release")).unwrap();
    let stub = plugin.join("target/release/pmux-explorer");
    fs::write(&stub, "#!/bin/sh\nprintf '%s\\n' \"$@\"\n").unwrap();
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).unwrap();
    let script = wrapper_copy(&plugin);

    let output = Command::new(&script)
        .env("HOME", tmp.path()) // no TPM fallback anywhere under it
        .env("PATH", stripped_path(tmp.path()))
        .output()
        .expect("the wrapper script must run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim_end(), "--mcp");
}

#[test]
fn the_wrapper_exits_1_when_no_binary_is_found() {
    let tmp = tempfile::tempdir().unwrap();
    // An empty plugin dir (no bin/, no target/), an empty HOME (no TPM
    // fallback), and a PATH without the binary: every lookup fails.
    let plugin = tmp.path().join("plugin");
    fs::create_dir(&plugin).unwrap();
    let script = wrapper_copy(&plugin);
    let home = tmp.path().join("home");
    fs::create_dir(&home).unwrap();

    let output = Command::new(&script)
        .env("HOME", &home)
        .env("PATH", stripped_path(tmp.path()))
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("the wrapper script must run");

    assert_eq!(output.status.code(), Some(1), "exit code");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("pmux-explorer"),
        "the message must name the lookup: {stderr:?}"
    );
}
