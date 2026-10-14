// Generated from schema/events.json. Do not edit.
export namespace UiPressInput {
// Generated from JSON Schema. Do not edit.
export type Payload = { "event": string; "handler": string; "value"?: unknown; [key: string]: unknown; };
}
export namespace UiPressResult {
// Generated from JSON Schema. Do not edit.
export type Payload = null;
}
export type UiPress = UiPressInput.Payload;
export namespace UiInputInput {
// Generated from JSON Schema. Do not edit.
export type Payload = { "event": string; "handler": string; "value"?: unknown; [key: string]: unknown; };
}
export namespace UiInputResult {
// Generated from JSON Schema. Do not edit.
export type Payload = null;
}
export type UiInput = UiInputInput.Payload;
export namespace UiSelectInput {
// Generated from JSON Schema. Do not edit.
export type Payload = { "event": string; "handler": string; "value"?: unknown; [key: string]: unknown; };
}
export namespace UiSelectResult {
// Generated from JSON Schema. Do not edit.
export type Payload = null;
}
export type UiSelect = UiSelectInput.Payload;
export namespace UiRenderInput {
// Generated from JSON Schema. Do not edit.
export type Anchor = ("topLeft" | "topRight" | "bottomLeft" | "bottomRight" | "center");
export type Component = ("Question" | "UserMessage" | "AssistantMessage" | "ToolUse" | "ToolResult" | "ToolGroup" | "ToolProgress" | "CommandOutput" | "Spinner" | "TurnDuration" | "InfoNotice" | "SessionMode" | "PromptHint" | "AbovePrompt" | "Pane" | "Permission" | "SignIn" | "Reasoning" | "Compaction" | "Rewind" | "Interruption" | "QuestionAnswer" | "PermissionReceipt" | "Asset" | "ShellRun" | "Action" | "TurnProgress" | "SessionStatus" | "Toast" | "Welcome");
export type Length = (number | string);
export type PanePlace = (("above" | "side" | "overlay") | { "rect": Rect; });
export type Rect = { "anchor"?: Anchor; "height": Length; "width": Length; "x"?: Length; "y"?: Length; [key: string]: unknown; };
export type SurfaceName = ("terminal" | "desktop" | "text");
export type Viewport = { "columns": number; "place"?: (PanePlace | null); "rows": number; [key: string]: unknown; };
export type Payload = { "component": Component; "props": unknown; "requestId": string; "surface": SurfaceName; "viewport": Viewport; [key: string]: unknown; };
}
export namespace UiRenderResult {
// Generated from JSON Schema. Do not edit.
export type Action = { "args"?: unknown; "name": string; [key: string]: unknown; };
export type ActionItem = { "action": Action; "key"?: (string | null); "label": string; [key: string]: unknown; };
export type Align = ("flex-start" | "center" | "flex-end" | "stretch");
export type Component = ("Question" | "UserMessage" | "AssistantMessage" | "ToolUse" | "ToolResult" | "ToolGroup" | "ToolProgress" | "CommandOutput" | "Spinner" | "TurnDuration" | "InfoNotice" | "SessionMode" | "PromptHint" | "AbovePrompt" | "Pane" | "Permission" | "SignIn" | "Reasoning" | "Compaction" | "Rewind" | "Interruption" | "QuestionAnswer" | "PermissionReceipt" | "Asset" | "ShellRun" | "Action" | "TurnProgress" | "SessionStatus" | "Toast" | "Welcome");
export type Direction = ("row" | "column");
export type Justify = ("flex-start" | "center" | "flex-end" | "space-between" | "space-around" | "space-evenly");
export type SelectOption = { "label": string; "value": string; [key: string]: unknown; };
export type TextChild = (string | unknown);
export type Tone = (("neutral" | "good" | "bad") | "attention");
export type TreeNode = { "badge"?: (string | null); "children"?: Array<TreeNode>; "label": string; "tone"?: Tone; [key: string]: unknown; };
export type Wrap = ("wrap" | "nowrap");
export type Payload = ({ "bold"?: (boolean | null); "children": Array<TextChild>; "color"?: (string | null); "dimColor"?: (boolean | null); "inverse"?: (boolean | null); "italic"?: (boolean | null); "kind": "Text"; "strikethrough"?: (boolean | null); "underline"?: (boolean | null); "wrap"?: (string | null); [key: string]: unknown; } | { "dimColor"?: boolean; "kind": "Markdown"; "text": string; [key: string]: unknown; } | { "kind": "Code"; "language"?: (string | null); "source": string; [key: string]: unknown; } | { "kind": "diff"; "unified": string; [key: string]: unknown; } | { "items": Array<string>; "kind": "list"; [key: string]: unknown; } | { "headers": Array<string>; "kind": "table"; "rows": Array<Array<string>>; [key: string]: unknown; } | { "kind": "keyValue"; "rows": Array<[string, string, ...Array<unknown>]>; [key: string]: unknown; } | { "kind": "progress"; "label"?: (string | null); "total"?: (number | null); "value": number; [key: string]: unknown; } | { "kind": "badge"; "text": string; "tone"?: Tone; [key: string]: unknown; } | { "kind": "tree"; "nodes": Array<TreeNode>; [key: string]: unknown; } | { "alignItems"?: (Align | null); "borderColor"?: (string | null); "borderStyle"?: (string | null); "children": Array<unknown>; "display"?: (string | null); "flexDirection"?: (Direction | null); "flexGrow"?: (number | null); "flexWrap"?: (Wrap | null); "gap"?: (number | null); "height"?: (number | null); "justifyContent"?: (Justify | null); "key"?: (string | null); "kind": "Box"; "margin"?: (number | null); "marginX"?: (number | null); "marginY"?: (number | null); "padding"?: (number | null); "paddingX"?: (number | null); "paddingY"?: (number | null); "width"?: (number | null); [key: string]: unknown; } | { "child": unknown; "kind": "panel"; "title": string; [key: string]: unknown; } | { "items": Array<ActionItem>; "kind": "actions"; [key: string]: unknown; } | { "children"?: Array<TextChild>; "dimColor"?: (boolean | null); "hotkey"?: (string | null); "key"?: (string | null); "kind": "Button"; "label"?: (string | null); "onPress"?: (Action | null); "variant"?: (string | null); [key: string]: unknown; } | { "key"?: (string | null); "kind": "Input"; "label"?: (string | null); "onInput"?: (Action | null); "onSubmit"?: (Action | null); "placeholder"?: (string | null); "value"?: (string | null); [key: string]: unknown; } | { "key"?: (string | null); "kind": "Select"; "label"?: (string | null); "onSelect"?: (Action | null); "options"?: (Array<SelectOption> | null); "value"?: (string | null); [key: string]: unknown; } | { "href": string; "kind": "Link"; "label"?: (string | null); [key: string]: unknown; } | { "alt": string; "columns"?: (number | null); "kind": "Image"; "rows"?: (number | null); "source": string; [key: string]: unknown; } | { "cells": Array<string>; "columns": number; "kind": "Raster"; "rows": number; [key: string]: unknown; } | { "alt": string; "height"?: (number | null); "kind": "Svg"; "source": string; "width"?: (number | null); [key: string]: unknown; } | { "component": Component; "kind": "Engine"; "props": unknown; [key: string]: unknown; } | { "customKind": string; "data": unknown; "fold": string; "kind": "custom"; [key: string]: unknown; });
}
export type UiRender = UiRenderInput.Payload;
export namespace PluginRegisterInput {
// Generated from JSON Schema. Do not edit.
export type PluginOrigin = ("user" | "project" | "explicit");
export type PluginRuntime = ("process" | "module");
export type Tier = ("tighten" | "rewrite" | "full");
export type Payload = { "name": string; "origin": PluginOrigin; "runtime": PluginRuntime; "tier": Tier; "uses": Array<string>; "version": string; [key: string]: unknown; };
}
export namespace PluginRegisterResult {
// Generated from JSON Schema. Do not edit.
export type PluginGrant = { "tier": Tier; "uses": Array<string>; [key: string]: unknown; };
export type Tier = ("tighten" | "rewrite" | "full");
export type Payload = (PluginGrant | { "reason": string; [key: string]: unknown; });
}
export type PluginRegister = PluginRegisterInput.Payload;
export namespace PromptSubmitInput {
// Generated from JSON Schema. Do not edit.
export type Action = { "args"?: unknown; "name": string; [key: string]: unknown; };
export type Delivery = ("wake" | "hold");
export type Image = { "data": string; "height"?: (number | null); "mediaType": string; "path"?: (string | null); "width"?: (number | null); [key: string]: unknown; };
export type Origin = { "conversation"?: (string | null); "principal"?: (string | null); "surface": string; "voice"?: (Voice | null); [key: string]: unknown; };
export type Voice = ("said" | "note" | "quiet" | "aside");
export type Payload = ({ "delivery"?: Delivery; "images"?: Array<Image>; "kind": "text"; "origin": Origin; "text": string; [key: string]: unknown; } | { "action": Action; "kind": "action"; [key: string]: unknown; });
}
export namespace PromptSubmitResult {
// Generated from JSON Schema. Do not edit.
export type Action = { "args"?: unknown; "name": string; [key: string]: unknown; };
export type Delivery = ("wake" | "hold");
export type ErrorCode = (("SESSION_NOT_FOUND" | "SESSION_LOCKED" | "SESSION_CLOSED" | "INTERACTION_CLOSED" | "NOT_READY" | "STALE_GENERATION" | "NOT_INITIALIZED" | "INVALID_INPUT" | "PERMISSION_DENIED" | "TOOL_NOT_FOUND" | "TOOL_FAILED" | "PROVIDER_UNAVAILABLE" | "AUTH_REQUIRED" | "RATE_LIMITED" | "CONTEXT_OVERFLOW" | "TIMEOUT" | "OFFLINE" | "SERVER_ERROR" | "TURN_LOST" | "TURN_BUDGET_EXHAUSTED" | "UNSUPPORTED" | "INTERNAL") | "INTERRUPTED" | "PROTOCOL_LIMIT" | "NOT_FOUND" | "STORAGE");
export type Image = { "data": string; "height"?: (number | null); "mediaType": string; "path"?: (string | null); "width"?: (number | null); [key: string]: unknown; };
export type Input = ({ "delivery"?: Delivery; "images"?: Array<Image>; "kind": "text"; "origin": Origin; "text": string; [key: string]: unknown; } | { "action": Action; "kind": "action"; [key: string]: unknown; });
export type KernelError = { "code": ErrorCode; "message": string; [key: string]: unknown; };
export type Origin = { "conversation"?: (string | null); "principal"?: (string | null); "surface": string; "voice"?: (Voice | null); [key: string]: unknown; };
export type SessionId = string;
export type Voice = ("said" | "note" | "quiet" | "aside");
export type Payload = ({ "accepted": Input; } | { "rejected": KernelError; } | { "redirect": { "input": Input; "session": SessionId; [key: string]: unknown; }; });
}
export type PromptSubmit = PromptSubmitInput.Payload;
export namespace ToolCheckInput {
// Generated from JSON Schema. Do not edit.
export type Payload = { "tool": string; "tool_use_id": string; [key: string]: unknown; };
}
export namespace ToolCheckResult {
// Generated from JSON Schema. Do not edit.
export type Decision = ({ "kind": "allow"; "reason": Reason; [key: string]: unknown; } | { "kind": "deny"; "reason": Reason; [key: string]: unknown; } | { "kind": "ask"; "reason": Reason; "scope"?: (string | null); [key: string]: unknown; });
export type Reason = ({ "kind": "rule"; "rule": string; [key: string]: unknown; } | { "kind": "mode"; "mode": string; [key: string]: unknown; } | { "hook": string; "kind": "hook"; [key: string]: unknown; } | { "detail": string; "kind": "safety"; [key: string]: unknown; } | { "kind": "readOnly"; [key: string]: unknown; } | { "detail": string; "kind": "confirm"; [key: string]: unknown; } | { "kind": "default"; [key: string]: unknown; });
export type ToolCall = { "callId": string; "input": unknown; "name": string; [key: string]: unknown; };
export type Payload = { "call": ToolCall; "decision": Decision; [key: string]: unknown; };
}
export type ToolCheck = ToolCheckInput.Payload;
export namespace ToolCallInput {
// Generated from JSON Schema. Do not edit.
export type Payload = { "tool": string; "tool_use_id": string; [key: string]: unknown; };
}
export namespace ToolCallResult {
// Generated from JSON Schema. Do not edit.
export type Action = { "args"?: unknown; "name": string; [key: string]: unknown; };
export type ActionItem = { "action": Action; "key"?: (string | null); "label": string; [key: string]: unknown; };
export type Align = ("flex-start" | "center" | "flex-end" | "stretch");
export type Component = ("Question" | "UserMessage" | "AssistantMessage" | "ToolUse" | "ToolResult" | "ToolGroup" | "ToolProgress" | "CommandOutput" | "Spinner" | "TurnDuration" | "InfoNotice" | "SessionMode" | "PromptHint" | "AbovePrompt" | "Pane" | "Permission" | "SignIn" | "Reasoning" | "Compaction" | "Rewind" | "Interruption" | "QuestionAnswer" | "PermissionReceipt" | "Asset" | "ShellRun" | "Action" | "TurnProgress" | "SessionStatus" | "Toast" | "Welcome");
export type ContentPart = ({ "text": string; "type": "text"; [key: string]: unknown; } | Image | { "id": string; "input": unknown; "name": string; "type": "toolUse"; [key: string]: unknown; } | { "isError"?: boolean; "parts": Array<ContentPart>; "toolUseId": string; "type": "toolResult"; [key: string]: unknown; } | { "providerMetadata"?: { [key: string]: { [key: string]: unknown; }; }; "text": string; "type": "reasoning"; [key: string]: unknown; });
export type Direction = ("row" | "column");
export type Image = { "data": string; "height"?: (number | null); "mediaType": string; "path"?: (string | null); "width"?: (number | null); [key: string]: unknown; };
export type Justify = ("flex-start" | "center" | "flex-end" | "space-between" | "space-around" | "space-evenly");
export type SelectOption = { "label": string; "value": string; [key: string]: unknown; };
export type TextChild = (string | View);
export type Tone = (("neutral" | "good" | "bad") | "attention");
export type ToolOutput = { "display"?: (View | null); "isError"?: boolean; "parts": Array<ContentPart>; [key: string]: unknown; };
export type TreeNode = { "badge"?: (string | null); "children"?: Array<TreeNode>; "label": string; "tone"?: Tone; [key: string]: unknown; };
export type View = ({ "bold"?: (boolean | null); "children": Array<TextChild>; "color"?: (string | null); "dimColor"?: (boolean | null); "inverse"?: (boolean | null); "italic"?: (boolean | null); "kind": "Text"; "strikethrough"?: (boolean | null); "underline"?: (boolean | null); "wrap"?: (string | null); [key: string]: unknown; } | { "dimColor"?: boolean; "kind": "Markdown"; "text": string; [key: string]: unknown; } | { "kind": "Code"; "language"?: (string | null); "source": string; [key: string]: unknown; } | { "kind": "diff"; "unified": string; [key: string]: unknown; } | { "items": Array<string>; "kind": "list"; [key: string]: unknown; } | { "headers": Array<string>; "kind": "table"; "rows": Array<Array<string>>; [key: string]: unknown; } | { "kind": "keyValue"; "rows": Array<[string, string, ...Array<unknown>]>; [key: string]: unknown; } | { "kind": "progress"; "label"?: (string | null); "total"?: (number | null); "value": number; [key: string]: unknown; } | { "kind": "badge"; "text": string; "tone"?: Tone; [key: string]: unknown; } | { "kind": "tree"; "nodes": Array<TreeNode>; [key: string]: unknown; } | { "alignItems"?: (Align | null); "borderColor"?: (string | null); "borderStyle"?: (string | null); "children": Array<View>; "display"?: (string | null); "flexDirection"?: (Direction | null); "flexGrow"?: (number | null); "flexWrap"?: (Wrap | null); "gap"?: (number | null); "height"?: (number | null); "justifyContent"?: (Justify | null); "key"?: (string | null); "kind": "Box"; "margin"?: (number | null); "marginX"?: (number | null); "marginY"?: (number | null); "padding"?: (number | null); "paddingX"?: (number | null); "paddingY"?: (number | null); "width"?: (number | null); [key: string]: unknown; } | { "child": View; "kind": "panel"; "title": string; [key: string]: unknown; } | { "items": Array<ActionItem>; "kind": "actions"; [key: string]: unknown; } | { "children"?: Array<TextChild>; "dimColor"?: (boolean | null); "hotkey"?: (string | null); "key"?: (string | null); "kind": "Button"; "label"?: (string | null); "onPress"?: (Action | null); "variant"?: (string | null); [key: string]: unknown; } | { "key"?: (string | null); "kind": "Input"; "label"?: (string | null); "onInput"?: (Action | null); "onSubmit"?: (Action | null); "placeholder"?: (string | null); "value"?: (string | null); [key: string]: unknown; } | { "key"?: (string | null); "kind": "Select"; "label"?: (string | null); "onSelect"?: (Action | null); "options"?: (Array<SelectOption> | null); "value"?: (string | null); [key: string]: unknown; } | { "href": string; "kind": "Link"; "label"?: (string | null); [key: string]: unknown; } | { "alt": string; "columns"?: (number | null); "kind": "Image"; "rows"?: (number | null); "source": string; [key: string]: unknown; } | { "cells": Array<string>; "columns": number; "kind": "Raster"; "rows": number; [key: string]: unknown; } | { "alt": string; "height"?: (number | null); "kind": "Svg"; "source": string; "width"?: (number | null); [key: string]: unknown; } | { "component": Component; "kind": "Engine"; "props": unknown; [key: string]: unknown; } | { "customKind": string; "data": unknown; "fold": string; "kind": "custom"; [key: string]: unknown; });
export type Wrap = ("wrap" | "nowrap");
export type Payload = ({ "deny": string; [key: string]: unknown; } | ToolOutput);
}
export type ToolCall = ToolCallInput.Payload;
export namespace ModelCompleteInput {
// Generated from JSON Schema. Do not edit.
export type ContentPart = ({ "text": string; "type": "text"; [key: string]: unknown; } | Image | { "id": string; "input": unknown; "name": string; "type": "toolUse"; [key: string]: unknown; } | { "isError"?: boolean; "parts": Array<ContentPart>; "toolUseId": string; "type": "toolResult"; [key: string]: unknown; } | { "providerMetadata"?: { [key: string]: { [key: string]: unknown; }; }; "text": string; "type": "reasoning"; [key: string]: unknown; });
export type Effort = ("minimal" | "low" | "medium" | "high" | "xHigh" | "max");
export type Image = { "data": string; "height"?: (number | null); "mediaType": string; "path"?: (string | null); "width"?: (number | null); [key: string]: unknown; };
export type Message = { "parts": Array<ContentPart>; "providerOptions"?: { [key: string]: { [key: string]: unknown; }; }; "role": Role; [key: string]: unknown; };
export type ModelRequest = { "maxTokens": number; "messages": Array<Message>; "model": string; "providerOptions"?: { [key: string]: { [key: string]: unknown; }; }; "reasoning"?: (Effort | null); "session"?: (SessionId | null); "system": Array<SystemBlock>; "tools": Array<ToolSpec>; [key: string]: unknown; };
export type Role = ("user" | "assistant");
export type SessionId = string;
export type SystemBlock = { "cache"?: boolean; "text": string; [key: string]: unknown; };
export type ToolSpec = { "description": string; "inputSchema": unknown; "meta"?: { [key: string]: unknown; }; "name": string; [key: string]: unknown; };
export type Payload = { "plugin": string; "request": ModelRequest; [key: string]: unknown; };
}
export namespace ModelCompleteResult {
// Generated from JSON Schema. Do not edit.
export type FinishReason = { "raw"?: (string | null); "unified": UnifiedFinish; [key: string]: unknown; };
export type ModelEvent = ({ "type": "streamStart"; "warnings"?: Array<string>; [key: string]: unknown; } | { "id"?: (string | null); "model"?: (string | null); "type": "responseMetadata"; [key: string]: unknown; } | { "id": string; "type": "textStart"; [key: string]: unknown; } | { "delta": string; "id": string; "type": "textDelta"; [key: string]: unknown; } | { "id": string; "type": "textEnd"; [key: string]: unknown; } | { "id": string; "type": "reasoningStart"; [key: string]: unknown; } | { "delta": string; "id": string; "type": "reasoningDelta"; [key: string]: unknown; } | { "id": string; "providerMetadata"?: { [key: string]: { [key: string]: unknown; }; }; "type": "reasoningEnd"; [key: string]: unknown; } | { "id": string; "name": string; "type": "toolInputStart"; [key: string]: unknown; } | { "delta": string; "id": string; "type": "toolInputDelta"; [key: string]: unknown; } | { "id": string; "type": "toolInputEnd"; [key: string]: unknown; } | { "id": string; "input": string; "name": string; "type": "toolCall"; [key: string]: unknown; } | { "type": "context"; "used": number; "window": number; [key: string]: unknown; } | { "after": number; "before": number; "type": "compacted"; [key: string]: unknown; } | { "finishReason": FinishReason; "type": "finish"; "usage": Usage; [key: string]: unknown; });
export type UnifiedFinish = ("stop" | "length" | "toolCalls" | "contentFilter" | "error" | "other");
export type Usage = { "cacheReadTokens"?: number; "cacheWriteTokens"?: number; "inputTokens": number; "outputTokens": number; "reasoningTokens"?: number; [key: string]: unknown; };
export type Payload = Array<ModelEvent>;
}
export type ModelComplete = ModelCompleteInput.Payload;
export namespace TurnStepInput {
// Generated from JSON Schema. Do not edit.
export type ContentPart = ({ "text": string; "type": "text"; [key: string]: unknown; } | Image | { "id": string; "input": unknown; "name": string; "type": "toolUse"; [key: string]: unknown; } | { "isError"?: boolean; "parts": Array<ContentPart>; "toolUseId": string; "type": "toolResult"; [key: string]: unknown; } | { "providerMetadata"?: { [key: string]: { [key: string]: unknown; }; }; "text": string; "type": "reasoning"; [key: string]: unknown; });
export type Effort = ("minimal" | "low" | "medium" | "high" | "xHigh" | "max");
export type Image = { "data": string; "height"?: (number | null); "mediaType": string; "path"?: (string | null); "width"?: (number | null); [key: string]: unknown; };
export type Message = { "parts": Array<ContentPart>; "providerOptions"?: { [key: string]: { [key: string]: unknown; }; }; "role": Role; [key: string]: unknown; };
export type Role = ("user" | "assistant");
export type SessionId = string;
export type SystemBlock = { "cache"?: boolean; "text": string; [key: string]: unknown; };
export type ToolSpec = { "description": string; "inputSchema": unknown; "meta"?: { [key: string]: unknown; }; "name": string; [key: string]: unknown; };
export type Payload = { "maxTokens": number; "messages": Array<Message>; "model": string; "providerOptions"?: { [key: string]: { [key: string]: unknown; }; }; "reasoning"?: (Effort | null); "session"?: (SessionId | null); "system": Array<SystemBlock>; "tools": Array<ToolSpec>; [key: string]: unknown; };
}
export namespace TurnStepResult {
// Generated from JSON Schema. Do not edit.
export type FinishReason = { "raw"?: (string | null); "unified": UnifiedFinish; [key: string]: unknown; };
export type ModelEvent = ({ "type": "streamStart"; "warnings"?: Array<string>; [key: string]: unknown; } | { "id"?: (string | null); "model"?: (string | null); "type": "responseMetadata"; [key: string]: unknown; } | { "id": string; "type": "textStart"; [key: string]: unknown; } | { "delta": string; "id": string; "type": "textDelta"; [key: string]: unknown; } | { "id": string; "type": "textEnd"; [key: string]: unknown; } | { "id": string; "type": "reasoningStart"; [key: string]: unknown; } | { "delta": string; "id": string; "type": "reasoningDelta"; [key: string]: unknown; } | { "id": string; "providerMetadata"?: { [key: string]: { [key: string]: unknown; }; }; "type": "reasoningEnd"; [key: string]: unknown; } | { "id": string; "name": string; "type": "toolInputStart"; [key: string]: unknown; } | { "delta": string; "id": string; "type": "toolInputDelta"; [key: string]: unknown; } | { "id": string; "type": "toolInputEnd"; [key: string]: unknown; } | { "id": string; "input": string; "name": string; "type": "toolCall"; [key: string]: unknown; } | { "type": "context"; "used": number; "window": number; [key: string]: unknown; } | { "after": number; "before": number; "type": "compacted"; [key: string]: unknown; } | { "finishReason": FinishReason; "type": "finish"; "usage": Usage; [key: string]: unknown; });
export type ProviderError = ({ "kind": "auth"; "message": string; [key: string]: unknown; } | { "kind": "rateLimited"; "retryAfterMs"?: (number | null); [key: string]: unknown; } | { "kind": "contextOverflow"; "message": string; [key: string]: unknown; } | { "kind": "server"; "message": string; "status": number; [key: string]: unknown; } | { "kind": "request"; "message": string; [key: string]: unknown; } | { "kind": "transport"; "message": string; [key: string]: unknown; } | { "kind": "stream"; "message": string; [key: string]: unknown; } | { "kind": "timeout"; [key: string]: unknown; } | { "kind": "unsupported"; "message": string; [key: string]: unknown; } | { "kind": "config"; "message": string; [key: string]: unknown; });
export type UnifiedFinish = ("stop" | "length" | "toolCalls" | "contentFilter" | "error" | "other");
export type Usage = { "cacheReadTokens"?: number; "cacheWriteTokens"?: number; "inputTokens": number; "outputTokens": number; "reasoningTokens"?: number; [key: string]: unknown; };
export type Payload = ({ "Ok": Array<ModelEvent>; [key: string]: unknown; } | { "Err": ProviderError; [key: string]: unknown; });
}
export type TurnStep = TurnStepInput.Payload;
export namespace PromptComposeInput {
// Generated from JSON Schema. Do not edit.
export type ContentPart = ({ "text": string; "type": "text"; [key: string]: unknown; } | Image | { "id": string; "input": unknown; "name": string; "type": "toolUse"; [key: string]: unknown; } | { "isError"?: boolean; "parts": Array<ContentPart>; "toolUseId": string; "type": "toolResult"; [key: string]: unknown; } | { "providerMetadata"?: { [key: string]: { [key: string]: unknown; }; }; "text": string; "type": "reasoning"; [key: string]: unknown; });
export type Effort = ("minimal" | "low" | "medium" | "high" | "xHigh" | "max");
export type Image = { "data": string; "height"?: (number | null); "mediaType": string; "path"?: (string | null); "width"?: (number | null); [key: string]: unknown; };
export type Message = { "parts": Array<ContentPart>; "providerOptions"?: { [key: string]: { [key: string]: unknown; }; }; "role": Role; [key: string]: unknown; };
export type Role = ("user" | "assistant");
export type SessionId = string;
export type SystemBlock = { "cache"?: boolean; "text": string; [key: string]: unknown; };
export type ToolSpec = { "description": string; "inputSchema": unknown; "meta"?: { [key: string]: unknown; }; "name": string; [key: string]: unknown; };
export type Payload = { "maxTokens": number; "messages": Array<Message>; "model": string; "providerOptions"?: { [key: string]: { [key: string]: unknown; }; }; "reasoning"?: (Effort | null); "session"?: (SessionId | null); "system": Array<SystemBlock>; "tools": Array<ToolSpec>; [key: string]: unknown; };
}
export namespace PromptComposeResult {
// Generated from JSON Schema. Do not edit.
export type ContentPart = ({ "text": string; "type": "text"; [key: string]: unknown; } | Image | { "id": string; "input": unknown; "name": string; "type": "toolUse"; [key: string]: unknown; } | { "isError"?: boolean; "parts": Array<ContentPart>; "toolUseId": string; "type": "toolResult"; [key: string]: unknown; } | { "providerMetadata"?: { [key: string]: { [key: string]: unknown; }; }; "text": string; "type": "reasoning"; [key: string]: unknown; });
export type Effort = ("minimal" | "low" | "medium" | "high" | "xHigh" | "max");
export type Image = { "data": string; "height"?: (number | null); "mediaType": string; "path"?: (string | null); "width"?: (number | null); [key: string]: unknown; };
export type Message = { "parts": Array<ContentPart>; "providerOptions"?: { [key: string]: { [key: string]: unknown; }; }; "role": Role; [key: string]: unknown; };
export type Role = ("user" | "assistant");
export type SessionId = string;
export type SystemBlock = { "cache"?: boolean; "text": string; [key: string]: unknown; };
export type ToolSpec = { "description": string; "inputSchema": unknown; "meta"?: { [key: string]: unknown; }; "name": string; [key: string]: unknown; };
export type Payload = { "maxTokens": number; "messages": Array<Message>; "model": string; "providerOptions"?: { [key: string]: { [key: string]: unknown; }; }; "reasoning"?: (Effort | null); "session"?: (SessionId | null); "system": Array<SystemBlock>; "tools": Array<ToolSpec>; [key: string]: unknown; };
}
export type PromptCompose = PromptComposeInput.Payload;
export namespace SessionCompactInput {
// Generated from JSON Schema. Do not edit.
export type Action = { "args"?: unknown; "name": string; [key: string]: unknown; };
export type ActionItem = { "action": Action; "key"?: (string | null); "label": string; [key: string]: unknown; };
export type Align = ("flex-start" | "center" | "flex-end" | "stretch");
export type CompactReason = ({ "kind": "threshold"; [key: string]: unknown; } | { "kind": "overflow"; "message": string; [key: string]: unknown; } | { "instructions"?: (string | null); "kind": "manual"; [key: string]: unknown; });
export type Component = ("Question" | "UserMessage" | "AssistantMessage" | "ToolUse" | "ToolResult" | "ToolGroup" | "ToolProgress" | "CommandOutput" | "Spinner" | "TurnDuration" | "InfoNotice" | "SessionMode" | "PromptHint" | "AbovePrompt" | "Pane" | "Permission" | "SignIn" | "Reasoning" | "Compaction" | "Rewind" | "Interruption" | "QuestionAnswer" | "PermissionReceipt" | "Asset" | "ShellRun" | "Action" | "TurnProgress" | "SessionStatus" | "Toast" | "Welcome");
export type ContentPart = ({ "text": string; "type": "text"; [key: string]: unknown; } | Image | { "id": string; "input": unknown; "name": string; "type": "toolUse"; [key: string]: unknown; } | { "isError"?: boolean; "parts": Array<ContentPart>; "toolUseId": string; "type": "toolResult"; [key: string]: unknown; } | { "providerMetadata"?: { [key: string]: { [key: string]: unknown; }; }; "text": string; "type": "reasoning"; [key: string]: unknown; });
export type ContextUsage = { "trigger": number; "used": number; "window": number; [key: string]: unknown; };
export type DecisionKind = ("allow" | "allowSession" | "deny");
export type Direction = ("row" | "column");
export type Effort = ("minimal" | "low" | "medium" | "high" | "xHigh" | "max");
export type Image = { "data": string; "height"?: (number | null); "mediaType": string; "path"?: (string | null); "width"?: (number | null); [key: string]: unknown; };
export type IntentId = string;
export type InteractionId = string;
export type Item = { "body": ItemBody; "completedAt"?: (string | null); "id": ItemId; "intent"?: (IntentId | null); "meta"?: { [key: string]: unknown; }; "round"?: number; "startedAt": string; "status": ItemStatus; "turn"?: (TurnId | null); [key: string]: unknown; };
export type ItemBody = ({ "kind": "user"; "origin": Origin; "parts": Array<ContentPart>; [key: string]: unknown; } | { "kind": "assistant"; "text": string; [key: string]: unknown; } | { "kind": "reasoning"; "providerMetadata"?: { [key: string]: { [key: string]: unknown; }; }; "text": string; [key: string]: unknown; } | { "callId": string; "durationMs"?: (number | null); "input": unknown; "kind": "toolCall"; "name": string; "output"?: (ToolOutput | null); "progress"?: (string | null); "quiet"?: boolean; [key: string]: unknown; } | { "command": string; "cwd": string; "exit"?: (number | null); "kind": "shell"; "output": string; [key: string]: unknown; } | { "args"?: unknown; "kind": "action"; "name": string; "result"?: unknown; [key: string]: unknown; } | { "after": number; "before": number; "durationMs": number; "kind": "compaction"; "replaced": number; "summary": string; [key: string]: unknown; } | { "dropped": number; "kind": "rewind"; "toTurn": TurnId; [key: string]: unknown; } | { "kind": "interruption"; "marker": string; [key: string]: unknown; } | { "code": string; "kind": "notice"; "level": Level; "text": string; [key: string]: unknown; } | { "answer": string; "interaction": InteractionId; "kind": "questionAnswer"; "question": string; [key: string]: unknown; } | { "decision": DecisionKind; "feedback"?: (string | null); "interaction": InteractionId; "kind": "permissionReceipt"; "tool": string; [key: string]: unknown; } | { "asset": string; "kind": "asset"; "label"?: (string | null); [key: string]: unknown; } | { [key: string]: unknown; });
export type ItemId = string;
export type ItemStatus = ("pending" | "running" | "completed" | "failed" | "interrupted");
export type Justify = ("flex-start" | "center" | "flex-end" | "space-between" | "space-around" | "space-evenly");
export type Level = ("info" | "warn" | "error");
export type Message = { "parts": Array<ContentPart>; "providerOptions"?: { [key: string]: { [key: string]: unknown; }; }; "role": Role; [key: string]: unknown; };
export type ModelRequest = { "maxTokens": number; "messages": Array<Message>; "model": string; "providerOptions"?: { [key: string]: { [key: string]: unknown; }; }; "reasoning"?: (Effort | null); "session"?: (SessionId | null); "system": Array<SystemBlock>; "tools": Array<ToolSpec>; [key: string]: unknown; };
export type Origin = { "conversation"?: (string | null); "principal"?: (string | null); "surface": string; "voice"?: (Voice | null); [key: string]: unknown; };
export type Role = ("user" | "assistant");
export type SelectOption = { "label": string; "value": string; [key: string]: unknown; };
export type SessionId = string;
export type SystemBlock = { "cache"?: boolean; "text": string; [key: string]: unknown; };
export type TextChild = (string | View);
export type Tone = (("neutral" | "good" | "bad") | "attention");
export type ToolOutput = { "display"?: (View | null); "isError"?: boolean; "parts": Array<ContentPart>; [key: string]: unknown; };
export type ToolSpec = { "description": string; "inputSchema": unknown; "meta"?: { [key: string]: unknown; }; "name": string; [key: string]: unknown; };
export type TreeNode = { "badge"?: (string | null); "children"?: Array<TreeNode>; "label": string; "tone"?: Tone; [key: string]: unknown; };
export type TurnId = string;
export type View = ({ "bold"?: (boolean | null); "children": Array<TextChild>; "color"?: (string | null); "dimColor"?: (boolean | null); "inverse"?: (boolean | null); "italic"?: (boolean | null); "kind": "Text"; "strikethrough"?: (boolean | null); "underline"?: (boolean | null); "wrap"?: (string | null); [key: string]: unknown; } | { "dimColor"?: boolean; "kind": "Markdown"; "text": string; [key: string]: unknown; } | { "kind": "Code"; "language"?: (string | null); "source": string; [key: string]: unknown; } | { "kind": "diff"; "unified": string; [key: string]: unknown; } | { "items": Array<string>; "kind": "list"; [key: string]: unknown; } | { "headers": Array<string>; "kind": "table"; "rows": Array<Array<string>>; [key: string]: unknown; } | { "kind": "keyValue"; "rows": Array<[string, string, ...Array<unknown>]>; [key: string]: unknown; } | { "kind": "progress"; "label"?: (string | null); "total"?: (number | null); "value": number; [key: string]: unknown; } | { "kind": "badge"; "text": string; "tone"?: Tone; [key: string]: unknown; } | { "kind": "tree"; "nodes": Array<TreeNode>; [key: string]: unknown; } | { "alignItems"?: (Align | null); "borderColor"?: (string | null); "borderStyle"?: (string | null); "children": Array<View>; "display"?: (string | null); "flexDirection"?: (Direction | null); "flexGrow"?: (number | null); "flexWrap"?: (Wrap | null); "gap"?: (number | null); "height"?: (number | null); "justifyContent"?: (Justify | null); "key"?: (string | null); "kind": "Box"; "margin"?: (number | null); "marginX"?: (number | null); "marginY"?: (number | null); "padding"?: (number | null); "paddingX"?: (number | null); "paddingY"?: (number | null); "width"?: (number | null); [key: string]: unknown; } | { "child": View; "kind": "panel"; "title": string; [key: string]: unknown; } | { "items": Array<ActionItem>; "kind": "actions"; [key: string]: unknown; } | { "children"?: Array<TextChild>; "dimColor"?: (boolean | null); "hotkey"?: (string | null); "key"?: (string | null); "kind": "Button"; "label"?: (string | null); "onPress"?: (Action | null); "variant"?: (string | null); [key: string]: unknown; } | { "key"?: (string | null); "kind": "Input"; "label"?: (string | null); "onInput"?: (Action | null); "onSubmit"?: (Action | null); "placeholder"?: (string | null); "value"?: (string | null); [key: string]: unknown; } | { "key"?: (string | null); "kind": "Select"; "label"?: (string | null); "onSelect"?: (Action | null); "options"?: (Array<SelectOption> | null); "value"?: (string | null); [key: string]: unknown; } | { "href": string; "kind": "Link"; "label"?: (string | null); [key: string]: unknown; } | { "alt": string; "columns"?: (number | null); "kind": "Image"; "rows"?: (number | null); "source": string; [key: string]: unknown; } | { "cells": Array<string>; "columns": number; "kind": "Raster"; "rows": number; [key: string]: unknown; } | { "alt": string; "height"?: (number | null); "kind": "Svg"; "source": string; "width"?: (number | null); [key: string]: unknown; } | { "component": Component; "kind": "Engine"; "props": unknown; [key: string]: unknown; } | { "customKind": string; "data": unknown; "fold": string; "kind": "custom"; [key: string]: unknown; });
export type Voice = ("said" | "note" | "quiet" | "aside");
export type Wrap = ("wrap" | "nowrap");
export type Payload = { "failures": number; "items": Array<Item>; "keepBudget": number; "reason": CompactReason; "request": ModelRequest; "usage": ContextUsage; [key: string]: unknown; };
}
export namespace SessionCompactResult {
// Generated from JSON Schema. Do not edit.
export type CompactError = { "error": KernelError; "usage"?: Usage; [key: string]: unknown; };
export type Compaction = { "after": number; "before": number; "boundary": ItemId; "kept"?: Array<ItemId>; "summary": string; "usage"?: Usage; [key: string]: unknown; };
export type ErrorCode = (("SESSION_NOT_FOUND" | "SESSION_LOCKED" | "SESSION_CLOSED" | "INTERACTION_CLOSED" | "NOT_READY" | "STALE_GENERATION" | "NOT_INITIALIZED" | "INVALID_INPUT" | "PERMISSION_DENIED" | "TOOL_NOT_FOUND" | "TOOL_FAILED" | "PROVIDER_UNAVAILABLE" | "AUTH_REQUIRED" | "RATE_LIMITED" | "CONTEXT_OVERFLOW" | "TIMEOUT" | "OFFLINE" | "SERVER_ERROR" | "TURN_LOST" | "TURN_BUDGET_EXHAUSTED" | "UNSUPPORTED" | "INTERNAL") | "INTERRUPTED" | "PROTOCOL_LIMIT" | "NOT_FOUND" | "STORAGE");
export type ItemId = string;
export type KernelError = { "code": ErrorCode; "message": string; [key: string]: unknown; };
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
export type Payload = (("complete") | { "continue": { "origin": string; "reason": string; [key: string]: unknown; }; });
}
export type TurnStop = TurnStopInput.Payload;
export namespace ToolDescribeInput {
// Generated from JSON Schema. Do not edit.
export type Payload = { "description": string; "inputSchema": unknown; "meta"?: { [key: string]: unknown; }; "name": string; [key: string]: unknown; };
}
export namespace ToolDescribeResult {
// Generated from JSON Schema. Do not edit.
export type ToolSpec = { "description": string; "inputSchema": unknown; "meta"?: { [key: string]: unknown; }; "name": string; [key: string]: unknown; };
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
export type Action = { "args"?: unknown; "name": string; [key: string]: unknown; };
export type Delivery = ("wake" | "hold");
export type Image = { "data": string; "height"?: (number | null); "mediaType": string; "path"?: (string | null); "width"?: (number | null); [key: string]: unknown; };
export type Input = ({ "delivery"?: Delivery; "images"?: Array<Image>; "kind": "text"; "origin": Origin; "text": string; [key: string]: unknown; } | { "action": Action; "kind": "action"; [key: string]: unknown; });
export type Origin = { "conversation"?: (string | null); "principal"?: (string | null); "surface": string; "voice"?: (Voice | null); [key: string]: unknown; };
export type SessionId = string;
export type Voice = ("said" | "note" | "quiet" | "aside");
export type Payload = { "delivery": Delivery; "input": Input; "to": SessionId; [key: string]: unknown; };
}
export namespace SessionSendResult {
// Generated from JSON Schema. Do not edit.
export type Payload = null;
}
export type SessionSend = SessionSendInput.Payload;
export namespace SessionReceiveInput {
// Generated from JSON Schema. Do not edit.
export type Action = { "args"?: unknown; "name": string; [key: string]: unknown; };
export type Delivery = ("wake" | "hold");
export type Image = { "data": string; "height"?: (number | null); "mediaType": string; "path"?: (string | null); "width"?: (number | null); [key: string]: unknown; };
export type Origin = { "conversation"?: (string | null); "principal"?: (string | null); "surface": string; "voice"?: (Voice | null); [key: string]: unknown; };
export type Voice = ("said" | "note" | "quiet" | "aside");
export type Payload = ({ "delivery"?: Delivery; "images"?: Array<Image>; "kind": "text"; "origin": Origin; "text": string; [key: string]: unknown; } | { "action": Action; "kind": "action"; [key: string]: unknown; });
}
export namespace SessionReceiveResult {
// Generated from JSON Schema. Do not edit.
export type Action = { "args"?: unknown; "name": string; [key: string]: unknown; };
export type Delivery = ("wake" | "hold");
export type Image = { "data": string; "height"?: (number | null); "mediaType": string; "path"?: (string | null); "width"?: (number | null); [key: string]: unknown; };
export type Origin = { "conversation"?: (string | null); "principal"?: (string | null); "surface": string; "voice"?: (Voice | null); [key: string]: unknown; };
export type Voice = ("said" | "note" | "quiet" | "aside");
export type Payload = ({ "delivery"?: Delivery; "images"?: Array<Image>; "kind": "text"; "origin": Origin; "text": string; [key: string]: unknown; } | { "action": Action; "kind": "action"; [key: string]: unknown; });
}
export type SessionReceive = SessionReceiveInput.Payload;
export namespace SessionAppendInput {
// Generated from JSON Schema. Do not edit.
export type Payload = { "kind": string; "payload": unknown; "plugin": string; [key: string]: unknown; };
}
export namespace SessionAppendResult {
// Generated from JSON Schema. Do not edit.
export type Payload = { "kind": string; "payload": unknown; "plugin": string; [key: string]: unknown; };
}
export type SessionAppend = SessionAppendInput.Payload;
export namespace TurnCompleteInput {
// Generated from JSON Schema. Do not edit.
export type ErrorCode = (("SESSION_NOT_FOUND" | "SESSION_LOCKED" | "SESSION_CLOSED" | "INTERACTION_CLOSED" | "NOT_READY" | "STALE_GENERATION" | "NOT_INITIALIZED" | "INVALID_INPUT" | "PERMISSION_DENIED" | "TOOL_NOT_FOUND" | "TOOL_FAILED" | "PROVIDER_UNAVAILABLE" | "AUTH_REQUIRED" | "RATE_LIMITED" | "CONTEXT_OVERFLOW" | "TIMEOUT" | "OFFLINE" | "SERVER_ERROR" | "TURN_LOST" | "TURN_BUDGET_EXHAUSTED" | "UNSUPPORTED" | "INTERNAL") | "INTERRUPTED" | "PROTOCOL_LIMIT" | "NOT_FOUND" | "STORAGE");
export type InterruptReason = ("userCancel" | "newInput" | "shutdown" | "budget");
export type KernelError = { "code": ErrorCode; "message": string; [key: string]: unknown; };
export type TurnId = string;
export type TurnStatus = ({ "kind": "completed"; [key: string]: unknown; } | { "error": KernelError; "kind": "failed"; [key: string]: unknown; } | { "kind": "interrupted"; "reason": InterruptReason; [key: string]: unknown; });
export type Usage = { "cacheReadTokens"?: number; "cacheWriteTokens"?: number; "inputTokens": number; "outputTokens": number; "reasoningTokens"?: number; [key: string]: unknown; };
export type Payload = { "answer": string; "outcome": TurnStatus; "turn": TurnId; "usage": Usage; [key: string]: unknown; };
}
export namespace TurnCompleteResult {
// Generated from JSON Schema. Do not edit.
export type Payload = (string | null);
}
export type TurnComplete = TurnCompleteInput.Payload;
export namespace CommandRunInput {
// Generated from JSON Schema. Do not edit.
export type Payload = { "args": string; "command": string; [key: string]: unknown; };
}
export namespace CommandRunResult {
// Generated from JSON Schema. Do not edit.
export type Action = { "args"?: unknown; "name": string; [key: string]: unknown; };
export type ActionItem = { "action": Action; "key"?: (string | null); "label": string; [key: string]: unknown; };
export type Align = ("flex-start" | "center" | "flex-end" | "stretch");
export type CommandOutcome = ({ "kind": "applied"; "message"?: (string | null); [key: string]: unknown; } | { "kind": "view"; "view": View; [key: string]: unknown; } | { "kind": "prompt"; "text": string; [key: string]: unknown; } | { "body": ItemBody; "kind": "record"; [key: string]: unknown; });
export type Component = ("Question" | "UserMessage" | "AssistantMessage" | "ToolUse" | "ToolResult" | "ToolGroup" | "ToolProgress" | "CommandOutput" | "Spinner" | "TurnDuration" | "InfoNotice" | "SessionMode" | "PromptHint" | "AbovePrompt" | "Pane" | "Permission" | "SignIn" | "Reasoning" | "Compaction" | "Rewind" | "Interruption" | "QuestionAnswer" | "PermissionReceipt" | "Asset" | "ShellRun" | "Action" | "TurnProgress" | "SessionStatus" | "Toast" | "Welcome");
export type ContentPart = ({ "text": string; "type": "text"; [key: string]: unknown; } | Image | { "id": string; "input": unknown; "name": string; "type": "toolUse"; [key: string]: unknown; } | { "isError"?: boolean; "parts": Array<ContentPart>; "toolUseId": string; "type": "toolResult"; [key: string]: unknown; } | { "providerMetadata"?: { [key: string]: { [key: string]: unknown; }; }; "text": string; "type": "reasoning"; [key: string]: unknown; });
export type DecisionKind = ("allow" | "allowSession" | "deny");
export type Direction = ("row" | "column");
export type Image = { "data": string; "height"?: (number | null); "mediaType": string; "path"?: (string | null); "width"?: (number | null); [key: string]: unknown; };
export type InteractionId = string;
export type ItemBody = ({ "kind": "user"; "origin": Origin; "parts": Array<ContentPart>; [key: string]: unknown; } | { "kind": "assistant"; "text": string; [key: string]: unknown; } | { "kind": "reasoning"; "providerMetadata"?: { [key: string]: { [key: string]: unknown; }; }; "text": string; [key: string]: unknown; } | { "callId": string; "durationMs"?: (number | null); "input": unknown; "kind": "toolCall"; "name": string; "output"?: (ToolOutput | null); "progress"?: (string | null); "quiet"?: boolean; [key: string]: unknown; } | { "command": string; "cwd": string; "exit"?: (number | null); "kind": "shell"; "output": string; [key: string]: unknown; } | { "args"?: unknown; "kind": "action"; "name": string; "result"?: unknown; [key: string]: unknown; } | { "after": number; "before": number; "durationMs": number; "kind": "compaction"; "replaced": number; "summary": string; [key: string]: unknown; } | { "dropped": number; "kind": "rewind"; "toTurn": TurnId; [key: string]: unknown; } | { "kind": "interruption"; "marker": string; [key: string]: unknown; } | { "code": string; "kind": "notice"; "level": Level; "text": string; [key: string]: unknown; } | { "answer": string; "interaction": InteractionId; "kind": "questionAnswer"; "question": string; [key: string]: unknown; } | { "decision": DecisionKind; "feedback"?: (string | null); "interaction": InteractionId; "kind": "permissionReceipt"; "tool": string; [key: string]: unknown; } | { "asset": string; "kind": "asset"; "label"?: (string | null); [key: string]: unknown; } | { [key: string]: unknown; });
export type Justify = ("flex-start" | "center" | "flex-end" | "space-between" | "space-around" | "space-evenly");
export type Level = ("info" | "warn" | "error");
export type Origin = { "conversation"?: (string | null); "principal"?: (string | null); "surface": string; "voice"?: (Voice | null); [key: string]: unknown; };
export type SelectOption = { "label": string; "value": string; [key: string]: unknown; };
export type TextChild = (string | View);
export type Tone = (("neutral" | "good" | "bad") | "attention");
export type ToolOutput = { "display"?: (View | null); "isError"?: boolean; "parts": Array<ContentPart>; [key: string]: unknown; };
export type TreeNode = { "badge"?: (string | null); "children"?: Array<TreeNode>; "label": string; "tone"?: Tone; [key: string]: unknown; };
export type TurnId = string;
export type View = ({ "bold"?: (boolean | null); "children": Array<TextChild>; "color"?: (string | null); "dimColor"?: (boolean | null); "inverse"?: (boolean | null); "italic"?: (boolean | null); "kind": "Text"; "strikethrough"?: (boolean | null); "underline"?: (boolean | null); "wrap"?: (string | null); [key: string]: unknown; } | { "dimColor"?: boolean; "kind": "Markdown"; "text": string; [key: string]: unknown; } | { "kind": "Code"; "language"?: (string | null); "source": string; [key: string]: unknown; } | { "kind": "diff"; "unified": string; [key: string]: unknown; } | { "items": Array<string>; "kind": "list"; [key: string]: unknown; } | { "headers": Array<string>; "kind": "table"; "rows": Array<Array<string>>; [key: string]: unknown; } | { "kind": "keyValue"; "rows": Array<[string, string, ...Array<unknown>]>; [key: string]: unknown; } | { "kind": "progress"; "label"?: (string | null); "total"?: (number | null); "value": number; [key: string]: unknown; } | { "kind": "badge"; "text": string; "tone"?: Tone; [key: string]: unknown; } | { "kind": "tree"; "nodes": Array<TreeNode>; [key: string]: unknown; } | { "alignItems"?: (Align | null); "borderColor"?: (string | null); "borderStyle"?: (string | null); "children": Array<View>; "display"?: (string | null); "flexDirection"?: (Direction | null); "flexGrow"?: (number | null); "flexWrap"?: (Wrap | null); "gap"?: (number | null); "height"?: (number | null); "justifyContent"?: (Justify | null); "key"?: (string | null); "kind": "Box"; "margin"?: (number | null); "marginX"?: (number | null); "marginY"?: (number | null); "padding"?: (number | null); "paddingX"?: (number | null); "paddingY"?: (number | null); "width"?: (number | null); [key: string]: unknown; } | { "child": View; "kind": "panel"; "title": string; [key: string]: unknown; } | { "items": Array<ActionItem>; "kind": "actions"; [key: string]: unknown; } | { "children"?: Array<TextChild>; "dimColor"?: (boolean | null); "hotkey"?: (string | null); "key"?: (string | null); "kind": "Button"; "label"?: (string | null); "onPress"?: (Action | null); "variant"?: (string | null); [key: string]: unknown; } | { "key"?: (string | null); "kind": "Input"; "label"?: (string | null); "onInput"?: (Action | null); "onSubmit"?: (Action | null); "placeholder"?: (string | null); "value"?: (string | null); [key: string]: unknown; } | { "key"?: (string | null); "kind": "Select"; "label"?: (string | null); "onSelect"?: (Action | null); "options"?: (Array<SelectOption> | null); "value"?: (string | null); [key: string]: unknown; } | { "href": string; "kind": "Link"; "label"?: (string | null); [key: string]: unknown; } | { "alt": string; "columns"?: (number | null); "kind": "Image"; "rows"?: (number | null); "source": string; [key: string]: unknown; } | { "cells": Array<string>; "columns": number; "kind": "Raster"; "rows": number; [key: string]: unknown; } | { "alt": string; "height"?: (number | null); "kind": "Svg"; "source": string; "width"?: (number | null); [key: string]: unknown; } | { "component": Component; "kind": "Engine"; "props": unknown; [key: string]: unknown; } | { "customKind": string; "data": unknown; "fold": string; "kind": "custom"; [key: string]: unknown; });
export type Voice = ("said" | "note" | "quiet" | "aside");
export type Wrap = ("wrap" | "nowrap");
export type Payload = (CommandOutcome | { "text": string; [key: string]: unknown; });
}
export type CommandRun = CommandRunInput.Payload;
export namespace CommandDescribeInput {
// Generated from JSON Schema. Do not edit.
export type ArgSpec = ({ "kind": "none"; [key: string]: unknown; } | { "hint": string; "kind": "free"; [key: string]: unknown; } | { "kind": "catalog"; "source": string; [key: string]: unknown; } | { "alone"?: Array<string>; "kind": "words"; "then"?: (ArgSpec | null); "values": Array<string>; [key: string]: unknown; });
export type Payload = { "aliases"?: Array<string>; "args": ArgSpec; "family": string; "hidden"?: boolean; "hint": string; "instant"?: boolean; "name": string; [key: string]: unknown; };
}
export namespace CommandDescribeResult {
// Generated from JSON Schema. Do not edit.
export type ArgSpec = ({ "kind": "none"; [key: string]: unknown; } | { "hint": string; "kind": "free"; [key: string]: unknown; } | { "kind": "catalog"; "source": string; [key: string]: unknown; } | { "alone"?: Array<string>; "kind": "words"; "then"?: (ArgSpec | null); "values": Array<string>; [key: string]: unknown; });
export type CommandSpec = { "aliases"?: Array<string>; "args": ArgSpec; "family": string; "hidden"?: boolean; "hint": string; "instant"?: boolean; "name": string; [key: string]: unknown; };
export type Payload = (CommandSpec | null);
}
export type CommandDescribe = CommandDescribeInput.Payload;
export namespace ConfigSetInput {
// Generated from JSON Schema. Do not edit.
export type Payload = { "key": string; "value": unknown; [key: string]: unknown; };
}
export namespace ConfigSetResult {
// Generated from JSON Schema. Do not edit.
export type Payload = { "key": string; "value": unknown; [key: string]: unknown; };
}
export type ConfigSet = ConfigSetInput.Payload;
export namespace ConfigDescribeInput {
// Generated from JSON Schema. Do not edit.
export type Payload = { "kernel"?: unknown; "plugins"?: { [key: string]: unknown; }; [key: string]: unknown; };
}
export namespace ConfigDescribeResult {
// Generated from JSON Schema. Do not edit.
export type Payload = { "kernel"?: unknown; "plugins"?: { [key: string]: unknown; }; [key: string]: unknown; };
}
export type ConfigDescribe = ConfigDescribeInput.Payload;
export namespace TurnStartInput {
// Generated from JSON Schema. Do not edit.
export type ItemId = string;
export type TurnId = string;
export type TurnOrigin = ("submit" | "queue" | "peer" | "auto");
export type Payload = { "inputs": Array<ItemId>; "origin": TurnOrigin; "turn": TurnId; [key: string]: unknown; };
}
export namespace TurnStartResult {
// Generated from JSON Schema. Do not edit.
export type Payload = null;
}
export type TurnStart = TurnStartInput.Payload;
export namespace SessionMeasureInput {
// Generated from JSON Schema. Do not edit.
export type ContextUsage = { "trigger": number; "used": number; "window": number; [key: string]: unknown; };
export type TurnId = string;
export type Usage = { "cacheReadTokens"?: number; "cacheWriteTokens"?: number; "inputTokens": number; "outputTokens": number; "reasoningTokens"?: number; [key: string]: unknown; };
export type Payload = { "context": ContextUsage; "turn": TurnId; "usage": Usage; [key: string]: unknown; };
}
export namespace SessionMeasureResult {
// Generated from JSON Schema. Do not edit.
export type Payload = null;
}
export type SessionMeasure = SessionMeasureInput.Payload;
export namespace SessionStartInput {
// Generated from JSON Schema. Do not edit.
export type SessionId = string;
export type Payload = { "session": SessionId; [key: string]: unknown; };
}
export namespace SessionStartResult {
// Generated from JSON Schema. Do not edit.
export type Payload = null;
}
export type SessionStart = SessionStartInput.Payload;
export namespace SessionEndInput {
// Generated from JSON Schema. Do not edit.
export type SessionId = string;
export type Payload = { "session": SessionId; [key: string]: unknown; };
}
export namespace SessionEndResult {
// Generated from JSON Schema. Do not edit.
export type Payload = null;
}
export type SessionEnd = SessionEndInput.Payload;
export namespace SessionAttachInput {
// Generated from JSON Schema. Do not edit.
export type ClientIdentity = { "name": string; "surface": string; [key: string]: unknown; };
export type SessionId = string;
export type Payload = { "client": ClientIdentity; "session": SessionId; [key: string]: unknown; };
}
export namespace SessionAttachResult {
// Generated from JSON Schema. Do not edit.
export type Payload = null;
}
export type SessionAttach = SessionAttachInput.Payload;
export namespace SessionDetachInput {
// Generated from JSON Schema. Do not edit.
export type ClientIdentity = { "name": string; "surface": string; [key: string]: unknown; };
export type SessionId = string;
export type Payload = { "client": ClientIdentity; "session": SessionId; [key: string]: unknown; };
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
