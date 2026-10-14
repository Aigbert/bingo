# Choosing a model

`/model` shows the session's provider, model and requested thinking level.
Use `/model openai/gpt-5.4` to try a model in this session, or
`/model openai/gpt-5.4 --keep` to also save the provider and model for new
sessions. Project and command-line settings still take precedence at startup.
`--model` and `--provider` never write settings.

`/think high` changes the requested effort for this session. Add `--keep`
to save it for new sessions. `/think off` omits the reasoning parameter;
the provider's default then applies. Model reasoning metadata is informative
and does not prohibit a requested effort.

`/model pin openai/gpt-5.4` adds an offered model to the ordered `pinned`
setting. `/model unpin openai/gpt-5.4` removes it. Both preserve unrelated
user settings and TOML comments, and neither changes the active model.

The model catalogue supplies context and output limits, prices per million
tokens (input, output and cache reads), image input, chat suitability, dated
aliases, distinct supported effort levels and pin state. Missing prices mean
unknown. Unknown models remain visible as chat candidates. Facts come from
the bundled snapshot; listing facts never fetches prices from a provider.

If the current context exceeds the target model's compaction threshold, the
switch waits behind the active turn and compacts on the current model first.
The outcome reports the before and after token counts. Failed or insufficient
compaction leaves the model unchanged. Switching to a model without image
input keeps pictures in the transcript and warns that the model cannot see them.
