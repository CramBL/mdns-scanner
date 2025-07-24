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

// TODO: Optimize this
fn inner_unescape_dns_name(dns_name: impl AsRef<str>) -> String {
    let mut chars = dns_name.as_ref().chars().peekable();
    let mut unescaped_bytes: Vec<u8> = Vec::with_capacity(dns_name.as_ref().len());

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next_char) = chars.peek() {
                if next_char.is_ascii_digit() {
                    let mut octal_digits = String::with_capacity(3);
                    let mut successful_parse = false;

                    for _ in 0..3 {
                        if let Some(&digit_char) = chars.peek() {
                            if digit_char.is_digit(8) {
                                octal_digits.push(chars.next().unwrap());
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    if octal_digits.len() == 3 {
                        if let Ok(byte) = u8::from_str_radix(&octal_digits, 8) {
                            unescaped_bytes.push(byte);
                            successful_parse = true;
                        }
                    }

                    if !successful_parse {
                        unescaped_bytes.push(b'\\');
                        unescaped_bytes.extend_from_slice(octal_digits.as_bytes());
                    }
                } else {
                    unescaped_bytes.push(chars.next().unwrap() as u8);
                }
            } else {
                unescaped_bytes.push(b'\\');
            }
        } else {
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
        let unescaped = inner_unescape_dns_name("Music\\040Player\\040@\\040rpi5._mpd._tcp.local.");
        assert_eq!(unescaped, "Music Player @ rpi5._mpd._tcp.local.");
    }

    #[test]
    fn test_other_octal_values() {
        let unescaped = inner_unescape_dns_name("a\\050b.local.");
        assert_eq!(unescaped, "a(b.local.");
    }

    #[test]
    fn test_standard_character_escapes() {
        let unescaped = inner_unescape_dns_name("special\\.\\@chars\\.test.");
        assert_eq!(unescaped, "special.@chars.test.");
    }

    #[test]
    fn test_invalid_octal_sequence() {
        // \078 is invalid because 8 is not an octal digit. Should be treated literally.
        assert_eq!(
            inner_unescape_dns_name("invalid\\078sequence.com."),
            "invalid\\078sequence.com."
        );
    }

    #[test]
    fn test_non_ascii_utf8_character_via_octal() {
        // The copyright symbol © is UTF-8 bytes 195, 169.
        // Decimal 195 is octal 303.
        // Decimal 169 is octal 251.
        let name = "my\\302\\251site.com.";
        let unescaped = inner_unescape_dns_name(name);
        assert_eq!(unescaped, "my©site.com.");
    }
}
