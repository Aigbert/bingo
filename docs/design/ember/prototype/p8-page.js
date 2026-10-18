/* ---------- page: scenes, the paint loop, and the design system drawn by the same renderer ---------- */
const K = k => `<kbd>${esc(k)}</kbd>`;
const CD = t => `<code>${esc(t)}</code>`;
const TITLE = RECENTS[0].t;
const QUEUED = '做完以后更新一下 CHANGELOG';
let SIZE = [100, 32];

/* ----- scene setups: the same engine, put into a state ----- */
function past() { S.log.push(T1(), T2done()); if (!S.title) S.title = TITLE; }
function liveSetup(ago) { S.mode = 1; S.log.push(T1()); return startTurn(T2P, mainScript({}), { t0: NOW() - ago, title: TITLE }); }
const sheetSetup = name => () => { past(); openSheet(name); };
const CREW_P = '让 scout 和 reviewer 并行：一个找 drain() 的调用点，一个审 queue.rs 的改动';
/* a teammate already some way into its task */
function ago(id, ms) { withSession(id, () => { const t = liveTurn(); if (t) { t.t0 -= ms; runScript(t, NOW()); } }); }

const SCENES = [
  { id: 'welcome', grp: '对话', name: '开场', t: 2600,
    card: '还没说话。字标、光标和最近的会话。',
    setup() { S.welcome = { at: NOW(), sel: 0 }; },
    what: '刚打开 bingo、还没开口的时候。光标把字标写出来，火星从光标上飘起；下面一行是模型、思考强度和模式，一行是目录和分支，再下面是最近的三个会话。',
    changed: [
      '开场只有一件事要做：开口。所以焦点就在输入行，最近的会话放在 ↑ 上，不抢位置。',
      '字标由光标写出（900 ms），火星只在开场出现；之后整个会话里不再有装饰动画。',
      '模型和目录各占一行、用同一种灰，读起来像一句话，不像一张表。',
      '一开口，字标化成点散掉（520 ms），把屏幕让给对话。',
    ],
    tryit: [`${K('↑')} 从最下面一条开始选最近的会话，${K('⏎')} 接着聊`, `直接打字再按 ${K('⏎')}，看字标散开`, `${K('?')} 打开按键表，${K('/')} 打开命令`] },
  { id: 'live', grp: '对话', name: '在干活', t: 700,
    card: '左边的光在呼吸，每一步落成一行，正在跑的一步在转圈。',
    setup() { liveSetup(2600); },
    what: '一轮进行中。你的话在一条浅色带上，右边是这一轮用了多久，带子左边缘那道光是这一轮的状态。下面每一步一行：图标、动词、参数、结果，bingo 的旁白夹在步骤之间；正在跑的一步在图标的位置转圈，最后一行输出贴在它下面。输入框的上沿有一颗彗星在跑。',
    changed: [
      `“在干活”只用三个信号：边光呼吸（1.8 s）、上沿彗星（2.2 s）、正在跑的一步转圈。没有百分比、token 计数和 ${CD('Thinking…')} 标题。`,
      `一行一行往下推，做完也不收起。每种工具有自己的图标：${CD('¶')} 读、${CD('±')} 改、${CD('$')} 跑；转圈跑完，落成这一步的图标。`,
      '查东西的步骤（读、找、搜）暗一档，动手的步骤（改、写、跑）亮一档，一眼看出这一轮动了什么。',
      '输入框一直可用：⏎ 是转向（下一步开始前送到），tab 是排队（这一轮结束后发）。',
    ],
    tryit: [`打一句话按 ${K('⏎')} 转向，或按 ${K('tab')} 排队`, `bash 在跑时按 ${K('ctrl+b')} 放到后台`, `${K('esc')} 停下这一轮`] },
  { id: 'done', grp: '对话', name: '做完了', t: 0,
    card: '刚做完的一轮左边亮一道绿光；每一步留在原地，一步一行。',
    setup: past,
    what: '两轮都做完了。每一轮三层：你的话、每步一行、回答；用时写在你的话右边。刚做完的那轮，带子左边缘亮着一道绿光；更早的不亮，一眼就知道哪一轮是刚才那轮。',
    changed: [
      `去掉了 ${CD('▸ 6 steps')} 和 ${CD('Worked for 6s · done 21:58')}：步骤不折叠，用时写在你的话右边，暗色。`,
      `中途失败又修好的测试只留一行：图标和动词变红，右边写 ${CD('exit 101')}，输出不展开；只有最后一步失败，输出才留在屏幕上。`,
      '回答正文最宽 100 列。',
      '做完那一刻边光从橙色变绿（320 ms），从光上溅出三颗火星（700 ms），上沿闪一下绿光，然后归于安静。',
    ],
    tryit: ['点任意一步，打开它的输出或 diff', `${K('ctrl+o')} 看这一轮的细节：想了什么、你什么时候插的话`, `${K('esc')} ${K('esc')} 回到某句话之前`] },
  { id: 'ledger', grp: '对话', name: '细节', t: 0,
    card: 'ctrl+o：想过的和插过的话回到原位，失败那步的输出就地打开。',
    setup() {
      past(); toggleLedger();
      const t = S.sel.t, i = t.items.findIndex(it => it.k === 'step' && it.st === 'fail');
      if (i >= 0) { S.sel.i = i; t.well = i; }
      S.wantSel = true;
    },
    what: `ctrl+o 打开最后一轮的细节：步骤还在原地，中间多出 bingo 想过的几段（${CD('≈ think')}，打开看全文）和你插过的话，都回到发生的位置。选中的是失败的那一步，它的输出就地打开在更深的底色上。`,
    changed: [
      '细节不换屏、不弹窗，原地多出几行，esc 收起。',
      `diff 去掉了 ${CD('---/+++/@@')}，只留行号、+/− 和改动；一行里改了哪几个字用更深的底色标出。`,
      '输出和 diff 放在“井”里：比背景深一档，从动词那一列开始，不需要框线。',
      `没有 ▸/▾，也不在每行右边写 ${CD('you: yes')}：只有自动放行的步骤才写原因（accept edits、session rule）。`,
    ],
    tryit: [`${K('↑')} ${K('↓')} 换一步，${K('⏎')} 打开或收起它的输出`, `${K('esc')} 或 ${K('ctrl+o')} 收起`] },
  { id: 'stop', grp: '对话', name: '停下', t: 2600,
    card: 'esc 停下这一轮，排队的话回到输入框。',
    setup() {
      S.log.push(T1());
      const t = startTurn(T2P, mainScript({}), { t0: NOW() - 2000, title: TITLE });
      S.queued.push({ text: QUEUED, at: NOW() });
      later(1800, () => { if (isLive(t)) { halt(t); flashKey('esc'); } });
    },
    what: '这一轮做到一半，后面还排着一句“做完以后更新一下 CHANGELOG”。1.8 秒时自动按下 esc：正在跑的那一步停在原地，图标变暗，右边写 stopped；你那句话右边写着 stopped 和用时；排队的那句回到输入框。',
    changed: [
      '一个 esc 就停，不确认，也不退出程序。',
      '排队的消息不丢，也不自己发出去：它回到输入框，决定权回到你手里。',
      '退出要在空输入行上连按两次 ctrl+c。',
    ],
    tryit: ['点工具栏的「重放」再看一次', `${K('⏎')} 把回到输入框的话发出去`, `清空输入后连按两次 ${K('ctrl+c')} 退出`] },
  { id: 'steer', grp: '对话', name: '转向和排队', t: 200,
    card: '转向在下一步送到，排队等这一轮结束。',
    setup() {
      const t = liveSetup(1600);
      runScript(t, NOW());
      S.queued.push({ text: '测试名字用英文', steer: true, at: NOW() }, { text: QUEUED, at: NOW() });
    },
    what: '一轮进行中，你说了两句：一句转向“测试名字用英文”，一句排队“做完以后更新一下 CHANGELOG”。它们列在输入框上方，右边写着各自什么时候送到。',
    changed: [
      '⏎ 是转向：下一步开始前送到，送到后并进你这一轮的话里，右边标 steered；ctrl+o 能看到它在哪一步被听到。',
      'tab 是排队：这一轮结束后自动发出。',
      `右边写 ${CD('at the next step')} 或 ${CD('queued')}。`,
      '空输入时按 ↑ 把最后排队的那句拿回来改。',
    ],
    tryit: ['等一下，看“测试名字用英文”送到后并进你的话里', `${K('↑')} 拿回排队的那句`, `再打一句按 ${K('tab')} 排队`] },
  { id: 'shell', grp: '对话', name: '直接跑命令', t: 600,
    card: '! 开头直接跑：不经过模型，命令和输出就是全部。',
    setup() { S.log.push(T1()); S.title = TITLE; send('!git status --short'); },
    what: '以 ! 开头的一行直接交给 shell，不经过模型。命令显示在带上，输出放在井里，不折叠。',
    changed: [
      '查个 git status 不该花一次模型调用：跑的时候是一行 $ run，跑完只留输出，原样展示。',
      '! 用代码色，和命令连成一体。',
      '进历史，↑ 能找回；ctrl+o 和回退都跳过它。',
    ],
    tryit: [`输入 ${CD('!git log --oneline -3')} 再按 ${K('⏎')}`, `输入 ${CD('!ls')} 再按 ${K('⏎')}`] },
  { id: 'image', grp: '对话', name: '图片', t: 0,
    card: '贴进来的图是一个标签，跟着这句话走。',
    setup() { past(); S.chips = ['image 1']; S.images = 1; S.input = '这个报错什么意思'; },
    what: '输入框里贴了一张截图，显示成 [image 1] 标签，后面是你的问题。⏎ 发出去，bingo 读图回答。',
    changed: [
      '图片不在终端里画，只占一个标签。',
      '当前模型读不了图时，贴图那一刻就提示 alt+m 换模型。',
      '删光文字后再按 ⌫ 删掉标签。',
    ],
    tryit: [`${K('⏎')} 发出去`, `${K('ctrl+v')} 再贴一张`, `${K('alt+m')} 换到只读文字的模型再贴，看提示`] },

  { id: 'ask-edit', grp: '询问', name: '要改文件', t: 700,
    card: '琥珀色上沿在呼吸：这一步等你点头。',
    setup() { S.log.push(T1()); startTurn(T2P, mainScript({ askEdit: true }), { t0: NOW() - 1500, title: TITLE }); },
    what: 'bingo 要改 queue.rs。输入框的位置换成了这个问题：先给你看 diff，下面是三个选项。上沿变成琥珀色、慢慢呼吸，这一轮的边光也换成琥珀色。',
    changed: [
      '问题出现在输入框的位置，不是屏幕中间的弹窗。',
      '选项不编号、不用字母键：↑↓ 选，⏎ 确认，默认停在 Yes。',
      '刚出现的 400 ms 里按键无效，免得手快误按。',
      `直接打字就是 ${CD('No, and tell it why')}。`,
    ],
    tryit: [`${K('↓')} ${K('⏎')} 允许这个会话里改这个目录`, '直接打字写下为什么不行', `${K('esc')} 拒绝并停下这一轮`] },
  { id: 'ask-bash', grp: '询问', name: '要跑命令', t: 700,
    card: '命令用代码色，下一行说为什么要跑。',
    setup() { S.log.push(T1()); startTurn('跑一下 queue 的测试', testScript(), { title: TITLE }); },
    what: `bingo 要跑 ${CD('cargo test -p bingo-core queue')}。标题 ${CD('Run this command?')}，右边写着在哪个目录跑；命令单独一行，下面一行是为什么。`,
    changed: [
      '先说做什么再说为什么：命令最醒目，理由灰色。',
      `四个选项：Yes、这个会话里都允许这条命令、${CD('No, and tell it why')}、${CD('Change it first')}。`,
      `改过的步骤右边标 ${CD('changed by you')}。`,
    ],
    tryit: [`${K('↓')}${K('↓')}${K('↓')}${K('⏎')} 先改命令再跑`, `${K('⏎')} 直接允许，看测试跑完`] },
  { id: 'caution', grp: '询问', name: '危险操作', t: 800,
    card: '红色上沿不呼吸；默认停在“不”。',
    setup() { past(); startTurn('把 dev 强推上去', forceScript(), { t0: NOW() - 1300 }); },
    what: `bingo 要强推 dev。这会改写远端历史，所以问题换成红色：上沿是静止的红线，右上写着 ${CD('rewrites remote history')}，下面列出会被丢掉的两个提交。`,
    changed: [
      `长得和普通询问一样，只换颜色和默认项：默认 ${CD('No, and tell it why')}。`,
      '不问“你确定吗”，而是说清后果：丢掉哪两个只有 origin/dev 才有的提交。',
      `允许项自己也是红色：${CD('Yes, force-push')}。`,
      '没有“这个会话里都允许”，bypass 也不替你回答。',
    ],
    tryit: [`${K('⏎')} 选“不”，写下理由`, `${K('↑')} ${K('⏎')} 强推（原型里没关系）`] },
  { id: 'question', grp: '询问', name: '提问', t: 800,
    card: '两个问题，←→ 切换，第二个可以多选。',
    setup() { S.mode = 1; S.log.push(T1()); startTurn(T2P, mainScript({ question: true }), { t0: NOW() - 1900, title: TITLE }); },
    what: 'bingo 需要你做两个决定：回归测试放在哪、要覆盖哪些情况。问题在输入框的位置，上面一行是两个问题的标签，选中的选项旁边有预览。',
    changed: [
      '相关的问题一次问完，←→ 切换。',
      '第一个问题每个选项都带预览。',
      '多选用 space 打勾，⏎ 确认。',
      `最后总有一项 ${CD('Something else')}，可以自己写。`,
      '选“单元测试”，测试就写进 queue.rs 底部。',
    ],
    tryit: [`${K('↓')} 看单元测试的预览，${K('⏎')} 选它`, `第二个问题里按 ${K('space')} 勾几项，${K('⏎')} 确认`, `${K('←')} ${K('→')} 切换`] },
  { id: 'mode', grp: '询问', name: '模式', t: 2300,
    card: 'shift+tab 换模式：上沿闪一下它的颜色，页脚记住它。',
    setup() { past(); [600, 2000, 3400, 4800].forEach(at => later(at, () => { cycleMode(); flashKey('shift+tab'); })); },
    what: '每 1.4 秒自动按一次 shift+tab：default → accept edits → plan → bypass → default。每换一次，上沿闪一下这个模式的颜色，页脚出现模式名和一句说明。',
    changed: [
      '模式是会话的状态，住在页脚。',
      'default 时页脚不写模式；bypass 是红色。',
      '确认就是上沿闪一下（900 ms）加一句说明，不弹窗。',
    ],
    tryit: [`${K('⇧tab')} 自己切几次`, '切到 plan 再输入“修好它”，看它只给计划、不动文件'] },

  { id: 'palette', grp: '面板', name: '命令', t: 0,
    card: '/ 打开命令：模糊匹配，快捷键写在右边。',
    setup() { past(); S.input = '/'; },
    what: `输入 / 后，输入框上方列出命令：命令名在左，说明在中间，快捷键在右。最多 8 行，下面写着 ${CD('8 of 29')}。`,
    changed: [
      '命令和技能在同一个列表里，按匹配程度排：前缀、包含、按顺序出现的字母。',
      '匹配上的字母用 ember 加粗。',
      '快捷键写在右边，用几次就不必再打开。',
      '列表一行一行升起，收起时立刻消失。',
    ],
    tryit: [`接着打字过滤，比如 ${CD('/st')}`, `${K('↑')}${K('↓')} 选择，${K('⏎')} 执行，${K('tab')} 只补全`, `${K('esc')} 清掉`] },
  { id: 'skills', grp: '面板', name: '技能', t: 0,
    card: '/re：命令在前，技能在后，标着来源。',
    setup() { past(); S.input = '/re'; },
    what: `输入 ${CD('/re')}：resume、rewind、rename 是命令，review、release 是技能，后面还有模糊匹配到的 experience 和 provider。技能右边写 skill 和来源：project、yours、built in 或 guide。`,
    changed: ['技能和命令共用 /，区别只在右边的来源。', '命令作用于会话；技能是一句交给 bingo 的话，⏎ 就发出去。'],
    tryit: [`${K('↓')} 选到 review，${K('⏎')} 发出去`, '删掉 re 换个词试试'] },
  { id: 'mention', grp: '面板', name: '提到文件', t: 0,
    card: '@ 列出文件，改过的、新建的标在右边。',
    setup() { past(); S.input = '看一下 @queue'; },
    what: `输入 ${CD('@queue')}，输入框上方列出匹配的文件：路径在左，目录部分暗一些；右边标着 modified 或 new。`,
    changed: ['git 状态写在右边：改过的琥珀色，新建的绿色。', 'tab 或 ⏎ 插入完整路径。', '队友也在列表里，用它自己的颜色；在房间里只列坐在那里的人，外加 @all。'],
    tryit: [`${K('↓')} 选一个，${K('tab')} 插入`, '接着打字缩小范围'] },
  { id: 'model', grp: '面板', name: '模型', t: 0,
    card: '模型一行一个，读不了图的写明白；思考强度在下面。',
    setup: sheetSetup('model'),
    what: `/model 打开模型面板。每行一个模型：名字、服务商、上下文长度、能不能读图；正在用的写 ${CD('in use')}。底部是思考强度，←→ 调，每档一句说明。`,
    changed: ['模型和思考强度在同一个面板。', `能力写成人话：${CD('reads images')} / ${CD('text only')}。`, 'alt+m 换模型，alt+t 换思考强度，页脚说一声。'],
    tryit: [`${K('↓')} 选一个，${K('⏎')} 换上`, `${K('←')}${K('→')} 调思考强度`, `关掉面板后试 ${K('alt+m')}`] },
  { id: 'status', grp: '面板', name: '状态', t: 0,
    card: '一屏看完这个会话：模型、模式、上下文、花费、插件。',
    setup: sheetSetup('status'),
    what: '/status 把会话状态列在一个面板里：模型、模式、上下文用量、花费、目录、插件、agents、规则。有问题的项是琥珀色，比如 github 插件的 token 过期了。',
    changed: ['页脚只显示偏离默认的东西，完整状态在这里。', '能处理的行，⏎ 直接去处理。', '上下文细条超过 80% 变琥珀色。'],
    tryit: [`${K('↓')} 移到 plugins，${K('⏎')} 打开`, `${K('esc')} 关掉`] },
  { id: 'plugins', grp: '面板', name: '插件', t: 0,
    card: '出错的排最前；space 开关，下一轮生效。',
    setup: sheetSetup('plugins'),
    what: '/plugins 列出 10 个插件，8 个开着。出错的排最前：github 的 token 过期了，右边用琥珀色写出来。开着的打 ✓，关着的是一个点。',
    changed: ['需要处理的在第一行。', '没有树形结构，也不需要图例。', `space 开关，提示 ${CD('from the next turn')}。`, '在 github 那行按 ⏎ 直接去登录。'],
    tryit: [`${K('space')} 开关一个插件`, `在 github 上按 ${K('⏎')}`] },
  { id: 'sessions', grp: '面板', name: '会话', t: 0,
    card: '等你的会话排最前，灯告诉你它在干什么。',
    setup: sheetSetup('sessions'),
    what: 'ctrl+r 打开会话列表。等你回答的排最前，琥珀色的灯；正在干活的其次，灯在呼吸；其余按时间。上面一行可以打字筛选。',
    changed: ['排序回答的是“哪个需要我”，不是“哪个最新”。', '列表里的灯只有三种：琥珀色是等你，呼吸是在干活，暗的是空闲。', '标题和目录都能筛。'],
    tryit: ['直接打字筛选', `${K('⏎')} 切过去`] },
  { id: 'rewind', grp: '面板', name: '回退', t: 0,
    card: '回到某句话之前：对话和文件一起回去。',
    setup: sheetSetup('rewind'),
    what: 'esc esc 打开回退面板，列出你说过的每一句话，最新的在上，每行右边写着回到那里要恢复几个文件。',
    changed: ['回退的单位是“你说的一句话”。', '对话和文件一起回去，面板顶上写明。', '回退后那句话回到输入框，改一改再发。'],
    tryit: [`${K('↓')} 选第一句，${K('⏎')} 回退`] },
  { id: 'keys', grp: '面板', name: '按键', t: 0,
    card: '? 打开按键表：四组，宽屏两列。',
    setup: sheetSetup('keys'),
    what: '在空输入行按 ?，按键表出现在输入框的位置，分四组：写、它在干活时、回看、切换。',
    changed: ['按用途分组，不按字母排。', '宽度够时两列，一屏放下。', '同一个键在哪都是同一个意思。'],
    tryit: [`${K('esc')} 关掉`] },
  { id: 'tasks', grp: '面板', name: '任务', t: 0,
    card: '进行中的在转圈，用负责人的颜色；做完的打勾。',
    setup: sheetSetup('tasks'),
    what: `ctrl+t 打开任务列表：进行中的排最前，转圈用负责人的颜色；待办是空心圈，做完的打勾；负责人在右边，右上写 ${CD('2 of 5 done')}。`,
    changed: ['同一个 agent 在页脚、对话和任务里是同一种颜色。', `写 ${CD('2 of 5 done')}，不画进度条。`],
    tryit: [`${K('esc')} 关掉，${K('ctrl+t')} 再打开`] },
  { id: 'schedule', grp: '面板', name: '定时', t: 0,
    card: '定时任务的灯慢慢呼吸；下一次什么时候跑写在右边。',
    setup: sheetSetup('schedule'),
    what: `/schedule 列出定时任务。开着的灯慢慢呼吸（2.4 s），右边写下一次什么时候跑；暂停的写 ${CD('paused')}；底部告诉你怎么加一个。`,
    changed: ['“在后台活着”的东西用呼吸的灯。', 'space 暂停或恢复，⏎ 立刻跑一次。'],
    tryit: [`${K('space')} 暂停一个`, `${K('⏎')} 立刻跑`] },
  { id: 'memory', grp: '面板', name: '记忆', t: 0,
    card: '指令和记忆分开：指令在文件里，记忆可以删。',
    setup: sheetSetup('memory'),
    what: `/memory 列出每一轮都会读的东西：上面两条是指令（这个项目的 ${CD('AGENTS.md')} 和所有项目共用的 ${CD('~/.bingo/AGENTS.md')}），下面三条是 bingo 记住的事。`,
    changed: ['指令去文件里改，面板只告诉你是哪个文件；记忆可以直接 ⌫ 忘掉。'],
    tryit: [`${K('↓')} 选一条记忆，${K('⌫')} 忘掉它`] },
  { id: 'find', grp: '面板', name: '查找', t: 0,
    card: '所有匹配都标出来，当前这个是实心的 ember。',
    setup() { past(); openFind(); S.dock.q = 'reset'; S.dock.want = true; },
    what: 'ctrl+f 打开查找，输入 reset：所有匹配染上浅橙色，当前这个是实心 ember，右边写第几个、一共几个。',
    changed: ['查找条在输入框的位置，不挡对话。', '↑↓ 跳，屏幕跟着滚；⏎ 停在这里，esc 回到原位。'],
    tryit: [`${K('↑')}${K('↓')} 在匹配之间跳`, '改一下查找的词', `${K('esc')} 回到原来的位置`] },

  { id: 'crew', grp: '协作', name: '一起干', t: 4000,
    card: '两个 agent 并行：做完的回来报告，要你点头的在页脚喊你。',
    setup() { S.mode = 1; past(); designRoom(); startTurn(CREW_P, crewScript()); },
    what: `bingo 派出两个 agent，各在自己的会话里干活。3 秒左右 scout 做完了，回到这里用自己的颜色报告；reviewer 想跑 ${CD('cargo test')}，这个询问留在 reviewer 自己的会话里，不抢你的输入框。页脚最前面写着 ${CD('◆ reviewer needs you')}，右边是 ${CD('ctrl+g answer')}。`,
    changed: ['队友的询问不抢你的输入框：它在队友自己的会话里等，页脚只说一句谁在等、按什么过去。', '彩色竖线加名字，颜色在页脚、任务和对话里一致；点一下名字就走进它的会话。', '派出去后这一轮就结束，不用等。'],
    tryit: [`${K('ctrl+g')} 去 reviewer 那里回答，${K('esc')} 回来`, `${K('/')} 输入 agents 打开团队面板，${K('⏎')} 走进一个队友的会话`] },
  { id: 'agent', grp: '协作', name: '队友', t: 800,
    card: '走进一个队友的会话：它在干什么、用什么模型，一眼看完。',
    setup() { S.mode = 1; past(); designRoom(); S.log.push(simulate(CREW_P, crewScript())); ago('scout', 600); ago('reviewer', 2600); enterView('scout'); },
    what: `这是 scout 自己的会话。标题栏写着 ${CD(`${TITLE} › scout`)}；带子上是 bingo 交给它的活，边光和彗星都换成了 scout 的颜色；页脚是 scout 和它用的 ${CD('haiku-5.5 low')}，最前面写着 reviewer 在等你。`,
    changed: ['队友的会话和你的会话长得一样，只换颜色。', 'esc 回到主会话，它接着干；ctrl+c 才会停下它。', 'ctrl+g 总是带你去等你的那一个；没人等你时回到主会话。'],
    tryit: [`${K('ctrl+g')} 去 reviewer 那里，${K('⏎')} 让它跑测试`, `打一句话按 ${K('⏎')}，直接跟 scout 说`, `${K('esc')} 回到主会话`] },
  { id: 'room', grp: '协作', name: '房间', t: 600,
    card: '房间里 @ 谁，谁就欠你一句话；它回了，债就清了。',
    setup() { past(); designRoom(); enterView('#design'); },
    what: `${CD('#design')} 房间：最上面一行是房间的目的，页脚是谁坐在这里。队友发言前面是它颜色的竖线，名字在第 3 列，话从第 14 列开始；同一个人连着说的话归成一组。你 @ 过的人都回答了，所以哪里都没有“在等”。`,
    changed: ['房间和队友一样是自己的会话：/agents 里 ⏎ 走进来，esc 回去。', `@ 了谁还没回，你那句话右边写着 ${CD('waiting on reviewer')}，左边一道它颜色的光在呼吸；回了就灭。`, '对方在写的时候，底下有一行转圈，上沿的彗星也换成它的颜色。', `${CD('/')} 在房间里多了 seat、unseat、close，技能不出现。`, `请走的人不再欠你回答；${CD('/close')} 写上为什么，房间就关了，谁也不再接话，你没发出去的草稿留着。`],
    tryit: [`输入 ${CD('@rev')}，${K('tab')} 补全，再写一句话按 ${K('⏎')}`, `${K('/')} 看房间里的命令`, `输入 ${CD('/close 结论定了')} 关掉房间`, `${K('esc')} 回到主会话`] },
  { id: 'background', grp: '协作', name: '后台', t: 3400,
    card: 'ctrl+b 把跑着的命令放到后台，这一轮接着走。',
    setup() {
      S.log.push(T1());
      startTurn('跑一遍全量测试', bgScript(), { t0: NOW() - 3000, title: TITLE });
      later(2500, () => { backgroundBash(); flashKey('ctrl+b'); });
    },
    what: `全量测试要跑很久。2.5 秒时自动按下 ctrl+b：这一步变成 ${CD('in background')}，bingo 回一句话，这一轮结束，页脚出现 ${CD('1 shell')}。9 秒后跑完，对话里出现一条通知。`,
    changed: ['长命令不该占住会话。', '后台的东西在页脚只占一个计数。', '跑完用一条安静的分隔线通知，页脚也说一声。'],
    tryit: ['等 9 秒看通知', '接着打字问点别的'] },
  { id: 'wake', grp: '协作', name: '定时醒来', t: 7400,
    card: 'bingo 给自己定的闹钟：时间到了，它自己开一轮。',
    setup() { S.log.push(T1(), simulate('盯一下 CI，过了告诉我', wakeScript())); S.title = TITLE; S.wake = { at: NOW() + 5000 }; },
    what: `上一轮 CI 还没跑完，bingo 定了个闹钟，页脚写着 ${CD('wakes in 5s')}。时间到了出现一轮 wake-up：带上是橙色的 wake-up 和它给自己的一句话（灰色）。它看完 CI，告诉你全绿了。`,
    changed: ['和你发起的一轮长得一样，只用 wake-up 代替你的话。', '等着的闹钟在页脚只占一个倒计时。'],
    tryit: ['看页脚倒计时走完', `${K('/')} 输入 schedule 看定时任务`] },

  { id: 'retry', grp: '异常', name: '重试', t: 500,
    card: '被限流了：上沿变琥珀色，倒计时写在那一行。',
    setup() { S.log.push(T1()); startTurn(T2P, retryScript(), { t0: NOW() - 500, title: TITLE }); },
    what: `服务商限流了。这一轮没停：上沿换成静止的琥珀色，那一行写着 ${CD('rate limited · retrying in 3s')}，右边是第几次尝试。重试成功后照常进行。`,
    changed: ['暂时的错误不是失败：不用红色，只说在等什么、还要多久。', '尝试次数在右边，暗色。'],
    tryit: ['等几秒看它重试成功', `${K('esc')} 不等了`] },
  { id: 'signedout', grp: '异常', name: '登录过期', t: 900,
    card: '出错时先说出了什么事，再说怎么办。',
    setup() { S.log.push(T1()); S.title = TITLE; S.signedOut = true; send(T2P); },
    what: `登录过期了。这一轮的边光是红色的，下面一行是 ${CD('signed out of anthropic · the token expired')}，再下面是 /login 重新登录，你的消息会保留。`,
    changed: ['错误两行：发生了什么、怎么办；没有错误码、堆栈、请求 ID。', '“你的消息会保留”最要紧：登录后自动重发。'],
    tryit: [`输入 ${CD('/login')} 再按 ${K('⏎')}，登录后看消息自动重发`] },
  { id: 'context', grp: '异常', name: '上下文快满了', t: 0,
    card: '页脚的上下文变成琥珀色；/compact 压缩。',
    setup() { past(); S.ctx = .86; },
    what: `上下文用到了 86%。页脚出现 ${CD('context 86%')}，琥珀色；状态面板里的用量条也是琥珀色。`,
    changed: ['低于 70% 不提；70% 以上显示，80% 以上琥珀色。', '不弹窗，不自动建议压缩。', `/compact 后出现分隔线 ${CD('compacted · context 86% → 18%')}，上沿闪一下绿光。`],
    tryit: [`输入 ${CD('/compact')} 再按 ${K('⏎')}`, `输入 ${CD('/status')} 看用量条`] },

  { id: 'exit', grp: '边界', name: '退出之后', t: 400,
    card: '回到 shell：一张小票，和回来的路。',
    setup() { past(); exitApp(); },
    what: `退出 bingo 回到 shell。bingo 留下一张小票：会话标题、几轮、多久、改了几个文件，下一行告诉你怎么回来；提示符后面已经像 fish 的自动建议一样，灰着写好了 ${CD('bingo --resume')} 和这个会话的 id。`,
    changed: ['不倒出一大段日志，一张小票足够。', '回来的路写在你下一步要打字的地方。', '带着 id，回来的一定是这一个会话，不是这个目录里最近的那个。'],
    tryit: [`${K('⏎')} 或 ${K('→')} 接受建议，回到会话`, '按别的键开始新会话'] },
  { id: 'narrow', grp: '边界', name: '80×24', t: 700,
    card: '最小的终端：同样的一屏，什么都不丢。',
    setup() { S.W = 80; S.H = 24; liveSetup(2600); },
    what: '同样是进行中的一轮，放进 80×24 的终端。',
    changed: ['布局只靠几条固定的列（0、3、5、13 和右边距 2），窄了只是右边更早截断。', '输入框最多占屏幕的 66%。', '路径放不下先丢前面的目录，文件名留着；结果始终在右边。'],
    tryit: ['用工具栏切回 100 × 32 对比'] },

  { id: 'icons', hidden: true, grp: '图标', name: '图标', t: 0, card: '', what: '', changed: [], tryit: [],
    setup() { S.log.push(simulate(ICONS_P, iconsScript())); S.title = '查清 drain 的坑'; } },
  { id: 'anatomy', hidden: true, grp: '解剖', name: '解剖', t: 500, card: '', what: '', changed: [], tryit: [],
    setup() { liveSetup(2700); S.queued.push({ text: QUEUED, at: NOW() }); } },
];
const VISIBLE = SCENES.filter(sc => !sc.hidden);
const sceneOf = id => SCENES.find(sc => sc.id === id);

