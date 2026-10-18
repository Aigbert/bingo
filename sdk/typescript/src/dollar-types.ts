// Generated from JSON Schema. Do not edit.
import type { Callback } from './callbacks.ts';
import type { AtSite, CommandRegistration, ProcessHandle } from './dollar.ts';
import type { ElementTable, OpenPane } from './ui.ts';
import type * as Wire from './wire.ts';
/** A typed action a client asks for (GUI buttons, hosts); the kernel dispatches it to a command. */
export type Action = { "args"?: unknown; "name": string; [key: string]: unknown; };
/** A button: the surface fires `Input::Action` with `action` (ADR-0008 §1). */
export type ActionItem = {
  "action": Action;
  /** A single-key hint; the surface may use another key. */
  "key"?: (string | null);
  "label": string;
  [key: string]: unknown;
};
export type Align = ("flex-start" | "center" | "flex-end" | "stretch");
export type Ancestors = {
  /** One file or directory name, without a separator. */
  "name": string;
  "session": SessionId;
  [key: string]: unknown;
};
export type Answer = ({ "input": unknown; "kind": "allowChanged"; [key: string]: unknown; } | { "kind": "allowOnce"; [key: string]: unknown; } | { "kind": "allowSession"; "scope": string; [key: string]: unknown; } | { "feedback"?: (string | null); "kind": "deny"; [key: string]: unknown; } | { "ids": Array<string>; "kind": "choice"; "other"?: (string | null); [key: string]: unknown; } | { "kind": "text"; "text": string; [key: string]: unknown; } | { "kind": "confirm"; [key: string]: unknown; } | { "kind": "cancel"; [key: string]: unknown; } | { "answers": Array<Answer>; "kind": "form"; [key: string]: unknown; });
/**
 * The two options a session may answer a question with when nobody is asked
 * (ADR-0039 §2). The asker names them in its own words; the kernel reads
 * which is which and never the words.
 */
export type AnswerRole = ("allowing" | "refusing");
export type AnswerSpec = ("allowChanged" | "allowOnce" | "allowSession" | "deny" | "choice" | "text" | "confirm" | "cancel" | "form");
export type ArgSpec = ({ "kind": "none"; [key: string]: unknown; } | { "args": ArgSpec; "flags": Array<string>; "kind": "options"; [key: string]: unknown; } | { "hint": string; "kind": "free"; [key: string]: unknown; } | { "kind": "catalog"; "source": string; [key: string]: unknown; } | {
  /** Words among `values` that end the argument; `then` does not apply. */
  "alone"?: Array<string>;
  "kind": "words";
  /**
   * What a surface shows for a value that is not its own best name;
   * the line still runs the value, and a value without one reads as
   * itself.
   */
  "labels"?: { [key: string]: string; };
  /**
   * Words among `values` only the person may run: they raise trust or
   * let repository code run, so no link rewrites a line that starts
   * with one and no view bingo did not draw may send one.
   */
  "person"?: Array<string>;
  "then"?: (ArgSpec | null);
  "values": Array<string>;
  [key: string]: unknown;
});
export type Authorize = {
  /** The answers the person may give. */
  "answers": Array<AnswerSpec>;
  /** What kind of question it is. */
  "kind": InteractionKind;
  "session": SessionId;
  [key: string]: unknown;
};
export type Catalog = { "entries": Array<CatalogEntry>; "kind": CatalogKind; [key: string]: unknown; };
export type CatalogEntry = { "id": string; "label": string; "meta"?: unknown; [key: string]: unknown; };
export type CatalogKind = ("events" | "models" | "providers" | "tools" | "commands" | "plugins");
/** Why a call needs care, and the act its one-time approval names. */
export type Caution = { "act"?: (string | null); "reason": string; [key: string]: unknown; };
/** Whether the chosen model applies only here or is the default for new sessions. */
export type ChoiceScope = ("session" | "default");
export type Classify = {
  /** The labels the model chooses from. */
  "labels": Array<string>;
  "session": SessionId;
  /** The text to classify. */
  "text": string;
  "turn"?: (TurnId | null);
  [key: string]: unknown;
};
export type ClientIdentity = { "name": string; "surface": string; [key: string]: unknown; };
export type CommandSpec = {
  "aliases"?: Array<string>;
  "args": ArgSpec;
  "family": string;
  "hidden"?: boolean;
  /** A one-line description for the command menu; argument syntax belongs in `args`. */
  "hint": string;
  /** May run while a turn is busy (read-only commands). */
  "instant"?: boolean;
  "keys"?: Array<KeyBinding>;
  "name": string;
  /**
   * Run bare, it opens the place where its argument is chosen, so a
   * palette runs it rather than listing `args`; `args` stays for every
   * surface that completes it.
   */
  "opens"?: boolean;
  [key: string]: unknown;
};
export type Complete = { "request": ModelRequest; "session": SessionId; "turn"?: (TurnId | null); [key: string]: unknown; };
export type Completed = { "events": Array<ModelEvent>; [key: string]: unknown; };
export type Component = ("KeyBar" | "Composer" | "ModelPicker" | "Browse" | "Plugins" | "Palette" | "Sessions" | "Status" | "Question" | "UserMessage" | "AssistantMessage" | "ToolUse" | "ToolResult" | "ToolGroup" | "ToolProgress" | "CommandOutput" | "Spinner" | "TurnDuration" | "InfoNotice" | "SessionMode" | "PromptHint" | "AbovePrompt" | "Pane" | "Permission" | "SignIn" | "Reasoning" | "Compaction" | "Rewind" | "Interruption" | "QuestionAnswer" | "PermissionReceipt" | "Asset" | "ShellRun" | "Action" | "TurnProgress" | "SessionStatus" | "Toast" | "Welcome" | "Ledger" | "TurnFold");
export type ComposerProps = { "placeholder": string; "tag": string; [key: string]: unknown; };
/** Configuration as a client sees it: the kernel's keys and each plugin's claimed slice. */
export type ConfigView = { "kernel"?: unknown; "plugins"?: { [key: string]: unknown; }; [key: string]: unknown; };
export type ContentPart = ({ "text": string; "type": "text"; [key: string]: unknown; } | Image | { "id": string; "input": unknown; "name": string; "type": "toolUse"; [key: string]: unknown; } | { "isError"?: boolean; "parts": Array<ContentPart>; "toolUseId": string; "type": "toolResult"; [key: string]: unknown; } | { "providerMetadata"?: { [key: string]: { [key: string]: unknown; }; }; "text": string; "type": "reasoning"; [key: string]: unknown; });
export type ContextKind = ("system" | "tools" | "chat" | "pictures");
export type ContextPart = { "name": ContextKind; "tokens": number; [key: string]: unknown; };
/**
 * The one ruler for context: used tokens, the model window, and the point at
 * which compaction triggers.
 */
export type ContextUsage = {
  /** Approximate local request composition, independent of server anchoring. */
  "parts"?: (Array<ContextPart> | null);
  /**
   * `used` at which the older turns are summarised, equal to `window`
   * where the kernel draws no line because the endpoint holds the context
   * and cuts it itself (ADR-0055 §2).
   */
  "trigger": number;
  /** Tokens the next request would carry, anchored on the server's last count. */
  "used": number;
  /**
   * The input side of the model's window: what is left once the output
   * budget is reserved (ADR-0006).
   */
  "window": number;
  [key: string]: unknown;
};
export type DecisionKind = ("allowChanged" | "allowChangedSession" | "allow" | "allowSession" | "deny");
/**
 * How a line reaches a session's queue: a peer's message (ADR-0010 §1) and
 * a person's own line are the same question, so they are the same word.
 */
