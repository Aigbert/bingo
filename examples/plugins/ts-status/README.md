# TypeScript status

A module author-kit example with a protocol-2 process entry. It counts completed dispatches in the host's
versioned global store, places the count in a status slot, and denies Bash
commands starting with `rm -rf` without running the command.

Run `bingo plugins validate examples/plugins/ts-status`, then
`bingo plugins test examples/plugins/ts-status`. Tests run in the in-process QuickJS runtime with the fake provider.
Node 22.18+, 23.6+, or 24+ can run the process entry directly. Copy this directory and
`sdk/typescript` with their relative paths preserved to run outside this tree.

`hooks/register.ts` has no Node dependencies. `main.ts` is the process
transport entry; the manifest loads the same register module in process.
The acceptance test in `hooks/register.test.ts` reads this file twice, reloads, reads again, checks
that the count is three, and verifies the denial came from this hook.
