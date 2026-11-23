//! Focus hand-back when a toplevel goes away.
//!
//! The parent compositor can briefly carry a second toplevel beside the nested desktop (a
//! probe window, a dialog, a short-lived client). When that toplevel closed, keyboard and
//! pointer focus stayed on the dead window and the remaining toplevel never got either
//! back: measured on cstream with KWin nested, the stream's keys and pointer then reached
//! nothing until another window mapped. These tests pin the hand-back.

use crate::tests::client::MouseEvents;
use crate::tests::fixture::Fixture;
use smithay::utils::Point;
use test_log::test;
use wayland_client::Proxy;
use wayland_client::protocol::{wl_keyboard, wl_pointer};

#[test]
fn closing_the_focused_window_returns_keyboard_and_pointer_focus_to_the_one_below() {
    let mut f = Fixture::new();
    f.round_trip();
    f.create_window(320, 240); // A: the desktop that stays
    f.create_window(320, 240); // B: the short-lived window that takes focus
    let a = f.client.window_surface(0).id();
    let b = f.client.window_surface(1).id();

    // Settle the pointer inside both windows (both are mapped at the origin).
    f.server
        .pointer_motion_absolute(0, Point::from((10.0, 12.0)));
    f.round_trip();

    // Precondition: B, mapped last, holds keyboard focus.
    let last_enter = f
        .client
        .get_keyboard_events()
        .iter()
        .rev()
        .find_map(|e| match e {
            wl_keyboard::Event::Enter { surface, .. } => Some(surface.id()),
            _ => None,
        });
    assert_eq!(
        last_enter,
        Some(b.clone()),
        "precondition: B holds keyboard focus"
    );
    f.client.get_keyboard_events().clear();
    f.client.get_client_events().clear();

    f.client.destroy_window_at(1);
    f.round_trip();
    f.round_trip();

    let entered_keyboard = f
        .client
        .get_keyboard_events()
        .iter()
        .any(|e| matches!(e, wl_keyboard::Event::Enter { surface, .. } if surface.id() == a));
    assert!(
        entered_keyboard,
        "keyboard focus must return to A, got: {:?}",
        f.client.get_keyboard_events()
    );

    let pointer: Vec<&wl_pointer::Event> = f
        .client
        .get_client_events()
        .iter()
        .filter_map(|e| match e {
            MouseEvents::Pointer(p) => Some(p),
            _ => None,
        })
        .filter(|p| !matches!(p, wl_pointer::Event::Frame))
        .collect();
    let enter = pointer
        .iter()
        .position(|p| matches!(p, wl_pointer::Event::Enter { surface, .. } if surface.id() == a));
    let Some(enter) = enter else {
        panic!("pointer focus must return to A, got: {pointer:?}");
    };
    let wl_pointer::Event::Enter {
        surface_x,
        surface_y,
        ..
    } = pointer[enter]
    else {
        unreachable!()
    };
    // The enter is followed by a plain motion at the same position (see
    // `follows_every_enter_with_a_motion` in test_pointer.rs for why).
    assert!(
        matches!(pointer.get(enter + 1), Some(wl_pointer::Event::Motion { surface_x: x, surface_y: y, .. }) if x == surface_x && y == surface_y),
        "the enter onto A must be followed by a motion at ({surface_x}, {surface_y}), got: {pointer:?}"
    );
}
