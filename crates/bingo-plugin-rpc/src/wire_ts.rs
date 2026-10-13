//! The author SDK's wire is derived from the same document as every process.
#[cfg(test)]
mod tests {
    #[test]
    fn committed_wire_is_generated() {
        let rendered = bingo_schema_ts::render(&crate::schema::document());
        assert!(
            !rendered.is_empty(),
            "the renderer must emit the protocol types"
        );
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../sdk/typescript/src/wire.ts");
        if std::env::var_os("BINGO_UPDATE_SCHEMA").is_some() {
            std::fs::write(path, rendered).unwrap();
        } else {
            assert_eq!(
                std::fs::read_to_string(path).unwrap_or_default(),
                rendered,
                "wire.ts drift: run BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc wire_ts::"
            );
        }
    }
}
