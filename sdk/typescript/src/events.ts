// Generated from schema/events.json. Do not edit.
export namespace UiPressInput {
// Generated from JSON Schema. Do not edit.
export type Payload = {
  /** `press`, `input`, `select` or `close`. */
  "event": string;
  /** The token a drawn handler became, or a pane id for `close`. */
  "handler": string;
  /** What the surface sent: the text, the chosen value, or the key. */
  "value"?: unknown;
  [key: string]: unknown;
};
}
export namespace UiPressResult {
// Generated from JSON Schema. Do not edit.
/** A control may return the key to the asking surface. */
export type HandlerResult = {
  /** `false` returns the key to the surface. */
  "handled": boolean;
  [key: string]: unknown;
};
export type Payload = (HandlerResult | null);
}
export type UiPress = UiPressInput.Payload;
export namespace UiInputInput {
// Generated from JSON Schema. Do not edit.
export type Payload = {
  /** `press`, `input`, `select` or `close`. */
  "event": string;
  /** The token a drawn handler became, or a pane id for `close`. */
  "handler": string;
  /** What the surface sent: the text, the chosen value, or the key. */
  "value"?: unknown;
  [key: string]: unknown;
};
}
export namespace UiInputResult {
// Generated from JSON Schema. Do not edit.
export type Payload = null;
}
export type UiInput = UiInputInput.Payload;
export namespace UiSelectInput {
// Generated from JSON Schema. Do not edit.
export type Payload = {
  /** `press`, `input`, `select` or `close`. */
  "event": string;
  /** The token a drawn handler became, or a pane id for `close`. */
  "handler": string;
  /** What the surface sent: the text, the chosen value, or the key. */
  "value"?: unknown;
  [key: string]: unknown;
};
}
export namespace UiSelectResult {
// Generated from JSON Schema. Do not edit.
export type Payload = null;
}
export type UiSelect = UiSelectInput.Payload;
export namespace UiRenderInput {
// Generated from JSON Schema. Do not edit.
export type Anchor = ("topLeft" | "topRight" | "bottomLeft" | "bottomRight" | "center");
export type Component = ("KeyBar" | "Composer" | "ModelPicker" | "Browse" | "Plugins" | "Palette" | "Sessions" | "Status" | "Question" | "UserMessage" | "AssistantMessage" | "ToolUse" | "ToolResult" | "ToolGroup" | "ToolProgress" | "CommandOutput" | "Spinner" | "TurnDuration" | "InfoNotice" | "SessionMode" | "PromptHint" | "AbovePrompt" | "Pane" | "Permission" | "SignIn" | "Reasoning" | "Compaction" | "Rewind" | "Interruption" | "QuestionAnswer" | "PermissionReceipt" | "Asset" | "ShellRun" | "Action" | "SessionStatus" | "Toast" | "Welcome" | "Ledger");
export type GlyphSet = ("unicode" | "ascii");
/**
 * A length: a number of terminal cells, or a share of the screen written as
 * a percentage string such as `'40%'`.
 */
export type Length = (number | string);
/**
 * Where a pane sits: `'above'` the prompt, at the `'side'`, as an
 * `'overlay'` floating over the screen, in the `'dock'`, or at a fixed `{rect}`.
 */
export type PanePlace = (("above" | "side" | "overlay") | "dock" | { "rect": Rect; });
/**
 * A fixed rectangle: `width` by `height`, offset by `x` and `y` from
 * `anchor`, a corner or the `center` (`topLeft` unless given).
 */
export type Rect = { "anchor"?: Anchor; "height": Length; "width": Length; "x"?: Length; "y"?: Length; [key: string]: unknown; };
export type SurfaceName = ("terminal" | "desktop" | "text" | "chat");
export type Viewport = {
  /** Width in cells. */
  "columns": number;
  "glyphs"?: GlyphSet;
  /** Where a pane sits, when the site is a pane. */
  "place"?: (PanePlace | null);
  /** Height in cells. */
  "rows": number;
  [key: string]: unknown;
};
export type Payload = {
  /** The site asked for. */
  "component": Component;
  /** The site's facts; their shape is the component's props. */
  "props": unknown;
  /** Which instance of the site; for `Pane`, the id given to `$.ui.open`. */
  "requestId": string;
  /** The asking surface. */
  "surface": SurfaceName;
  /** The cells the surface offers the drawing. */
  "viewport": Viewport;
  [key: string]: unknown;
};
}
export namespace UiRenderResult {
// Generated from JSON Schema. Do not edit.
/** A typed action a client asks for (GUI buttons, hosts); the kernel dispatches it to a command. */
export type Action = { "args"?: unknown; "name": string; [key: string]: unknown; };
/** A button: the surface fires `Input::Action` with `action` (ADR-0008 §1). */
export type ActionItem = {
  "action": Action;
  /** A single-key hint; the surface may use another key. */
  "key"?: (string | null);
  "label": string;
  /** A one-line explanation supplied by the action owner. */
  "meaning"?: (string | null);
  [key: string]: unknown;
};
export type Align = ("flex-start" | "center" | "flex-end" | "stretch");
export type Component = ("KeyBar" | "Composer" | "ModelPicker" | "Browse" | "Plugins" | "Palette" | "Sessions" | "Status" | "Question" | "UserMessage" | "AssistantMessage" | "ToolUse" | "ToolResult" | "ToolGroup" | "ToolProgress" | "CommandOutput" | "Spinner" | "TurnDuration" | "InfoNotice" | "SessionMode" | "PromptHint" | "AbovePrompt" | "Pane" | "Permission" | "SignIn" | "Reasoning" | "Compaction" | "Rewind" | "Interruption" | "QuestionAnswer" | "PermissionReceipt" | "Asset" | "ShellRun" | "Action" | "SessionStatus" | "Toast" | "Welcome" | "Ledger");
export type ComposerProps = { "placeholder": string; "tag": string; [key: string]: unknown; };
export type Direction = ("row" | "column");
/** The progress curve of a text fade. */
export type FadeCurve = ("easeOut");
export type Justify = ("flex-start" | "center" | "flex-end" | "space-between" | "space-around" | "space-evenly");
export type KeyPair = { "action": string; "emphasis"?: boolean; "keys": string; [key: string]: unknown; };
/** A question owner's editing projection; sealed labels remain the host's. */
export type QuestionFocus = { "checked": number; "field": number; "writing": boolean; [key: string]: unknown; };
export type SelectOption = {
  /** The text shown. */
  "label": string;
  /** The value chosen. */
  "value": string;
  [key: string]: unknown;
};
export type SiteHints = { "composer"?: (ComposerProps | null); "pairs"?: Array<KeyPair>; [key: string]: unknown; };
/** A text child preserves the style of its nested Text node. */
export type TextChild = (string | unknown);
/** Declarative motion on the surface clock, with a reduced-motion still form. */
export type TextMotion = ({ "depth": number; "kind": "breathe"; "periodMs": number; "with": string; [key: string]: unknown; } | { "kind": "spin"; [key: string]: unknown; } | { "kind": "shimmer"; [key: string]: unknown; } | { "atMs": number; "curve"?: (FadeCurve | null); "from": string; "kind": "fade"; "ms": number; [key: string]: unknown; } | { "atMs": number; "kind": "countdown"; "prefix": string; [key: string]: unknown; });
/** The one styling hook a plugin has; the surface owns the colour. */
export type Tone = (("neutral" | "good" | "bad") | "attention");
export type TreeNode = { "badge"?: (string | null); "children"?: Array<TreeNode>; "label": string; "tone"?: Tone; [key: string]: unknown; };
export type Wrap = ("wrap" | "nowrap");
export type Payload = ({
  /** Draw the text bold. */
  "bold"?: (boolean | null);
  /** Strings and nested `Text` nodes, drawn in order. */
  "children": Array<TextChild>;
  /**
   * An Ember ink: fg, muted, dim, faint, ember, glow, coal, amber, amberGlow,
   * good, bad, plan, auto, code, crew0–crew5, kw, str, num, fn, ty; or `#rrggbb`.
   * Ground inks: bg, you, well, wellDeep, pick, pickBg, addTint, delTint,
   * addWord, delWord, badTint, amberTint, glowTint.
   * Legacy ash, presence and attention are accepted; unknown names draw plain.
   */
  "color"?: (string | null);
  /** Draw the text dimmed. */
  "dimColor"?: (boolean | null);
  /** An Ember ground, filled to the available width. */
  "ground"?: (string | null);
  /** Swap the foreground and background colours. */
  "inverse"?: (boolean | null);
  /** Draw the text in italics. */
  "italic"?: (boolean | null);
  "kind": "Text";
  /** A surface-clock animation; the author never needs to redraw it. */
  "motion"?: (TextMotion | null);
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
  "children": Array<unknown>;
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
  /** An Ember ground, filled across the box including padding. */
  "ground"?: (string | null);
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
  "child": unknown;
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
}
export type UiRender = UiRenderInput.Payload;
export namespace PluginRegisterInput {
// Generated from JSON Schema. Do not edit.
/** The directory that supplied an external plugin. */
export type PluginOrigin = ("user" | "project" | "explicit");
export type PluginRuntime = ("process" | "module");
export type Tier = ("tighten" | "rewrite" | "full");
export type Payload = { "name": string; "origin": PluginOrigin; "runtime": PluginRuntime; "tier": Tier; "uses": Array<string>; "version": string; [key: string]: unknown; };
}
export namespace PluginRegisterResult {
// Generated from JSON Schema. Do not edit.
export type PluginGrant = { "tier": Tier; "uses": Array<string>; [key: string]: unknown; };
export type Tier = ("tighten" | "rewrite" | "full");
/** The registration chain may reduce a grant or refuse the plugin in words. */
export type Payload = (PluginGrant | { "reason": string; [key: string]: unknown; });
}
export type PluginRegister = PluginRegisterInput.Payload;
export namespace PromptSubmitInput {
// Generated from JSON Schema. Do not edit.
/** A typed action a client asks for (GUI buttons, hosts); the kernel dispatches it to a command. */
export type Action = { "args"?: unknown; "name": string; [key: string]: unknown; };
/**
 * How a line reaches a session's queue: a peer's message (ADR-0010 §1) and
 * a person's own line are the same question, so they are the same word.
 */
export type Delivery = ("wake" | "hold");
export type Headline = {
  /** Each part is one line of inline Markdown. */
  "parts": Array<string>;
  "tone": Tone;
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
export type SurfaceName = ("terminal" | "desktop" | "text" | "chat");
/** The one styling hook a plugin has; the surface owns the colour. */
export type Tone = (("neutral" | "good" | "bad") | "attention");
/** How a producer intends an utterance to be drawn. */
export type Voice = ("said" | "note" | "quiet" | "aside");
/**
 * The one submission entry. The kernel parses `/`, `!` and `@` in text and
 * decides turn, queue, steer or deliver; a client never chooses.
 */
export type Payload = ({
  /**
   * Whether a running turn may be steered with it (ADR-0008 §2,
   * amended M68). `Wake` is the line every client has always sent, so
   * it is the default and nothing is written for it.
   */
  "delivery"?: Delivery;
  "headline"?: (Headline | null);
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
}
export namespace PromptSubmitResult {
// Generated from JSON Schema. Do not edit.
/** A typed action a client asks for (GUI buttons, hosts); the kernel dispatches it to a command. */
export type Action = { "args"?: unknown; "name": string; [key: string]: unknown; };
/**
 * How a line reaches a session's queue: a peer's message (ADR-0010 §1) and
 * a person's own line are the same question, so they are the same word.
 */
export type Delivery = ("wake" | "hold");
export type ErrorCode = (("SESSION_NOT_FOUND" | "SESSION_LOCKED" | "SESSION_CLOSED" | "INTERACTION_CLOSED" | "NOT_READY" | "STALE_GENERATION" | "NOT_INITIALIZED" | "INVALID_INPUT" | "PERMISSION_DENIED" | "TOOL_NOT_FOUND" | "TOOL_FAILED" | "PROVIDER_UNAVAILABLE" | "AUTH_REQUIRED" | "RATE_LIMITED" | "CONTEXT_OVERFLOW" | "TIMEOUT" | "OFFLINE" | "SERVER_ERROR" | "TURN_LOST" | "TURN_BUDGET_EXHAUSTED" | "UNSUPPORTED" | "INTERNAL") | "INTERRUPTED" | "PROTOCOL_LIMIT" | "NOT_FOUND" | "STORAGE");
export type Headline = {
  /** Each part is one line of inline Markdown. */
  "parts": Array<string>;
  "tone": Tone;
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
  "headline"?: (Headline | null);
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
/** An error as it appears on the wire and in the journal. */
export type KernelError = { "code": ErrorCode; "message": string; [key: string]: unknown; };
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
/** One persisted context: one journal, one actor. */
export type SessionId = string;
export type SurfaceName = ("terminal" | "desktop" | "text" | "chat");
/** The one styling hook a plugin has; the surface owns the colour. */
export type Tone = (("neutral" | "good" | "bad") | "attention");
/** How a producer intends an utterance to be drawn. */
export type Voice = ("said" | "note" | "quiet" | "aside");
export type Payload = ({ "accepted": Input; } | { "rejected": KernelError; } | { "redirect": { "input": Input; "session": SessionId; [key: string]: unknown; }; });
}
export type PromptSubmit = PromptSubmitInput.Payload;
export namespace ToolCheckInput {
// Generated from JSON Schema. Do not edit.
/**
 * A tool call as a hook sees it: the tool's own arguments sit beside
 * `tool` and `tool_use_id`, not under an `input` field.
 */
export type Payload = {
  /** The tool's name, such as `Bash` or `Edit`. */
  "tool": string;
  /** The model's id for this call. */
  "tool_use_id": string;
  [key: string]: unknown;
};
}
export namespace ToolCheckResult {
// Generated from JSON Schema. Do not edit.
export type Decision = ({ "kind": "allow"; "reason": Reason; [key: string]: unknown; } | { "kind": "deny"; "reason": Reason; [key: string]: unknown; } | {
  "kind": "ask";
  "reason": Reason;
  /** The narrowest rule that would silence this prompt, offered as "allow for session". */
  "scope"?: (string | null);
  [key: string]: unknown;
});
export type Reason = ({ "kind": "rule"; "rule": string; "source"?: (RuleSource | null); [key: string]: unknown; } | { "kind": "mode"; "mode": string; [key: string]: unknown; } | { "hook": string; "kind": "hook"; [key: string]: unknown; } | { "detail": string; "kind": "safety"; [key: string]: unknown; } | { "kind": "readOnly"; [key: string]: unknown; } | { "detail": string; "kind": "confirm"; [key: string]: unknown; } | { "kind": "default"; [key: string]: unknown; });
/** Where the matched permission rule was installed. */
export type RuleSource = ("settings" | "session");
/** A call the model asked for, before or after the gate. */
export type ToolCall = { "callId": string; "input": unknown; "name": string; [key: string]: unknown; };
export type Payload = {
  /** The call as judged, possibly with tightened arguments. */
  "call": ToolCall;
  /** Whether it may run. */
  "decision": Decision;
  [key: string]: unknown;
};
}
export type ToolCheck = ToolCheckInput.Payload;
export namespace ToolCallInput {
// Generated from JSON Schema. Do not edit.
/**
 * A tool call as a hook sees it: the tool's own arguments sit beside
 * `tool` and `tool_use_id`, not under an `input` field.
 */
export type Payload = {
  /** The tool's name, such as `Bash` or `Edit`. */
  "tool": string;
  /** The model's id for this call. */
  "tool_use_id": string;
  [key: string]: unknown;
};
}
export namespace ToolCallResult {
// Generated from JSON Schema. Do not edit.
/** A typed action a client asks for (GUI buttons, hosts); the kernel dispatches it to a command. */
export type Action = { "args"?: unknown; "name": string; [key: string]: unknown; };
/** A button: the surface fires `Input::Action` with `action` (ADR-0008 §1). */
export type ActionItem = {
  "action": Action;
  /** A single-key hint; the surface may use another key. */
  "key"?: (string | null);
  "label": string;
  /** A one-line explanation supplied by the action owner. */
  "meaning"?: (string | null);
  [key: string]: unknown;
};
export type Align = ("flex-start" | "center" | "flex-end" | "stretch");
export type Component = ("KeyBar" | "Composer" | "ModelPicker" | "Browse" | "Plugins" | "Palette" | "Sessions" | "Status" | "Question" | "UserMessage" | "AssistantMessage" | "ToolUse" | "ToolResult" | "ToolGroup" | "ToolProgress" | "CommandOutput" | "Spinner" | "TurnDuration" | "InfoNotice" | "SessionMode" | "PromptHint" | "AbovePrompt" | "Pane" | "Permission" | "SignIn" | "Reasoning" | "Compaction" | "Rewind" | "Interruption" | "QuestionAnswer" | "PermissionReceipt" | "Asset" | "ShellRun" | "Action" | "SessionStatus" | "Toast" | "Welcome" | "Ledger");
export type ComposerProps = { "placeholder": string; "tag": string; [key: string]: unknown; };
export type ContentPart = ({ "text": string; "type": "text"; [key: string]: unknown; } | Image | { "id": string; "input": unknown; "name": string; "type": "toolUse"; [key: string]: unknown; } | { "isError"?: boolean; "parts": Array<ContentPart>; "toolUseId": string; "type": "toolResult"; [key: string]: unknown; } | { "providerMetadata"?: { [key: string]: { [key: string]: unknown; }; }; "text": string; "type": "reasoning"; [key: string]: unknown; });
export type Direction = ("row" | "column");
/** The progress curve of a text fade. */
export type FadeCurve = ("easeOut");
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
export type Justify = ("flex-start" | "center" | "flex-end" | "space-between" | "space-around" | "space-evenly");
export type KeyPair = { "action": string; "emphasis"?: boolean; "keys": string; [key: string]: unknown; };
/** A question owner's editing projection; sealed labels remain the host's. */
export type QuestionFocus = { "checked": number; "field": number; "writing": boolean; [key: string]: unknown; };
export type SelectOption = {
  /** The text shown. */
  "label": string;
  /** The value chosen. */
  "value": string;
  [key: string]: unknown;
};
export type SiteHints = { "composer"?: (ComposerProps | null); "pairs"?: Array<KeyPair>; [key: string]: unknown; };
/** A text child preserves the style of its nested Text node. */
export type TextChild = (string | View);
/** Declarative motion on the surface clock, with a reduced-motion still form. */
export type TextMotion = ({ "depth": number; "kind": "breathe"; "periodMs": number; "with": string; [key: string]: unknown; } | { "kind": "spin"; [key: string]: unknown; } | { "kind": "shimmer"; [key: string]: unknown; } | { "atMs": number; "curve"?: (FadeCurve | null); "from": string; "kind": "fade"; "ms": number; [key: string]: unknown; } | { "atMs": number; "kind": "countdown"; "prefix": string; [key: string]: unknown; });
/** The one styling hook a plugin has; the surface owns the colour. */
export type Tone = (("neutral" | "good" | "bad") | "attention");
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
export type TreeNode = { "badge"?: (string | null); "children"?: Array<TreeNode>; "label": string; "tone"?: Tone; [key: string]: unknown; };
/** A call's result in one line: `+1 −1`, `3 passed`, `exit 1`. */
export type Upshot = ({ "added": number; "kind": "diff"; "removed": number; [key: string]: unknown; } | { "kind": "words"; "text": string; "tone": Tone; [key: string]: unknown; });
export type View = ({
  /** Draw the text bold. */
  "bold"?: (boolean | null);
  /** Strings and nested `Text` nodes, drawn in order. */
  "children": Array<TextChild>;
  /**
   * An Ember ink: fg, muted, dim, faint, ember, glow, coal, amber, amberGlow,
   * good, bad, plan, auto, code, crew0–crew5, kw, str, num, fn, ty; or `#rrggbb`.
   * Ground inks: bg, you, well, wellDeep, pick, pickBg, addTint, delTint,
   * addWord, delWord, badTint, amberTint, glowTint.
   * Legacy ash, presence and attention are accepted; unknown names draw plain.
   */
  "color"?: (string | null);
  /** Draw the text dimmed. */
  "dimColor"?: (boolean | null);
  /** An Ember ground, filled to the available width. */
  "ground"?: (string | null);
  /** Swap the foreground and background colours. */
  "inverse"?: (boolean | null);
  /** Draw the text in italics. */
  "italic"?: (boolean | null);
  "kind": "Text";
  /** A surface-clock animation; the author never needs to redraw it. */
  "motion"?: (TextMotion | null);
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
  /** An Ember ground, filled across the box including padding. */
  "ground"?: (string | null);
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
export type Wrap = ("wrap" | "nowrap");
/** A denial is an answer, never an invalid output that falls through to execution. */
export type Payload = ({ "deny": string; [key: string]: unknown; } | ToolOutput);
}
export type ToolCall = ToolCallInput.Payload;
export namespace ModelCompleteInput {
// Generated from JSON Schema. Do not edit.
export type ContentPart = ({ "text": string; "type": "text"; [key: string]: unknown; } | Image | { "id": string; "input": unknown; "name": string; "type": "toolUse"; [key: string]: unknown; } | { "isError"?: boolean; "parts": Array<ContentPart>; "toolUseId": string; "type": "toolResult"; [key: string]: unknown; } | { "providerMetadata"?: { [key: string]: { [key: string]: unknown; }; }; "text": string; "type": "reasoning"; [key: string]: unknown; });
/** Reasoning effort, in the vocabulary providers converge on. */
export type Effort = ("minimal" | "low" | "medium" | "high" | "xHigh" | "max");
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
export type Message = { "parts": Array<ContentPart>; "providerOptions"?: { [key: string]: { [key: string]: unknown; }; }; "role": Role; [key: string]: unknown; };
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
/** How a surface reads a row's target. */
export type Reads = ("path" | "command" | "text");
export type Role = ("user" | "assistant");
/** One persisted context: one journal, one actor. */
export type SessionId = string;
/** One cacheable segment of the system prompt. */
export type SystemBlock = { "cache"?: boolean; "text": string; [key: string]: unknown; };
/** The top-level input field a tool's row names. */
export type Target = { "field": string; "reads": Reads; [key: string]: unknown; };
/** How a tool reads on a surface; never sent to a model. */
export type ToolLook = { "ascii"?: (string | null); "mark": string; "target"?: (Target | null); "verb": string; [key: string]: unknown; };
/** A tool as the model sees it. */
export type ToolSpec = {
  "description": string;
  "inputSchema": unknown;
  "look"?: (ToolLook | null);
  /**
   * What a catalogue may show beside the tool (an MCP tool's `server`);
   * never sent to a model.
   */
  "meta"?: { [key: string]: unknown; };
  "name": string;
  [key: string]: unknown;
};
export type Payload = {
  /** The plugin that asked. */
  "plugin": string;
  /** The request it sent. */
  "request": ModelRequest;
  [key: string]: unknown;
};
}
export namespace ModelCompleteResult {
// Generated from JSON Schema. Do not edit.
export type FinishReason = { "raw"?: (string | null); "unified": UnifiedFinish; [key: string]: unknown; };
/** Provider stream events. Never published; the accumulator folds them into items. */
export type ModelEvent = ({ "type": "streamStart"; "warnings"?: Array<string>; [key: string]: unknown; } | { "id"?: (string | null); "model"?: (string | null); "type": "responseMetadata"; [key: string]: unknown; } | { "id": string; "type": "textStart"; [key: string]: unknown; } | { "delta": string; "id": string; "type": "textDelta"; [key: string]: unknown; } | { "id": string; "type": "textEnd"; [key: string]: unknown; } | { "id": string; "type": "reasoningStart"; [key: string]: unknown; } | { "delta": string; "id": string; "type": "reasoningDelta"; [key: string]: unknown; } | { "id": string; "providerMetadata"?: { [key: string]: { [key: string]: unknown; }; }; "type": "reasoningEnd"; [key: string]: unknown; } | { "id": string; "name": string; "type": "toolInputStart"; [key: string]: unknown; } | { "delta": string; "id": string; "type": "toolInputDelta"; [key: string]: unknown; } | { "id": string; "type": "toolInputEnd"; [key: string]: unknown; } | { "id": string; "input": string; "name": string; "type": "toolCall"; [key: string]: unknown; } | { "type": "context"; "used": number; "window": number; [key: string]: unknown; } | { "after": number; "before": number; "type": "compacted"; [key: string]: unknown; } | { "finishReason": FinishReason; "type": "finish"; "usage": Usage; [key: string]: unknown; });
export type UnifiedFinish = ("stop" | "length" | "toolCalls" | "contentFilter" | "error" | "other");
/**
 * Token counts as the provider reports them, the three input counts kept
 * apart: `input_tokens` is what the model read fresh, the cache counts are
 * what it read from or wrote to a cached prefix. The whole input side is
 * `input_total()`.
 */
export type Usage = { "cacheReadTokens"?: number; "cacheWriteTokens"?: number; "inputTokens": number; "outputTokens": number; "reasoningTokens"?: number; [key: string]: unknown; };
export type Payload = Array<ModelEvent>;
}
export type ModelComplete = ModelCompleteInput.Payload;
export namespace TurnStepInput {
// Generated from JSON Schema. Do not edit.
export type ContentPart = ({ "text": string; "type": "text"; [key: string]: unknown; } | Image | { "id": string; "input": unknown; "name": string; "type": "toolUse"; [key: string]: unknown; } | { "isError"?: boolean; "parts": Array<ContentPart>; "toolUseId": string; "type": "toolResult"; [key: string]: unknown; } | { "providerMetadata"?: { [key: string]: { [key: string]: unknown; }; }; "text": string; "type": "reasoning"; [key: string]: unknown; });
/** Reasoning effort, in the vocabulary providers converge on. */
export type Effort = ("minimal" | "low" | "medium" | "high" | "xHigh" | "max");
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
export type Message = { "parts": Array<ContentPart>; "providerOptions"?: { [key: string]: { [key: string]: unknown; }; }; "role": Role; [key: string]: unknown; };
/** How a surface reads a row's target. */
export type Reads = ("path" | "command" | "text");
export type Role = ("user" | "assistant");
/** One persisted context: one journal, one actor. */
export type SessionId = string;
/** One cacheable segment of the system prompt. */
export type SystemBlock = { "cache"?: boolean; "text": string; [key: string]: unknown; };
/** The top-level input field a tool's row names. */
export type Target = { "field": string; "reads": Reads; [key: string]: unknown; };
/** How a tool reads on a surface; never sent to a model. */
export type ToolLook = { "ascii"?: (string | null); "mark": string; "target"?: (Target | null); "verb": string; [key: string]: unknown; };
/** A tool as the model sees it. */
export type ToolSpec = {
  "description": string;
  "inputSchema": unknown;
  "look"?: (ToolLook | null);
  /**
   * What a catalogue may show beside the tool (an MCP tool's `server`);
   * never sent to a model.
   */
  "meta"?: { [key: string]: unknown; };
  "name": string;
  [key: string]: unknown;
};
export type Payload = {
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
}
export namespace TurnStepResult {
// Generated from JSON Schema. Do not edit.
export type FinishReason = { "raw"?: (string | null); "unified": UnifiedFinish; [key: string]: unknown; };
/** Provider stream events. Never published; the accumulator folds them into items. */
export type ModelEvent = ({ "type": "streamStart"; "warnings"?: Array<string>; [key: string]: unknown; } | { "id"?: (string | null); "model"?: (string | null); "type": "responseMetadata"; [key: string]: unknown; } | { "id": string; "type": "textStart"; [key: string]: unknown; } | { "delta": string; "id": string; "type": "textDelta"; [key: string]: unknown; } | { "id": string; "type": "textEnd"; [key: string]: unknown; } | { "id": string; "type": "reasoningStart"; [key: string]: unknown; } | { "delta": string; "id": string; "type": "reasoningDelta"; [key: string]: unknown; } | { "id": string; "providerMetadata"?: { [key: string]: { [key: string]: unknown; }; }; "type": "reasoningEnd"; [key: string]: unknown; } | { "id": string; "name": string; "type": "toolInputStart"; [key: string]: unknown; } | { "delta": string; "id": string; "type": "toolInputDelta"; [key: string]: unknown; } | { "id": string; "type": "toolInputEnd"; [key: string]: unknown; } | { "id": string; "input": string; "name": string; "type": "toolCall"; [key: string]: unknown; } | { "type": "context"; "used": number; "window": number; [key: string]: unknown; } | { "after": number; "before": number; "type": "compacted"; [key: string]: unknown; } | { "finishReason": FinishReason; "type": "finish"; "usage": Usage; [key: string]: unknown; });
export type ProviderError = ({ "kind": "auth"; "message": string; [key: string]: unknown; } | { "kind": "rateLimited"; "retryAfterMs"?: (number | null); [key: string]: unknown; } | { "kind": "contextOverflow"; "message": string; [key: string]: unknown; } | { "kind": "server"; "message": string; "status": number; [key: string]: unknown; } | { "kind": "request"; "message": string; [key: string]: unknown; } | { "kind": "transport"; "message": string; [key: string]: unknown; } | { "kind": "stream"; "message": string; [key: string]: unknown; } | { "kind": "timeout"; [key: string]: unknown; } | { "kind": "unsupported"; "message": string; [key: string]: unknown; } | { "kind": "config"; "message": string; [key: string]: unknown; });
export type UnifiedFinish = ("stop" | "length" | "toolCalls" | "contentFilter" | "error" | "other");
/**
 * Token counts as the provider reports them, the three input counts kept
 * apart: `input_tokens` is what the model read fresh, the cache counts are
 * what it read from or wrote to a cached prefix. The whole input side is
 * `input_total()`.
 */
export type Usage = { "cacheReadTokens"?: number; "cacheWriteTokens"?: number; "inputTokens": number; "outputTokens": number; "reasoningTokens"?: number; [key: string]: unknown; };
export type Payload = ({ "Ok": Array<ModelEvent>; [key: string]: unknown; } | { "Err": ProviderError; [key: string]: unknown; });
}
export type TurnStep = TurnStepInput.Payload;
export namespace PromptComposeInput {
// Generated from JSON Schema. Do not edit.
export type ContentPart = ({ "text": string; "type": "text"; [key: string]: unknown; } | Image | { "id": string; "input": unknown; "name": string; "type": "toolUse"; [key: string]: unknown; } | { "isError"?: boolean; "parts": Array<ContentPart>; "toolUseId": string; "type": "toolResult"; [key: string]: unknown; } | { "providerMetadata"?: { [key: string]: { [key: string]: unknown; }; }; "text": string; "type": "reasoning"; [key: string]: unknown; });
/** Reasoning effort, in the vocabulary providers converge on. */
export type Effort = ("minimal" | "low" | "medium" | "high" | "xHigh" | "max");
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
export type Message = { "parts": Array<ContentPart>; "providerOptions"?: { [key: string]: { [key: string]: unknown; }; }; "role": Role; [key: string]: unknown; };
/** How a surface reads a row's target. */
export type Reads = ("path" | "command" | "text");
export type Role = ("user" | "assistant");
/** One persisted context: one journal, one actor. */
export type SessionId = string;
/** One cacheable segment of the system prompt. */
export type SystemBlock = { "cache"?: boolean; "text": string; [key: string]: unknown; };
/** The top-level input field a tool's row names. */
export type Target = { "field": string; "reads": Reads; [key: string]: unknown; };
/** How a tool reads on a surface; never sent to a model. */
export type ToolLook = { "ascii"?: (string | null); "mark": string; "target"?: (Target | null); "verb": string; [key: string]: unknown; };
/** A tool as the model sees it. */
export type ToolSpec = {
  "description": string;
  "inputSchema": unknown;
  "look"?: (ToolLook | null);
  /**
   * What a catalogue may show beside the tool (an MCP tool's `server`);
   * never sent to a model.
   */
  "meta"?: { [key: string]: unknown; };
  "name": string;
  [key: string]: unknown;
};
export type Payload = {
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
}
export namespace PromptComposeResult {
// Generated from JSON Schema. Do not edit.
export type ContentPart = ({ "text": string; "type": "text"; [key: string]: unknown; } | Image | { "id": string; "input": unknown; "name": string; "type": "toolUse"; [key: string]: unknown; } | { "isError"?: boolean; "parts": Array<ContentPart>; "toolUseId": string; "type": "toolResult"; [key: string]: unknown; } | { "providerMetadata"?: { [key: string]: { [key: string]: unknown; }; }; "text": string; "type": "reasoning"; [key: string]: unknown; });
/** Reasoning effort, in the vocabulary providers converge on. */
export type Effort = ("minimal" | "low" | "medium" | "high" | "xHigh" | "max");
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
export type Message = { "parts": Array<ContentPart>; "providerOptions"?: { [key: string]: { [key: string]: unknown; }; }; "role": Role; [key: string]: unknown; };
/** How a surface reads a row's target. */
export type Reads = ("path" | "command" | "text");
export type Role = ("user" | "assistant");
/** One persisted context: one journal, one actor. */
export type SessionId = string;
/** One cacheable segment of the system prompt. */
export type SystemBlock = { "cache"?: boolean; "text": string; [key: string]: unknown; };
/** The top-level input field a tool's row names. */
export type Target = { "field": string; "reads": Reads; [key: string]: unknown; };
/** How a tool reads on a surface; never sent to a model. */
export type ToolLook = { "ascii"?: (string | null); "mark": string; "target"?: (Target | null); "verb": string; [key: string]: unknown; };
/** A tool as the model sees it. */
export type ToolSpec = {
  "description": string;
  "inputSchema": unknown;
  "look"?: (ToolLook | null);
  /**
   * What a catalogue may show beside the tool (an MCP tool's `server`);
   * never sent to a model.
   */
  "meta"?: { [key: string]: unknown; };
  "name": string;
  [key: string]: unknown;
};
export type Payload = {
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
}
export type PromptCompose = PromptComposeInput.Payload;
export namespace SessionCompactInput {
// Generated from JSON Schema. Do not edit.
/** A typed action a client asks for (GUI buttons, hosts); the kernel dispatches it to a command. */
export type Action = { "args"?: unknown; "name": string; [key: string]: unknown; };
/** A button: the surface fires `Input::Action` with `action` (ADR-0008 §1). */
export type ActionItem = {
  "action": Action;
  /** A single-key hint; the surface may use another key. */
  "key"?: (string | null);
  "label": string;
  /** A one-line explanation supplied by the action owner. */
  "meaning"?: (string | null);
  [key: string]: unknown;
};
export type Align = ("flex-start" | "center" | "flex-end" | "stretch");
/**
 * Why a compaction was asked for. Serializable because a strategy may live in
 * another process: the reason crosses the bridge as it is.
 */
export type CompactReason = ({ "kind": "threshold"; [key: string]: unknown; } | { "kind": "overflow"; "message": string; [key: string]: unknown; } | { "instructions"?: (string | null); "kind": "manual"; [key: string]: unknown; });
export type Component = ("KeyBar" | "Composer" | "ModelPicker" | "Browse" | "Plugins" | "Palette" | "Sessions" | "Status" | "Question" | "UserMessage" | "AssistantMessage" | "ToolUse" | "ToolResult" | "ToolGroup" | "ToolProgress" | "CommandOutput" | "Spinner" | "TurnDuration" | "InfoNotice" | "SessionMode" | "PromptHint" | "AbovePrompt" | "Pane" | "Permission" | "SignIn" | "Reasoning" | "Compaction" | "Rewind" | "Interruption" | "QuestionAnswer" | "PermissionReceipt" | "Asset" | "ShellRun" | "Action" | "SessionStatus" | "Toast" | "Welcome" | "Ledger");
export type ComposerProps = { "placeholder": string; "tag": string; [key: string]: unknown; };
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
export type Direction = ("row" | "column");
/** Reasoning effort, in the vocabulary providers converge on. */
export type Effort = ("minimal" | "low" | "medium" | "high" | "xHigh" | "max");
/** The progress curve of a text fade. */
export type FadeCurve = ("easeOut");
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
/** A client-minted write id; the outcome arrives as `IntentAck`. */
export type IntentId = string;
/** A pending question or permission, answered once by id. */
export type InteractionId = string;
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
export type ItemBody = ({ "kind": "user"; "origin": Origin; "parts": Array<ContentPart>; [key: string]: unknown; } | { "kind": "assistant"; "text": string; [key: string]: unknown; } | { "kind": "reasoning"; "providerMetadata"?: { [key: string]: { [key: string]: unknown; }; }; "text": string; [key: string]: unknown; } | {
  /** The policy that answered without opening an ask. */
  "allowedBy"?: (Reason | null);
  /**
   * When the call's own ask opened. Its wait runs until `began_at`, or
   * until `completed_at` for a call that never ran.
   */
  "askedAt"?: (string | null);
  /** When the call began running. */
  "beganAt"?: (string | null);
  "callId": string;
  "durationMs"?: (number | null);
  "input": unknown;
  "kind": "toolCall";
  "name": string;
  "output"?: (ToolOutput | null);
  "progress"?: (string | null);
  "quiet"?: boolean;
  [key: string]: unknown;
} | { "command": string; "cwd": string; "exit"?: (number | null); "kind": "shell"; "output": string; [key: string]: unknown; } | { "args"?: unknown; "kind": "action"; "name": string; "result"?: unknown; [key: string]: unknown; } | { "after": number; "before": number; "durationMs": number; "kind": "compaction"; "replaced": number; "summary": string; [key: string]: unknown; } | { "dropped": number; "kind": "rewind"; "toTurn": TurnId; [key: string]: unknown; } | { "askedAt"?: (string | null); "kind": "interruption"; "marker": string; [key: string]: unknown; } | { "code": string; "kind": "notice"; "level": Level; "text": string; [key: string]: unknown; } | { "answer": string; "askedAt"?: (string | null); "interaction": InteractionId; "item"?: (ItemId | null); "kind": "questionAnswer"; "question": string; [key: string]: unknown; } | { "askedAt"?: (string | null); "decision": DecisionKind; "feedback"?: (string | null); "interaction": InteractionId; "item"?: (ItemId | null); "kind": "permissionReceipt"; "tool": string; [key: string]: unknown; } | { "asset": string; "kind": "asset"; "label"?: (string | null); [key: string]: unknown; } | { [key: string]: unknown; });
/** One unit in the transcript order. */
export type ItemId = string;
export type ItemStatus = ("pending" | "running" | "completed" | "failed" | "interrupted");
export type Justify = ("flex-start" | "center" | "flex-end" | "space-between" | "space-around" | "space-evenly");
export type KeyPair = { "action": string; "emphasis"?: boolean; "keys": string; [key: string]: unknown; };
export type Level = ("info" | "warn" | "error");
export type Message = { "parts": Array<ContentPart>; "providerOptions"?: { [key: string]: { [key: string]: unknown; }; }; "role": Role; [key: string]: unknown; };
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
/** A question owner's editing projection; sealed labels remain the host's. */
export type QuestionFocus = { "checked": number; "field": number; "writing": boolean; [key: string]: unknown; };
/** How a surface reads a row's target. */
export type Reads = ("path" | "command" | "text");
export type Reason = ({ "kind": "rule"; "rule": string; "source"?: (RuleSource | null); [key: string]: unknown; } | { "kind": "mode"; "mode": string; [key: string]: unknown; } | { "hook": string; "kind": "hook"; [key: string]: unknown; } | { "detail": string; "kind": "safety"; [key: string]: unknown; } | { "kind": "readOnly"; [key: string]: unknown; } | { "detail": string; "kind": "confirm"; [key: string]: unknown; } | { "kind": "default"; [key: string]: unknown; });
export type Role = ("user" | "assistant");
/** Where the matched permission rule was installed. */
export type RuleSource = ("settings" | "session");
export type SelectOption = {
  /** The text shown. */
  "label": string;
  /** The value chosen. */
  "value": string;
  [key: string]: unknown;
};
/** One persisted context: one journal, one actor. */
export type SessionId = string;
export type SiteHints = { "composer"?: (ComposerProps | null); "pairs"?: Array<KeyPair>; [key: string]: unknown; };
export type SurfaceName = ("terminal" | "desktop" | "text" | "chat");
/** One cacheable segment of the system prompt. */
export type SystemBlock = { "cache"?: boolean; "text": string; [key: string]: unknown; };
/** The top-level input field a tool's row names. */
export type Target = { "field": string; "reads": Reads; [key: string]: unknown; };
/** A text child preserves the style of its nested Text node. */
export type TextChild = (string | View);
/** Declarative motion on the surface clock, with a reduced-motion still form. */
export type TextMotion = ({ "depth": number; "kind": "breathe"; "periodMs": number; "with": string; [key: string]: unknown; } | { "kind": "spin"; [key: string]: unknown; } | { "kind": "shimmer"; [key: string]: unknown; } | { "atMs": number; "curve"?: (FadeCurve | null); "from": string; "kind": "fade"; "ms": number; [key: string]: unknown; } | { "atMs": number; "kind": "countdown"; "prefix": string; [key: string]: unknown; });
/** The one styling hook a plugin has; the surface owns the colour. */
export type Tone = (("neutral" | "good" | "bad") | "attention");
/** How a tool reads on a surface; never sent to a model. */
export type ToolLook = { "ascii"?: (string | null); "mark": string; "target"?: (Target | null); "verb": string; [key: string]: unknown; };
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
  "look"?: (ToolLook | null);
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
/** A call's result in one line: `+1 −1`, `3 passed`, `exit 1`. */
export type Upshot = ({ "added": number; "kind": "diff"; "removed": number; [key: string]: unknown; } | { "kind": "words"; "text": string; "tone": Tone; [key: string]: unknown; });
export type View = ({
  /** Draw the text bold. */
  "bold"?: (boolean | null);
  /** Strings and nested `Text` nodes, drawn in order. */
  "children": Array<TextChild>;
  /**
   * An Ember ink: fg, muted, dim, faint, ember, glow, coal, amber, amberGlow,
   * good, bad, plan, auto, code, crew0–crew5, kw, str, num, fn, ty; or `#rrggbb`.
   * Ground inks: bg, you, well, wellDeep, pick, pickBg, addTint, delTint,
   * addWord, delWord, badTint, amberTint, glowTint.
   * Legacy ash, presence and attention are accepted; unknown names draw plain.
   */
  "color"?: (string | null);
  /** Draw the text dimmed. */
  "dimColor"?: (boolean | null);
  /** An Ember ground, filled to the available width. */
  "ground"?: (string | null);
  /** Swap the foreground and background colours. */
  "inverse"?: (boolean | null);
  /** Draw the text in italics. */
  "italic"?: (boolean | null);
  "kind": "Text";
  /** A surface-clock animation; the author never needs to redraw it. */
  "motion"?: (TextMotion | null);
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
  /** An Ember ground, filled across the box including padding. */
  "ground"?: (string | null);
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
export type Payload = {
  /** Earlier failed attempts this round. */
  "failures": number;
  /** The history to compact. */
  "items": Array<Item>;
  /** Tokens of recent history to keep verbatim. */
  "keepBudget": number;
  /** Why compaction runs. */
  "reason": CompactReason;
  /** The request the next round would send. */
  "request": ModelRequest;
  /** The context reading that triggered it. */
  "usage": ContextUsage;
  [key: string]: unknown;
};
}
export namespace SessionCompactResult {
// Generated from JSON Schema. Do not edit.
/** A rejected summary still bills any usage observed before it failed. */
export type CompactError = { "error": KernelError; "usage"?: Usage; [key: string]: unknown; };
/**
 * The result: a summary, the boundary before which items are replaced, the
 * items before it to keep anyway, and what the summary cost. The kernel
 * accepts it only when `after < before`; the cost is billed either way.
 */
export type Compaction = {
  "after": number;
  "before": number;
  "boundary": ItemId;
  "kept"?: Array<ItemId>;
  "summary": string;
  /** What the summary cost. A strategy that spends no tokens says nothing. */
  "usage"?: Usage;
  [key: string]: unknown;
};
export type ErrorCode = (("SESSION_NOT_FOUND" | "SESSION_LOCKED" | "SESSION_CLOSED" | "INTERACTION_CLOSED" | "NOT_READY" | "STALE_GENERATION" | "NOT_INITIALIZED" | "INVALID_INPUT" | "PERMISSION_DENIED" | "TOOL_NOT_FOUND" | "TOOL_FAILED" | "PROVIDER_UNAVAILABLE" | "AUTH_REQUIRED" | "RATE_LIMITED" | "CONTEXT_OVERFLOW" | "TIMEOUT" | "OFFLINE" | "SERVER_ERROR" | "TURN_LOST" | "TURN_BUDGET_EXHAUSTED" | "UNSUPPORTED" | "INTERNAL") | "INTERRUPTED" | "PROTOCOL_LIMIT" | "NOT_FOUND" | "STORAGE");
/** One unit in the transcript order. */
export type ItemId = string;
/** An error as it appears on the wire and in the journal. */
export type KernelError = { "code": ErrorCode; "message": string; [key: string]: unknown; };
/**
 * Token counts as the provider reports them, the three input counts kept
 * apart: `input_tokens` is what the model read fresh, the cache counts are
 * what it read from or wrote to a cached prefix. The whole input side is
 * `input_total()`.
 */
export type Usage = { "cacheReadTokens"?: number; "cacheWriteTokens"?: number; "inputTokens": number; "outputTokens": number; "reasoningTokens"?: number; [key: string]: unknown; };
export type Payload = ({ "Ok": Compaction; [key: string]: unknown; } | { "Err": CompactError; [key: string]: unknown; });
}
export type SessionCompact = SessionCompactInput.Payload;
export namespace TurnStopInput {
// Generated from JSON Schema. Do not edit.
export type Payload = null;
}
export namespace TurnStopResult {
// Generated from JSON Schema. Do not edit.
export type Payload = ("complete" | { "continue": { "origin": string; "reason": string; [key: string]: unknown; }; });
}
export type TurnStop = TurnStopInput.Payload;
export namespace ToolDescribeInput {
// Generated from JSON Schema. Do not edit.
/** How a surface reads a row's target. */
export type Reads = ("path" | "command" | "text");
/** The top-level input field a tool's row names. */
export type Target = { "field": string; "reads": Reads; [key: string]: unknown; };
/** How a tool reads on a surface; never sent to a model. */
export type ToolLook = { "ascii"?: (string | null); "mark": string; "target"?: (Target | null); "verb": string; [key: string]: unknown; };
/** A tool as the model sees it. */
export type Payload = {
  "description": string;
  "inputSchema": unknown;
  "look"?: (ToolLook | null);
  /**
   * What a catalogue may show beside the tool (an MCP tool's `server`);
   * never sent to a model.
   */
  "meta"?: { [key: string]: unknown; };
  "name": string;
  [key: string]: unknown;
};
}
export namespace ToolDescribeResult {
// Generated from JSON Schema. Do not edit.
/** How a surface reads a row's target. */
export type Reads = ("path" | "command" | "text");
/** The top-level input field a tool's row names. */
export type Target = { "field": string; "reads": Reads; [key: string]: unknown; };
/** How a tool reads on a surface; never sent to a model. */
export type ToolLook = { "ascii"?: (string | null); "mark": string; "target"?: (Target | null); "verb": string; [key: string]: unknown; };
/** A tool as the model sees it. */
export type ToolSpec = {
  "description": string;
  "inputSchema": unknown;
  "look"?: (ToolLook | null);
  /**
   * What a catalogue may show beside the tool (an MCP tool's `server`);
   * never sent to a model.
   */
  "meta"?: { [key: string]: unknown; };
  "name": string;
  [key: string]: unknown;
};
export type Payload = (ToolSpec | null);
}
export type ToolDescribe = ToolDescribeInput.Payload;
export namespace PromptAutocompleteInput {
// Generated from JSON Schema. Do not edit.
export type CatalogEntry = { "id": string; "label": string; "meta"?: unknown; [key: string]: unknown; };
export type CatalogKind = ("events" | "models" | "providers" | "tools" | "commands" | "plugins");
export type Payload = { "entries": Array<CatalogEntry>; "kind": CatalogKind; [key: string]: unknown; };
}
export namespace PromptAutocompleteResult {
// Generated from JSON Schema. Do not edit.
export type CatalogEntry = { "id": string; "label": string; "meta"?: unknown; [key: string]: unknown; };
export type CatalogKind = ("events" | "models" | "providers" | "tools" | "commands" | "plugins");
export type Payload = { "entries": Array<CatalogEntry>; "kind": CatalogKind; [key: string]: unknown; };
}
export type PromptAutocomplete = PromptAutocompleteInput.Payload;
export namespace SessionSendInput {
// Generated from JSON Schema. Do not edit.
/** A typed action a client asks for (GUI buttons, hosts); the kernel dispatches it to a command. */
export type Action = { "args"?: unknown; "name": string; [key: string]: unknown; };
/**
 * How a line reaches a session's queue: a peer's message (ADR-0010 §1) and
 * a person's own line are the same question, so they are the same word.
 */
export type Delivery = ("wake" | "hold");
export type Headline = {
  /** Each part is one line of inline Markdown. */
  "parts": Array<string>;
  "tone": Tone;
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
  "headline"?: (Headline | null);
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
/** One persisted context: one journal, one actor. */
export type SessionId = string;
export type SurfaceName = ("terminal" | "desktop" | "text" | "chat");
/** The one styling hook a plugin has; the surface owns the colour. */
export type Tone = (("neutral" | "good" | "bad") | "attention");
/** How a producer intends an utterance to be drawn. */
export type Voice = ("said" | "note" | "quiet" | "aside");
export type Payload = {
  /** Whether it wakes the receiver or waits for its next turn. */
  "delivery": Delivery;
  /** What is sent. */
  "input": Input;
  /** The receiving session. */
  "to": SessionId;
  [key: string]: unknown;
};
}
export namespace SessionSendResult {
// Generated from JSON Schema. Do not edit.
export type Payload = null;
}
export type SessionSend = SessionSendInput.Payload;
export namespace SessionReceiveInput {
// Generated from JSON Schema. Do not edit.
/** A typed action a client asks for (GUI buttons, hosts); the kernel dispatches it to a command. */
export type Action = { "args"?: unknown; "name": string; [key: string]: unknown; };
/**
 * How a line reaches a session's queue: a peer's message (ADR-0010 §1) and
 * a person's own line are the same question, so they are the same word.
 */
export type Delivery = ("wake" | "hold");
export type Headline = {
  /** Each part is one line of inline Markdown. */
  "parts": Array<string>;
  "tone": Tone;
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
export type SurfaceName = ("terminal" | "desktop" | "text" | "chat");
/** The one styling hook a plugin has; the surface owns the colour. */
export type Tone = (("neutral" | "good" | "bad") | "attention");
/** How a producer intends an utterance to be drawn. */
export type Voice = ("said" | "note" | "quiet" | "aside");
/**
 * The one submission entry. The kernel parses `/`, `!` and `@` in text and
 * decides turn, queue, steer or deliver; a client never chooses.
 */
export type Payload = ({
  /**
   * Whether a running turn may be steered with it (ADR-0008 §2,
   * amended M68). `Wake` is the line every client has always sent, so
   * it is the default and nothing is written for it.
   */
  "delivery"?: Delivery;
  "headline"?: (Headline | null);
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
}
export namespace SessionReceiveResult {
// Generated from JSON Schema. Do not edit.
/** A typed action a client asks for (GUI buttons, hosts); the kernel dispatches it to a command. */
export type Action = { "args"?: unknown; "name": string; [key: string]: unknown; };
/**
 * How a line reaches a session's queue: a peer's message (ADR-0010 §1) and
 * a person's own line are the same question, so they are the same word.
 */
export type Delivery = ("wake" | "hold");
export type Headline = {
  /** Each part is one line of inline Markdown. */
  "parts": Array<string>;
  "tone": Tone;
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
export type SurfaceName = ("terminal" | "desktop" | "text" | "chat");
/** The one styling hook a plugin has; the surface owns the colour. */
export type Tone = (("neutral" | "good" | "bad") | "attention");
/** How a producer intends an utterance to be drawn. */
export type Voice = ("said" | "note" | "quiet" | "aside");
/**
 * The one submission entry. The kernel parses `/`, `!` and `@` in text and
 * decides turn, queue, steer or deliver; a client never chooses.
 */
export type Payload = ({
  /**
   * Whether a running turn may be steered with it (ADR-0008 §2,
   * amended M68). `Wake` is the line every client has always sent, so
   * it is the default and nothing is written for it.
   */
  "delivery"?: Delivery;
  "headline"?: (Headline | null);
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
}
export type SessionReceive = SessionReceiveInput.Payload;
export namespace SessionAppendInput {
// Generated from JSON Schema. Do not edit.
export type Payload = {
  /** The record's kind, named by its plugin. */
  "kind": string;
  /** The record itself. */
  "payload": unknown;
  /** The plugin that owns the record. */
  "plugin": string;
  [key: string]: unknown;
};
}
export namespace SessionAppendResult {
// Generated from JSON Schema. Do not edit.
export type Payload = {
  /** The record's kind, named by its plugin. */
  "kind": string;
  /** The record itself. */
  "payload": unknown;
  /** The plugin that owns the record. */
  "plugin": string;
  [key: string]: unknown;
};
}
export type SessionAppend = SessionAppendInput.Payload;
export namespace TurnCompleteInput {
// Generated from JSON Schema. Do not edit.
export type ErrorCode = (("SESSION_NOT_FOUND" | "SESSION_LOCKED" | "SESSION_CLOSED" | "INTERACTION_CLOSED" | "NOT_READY" | "STALE_GENERATION" | "NOT_INITIALIZED" | "INVALID_INPUT" | "PERMISSION_DENIED" | "TOOL_NOT_FOUND" | "TOOL_FAILED" | "PROVIDER_UNAVAILABLE" | "AUTH_REQUIRED" | "RATE_LIMITED" | "CONTEXT_OVERFLOW" | "TIMEOUT" | "OFFLINE" | "SERVER_ERROR" | "TURN_LOST" | "TURN_BUDGET_EXHAUSTED" | "UNSUPPORTED" | "INTERNAL") | "INTERRUPTED" | "PROTOCOL_LIMIT" | "NOT_FOUND" | "STORAGE");
export type InterruptReason = ("userCancel" | "newInput" | "shutdown" | "budget");
/** An error as it appears on the wire and in the journal. */
export type KernelError = { "code": ErrorCode; "message": string; [key: string]: unknown; };
/** One run of the turn loop, closed exactly once. */
export type TurnId = string;
export type TurnStatus = ({ "kind": "completed"; [key: string]: unknown; } | { "error": KernelError; "kind": "failed"; [key: string]: unknown; } | { "kind": "interrupted"; "reason": InterruptReason; [key: string]: unknown; });
/**
 * Token counts as the provider reports them, the three input counts kept
 * apart: `input_tokens` is what the model read fresh, the cache counts are
 * what it read from or wrote to a cached prefix. The whole input side is
 * `input_total()`.
 */
export type Usage = { "cacheReadTokens"?: number; "cacheWriteTokens"?: number; "inputTokens": number; "outputTokens": number; "reasoningTokens"?: number; [key: string]: unknown; };
export type Payload = {
  /** The assistant's final text. */
  "answer": string;
  /** How it ended. */
  "outcome": TurnStatus;
  /** The turn that ended. */
  "turn": TurnId;
  /** Tokens the turn used. */
  "usage": Usage;
  [key: string]: unknown;
};
}
export namespace TurnCompleteResult {
// Generated from JSON Schema. Do not edit.
export type Payload = (string | null);
}
export type TurnComplete = TurnCompleteInput.Payload;
export namespace CommandRunInput {
// Generated from JSON Schema. Do not edit.
export type SurfaceName = ("terminal" | "desktop" | "text" | "chat");
export type Payload = {
  /** The rest of the line after the name. */
  "args": string;
  /** The command's name, without the slash. */
  "command": string;
  /** The surface the command was typed on, when known. */
  "surface"?: (SurfaceName | null);
  [key: string]: unknown;
};
}
export namespace CommandRunResult {
// Generated from JSON Schema. Do not edit.
/** A typed action a client asks for (GUI buttons, hosts); the kernel dispatches it to a command. */
export type Action = { "args"?: unknown; "name": string; [key: string]: unknown; };
/** A button: the surface fires `Input::Action` with `action` (ADR-0008 §1). */
export type ActionItem = {
  "action": Action;
  /** A single-key hint; the surface may use another key. */
  "key"?: (string | null);
  "label": string;
  /** A one-line explanation supplied by the action owner. */
  "meaning"?: (string | null);
  [key: string]: unknown;
};
export type Align = ("flex-start" | "center" | "flex-end" | "stretch");
export type Answer = ({ "input": unknown; "kind": "allowChanged"; [key: string]: unknown; } | { "kind": "allowOnce"; [key: string]: unknown; } | { "kind": "allowSession"; "scope": string; [key: string]: unknown; } | { "feedback"?: (string | null); "kind": "deny"; [key: string]: unknown; } | { "ids": Array<string>; "kind": "choice"; "other"?: (string | null); [key: string]: unknown; } | { "kind": "text"; "text": string; [key: string]: unknown; } | { "kind": "confirm"; [key: string]: unknown; } | { "kind": "cancel"; [key: string]: unknown; } | { "answers": Array<Answer>; "kind": "form"; [key: string]: unknown; });
export type CommandOutcome = ({ "kind": "acknowledged"; "result": unknown; [key: string]: unknown; } | { "action": Action; "fallback"?: (string | null); "kind": "surface"; [key: string]: unknown; } | { "answer": Answer; "kind": "answer"; [key: string]: unknown; } | { "handled": boolean; "kind": "handled"; [key: string]: unknown; } | { "kind": "applied"; "message"?: (string | null); [key: string]: unknown; } | { "kind": "view"; "view": View; [key: string]: unknown; } | { "kind": "prompt"; "text": string; [key: string]: unknown; } | { "body": ItemBody; "kind": "record"; [key: string]: unknown; });
export type Component = ("KeyBar" | "Composer" | "ModelPicker" | "Browse" | "Plugins" | "Palette" | "Sessions" | "Status" | "Question" | "UserMessage" | "AssistantMessage" | "ToolUse" | "ToolResult" | "ToolGroup" | "ToolProgress" | "CommandOutput" | "Spinner" | "TurnDuration" | "InfoNotice" | "SessionMode" | "PromptHint" | "AbovePrompt" | "Pane" | "Permission" | "SignIn" | "Reasoning" | "Compaction" | "Rewind" | "Interruption" | "QuestionAnswer" | "PermissionReceipt" | "Asset" | "ShellRun" | "Action" | "SessionStatus" | "Toast" | "Welcome" | "Ledger");
export type ComposerProps = { "placeholder": string; "tag": string; [key: string]: unknown; };
export type ContentPart = ({ "text": string; "type": "text"; [key: string]: unknown; } | Image | { "id": string; "input": unknown; "name": string; "type": "toolUse"; [key: string]: unknown; } | { "isError"?: boolean; "parts": Array<ContentPart>; "toolUseId": string; "type": "toolResult"; [key: string]: unknown; } | { "providerMetadata"?: { [key: string]: { [key: string]: unknown; }; }; "text": string; "type": "reasoning"; [key: string]: unknown; });
export type DecisionKind = ("allowChanged" | "allowChangedSession" | "allow" | "allowSession" | "deny");
export type Direction = ("row" | "column");
/** The progress curve of a text fade. */
export type FadeCurve = ("easeOut");
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
/** A pending question or permission, answered once by id. */
export type InteractionId = string;
export type ItemBody = ({ "kind": "user"; "origin": Origin; "parts": Array<ContentPart>; [key: string]: unknown; } | { "kind": "assistant"; "text": string; [key: string]: unknown; } | { "kind": "reasoning"; "providerMetadata"?: { [key: string]: { [key: string]: unknown; }; }; "text": string; [key: string]: unknown; } | {
  /** The policy that answered without opening an ask. */
  "allowedBy"?: (Reason | null);
  /**
   * When the call's own ask opened. Its wait runs until `began_at`, or
   * until `completed_at` for a call that never ran.
   */
  "askedAt"?: (string | null);
  /** When the call began running. */
  "beganAt"?: (string | null);
  "callId": string;
  "durationMs"?: (number | null);
  "input": unknown;
  "kind": "toolCall";
  "name": string;
  "output"?: (ToolOutput | null);
  "progress"?: (string | null);
  "quiet"?: boolean;
  [key: string]: unknown;
} | { "command": string; "cwd": string; "exit"?: (number | null); "kind": "shell"; "output": string; [key: string]: unknown; } | { "args"?: unknown; "kind": "action"; "name": string; "result"?: unknown; [key: string]: unknown; } | { "after": number; "before": number; "durationMs": number; "kind": "compaction"; "replaced": number; "summary": string; [key: string]: unknown; } | { "dropped": number; "kind": "rewind"; "toTurn": TurnId; [key: string]: unknown; } | { "askedAt"?: (string | null); "kind": "interruption"; "marker": string; [key: string]: unknown; } | { "code": string; "kind": "notice"; "level": Level; "text": string; [key: string]: unknown; } | { "answer": string; "askedAt"?: (string | null); "interaction": InteractionId; "item"?: (ItemId | null); "kind": "questionAnswer"; "question": string; [key: string]: unknown; } | { "askedAt"?: (string | null); "decision": DecisionKind; "feedback"?: (string | null); "interaction": InteractionId; "item"?: (ItemId | null); "kind": "permissionReceipt"; "tool": string; [key: string]: unknown; } | { "asset": string; "kind": "asset"; "label"?: (string | null); [key: string]: unknown; } | { [key: string]: unknown; });
/** One unit in the transcript order. */
export type ItemId = string;
export type Justify = ("flex-start" | "center" | "flex-end" | "space-between" | "space-around" | "space-evenly");
export type KeyPair = { "action": string; "emphasis"?: boolean; "keys": string; [key: string]: unknown; };
export type Level = ("info" | "warn" | "error");
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
/** A question owner's editing projection; sealed labels remain the host's. */
export type QuestionFocus = { "checked": number; "field": number; "writing": boolean; [key: string]: unknown; };
export type Reason = ({ "kind": "rule"; "rule": string; "source"?: (RuleSource | null); [key: string]: unknown; } | { "kind": "mode"; "mode": string; [key: string]: unknown; } | { "hook": string; "kind": "hook"; [key: string]: unknown; } | { "detail": string; "kind": "safety"; [key: string]: unknown; } | { "kind": "readOnly"; [key: string]: unknown; } | { "detail": string; "kind": "confirm"; [key: string]: unknown; } | { "kind": "default"; [key: string]: unknown; });
/** Where the matched permission rule was installed. */
export type RuleSource = ("settings" | "session");
export type SelectOption = {
  /** The text shown. */
  "label": string;
  /** The value chosen. */
  "value": string;
  [key: string]: unknown;
};
export type SiteHints = { "composer"?: (ComposerProps | null); "pairs"?: Array<KeyPair>; [key: string]: unknown; };
export type SurfaceName = ("terminal" | "desktop" | "text" | "chat");
/** A text child preserves the style of its nested Text node. */
export type TextChild = (string | View);
/** Declarative motion on the surface clock, with a reduced-motion still form. */
export type TextMotion = ({ "depth": number; "kind": "breathe"; "periodMs": number; "with": string; [key: string]: unknown; } | { "kind": "spin"; [key: string]: unknown; } | { "kind": "shimmer"; [key: string]: unknown; } | { "atMs": number; "curve"?: (FadeCurve | null); "from": string; "kind": "fade"; "ms": number; [key: string]: unknown; } | { "atMs": number; "kind": "countdown"; "prefix": string; [key: string]: unknown; });
/** The one styling hook a plugin has; the surface owns the colour. */
export type Tone = (("neutral" | "good" | "bad") | "attention");
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
export type TreeNode = { "badge"?: (string | null); "children"?: Array<TreeNode>; "label": string; "tone"?: Tone; [key: string]: unknown; };
/** One run of the turn loop, closed exactly once. */
export type TurnId = string;
/** A call's result in one line: `+1 −1`, `3 passed`, `exit 1`. */
export type Upshot = ({ "added": number; "kind": "diff"; "removed": number; [key: string]: unknown; } | { "kind": "words"; "text": string; "tone": Tone; [key: string]: unknown; });
export type View = ({
  /** Draw the text bold. */
  "bold"?: (boolean | null);
  /** Strings and nested `Text` nodes, drawn in order. */
  "children": Array<TextChild>;
  /**
   * An Ember ink: fg, muted, dim, faint, ember, glow, coal, amber, amberGlow,
   * good, bad, plan, auto, code, crew0–crew5, kw, str, num, fn, ty; or `#rrggbb`.
   * Ground inks: bg, you, well, wellDeep, pick, pickBg, addTint, delTint,
   * addWord, delWord, badTint, amberTint, glowTint.
   * Legacy ash, presence and attention are accepted; unknown names draw plain.
   */
  "color"?: (string | null);
  /** Draw the text dimmed. */
  "dimColor"?: (boolean | null);
  /** An Ember ground, filled to the available width. */
  "ground"?: (string | null);
  /** Swap the foreground and background colours. */
  "inverse"?: (boolean | null);
  /** Draw the text in italics. */
  "italic"?: (boolean | null);
  "kind": "Text";
  /** A surface-clock animation; the author never needs to redraw it. */
  "motion"?: (TextMotion | null);
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
  /** An Ember ground, filled across the box including padding. */
  "ground"?: (string | null);
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
export type Payload = (CommandOutcome | { "text": string; [key: string]: unknown; });
}
export type CommandRun = CommandRunInput.Payload;
export namespace CommandDescribeInput {
// Generated from JSON Schema. Do not edit.
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
/**
 * A chord a command declares for itself. A bare chord runs the command
 * alone; one with `args` submits the command with them, so a shortcut for
 * `/model next` is a binding on `model` and not a second catalogue entry.
 */
export type KeyBinding = (string | { "args": string; "chord": string; [key: string]: unknown; });
export type Payload = {
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
}
export namespace CommandDescribeResult {
// Generated from JSON Schema. Do not edit.
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
/**
 * A chord a command declares for itself. A bare chord runs the command
 * alone; one with `args` submits the command with them, so a shortcut for
 * `/model next` is a binding on `model` and not a second catalogue entry.
 */
export type KeyBinding = (string | { "args": string; "chord": string; [key: string]: unknown; });
export type Payload = (CommandSpec | null);
}
export type CommandDescribe = CommandDescribeInput.Payload;
export namespace ConfigSetInput {
// Generated from JSON Schema. Do not edit.
export type Payload = {
  /** The setting's dotted key. */
  "key": string;
  /** Its new value. */
  "value": unknown;
  [key: string]: unknown;
};
}
export namespace ConfigSetResult {
// Generated from JSON Schema. Do not edit.
export type Payload = {
  /** The setting's dotted key. */
  "key": string;
  /** Its new value. */
  "value": unknown;
  [key: string]: unknown;
};
}
export type ConfigSet = ConfigSetInput.Payload;
export namespace ConfigDescribeInput {
// Generated from JSON Schema. Do not edit.
/** Configuration as a client sees it: the kernel's keys and each plugin's claimed slice. */
export type Payload = { "kernel"?: unknown; "plugins"?: { [key: string]: unknown; }; [key: string]: unknown; };
}
export namespace ConfigDescribeResult {
// Generated from JSON Schema. Do not edit.
/** Configuration as a client sees it: the kernel's keys and each plugin's claimed slice. */
export type Payload = { "kernel"?: unknown; "plugins"?: { [key: string]: unknown; }; [key: string]: unknown; };
}
export type ConfigDescribe = ConfigDescribeInput.Payload;
export namespace TurnStartInput {
// Generated from JSON Schema. Do not edit.
/** One unit in the transcript order. */
export type ItemId = string;
/** One run of the turn loop, closed exactly once. */
export type TurnId = string;
export type TurnOrigin = ("submit" | "queue" | "peer" | "auto");
export type Payload = {
  /** The history items it took as input. */
  "inputs": Array<ItemId>;
  /** What opened it. */
  "origin": TurnOrigin;
  /** The turn that opened. */
  "turn": TurnId;
  [key: string]: unknown;
};
}
export namespace TurnStartResult {
// Generated from JSON Schema. Do not edit.
export type Payload = null;
}
export type TurnStart = TurnStartInput.Payload;
export namespace SessionMeasureInput {
// Generated from JSON Schema. Do not edit.
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
/** One run of the turn loop, closed exactly once. */
export type TurnId = string;
/**
 * Token counts as the provider reports them, the three input counts kept
 * apart: `input_tokens` is what the model read fresh, the cache counts are
 * what it read from or wrote to a cached prefix. The whole input side is
 * `input_total()`.
 */
export type Usage = { "cacheReadTokens"?: number; "cacheWriteTokens"?: number; "inputTokens": number; "outputTokens": number; "reasoningTokens"?: number; [key: string]: unknown; };
export type Payload = {
  /** The context window reading after this round. */
  "context": ContextUsage;
  /** The turn being measured. */
  "turn": TurnId;
  /** Tokens used so far. */
  "usage": Usage;
  [key: string]: unknown;
};
}
export namespace SessionMeasureResult {
// Generated from JSON Schema. Do not edit.
export type Payload = null;
}
export type SessionMeasure = SessionMeasureInput.Payload;
export namespace SessionStartInput {
// Generated from JSON Schema. Do not edit.
/** One persisted context: one journal, one actor. */
export type SessionId = string;
export type Payload = {
  /** The session this concerns; `$` methods default it to the current one. */
  "session": SessionId;
  [key: string]: unknown;
};
}
export namespace SessionStartResult {
// Generated from JSON Schema. Do not edit.
export type Payload = null;
}
export type SessionStart = SessionStartInput.Payload;
export namespace SessionEndInput {
// Generated from JSON Schema. Do not edit.
/** One persisted context: one journal, one actor. */
export type SessionId = string;
export type Payload = {
  /** The session this concerns; `$` methods default it to the current one. */
  "session": SessionId;
  [key: string]: unknown;
};
}
export namespace SessionEndResult {
// Generated from JSON Schema. Do not edit.
export type Payload = null;
}
export type SessionEnd = SessionEndInput.Payload;
export namespace SessionAttachInput {
// Generated from JSON Schema. Do not edit.
export type ClientIdentity = { "name": string; "surface": string; [key: string]: unknown; };
/** One persisted context: one journal, one actor. */
export type SessionId = string;
export type Payload = {
  /** The client that attached or detached. */
  "client": ClientIdentity;
  /** The session. */
  "session": SessionId;
  [key: string]: unknown;
};
}
export namespace SessionAttachResult {
// Generated from JSON Schema. Do not edit.
export type Payload = null;
}
export type SessionAttach = SessionAttachInput.Payload;
export namespace SessionDetachInput {
// Generated from JSON Schema. Do not edit.
export type ClientIdentity = { "name": string; "surface": string; [key: string]: unknown; };
/** One persisted context: one journal, one actor. */
export type SessionId = string;
export type Payload = {
  /** The client that attached or detached. */
  "client": ClientIdentity;
  /** The session. */
  "session": SessionId;
  [key: string]: unknown;
};
}
export namespace SessionDetachResult {
// Generated from JSON Schema. Do not edit.
export type Payload = null;
}
export type SessionDetach = SessionDetachInput.Payload;
export type Turn = TurnComplete;
export interface EventInputs {
  "ui.press": UiPressInput.Payload;
  "ui.input": UiInputInput.Payload;
  "ui.select": UiSelectInput.Payload;
  "ui.render": UiRenderInput.Payload;
  "plugin.register": PluginRegisterInput.Payload;
  "prompt.submit": PromptSubmitInput.Payload;
  "tool.check": ToolCheckInput.Payload;
  "tool.call": ToolCallInput.Payload;
  "model.complete": ModelCompleteInput.Payload;
  "turn.step": TurnStepInput.Payload;
  "prompt.compose": PromptComposeInput.Payload;
  "session.compact": SessionCompactInput.Payload;
  "turn.stop": TurnStopInput.Payload;
  "tool.describe": ToolDescribeInput.Payload;
  "prompt.autocomplete": PromptAutocompleteInput.Payload;
  "session.send": SessionSendInput.Payload;
  "session.receive": SessionReceiveInput.Payload;
  "session.append": SessionAppendInput.Payload;
  "turn.complete": TurnCompleteInput.Payload;
  "command.run": CommandRunInput.Payload;
  "command.describe": CommandDescribeInput.Payload;
  "config.set": ConfigSetInput.Payload;
  "config.describe": ConfigDescribeInput.Payload;
  "turn.start": TurnStartInput.Payload;
  "session.measure": SessionMeasureInput.Payload;
  "session.start": SessionStartInput.Payload;
  "session.end": SessionEndInput.Payload;
  "session.attach": SessionAttachInput.Payload;
  "session.detach": SessionDetachInput.Payload;
}
export interface EventResults {
  "ui.press": UiPressResult.Payload;
  "ui.input": UiInputResult.Payload;
  "ui.select": UiSelectResult.Payload;
  "ui.render": UiRenderResult.Payload;
  "plugin.register": PluginRegisterResult.Payload;
  "prompt.submit": PromptSubmitResult.Payload;
  "tool.check": ToolCheckResult.Payload;
  "tool.call": ToolCallResult.Payload;
  "model.complete": ModelCompleteResult.Payload;
  "turn.step": TurnStepResult.Payload;
  "prompt.compose": PromptComposeResult.Payload;
  "session.compact": SessionCompactResult.Payload;
  "turn.stop": TurnStopResult.Payload;
  "tool.describe": ToolDescribeResult.Payload;
  "prompt.autocomplete": PromptAutocompleteResult.Payload;
  "session.send": SessionSendResult.Payload;
  "session.receive": SessionReceiveResult.Payload;
  "session.append": SessionAppendResult.Payload;
  "turn.complete": TurnCompleteResult.Payload;
  "command.run": CommandRunResult.Payload;
  "command.describe": CommandDescribeResult.Payload;
  "config.set": ConfigSetResult.Payload;
  "config.describe": ConfigDescribeResult.Payload;
  "turn.start": TurnStartResult.Payload;
  "session.measure": SessionMeasureResult.Payload;
  "session.start": SessionStartResult.Payload;
  "session.end": SessionEndResult.Payload;
  "session.attach": SessionAttachResult.Payload;
  "session.detach": SessionDetachResult.Payload;
}
