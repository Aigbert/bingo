//! The author's view of each `$` noun: what it is, one example, and the
//! calls whose author signature is not the row's. Rows say what a method
//! does; this table says how an author holds it.

/// One `$` noun as an author meets it.
pub struct Noun {
    pub name: &'static str,
    pub doc: &'static str,
    /// Statements run inside a hook or test that has `$` in scope.
    pub example: &'static str,
    /// Whether `bingo plugins test` can run the example without grants or
    /// services a test host lacks; the kit's test skips it otherwise.
    pub runs: bool,
}

/// A method the author SDK wraps, or a call with no row behind it. A call
/// with a row says nothing of its own: the row's doc is the description.
pub struct Call {
    pub noun: &'static str,
    pub method: &'static str,
    /// The TypeScript signature as an author calls it.
    pub signature: &'static str,
    /// Only a call no row describes carries a doc.
    pub doc: Option<&'static str>,
}

pub static NOUNS: &[Noun] = &[
    Noun {
        name: "plugin",
        doc: "This plugin's own `name`, `root` and `version`: plain data, not methods.",
        example: "const {name, root} = $.plugin;\nconst manifest = `${root}/plugin.json`;",
        runs: true,
    },
    Noun {
        name: "session",
        doc: "The session the hook runs in: its history, directories, model, usage and clients, and delivery into it. Methods take `session` from the hook unless given.",
        example: "const cwd = await $.session.cwd();\nconst {context} = await $.session.usage();\nconst model = await $.session.model();",
        runs: true,
    },
    Noun {
        name: "turn",
        doc: "The session's running turn.",
        example: "await $.turn.abort();",
        runs: true,
    },
    Noun {
        name: "prompt",
        doc: "The prompt: what is queued, and submitting input as a person would.",
        example: "const {queue, last} = await $.prompt.read();",
        runs: true,
    },
    Noun {
        name: "tool",
        doc: "The tools the model can call: list them, call one through the permission gate, or register one this plugin answers.",
        example: "await $.tool.register({\n  name: 'Shout',\n  description: 'Upper-case the given text.',\n  inputSchema: {type: 'object', properties: {text: {type: 'string'}}, required: ['text']},\n}, async ($, input) => {\n  const {text} = input as {text: string};\n  return {parts: [{type: 'text', text: text.toUpperCase()}], isError: false};\n});\nconst tools = await $.tool.list();",
        runs: true,
    },
    Noun {
        name: "command",
        doc: "Slash commands: list them, run one, or register one this plugin answers. The handler receives `($, args)`, `args` being the text typed after the name (`''` for none), and returns an outcome such as `{kind: 'applied', message}`. `$.command.run({name, args})` runs one as if typed.",
        example: "await $.command.register({name: 'hello', description: 'Say hello.', args: {kind: 'free', hint: '[name]'}}, async ($, args) => {\n  await $.ui.toast(`Hello, ${String(args) || 'world'}.`);\n  return {kind: 'applied', message: 'Said hello.'};\n});",
        runs: true,
    },
    Noun {
        name: "config",
        doc: "Settings: list the view a client sees, read one by its dotted key, or change one.",
        example: "const settings = await $.config.list();",
        runs: true,
    },
    Noun {
        name: "model",
        doc: "The host's model, asked on this plugin's behalf; every method needs the `model` grant.",
        example: "const label = await $.model.classify({text: 'the build fails on Windows', labels: ['bug', 'feature']});",
        runs: false,
    },
    Noun {
        name: "state",
        doc: "Values this plugin keeps per session, each with a version. For typed keys, `atom`, `read` and `update` from 'bingo' wrap these calls.",
        example: "const saved = await $.state.get('count');\nconst count = typeof saved?.value === 'number' ? saved.value : 0;\nawait $.state.set('count', count + 1, saved?.version);",
        runs: true,
    },
    Noun {
        name: "store",
        doc: "Values this plugin keeps across sessions, each with a version.",
        example: "await $.store.set('greeted', true);\nconst keys = await $.store.keys();\nawait $.store.delete('greeted');",
        runs: true,
    },
    Noun {
        name: "clock",
        doc: "The host's clock. Use it instead of `setTimeout`: callbacks receive a fresh `$`, and tests can set or advance the time.",
        example: "const started = await $.clock.now();\nawait $.clock.sleep(10);\nawait $.clock.after(60_000, async $ => {\n  await $.ui.toast('A minute has passed.');\n});",
        runs: true,
    },
    Noun {
        name: "fs",
        doc: "Files under the roots this plugin is granted; paths are relative to the session's cwd.",
        example: "if (await $.fs.exists({path: 'package.json'})) {\n  const text = await $.fs.read({path: 'package.json'});\n}",
        runs: true,
    },
    Noun {
        name: "http",
        doc: "HTTP through the host; needs the `http` grant.",
        example: "const {status, body} = await $.http.fetch({url: 'https://example.com/'});",
        runs: false,
    },
    Noun {
        name: "env",
        doc: "This plugin's environment overlay: variables its processes see. Needs the `env` grant.",
        example: "await $.env.set({name: 'MY_MOD_MODE', value: 'quiet'});\nconst mode = await $.env.get({name: 'MY_MOD_MODE'});",
        runs: false,
    },
    Noun {
        name: "process",
        doc: "Programs run by argv, never through a shell; needs the `process` grant.",
        example: "const {code, stdout} = await $.process.run({argv: ['git', 'status', '--short']});\nconst job = await $.process.spawn({argv: ['sleep', '5']});\nawait job.kill();",
        runs: false,
    },
    Noun {
        name: "agent",
        doc: "Sub-agents, through the agent service.",
        example: "const children = await $.agent.list();\nawait $.agent.spawn({args: {prompt: 'List the open TODOs.'}});",
        runs: false,
    },
    Noun {
        name: "mcp",
        doc: "MCP servers' tools, through the MCP service.",
        example: "const output = await $.mcp.call({server: 'docs', tool: 'search', args: {query: 'bingo'}});",
        runs: false,
    },
    Noun {
        name: "ui",
        doc: "The interface: status entries, toasts and notices, pulled panes, re-rendering, and questions to the person. Drawing itself is a `ui.render` hook.",
        example: "await $.ui.status('Ready', {key: 'demo'});\nawait $.ui.toast('Saved.');\nawait $.ui.open({id: 'demo', title: 'Demo'});\nawait $.ui.close('demo');",
        runs: true,
    },
];

