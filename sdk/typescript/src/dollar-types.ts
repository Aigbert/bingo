// Generated from JSON Schema. Do not edit.
export type Action = { "args"?: unknown; "name": string; [key: string]: unknown; };
export type ActionItem = { "action": Action; "key"?: (string | null); "label": string; [key: string]: unknown; };
export type Align = ("flex-start" | "center" | "flex-end" | "stretch");
export type Ancestors = { "name": string; "session": SessionId; [key: string]: unknown; };
export type Answer = ({ "kind": "allowOnce"; [key: string]: unknown; } | { "kind": "allowSession"; "scope": string; [key: string]: unknown; } | { "feedback"?: (string | null); "kind": "deny"; [key: string]: unknown; } | { "ids": Array<string>; "kind": "choice"; "other"?: (string | null); [key: string]: unknown; } | { "kind": "text"; "text": string; [key: string]: unknown; } | { "kind": "confirm"; [key: string]: unknown; } | { "kind": "cancel"; [key: string]: unknown; } | { "answers": Array<Answer>; "kind": "form"; [key: string]: unknown; });
export type AnswerRole = ("allowing" | "refusing");
export type AnswerSpec = ("allowOnce" | "allowSession" | "deny" | "choice" | "text" | "confirm" | "cancel" | "form");
export type ArgSpec = ({ "kind": "none"; [key: string]: unknown; } | { "hint": string; "kind": "free"; [key: string]: unknown; } | { "kind": "catalog"; "source": string; [key: string]: unknown; } | { "kind": "words"; "then"?: (ArgSpec | null); "values": Array<string>; [key: string]: unknown; });
export type Authorize = { "answers": Array<AnswerSpec>; "kind": InteractionKind; "session": SessionId; [key: string]: unknown; };
export type Catalog = { "entries": Array<CatalogEntry>; "kind": CatalogKind; [key: string]: unknown; };
export type CatalogEntry = { "id": string; "label": string; "meta"?: unknown; [key: string]: unknown; };
export type CatalogKind = ("events" | "models" | "providers" | "tools" | "commands" | "plugins");
export type Classify = { "labels": Array<string>; "session": SessionId; "text": string; "turn"?: (TurnId | null); [key: string]: unknown; };
export type ClientIdentity = { "name": string; "surface": string; [key: string]: unknown; };
export type CommandSpec = { "aliases"?: Array<string>; "args": ArgSpec; "family": string; "hidden"?: boolean; "hint": string; "instant"?: boolean; "name": string; [key: string]: unknown; };
export type Complete = { "request": ModelRequest; "session": SessionId; "turn"?: (TurnId | null); [key: string]: unknown; };
export type Completed = { "events": Array<ModelEvent>; [key: string]: unknown; };
export type Component = ("Question" | "UserMessage" | "AssistantMessage" | "ToolUse" | "ToolResult" | "ToolGroup" | "ToolProgress" | "CommandOutput" | "Spinner" | "TurnDuration" | "InfoNotice" | "SessionMode" | "PromptHint" | "AbovePrompt" | "Pane");
export type ConfigView = { "kernel"?: unknown; "plugins"?: { [key: string]: unknown; }; [key: string]: unknown; };
export type ContentPart = ({ "text": string; "type": "text"; [key: string]: unknown; } | Image | { "id": string; "input": unknown; "name": string; "type": "toolUse"; [key: string]: unknown; } | { "isError"?: boolean; "parts": Array<ContentPart>; "toolUseId": string; "type": "toolResult"; [key: string]: unknown; } | { "providerMetadata"?: { [key: string]: { [key: string]: unknown; }; }; "text": string; "type": "reasoning"; [key: string]: unknown; });
export type ContextUsage = { "trigger": number; "used": number; "window": number; [key: string]: unknown; };
export type DecisionKind = ("allow" | "allowSession" | "deny");
export type Delivery = ("wake" | "hold");
export type Direction = ("row" | "column");
export type DollarData = { "plugin": PluginData; [key: string]: unknown; };
export type DollarInputs = { "agent.list": Session; "agent.register": unknown; "agent.spawn": unknown; "clock.after": Timer; "clock.every": Timer; "clock.now": null; "clock.sleep": Millis; "command.list": null; "command.register": RegisterCommand; "command.run": Run; "config.list": Session; "config.read": Read; "config.set": Setting; "env.get": EnvGet; "env.set": EnvSet; "fs.ancestors": Ancestors; "fs.exists": FilePath; "fs.list": FileList; "fs.read": FileRead; "fs.stat": FilePath; "fs.write": FileWrite; "http.fetch": Fetch; "mcp.call": unknown; "mcp.connect": unknown; "model.classify": Classify; "model.complete": Complete; "model.fork": Fork; "process.kill": ProcessId; "process.read": ProcessId; "process.run": ProcessInput; "process.spawn": ProcessInput; "process.write": ProcessWrite; "prompt.read": Session; "prompt.submit": Submit; "session.append": Extend; "session.authorize": Authorize; "session.compact": Session; "session.cwd": Session; "session.id": Session; "session.messages": Messages; "session.model": Session; "session.root": Session; "session.send": Send; "session.surfaces": Session; "session.turns": Session; "session.usage": Session; "session.version": null; "state.get": StateGet; "state.set": StateSet; "store.delete": StateGet; "store.get": StateGet; "store.keys": null; "store.set": StateSet; "tool.call": Invoke; "tool.list": null; "tool.register": RegisterTool; "turn.abort": Session; };
export type DollarResults = { "agent.list": Array<SessionSummary>; "agent.register": null; "agent.spawn": ToolOutput; "clock.after": string; "clock.every": string; "clock.now": string; "clock.sleep": null; "command.list": Catalog; "command.register": string; "command.run": IntentId; "config.list": ConfigView; "config.read": unknown; "config.set": null; "env.get": (string | null); "env.set": null; "fs.ancestors": Array<string>; "fs.exists": boolean; "fs.list": Array<string>; "fs.read": string; "fs.stat": FileStat; "fs.write": null; "http.fetch": Fetched; "mcp.call": ToolOutput; "mcp.connect": null; "model.classify": (string | null); "model.complete": Completed; "model.fork": Completed; "process.kill": null; "process.read": ProcessOutput; "process.run": ProcessOutput; "process.spawn": string; "process.write": null; "prompt.read": PromptView; "prompt.submit": IntentId; "session.append": null; "session.authorize": Answer; "session.compact": IntentId; "session.cwd": string; "session.id": SessionId; "session.messages": HistoryChunk; "session.model": (string | null); "session.root": string; "session.send": null; "session.surfaces": Array<ClientIdentity>; "session.turns": Array<TurnId>; "session.usage": UsageView; "session.version": string; "state.get": (StateValue | null); "state.set": StateValue; "store.delete": boolean; "store.get": (StateValue | null); "store.keys": Array<string>; "store.set": StateValue; "tool.call": ToolOutput; "tool.list": Catalog; "tool.register": string; "turn.abort": null; };
export type Driver = ("model" | "log");
export type Effort = ("minimal" | "low" | "medium" | "high" | "xHigh" | "max");
export type Encoding = ("utf8" | "base64");
export type EnvGet = { "name": string; "session": SessionId; [key: string]: unknown; };
export type EnvSet = { "name": string; "session": SessionId; "value": string; [key: string]: unknown; };
export type Extend = { "key": string; "session": SessionId; "value": unknown; [key: string]: unknown; };
export type Fetch = { "body"?: (string | null); "headers"?: ({ [key: string]: string; } | null); "method"?: (string | null); "session": SessionId; "url": string; [key: string]: unknown; };
export type Fetched = { "body": string; "headers": { [key: string]: string; }; "notice"?: (string | null); "status": number; "truncated": boolean; [key: string]: unknown; };
export type FileList = { "limit"?: (number | null); "path": string; "session": SessionId; [key: string]: unknown; };
export type FilePath = { "path": string; "session": SessionId; [key: string]: unknown; };
export type FileRead = { "encoding"?: (Encoding | null); "path": string; "session": SessionId; [key: string]: unknown; };
export type FileStat = { "directory": boolean; "file": boolean; "size": number; [key: string]: unknown; };
export type FileWrite = { "body": string; "encoding"?: (Encoding | null); "path": string; "session": SessionId; [key: string]: unknown; };
export type FinishReason = { "raw"?: (string | null); "unified": UnifiedFinish; [key: string]: unknown; };
export type Fork = { "prompt": string; "session": SessionId; "turn"?: (TurnId | null); [key: string]: unknown; };
export type HistoryChunk = { "generation": number; "items": Array<Item>; "next"?: (ItemId | null); "oversized"?: (OversizedItem | null); [key: string]: unknown; };
export type HistoryPage = { "before"?: (ItemId | null); "generation"?: (number | null); "limit"?: number; "maxBytes"?: (number | null); [key: string]: unknown; };
export type Image = { "data": string; "height"?: (number | null); "mediaType": string; "path"?: (string | null); "width"?: (number | null); [key: string]: unknown; };
export type Input = ({ "delivery"?: Delivery; "images"?: Array<Image>; "kind": "text"; "origin": Origin; "text": string; [key: string]: unknown; } | { "action": Action; "kind": "action"; [key: string]: unknown; });
export type IntentId = string;
export type InteractionId = string;
export type InteractionKind = ({ "kind": "permission"; "preview"?: (Preview | null); "sessionScope"?: (string | null); "summary": string; "tool": string; [key: string]: unknown; } | Question | { "kind": "form"; "questions": Array<Question>; "title"?: (string | null); [key: string]: unknown; } | { "detail": string; "kind": "confirm"; "title": string; [key: string]: unknown; } | { "flow": LoginFlow; "kind": "login"; "provider": string; [key: string]: unknown; });
export type Invoke = { "call": ToolCall; "session": SessionId; [key: string]: unknown; };
export type Item = { "body": ItemBody; "completedAt"?: (string | null); "id": ItemId; "intent"?: (IntentId | null); "meta"?: { [key: string]: unknown; }; "round"?: number; "startedAt": string; "status": ItemStatus; "turn"?: (TurnId | null); [key: string]: unknown; };
export type ItemBody = ({ "kind": "user"; "origin": Origin; "parts": Array<ContentPart>; [key: string]: unknown; } | { "kind": "assistant"; "text": string; [key: string]: unknown; } | { "kind": "reasoning"; "providerMetadata"?: { [key: string]: { [key: string]: unknown; }; }; "text": string; [key: string]: unknown; } | { "callId": string; "durationMs"?: (number | null); "input": unknown; "kind": "toolCall"; "name": string; "output"?: (ToolOutput | null); "progress"?: (string | null); "quiet"?: boolean; [key: string]: unknown; } | { "command": string; "cwd": string; "exit"?: (number | null); "kind": "shell"; "output": string; [key: string]: unknown; } | { "args"?: unknown; "kind": "action"; "name": string; "result"?: unknown; [key: string]: unknown; } | { "after": number; "before": number; "durationMs": number; "kind": "compaction"; "replaced": number; "summary": string; [key: string]: unknown; } | { "dropped": number; "kind": "rewind"; "toTurn": TurnId; [key: string]: unknown; } | { "kind": "interruption"; "marker": string; [key: string]: unknown; } | { "code": string; "kind": "notice"; "level": Level; "text": string; [key: string]: unknown; } | { "answer": string; "interaction": InteractionId; "kind": "questionAnswer"; "question": string; [key: string]: unknown; } | { "decision": DecisionKind; "feedback"?: (string | null); "interaction": InteractionId; "kind": "permissionReceipt"; "tool": string; [key: string]: unknown; } | { "asset": string; "kind": "asset"; "label"?: (string | null); [key: string]: unknown; } | { [key: string]: unknown; });
export type ItemId = string;
export type ItemStatus = ("pending" | "running" | "completed" | "failed" | "interrupted");
export type Justify = ("flex-start" | "center" | "flex-end" | "space-between" | "space-around" | "space-evenly");
export type Level = ("info" | "warn" | "error");
export type LoginFlow = ({ "kind": "browser"; "url": string; [key: string]: unknown; } | { "code": string; "kind": "device"; "url": string; [key: string]: unknown; } | { "kind": "paste"; [key: string]: unknown; });
export type Message = { "parts": Array<ContentPart>; "providerOptions"?: { [key: string]: { [key: string]: unknown; }; }; "role": Role; [key: string]: unknown; };
export type Messages = { "page": HistoryPage; "session": SessionId; [key: string]: unknown; };
export type Millis = { "ms": number; [key: string]: unknown; };
export type ModelEvent = ({ "type": "streamStart"; "warnings"?: Array<string>; [key: string]: unknown; } | { "id"?: (string | null); "model"?: (string | null); "type": "responseMetadata"; [key: string]: unknown; } | { "id": string; "type": "textStart"; [key: string]: unknown; } | { "delta": string; "id": string; "type": "textDelta"; [key: string]: unknown; } | { "id": string; "type": "textEnd"; [key: string]: unknown; } | { "id": string; "type": "reasoningStart"; [key: string]: unknown; } | { "delta": string; "id": string; "type": "reasoningDelta"; [key: string]: unknown; } | { "id": string; "providerMetadata"?: { [key: string]: { [key: string]: unknown; }; }; "type": "reasoningEnd"; [key: string]: unknown; } | { "id": string; "name": string; "type": "toolInputStart"; [key: string]: unknown; } | { "delta": string; "id": string; "type": "toolInputDelta"; [key: string]: unknown; } | { "id": string; "type": "toolInputEnd"; [key: string]: unknown; } | { "id": string; "input": string; "name": string; "type": "toolCall"; [key: string]: unknown; } | { "type": "context"; "used": number; "window": number; [key: string]: unknown; } | { "after": number; "before": number; "type": "compacted"; [key: string]: unknown; } | { "finishReason": FinishReason; "type": "finish"; "usage": Usage; [key: string]: unknown; });
export type ModelRequest = { "maxTokens": number; "messages": Array<Message>; "model": string; "providerOptions"?: { [key: string]: { [key: string]: unknown; }; }; "reasoning"?: (Effort | null); "session"?: (SessionId | null); "system": Array<SystemBlock>; "tools": Array<ToolSpec>; [key: string]: unknown; };
export type Origin = { "conversation"?: (string | null); "principal"?: (string | null); "surface": string; "voice"?: (Voice | null); [key: string]: unknown; };
export type OversizedItem = { "checksum": string; "id": ItemId; "totalBytes": number; [key: string]: unknown; };
export type ParentLink = { "item"?: (ItemId | null); "session": SessionId; [key: string]: unknown; };
export type PluginData = { "name": string; "root": string; "version"?: (string | null); [key: string]: unknown; };
export type Preview = ({ "kind": "diff"; "unified": string; [key: string]: unknown; } | { "command": string; "cwd": string; "description"?: (string | null); "kind": "command"; [key: string]: unknown; } | { "kind": "url"; "url": string; [key: string]: unknown; });
export type ProcessId = { "id": string; "session": SessionId; [key: string]: unknown; };
export type ProcessInput = { "argv": Array<string>; "cwd"?: (string | null); "session": SessionId; "timeoutMs"?: (number | null); [key: string]: unknown; };
export type ProcessOutput = { "code"?: (number | null); "stderr": string; "stdout": string; [key: string]: unknown; };
export type ProcessWrite = { "body"?: (string | null); "id": string; "session": SessionId; [key: string]: unknown; };
export type PromptView = { "last"?: (string | null); "queue": Array<QueueEntry>; [key: string]: unknown; };
export type Question = { "freeText"?: boolean; "header"?: (string | null); "multi"?: boolean; "options": Array<QuestionOption>; "question": string; [key: string]: unknown; };
export type QuestionOption = { "description"?: (string | null); "id": string; "label": string; "preview"?: (string | null); "role"?: (AnswerRole | null); [key: string]: unknown; };
export type QueueEntry = { "intent": IntentId; "origin": Origin; "position": number; "preview": string; "steerable": boolean; [key: string]: unknown; };
export type Read = { "key": string; "session": SessionId; [key: string]: unknown; };
export type RegisterCommand = { "callback": string; "session": SessionId; "spec": CommandSpec; [key: string]: unknown; };
export type RegisterTool = { "callback": string; "session": SessionId; "spec": ToolSpec; [key: string]: unknown; };
export type Role = ("user" | "assistant");
export type Run = { "args": unknown; "name": string; "session": SessionId; [key: string]: unknown; };
export type SelectOption = { "label": string; "value": string; [key: string]: unknown; };
export type Send = { "delivery": Delivery; "input": Input; "to": SessionId; [key: string]: unknown; };
export type Session = { "session": SessionId; [key: string]: unknown; };
export type SessionId = string;
export type SessionSummary = { "asking"?: boolean; "busy"?: boolean; "createdAt": string; "cwd": string; "driver"?: Driver; "id": SessionId; "key"?: (string | null); "messages"?: (number | null); "model"?: (string | null); "parent"?: (ParentLink | null); "provider"?: (string | null); "systemExtra"?: (string | null); "title"?: (string | null); "tools"?: (Array<string> | null); "updatedAt": string; "usage"?: Usage; [key: string]: unknown; };
export type Setting = { "key": string; "session": SessionId; "value": unknown; [key: string]: unknown; };
export type StateGet = { "key": string; "scope": string; [key: string]: unknown; };
export type StateSet = { "ifVersion"?: (number | null); "key": string; "scope": string; "value": unknown; [key: string]: unknown; };
export type StateValue = { "value": unknown; "version": number; [key: string]: unknown; };
export type Submit = { "input": Input; "session": SessionId; [key: string]: unknown; };
export type SystemBlock = { "cache"?: boolean; "text": string; [key: string]: unknown; };
export type TextChild = (string | View);
export type Timer = { "callback": string; "ms": number; "session": SessionId; [key: string]: unknown; };
export type Tone = (("neutral" | "good" | "bad") | "attention");
export type ToolCall = { "callId": string; "input": unknown; "name": string; [key: string]: unknown; };
export type ToolOutput = { "display"?: (View | null); "isError"?: boolean; "parts": Array<ContentPart>; [key: string]: unknown; };
export type ToolSpec = { "description": string; "inputSchema": unknown; "meta"?: { [key: string]: unknown; }; "name": string; [key: string]: unknown; };
export type TreeNode = { "badge"?: (string | null); "children"?: Array<TreeNode>; "label": string; "tone"?: Tone; [key: string]: unknown; };
export type TurnId = string;
export type UnifiedFinish = ("stop" | "length" | "toolCalls" | "contentFilter" | "error" | "other");
export type Usage = { "cacheReadTokens"?: number; "cacheWriteTokens"?: number; "inputTokens": number; "outputTokens": number; "reasoningTokens"?: number; [key: string]: unknown; };
export type UsageView = { "context"?: (ContextUsage | null); "lastTurn"?: (Usage | null); "session": Usage; [key: string]: unknown; };
export type View = ({ "bold"?: (boolean | null); "children": Array<TextChild>; "color"?: (string | null); "dimColor"?: (boolean | null); "inverse"?: (boolean | null); "italic"?: (boolean | null); "kind": "Text"; "strikethrough"?: (boolean | null); "underline"?: (boolean | null); "wrap"?: (string | null); [key: string]: unknown; } | { "dimColor"?: boolean; "kind": "Markdown"; "text": string; [key: string]: unknown; } | { "kind": "Code"; "language"?: (string | null); "source": string; [key: string]: unknown; } | { "kind": "diff"; "unified": string; [key: string]: unknown; } | { "items": Array<string>; "kind": "list"; [key: string]: unknown; } | { "headers": Array<string>; "kind": "table"; "rows": Array<Array<string>>; [key: string]: unknown; } | { "kind": "keyValue"; "rows": Array<[string, string, ...Array<unknown>]>; [key: string]: unknown; } | { "kind": "progress"; "label"?: (string | null); "total"?: (number | null); "value": number; [key: string]: unknown; } | { "kind": "badge"; "text": string; "tone"?: Tone; [key: string]: unknown; } | { "kind": "tree"; "nodes": Array<TreeNode>; [key: string]: unknown; } | { "alignItems"?: (Align | null); "borderColor"?: (string | null); "borderStyle"?: (string | null); "children": Array<View>; "display"?: (string | null); "flexDirection"?: (Direction | null); "flexGrow"?: (number | null); "flexWrap"?: (Wrap | null); "gap"?: (number | null); "height"?: (number | null); "justifyContent"?: (Justify | null); "key"?: (string | null); "kind": "Box"; "margin"?: (number | null); "marginX"?: (number | null); "marginY"?: (number | null); "padding"?: (number | null); "paddingX"?: (number | null); "paddingY"?: (number | null); "width"?: (number | null); [key: string]: unknown; } | { "child": View; "kind": "panel"; "title": string; [key: string]: unknown; } | { "items": Array<ActionItem>; "kind": "actions"; [key: string]: unknown; } | { "children"?: Array<TextChild>; "dimColor"?: (boolean | null); "hotkey"?: (string | null); "key"?: (string | null); "kind": "Button"; "label"?: (string | null); "onPress"?: (Action | null); "variant"?: (string | null); [key: string]: unknown; } | { "key"?: (string | null); "kind": "Input"; "label"?: (string | null); "onInput"?: (Action | null); "onSubmit"?: (Action | null); "placeholder"?: (string | null); "value"?: (string | null); [key: string]: unknown; } | { "key"?: (string | null); "kind": "Select"; "label"?: (string | null); "onSelect"?: (Action | null); "options"?: (Array<SelectOption> | null); "value"?: (string | null); [key: string]: unknown; } | { "href": string; "kind": "Link"; "label"?: (string | null); [key: string]: unknown; } | { "alt": string; "columns"?: (number | null); "kind": "Image"; "rows"?: (number | null); "source": string; [key: string]: unknown; } | { "cells": Array<string>; "columns": number; "kind": "Raster"; "rows": number; [key: string]: unknown; } | { "alt": string; "height"?: (number | null); "kind": "Svg"; "source": string; "width"?: (number | null); [key: string]: unknown; } | { "component": Component; "kind": "Engine"; "props": unknown; [key: string]: unknown; } | { "customKind": string; "data": unknown; "fold": string; "kind": "custom"; [key: string]: unknown; });
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
