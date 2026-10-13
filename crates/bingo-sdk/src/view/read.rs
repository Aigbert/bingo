use super::*;
use serde::{Deserializer, Serializer, de};

impl<'de> Deserialize<'de> for View {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut node = Value::deserialize(deserializer)?;
        migrate(&mut node);
        let kind = node.get("kind").and_then(Value::as_str);
        if let Some(kind) = kind.filter(|kind| !KNOWN.contains(kind)) {
            return Ok(caught(kind.to_owned(), node));
        }
        #[derive(Deserialize)]
        struct Node(#[serde(with = "View")] View);
        serde_json::from_value::<Node>(node)
            .map(|node| node.0)
            .map_err(de::Error::custom)
    }
}

impl Serialize for View {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Node<'a>(#[serde(with = "View")] &'a View);
        Node(self).serialize(serializer)
    }
}

fn migrate(node: &mut Value) {
    let Some(object) = node.as_object_mut() else {
        return;
    };
    match object.get("kind").and_then(Value::as_str) {
        Some("text") => {
            object.insert("kind".into(), "Text".into());
        }
        Some("code") => {
            object.insert("kind".into(), "Code".into());
        }
        Some("markdown") => {
            object.insert("kind".into(), "Markdown".into());
        }
        Some("stack" | "columns") => {
            let columns = object["kind"] == "columns";
            object.insert("kind".into(), "Box".into());
            if columns {
                columns_layout(object);
            }
        }
        _ => {}
    }
    match object.get("kind").and_then(Value::as_str) {
        Some("Text") if !object.contains_key("children") => {
            if let Some(text) = object.remove("text") {
                object.insert("children".into(), Value::Array(vec![text]));
            }
        }
        Some("Code") => {
            rename(object, "text", "source");
            rename(object, "lang", "language");
        }
        _ => {}
    }
}

fn rename(object: &mut serde_json::Map<String, Value>, old: &str, new: &str) {
    if let Some(value) = object.remove(old) {
        object.entry(new).or_insert(value);
    }
}

fn columns_layout(object: &mut serde_json::Map<String, Value>) {
    object.insert("flexDirection".into(), "row".into());
    object.insert("flexWrap".into(), "wrap".into());
    object.insert("gap".into(), COLUMN_GAP.into());
    if let Some(Value::Array(children)) = object.get_mut("children") {
        let count = children.len();
        let width = NARROW
            .saturating_sub(COLUMN_GAP * count.saturating_sub(1))
            .checked_div(count)
            .unwrap_or(NARROW);
        for child in children {
            *child = serde_json::json!({"kind":"Box", "flexGrow":1, "width":width, "children":[child.clone()]});
        }
    }
}

fn caught(kind: String, node: Value) -> View {
    let fold = node
        .get("fold")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| format!("[{kind}]"));
    View::Custom {
        kind,
        data: node,
        fold,
    }
}