/* ----- the clock: timers, scripts, and the things that expire ----- */
function tick(now) {
  if (S.timers.length) {
    const due = S.timers.filter(x => x.at <= now);
    if (due.length) { S.timers = S.timers.filter(x => x.at > now); due.forEach(runTimer); }
  }
  for (const id of sessionIds()) withSession(id, () => { for (const e of S.log) if (e.kind === 'turn') runScript(e, now); });
  if (S.toast && now - S.toast.at > (S.toast.ms || 2400) + 300) S.toast = null;
  if (S.welcome && S.welcome.out && now - S.welcome.out > 600) S.welcome = null;
  if (S.wake && now >= S.wake.at) withSession('main', wakeNow);
}
/* a timer runs in the session that set it; one whose session is gone is dropped */
function runTimer(x) { const id = x.sid || 'main'; if (id === 'main' || S.sessions[id]) withSession(id, x.f); }

/* ----- the window title carries the session's state, like a tab would ----- */
function emberState(now) {
  if (S.exited) return 'off';
  const d = S.dock;
  if (d && (d.kind === 'ask' || d.kind === 'question')) return d.danger ? 'danger' : 'asking';
  if (needers().length) return 'asking';
  const t = liveTurn();
  if (t) return t.retry ? 'asking' : 'live';
  const last = lastTurn();
  if (last && last.state === 'failed') return 'failed';
  if (last && last.state === 'done' && S.keyAt < last.endAt && now - last.endAt < 60000) return 'done';
  return 'idle';
}
function paintBar(now) {
  const v = viewOf(), st = emberState(now);
  const title = S.exited ? `${HERE.dir} — fish` : v ? `${sessionOf('main').title || 'bingo'} › ${v.name} — ${HERE.dir}` : `${S.title || 'bingo'} — ${HERE.dir}`;
  const el = $('#ttl'), key = st + '|' + title;
  if (el.dataset.k !== key) { el.dataset.k = key; el.innerHTML = `<i class="st-${st}"></i>${esc(title)}`; }
  const sz = `${S.W}×${S.H}`;
  if ($('#sz').textContent !== sz) $('#sz').textContent = sz;
}

