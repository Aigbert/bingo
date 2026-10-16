# Band example

From this directory, run `bingo --plugin-dir .`. Click the Hide band button or
press Alt+h while the composer has focus. The state read invalidates the
AbovePrompt request after the handler writes; hiding persists across reload.
The module yields to a survey when `hasSurvey` is true. Typing `h` still edits
the composer; its existing Alt+b and Alt+f bindings keep their meaning.

This original module uses only session state and UI operations. Hiding writes
one `$.ui.log` operation to the application's existing `mod::band`
tracing target. It needs no credentials or full trust.
