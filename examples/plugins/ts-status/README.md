# TypeScript status

A module that counts completed dispatches in the host's versioned global
store, places the count in a status slot, and denies Bash commands starting
with `rm -rf` without running the command.

From this directory, run `bingo plugins validate .`, then
`bingo plugins test .`. Tests run in the in-process QuickJS runtime with the
fake provider.

`hooks/register.ts` has no Node dependencies, and the manifest loads it in
process. The repository's copy also keeps `main.ts`, a protocol-2 process
entry for the same register module that Node 22.18+, 23.6+ or 24+ runs
directly.

The acceptance test in `hooks/register.test.ts` reads this file twice, reloads, reads again, checks
that the count is three, and verifies the denial came from this hook.
