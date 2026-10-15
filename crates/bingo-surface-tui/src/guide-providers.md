# Managing providers

`/provider add work openai https://proxy.example` adds a named instance
immediately. Names use only ASCII letters, digits, `_` and `-`; an existing
stored credential must be removed before its name can be reused. With no active
provider or owning-family declaration in any layer, `/provider remove work`
deletes that orphaned credential. Shapes are
`openai`, `anthropic`, and `codex`. Key-based shapes
open a paste dialog; the key goes only to `auth.json`. Cancelling sign-in
keeps the instance, and `/login work` can finish later. URLs must use HTTP(S)
and contain no userinfo, query, or fragment; paste credentials into login.

`/models refresh work` checks one endpoint. The providers catalogue reports
its last reach result and whether its credential came from the store,
settings, or environment. A failed check keeps the instance and its previous
model list. `/provider set work url https://new.example` replaces its address.
A same-origin path change keeps the key. Changing the scheme, host, or port
clears the key and opens a paste dialog. If a project, local, or other non-user
settings file holds its `apiKey`, the change is refused and names that file;
remove the key there first. Existing sessions keep their resolved
endpoint, but a retired origin stops at its next request; choose the provider
again to use its new address.

`/provider remove work` removes its user settings entry and stored credential
and clears its saved provider, model, and matching model pins. Live sessions
using it stop at their next request with `AUTH_REQUIRED` naming the removal; choose another
provider to continue. Default providers cannot be removed.
Instances supplied only by another settings layer must be edited there.
`bingo provider add` uses the same settings writer before a session starts.

Stored keys are bound to the address where they were entered; OAuth tokens
also bind their issuer. If a settings override changes an origin, the instance
stays listed but signed out. The notice names the address and override file:
sign in again for that address, or remove the override there. Old credentials
migrate only from an unambiguous instance declaration in the user settings.
Environment keys follow only the default address or their `*_BASE_URL` variable.
Settings keys follow addresses from their own or more trusted settings layers.
A repository override cannot redirect either key to another origin. Before you
sign in under an override, confirm the destination origin and file shown in the
prompt. Blank addresses mean unset; malformed addresses cannot store a key.

Boot skips invalid or colliding names and names the declaration file to edit.
A credential owned by another family stays with that family; rename or delete
the conflicting declaration. A plugin provider takes precedence over an instance
with the same name; its notice names the plugin and the instance's file.
Shipped family names remain reserved even when their plugins are switched off.