/* ----- painting: rows are diffed by their HTML, so a still screen costs nothing ----- */
let lastFrame = [], CUR = null;
function paint() {
  if (!S) return;
  const now = NOW();
  tick(now);
  const F = frame(now), el = $('#term');
  S.clicks = F.lines.map(l => l.click);
  if (el.children.length !== F.rows.length || +el.dataset.w !== S.W) {
    el.innerHTML = F.rows.map(() => '<div class="r"></div>').join('');
    el.dataset.w = S.W; lastFrame = []; fitOne(el);
  }
  F.rows.forEach((R, i) => {
    const h = R.html(), r = el.children[i];
    if (lastFrame[i] !== h) { r.innerHTML = h; lastFrame[i] = h; }
    r.classList.toggle('ck', !!F.lines[i].click);
  });
  paintBar(now);
  if (miniVis) paintMinis(now);
}
const rowsHTML = F => F.rows.map(R => `<div class="r">${R.html()}</div>`).join('');

/* a still of any scene at any moment: a private session on a private clock */
function snapshot(id, simT = 0, size = '100x32') {
  const sc = sceneOf(id), saved = S, savedNow = NOWFN, T0 = 1e7;
  let clock = T0;
  NOWFN = () => clock; SNAP = true;
  try {
    const [w, h] = size.split('x').map(Number);
    S = fresh({ W: w, H: h, ascii: !!(saved && saved.ascii), reduced: !!(saved && saved.reduced) });
    sc.setup();
    for (let t = 50; t < simT; t += 50) { clock = T0 + t; tick(clock); }
    clock = T0 + simT; tick(clock);
    const F = frame(clock);
    return { html: rowsHTML(F), W: S.W, H: S.H, rows: F.rows, lines: F.lines };
  } finally { S = saved; NOWFN = savedNow; SNAP = false; }
}
/* the reference captures the plans test against: a scene, then waits (ms), keys and typed text
   (['text']); each 'shot' or 'shot:<name>' keeps a still as characters and ink tokens */
