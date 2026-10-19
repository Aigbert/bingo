use super::Capture;

#[test]
fn acting_full_frames_match_the_prototype() {
    for name in [
        "live-steered",
        "live-stopped",
        "stop",
        "stop-resent",
        "steer",
        "steer-landed",
        "shell",
        "shell-ls",
        "image",
        "image-sent",
    ] {
        let spinners: &[(u16, u16)] = match name {
            "live-steered" => &[(25, 3)],
            "steer-landed" => &[(24, 3)],
            _ => &[],
        };
        let capture = Capture::load(name).spinners(spinners);
        capture.render(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(crate::run::acting::draw(&capture));
        });
    }
}
