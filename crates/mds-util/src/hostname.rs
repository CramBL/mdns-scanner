/// A DNS name as it appeared on the wire.
///
/// Multicast DNS names match case-insensitively for ASCII letters ignoring a
/// single trailing dot (RFC 6762 section 16), but the advertised spelling is
/// what users should see. To keep those two concerns apart this type
/// deliberately implements neither comparison traits nor Display: compare
/// names with [`matches`], use [`canonical`] for map keys and messages about
/// DNS semantics, and [`display_name`] to render the name to a user.
///
/// https://datatracker.ietf.org/doc/html/rfc6762#section-16
#[derive(Debug, Clone)]
pub struct DnsName {
    /// The name exactly as advertised on the network
    advertised: String,
    /// Only populated when lowercasing changed at least one byte; otherwise
    /// the canonical form is the trailing-dot-stripped prefix of `advertised`
    /// and no second allocation is needed
    canonical: Option<String>,
}

impl DnsName {
    pub fn new(name: impl Into<String>) -> Self {
        let advertised = name.into();
        let stripped = strip_trailing_dot(&advertised);
        let canonical = stripped
            .bytes()
            .any(|b| b.is_ascii_uppercase())
            .then(|| stripped.to_ascii_lowercase());
        Self {
            advertised,
            canonical,
        }
    }

    /// RFC 6762 section 16 comparison: ASCII-case-insensitive, a single
    /// trailing dot ignored. Non-ASCII bytes are compared verbatim, so
    /// Unicode case folding is deliberately not applied.
    pub fn matches(&self, other: &Self) -> bool {
        self.canonical() == other.canonical()
    }

    /// Like [`Self::matches`] for a name that hasn't been wrapped yet
    pub fn matches_str(&self, other: &str) -> bool {
        self.canonical()
            .eq_ignore_ascii_case(strip_trailing_dot(other))
    }

    /// The canonical form: trailing dot stripped, ASCII letters lowercased.
    /// Meant for matching, map keys, and messages where the DNS semantics
    /// matter.
    pub fn canonical(&self) -> &str {
        match &self.canonical {
            Some(canonical) => canonical,
            None => strip_trailing_dot(&self.advertised),
        }
    }

    /// The advertised spelling without the trailing dot (user-facing)
    pub fn display_name(&self) -> &str {
        strip_trailing_dot(&self.advertised)
    }

    /// The name exactly as it appeared on the wire, trailing dot included
    pub fn as_advertised(&self) -> &str {
        &self.advertised
    }
}

/// Strips the trailing dot from an absolute DNS name in presentation format,
/// so that "hostname.local" and "hostname.local." refer to the same name.
///
/// Preserves the original letter case, making it suitable for display.
fn strip_trailing_dot(host: &str) -> &str {
    host.strip_suffix('.').unwrap_or(host)
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
    fn matches_is_case_insensitive_and_ignores_trailing_dot() {
        let a = DnsName::new("MyHost.Local.");
        let b = DnsName::new("myhost.local");
        assert!(a.matches(&b));
        assert!(b.matches(&a));
        assert!(a.matches_str("MYHOST.LOCAL."));
    }

    #[test]
    fn matches_distinguishes_different_names() {
        let a = DnsName::new("myhost.local");
        let b = DnsName::new("otherhost.local");
        assert!(!a.matches(&b));
        assert!(!a.matches_str("otherhost.local"));
    }

    #[test]
    fn matches_does_not_fold_non_ascii() {
        // RFC 6762 case-insensitivity only covers ASCII letters; non-ASCII
        // bytes must be compared verbatim.
        assert!(!DnsName::new("café.local").matches(&DnsName::new("CAFÉ.local")));
        assert!(DnsName::new("CAFé.local").matches(&DnsName::new("café.LOCAL")));
    }

    #[test]
    fn canonical_strips_and_lowercases() {
        assert_eq!(DnsName::new("MyHost.Local.").canonical(), "myhost.local");
        // Already-lowercase names reuse the advertised allocation
        assert_eq!(DnsName::new("myhost.local.").canonical(), "myhost.local");
    }

    #[test]
    fn display_name_preserves_case_and_strips_dot() {
        let name = DnsName::new("MyHost.Local.");
        assert_eq!(name.display_name(), "MyHost.Local");
        assert_eq!(name.as_advertised(), "MyHost.Local.");
    }
}
