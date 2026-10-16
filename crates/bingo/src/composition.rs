//! Decisions made before a host exists, from the kernel's settings merge.

use crate::Cli;
use bingo_channel_adapter::Platforms;
use bingo_core::settings::{self, Claim, Merged};
use bingo_sdk::{ErrorCode, KernelError, Plugin};

#[derive(Default)]
pub(crate) struct Composition {
    pub mods: bingo_plugin_rpc::Settings,
    pub print: bingo_surface_print::Options,
    pub rpc: bingo_surface_rpc::Options,
    pub tui: bingo_surface_tui::Options,
}

impl Composition {
    pub fn for_cli(cli: &Cli, merged: &Merged) -> Self {
        Self {
            mods: serde_json::from_value(
                merged
                    .plugins
                    .get(bingo_plugin_rpc::PluginRpcPlugin::default().manifest().id)
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({})),
            )
            .unwrap_or_default(),
            print: bingo_surface_print::Options {
                output_format: cli.output_format,
                input_format: cli.input_format,
                permission_prompt_tool: cli.permission_prompt_tool,
                images: cli.image.clone(),
            },
            rpc: bingo_surface_rpc::Options::default(),
            tui: bingo_surface_tui::Options {
                print_on_exit: !cli.no_print_on_exit,
                picture_cache_days: merged.kernel.picture_cache_days,
            },
        }
    }
}

pub(crate) fn merged(
    layers: &[settings::Layer],
    platforms: &Platforms,
) -> Result<Merged, KernelError> {
    let candidates = crate::plugins(Composition::default(), platforms, std::path::Path::new("."))?;
    let claims: Vec<_> = candidates
        .iter()
        .filter_map(|plugin| Claim::from_manifest(plugin.manifest()))
        .collect();
    settings::merge(layers, &claims)
        .map_err(|e| KernelError::new(ErrorCode::InvalidInput, e.to_string()))
}

pub(crate) fn listening(
    cli: &Cli,
    merged: &Merged,
    platforms: &Platforms,
) -> Result<bool, KernelError> {
    let plugin = bingo_channels::ChannelsPlugin::new(platforms.clone());
    let slice = merged
        .plugins
        .get(plugin.manifest().id)
        .cloned()
        .unwrap_or_default();
    Ok(!cli.channels.is_empty()
        || bingo_channels::wanted(&slice, platforms)
            .map_err(|e| KernelError::new(ErrorCode::InvalidInput, e))?)
}

// Remove these compatibility inputs after the next release; never rewrite settings.
pub(crate) fn retired_demo(cli: &Cli, layers: &[settings::Layer]) {
    if cli.demo_ui
        || layers.iter().any(|layer| {
            layer.value.contains_key("demoUi")
                || layer
                    .value
                    .get("enabledPlugins")
                    .and_then(|value| value.get("bingo.demo.ui"))
                    .is_some()
        })
    {
        eprintln!(
            "[notice] DEMO_REMOVED: the demo plugin is gone; remove --demo-ui, demoUi, or enabledPlugins[\"bingo.demo.ui\"]"
        );
    }
}
