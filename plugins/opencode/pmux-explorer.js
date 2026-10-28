// pmux-explorer bridge for OpenCode 2.
//
// OpenCode 2 loads local plugin files from `~/.config/opencode/plugins/` and
// the project's `.opencode/plugin|plugins/` directories. The v2 plugin API is
// a default export of `{ id, setup(context) }`: hooks register through the
// context domains (`event`, `tool`, `session`) and the loader never calls a
// `server` function — that was the OpenCode 1 module shape, and a v1-shaped
// bridge silently registers nothing on OpenCode 2.
//
// The canonical install is a single symlink of this file into
// `~/.config/opencode/plugins/`. Do not also keep a copy under a scanned
// `.opencode/plugins/` directory: OpenCode loads every source, keeps the
// first copy per plugin id and marks later duplicates as failed, which is
// what raises the `Plugin failed` toast in the TUI.

import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const resolveHookScript = () => {
  let dir = dirname(fileURLToPath(import.meta.url));
  for (let i = 0; i < 4; i += 1) {
    const candidate = resolve(dir, "hook.sh");
    if (existsSync(candidate)) {
      return candidate;
    }
    const parent = dirname(dir);
    if (parent === dir) {
      break;
    }
    dir = parent;
  }
  return null;
};

const HOOK_COMMAND = (() => {
  const hookScript = resolveHookScript();
  return hookScript
    ? { cmd: "bash", prefix: [hookScript, "opencode"] }
    : { cmd: "pmux-explorer", prefix: ["hook", "opencode"] };
})();

// Fire-and-forget: the sidebar's hook CLI only reads one JSON object from
// stdin, and serializing subprocess exits would add latency without
// backpressure.
const hook = (eventName, payload) => {
  try {
    const child = spawn(HOOK_COMMAND.cmd, [...HOOK_COMMAND.prefix, eventName], {
      stdio: ["pipe", "ignore", "ignore"],
    });
    child.on("error", () => {});
    child.stdin.on("error", () => {});
    child.stdin.end(JSON.stringify(payload));
  } catch {
    // OpenCode should keep running even if the bridge is missing or
    // the sidebar binary is unavailable.
  }
};

const errorMessage = (err) => {
  if (!err) return "";
  if (typeof err === "string") return err;
  if (typeof err === "object") {
    if (typeof err.message === "string" && err.message) return err.message;
    if (typeof err.type === "string" && err.type) return err.type;
    return JSON.stringify(err);
  }
  return String(err);
};

// Per-turn state, keyed by session ID: the newest user prompt and the latest
// reply text. v2 bus events carry both directly (`session.inbox.enqueued`
// payload text, `session.text.ended` text), so no session read-back is
// needed; the state only glues the two together per turn.
const turnState = new Map();

const turnFor = (sessionID) => {
  let state = turnState.get(sessionID);
  if (!state) {
    state = { warming: false, lastText: "" };
    turnState.set(sessionID, state);
  }
  return state;
};

const endTurn = (sessionID) => {
  turnState.delete(sessionID);
};

// OpenCode's session warming reuses the user's session to send a keep-alive
// prompt on a timer. Those turns are not user activity: they would flip the
// pane to running and overwrite the prompt text with the keep-alive string.
const WARMING_PROMPT =
  "This is a keep-alive request. Do not perform any work or use tools. Reply with exactly: OK";

export const PmuxExplorer = async (context) => {
  const cwd =
    typeof context?.location?.directory === "string"
      ? context.location.directory
      : "";

  const controller = new AbortController();
  const registrations = [];

  const onEvent = async (event) => {
    if (!event || typeof event.type !== "string") return;
    const data = event.data ?? {};
    const sessionID = typeof data.sessionID === "string" ? data.sessionID : "";

    switch (event.type) {
      case "session.created":
        hook("session-start", { cwd, session_id: sessionID, source: "startup" });
        return;

      case "session.inbox.enqueued": {
        // A user prompt entering the queue. The payload text is the prompt;
        // steered messages enqueue the same way, which matches the sidebar's
        // "prompt overwritten by the newest turn" semantics.
        if (data.item?.type !== "user") return;
        const prompt = typeof data.item.payload?.text === "string" ? data.item.payload.text : "";
        const turn = turnFor(sessionID);
        turn.lastText = "";
        if (prompt === WARMING_PROMPT) {
          turn.warming = true;
          return;
        }
        turn.warming = false;
        hook("user-prompt-submit", { cwd, session_id: sessionID, prompt });
        return;
      }

      case "session.text.ended": {
        // Final text of one assistant message; later steps overwrite.
        const turn = turnFor(sessionID);
        if (typeof data.text === "string") turn.lastText = data.text;
        return;
      }

      case "session.execution.succeeded":
      case "session.execution.interrupted": {
        const turn = turnFor(sessionID);
        const warming = turn.warming;
        const lastMessage = turn.lastText;
        endTurn(sessionID);
        if (warming) return;
        hook("stop", { cwd, session_id: sessionID, last_message: lastMessage });
        return;
      }

      case "session.execution.failed": {
        endTurn(sessionID);
        hook("stop-failure", {
          cwd,
          session_id: sessionID,
          error: errorMessage(data.error) || "session.execution.failed",
        });
        return;
      }

      case "permission.asked":
        hook("notification", { cwd, session_id: sessionID, wait_reason: "permission" });
    }
  };

  const onToolAfter = (input) => {
    hook("activity-log", {
      cwd,
      session_id: typeof input?.sessionID === "string" ? input.sessionID : "",
      tool_name: typeof input?.tool === "string" ? input.tool : "",
      tool_input: input?.input ?? {},
      tool_response:
        input?.status === "completed"
          ? {
              output: input.result?.output ?? "",
              content: input.result?.content ?? null,
              metadata: input.result?.metadata ?? null,
            }
          : { error: errorMessage(input?.error) || "failed" },
    });
  };

  const loop = (async () => {
    for await (const event of context.event.subscribe({
      signal: controller.signal,
    })) {
      await onEvent(event);
    }
  })().catch(() => {});

  try {
    registrations.push(await context.tool.hook("execute.after", onToolAfter));
  } catch {
    // The sidebar loses tool activity but keeps status events.
  }

  void loop;

  return async () => {
    controller.abort();
    turnState.clear();
    await Promise.allSettled(
      registrations.map((registration) => registration?.dispose?.()),
    );
  };
};

export default {
  id: "pmux-explorer",
  async setup(context) {
    return PmuxExplorer(context);
  },
};
