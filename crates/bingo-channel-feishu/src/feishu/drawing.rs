use super::*;
use bingo_channel_adapter::drawing::Drawing;

impl Feishu {
    pub(super) async fn draw(
        &self,
        to: &Conversation,
        parent: Option<&Posted>,
        at: Option<&Posted>,
        drawing: &Drawing,
    ) -> Result<Posted, ChannelError> {
        let images = self.view_images(&drawing.view, &drawing.cwd).await;
        let card = view::card(drawing, to, &images);
        let handle = match at.and_then(Handle::of) {
            Some(Handle::Card(id)) => {
                // Close through the existing renewal path before enabling controls.
                if locked(&self.streaming_since).contains_key(&id) {
                    self.write(&id, &self.limits.clip(&drawing.fold())).await?;
                    self.streaming(&id, false).await?;
                }
                self.replace_card(&id, &card).await?;
                Handle::Card(id)
            }
            _ => self.post_view(to, parent, &card).await?,
        };
        Ok(handle.posted())
    }

    async fn replace_card(&self, id: &str, card: &Value) -> Result<(), ChannelError> {
        if let Some(chat) = self.chat_of(id) {
            self.queue.turn(&chat).await;
        }
        let sequence = self.sequence(id);
        self.api
            .put(
                &format!("{CARDS}/{id}"),
                json!({"card":card::entity(card),
            "sequence":sequence,"uuid":format!("{id}-{sequence}")}),
            )
            .await?;
        Ok(())
    }

    async fn post_view(
        &self,
        to: &Conversation,
        parent: Option<&Posted>,
        card: &Value,
    ) -> Result<Handle, ChannelError> {
        let answer = self.api.post(CARDS, card::entity(card)).await?;
        let id = answer["data"]["card_id"]
            .as_str()
            .ok_or_else(|| ChannelError::Platform("feishu created no card".into()))?;
        self.deliver(to, parent, "interactive", card::by_id(id))
            .await?;
        self.remember(id, &to.chat);
        Ok(Handle::Card(id.into()))
    }

    async fn view_images(
        &self,
        view: &bingo_sdk::View,
        cwd: &std::path::Path,
    ) -> BTreeMap<String, String> {
        let mut images = BTreeMap::new();
        for source in view::image_sources(view) {
            if images.contains_key(&source) {
                continue;
            }
            match self.view_image(&source, cwd).await {
                Ok(key) => {
                    images.insert(source, key);
                }
                Err(error) => tracing::warn!(%error, "the card image uses its text alternative"),
            }
        }
        images
    }

    async fn view_image(
        &self,
        source: &str,
        cwd: &std::path::Path,
    ) -> Result<String, ChannelError> {
        let source = bingo_pictures::Source::parse(source, cwd);
        let image = bingo_pictures::load(&source, None)
            .await
            .map_err(|e| ChannelError::Platform(e.to_string()))?;
        let png =
            bingo_pictures::to_png(&image).map_err(|e| ChannelError::Platform(e.to_string()))?;
        self.upload(
            IMAGES,
            &[("image_type", "message")],
            "image",
            &Outgoing {
                name: "view.png".into(),
                bytes: png.bytes,
                caption: None,
            },
        )
        .await
    }
}

#[cfg(test)]
mod tests;
