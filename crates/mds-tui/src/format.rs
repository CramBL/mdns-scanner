use std::time::Duration;

/// Formats a measured or estimated span as e.g. `420ms`, `4.2s`, `1m 40s`, `2h 3m`.
pub(crate) fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs >= 3600 {
        format!("{h}h {m}m", h = secs / 3600, m = (secs % 3600) / 60)
    } else if secs >= 60 {
        format!("{m}m {s}s", m = secs / 60, s = secs % 60)
    } else if secs >= 1 {
        format!("{:.1}s", duration.as_secs_f32())
    } else {
        format!("{}ms", duration.as_millis())
    }
}

/// Formats how long ago something happened, in a single unit: `12s`, `3m`, `2h`.
pub(crate) fn format_age(age: Duration) -> String {
    let secs = age.as_secs();
    if secs >= 3600 {
        format!("{}h", secs / 3600)
    } else if secs >= 60 {
        format!("{}m", secs / 60)
    } else {
        format!("{secs}s")
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(Duration::from_millis(420), "420ms")]
    #[case(Duration::from_millis(4200), "4.2s")]
    #[case(Duration::from_secs(100), "1m 40s")]
    #[case(Duration::from_secs(7380), "2h 3m")]
    fn duration_formatting(#[case] duration: Duration, #[case] expected: &str) {
        assert_eq!(format_duration(duration), expected);
    }

    #[rstest]
    #[case(Duration::from_secs(12), "12s")]
    #[case(Duration::from_secs(190), "3m")]
    #[case(Duration::from_secs(7380), "2h")]
    fn age_formatting(#[case] age: Duration, #[case] expected: &str) {
        assert_eq!(format_age(age), expected);
    }
}
