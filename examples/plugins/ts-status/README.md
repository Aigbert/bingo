# TypeScript status

A protocol-2 author-kit example. It counts completed dispatches in the host's
versioned global store, places the count in a status slot, and denies Bash
commands starting with `rm -rf` at the default tighten tier.

Run `bingo plugins validate examples/plugins/ts-status`, then
`bingo plugins test examples/plugins/ts-status`. Node 22.18+, 23.6+, or 24+
can run the erasable TypeScript directly. Copy this directory and
`sdk/typescript` with their relative paths preserved to run outside this tree.

`hooks/register.ts` has no Node dependencies. `main.ts` is the process
transport entry; the same register module is intended for M137's runtime.
The acceptance scenario reads this file twice, reloads, reads again, checks
that the count is three, and verifies the denial came from this hook.
