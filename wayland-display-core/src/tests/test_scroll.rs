//! Tests for the two scroll sources: notched wheel (`MouseAxis` ->
//! `AxisSource::Wheel`) and continuous/touchpad (`MouseAxisSmooth` ->
//! `AxisSource::Finger`).
//!
//! Clients tell the two apart by the `wl_pointer.axis_source` a frame carries,
//! and by whether it is accompanied by `axis_value120` discrete steps. Asserting
//! on the wire events is the only way to prove a smooth-scroll event does not
//! reach the client as a wheel notch.

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
            axis_source.clone().into_result().ok()
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

fn has_horizontal_axis_stop(f: &mut Fixture) -> bool {
    f.client.get_client_events().iter().any(|e| {
        matches!(
            e,
            MouseEvents::Pointer(wl_pointer::Event::AxisStop {
                axis: wayland_client::WEnum::Value(wl_pointer::Axis::HorizontalScroll),
                ..
            })
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

#[test]
fn smooth_scroll_is_reported_as_finger_without_discrete_steps() {
    let mut f = Fixture::new();
    focus(&mut f);

    // What Command::PointerAxisSmooth drives: continuous pixel deltas, no v120.
    f.server
        .pointer_axis(0, AxisSource::Finger, 0.0, 12.5, None, None);
    f.round_trip();

    let events = dump(&mut f);
    assert_eq!(
        axis_source_of(&mut f),
        Some(wl_pointer::AxisSource::Finger),
        "MouseAxisSmooth must reach the client as axis_source=finger, got {events}",
    );
    assert!(
        !has_value120(&mut f),
        "a finger frame must NOT carry axis_value120 - that is what makes it \
         distinguishable from a wheel notch. Got {events}",
    );
}

#[test]
fn finger_scroll_emits_axis_stop_on_the_idle_axis() {
    let mut f = Fixture::new();
    focus(&mut f);

    // Vertical-only smooth scroll: the horizontal axis must be explicitly stopped,
    // which is how a client ends kinetic scrolling on that axis.
    f.server
        .pointer_axis(0, AxisSource::Finger, 0.0, 8.0, None, None);
    f.round_trip();

    let events = dump(&mut f);
    assert!(
        has_horizontal_axis_stop(&mut f),
        "a finger frame with no horizontal motion must emit axis_stop for the \
         horizontal axis, got {events}",
    );
}