function capture(id, size = '100x32', seq = [], flags = {}) {
  const sc = sceneOf(id), saved = S, savedNow = NOWFN, T0 = 1e7;
  let clock = T0;
  NOWFN = () => clock; SNAP = true;
  const run = ms => { for (let left = ms; left > 0; left -= 50) { clock += Math.min(50, left); tick(clock); frame(clock); } };
  const still = name => { const F = frame(clock); return { name, t: clock - T0, W: S.W, H: S.H, rows: F.rows.map(rowGrid), ink: F.rows.map(rowInk) }; };
  try {
    const [w, h] = size.split('x').map(Number);
    S = fresh({ W: w, H: h, ascii: !!flags.ascii, reduced: !!flags.reduced });
    sc.setup();
    const out = [];
    for (const st of seq) {
      if (typeof st === 'number') run(st);
      else if (Array.isArray(st)) for (const ch of st[0]) { press(ch === ' ' ? 'space' : ch); run(60); }
      else if (st === 'shot' || st.startsWith('shot:')) out.push(still(st.slice(5)));
      else { press(st); run(60); }
    }
    return out;
  } finally { S = saved; NOWFN = savedNow; SNAP = false; }
}
const rowGrid = R => R.c.map((c, k) => c.ch !== '' ? c.ch : k && R.c[k - 1].ch && wcw(R.c[k - 1].ch.codePointAt(0)) === 2 ? '' : ' ').join('');
/* a colour is named by its token when it is one; a blend at this moment stays a hex */
function inkName(c) {
  if (!c) return 'fg';
  if (c[0] !== '#') return c;
  for (const [k, v] of Object.entries(PAL)) if (v === c && k !== 'crew') return k;
  const i = PAL.crew.indexOf(c);
  return i >= 0 ? 'crew' + i : c;
}
function rowInk(R) {
  const bg = [], fg = [];
  R.c.forEach((c, x) => {
    const b = c.bg ? inkName(c.bg) : null, last = bg[bg.length - 1];
    if (b && last && last[1] === x - 1 && last[2] === b) last[1] = x; else if (b) bg.push([x, x, b]);
    if (c.ch === ' ') return;
    const f = c.ch === '' ? null : inkName(c.fg) + (c.b ? '+b' : '') + (c.i ? '+i' : '') + (c.u ? '+u' : ''), cur = fg[fg.length - 1];
    if (f === null) { if (cur && cur[1] === x - 1) cur[1] = x; return; }
    if (cur && cur[2] === f) cur[1] = x; else fg.push([x, x, f]);
  });
  return { bg, fg };
}
function fillShot(el, id, t, size) {
  const snap = snapshot(id, t, size);
  el.dataset.w = snap.W; el.innerHTML = snap.html; fitOne(el);
  return snap;
}