export type Delivery = ("wake" | "hold");
export type Direction = ("row" | "column");
export type DollarData = { "plugin": PluginData; [key: string]: unknown; };
export type DollarInputs = {
  /** The sub-agents of a session, through the agent service. */
  "agent.list": Session;
  /** Register an agent `{definition}`, through the agent service. */
  "agent.register": unknown;
  /** Start a sub-agent with `{args: {prompt, agent?}}`, through the agent service; answers with its result. */
  "agent.spawn": unknown;
  /** Run a callback once after `ms` milliseconds; answers its timer id. */
  "clock.after": Timer;
  /** Run a callback every `ms` milliseconds; answers its timer id. */
  "clock.every": Timer;
  /** The host's time in milliseconds since the epoch; tests can set it. */
  "clock.now": null;
  /** Wait `ms` milliseconds on the host clock. */
  "clock.sleep": Millis;
  /** Every slash command the session offers. */
  "command.list": null;
  /** Register a slash command; the author call takes the spec and an optional handler. */
  "command.register": RegisterCommand;
  /** Run a slash command by name; runs `command.run`. */
  "command.run": Run;
  /** Every setting as the session sees it. */
  "config.list": Session;
  /** Read one setting by its dotted key. */
  "config.read": Read;
  /** Change one setting; runs `config.set`. */
  "config.set": Setting;
  /** Read a variable from the mod's environment overlay, or `null`. */
  "env.get": EnvGet;
  /** Set a variable in the mod's environment overlay; processes it starts see it. */
  "env.set": EnvSet;
  /** Each directory from the session's cwd up to the project root that holds `name`, nearest first. */
  "fs.ancestors": Ancestors;
  /** Whether a path exists under a granted root. */
  "fs.exists": FilePath;
  /** List the entries of a directory under a granted root, up to `limit`. */
  "fs.list": FileList;
  /** Read a file under a granted root, as UTF-8 text or base64. */
  "fs.read": FileRead;
  /** Size and kind of a path under a granted root. */
  "fs.stat": FilePath;
  /** Write a file under a granted root, replacing it. */
  "fs.write": FileWrite;
  /** Fetch a URL through the host; the body is text and may be `truncated`. */
  "http.fetch": Fetch;
  /** Call `tool` on MCP `server` with `args`, through the MCP service. */
  "mcp.call": unknown;
  /** Connect MCP `server`, through the MCP service. */
  "mcp.connect": unknown;
  /** Ask the model to pick one of `labels` for `text`, or `null`. */
  "model.classify": Classify;
  /** Ask the host's model for a completion; runs `model.complete`. */
  "model.complete": Complete;
  /** Ask the model a one-off `prompt` with the session's context, outside the history. */
  "model.fork": Fork;
  /** Kill a spawned process. */
  "process.kill": ProcessId;
  /** Wait for a spawned process to exit and read all its output; a process is read once. */
  "process.read": ProcessId;
  /** Run `argv` to completion and answer with its exit code and output. */
  "process.run": ProcessInput;
  /** Start `argv` in the background; the author call answers with a handle to read, write or kill. */
  "process.spawn": ProcessInput;
  /** Write `body` to a spawned process's stdin; `null` closes it. */
  "process.write": ProcessWrite;
  /** The session's queued inputs and the last submitted line. */
  "prompt.read": Session;
  /** Submit input to the session as if typed; runs `prompt.submit`. */
  "prompt.submit": Submit;
  /** Append a record this plugin owns to the session's history; runs `session.append`. */
  "session.append": Extend;
  /** Ask the person a question with the given answer kinds and wait for the answer. */
  "session.authorize": Authorize;
  /** Ask the session to compact its history; runs `session.compact`. */
  "session.compact": Session;
  /** The session's working directory. */
  "session.cwd": Session;
  /** The current session's id. */
  "session.id": Session;
  /** A page of the session's history. */
  "session.messages": Messages;
  /** The model the session is using, or `null`. */
  "session.model": Session;
  /** The session's project root. */
  "session.root": Session;
  /** Send input to another session; runs `session.send`. */
  "session.send": Send;
  /** The clients attached to the session. */
  "session.surfaces": Session;
  /** The ids of the session's turns, oldest first. */
  "session.turns": Session;
  /** Token usage: the whole session, the last turn, and `context`, the current context window reading. */
  "session.usage": Session;
  /** The bingo version running the mod. */
  "session.version": null;
  /** Read this plugin's value for `key` in the current session, with its version, or `null`. */
  "state.get": StateGet;
  /** Write this plugin's value for `key` in the current session; `ifVersion` refuses a stale write. */
  "state.set": StateSet;
  /** Delete this plugin's global value for `key`; answers whether it existed. */
  "store.delete": StateGet;
  /** Read this plugin's global value for `key`, shared across sessions, or `null`. */
  "store.get": StateGet;
  /** Every key this plugin has in the global store. */
  "store.keys": null;
  /** Write this plugin's global value for `key`; `ifVersion` refuses a stale write. */
  "store.set": StateSet;
  /** Call a tool as the model would; runs `tool.check` and `tool.call`, so permission still applies. */
  "tool.call": Invoke;
  /** Every tool the session offers the model. */
  "tool.list": null;
  /** Register a tool the model can call; the author call takes the spec and a handler. */
  "tool.register": RegisterTool;
  /** End the session's running turn, as `esc` does. */
  "turn.abort": Session;
};
export type DollarResults = {
  /** The sub-agents of a session, through the agent service. */
  "agent.list": Array<SessionSummary>;
  /** Register an agent `{definition}`, through the agent service. */
  "agent.register": null;
  /** Start a sub-agent with `{args: {prompt, agent?}}`, through the agent service; answers with its result. */
  "agent.spawn": ToolOutput;
  /** Run a callback once after `ms` milliseconds; answers its timer id. */
  "clock.after": string;
  /** Run a callback every `ms` milliseconds; answers its timer id. */
  "clock.every": string;
  /** The host's time in milliseconds since the epoch; tests can set it. */
  "clock.now": number;
  /** Wait `ms` milliseconds on the host clock. */
  "clock.sleep": null;
  /** Every slash command the session offers. */
  "command.list": Catalog;
  /** Register a slash command; the author call takes the spec and an optional handler. */
  "command.register": string;
  /** Run a slash command by name; runs `command.run`. */
  "command.run": IntentId;
  /** Every setting as the session sees it. */
  "config.list": ConfigView;
  /** Read one setting by its dotted key. */
  "config.read": unknown;
  /** Change one setting; runs `config.set`. */
  "config.set": null;
  /** Read a variable from the mod's environment overlay, or `null`. */
  "env.get": (string | null);
  /** Set a variable in the mod's environment overlay; processes it starts see it. */
  "env.set": null;
  /** Each directory from the session's cwd up to the project root that holds `name`, nearest first. */
  "fs.ancestors": Array<string>;
  /** Whether a path exists under a granted root. */
  "fs.exists": boolean;
  /** List the entries of a directory under a granted root, up to `limit`. */
  "fs.list": Array<string>;
  /** Read a file under a granted root, as UTF-8 text or base64. */
  "fs.read": string;
  /** Size and kind of a path under a granted root. */
  "fs.stat": FileStat;
  /** Write a file under a granted root, replacing it. */
  "fs.write": null;
  /** Fetch a URL through the host; the body is text and may be `truncated`. */
  "http.fetch": Fetched;
  /** Call `tool` on MCP `server` with `args`, through the MCP service. */
  "mcp.call": ToolOutput;
  /** Connect MCP `server`, through the MCP service. */
  "mcp.connect": null;
  /** Ask the model to pick one of `labels` for `text`, or `null`. */
  "model.classify": (string | null);
  /** Ask the host's model for a completion; runs `model.complete`. */
  "model.complete": Completed;
  /** Ask the model a one-off `prompt` with the session's context, outside the history. */
  "model.fork": Completed;
  /** Kill a spawned process. */
  "process.kill": null;
  /** Wait for a spawned process to exit and read all its output; a process is read once. */
  "process.read": ProcessOutput;
  /** Run `argv` to completion and answer with its exit code and output. */
  "process.run": ProcessOutput;
  /** Start `argv` in the background; the author call answers with a handle to read, write or kill. */
  "process.spawn": string;
  /** Write `body` to a spawned process's stdin; `null` closes it. */
  "process.write": null;
  /** The session's queued inputs and the last submitted line. */
  "prompt.read": PromptView;
  /** Submit input to the session as if typed; runs `prompt.submit`. */
  "prompt.submit": IntentId;
  /** Append a record this plugin owns to the session's history; runs `session.append`. */
  "session.append": null;
  /** Ask the person a question with the given answer kinds and wait for the answer. */
  "session.authorize": Answer;
  /** Ask the session to compact its history; runs `session.compact`. */
  "session.compact": IntentId;
  /** The session's working directory. */
  "session.cwd": string;
  /** The current session's id. */
  "session.id": SessionId;
  /** A page of the session's history. */
  "session.messages": HistoryChunk;
  /** The model the session is using, or `null`. */
  "session.model": (string | null);
  /** The session's project root. */
  "session.root": string;
  /** Send input to another session; runs `session.send`. */
  "session.send": null;
  /** The clients attached to the session. */
  "session.surfaces": Array<ClientIdentity>;
  /** The ids of the session's turns, oldest first. */
  "session.turns": Array<TurnId>;
  /** Token usage: the whole session, the last turn, and `context`, the current context window reading. */
  "session.usage": UsageView;
  /** The bingo version running the mod. */
  "session.version": string;
  /** Read this plugin's value for `key` in the current session, with its version, or `null`. */
  "state.get": (StateValue | null);
  /** Write this plugin's value for `key` in the current session; `ifVersion` refuses a stale write. */
  "state.set": StateValue;
  /** Delete this plugin's global value for `key`; answers whether it existed. */
  "store.delete": boolean;
  /** Read this plugin's global value for `key`, shared across sessions, or `null`. */
  "store.get": (StateValue | null);
  /** Every key this plugin has in the global store. */
  "store.keys": Array<string>;
  /** Write this plugin's global value for `key`; `ifVersion` refuses a stale write. */
  "store.set": StateValue;
  /** Call a tool as the model would; runs `tool.check` and `tool.call`, so permission still applies. */
  "tool.call": ToolOutput;
  /** Every tool the session offers the model. */
  "tool.list": Catalog;
  /** Register a tool the model can call; the author call takes the spec and a handler. */
  "tool.register": string;
  /** End the session's running turn, as `esc` does. */
  "turn.abort": null;
};
/** What a session does with what it is told (ADR-0011 §1). */
export type Driver = ("model" | "log");
/** Reasoning effort, in the vocabulary providers converge on. */
export type Effort = ("minimal" | "low" | "medium" | "high" | "xHigh" | "max");
export type Encoding = ("utf8" | "base64");
export type EnvGet = {
  /** The variable's name. */
  "name": string;
  "session": SessionId;
  [key: string]: unknown;
};
export type EnvSet = {
  /** The variable's name. */
  "name": string;
  "session": SessionId;
  /** Its value. */
  "value": string;
  [key: string]: unknown;
};
export type Extend = { "key": string; "session": SessionId; "value": unknown; [key: string]: unknown; };
export type Fetch = {
  /** The request body, as text. */
  "body"?: (string | null);
  /** Request headers. */
  "headers"?: ({ [key: string]: string; } | null);
  /** `GET` unless given. */
  "method"?: (string | null);
  "session": SessionId;
  /** The URL to fetch. */
  "url": string;
  [key: string]: unknown;
};
export type Fetched = {
  /** The body, as text. */
  "body": string;
  /** Response headers. */
  "headers": { [key: string]: string; };
  /** A note from the host about the fetch, if any. */
  "notice"?: (string | null);
  /** The HTTP status. */
  "status": number;
  /** Whether the body was cut at the host's limit. */
  "truncated": boolean;
  [key: string]: unknown;
};
export type FileList = {
  /** At most this many entries. */
  "limit"?: (number | null);
  /** The directory to list. */
  "path": string;
  "session": SessionId;
  [key: string]: unknown;
};
export type FilePath = {
  /** Relative to the session's cwd, or absolute under a granted root. */
  "path": string;
  "session": SessionId;
  [key: string]: unknown;
};
export type FileRead = {
  /** `utf8` (the default) or `base64`. */
  "encoding"?: (Encoding | null);
  /** Relative to the session's cwd, or absolute under a granted root. */
  "path": string;
  "session": SessionId;
  [key: string]: unknown;
};
export type FileStat = {
  /** Whether it is a directory. */
  "directory": boolean;
  /** Whether it is a regular file. */
  "file": boolean;
  /** Size in bytes. */
  "size": number;
  [key: string]: unknown;
};
export type FileWrite = {
  /** The new contents. */
  "body": string;
  /** How `body` is encoded: `utf8` (the default) or `base64`. */
  "encoding"?: (Encoding | null);
  /** Relative to the session's cwd, or absolute under a granted root. */
  "path": string;
  "session": SessionId;
  [key: string]: unknown;
};
export type FinishReason = { "raw"?: (string | null); "unified": UnifiedFinish; [key: string]: unknown; };
export type Fork = {
  /** The one-off question. */
  "prompt": string;
  "session": SessionId;
  "turn"?: (TurnId | null);
  [key: string]: unknown;
};
export type ForkLink = { "session": SessionId; "turn"?: (TurnId | null); [key: string]: unknown; };
export type HistoryChunk = {
  "generation": number;
  "items": Array<Item>;
  "next"?: (ItemId | null);
  /**
   * This item alone cannot fit the requested page. Fetch it explicitly in
   * parts before following `next`, which excludes that item.
   */
  "oversized"?: (OversizedItem | null);
  [key: string]: unknown;
};
export type HistoryPage = {
  "before"?: (ItemId | null);
  /** Reject a stale page after a rewind/compaction rather than mixing views. */
  "generation"?: (number | null);
  "limit"?: number;
  /**
   * Optional byte budget for the whole JSON-RPC response; absent preserves
   * `limit == 0` meaning all, not an implicit bounded page.
   */
  "maxBytes"?: (number | null);
  [key: string]: unknown;
};
/**
 * A picture handed to the model, already the shape the journal keeps and a
 * provider's request encodes: one struct, so a surface that reads a picture
 * off disk, a wire client that already holds the bytes, and a tool result
 * that returns one all produce the same thing.
 */
