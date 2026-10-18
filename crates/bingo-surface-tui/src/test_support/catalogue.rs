//! Screen catalogues register the same tool declarations as a session.
use super::*;

pub fn catalogue(ui: &mut Ui) {
    if ui.catalogs.values.contains_key("tools") {
        return;
    }
    let plugins: Vec<Box<dyn bingo_sdk::Plugin>> = vec![
        Box::new(bingo_tool_fs::FsPlugin),
        Box::new(bingo_tool_bash::BashPlugin::default()),
        Box::new(bingo_tool_web::WebPlugin),
        Box::new(bingo_skills::SkillsPlugin::default()),
        Box::new(bingo_agents::AgentsPlugin::default()),
        Box::new(bingo_tasks::TasksPlugin::default()),
        Box::new(bingo_schedule::SchedulePlugin::default()),
        Box::new(bingo_experience::ExperiencePlugin::default()),
    ];
    let mut entries = Vec::new();
    for plugin in plugins {
        let mut registrar = bingo_sdk::Registrar::new(
            plugin.manifest().id,
            json!({}),
            bingo_sdk::Env::rooted("/tmp/project"),
            Default::default(),
        );
        plugin.register(&mut registrar).unwrap();
        for contribution in registrar.into_contributions() {
            if let bingo_sdk::Contribution::Tool(tool) = contribution {
                let spec = tool.spec();
                entries.push(bingo_sdk::CatalogEntry {
                    id: spec.name.clone(),
                    label: spec.name,
                    meta: json!({"look":spec.look, "traits":tool.traits(&serde_json::Value::Null)}),
                });
            }
        }
    }
    ui.catalogs.values.insert("tools".into(), entries);
}
