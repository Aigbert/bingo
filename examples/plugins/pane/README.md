# Pane example

Run `bingo --plugin-dir examples/plugins/pane`, run two tool calls, then type
`/ledger`. The pane has one row per call; Escape closes it. `/ledger` opens it
again with the same rows. The state lives in the host, so editing this module
and completing a turn reloads the module without losing the ledger.

This original module uses `atom`, a render-local `derive`, `read` and `update`.
Its render matcher selects both Pane and the stable `ledger` request id.
Opening happens only from the person's command. It uses state, UI and command
registration, with the ordinary rewrite grant and no credentials.
