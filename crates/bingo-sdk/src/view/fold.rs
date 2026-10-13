use super::*;

impl View {
    pub fn column(children: Vec<View>) -> Self {
        Self::Box {
            children,
            layout: BoxStyle::default(),
        }
    }
    pub fn columns(children: Vec<View>) -> Self {
        let count = children.len();
        let width = NARROW
            .saturating_sub(COLUMN_GAP * count.saturating_sub(1))
            .checked_div(count)
            .unwrap_or(NARROW);
        let children = children
            .into_iter()
            .map(|child| Self::Box {
                children: vec![child],
                layout: BoxStyle {
                    flex_grow: Some(1),
                    width: Some(width as u16),
                    ..Default::default()
                },
            })
            .collect();
        Self::Box {
            children,
            layout: BoxStyle {
                flex_direction: Some(Direction::Row),
                flex_wrap: Some(Wrap::Wrap),
                gap: Some(COLUMN_GAP as u16),
                ..Default::default()
            },
        }
    }

    pub fn text(text: impl Into<String>) -> Self {
        View::Text {
            children: vec![TextChild::String(text.into())],
            style: TextStyle::default(),
        }
    }

    /// The degrade: what `--print`, an IM channel and a surface that cannot
    /// draw a node show instead.
    pub fn fold(&self) -> String {
        self.drawn(Table::Joined)
    }

    /// The same walk, for a surface that draws markdown — a card. Only the
    /// table differs: cells joined with ` · ` are not a table to anything that
    /// can draw one, so they are written as the markdown table it draws.
    pub fn fold_markdown(&self) -> String {
        self.drawn(Table::Markdown)
    }

    fn drawn(&self, table: Table) -> String {
        match self {
            View::Text { children, .. } => text_children(children),
            View::Markdown { text, .. } => text.clone(),
            View::Code { source, .. } => source.clone(),
            View::Button { children, button } => format!(
                "[{}]",
                button
                    .label
                    .clone()
                    .unwrap_or_else(|| text_children(children))
            ),
            View::Input { input } => labeled(
                input.label.as_deref(),
                input.value.as_deref().unwrap_or_default(),
            ),
            View::Select { select } => {
                let selected = select
                    .options
                    .iter()
                    .flatten()
                    .find(|option| Some(&option.value) == select.value.as_ref());
                labeled(
                    select.label.as_deref(),
                    selected
                        .map(|option| option.label.as_str())
                        .or(select.value.as_deref())
                        .unwrap_or_default(),
                )
            }
            View::Link { href, label } => label
                .as_ref()
                .map(|label| format!("{label} ({href})"))
                .unwrap_or_else(|| href.clone()),
            View::Image { alt, .. } | View::Svg { alt, .. } => alt.clone(),
            View::Raster { cells, .. } => cells.join("\n"),
            View::Engine { .. } => String::new(),
            View::Diff { unified } => unified.clone(),
            View::List { items } => lines(items.iter().map(|item| format!("- {item}"))),
            View::Table { headers, rows } => table.draw(headers, rows),
            View::KeyValue { rows } => lines(rows.iter().map(|(k, v)| format!("{k}: {v}"))),
            View::Progress {
                value,
                total,
                label,
            } => progress(*value, *total, label.as_deref()),
            View::Badge { text, .. } => format!("[{text}]"),
            View::Tree { nodes } => lines(nodes.iter().flat_map(|node| node.fold(0))),
            View::Box { children, .. } => children
                .iter()
                .map(|child| child.drawn(table))
                .collect::<Vec<_>>()
                .join(table.between()),
            View::Panel { title, child } => {
                format!("{title}{}{}", table.between(), child.drawn(table))
            }
            View::Actions { items } => items
                .iter()
                .map(|item| format!("[{}]", item.label))
                .collect::<Vec<_>>()
                .join(" "),
            // The one node whose fold its author wrote (ADR-0038 §1): the sdk
            // has no word for it, so it has nothing of its own to say.
            View::Custom { fold, .. } => fold.clone(),
        }
    }
}

/// How a table is written: one line per row, or the markdown a surface with a
/// renderer draws it from. The two differ in one thing only — the separator
/// between cells, which is what makes a table a table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Table {
    Joined,
    Markdown,
}

impl Table {
    /// What goes between two nodes of a stack, a columns node or a panel. A
    /// markdown table is not one while a line of prose runs into it — the
    /// prose would be read as another row — so each block gets air of its own.
    fn between(self) -> &'static str {
        match self {
            Table::Joined => "\n",
            Table::Markdown => "\n\n",
        }
    }

    fn draw(self, headers: &[String], rows: &[Vec<String>]) -> String {
        match self {
            Table::Joined => lines(
                std::iter::once(headers)
                    .chain(rows.iter().map(Vec::as_slice))
                    .map(|row| row.join(" · ")),
            ),
            Table::Markdown => self.markdown(headers, rows),
        }
    }

    /// A header row, the rule under it, then the rows. A table with no header
    /// at all has no columns to rule, so it folds to nothing.
    fn markdown(self, headers: &[String], rows: &[Vec<String>]) -> String {
        if headers.is_empty() {
            return String::new();
        }
        let rule = vec!["---".to_string(); headers.len()];
        lines(
            std::iter::once(headers)
                .chain(std::iter::once(rule.as_slice()))
                .chain(rows.iter().map(Vec::as_slice))
                .map(markdown_row),
        )
    }
}

/// One row. A cell carrying the column separator or a line break would break
/// the table it sits in, so each is written as the markdown that stands for it.
fn markdown_row(cells: &[String]) -> String {
    let cells: Vec<String> = cells
        .iter()
        .map(|cell| {
            cell.replace("\r\n", "\n")
                .replace('\r', "\n")
                .replace('|', "\\|")
                .replace('\n', "<br>")
        })
        .collect();
    format!("| {} |", cells.join(" | "))
}

impl TreeNode {
    fn fold(&self, depth: usize) -> Vec<String> {
        let badge = self
            .badge
            .as_ref()
            .map(|badge| format!(" [{badge}]"))
            .unwrap_or_default();
        let mut out = vec![format!("{}{}{badge}", "  ".repeat(depth), self.label)];
        out.extend(self.children.iter().flat_map(|child| child.fold(depth + 1)));
        out
    }
}

fn lines(rows: impl Iterator<Item = String>) -> String {
    rows.collect::<Vec<_>>().join("\n")
}

fn progress(value: u64, total: Option<u64>, label: Option<&str>) -> String {
    let amount = match total {
        Some(total) if total > 0 => format!("{} %", value * 100 / total),
        _ => value.to_string(),
    };
    match label {
        Some(label) => format!("{label} {amount}"),
        None => amount,
    }
}

pub fn text_children(children: &[TextChild]) -> String {
    children
        .iter()
        .map(|child| match child {
            TextChild::String(text) => text.clone(),
            TextChild::Node(view) => view.fold(),
        })
        .collect()
}
fn labeled(label: Option<&str>, value: &str) -> String {
    label
        .map(|label| format!("{label}: {value}"))
        .unwrap_or_else(|| value.to_owned())
}
