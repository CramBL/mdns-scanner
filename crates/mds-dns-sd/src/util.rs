use hickory_proto::rr::Name;

/// Unescapes a DNS name's string representation as specified by <https://datatracker.ietf.org/doc/html/rfc1035#section-5.1>
///
/// It handles two types of escape sequences:
///
/// 1.  **Byte-value escapes (`\OOO`):** A backslash followed by three *octal* digits
///     (0-7) represents a single byte. For example, `\040` is a space (32 decimal).
///
/// 2.  **Character escapes (`\X`):** A backslash followed by any non-digit character
///     simply represents that literal character (e.g., `\.` becomes `.`).
///
/// The resulting byte sequence is decoded into a UTF-8 `String`, replacing invalid
/// sequences with the Unicode replacement character `�` (U+FFFD).
///
/// # Returns
///
/// A `String` containing the unescaped and decoded name.
pub fn unescape_dns_name_to_string(name: &Name) -> String {
    let escaped_string = name.to_utf8();
    inner_unescape_dns_name(escaped_string)
}

fn inner_unescape_dns_name(dns_name: String) -> String {
    if !dns_name.contains('\\') {
        return dns_name;
    }

    let mut unescaped_bytes: Vec<u8> = Vec::with_capacity(dns_name.len());
    let mut chars = dns_name.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next_char) = chars.peek() {
                if next_char.is_digit(8) {
                    let mut octal_digits = String::with_capacity(3);
                    for _ in 0..3 {
                        if let Some(digit_char) = chars.peek()
                            && digit_char.is_digit(8)
                        {
                            octal_digits.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }

                    // Attempt to parse only if we have exactly three digits.
                    if octal_digits.len() == 3 {
                        if let Ok(byte) = u8::from_str_radix(&octal_digits, 8) {
                            unescaped_bytes.push(byte);
                        } else {
                            // This branch is technically unreachable if all are octal digits
                            // and the string length is 3, but it's safe to keep.
                            unescaped_bytes.push(b'\\');
                            unescaped_bytes.extend_from_slice(octal_digits.as_bytes());
                        }
                    } else {
                        // Not a full 3-digit octal, so treat as literal characters.
                        unescaped_bytes.push(b'\\');
                        unescaped_bytes.extend_from_slice(octal_digits.as_bytes());
                    }
                } else {
                    // Handle `\X` escapes for ALL characters, including
                    // multi-byte UTF-8 ones.
                    let escaped_char = chars.next().unwrap();
                    let mut buf = [0; 4];
                    let encoded_bytes = escaped_char.encode_utf8(&mut buf).as_bytes();
                    unescaped_bytes.extend_from_slice(encoded_bytes);
                }
            } else {
                // A trailing backslash at the end of the string.
                unescaped_bytes.push(b'\\');
            }
        } else {
            // A regular, non-escaped character.
            let mut buf = [0; 4];
            let encoded_bytes = c.encode_utf8(&mut buf).as_bytes();
            unescaped_bytes.extend_from_slice(encoded_bytes);
        }
    }

    String::from_utf8_lossy(&unescaped_bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_octal_escapes_for_spaces_and_special_chars() {
        let unescaped =
            inner_unescape_dns_name("Music\\040Player\\040@\\040rpi5._mpd._tcp.local.".into());
        assert_eq!(unescaped, "Music Player @ rpi5._mpd._tcp.local.");
    }

    #[test]
    fn test_other_octal_values() {
        let unescaped = inner_unescape_dns_name("a\\050b.local.".into());
        assert_eq!(unescaped, "a(b.local.");
    }

    #[test]
    fn test_standard_character_escapes() {
        let unescaped = inner_unescape_dns_name("special\\.\\@chars\\.test.".into());
        assert_eq!(unescaped, "special.@chars.test.");
    }

    #[test]
    fn test_invalid_octal_sequence() {
        // \078 is invalid because 8 is not an octal digit. Should be treated literally.
        assert_eq!(
            inner_unescape_dns_name("invalid\\078sequence.com.".into()),
            "invalid\\078sequence.com."
        );
    }

    #[test]
    fn test_non_ascii_utf8_character_via_octal() {
        // The copyright symbol © is UTF-8 bytes 195, 169.
        // Decimal 195 is octal 303.
        // Decimal 169 is octal 251.
        let name = "my\\302\\251site.com.";
        let unescaped = inner_unescape_dns_name(name.to_owned());
        assert_eq!(unescaped, "my©site.com.");
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    /// A simple, clear reference implementation to test against.
    /// This "oracle" is designed for correctness over performance.
    fn oracle_unescape(dns_name: &str) -> String {
        let mut unescaped_bytes: Vec<u8> = Vec::new();
        let mut chars = dns_name.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '\\' {
                if let Some(d1) = chars.peek() {
                    // Check for a 3-digit octal sequence
                    if d1.is_digit(8) {
                        let mut octal_str = String::new();
                        octal_str.push(chars.next().unwrap()); // Consume d1

                        if let Some(_d2) = chars.peek().filter(|c| c.is_digit(8)) {
                            octal_str.push(chars.next().unwrap()); // Consume d2
                            if let Some(_d3) = chars.peek().filter(|c| c.is_digit(8)) {
                                octal_str.push(chars.next().unwrap()); // Consume d3
                            }
                        }

                        // If we got 3 octal digits, parse them.
                        if octal_str.len() == 3
                            && let Ok(byte) = u8::from_str_radix(&octal_str, 8)
                        {
                            unescaped_bytes.push(byte);
                            continue;
                        }

                        // If it wasn't a valid 3-digit octal, it's a literal backslash
                        // followed by the digits we consumed.
                        unescaped_bytes.push(b'\\');
                        unescaped_bytes.extend_from_slice(octal_str.as_bytes());
                    } else {
                        // It's a `\X` escape, where X is not an octal digit.
                        // Correctly encode the character X to its UTF-8 bytes.
                        let next_c = chars.next().unwrap();
                        let mut buf = [0; 4];
                        unescaped_bytes.extend_from_slice(next_c.encode_utf8(&mut buf).as_bytes());
                    }
                } else {
                    // Trailing backslash
                    unescaped_bytes.push(b'\\');
                }
            } else {
                // Not an escape, just a regular character
                let mut buf = [0; 4];
                unescaped_bytes.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
        }
        String::from_utf8_lossy(&unescaped_bytes).into_owned()
    }

    // === Strategy Generators ===

    fn dns_label_chars() -> impl Strategy<Value = String> {
        prop::collection::vec(
            prop_oneof![
                prop::char::range('a', 'z'),
                prop::char::range('A', 'Z'),
                prop::char::range('0', '9'),
                Just('-'),
            ],
            1..=63,
        )
        .prop_map(|chars| chars.into_iter().collect())
    }

    fn valid_octal_escape() -> impl Strategy<Value = String> {
        (0u8..=255).prop_map(|byte| format!("\\{byte:03o}"))
    }

    fn invalid_octal_escape() -> impl Strategy<Value = String> {
        prop_oneof![
            // Two digits only
            "[0-7][0-7]".prop_map(|s| format!("\\{s}")),
            // One digit only
            "[0-7]".prop_map(|s| format!("\\{s}")),
            // Invalid digit in sequence
            "[0-3][0-7][8-9]".prop_map(|s| format!("\\{s}"))
        ]
    }

    fn character_escape() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("\\.".to_string()),
            Just("\\@".to_string()),
            Just("\\\\".to_string()),
            Just("\\ ".to_string()),
            // Unicode characters
            prop::char::range('\u{0080}', '\u{00FF}').prop_map(|c| format!("\\{c}")),
        ]
    }

    fn any_escape() -> impl Strategy<Value = String> {
        prop_oneof![
            valid_octal_escape(),
            invalid_octal_escape(),
            character_escape(),
        ]
    }

    // === Core Property Tests ===

    proptest! {
        /// Main fuzzing test - ensures implementation matches oracle on all inputs
        #[test]
        fn matches_oracle_on_arbitrary_input(
            s in r"([a-zA-Z0-9._\p{L}]|\\(\\|[0-3][0-7][0-7]|[^\d\s])|\\[0-7]{1,2}){0,50}"
        ) {
            let result = inner_unescape_dns_name(s.clone());
            let oracle_result = oracle_unescape(&s);

            prop_assert_eq!(
                result, oracle_result,
                "Implementation diverged from oracle for input: {:?}", s
            );
        }

        /// Test specific escape sequence types in isolation
        #[test]
        fn escape_sequences_work_correctly(
            prefix in "[a-zA-Z0-9._]{0,10}",
            escape in any_escape(),
            suffix in "[a-zA-Z0-9._]{0,10}"
        ) {
            let input = format!("{prefix}{escape}{suffix}");
            let result = inner_unescape_dns_name(input.clone());
            let oracle = oracle_unescape(&input);

            prop_assert_eq!(&result, &oracle);

            // Valid 3-digit octal escapes should compress the string
            if escape.len() == 4 && escape.starts_with('\\') &&
               escape.chars().skip(1).all(|c| c.is_digit(8)) {
                prop_assert!(result.len() < input.len(),
                    "Valid octal escape should compress string length");
            }
        }

        /// Test sequences with multiple escapes
        #[test]
        fn multiple_escapes_handled_correctly(
            escapes in prop::collection::vec(any_escape(), 1..=10)
        ) {
            let input = escapes.join("");
            let result = inner_unescape_dns_name(input.clone());
            let oracle = oracle_unescape(&input);

            prop_assert_eq!(result, oracle);
        }

        /// Test that unescaped strings remain unchanged (idempotency)
        #[test]
        fn unescaped_strings_unchanged(
            s in r"[a-zA-Z0-9._\p{L}&&[^\\]]{0,50}"
        ) {
            let result = inner_unescape_dns_name(s.clone());
            prop_assert_eq!(result, s, "Unescaped string should remain unchanged");
        }

        /// Test UTF-8 sequences encoded as octal escapes
        #[test]
        fn utf8_via_octal_escapes(
            utf8_bytes in prop::collection::vec(0u8..=255, 1..=4)
                .prop_filter("Valid UTF-8 only", |bytes| {
                    std::str::from_utf8(bytes).is_ok()
                })
        ) {
            let octal_input: String = utf8_bytes
                .into_iter()
                .map(|byte| format!("\\{byte:03o}"))
                .collect();

            let result = inner_unescape_dns_name(octal_input.clone());
            let oracle = oracle_unescape(&octal_input);

            prop_assert_eq!(&result, &oracle);
            // Ensure no replacement characters for valid UTF-8 input
            prop_assert!(!result.contains('\u{FFFD}'),
                "Valid UTF-8 should not contain replacement characters");
        }

        /// Test boundary conditions and special cases
        #[test]
        fn boundary_conditions(
            prefix in "[a-zA-Z0-9._]{0,10}",
            test_case in prop_oneof![
                // Boundary octal values
                Just("\\000"), Just("\\377"), Just("\\177"), Just("\\040"), Just("\\134"),
                // Trailing backslashes
                Just("\\"), Just("\\\\"), Just("\\\\\\"),
                // Edge cases
                Just("\\8"), Just("\\9"), Just("\\07"), Just("\\377\\000"),
            ],
            suffix in "[a-zA-Z0-9._]{0,10}"
        ) {
            let input = format!("{prefix}{test_case}{suffix}");
            let result = inner_unescape_dns_name(input.clone());
            let oracle = oracle_unescape(&input);

            prop_assert_eq!(result, oracle);
        }

        /// Test that DNS structure is preserved (dots remain dots)
        #[test]
        fn preserves_dns_structure(
            labels in prop::collection::vec(dns_label_chars(), 1..=5)
        ) {
            let dns_name = labels.join(".");
            let unescaped = inner_unescape_dns_name(dns_name.clone());

            prop_assert_eq!(
                dns_name.matches('.').count(),
                unescaped.matches('.').count(),
                "Number of dots should be preserved"
            );
        }

        /// Test output properties and invariants
        #[test]
        fn output_properties(
            s in r"([a-zA-Z0-9._]|\\[0-3][0-7][0-7]|\\[^0-7]){0,100}"
        ) {
            let result = inner_unescape_dns_name(s.clone());

            // Output should never be longer than input
            prop_assert!(result.len() <= s.len(),
                "Output should never exceed input length");

            // Output should always be valid UTF-8 (test by iteration)
            let char_count = result.chars().count();
            prop_assert!(char_count <= result.len(),
                "Character count should be valid for UTF-8");
        }
    }
}
