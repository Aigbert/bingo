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
                let action = mount::action(tree, key, method, value)?;
                let intent = IntentId::mint();
                self.session
                    .handle
                    .submit(intent.clone(), Input::Action { action });
                self.wait(intent, false).await?;
                Ok(Value::Null)
            }
            _ => Err(format!("unknown mount operation {method}")),
        }
    }
}