pub static CALLS: &[Call] = &[
    Call {
        noun: "state",
        method: "get",
        signature: "get(key: string): Promise<StateValue | null>",
        doc: None,
    },
    Call {
        noun: "state",
        method: "set",
        signature: "set(key: string, value: unknown, ifVersion?: number): Promise<StateValue>",
        doc: None,
    },
    Call {
        noun: "store",
        method: "get",
        signature: "get(key: string): Promise<StateValue | null>",
        doc: None,
    },
    Call {
        noun: "store",
        method: "set",
        signature: "set(key: string, value: unknown, ifVersion?: number): Promise<StateValue>",
        doc: None,
    },
    Call {
        noun: "store",
        method: "delete",
        signature: "delete(key: string): Promise<boolean>",
        doc: None,
    },
    Call {
        noun: "clock",
        method: "sleep",
        signature: "sleep(ms: number): Promise<null>",
        doc: None,
    },
    Call {
        noun: "clock",
        method: "after",
        signature: "after(ms: number, handler: Callback): Promise<string>",
        doc: None,
    },
    Call {
        noun: "clock",
        method: "every",
        signature: "every(ms: number, handler: Callback): Promise<string>",
        doc: None,
    },
    Call {
        noun: "model",
        method: "complete",
        signature: "complete(request: ModelRequest): Promise<Completed>",
        doc: None,
    },
    Call {
        noun: "tool",
        method: "register",
        signature: "register(spec: ToolSpec, handler: Callback): Promise<string>",
        doc: None,
    },
    Call {
        noun: "command",
        method: "register",
        signature: "register(spec: CommandRegistration, handler?: Callback): Promise<string>",
        doc: None,
    },
    Call {
        noun: "process",
        method: "spawn",
        signature: "spawn(input: AtSite<DollarInputs['process.spawn']>): Promise<ProcessHandle>",
        doc: None,
    },
    Call {
        noun: "session",
        method: "deliver",
        signature: "deliver(input: Input, wake?: boolean): Promise<null>",
        doc: Some(
            "Deliver text input into this session as this plugin's message; `wake` (default `true`) opens a turn when the session is idle, otherwise it waits for the next one.",
        ),
    },
    Call {
        noun: "session",
        method: "extend",
        signature: "extend(key: string, value: unknown): Promise<null>",
        doc: Some(
            "Record this plugin's state for `key` in the session's journal; the value replaces the whole of `key` and survives a restart.",
        ),
    },
    Call {
        noun: "ui",
        method: "status",
        signature: "status(text: string | null | undefined, options?: {key?: string; tone?: Wire.Tone}): Promise<null>",
        doc: Some("Place a status entry under `key` (default `status`); `null` removes it."),
    },
    Call {
        noun: "ui",
        method: "toast",
        signature: "toast(text: string, options?: {duration?: number}): Promise<null>",
        doc: Some("Show a short-lived notification; `duration` is in milliseconds."),
    },
    Call {
        noun: "ui",
        method: "notify",
        signature: "notify(text: string, options?: {duration?: number}): Promise<null>",
        doc: Some("Like `toast`, but marked as an alert for surfaces that raise one."),
    },
    Call {
        noun: "ui",
        method: "open",
        signature: "open(options: OpenPane): Promise<Record<string, never>>",
        doc: Some(
            "Publish a pulled pane; draw it by matching `ui.render` with `{component: 'Pane', requestId: id}`.",
        ),
    },
    Call {
        noun: "ui",
        method: "close",
        signature: "close(id: string): Promise<null>",
        doc: Some("Remove the pane this plugin opened as `id`."),
    },
    Call {
        noun: "ui",
        method: "panes",
        signature: "panes(): Promise<Wire.Slot[]>",
        doc: Some("The panes and slots placed in this session."),
    },
    Call {
        noun: "ui",
        method: "invalidate",
        signature: "invalidate(requests: string | string[]): Promise<null>",
        doc: Some("Ask surfaces to render these request ids again."),
    },
    Call {
        noun: "ui",
        method: "resolve",
        signature: "resolve(event: {surface: Wire.SurfaceName}): ElementTable",
        doc: Some(
            "The elements the asking surface draws, by name, such as `Box`, `Text` and `Button`; pass the `ui.render` event.",
        ),
    },
    Call {
        noun: "ui",
        method: "log",
        signature: "log(text: string): Promise<null>",
        doc: Some("Write a line to bingo's log under `mod::<name>`."),
    },
    Call {
        noun: "ui",
        method: "notice",
        signature: "notice(text: string, options?: {level?: Wire.Level}): Promise<null>",
        doc: Some("Post a notice to the session's event stream."),
    },
    Call {
        noun: "ui",
        method: "ask",
        signature: "ask(kind: Wire.InteractionKind, answers: Wire.AnswerSpec[]): Promise<Wire.Answer>",
        doc: Some("Ask the person a question with these answer kinds and wait for the answer."),
    },
];

/// The calls `noun` carries in place of, or beside, its rows.
pub fn calls(noun: &str) -> impl Iterator<Item = &'static Call> + '_ {
    CALLS.iter().filter(move |call| call.noun == noun)
}