export type Image = {
  /** Base64 payload. */
  "data": string;
  "height"?: (number | null);
  "mediaType": string;
  /**
   * Where the same bytes are on this machine, when they are anywhere: a
   * picture read off a disk, or one a surface wrote before handing it on
   * (ADR-0052). A location, not a second copy — absent from the wire and
   * the journal when there is none, so a picture without one is the
   * bytes it always was.
   */
  "path"?: (string | null);
  "width"?: (number | null);
  [key: string]: unknown;
};
/**
 * The one submission entry. The kernel parses `/`, `!` and `@` in text and
 * decides turn, queue, steer or deliver; a client never chooses.
 */
export type Input = ({
  /**
   * Whether a running turn may be steered with it (ADR-0008 §2,
   * amended M68). `Wake` is the line every client has always sent, so
   * it is the default and nothing is written for it.
   */
  "delivery"?: Delivery;
  /**
   * The pictures the ask carries, in the order they reach the journal
   * (ADR-0040): exactly as they will be journaled, because a surface
   * resolves a picture and the kernel does no file I/O for input.
   */
  "images"?: Array<Image>;
  "kind": "text";
  "origin": Origin;
  "text": string;
  [key: string]: unknown;
} | {
  "action": Action;
  "kind": "action";
  /**
   * The door it came through. Absent, or a door that is not a person
   * surface, and the action is not the person's own: it fails closed.
   */
  "origin"?: (Origin | null);
  [key: string]: unknown;
});
/** A client-minted write id; the outcome arrives as `IntentAck`. */
export type IntentId = string;
/** A pending question or permission, answered once by id. */
export type InteractionId = string;
export type InteractionKind = ({
  "caution"?: (Caution | null);
  /** The call after hooks and interceptors, before this decision. */
  "input"?: unknown;
  "kind": "permission";
  "preview"?: (Preview | null);
  /** The rule `AllowSession` would install, when one would silence the prompt. */
  "sessionScope"?: (string | null);
  "summary": string;
  "tool": string;
  [key: string]: unknown;
} | Question | {
  "kind": "form";
  "questions": Array<Question>;
  /**
   * What the whole set is about, in the asker's own words: an MCP
   * server's elicitation says which server is asking. A set the model
   * opened has nothing to add to its own questions and names none.
   */
  "title"?: (string | null);
  [key: string]: unknown;
} | { "detail": string; "kind": "confirm"; "title": string; [key: string]: unknown; } | { "flow": LoginFlow; "kind": "login"; "provider": string; [key: string]: unknown; });
export type Invoke = {
  /** The tool and its arguments. */
  "call": ToolCall;
  "session": SessionId;
  [key: string]: unknown;
};
export type Item = {
  "body": ItemBody;
  "completedAt"?: (string | null);
  "id": ItemId;
  /** The client intent that produced this item, when one did (user items). */
  "intent"?: (IntentId | null);
  "meta"?: { [key: string]: unknown; };
  "round"?: number;
  "startedAt": string;
  "status": ItemStatus;
  "turn"?: (TurnId | null);
  [key: string]: unknown;
};
export type ItemBody = ({ "kind": "user"; "origin": Origin; "parts": Array<ContentPart>; [key: string]: unknown; } | { "kind": "assistant"; "text": string; [key: string]: unknown; } | { "kind": "reasoning"; "providerMetadata"?: { [key: string]: { [key: string]: unknown; }; }; "text": string; [key: string]: unknown; } | { "callId": string; "durationMs"?: (number | null); "input": unknown; "kind": "toolCall"; "name": string; "output"?: (ToolOutput | null); "progress"?: (string | null); "quiet"?: boolean; [key: string]: unknown; } | { "command": string; "cwd": string; "exit"?: (number | null); "kind": "shell"; "output": string; [key: string]: unknown; } | { "args"?: unknown; "kind": "action"; "name": string; "result"?: unknown; [key: string]: unknown; } | { "after": number; "before": number; "durationMs": number; "kind": "compaction"; "replaced": number; "summary": string; [key: string]: unknown; } | { "dropped": number; "kind": "rewind"; "toTurn": TurnId; [key: string]: unknown; } | { "kind": "interruption"; "marker": string; [key: string]: unknown; } | { "code": string; "kind": "notice"; "level": Level; "text": string; [key: string]: unknown; } | { "answer": string; "interaction": InteractionId; "item"?: (ItemId | null); "kind": "questionAnswer"; "question": string; [key: string]: unknown; } | { "decision": DecisionKind; "feedback"?: (string | null); "interaction": InteractionId; "item"?: (ItemId | null); "kind": "permissionReceipt"; "tool": string; [key: string]: unknown; } | { "asset": string; "kind": "asset"; "label"?: (string | null); [key: string]: unknown; } | { [key: string]: unknown; });
/** One unit in the transcript order. */
export type ItemId = string;
export type ItemStatus = ("pending" | "running" | "completed" | "failed" | "interrupted");
export type Justify = ("flex-start" | "center" | "flex-end" | "space-between" | "space-around" | "space-evenly");
/**
 * A chord a command declares for itself. A bare chord runs the command
 * alone; one with `args` submits the command with them, so a shortcut for
 * `/model next` is a binding on `model` and not a second catalogue entry.
 */
