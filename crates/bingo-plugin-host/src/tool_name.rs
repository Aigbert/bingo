/// The model-visible name of a plugin's tool.
///
/// Both names are copied as they are. The permission grammar splits
/// `plugin__<plugin>__<tool>` on its first two separators, so a plugin or a
/// tool whose own name contains `__` still reaches the rule written for it,
/// and a rewritten name would reach none.
pub fn tool_name(plugin: &str, tool: &str) -> String {
    format!("plugin__{plugin}__{tool}")
}
