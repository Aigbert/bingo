# Remaining bodies

| Caller | Owner |
|---|---|
| `transcript::child_row` activity and summaries | M200 |
| `transcript::{answered,receipt}` | M197 |
| `transcript::action` login and reconnect | M198 |
| `transcript/ran.rs`, `transcript/said.rs::notice` | M196 |
| `activity::hung`, `transcript/slots.rs::placed_body` | M196 |
| `transcript/thinking.rs` | M195 |

# Syntect cells

Zero-based syntax snapshot cells:

- `inked_rust`, row 2, columns 19–21: `Now` remains fg without a type scope.
- `inked_rust`, row 2, columns 27–30: `bool` uses kw from the storage-type scope.
- `inked_python`, row 3, columns 24–27: `read` remains fg without a method-call scope.