export type KeyBinding = (string | { "args": string; "chord": string; [key: string]: unknown; });
export type KeyPair = { "action": string; "emphasis"?: boolean; "keys": string; [key: string]: unknown; };
export type Level = ("info" | "warn" | "error");
export type LoginFlow = ({ "kind": "browser"; "url": string; [key: string]: unknown; } | { "code": string; "kind": "device"; "url": string; [key: string]: unknown; } | { "kind": "paste"; [key: string]: unknown; });
export type Message = { "parts": Array<ContentPart>; "providerOptions"?: { [key: string]: { [key: string]: unknown; }; }; "role": Role; [key: string]: unknown; };
export type Messages = {
  /** Which page of history. */
  "page": HistoryPage;
  "session": SessionId;
  [key: string]: unknown;
};
export type Millis = {
  /** Milliseconds. */
  "ms": number;
  [key: string]: unknown;
};
/** Provider stream events. Never published; the accumulator folds them into items. */
export type ModelEvent = ({ "type": "streamStart"; "warnings"?: Array<string>; [key: string]: unknown; } | { "id"?: (string | null); "model"?: (string | null); "type": "responseMetadata"; [key: string]: unknown; } | { "id": string; "type": "textStart"; [key: string]: unknown; } | { "delta": string; "id": string; "type": "textDelta"; [key: string]: unknown; } | { "id": string; "type": "textEnd"; [key: string]: unknown; } | { "id": string; "type": "reasoningStart"; [key: string]: unknown; } | { "delta": string; "id": string; "type": "reasoningDelta"; [key: string]: unknown; } | { "id": string; "providerMetadata"?: { [key: string]: { [key: string]: unknown; }; }; "type": "reasoningEnd"; [key: string]: unknown; } | { "id": string; "name": string; "type": "toolInputStart"; [key: string]: unknown; } | { "delta": string; "id": string; "type": "toolInputDelta"; [key: string]: unknown; } | { "id": string; "type": "toolInputEnd"; [key: string]: unknown; } | { "id": string; "input": string; "name": string; "type": "toolCall"; [key: string]: unknown; } | { "type": "context"; "used": number; "window": number; [key: string]: unknown; } | { "after": number; "before": number; "type": "compacted"; [key: string]: unknown; } | { "finishReason": FinishReason; "type": "finish"; "usage": Usage; [key: string]: unknown; });
export type ModelRequest = {
  "maxTokens": number;
  "messages": Array<Message>;
  "model": string;
  "providerOptions"?: { [key: string]: { [key: string]: unknown; }; };
  "reasoning"?: (Effort | null);
  /**
   * Whose turn this is. A stateless provider ignores it; one that keeps a
   * conversation of its own per session — an ACP adapter holding one agent
   * session per bingo session (ADR-0035 §3), a stateful wire like the
   * Responses API — has no other way to know which conversation it is
   * answering. `None` is a request built by hand or a side question, which
   * belongs to no session.
   */
  "session"?: (SessionId | null);
  "system": Array<SystemBlock>;
  "tools": Array<ToolSpec>;
  [key: string]: unknown;
};
/** Who submitted the input, as every surface records it. */
export type Origin = {
  "conversation"?: (string | null);
  "principal"?: (string | null);
  /** Drawing family of the caller, independent of its attribution identifier. */
  "renderSurface"?: (SurfaceName | null);
  "surface": string;
  "voice"?: (Voice | null);
  [key: string]: unknown;
};
/**
 * The actor's factual observation; only the RPC server can add a
 * connection-scoped availability token after pinning an immutable Item.
 */
export type OversizedItem = {
  /** FNV-1a 64 of the exact serialized Item UTF-8, lowercase hex. */
  "checksum": string;
  "id": ItemId;
  "totalBytes": number;
  [key: string]: unknown;
};
/**
 * Where a child hangs in the tree: its parent, and the tool call that
 * spawned it when one did (ADR-0011 §3).
 */
