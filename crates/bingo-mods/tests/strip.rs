#![allow(clippy::unwrap_used, clippy::expect_used)]
use bingo_mods::strip::strip;
use std::path::Path;

#[test]
fn typescript_and_jsx_compile_without_a_runtime_import() {
    let js = strip(
        Path::new("view.tsx"),
        "const n: number = 1; export const view = <><Text>{n}</Text></>;",
    )
    .unwrap();
    assert!(!js.contains(": number"));
    assert!(js.contains("h(") && js.contains("Fragment"), "{js}");
    assert!(!js.contains("react"));
}

#[test]
fn imports_stay_inside_the_module_root_or_bingo() {
    for specifier in ["bingo", "./sibling.ts", "./nested/view.tsx"] {
        strip(
            Path::new("register.ts"),
            &format!("import {{ x }} from '{specifier}'; x();"),
        )
        .unwrap();
    }
    for source in [
        "import x from 'node:fs';",
        "import x from 'https://example.com/x.js';",
        "import x from '../escape.ts';",
        "import x from '/escape.ts';",
        "export * from 'other';",
        "import('./local.ts');",
        "require('./local.ts');",
        "const r = require; r('./local.ts');",
    ] {
        let error = strip(Path::new("register.ts"), &format!("\n{source}"))
            .unwrap_err()
            .to_string();
        assert!(error.contains("register.ts:2:"), "{error}");
    }
}

#[test]
fn strings_and_comments_do_not_count_as_dynamic_imports() {
    strip(
        Path::new("register.ts"),
        "// require('x')\nexport const text = `import('x')`;",
    )
    .unwrap();
}
