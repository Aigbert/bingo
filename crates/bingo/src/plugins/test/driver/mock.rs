use super::*;
impl Driver {
    pub(super) async fn mock(&mut self, method: &str, params: Value) -> Result<Value, String> {
        match method {
            "session.submit" => self.submit(params).await,
            "session.waitForSlot" => self.wait_for_slot(params).await,
            "session.snapshot" => {
                self.settle().await?;
                serde_json::to_value(&self.session.snapshot).map_err(error)
            }
            "session" => self.replace_session(params).await,
            "clock.set" => {
                self.pause();
                self.environment
                    .set(params.as_str().ok_or("clock.set requires a timestamp")?)?;
                Ok(Value::Null)
            }
            "clock.advance" => {
                self.advance(
                    params
                        .as_u64()
                        .ok_or("clock.advance requires milliseconds")?,
                )
                .await;
                Ok(Value::Null)
            }
            "env" => {
                self.environment
                    .seed_env(serde_json::from_value(params).map_err(error)?);
                Ok(Value::Null)
            }
            "store.seed" => self.seed_store(params).await,
            "store.read" => {
                self.modules
                    .testing_call(
                        &self.name,
                        "host/state/get",
                        json!({"scope":"global","key":params}),
                    )
                    .await
            }
            "reload" => {
                self.modules.reload(&self.root).await.map_err(error)?;
                for mount in self.mounts.values_mut() {
                    let request = mount.request.request_id.clone();
                    mount.invalidate(std::slice::from_ref(&request));
                }
                Ok(Value::Null)
            }
            "mount" => self.mount(params),
            _ if method.starts_with("mount.") => self.mounted(&method[6..], params).await,
            _ => Err(format!("unknown testing operation {method}")),
        }
    }
    async fn wait_for_slot(&mut self, params: Value) -> Result<Value, String> {
        let plugin = params["plugin"].as_str().ok_or("plugin is required")?;
        let key = params["slot"].as_str().ok_or("slot is required")?;
        loop {
            if bingo_sdk::slot::placed(&self.session.snapshot)
                .iter()
                .any(|placed| placed.plugin == plugin && placed.kind == key)
            {
                return Ok(Value::Null);
            }
            let frame = self
                .session
                .events
                .next()
                .await
                .ok_or("session stream ended")?;
            self.fold(&frame);
        }
    }
    async fn submit(&mut self, params: Value) -> Result<Value, String> {
        let intent = IntentId::mint();
        self.session.handle.submit(
            intent.clone(),
            Input::text(
                params.as_str().ok_or("submit requires text")?,
                Origin::surface("plugin-kit"),
            ),
        );
        self.wait(intent, true).await?;
        Ok(Value::Null)
    }
    async fn replace_session(&mut self, params: Value) -> Result<Value, String> {
        self.host.shutdown().await;
        let (host, modules, session) = build(
            &self.root,
            self.home.path(),
            self.environment.clone(),
            params,
        )
        .await?;
        self.host = host;
        self.modules = modules;
        self.session = session;
        self.mounts.clear();
        Ok(self.site_value())
    }
    async fn seed_store(&mut self, params: Value) -> Result<Value, String> {
        for (key, value) in params.as_object().ok_or("store.seed requires an object")? {
            self.modules
                .testing_call(
                    &self.name,
                    "host/state/set",
                    json!({"scope":"global","key":key,"value":value}),
                )
                .await?;
        }
        Ok(Value::Null)
    }
}
