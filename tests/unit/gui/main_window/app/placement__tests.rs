use super::*;

fn portrait_circle() -> ScreenCircle {
    ScreenCircle {
        monitor_index: 1,
        monitor: (1920, 0, 1080, 1920),
        centre: (1920 + 540, 1920 - 540),
    }
}

#[test]
fn on_the_circles_monitor_the_window_is_centred_on_the_circle() {
    let (x, y) = on_circle((1000, 700), 1, Some(portrait_circle())).unwrap();
    assert_eq!((x + 500, y + 350), portrait_circle().centre);
}

#[test]
fn a_window_that_would_leave_the_monitor_moves_only_as_far_as_needed() {
    let (x, y) = on_circle((1000, 1200), 1, Some(portrait_circle())).unwrap();
    assert_eq!(x + 500, portrait_circle().centre.0);
    assert_eq!(y + 1200, 1920);

    let (x, _) = on_circle((1400, 700), 1, Some(portrait_circle())).unwrap();
    assert_eq!(x, 1920);
}

#[test]
fn on_another_monitor_or_without_a_circle_the_window_is_placed_as_before() {
    assert_eq!(on_circle((1000, 700), 0, Some(portrait_circle())), None);
    assert_eq!(on_circle((1000, 700), 1, None), None);
}
