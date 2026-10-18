use super::*;

// Computed once with node from p3-core.js PALS; no JavaScript at test time.
#[test]
fn every_light_prototype_ink_matches() {
    for (name, expected) in [
        ("bg", 0xf9f4f2),
        ("fg", 0x3a342e),
        ("ember", 0xb04c14),
        ("glow", 0xef7a2c),
        ("coal", 0xb04c14),
        ("amber", 0x9a6200),
        ("amberGlow", 0xe0a01c),
        ("good", 0x2b7d3c),
        ("bad", 0xb3261e),
        ("plan", 0x2f62a8),
        ("auto", 0x7445c0),
        ("code", 0x8a5520),
        ("kw", 0x8a3fa8),
        ("str", 0x4d7a1c),
        ("num", 0xb04c14),
        ("fn", 0x2f62a8),
        ("ty", 0x11806c),
        ("crew0", 0x5d7a0e),
        ("crew1", 0x11806c),
        ("crew2", 0x6a4fc2),
        ("crew3", 0xa23a9e),
        ("crew4", 0xb8456e),
        ("crew5", 0x2a72b5),
        ("muted", 0x77716d),
        ("dim", 0xa39e9a),
        ("faint", 0xdad5d3),
        ("you", 0xefeae8),
        ("well", 0xf3eeec),
        ("wellDeep", 0xece7e4),
        ("pick", 0xefd7c8),
        ("pickBg", 0xf8e4d8),
        ("addTint", 0xe0e6dc),
        ("delTint", 0xf2dfdd),
        ("addWord", 0xc3d5c3),
        ("delWord", 0xeac7c3),
        ("badTint", 0xf3e4e1),
        ("amberTint", 0xede1d0),
        ("glowTint", 0xf8e8de),
    ] {
        assert_eq!(LIGHT.named(name), Some(rgb(expected)), "light {name}");
    }
}

#[test]
fn every_dark_prototype_ink_matches() {
    for (name, expected) in [
        ("bg", 0x2c2a2e),
        ("fg", 0xe6e2dc),
        ("ember", 0xf39050),
        ("glow", 0xff9b4a),
        ("coal", 0xd8622e),
        ("amber", 0xf2b84b),
        ("amberGlow", 0xf2b84b),
        ("good", 0x7fd38a),
        ("bad", 0xf0706a),
        ("plan", 0x8ab4f0),
        ("auto", 0xb89cf5),
        ("code", 0xe9c48f),
        ("kw", 0xd49cf0),
        ("str", 0xb5d98a),
        ("num", 0xf3a06a),
        ("fn", 0x8ab4f0),
        ("ty", 0x6fd0bd),
        ("crew0", 0xc2d86a),
        ("crew1", 0x5cc8b4),
        ("crew2", 0xb39cf2),
        ("crew3", 0xdc8cf0),
        ("crew4", 0xf08cb8),
        ("crew5", 0x7cb8f0),
        ("muted", 0xaeaba8),
        ("dim", 0x858282),
        ("faint", 0x514f51),
        ("you", 0x383639),
        ("well", 0x333135),
        ("wellDeep", 0x3c3a3d),
        ("pick", 0x604a3c),
        ("pickBg", 0x4e3c32),
        ("addTint", 0x38423b),
        ("delTint", 0x473436),
        ("addWord", 0x455d4a),
        ("delWord", 0x673f40),
        ("badTint", 0x443235),
        ("amberTint", 0x4e463b),
        ("glowTint", 0x453831),
    ] {
        assert_eq!(DARK.named(name), Some(rgb(expected)), "dark {name}");
    }
}

#[test]
fn terminal_foreground_and_background_reblend_rounded_grounds() {
    let palette = LIGHT.detected(
        Some((0x2020, 0x3030, 0x4040)),
        (0xfafa, 0xf0f0, 0xe0e0),
        true,
    );
    assert_eq!(palette.fg(), rgb(0x203040));
    assert_eq!(palette.bg(), rgb(0xfaf0e0));
    assert_eq!(palette.muted(), mix(palette.fg(), palette.bg(), 0.32));
    assert_eq!(palette.pick(), mix(palette.you(), LIGHT.glow(), 0.17));
    assert_eq!(
        palette.amber_tint(),
        mix(palette.you(), LIGHT.amber_glow(), 0.12)
    );
    assert_eq!(palette.glow(), LIGHT.glow());
}
