use super::*;

fn window(process: u32) -> Candidate {
    Candidate {
        process,
        visible: true,
        owned: false,
        console: false,
    }
}

#[test]
fn the_target_is_a_visible_unowned_window_of_the_started_processes() {
    let processes: HashSet<u32> = [10, 11].into_iter().collect();
    assert!(is_target(window(10), &processes));

    assert!(is_target(window(11), &processes));

    assert!(!is_target(window(12), &processes));
}

#[test]
fn hidden_owned_and_console_windows_are_passed_over() {
    let processes: HashSet<u32> = [10].into_iter().collect();
    assert!(!is_target(
        Candidate {
            visible: false,
            ..window(10)
        },
        &processes
    ));

    assert!(!is_target(
        Candidate {
            owned: true,
            ..window(10)
        },
        &processes
    ));

    assert!(!is_target(
        Candidate {
            console: true,
            ..window(10)
        },
        &processes
    ));
}

#[test]
fn a_target_has_fifteen_seconds_to_show_its_window() {
    assert_eq!(WINDOW_LIMIT, Duration::from_secs(15));
}

#[test]
fn only_the_main_window_title_is_the_main_window() {
    assert!(is_main_title("GameToaster"));
    assert!(!is_main_title("GameToaster Controller Launcher"));
    assert!(!is_main_title(""));
}
