/// Strips the trailing dot from an absolute DNS name in presentation format,
/// so that "hostname.local" and "hostname.local." refer to the same name.
///
/// Preserves the original letter case, making it suitable for display.
pub fn strip_trailing_dot(host: &str) -> &str {
    host.strip_suffix('.').unwrap_or(host)
}

/// Normalizes a hostname into a canonical form for comparison.
///
/// RFC 6762 Section 16 requires Multicast DNS names to be compared
/// case-insensitively for ASCII letters (non-ASCII bytes are compared
/// verbatim, so Unicode case folding must not be applied). This strips a
/// single trailing dot and lowercases ASCII letters.
///
/// The result is only meant for matching; keep the original spelling around
/// for display.
///
/// Link: https://datatracker.ietf.org/doc/html/rfc6762#section-16
pub fn normalize_hostname(host: impl AsRef<str>) -> String {
    strip_trailing_dot(host.as_ref()).to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_trailing_dot_removes_single_dot() {
        assert_eq!(strip_trailing_dot("myhost.local."), "myhost.local");
        assert_eq!(strip_trailing_dot("myhost.local"), "myhost.local");
    }

    #[test]
    fn strip_trailing_dot_preserves_case() {
        assert_eq!(strip_trailing_dot("MyHost.Local."), "MyHost.Local");
    }

    #[test]
    fn normalize_hostname_is_case_insensitive() {
        assert_eq!(
            normalize_hostname("MyHost.Local."),
            normalize_hostname("myhost.local")
        );
    }

    #[test]
    fn normalize_hostname_does_not_fold_non_ascii() {
        // RFC 6762 case-insensitivity only covers ASCII letters; non-ASCII
        // bytes must be compared verbatim.
        assert_ne!(normalize_hostname("café"), normalize_hostname("CAFÉ"));
        assert_eq!(normalize_hostname("CAFé"), "café");
    }
}
