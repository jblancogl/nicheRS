//! Java `Double.toString` / `Float.toString` formatting (JDK 19+ shortest-repr semantics).
//!
//! Needed so that `.lambdas` files and feature names (e.g. `(12.5<temp)`, `(landcover=1.0)`)
//! are byte-compatible with MaxEnt 3.4.4 output.

/// Format like Java's `Double.toString(double)`.
pub fn double_to_string(x: f64) -> String {
    format_java(x, format!("{:e}", x))
}

/// Format like Java's `Float.toString(float)`.
pub fn float_to_string(x: f32) -> String {
    format_java(f64::from(x), format!("{:e}", x))
}

fn format_java(x: f64, sci: String) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x.is_infinite() {
        return if x > 0.0 {
            "Infinity".into()
        } else {
            "-Infinity".into()
        };
    }
    if x == 0.0 {
        return if x.is_sign_negative() {
            "-0.0".into()
        } else {
            "0.0".into()
        };
    }
    // `sci` is Rust's shortest round-trip form, e.g. "-1.2345e-5" or "3e2".
    let (mant, exp) = sci.split_once('e').unwrap_or((sci.as_str(), "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let neg = mant.starts_with('-');
    let digits: String = mant.chars().filter(|c| c.is_ascii_digit()).collect();
    let sign = if neg { "-" } else { "" };
    let ax = x.abs();
    if (1e-3..1e7).contains(&ax) {
        // Plain notation with at least one fractional digit.
        let point = exp + 1; // number of digits before the decimal point
        let (int_part, frac_part) = if point <= 0 {
            (
                "0".to_string(),
                format!("{}{}", "0".repeat((-point) as usize), digits),
            )
        } else if point as usize >= digits.len() {
            (
                format!("{}{}", digits, "0".repeat(point as usize - digits.len())),
                String::new(),
            )
        } else {
            (
                digits[..point as usize].to_string(),
                digits[point as usize..].to_string(),
            )
        };
        let frac = if frac_part.is_empty() {
            "0".to_string()
        } else {
            frac_part
        };
        format!("{sign}{int_part}.{frac}")
    } else {
        let (first, rest) = digits.split_at(1);
        let rest = if rest.is_empty() { "0" } else { rest };
        format!("{sign}{first}.{rest}E{exp}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_java_examples() {
        assert_eq!(double_to_string(0.0), "0.0");
        assert_eq!(double_to_string(1.0), "1.0");
        assert_eq!(double_to_string(-3.4362625660446744), "-3.4362625660446744");
        assert_eq!(double_to_string(11.740897499999999), "11.740897499999999");
        assert_eq!(double_to_string(1100.4389965), "1100.4389965");
        assert_eq!(double_to_string(0.001), "0.001");
        assert_eq!(double_to_string(0.0001), "1.0E-4");
        assert_eq!(double_to_string(1.0e7), "1.0E7");
        assert_eq!(double_to_string(1234567.0), "1234567.0");
        assert_eq!(double_to_string(-2.5e-10), "-2.5E-10");
        assert_eq!(float_to_string(3.0), "3.0");
    }
}
