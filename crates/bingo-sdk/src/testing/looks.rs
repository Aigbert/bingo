//! Invariants shared by each plugin's pinned tool declarations.
use std::collections::BTreeMap;

use serde_json::Value;

use crate::{Contribution, Env, Plugin, PluginError, Registrar, ToolLook, ToolSpec};

/// Register a plugin, check its declared looks, and return them by tool name.
pub fn tool_looks(
    plugin: &dyn Plugin,
    settings: Value,
) -> Result<BTreeMap<String, ToolLook>, PluginError> {
    let mut registrar = Registrar::new("look-test", settings, Env::rooted("."), Default::default());
    plugin.register(&mut registrar)?;
    let mut looks = BTreeMap::new();
    for contribution in registrar.into_contributions() {
        let Contribution::Tool(tool) = contribution else {
            continue;
        };
        let spec = tool.spec();
        let look = checked_look(&spec);
        looks.insert(spec.name, look);
    }
    Ok(looks)
}

/// Check a tool's declared mark, verb and target against its input schema.
pub fn checked_look(spec: &ToolSpec) -> ToolLook {
    let Some(look) = spec.look.clone() else {
        panic!("{} has no look", spec.name)
    };
    assert!(!look.verb.trim().is_empty(), "{} verb", spec.name);
    let normal = crate::look::row(&spec.name, &Value::Null, Some(&look), false);
    let ascii = crate::look::row(&spec.name, &Value::Null, Some(&look), true);
    assert_eq!(normal.mark, look.mark, "{} mark", spec.name);
    assert_eq!(
        ascii.mark,
        look.ascii.unwrap_or(look.mark),
        "{} ASCII mark",
        spec.name
    );
    assert!(ascii.mark.is_ascii(), "{} ASCII mark", spec.name);
    if let Some(target) = &look.target {
        assert!(
            spec.input_schema
                .get("properties")
                .and_then(|p| p.get(&target.field))
                .is_some(),
            "{} target {} is not an input property",
            spec.name,
            target.field
        );
    }
    look
}
