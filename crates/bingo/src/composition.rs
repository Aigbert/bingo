//! Decisions made before a host exists, from the kernel's settings merge.

use crate::Cli;
use bingo_channel_adapter::Platforms;
use bingo_core::settings::{self, Claim, Merged};
use bingo_sdk::{ErrorCode, KernelError, Plugin};

#[derive(Default)]
pub(crate) struct Composition {
    pub demo: bool,
    pub mods: bingo_plugin_rpc::Settings,
    pub print: bingo_surface_print::Options,
    pub rpc: bingo_surface_rpc::Options,
    pub tui: bingo_surface_tui::Options,
}

impl Composition {
    pub fn for_cli(cli: &Cli, merged: &Merged) -> Self {
        Self {
            demo: demo(cli.demo_ui, merged),
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
    // Include the opt-in plugin's claim before deciding whether to compose it.
    let candidates = crate::plugins(
        Composition {
            demo: true,
            ..Default::default()
        },
        platforms,
        std::path::Path::new("."),
    )?;
    let claims: Vec<_> = candidates
        .iter()
        .filter_map(|plugin| Claim::from_manifest(plugin.manifest()))
        .collect();
    settings::merge(layers, &claims)
        .map_err(|e| KernelError::new(ErrorCode::InvalidInput, e.to_string()))
}

pub(crate) fn demo(flag: bool, merged: &Merged) -> bool {
    flag || merged
        .kernel
        .enabled_plugins
        .get(bingo_demo_ui::PLUGIN)
        .copied()
        .unwrap_or_else(|| {
            merged
                .plugins
                .get(bingo_demo_ui::PLUGIN)
                .is_some_and(bingo_demo_ui::wanted)
        })
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
