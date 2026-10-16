use super::*;

#[tokio::test]
async fn catalogue_requests_preserve_the_optional_session() {
    let host = bingo_sdk::testing::TestHost::default();
    host.on_catalog(|kind| async move {
        Ok(bingo_sdk::Catalog {
            kind,
            entries: vec![],
        })
    });
    host.on_catalog_for(|session, kind| async move {
        Ok(bingo_sdk::Catalog {
            kind,
            entries: vec![bingo_sdk::CatalogEntry {
                id: session.to_string(),
                label: "session commands".into(),
                meta: json!({}),
            }],
        })
    });
    let mut wire = Wire::started(host.handle()).await;
    let answer = wire
        .call(
            name::CATALOG_READ,
            json!({"kind":"commands","session":"other"}),
        )
        .await
        .unwrap();
    assert_eq!(answer["entries"][0]["id"], "other");
    let legacy = wire
        .call(name::CATALOG_READ, json!({"kind":"commands"}))
        .await
        .unwrap();
    assert_eq!(legacy["entries"], json!([]));
    wire.finish().await;

    let (client, server) = tokio::io::duplex(64 * 1024);
    let (sr, sw) = tokio::io::split(server);
    let (cr, cw) = tokio::io::split(client);
    let served = tokio::spawn(serve(host.handle(), sr, sw));
    let kernel = RemoteKernel::connect(cr, cw);
    kernel.initialize(who()).await.unwrap();
    let answer = kernel
        .catalog_for(
            &bingo_sdk::SessionId::from_raw("remote"),
            CatalogKind::Commands,
        )
        .await
        .unwrap();
    assert_eq!(answer.entries[0].id, "remote");
    kernel.shutdown().await.unwrap();
    served.await.unwrap().unwrap();
}
