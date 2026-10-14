#!/usr/bin/env bash
# Regenerate the embedded models.dev snapshot (ADR-0004).
#
# The kernel reads the raw api.json shape pruned to the fields it uses, so a
# runtime refresh is the same download without the pruning. Providers listed
# here are the ones a bingo provider can be pointed at: the two it speaks to
# directly, and the vendors an OpenAI-compatible proxy commonly fronts (found
# through the cross-provider lookup by model id).
set -euo pipefail
cd "$(dirname "$0")/.."
out=crates/bingo-core/models.dev.json
preserve=false
if [ "${1:-}" = --preserve-capabilities ]; then preserve=true; shift; fi
if [ "$#" -gt 1 ] || [[ "${1:-}" == --* ]]; then
  echo 'usage: models_dev.sh [--preserve-capabilities] [snapshot.json]' >&2
  exit 1
fi
fresh=$(mktemp)
chosen=$(mktemp)
trap 'rm -f "$fresh" "$chosen"' EXIT
source_json() {
  if [ "$#" -gt 0 ]; then cat "$1";
  else curl --fail -sSL --max-time 120 https://models.dev/api.json; fi
}
providers='["anthropic","openai","deepseek","google","xai","mistral","moonshotai","zai","alibaba","minimax","meta","groq"]'
source_json "$@" \
  | jq --argjson keep "$providers" '
      with_entries(select(.key as $k | $keep | index($k)))
      | with_entries(.value |= {
          models: (.models | with_entries(.value |= {
            limit: {context: .limit.context, output: .limit.output},
            reasoning: .reasoning,
            cost: (.cost | if . == null then null else {input, output, cache_read, cache_write} end),
            modalities: {input: .modalities.input, output: .modalities.output}
          }))
        })' > "$fresh"
if [ "$preserve" = true ]; then
  # Enrich choice metadata without changing an existing model's requests.
  jq --slurpfile previous "$out" '
    $previous[0] as $old | ($old * .)
    | with_entries(.key as $provider | .value.models |= with_entries(
        .key as $id | $old[$provider].models[$id] as $before
        | if $before == null then . else .value |= (
            .limit = $before.limit | .reasoning = $before.reasoning
            | .modalities.input = $before.modalities.input
          ) end
      ))' "$fresh" > "$chosen"
  mv "$chosen" "$out"
else
  mv "$fresh" "$out"
fi
printf 'wrote %s: %s providers, %s models, %s bytes\n' "$out" \
  "$(jq 'length' "$out")" "$(jq '[.[] | .models | length] | add' "$out")" "$(wc -c < "$out" | tr -d ' ')"
