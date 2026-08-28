//! Tests for `Command::SetKeymap` / the `xkb-layout`, `xkb-variant` and
//! `xkb-options` element properties.
//!
//! The seat keyboard is created with `XkbConfig::default()`, which libxkbcommon
//! resolves to the US layout. A nested compositor (Aquamarine's Wayland backend,
//! and hence Hyprland) derives its modifier state from the events THIS seat
//! sends, so the parent's keymap is one half of any keymap change.
//!
//! Asserting on the compositor's own xkb state rather than on a log line: the
//! `Loaded Keymap` trace is only emitted by `add_keyboard`, not by
//! `set_xkb_config`, so a log-based check cannot tell a real recompile from a
//! no-op.

use crate::KeymapConfig;
use crate::tests::fixture::Fixture;
use smithay::input::keyboard::XkbConfig;
use test_log::test;

/// Name of the layout the seat keyboard is currently using.
fn active_layout_name(f: &mut Fixture) -> String {
    let keyboard = f.server.seat.get_keyboard().expect("seat has a keyboard");
    keyboard.with_xkb_state(&mut f.server, |ctx| {
        let xkb = ctx.xkb().lock().unwrap();
        let layout = xkb.active_layout();
        xkb.layout_name(layout).to_string()
    })
}

fn set_keymap(f: &mut Fixture, keymap: KeymapConfig) {
    let keyboard = f.server.seat.get_keyboard().expect("seat has a keyboard");
    let config = XkbConfig {
        rules: &keymap.rules,
        model: &keymap.model,
        layout: &keymap.layout,
        variant: &keymap.variant,
        options: keymap.options.clone(),
    };
    keyboard
        .set_xkb_config(&mut f.server, config)
        .expect("set_xkb_config should accept a valid keymap");
}

#[test]
fn default_seat_keyboard_is_us() {
    let mut f = Fixture::new();
    assert_eq!(
        active_layout_name(&mut f),
        "English (US)",
        "XkbConfig::default() should resolve to the US layout",
    );
}

#[test]
fn set_keymap_switches_the_seat_layout() {
    let mut f = Fixture::new();
    assert_eq!(active_layout_name(&mut f), "English (US)");

    set_keymap(
        &mut f,
        KeymapConfig {
            layout: "de".into(),
            ..Default::default()
        },
    );

    assert_eq!(
        active_layout_name(&mut f),
        "German",
        "xkb-layout=de must recompile the seat keymap to German",
    );
}

#[test]
fn set_keymap_applies_variant_and_options() {
    let mut f = Fixture::new();

    set_keymap(
        &mut f,
        KeymapConfig {
            layout: "de".into(),
            variant: "nodeadkeys".into(),
            options: Some("compose:ralt".into()),
            ..Default::default()
        },
    );

    // libxkbcommon names the de/nodeadkeys layout "German (no dead keys)".
    let name = active_layout_name(&mut f);
    assert!(
        name.starts_with("German"),
        "expected a German layout, got {name:?}",
    );
    assert!(
        name.contains("dead"),
        "xkb-variant=nodeadkeys must reach the compiled keymap, got {name:?}",
    );
}

#[test]
fn switching_back_to_default_restores_us() {
    let mut f = Fixture::new();

    set_keymap(
        &mut f,
        KeymapConfig {
            layout: "de".into(),
            ..Default::default()
        },
    );
    assert_eq!(active_layout_name(&mut f), "German");

    set_keymap(&mut f, KeymapConfig::default());
    assert_eq!(
        active_layout_name(&mut f),
        "English (US)",
        "an empty KeymapConfig must fall back to the libxkbcommon default",
    );
}