/* ----- anatomy: the live turn, with numbered pins found by what the rows say ----- */
const ANCHORS = [
  { find: '看一下 session', b: '你的话', t: '你说的话在一条浅色带上，右边是这一轮用了多久；轮与轮之间空一行。更早做完的轮次，带子左边不亮。' },
  { find: 'drain|reset', b: '一步一行', t: '图标在第 3 列，动词在第 5 列，参数对齐第 13 列，结果在右边。查东西的步骤暗一档，动手的步骤亮一档。' },
  { find: '问题不在', b: '回答', t: '正文最宽 100 列，行内代码用代码色，代码块有高亮。' },
  { find: '修好它，加', b: '边光', t: '带子左边缘的一道光就是这一轮的状态：橙色呼吸是在干活，琥珀色是等你，绿色是刚做完，红色是失败。右边是这一轮用了多久。' },
  { find: '先改 reset()', b: '旁白', t: 'bingo 边干边说的话，斜体灰色，留在它说话的位置，夹在步骤之间。' },
  { find: 'cargo test -p', b: '正在跑', t: '转圈在图标的位置，跑完落成这一步的图标；最后一行输出贴在参数下面，用时在右边。' },
  { edge: true, b: '上沿', t: '输入框的上沿就是 bingo 的状态：彗星在跑是在干活，琥珀色呼吸是等你，红色是危险，闪一下是做完或换了模式。' },
  { find: 'CHANGELOG', b: '排队', t: '转向或排队的话列在输入框上方，右边写着什么时候送到。' },
  { find: 'Steer, or queue', b: '输入行', t: '位置永远不变，提示文字随状态变。' },
  { last: true, b: '页脚', t: '左边是要知道的环境：谁在等你、模型、偏离默认的模式、快满的上下文、在干活的 agent；右边是此刻能按的键。' },
];
const rowPlain = R => R.c.map(c => c.ch).join('');
function anchorRows(snap) {
  const text = snap.rows.map(rowPlain), E = g('edge');
  return ANCHORS.map(a => a.last ? text.length - 1 : a.edge ? snap.rows.findIndex(R => R.c.every(c => c.ch === E)) : text.findIndex(x => x.includes(a.find)));
}
/* neighbouring pins step sideways so two rows in a row never overlap */
function pinsHTML(order) {
  let prev = null;
  return order.map((o, n) => {
    const x = prev && o.y - prev.y <= 1 && prev.x === -22 ? -6 : -22;
    prev = { y: o.y, x };
    return `<i class="anc" style="left:${x}px;top:${(o.y + .5) * 1.45}em"><b class="pin">${n + 1}</b></i>`;
  }).join('');
}
function renderAnatomy() {
  let snap, ys;
  for (const h of [32, 40, 48]) { snap = snapshot('anatomy', 500, `100x${h}`); ys = anchorRows(snap); if (ys.every(y => y >= 0)) break; }
  const order = ANCHORS.map((a, i) => ({ a, y: ys[i] })).filter(o => o.y >= 0).sort((p, q) => p.y - q.y);
  const el = $('#anat');
  el.dataset.w = snap.W; el.innerHTML = rowsHTML(snap) + pinsHTML(order); fitOne(el);
  $('#anatSz').textContent = `${snap.W}×${snap.H}`;
  $('#callouts').innerHTML = order.map(o => `<li><div><b>${o.a.b}</b><span>${o.a.t}</span></div></li>`).join('');
}

/* ----- the grid ruler: a still with the columns drawn over it ----- */
const GUIDES = [[0, '0'], [TX, '3'], [WX, '5'], [AX, '13'], [-RM, 'W−2']];
function renderRuler(el) {
  const snap = fillShot(el, el.dataset.scene, 0, el.dataset.size || '100x32');
  if (!el.dataset.guides) return;
  el.insertAdjacentHTML('beforeend', GUIDES.map(([x, label]) => `<div class="guide" style="left:${x < 0 ? snap.W + x : x}ch"><b>${esc(label)}</b></div>`).join(''));
}