export type ParentLink = { "item"?: (ItemId | null); "session": SessionId; [key: string]: unknown; };
export type PluginData = { "name": string; "root": string; "version"?: (string | null); [key: string]: unknown; };
export type Preview = ({ "kind": "diff"; "unified": string; [key: string]: unknown; } | { "command": string; "cwd": string; "description"?: (string | null); "kind": "command"; "stakes"?: (Stakes | null); [key: string]: unknown; } | { "kind": "url"; "url": string; [key: string]: unknown; });
export type ProcessId = {
  /** The id `spawn` answered. */
  "id": string;
  "session": SessionId;
  [key: string]: unknown;
};
export type ProcessInput = {
  /** The program and its arguments; no shell is involved. */
  "argv": Array<string>;
  /** Working directory; the session's cwd unless given. */
  "cwd"?: (string | null);
  "session": SessionId;
  /** Kill the process after this many milliseconds. */
  "timeoutMs"?: (number | null);
  [key: string]: unknown;
};
export type ProcessOutput = {
  /** The exit code, or `null` when killed by a signal. */
  "code"?: (number | null);
  /** Everything written to stderr. */
  "stderr": string;
  /** Everything written to stdout. */
  "stdout": string;
  [key: string]: unknown;
};
export type ProcessWrite = {
  /** Text for stdin; `null` closes it. */
  "body"?: (string | null);
  /** The id `spawn` answered. */
  "id": string;
  "session": SessionId;
  [key: string]: unknown;
};
export type PromptView = {
  /** The last submitted line. */
  "last"?: (string | null);
  /** Inputs waiting for the next turn. */
  "queue": Array<QueueEntry>;
  [key: string]: unknown;
};
/**
 * One question put to a person: what is asked, what may be picked, and
 * whether words of one's own or more than one option count. A lone question
 * and one of a form are the same thing (M53).
 */
export type Question = {
  "freeText"?: boolean;
  /** A short tag a surface may show before the question (`Auth method`). */
  "header"?: (string | null);
  "multi"?: boolean;
  "options": Array<QuestionOption>;
  "question": string;
  [key: string]: unknown;
};
/** A question owner's editing projection; sealed labels remain the host's. */
export type QuestionFocus = { "checked": number; "field": number; "writing": boolean; [key: string]: unknown; };
export type QuestionOption = {
  "description"?: (string | null);
  "id": string;
  "label": string;
  /**
   * A few lines of monospace shown beside the option while the cursor is
   * on it — the layout or the snippet picking it would mean (M53).
   */
  "preview"?: (string | null);
  /**
   * What picking it would mean to a session that answers for the person
   * (ADR-0039 §2). Unmarked options are a person's alone.
   */
  "role"?: (AnswerRole | null);
  [key: string]: unknown;
};
export type QueueEntry = { "intent": IntentId; "origin": Origin; "position": number; "preview": string; "steerable": boolean; [key: string]: unknown; };
export type Read = {
  /** The setting's dotted key. */
  "key": string;
  "session": SessionId;
  [key: string]: unknown;
};
export type RegisterCommand = { "callback": string; "session": SessionId; "spec": CommandSpec; [key: string]: unknown; };
export type RegisterTool = { "callback": string; "session": SessionId; "spec": ToolSpec; [key: string]: unknown; };
export type Role = ("user" | "assistant");
export type Run = {
  /** Its arguments. */
  "args": unknown;
  /** The command's name, without the slash. */
  "name": string;
  "session": SessionId;
  [key: string]: unknown;
};
export type SelectOption = {
  /** The text shown. */
  "label": string;
  /** The value chosen. */
  "value": string;
  [key: string]: unknown;
};
export type Send = {
  /** Whether it wakes the receiver or waits for its next turn. */
  "delivery": Delivery;
  /** What is sent. */
  "input": Input;
  /** The receiving session. */
  "to": SessionId;
  [key: string]: unknown;
};
export type Session = {
  /** The session this concerns; `$` methods default it to the current one. */
  "session": SessionId;
  [key: string]: unknown;
};
/** One persisted context: one journal, one actor. */
export type SessionId = string;
export type SessionSummary = {
  /** An interaction is open; stamped only by the session actor. */
  "asking"?: boolean;
  /**
   * A turn is running. A summary travels without the turn it describes —
   * a host listing carries no `SessionState` — so the kernel stamps this
   * on every summary it publishes and nothing else writes it. A client
   * that holds the state derives the same fact with `SessionState::busy`.
   */
  "busy"?: boolean;
  "createdAt": string;
  "cwd": string;
  "driver"?: Driver;
  "effort"?: (Effort | null);
  "firstAsk"?: (string | null);
  "forkedFrom"?: (ForkLink | null);
  "id": SessionId;
  "key"?: (string | null);
  /**
   * How many messages have been said in this session, by
   * [`Event::completes_a_message`]. `None` is "never counted" — a summary
   * written before the field existed — and must be shown as nothing rather
   * than as a `0` it does not mean.
   */
  "messages"?: (number | null);
  "model"?: (string | null);
  "modelScope"?: (ChoiceScope | null);
  "parent"?: (ParentLink | null);
  "provider"?: (string | null);
  /**
   * What the session was opened with and a resume must give it back:
   * the prompt appended to the kernel's own, and the tool set it is held to.
   */
  "systemExtra"?: (string | null);
  "title"?: (string | null);
  "tools"?: (Array<string> | null);
  "updatedAt": string;
  "usage"?: Usage;
  [key: string]: unknown;
};
export type Setting = {
  /** The setting's dotted key. */
  "key": string;
  "session": SessionId;
  /** Its new value. */
  "value": unknown;
  [key: string]: unknown;
};
export type SiteHints = { "composer"?: (ComposerProps | null); "pairs"?: Array<KeyPair>; [key: string]: unknown; };
/** The local consequences a command preview can establish before it runs. */
export type Stakes = { "items": Array<string>; "line": string; [key: string]: unknown; };
export type StateGet = { "key": string; "scope": string; [key: string]: unknown; };
export type StateSet = { "ifVersion"?: (number | null); "key": string; "scope": string; "value": unknown; [key: string]: unknown; };
export type StateValue = { "value": unknown; "version": number; [key: string]: unknown; };
export type Submit = {
  /** What to submit, as a person would type it. */
  "input": Input;
  "session": SessionId;
  [key: string]: unknown;
};
export type SurfaceName = ("terminal" | "desktop" | "text" | "chat");
/** One cacheable segment of the system prompt. */
export type SystemBlock = { "cache"?: boolean; "text": string; [key: string]: unknown; };
/** A text child preserves the style of its nested Text node. */
export type TextChild = (string | View);
export type Timer = {
  "callback": string;
  /** Milliseconds. */
  "ms": number;
  "session": SessionId;
  [key: string]: unknown;
};
/** The one styling hook a plugin has; the surface owns the colour. */
export type Tone = (("neutral" | "good" | "bad") | "attention");
/** A call the model asked for, before or after the gate. */
export type ToolCall = { "callId": string; "input": unknown; "name": string; [key: string]: unknown; };
/**
 * What a tool returned: the parts the model sees, plus what a person sees
 * instead when the tool has something better than the text (ADR-0013 §2).
 */
export type ToolOutput = {
  "display"?: (View | null);
  "isError"?: boolean;
  "parts": Array<ContentPart>;
  /**
   * What the call came to in one line, for the step that shows it
   * (ADR-0095 §9). The model never reads it.
   */
  "upshot"?: (Upshot | null);
  [key: string]: unknown;
};
/** A tool as the model sees it. */
export type ToolSpec = {
  "description": string;
  "inputSchema": unknown;
  /**
   * What a catalogue may show beside the tool (an MCP tool's `server`);
   * never sent to a model.
   */
  "meta"?: { [key: string]: unknown; };
  "name": string;
  [key: string]: unknown;
};
export type TreeNode = { "badge"?: (string | null); "children"?: Array<TreeNode>; "label": string; "tone"?: Tone; [key: string]: unknown; };
/** One run of the turn loop, closed exactly once. */
export type TurnId = string;
export type UnifiedFinish = ("stop" | "length" | "toolCalls" | "contentFilter" | "error" | "other");
/** A call's result in one line: `+1 −1`, `3 passed`, `exit 1`. */
export type Upshot = ({ "added": number; "kind": "diff"; "removed": number; [key: string]: unknown; } | { "kind": "words"; "text": string; "tone": Tone; [key: string]: unknown; });
/**
 * Token counts as the provider reports them, the three input counts kept
 * apart: `input_tokens` is what the model read fresh, the cache counts are
 * what it read from or wrote to a cached prefix. The whole input side is
 * `input_total()`.
 */
