use super::driver::{Driver, error};
use bingo_mods::mount::{self, Mount};
use bingo_sdk::{Input, IntentId};
use serde_json::{Value, json};
impl Driver {
    pub(super) fn mount(&mut self, params: Value) -> Result<Value, String> {
        let mount = Mount::new(
            params["component"].clone(),
            params["props"].clone(),
            params["viewport"].clone(),
        )?;
        let id = mount.request.request_id.clone();
        self.mounts.insert(id.clone(), mount);
        Ok(json!(id))
    }
    pub(super) async fn mounted(&mut self, method: &str, input: Value) -> Result<Value, String> {
        let id = input["id"].as_str().ok_or("mount id is missing")?;
        let params = &input["params"];
        if method == "press" && params.as_str() == Some("Escape") {
            let action = self.pane_close(id)?;
            self.fire(action).await?;
            self.mounts.remove(id);
            return Ok(Value::Null);
        }

        if method == "advance" {
            self.advance(params.as_u64().ok_or("advance requires milliseconds")?)
                .await;
            return Ok(Value::Null);
        }
        let mount = self.mounts.get_mut(id).ok_or("mount is not open")?;
        if method == "resize" {
            let columns = serde_json::from_value(params["columns"].clone()).map_err(error)?;
            let rows = serde_json::from_value(params["rows"].clone()).map_err(error)?;
            mount.resize(columns, rows);
            return Ok(Value::Null);
        }
        let width = usize::from(mount.request.viewport.columns);
        let tree = mount
            .tree(&self.host.handle(), &self.session.snapshot.summary.id)
            .await?;
        match method {
            "tree" => serde_json::to_value(tree).map_err(error),
            "drawn" => Ok(json!(bingo_surface_tui::drawn(tree, width).join("\n"))),
            "find" => serde_json::to_value(mount::find(
                tree,
                params.as_str().ok_or("find requires text or a key")?,
            ))
            .map_err(error),
            "press" | "input" | "select" => {
                let action = interaction(tree, method, params)?;
                self.fire(action).await?;
                Ok(Value::Null)
            }
            _ => Err(format!("unknown mount operation {method}")),
        }
    }
}

impl Driver {
    async fn fire(&mut self, action: bingo_sdk::Action) -> Result<(), String> {
        let intent = IntentId::mint();
        self.session.handle.submit(
            intent.clone(),
            Input::Action {
                action,
                origin: Some(bingo_sdk::Origin::surface("plugin-kit")),
            },
        );
        self.wait(intent, false).await
    }
    fn pane_close(&self, id: &str) -> Result<bingo_sdk::Action, String> {
        let pane = bingo_sdk::slot::placed(&self.session.snapshot).into_iter().find(|placed| {
            placed.plugin == self.name && matches!(&placed.slot,bingo_sdk::slot::Slot::Pane {id: pane,close_on_escape:true,..} if pane == id)
        }).ok_or("no mounted pane accepts Escape")?;
        Ok(bingo_sdk::Action {
            name: format!("{}:ui", pane.plugin),
            args: json!({"handler":id,"event":"close"}),
        })
    }
}

fn interaction(
    tree: &bingo_sdk::View,
    method: &str,
    params: &Value,
) -> Result<bingo_sdk::Action, String> {
    let key = if method == "press" {
        params.as_str()
    } else {
        params["key"].as_str()
    }
    .ok_or("interaction requires a key")?;
    let value = if method == "input" {
        params["text"].clone()
    } else {
        params["value"].clone()
    };
    mount::action(tree, key, method, value)
}