/* ----- marks and inks: read straight from the tables the renderer uses ----- */
const MARKS = [
  ['side', ['glow', 'good', 'amberGlow', 'bad', 'dim'], '一轮的边光（第 0 列）：在干活、做完、等你、失败、停下'],
  ['lamp', ['glow', 'amberGlow', 'faint'], '列表和页脚的状态灯：在干活、等你、空闲'],
  ['spin', ['glow'], '正在跑的一步（80 ms 一帧）；跑完落成这一步的图标'],
  ['pin', ['amberGlow'], '这一步在等你'],
  ['think', ['dim'], '想过的一段（只在 ctrl+o 细节里）'],
  ['edge', ['glow'], '输入框上沿', 3],
  ['caret', ['glow'], '光标'],
  ['quote', ['crew1', 'glow'], 'agent 发言的竖线 / 列表里选中的那一行'],
  ['ok', ['good'], '做成了 / 插件开着'],
  ['still', ['dim'], '待办'],
  ['dot', ['dim'], '分隔 / 插件关着'],
  ['minus', ['bad'], '删掉的行数'],
  ['ell', ['dim'], '放不下，截断'],
];
function renderMarks() {
  const cell = (t, ink) => `<span style="color:${col(ink)}">${esc(t)}</span>`;
  const rows = MARKS.map(([k, inks, what, n = 1]) =>
    `<tr><td class="g">${inks.map(ink => cell(GU[k].repeat(n), ink)).join('')}</td><td class="g">${cell(GA[k].repeat(n), 'muted')}</td><td>${what}</td></tr>`);
  $('#marks').innerHTML = `<thead><tr><th>记号</th><th>ASCII</th><th>意思</th></tr></thead><tbody>${rows.join('')}</tbody>`;
}
/* the tool marks, read from the same declarations the rows use; which tools each one stands for, in bingo's own names */
const TOOL_ROWS = [
  ['read', 'Read', 'fs', '读文件'], ['glob', 'Glob', 'fs', '按名字找文件'], ['grep', 'Grep', 'fs', '在文件里搜'],
  ['edit', 'Edit', 'fs', '改文件，结果写 +/− 行数'], ['write', 'Write', 'fs', '写新文件'],
  ['bash', 'Bash · BashOutput', 'bash', '跑命令，参数用代码色'], ['kill', 'KillShell', 'bash', '停掉后台命令'],
  ['fetch', 'WebFetch · ShowPage', 'web', '取一个网页'], ['search', 'WebSearch', 'web', '上网搜'], ['look', 'View · Picture', 'pictures', '看图'],
  ['agent', 'SpawnAgent · SendMessage', 'agents', '派活给队友：图标和名字用队友的颜色'],
  ['room', 'Seat · Unseat · CloseRoom', 'rooms', '房间：和队友同一个图标，动词跟着工具走'],
  ['todo', 'TaskCreate · TaskUpdate', 'tasks', '记下要做的事'], ['ask', 'AskUserQuestion', 'fs', '问你：等你时换成琥珀色的 ◆'],
  ['skill', 'Skill', 'skills', '载入技能'], ['wake', 'Wake · ScheduleCreate', 'schedule', '给自己定个闹钟'],
  ['recall', 'ExperienceQuery', 'experience', '翻经验'], ['mcp', 'mcp__服务器__工具', 'mcp', '动词是服务器的名字'],
  ['query', 'query', 'postgres（你装的）', '外面的 mod 用同一份声明'],
];
function renderTools() {
  const cell = (t, ink) => `<span style="color:${col(ink)}">${esc(t)}</span>`;
  const verb = (k, d) => d.verb ? CD(d.verb) : `<i>${k === 'agent' ? '队友的名字' : '服务器的名字'}</i>`;
  const row = ([k, names, mod, what]) => {
    const d = TOOLS[k], ink = k === 'agent' ? 'crew1' : d.ro ? 'muted' : 'fg';
    return `<tr><td class="g">${cell(d.mark, ink)}</td><td class="g">${cell(d.ascii || d.mark, 'muted')}</td><td>${verb(k, d)}</td><td>${esc(names)}</td><td>${esc(mod)}</td><td>${what}</td></tr>`;
  };
  const tail = `<tr><td class="g">${cell(GU.bullet, 'fg')}</td><td class="g">${cell(GA.bullet, 'muted')}</td><td>${CD('deploy')}</td><td>没声明的工具</td><td>—</td><td>圆点加工具名，小写</td></tr>`;
  $('#tools').innerHTML = `<thead><tr><th>图标</th><th>ASCII</th><th>动词</th><th>工具</th><th>mod</th><th>说明</th></tr></thead><tbody>${TOOL_ROWS.map(row).join('')}${tail}</tbody>`;
}
const INKS = [
  ['fg', '正文'], ['muted', '次要'], ['dim', '暗'], ['faint', '最暗：分隔'], ['glow', 'ember：在干活、匹配、命令符'], ['coal', '边光呼吸的暗端'], ['amberGlow', '等你'],
  ['good', '做成了'], ['bad', '失败、危险、bypass'], ['plan', '蓝：plan 模式、标签'], ['auto', '紫：accept edits'], ['code', '代码和命令'], ['crew', '六种 agent 颜色'],
  ['you', '底色：你的话和输入框', true], ['well', '底色：井', true], ['pick', '底色：列表选中', true], ['addTint', '底色：加的行', true], ['delTint', '底色：删的行', true],
];
function inkHalf(P, k, ground) {
  if (k === 'crew') return `<span style="background:${P.bg};color:${P.fg}"><b>${P.crew.map(c => `<i style="color:${c}">●</i>`).join('')}</b><i>crew0–5</i></span>`;
  return ground ? `<span style="background:${P[k]};color:${P.fg}"><b>Aa</b><i>${P[k]}</i></span>`
    : `<span style="background:${P.bg};color:${P[k]}"><b>Aa</b><i>${P[k]}</i></span>`;
}
function renderInks() {
  $('#inks').innerHTML = INKS.map(([k, what, ground]) =>
    `<div class="ink"><div class="pair">${inkHalf(PALS.light, k, ground)}${inkHalf(PALS.dark, k, ground)}</div><div class="meta"><b>${k}</b><span>${what}</span></div></div>`).join('');
}

/* ----- motion: eight tiny terminals, each one effect on the live clock ----- */
const MW = 22;
let miniVis = false;
function lampMini(now) {
  const k = now % 3600, lit = k >= 2200, p = S.reduced ? 1 : clamp((k - 2200) / 320, 0, 1);
  const ink = lit ? mix('glow', 'good', easeOut(p)) : breatheInk('glow', now);
  return [BLANK(), BLANK(), ln({ p: [[0, [s(g('side'), ink)]], [TX, [s('修好它', 'fg')]]], fill: 'you', fx: lit ? { sparks: now - (k - 2200), x: 0 } : null })];
}
function dryMini(now) {
  const ch = [...'原因在 reset()。'], k = now % 3200, n = Math.min(ch.length, Math.floor(k / 1000 * 12));
  const segs = [s(ch.slice(0, n).join(''), 'fg')];
  if (S.reduced) return [L(TX, [s(ch.join(''), 'fg')])];
  if (n < ch.length) return [L(TX, [...recolorTail(segs, 10, now), s(g('caret'), 'glow')])];
  const tail = Math.round(10 * (1 - clamp((k - ch.length / 12 * 1000) / 500, 0, 1)));
  return [L(TX, tail ? recolorTail(segs, tail, now) : segs)];
}
function toastMini(now) {
  const k = now % 4200;
  if (k >= 2700) return [L(TX, [s(S.model.id, 'muted'), s(' ' + S.model.eff, 'dim')])];
  const a = S.reduced ? 1 : Math.min(clamp(k / 160, 0, 1), 1 - clamp((k - 2400) / 300, 0, 1));
  return [L(TX, [s(g('ok') + ' ', mix('bg', 'good', a)), s('wakes in 5m', mix('bg', 'fg', a))])];
}
/* the spinner runs, then lands on the step's own mark and cools from ember to its ink */
function spinMini(now) {
  const k = now % 3200, landed = k >= 1800 && !S.reduced;
  const ink = landed ? mix('glow', 'fg', easeOut(clamp((k - 1800) / 420, 0, 1))) : 'glow';
  return [L(TX, [s(landed ? '$' : spinG(now, 0), ink), s(' run', 'muted'), s('  cargo test', 'code')])];
}
const MINIS = {
  comet: now => [ln({ p: [[0, cometSegs(now, MW, 0)]], fill: 'you' })],
  lamp: lampMini,
  ask: now => [ln({ p: [[0, [s(g('edge').repeat(MW), mix('amberGlow', 'you', .35 * breathe(now, 1400)))]]], fill: 'you' }), L(TX, [s('Run this command?', 'fg', 'b')], { fill: 'you' })],
  spin: spinMini,
  shimmer: now => [L(TX, [s(spinG(now, 0), 'glow'), s(' '), ...shimmer('thinking', now, 'muted')])],
  caret: now => [L(TX, [s(g('caret'), caretInk(now)), s('Ask anything', 'dim')], { fill: 'you' })],
  dry: dryMini,
  toast: toastMini,
};
function paintMinis(now) {
  for (const el of $$('#motion .scr[data-m]')) {
    const ls = MINIS[el.dataset.m](now), rows = ls.map(l => rowOf(l, MW));
    ls.forEach((l, y) => { if (l.fx) effects(rows, l.fx, y, now); });
    const h = rows.map(R => `<div class="r">${R.html()}</div>`).join('');
    if (el._h !== h) { el._h = h; el.innerHTML = h; }
  }
}