export type Usage = { "cacheReadTokens"?: number; "cacheWriteTokens"?: number; "inputTokens": number; "outputTokens": number; "reasoningTokens"?: number; [key: string]: unknown; };
export type UsageView = {
  /** The current context window reading, once known. */
  "context"?: (ContextUsage | null);
  /** Tokens used by the last turn. */
  "lastTurn"?: (Usage | null);
  /** Tokens used by the whole session. */
  "session": Usage;
  [key: string]: unknown;
};
export type View = ({
  /** Draw the text bold. */
  "bold"?: (boolean | null);
  /** Strings and nested `Text` nodes, drawn in order. */
  "children": Array<TextChild>;
  /** A colour name or `#rrggbb`; the surface may adjust it to its theme. */
  "color"?: (string | null);
  /** Draw the text dimmed. */
  "dimColor"?: (boolean | null);
  /** Swap the foreground and background colours. */
  "inverse"?: (boolean | null);
  /** Draw the text in italics. */
  "italic"?: (boolean | null);
  "kind": "Text";
  /** Strike the text through. */
  "strikethrough"?: (boolean | null);
  /** Underline the text. */
  "underline"?: (boolean | null);
  /** `wrap` breaks long lines at the width; `truncate` cuts them. */
  "wrap"?: (string | null);
  [key: string]: unknown;
} | {
  /** Draw it dimmed. */
  "dimColor"?: boolean;
  "kind": "Markdown";
  /** The Markdown source. */
  "text": string;
  [key: string]: unknown;
} | {
  "kind": "Code";
  /** The language name for highlighting, such as `rust`. */
  "language"?: (string | null);
  /** The code itself. */
  "source": string;
  [key: string]: unknown;
} | {
  "kind": "diff";
  /** The diff in unified format. */
  "unified": string;
  [key: string]: unknown;
} | {
  /** One string per item. */
  "items": Array<string>;
  "kind": "list";
  [key: string]: unknown;
} | {
  /** The column headings. */
  "headers": Array<string>;
  "kind": "table";
  /** One array of cells per row, in column order. */
  "rows": Array<Array<string>>;
  [key: string]: unknown;
} | {
  "kind": "keyValue";
  /** `[label, value]` pairs, in order. */
  "rows": Array<[string, string, ...Array<unknown>]>;
  [key: string]: unknown;
} | {
  "kind": "progress";
  /** Text drawn beside the bar. */
  "label"?: (string | null);
  /** The whole; without it the surface shows a count. */
  "total"?: (number | null);
  /** How much is done. */
  "value": number;
  [key: string]: unknown;
} | {
  "kind": "badge";
  /** The label. */
  "text": string;
  /** `neutral`, `good`, `bad` or `attention`; the surface picks the colour. */
  "tone"?: Tone;
  [key: string]: unknown;
} | {
  "kind": "tree";
  /** The top-level nodes. */
  "nodes": Array<TreeNode>;
  [key: string]: unknown;
} | {
  /** Cross-axis alignment: `flex-start`, `center`, `flex-end` or `stretch`. */
  "alignItems"?: (Align | null);
  /** The border colour. */
  "borderColor"?: (string | null);
  /** Any value but `none` draws a border. */
  "borderStyle"?: (string | null);
  /** The views inside, in order. */
  "children": Array<View>;
  /** `none` hides the box and its controls. */
  "display"?: (string | null);
  /** `row` or `column` (the default). */
  "flexDirection"?: (Direction | null);
  /** How much of the spare room this box takes; `0` takes none. */
  "flexGrow"?: (number | null);
  /** `wrap` lets children flow onto another line. */
  "flexWrap"?: (Wrap | null);
  /** Cells between children. */
  "gap"?: (number | null);
  /** The height in rows. */
  "height"?: (number | null);
  /** The focused site declares chrome separately from its body. */
  "hints"?: (SiteHints | null);
  /** Main-axis distribution, such as `flex-start` or `space-between`. */
  "justifyContent"?: (Justify | null);
  /** A stable identity among siblings. */
  "key"?: (string | null);
  "kind": "Box";
  /** Cells outside the box on every side. */
  "margin"?: (number | null);
  /** Cells outside the box on the left and right. */
  "marginX"?: (number | null);
  /** Rows outside the box above and below. */
  "marginY"?: (number | null);
  /** A handler for a key pressed while this box's pane has focus; return `{handled: false}` to give the key back. */
  "onKey"?: (Action | null);
  /** Cells inside the box on every side. */
  "padding"?: (number | null);
  /** Cells inside the box on the left and right. */
  "paddingX"?: (number | null);
  /** Rows inside the box above and below. */
  "paddingY"?: (number | null);
  /**
   * A question owner's editing state, on the root of a drawn question;
   * the choices' labels and the answer stay the host's.
   */
  "question"?: (QuestionFocus | null);
  /** The width in cells. */
  "width"?: (number | null);
  [key: string]: unknown;
} | {
  /** The view inside. */
  "child": View;
  "kind": "panel";
  /** The heading. */
  "title": string;
  [key: string]: unknown;
} | {
  /** The buttons, in order. */
  "items": Array<ActionItem>;
  "kind": "actions";
  [key: string]: unknown;
} | {
  /** The button's text, when `label` is not given. */
  "children"?: Array<TextChild>;
  /** Draw the button dimmed. */
  "dimColor"?: (boolean | null);
  /** One letter: pressed with Alt in a band, or alone in a focused pane, it presses the button. */
  "hotkey"?: (string | null);
  /** A stable identity among siblings. */
  "key"?: (string | null);
  "kind": "Button";
  /** The button's text. */
  "label"?: (string | null);
  /** The handler run when the button is pressed: a mod passes a function. */
  "onPress"?: (Action | null);
  /** `primary` draws it bold, `danger` in the alarm colour. */
  "variant"?: (string | null);
  [key: string]: unknown;
} | {
  /** A stable identity among siblings. */
  "key"?: (string | null);
  "kind": "Input";
  /** Text before the field. */
  "label"?: (string | null);
  /** The handler run as the text changes: a mod passes a function. */
  "onInput"?: (Action | null);
  /** The handler run when the person presses Enter: a mod passes a function. */
  "onSubmit"?: (Action | null);
  /** Text shown while the field is empty. */
  "placeholder"?: (string | null);
  /** The text in the field. */
  "value"?: (string | null);
  [key: string]: unknown;
} | {
  /** A stable identity among siblings. */
  "key"?: (string | null);
  "kind": "Select";
  /** Text before the choice. */
  "label"?: (string | null);
  /** The handler run with the chosen value: a mod passes a function. */
  "onSelect"?: (Action | null);
  /** The choices, each `{label, value}`. */
  "options"?: (Array<SelectOption> | null);
  /** The chosen option's `value`. */
  "value"?: (string | null);
  [key: string]: unknown;
} | {
  /** The URL. */
  "href": string;
  "kind": "Link";
  /** The text shown; the URL when absent. */
  "label"?: (string | null);
  [key: string]: unknown;
} | {
  /** Text shown where the image cannot be. */
  "alt": string;
  /** The width in terminal columns. */
  "columns"?: (number | null);
  "kind": "Image";
  /** The height in terminal rows. */
  "rows"?: (number | null);
  /** A path or a `data:` URL. */
  "source": string;
  [key: string]: unknown;
} | {
  /** The cells, row by row. */
  "cells": Array<string>;
  /** The width in cells. */
  "columns": number;
  "kind": "Raster";
  /** The height in cells. */
  "rows": number;
  [key: string]: unknown;
} | {
  /** Text shown where the drawing cannot be. */
  "alt": string;
  /** The height in pixels. */
  "height"?: (number | null);
  "kind": "Svg";
  /** The SVG markup. */
  "source": string;
  /** The width in pixels. */
  "width"?: (number | null);
  [key: string]: unknown;
} | {
  /** The component bingo draws, such as `ToolUse`. */
  "component": Component;
  "kind": "Engine";
  /** That component's props, as its `ui.render` event carried them. */
  "props": unknown;
  [key: string]: unknown;
} | {
  /** The element's own name. */
  "customKind": string;
  /** Whatever the element needs. */
  "data": unknown;
  /** The plain text drawn where the element is unknown. */
  "fold": string;
  "kind": "custom";
  [key: string]: unknown;
});
/** How a producer intends an utterance to be drawn. */
export type Voice = ("said" | "note" | "quiet" | "aside");
export type Wrap = ("wrap" | "nowrap");
export const dollarRoutes = {
  "mcp.call": {
    "face": "mcp",
    "owner": true,
    "site": [
      "session"
    ]
  },
  "mcp.connect": {
    "face": "mcp",
    "owner": true,
    "site": [
      "session"
    ]
  },
  "agent.spawn": {
    "face": "agent",
    "owner": true,
    "site": [
      "session"
    ]
  },
  "agent.list": {
    "face": "agent",
    "owner": true,
    "site": [
      "session"
    ]
  },
  "agent.register": {
    "face": "agent",
    "owner": true,
    "site": [
      "session"
    ]
  },
  "http.fetch": {
    "face": "bingo.plugin.http",
    "site": [
      "session"
    ]
  },
  "env.get": {
    "face": "bingo.plugin.env",
    "site": [
      "session"
    ]
  },
  "env.set": {
    "face": "bingo.plugin.env",
    "site": [
      "session"
    ]
  },
  "process.run": {
    "face": "bingo.plugin.process",
    "site": [
      "session"
    ]
  },
  "process.spawn": {
    "face": "bingo.plugin.process",
    "site": [
      "session"
    ]
  },
  "process.read": {
    "face": "bingo.plugin.process",
    "site": [
      "session"
    ]
  },
  "process.write": {
    "face": "bingo.plugin.process",
    "site": [
      "session"
    ]
  },
  "process.kill": {
    "face": "bingo.plugin.process",
    "site": [
      "session"
    ]
  },
  "fs.read": {
    "face": "bingo.plugin.fs",
    "site": [
      "session"
    ]
  },
  "fs.write": {
    "face": "bingo.plugin.fs",
    "site": [
      "session"
    ]
  },
  "fs.list": {
    "face": "bingo.plugin.fs",
    "site": [
      "session"
    ]
  },
  "fs.exists": {
    "face": "bingo.plugin.fs",
    "site": [
      "session"
    ]
  },
  "fs.stat": {
    "face": "bingo.plugin.fs",
    "site": [
      "session"
    ]
  },
  "fs.ancestors": {
    "face": "bingo.plugin.fs",
    "site": [
      "session"
    ]
  },
  "session.messages": {
    "face": "bingo.plugin.session",
    "site": [
      "session"
    ]
  },
  "session.cwd": {
    "face": "bingo.plugin.session",
    "site": [
      "session"
    ]
  },
  "session.root": {
    "face": "bingo.plugin.session",
    "site": [
      "session"
    ]
  },
  "session.model": {
    "face": "bingo.plugin.session",
    "site": [
      "session"
    ]
  },
  "session.turns": {
    "face": "bingo.plugin.session",
    "site": [
      "session"
    ]
  },
  "session.id": {
    "face": "bingo.plugin.session",
    "site": [
      "session"
    ]
  },
  "session.surfaces": {
    "face": "bingo.plugin.session",
    "site": [
      "session"
    ]
  },
  "session.usage": {
    "face": "bingo.plugin.session",
    "site": [
      "session"
    ]
  },
  "session.version": {
    "face": "bingo.plugin.session",
    "site": []
  },
  "session.compact": {
    "kernel": "session.compact",
    "site": [
      "session"
    ]
  },
  "session.send": {
    "kernel": "session.send",
    "site": []
  },
  "session.append": {
    "kernel": "session.append",
    "site": [
      "session"
    ]
  },
  "session.authorize": {
    "face": "bingo.plugin.session",
    "site": [
      "session"
    ]
  },
  "turn.abort": {
    "face": "bingo.plugin.turn",
    "site": [
      "session"
    ]
  },
  "prompt.submit": {
    "kernel": "prompt.submit",
    "site": [
      "session"
    ]
  },
  "prompt.read": {
    "face": "bingo.plugin.prompt",
    "site": [
      "session"
    ]
  },
  "tool.list": {
    "face": "bingo.plugin.tool",
    "site": []
  },
  "tool.call": {
    "kernel": "tool.call",
    "site": [
      "session"
    ]
  },
  "tool.register": {
    "face": "bingo.plugin.tool",
    "site": [
      "session"
    ]
  },
  "command.list": {
    "face": "bingo.plugin.command",
    "site": []
  },
  "command.run": {
    "kernel": "command.run",
    "site": [
      "session"
    ]
  },
  "command.register": {
    "face": "bingo.plugin.command",
    "site": [
      "session"
    ]
  },
  "config.list": {
    "face": "bingo.plugin.config",
    "site": [
      "session"
    ]
  },
  "config.read": {
    "face": "bingo.plugin.config",
    "site": [
      "session"
    ]
  },
  "config.set": {
    "kernel": "config.set",
    "site": [
      "session"
    ]
  },
  "model.complete": {
    "kernel": "model.complete",
    "site": [
      "session",
      "turn"
    ]
  },
  "model.classify": {
    "face": "bingo.plugin.model",
    "site": [
      "session",
      "turn"
    ]
  },
  "model.fork": {
    "face": "bingo.plugin.model",
    "site": [
      "session",
      "turn"
    ]
  },
  "state.get": {
    "face": "bingo.plugin.state",
    "site": []
  },
  "state.set": {
    "face": "bingo.plugin.state",
    "site": []
  },
  "store.get": {
    "face": "bingo.plugin.store",
    "site": []
  },
  "store.set": {
    "face": "bingo.plugin.store",
    "site": []
  },
  "store.delete": {
    "face": "bingo.plugin.store",
    "site": []
  },
  "store.keys": {
    "face": "bingo.plugin.store",
    "site": []
  },
  "clock.now": {
    "face": "bingo.plugin.clock",
    "site": []
  },
  "clock.sleep": {
    "face": "bingo.plugin.clock",
    "site": []
  },
  "clock.after": {
    "face": "bingo.plugin.clock",
    "site": [
      "session"
    ]
  },
  "clock.every": {
    "face": "bingo.plugin.clock",
    "site": [
      "session"
    ]
  }
} as const;
/** The `$` nouns: what each is, with an example. */
export interface DollarNouns {
  /**
   * This plugin's own `name`, `root` and `version`: plain data, not methods.
   *
   * @example
   * const {name, root} = $.plugin;
   * const manifest = `${root}/plugin.json`;
   */
  readonly "plugin": unknown;
  /**
   * The session the hook runs in: its history, directories, model, usage and clients, and delivery into it. Methods take `session` from the hook unless given.
   *
   * @example
   * const cwd = await $.session.cwd();
   * const {context} = await $.session.usage();
   * const model = await $.session.model();
   */
  "session": unknown;
  /**
   * The session's running turn.
   *
   * @example
   * await $.turn.abort();
   */
  "turn": unknown;
  /**
   * The prompt: what is queued, and submitting input as a person would.
   *
   * @example
   * const {queue, last} = await $.prompt.read();
   */
  "prompt": unknown;
  /**
   * The tools the model can call: list them, call one through the permission gate, or register one this plugin answers.
   *
   * @example
   * await $.tool.register({
   *   name: 'Shout',
   *   description: 'Upper-case the given text.',
   *   inputSchema: {type: 'object', properties: {text: {type: 'string'}}, required: ['text']},
   * }, async ($, input) => {
   *   const {text} = input as {text: string};
   *   return {parts: [{type: 'text', text: text.toUpperCase()}], isError: false};
   * });
   * const tools = await $.tool.list();
   */
  "tool": unknown;
  /**
   * Slash commands: list them, run one, or register one this plugin answers. The handler receives `($, args)`, `args` being the text typed after the name (`''` for none), and returns an outcome such as `{kind: 'applied', message}`. `$.command.run({name, args})` runs one as if typed.
   *
   * @example
   * await $.command.register({name: 'hello', description: 'Say hello.', args: {kind: 'free', hint: '[name]'}}, async ($, args) => {
   *   await $.ui.toast(`Hello, ${String(args) || 'world'}.`);
   *   return {kind: 'applied', message: 'Said hello.'};
   * });
   */
  "command": unknown;
  /**
   * Settings: list the view a client sees, read one by its dotted key, or change one.
   *
   * @example
   * const settings = await $.config.list();
   */
  "config": unknown;
  /**
   * The host's model, asked on this plugin's behalf; every method needs the `model` grant.
   *
   * @example
   * const label = await $.model.classify({text: 'the build fails on Windows', labels: ['bug', 'feature']});
   */
  "model": unknown;
  /**
   * Values this plugin keeps per session, each with a version. For typed keys, `atom`, `read` and `update` from 'bingo' wrap these calls.
   *
   * @example
   * const saved = await $.state.get('count');
   * const count = typeof saved?.value === 'number' ? saved.value : 0;
   * await $.state.set('count', count + 1, saved?.version);
   */
  "state": unknown;
  /**
   * Values this plugin keeps across sessions, each with a version.
   *
   * @example
   * await $.store.set('greeted', true);
   * const keys = await $.store.keys();
   * await $.store.delete('greeted');
   */
  "store": unknown;
  /**
   * The host's clock. Use it instead of `setTimeout`: callbacks receive a fresh `$`, and tests can set or advance the time.
   *
   * @example
   * const started = await $.clock.now();
   * await $.clock.sleep(10);
   * await $.clock.after(60_000, async $ => {
   *   await $.ui.toast('A minute has passed.');
   * });
   */
  "clock": unknown;
  /**
   * Files under the roots this plugin is granted; paths are relative to the session's cwd.
   *
   * @example
   * if (await $.fs.exists({path: 'package.json'})) {
   *   const text = await $.fs.read({path: 'package.json'});
   * }
   */
  "fs": unknown;
  /**
   * HTTP through the host; needs the `http` grant.
   *
   * @example
   * const {status, body} = await $.http.fetch({url: 'https://example.com/'});
   */
  "http": unknown;
  /**
   * This plugin's environment overlay: variables its processes see. Needs the `env` grant.
   *
   * @example
   * await $.env.set({name: 'MY_MOD_MODE', value: 'quiet'});
   * const mode = await $.env.get({name: 'MY_MOD_MODE'});
   */
  "env": unknown;
  /**
   * Programs run by argv, never through a shell; needs the `process` grant.
   *
   * @example
   * const {code, stdout} = await $.process.run({argv: ['git', 'status', '--short']});
   * const job = await $.process.spawn({argv: ['sleep', '5']});
   * await job.kill();
   */
  "process": unknown;
  /**
   * Sub-agents, through the agent service.
   *
   * @example
   * const children = await $.agent.list();
   * await $.agent.spawn({args: {prompt: 'List the open TODOs.'}});
   */
  "agent": unknown;
  /**
   * MCP servers' tools, through the MCP service.
   *
   * @example
   * const output = await $.mcp.call({server: 'docs', tool: 'search', args: {query: 'bingo'}});
   */
  "mcp": unknown;
  /**
   * The interface: status entries, toasts and notices, pulled panes, re-rendering, and questions to the person. Drawing itself is a `ui.render` hook.
   *
   * @example
   * await $.ui.status('Ready', {key: 'demo'});
   * await $.ui.toast('Saved.');
   * await $.ui.open({id: 'demo', title: 'Demo'});
   * await $.ui.close('demo');
   */
  "ui": unknown;
}
/** Author signatures where the SDK wraps a row, and calls no row has. */
export interface DollarCalls {
  "state": {
    /** Read this plugin's value for `key` in the current session, with its version, or `null`. */
    get(key: string): Promise<StateValue | null>;
    /** Write this plugin's value for `key` in the current session; `ifVersion` refuses a stale write. */
    set(key: string, value: unknown, ifVersion?: number): Promise<StateValue>;
  };
  "store": {
    /** Read this plugin's global value for `key`, shared across sessions, or `null`. */
    get(key: string): Promise<StateValue | null>;
    /** Write this plugin's global value for `key`; `ifVersion` refuses a stale write. */
    set(key: string, value: unknown, ifVersion?: number): Promise<StateValue>;
    /** Delete this plugin's global value for `key`; answers whether it existed. */
    delete(key: string): Promise<boolean>;
  };
  "clock": {
    /** Wait `ms` milliseconds on the host clock. */
    sleep(ms: number): Promise<null>;
    /** Run a callback once after `ms` milliseconds; answers its timer id. */
    after(ms: number, handler: Callback): Promise<string>;
    /** Run a callback every `ms` milliseconds; answers its timer id. */
    every(ms: number, handler: Callback): Promise<string>;
  };
  "model": {
    /** Ask the host's model for a completion; runs `model.complete`. */
    complete(request: ModelRequest): Promise<Completed>;
  };
  "tool": {
    /** Register a tool the model can call; the author call takes the spec and a handler. */
    register(spec: ToolSpec, handler: Callback): Promise<string>;
  };
  "command": {
    /** Register a slash command; the author call takes the spec and an optional handler. */
    register(spec: CommandRegistration, handler?: Callback): Promise<string>;
  };
  "process": {
    /** Start `argv` in the background; the author call answers with a handle to read, write or kill. */
    spawn(input: AtSite<DollarInputs['process.spawn']>): Promise<ProcessHandle>;
  };
  "session": {
    /** Deliver text input into this session as this plugin's message; `wake` (default `true`) opens a turn when the session is idle, otherwise it waits for the next one. */
    deliver(input: Input, wake?: boolean): Promise<null>;
    /** Record this plugin's state for `key` in the session's journal; the value replaces the whole of `key` and survives a restart. */
    extend(key: string, value: unknown): Promise<null>;
  };
  "ui": {
    /** Place a status entry under `key` (default `status`); `null` removes it. */
    status(text: string | null | undefined, options?: {key?: string; tone?: Wire.Tone}): Promise<null>;
    /** Show a short-lived notification; `duration` is in milliseconds. */
    toast(text: string, options?: {duration?: number}): Promise<null>;
    /** Like `toast`, but marked as an alert for surfaces that raise one. */
    notify(text: string, options?: {duration?: number}): Promise<null>;
    /** Publish a pulled pane; draw it by matching `ui.render` with `{component: 'Pane', requestId: id}`. */
    open(options: OpenPane): Promise<Record<string, never>>;
    /** Remove the pane this plugin opened as `id`. */
    close(id: string): Promise<null>;
    /** The panes and slots placed in this session. */
    panes(): Promise<Wire.Slot[]>;
    /** Ask surfaces to render these request ids again. */
    invalidate(requests: string | string[]): Promise<null>;
    /** The elements the asking surface draws, by name, such as `Box`, `Text` and `Button`; pass the `ui.render` event. */
    resolve(event: {surface: Wire.SurfaceName}): ElementTable;
    /** Write a line to bingo's log under `mod::<name>`. */
    log(text: string): Promise<null>;
    /** Post a notice to the session's event stream. */
    notice(text: string, options?: {level?: Wire.Level}): Promise<null>;
    /** Ask the person a question with these answer kinds and wait for the answer. */
    ask(kind: Wire.InteractionKind, answers: Wire.AnswerSpec[]): Promise<Wire.Answer>;
  };
}
