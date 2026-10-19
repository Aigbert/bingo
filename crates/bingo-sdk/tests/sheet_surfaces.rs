use bingo_sdk::render::SurfaceName;

#[test]
fn only_terminal_and_desktop_draw_sheets() {
    for (surface, expected) in [
        (SurfaceName::Terminal, true),
        (SurfaceName::Desktop, true),
        (SurfaceName::Text, false),
        (SurfaceName::Chat, false),
    ] {
        assert_eq!(surface.shows_sheets(), expected, "{surface:?}");
    }
}