/* ----- before and after: the old screenshot beside the same moment in Ember ----- */
const PAIRS = [
  { img: '01_welcome', id: 'welcome', t: 2600, h: '开场', why: '开场不放机器的细节，放你接下来可能要做的事。', diffs: [
    `<b>不给你看机器的细节。</b>旧版顶行是临时目录 ${CD('/var/…/T/tmp.J1tHI0tmGG/cwd')} 和 ${CD('session 01M5')}，一个是机器路径，一个是内部 ID；新版写 ${CD('~/bingo-improve on dev · 3 changed')}。`,
    `<b>默认值不用说。</b>底部常驻的 ${CD('⊠ default ◆ solo ▱▱▱ ? / ?')} 全是默认值；新版只在偏离默认时才说。`,
    '<b>空白有了用处。</b>旧版开场是一整屏空白，新版用来写字标和最近的三个会话。',
    `<b>提示少一点。</b>五个提示压成三个：${CD('↑ recent')}、${CD('/ commands')}、${CD('? keys')}。`] },
  { img: '02_ask_edit', id: 'ask-edit', t: 700, h: '要改文件', why: '问题出现在你要打字的地方，只说一次在等你。', diffs: [
    `<b>问题说清楚是哪个文件。</b>${CD('Do you want to Edit src/main.rs?')} 加编号 1–4 和 ${CD('❯')}，改成 ${CD('Edit session/queue.rs?')}，选项不编号，ember 竖线标出选中的那项。`,
    `<b>diff 只留改动。</b>去掉 ${CD('--- /var/folders/…')}、${CD('+++')}、${CD('@@')}，换成行号和字级标记。`,
    `<b>“在等你”只说一次。</b>旧版说了三处：${CD('needs you')}、${CD('waiting for you')}、${CD('answer above · typing here leaves it for later')}；新版只用琥珀色呼吸的上沿。`,
    `<b>范围写明白。</b>${CD('Yes for this session')} 改为 ${CD('Yes, allow edits in src/session/ for this session')}。`] },
  { img: '04_running', id: 'live', t: 700, h: '在干活', why: '在干活只用三个信号；每一步落成一行，带着自己的图标。', diffs: [
    `<b>图标说是什么。</b>旧版每步前面都是同一个 ${CD('✓')}；新版每种工具有自己的图标（${CD('¶')} 读、${CD('±')} 改、${CD('$')} 跑），正在跑的那步转圈，跑完落成它的图标。`,
    `<b>去掉每行的 ${CD('you: yes')}。</b>放行是你的设置，不是每一步的新闻。`,
    `<b>后台键只在用得上时出现。</b>${CD('[ b background ]')} 改为只在 bash 在跑时出现在页脚右边的 ${CD('ctrl+b background')}。`,
    `<b>用动来表示在忙。</b>${CD('▍ stoking · 3s')} 换成边光的呼吸和上沿的彗星。`] },
  { img: '05_finished', id: 'done', t: 0, h: '做完了', why: '一轮三层：你的话、每步一行、回答。', diffs: [
    `<b>不折叠，排整齐。</b>${CD('▸ 6 steps · read · grep · glob · edit +1 −1 · bash ×2 exit 101')} 这一行去掉了：每步一行，参数对齐第 13 列，结果在右边。`,
    `<b>用时挪到你的话右边。</b>去掉 ${CD('Worked for 6s · done 21:58')}，用时写在这一轮你的话右边。`,
    '<b>代码有颜色。</b>反引号不再原样显示，行内代码用代码色。',
    `<b>轮与轮分开。</b>${CD('›')} 开头、轮与轮紧挨，改成浅色带加边光，轮与轮之间空一行。`] },
  { img: '06_expanded', id: 'ledger', t: 0, h: '细节', why: 'ctrl+o 看细节：想了什么、你什么时候插的话；输出放进更深的井里。', diffs: [
    '<b>输出放进井里。</b>旧版输出和 diff 与正文同底色、同缩进；新版放进更深一档的井。',
    `<b>diff 头换成行号。</b>去掉 ${CD('---/+++/@@')}，换成行号。`,
    `<b>失败只说一次。</b>${CD('exit 101')} 写了两遍，改成行尾一次 ${CD('exit 101')}，图标和动词变红。`,
    `<b>图标代替 ${CD('✓')}。</b>去掉 ${CD('you: yes')}、${CD('▸')} 和千篇一律的 ${CD('✓')}；每步用自己工具的图标，失败的那步图标和动词是红色。`] },
  { img: '08_question', id: 'question', t: 800, h: '提问', why: '问题在输入框的位置，说明和选项同一行。', diffs: [
    `<b>问题在输入框的位置。</b>旧版问题长在对话里，选项编号 1–4、选中用 ${CD('❯')}，每个选项的说明另起一行，四个选项占八行；新版说明和选项在同一行，选中那一项的预览放在右边。`,
    `<b>“在等你”只说一次。</b>旧版说了三遍：${CD('◆ asks you · needs you')}、${CD('form is waiting for you')}、输入框里的 ${CD('your message waits until the ask is answered')}；新版只用琥珀色呼吸的上沿和这一轮的边光。`,
    '<b>相关的问题一起问。</b>旧版一次只问一个；新版把两个问题一起问，上面一行是标签，←→ 切换，答过的打勾。',
    `<b>总能说点别的。</b>${CD('something else…')} 改成 ${CD('Something else')}，选中后可以自己写。`] },
  { img: '10_error', id: 'signedout', t: 900, h: '出错', why: '先说发生了什么，再说怎么办。旧截图里是另一种错误（模型读不了图），这里比较的是错误的写法。', diffs: [
    `<b>旧版只有服务商的原话。</b>一行红字 ${CD('bad request: model `fake-1` does not support images')}，反引号原样显示，没说接下来怎么办。`,
    `<b>新版两行：发生了什么、怎么办。</b>${CD('signed out of anthropic · the token expired')}，下一行 ${CD('/login signs you in again · your message is kept')}。`,
    `<b>能提前拦下的就提前拦下。</b>截图里的这个错误，新版在贴图那一刻就提示：${CD("opus-5.5 can't read images · alt+m switches model")}。`,
    '<b>同一道边光。</b>旧版行首的红色竖线留下了，但它不再只属于错误：它就是这一轮的边光，在干活时是橙色，等你时是琥珀色，出错时才是红色。'] },
  { img: '11_palette', id: 'palette', t: 0, h: '命令', why: '列表短一点，右边放快捷键。', diffs: [
    `<b>列表短一点。</b>旧版一次铺开 18 条命令，不说一共多少；新版最多 8 行，下面写 ${CD('8 of 29')}，接着打字就缩小。`,
    `<b>右边只放快捷键。</b>旧版右边是各自的当前值（${CD('fake/fake-1')}、${CD('off')}、${CD('default')}、${CD('14396 used')}、${CD('3 signed out')}，还有一整句会话标题），有的有、有的没有；新版右边只放快捷键。`,
    `<b>只列你会说出口的词。</b>旧版混着 ${CD('/!')}、${CD('/bash.promote')} 这种内部名字；新版跑命令用 ${CD('!')}，放到后台按 ${CD('ctrl+b')}。`,
    `<b>技能也在这里。</b>旧版底部用一整句解释能搜什么；新版技能和命令在同一个列表，技能右边标 ${CD('skill')} 和来源。`] },
  { img: '16_status', id: 'status', t: 0, h: '状态', why: '只写人看得懂的值，没有的就说没有。', diffs: [
    `<b>不露内部 ID。</b>旧版第一行是完整的会话 ID ${CD('ses_01M552FGHGP48EAVFM3T7GEFHF')} 和截断的机器路径 ${CD('/var/folders/5d…')}；新版标题右边写会话名，目录写成 ${CD('~/bingo-improve on dev · 3 changed')}。`,
    `<b>没有就说没有。</b>旧版把零也列出来：${CD('0 rules this session')}、${CD('0 connected · 0 tools')}、${CD('cache read 0% · price unknown')}；新版只写一个词：${CD('none yet')}、${CD('none working')}。`,
    `<b>上下文一根条。</b>旧版写成两行：${CD('14k of 191k · 7% · compacts at 172k')} 和 ${CD('about system 2k · tools 11k · chat 491')}；新版一根细条加百分比，80% 以上变琥珀色。`,
    `<b>花费写钱。</b>旧版是 token 数 ${CD('113k in · 174 out')}；新版直接写花了多少钱。`] },
  { img: '17_plugins', id: 'plugins', t: 0, h: '插件', why: '直接列插件，要处理的排第一。', diffs: [
    `<b>一行一个插件。</b>旧版是一棵树：${CD('▸ 38 built in · all on · 2 drawn over')} 折叠着，具体的插件藏在 ${CD('show all 38')} 后面；新版一行一个，名字后面一句话说它做什么。`,
    `<b>不需要图例。</b>旧版右上角一行图例 ${CD('● on ○ off × failed ◌ waiting')}；新版开着打 ${CD('✓')}，关着是一个点，出错的直接把原因写在右边（${CD('token expired')}）。`,
    '<b>不讲机制。</b>旧版右半屏是一段说明（drawn over、plain drawing、5 are needed and stay on）；新版只说 space 开关、下一轮生效。',
    `<b>要处理的排第一。</b>旧版标题 ${CD('38 plugins · 0 MCP servers · one list')}；新版右上写 ${CD('8 of 10 on')}，github 排在第一行。`] },
];
function buildPairs() {
  $('#pairs').innerHTML = PAIRS.map((p, i) => `<div class="pairbox"><h3>${p.h}</h3><p class="why">${p.why}</p><div class="duo">`
    + `<figure class="before"><figcaption><i></i>旧版</figcaption><img alt="旧版截图：${p.h}" width="1000" height="882" decoding="async" data-img="${p.img}"></figure>`
    + `<figure><figcaption class="new"><i></i>Ember</figcaption><div class="win"><div class="term static"><div class="scr" data-pair="${i}" data-w="100"></div></div></div></figure>`
    + `</div><ul class="diffs">${p.diffs.map(d => `<li>${d}</li>`).join('')}</ul></div>`).join('');
}
function syncImages() {
  const th = effTheme();
  for (const im of $$('#pairs img[data-img]')) { const src = `before/${th}-${im.dataset.img}.jpg`; if (im.getAttribute('src') !== src) im.setAttribute('src', src); }
}

/* ----- the gallery: every scene as a still; a click opens it live above ----- */
function buildGallery() {
  const el = $('#gallery');
  el.innerHTML = VISIBLE.map(sc => `<a class="card" href="#s-${sc.id}" data-id="${sc.id}"><div class="shot"><div class="scr" data-card="${sc.id}" data-w="100"></div></div>`
    + `<div class="meta"><b>${sc.name}<em>${sc.grp}</em></b><span>${sc.card}</span></div></a>`).join('');
  el.addEventListener('click', e => { const a = e.target.closest('a.card'); if (!a) return; e.preventDefault(); pick(a.dataset.id); });
}

