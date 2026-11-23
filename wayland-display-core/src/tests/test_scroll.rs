//! Test for the ONE scroll source this element has after the fork features go:
//! the notched wheel (`GstNavigation` `MouseScroll` -> `MouseAxis` -> the seat's
//! `AxisSource::Wheel`).
//!
//! Clients tell sources apart by the `wl_pointer.axis_source` a frame carries and by
//! whether it is accompanied by `axis_value120` discrete steps. Asserting on the wire
//! events is the only in-repo proof that a wheel notch reaches a client as a WHEEL —
//! which is the downstream the `MouseScroll` arm in `waylandsrc/imp.rs` feeds. (The
//! fork's continuous `MouseAxisSmooth` source, and its own two cases, are deleted in
//! the same cutover that removed the command; only this case survives it.)

use crate::tests::client::MouseEvents;
use crate::tests::fixture::Fixture;
use smithay::backend::input::AxisSource;
use smithay::utils::Point;
use test_log::test;
use wayland_client::protocol::wl_pointer;

/// Put the pointer in the surface so axis events are delivered, then drop the
/// enter/motion traffic.
fn focus(f: &mut Fixture) {
    f.create_window(320, 240);
    f.server
        .pointer_motion_absolute(0, Point::from((10.0, 10.0)));
    f.round_trip();
    f.client.get_client_events().clear();
}

/// `wl_pointer::Event` is not `Clone`, so every helper inspects the recorded
/// events in place rather than collecting them.
fn dump(f: &mut Fixture) -> String {
    format!("{:?}", f.client.get_client_events())
}

fn axis_source_of(f: &mut Fixture) -> Option<wl_pointer::AxisSource> {
    f.client.get_client_events().iter().find_map(|e| match e {
        MouseEvents::Pointer(wl_pointer::Event::AxisSource { axis_source }) => {
            (*axis_source).into_result().ok()
        }
        _ => None,
    })
}

fn has_value120(f: &mut Fixture) -> bool {
    f.client.get_client_events().iter().any(|e| {
        matches!(
            e,
            MouseEvents::Pointer(wl_pointer::Event::AxisValue120 { .. })
        )
    })
}

#[test]
fn wheel_scroll_is_reported_as_wheel_with_discrete_steps() {
    let mut f = Fixture::new();
    focus(&mut f);

    f.server
        .pointer_axis(0, AxisSource::Wheel, 0.0, 3.0, None, Some(120.0));
    f.round_trip();

    let events = dump(&mut f);
    assert_eq!(
        axis_source_of(&mut f),
        Some(wl_pointer::AxisSource::Wheel),
        "MouseAxis must reach the client as axis_source=wheel, got {events}",
    );
    assert!(
        has_value120(&mut f),
        "a wheel frame must carry axis_value120 discrete steps, got {events}",
    );
}
