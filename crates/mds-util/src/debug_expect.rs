/// A macro that behaves like `.expect("message")` in debug builds,
/// but like `?` in release builds and *all* test builds.
///
/// This is suitable when you want your tests to always propagate errors
/// gracefully, mimicking release behavior, while still getting early
/// panics in your regular debug runs.
///
/// # Usage
///
/// 1. `debug_expect!(expression, "Your custom error message")`
///    - In debug builds: `expression.expect("Your custom error message")`
///    - In release builds: `expression?`
///    - In test builds: `expression?`
///
/// 2. `debug_expect!(expression)`
///    - In debug builds: `expression.expect("debug_expect failed for: expression_string")`
///    - In release builds: `expression?`
///    - In test builds: `expression?`
///
/// # Examples
///
/// ```rust
/// fn might_fail(value: i32) -> Result<i32, String> {
///     if value == 0 { Err("Zero value!".to_string()) } else { Ok(value) }
/// }
///
/// fn process_with_debug_behavior(x: i32) -> Result<i32, String> {
///     // In debug: panics if might_fail(0)
///     // In release: propagates Err
///     // In test: propagates Err
///     let val = debug_expect!(might_fail(x), "Failed to process value");
///     Ok(val * 2)
/// }
///
/// fn main() {
///     // Example usage in main (debug/release behavior)
///     let _ = process_with_debug_behavior(10); // Ok
///     // let _ = process_with_debug_behavior(0); // Panics in debug, propagates in release
/// }
///
/// #[cfg(test)]
/// mod debug_expect_tests {
///     use super::*;
///
///     #[test]
///     fn test_propagates_error_in_test_for_debug_expect_macro() {
///         // This test will pass, as the error is propagated in test builds
///         let result = process_with_debug_behavior(0);
///         assert!(result.is_err());
///         assert_eq!(result, Err("Zero value!".to_string()));
///     }
/// }
/// ```
#[macro_export]
macro_rules! debug_expect {
    // Case 1: With a custom message
    ($expr:expr, $msg:literal) => {
        if cfg!(debug_assertions) {
            $expr.expect($msg)
        } else {
            $expr?
        }
    };
    // Case 2: Without a custom message, using stringify! to provide context
    ($expr:expr) => {
        if cfg!(debug_assertions) {
            $expr.expect(concat!("debug_expect failed for: ", stringify!($expr)))
        } else {
            $expr?
        }
    };
}

/// A macro that behaves like `.expect("message")` in debug builds and *all* test builds,
/// but like `?` in release builds.
///
/// This is suitable when you want your tests to fail loudly and quickly,
/// just like your debug runs, to pinpoint issues immediately.
///
/// # Usage
///
/// 1. `test_expect!(expression, "Your custom error message")`
///    - In debug builds: `expression.expect("Your custom error message")`
///    - In release builds: `expression?`
///    - In test builds: `expression.expect("Your custom error message")`
///
/// 2. `test_expect!(expression)`
///    - In debug builds: `expression.expect("test_expect failed for: expression_string")`
///    - In release builds: `expression?`
///    - In test builds: `expression.expect("test_expect failed for: expression_string")`
///
/// # Examples
///
/// ```rust
/// fn might_fail(value: i32) -> Result<i32, String> {
///     if value == 0 { Err("Zero value!".to_string()) } else { Ok(value) }
/// }
///
/// fn process_with_debug_and_test_behavior(x: i32) -> Result<i32, String> {
///     // In debug: panics if might_fail(0)
///     // In release: propagates Err
///     // In test: panics if might_fail(0)
///     let val = test_expect!(might_fail(x), "Failed to process value");
///     Ok(val * 2)
/// }
///
/// fn main() {
///     // Example usage in main (debug/release behavior)
///     let _ = process_with_debug_and_test_behavior(10); // Ok
///     // let _ = process_with_debug_and_test_behavior(0); // Panics in debug, propagates in release
/// }
///
/// #[cfg(test)]
/// mod test_expect_tests {
///     use super::*;
///
///     #[test]
///     #[should_panic(expected = "Failed to process value")]
///     fn test_panics_in_test_for_test_expect_macro() {
///         // This test will panic, as the macro is configured to panic in test builds
///         let _ = process_with_debug_and_test_behavior(0);
///     }
/// }
/// ```
#[macro_export]
macro_rules! test_expect {
    // Case 1: With a custom message
    ($expr:expr, $msg:literal) => {
        if cfg!(debug_assertions) && cfg!(test) {
            $expr.expect($msg)
        } else {
            $expr?
        }
    };
    // Case 2: Without a custom message, using stringify! to provide context
    ($expr:expr) => {
        if cfg!(debug_assertions) && cfg!(test) {
            $expr.expect(concat!("test_expect failed for: ", stringify!($expr)))
        } else {
            $expr?
        }
    };
}