/* ----- statics: rebuilt in small slices so a theme switch never stalls the page ----- */
let staticsGen = 0;
function staticJobs() {
  const jobs = [renderMarks, renderTools, renderInks];
  for (const el of $$('#gallery .scr[data-card]')) { const sc = sceneOf(el.dataset.card); jobs.push(() => fillShot(el, sc.id, sc.t, sc.id === 'narrow' ? '80x24' : '100x32')); }
  jobs.push(renderAnatomy);
  for (const el of $$('.scr[data-scene]')) jobs.push(() => renderRuler(el));
  for (const el of $$('#pairs .scr[data-pair]')) { const p = PAIRS[+el.dataset.pair]; jobs.push(() => fillShot(el, p.id, p.t, '100x36')); }
  return jobs;
}
function renderStatics() {
  const gen = ++staticsGen, jobs = staticJobs();
  syncImages();
  const step = () => {
    if (gen !== staticsGen) return;
    const t0 = performance.now();
    while (jobs.length && performance.now() - t0 < 12) jobs.shift()();
    if (jobs.length) setTimeout(step, 0); else { paintMinis(NOW()); fitAll(); }
  };
  step();
}

/* ----- fitting: every still is sized to its box by the width of one cell ----- */
let CHR = .6;
function measure() {
  const m = document.createElement('span');
  m.style.cssText = 'position:absolute;visibility:hidden;white-space:pre;font-size:100px;font-family:var(--tmono)';
  m.textContent = '0'.repeat(50);
  document.body.appendChild(m);
  CHR = m.getBoundingClientRect().width / 50 / 100 || .6;
  m.remove();
}
function fitScr(el, W, lo, hi) {
  const box = el.parentElement, cs = getComputedStyle(box);
  const avail = box.clientWidth - parseFloat(cs.paddingLeft) - parseFloat(cs.paddingRight);
  if (avail <= 0) return;
  const fs = clamp(Math.floor(avail / (W * CHR) * 10) / 10, lo, hi) + 'px';
  if (el.style.fontSize !== fs) el.style.fontSize = fs;
}
function fitOne(el) {
  if (el.id === 'term') return fitScr(el, S.W, 5, 15);
  const w = +el.dataset.w || 100;
  if (el.closest('.card')) return fitScr(el, w, 2.5, 7);
  if (el.closest('.term.static')) return fitScr(el, w, 4, 12);
}
function fitAll() { $$('.scr').forEach(fitOne); }
function updateCap() { $('#cap').textContent = `${S.W} × ${S.H} · Maple Mono NF（没装就用 JetBrains Mono） · 行高 1.45 · 颜色取自你的终端`; }

/* ----- theme: the page and every terminal on it follow one switch ----- */
const TKEY = 'bingo-tui3-theme';
function effTheme() {
  const a = document.documentElement.getAttribute('data-theme');
  if (a === 'light' || a === 'dark') return a;
  return matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}
function themeLabel() { $('#themeBtn span').textContent = effTheme() === 'dark' ? '浅色' : '深色'; }
function applyTheme() { PAL = PALS[effTheme()]; themeLabel(); renderStatics(); lastFrame = []; paint(); }
function flipTheme() {
  const n = effTheme() === 'dark' ? 'light' : 'dark';
  document.documentElement.setAttribute('data-theme', n);
  try { localStorage.setItem(TKEY, n); } catch (e) { /* storage is a convenience */ }
}
function wireTheme() {
  try { const t = localStorage.getItem(TKEY); if (t === 'light' || t === 'dark') document.documentElement.setAttribute('data-theme', t); } catch (e) { /* none stored */ }
  PAL = PALS[effTheme()]; themeLabel();
  $('#themeBtn').addEventListener('click', flipTheme);
  new MutationObserver(applyTheme).observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });
  const mq = matchMedia('(prefers-color-scheme: dark)');
  if (mq.addEventListener) mq.addEventListener('change', applyTheme);
}

/* ----- scenes in the prototype: chips, notes, play ----- */
function buildChips() {
  const groups = [];
  for (const sc of VISIBLE) {
    let grp = groups.find(x => x.name === sc.grp);
    if (!grp) groups.push(grp = { name: sc.grp, list: [] });
    grp.list.push(sc);
  }
  const el = $('#scenes');
  el.innerHTML = groups.map(gp => `<div class="grp"><span>${gp.name}</span>${gp.list.map(sc => `<button type="button" class="chip" data-id="${sc.id}" aria-pressed="false">${sc.name}</button>`).join('')}</div>`).join('');
  el.addEventListener('click', e => { const b = e.target.closest('.chip'); if (!b) return; play(b.dataset.id, true); $('#termwrap').focus({ preventScroll: true }); });
}
function renderNotes(sc) {
  const li = xs => xs.map(x => `<li>${x}</li>`).join('');
  $('#notes').innerHTML = `<p class="nk"><span>${sc.grp}</span><span>${VISIBLE.indexOf(sc) + 1} / ${VISIBLE.length}</span></p><h3>${sc.name}</h3>`
    + `<h4>这一屏</h4><p class="what">${sc.what}</p><h4>为什么这样</h4><ul>${li(sc.changed)}</ul><h4>可以试试</h4><ul>${li(sc.tryit)}</ul>`;
}
function syncSize() { const k = `${S.W}x${S.H}`; for (const b of $$('.tb[data-size]')) b.setAttribute('aria-pressed', String(b.dataset.size === k)); }
function play(id, picked) {
  const sc = sceneOf(id) || sceneOf('live');
  CUR = sc.id;
  S = fresh(Object.assign(base(), { W: SIZE[0], H: SIZE[1] }));
  sc.setup();
  for (const b of $$('#scenes .chip')) b.setAttribute('aria-pressed', String(b.dataset.id === CUR));
  renderNotes(sc);
  if (picked) { try { history.replaceState(null, '', '#s-' + CUR); } catch (e) { /* a sandboxed frame may refuse */ } }
  syncSize(); updateCap(); lastFrame = []; paint();
}
function pick(id) {
  play(id, true);
  $('#proto').scrollIntoView({ behavior: S.reduced ? 'auto' : 'smooth', block: 'start' });
  $('#termwrap').focus({ preventScroll: true });
}
function nextScene(dir) {
  const i = VISIBLE.findIndex(sc => sc.id === CUR);
  play(VISIBLE[(i + dir + VISIBLE.length) % VISIBLE.length].id, true);
}
function wireToolbar() {
  for (const b of $$('.tb[data-size]')) b.addEventListener('click', () => {
    SIZE = b.dataset.size.split('x').map(Number);
    S.W = SIZE[0]; S.H = SIZE[1]; S.grow = null;
    syncSize(); updateCap(); lastFrame = []; paint();
  });
  const flip = (btn, key) => btn.addEventListener('click', () => {
    S[key] = !S[key]; btn.setAttribute('aria-pressed', String(S[key]));
    lastFrame = []; paint(); renderStatics();
  });
  flip($('#asciiBtn'), 'ascii');
  flip($('#motionBtn'), 'reduced');
  $('#replayBtn').addEventListener('click', () => { play(CUR); $('#termwrap').focus({ preventScroll: true }); });
  $('#prevScene').addEventListener('click', () => nextScene(-1));
  $('#nextScene').addEventListener('click', () => nextScene(1));
}

/* ----- the loop: 25 frames a second while anything moving is on screen ----- */
let visible = true, lastPaint = 0;
function loop(t) {
  requestAnimationFrame(loop);
  if (document.hidden || (!visible && !miniVis) || t - lastPaint < 40) return;
  lastPaint = t;
  if (visible) paint(); else paintMinis(NOW());
}
function observe() {
  if ('IntersectionObserver' in window) {
    new IntersectionObserver(es => { for (const e of es) visible = e.isIntersecting; }).observe($('#termwrap'));
    new IntersectionObserver(es => { for (const e of es) miniVis = e.isIntersecting; }).observe($('#motion'));
  } else miniVis = true;
  if ('ResizeObserver' in window) {
    let raf = 0;
    const ro = new ResizeObserver(() => { cancelAnimationFrame(raf); raf = requestAnimationFrame(fitAll); });
    for (const el of [$('#termwrap'), $('#gallery'), $('.anat'), $('.term.ruler'), $('#pairs')]) if (el) ro.observe(el);
  } else window.addEventListener('resize', fitAll);
  if (document.fonts && document.fonts.ready) document.fonts.ready.then(() => { measure(); fitAll(); });
}
function init() {
  wireTheme();
  buildChips(); buildGallery(); buildPairs();
  wireInput(); wireToolbar();
  S = fresh({ reduced: matchMedia('(prefers-reduced-motion: reduce)').matches });
  $('#motionBtn').setAttribute('aria-pressed', String(S.reduced));
  $('#nScenes').textContent = String(VISIBLE.length);
  measure();
  const m = location.hash.match(/^#s-([\w-]+)$/);
  play(m && VISIBLE.some(sc => sc.id === m[1]) ? m[1] : 'live');
  renderStatics();
  observe();
  $('#hint').classList.add('on');
  requestAnimationFrame(loop);
  window.__bingo = { play, press, snapshot, capture, scenes: () => VISIBLE.map(sc => ({ id: sc.id, grp: sc.grp, name: sc.name, card: sc.card, t: sc.t })), get S() { return S; } };
}
init();
