// SPDX-License-Identifier: GPL-3.0-or-later

//! Value helpers shared by the daemon, the models and the clients: canonical numbers,
//! rounding to a precision (4.2 "Precision"), `raw:<n>` for undecodable values and value
//! equality that treats `1` and `1.0` as the same number.

use serde_json::{Number, Value};

/// A JSON number in canonical form: an integral value within ±2^53 is written as an integer
/// (`3`, not `3.0`), anything else as a float. Non-finite input gives `null`.
pub fn number(x: f64) -> Value {
    if !x.is_finite() {
        return Value::Null;
    }
    if x.fract() == 0.0 && x.abs() < 9_007_199_254_740_992.0 {
        return Value::Number(Number::from(x as i64));
    }
    Number::from_f64(x).map_or(Value::Null, Value::Number)
}

/// `x` rounded to `precision` (0.1, 0.01, 1, 100 …) in canonical form, so that state and
/// `--json` never show binary float noise.
pub fn rounded(x: f64, precision: f64) -> Value {
    if precision <= 0.0 {
        return number(x);
    }
    let steps = (x / precision).round();
    let decimals = (-precision.log10()).ceil().max(0.0) as i32;
    let scale = 10f64.powi(decimals);
    number((steps * precision * scale).round() / scale)
}

/// The published form of a camera value the model cannot decode: `raw:<n>` (4.2).
pub fn raw(n: i64) -> Value {
    Value::String(format!("raw:{n}"))
}

/// Whether a string is a `raw:<n>` value.
pub fn is_raw(text: &str) -> bool {
    text.strip_prefix("raw:").is_some_and(|n| {
        !n.is_empty()
            && n.trim_start_matches('-')
                .bytes()
                .all(|b| b.is_ascii_digit())
    })
}

/// Value equality in which numbers compare by value (`1` equals `1.0`), recursively.
pub fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w)))
        }
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn integral_numbers_are_written_without_a_fraction() {
        assert_eq!(number(3.0).to_string(), "3");
        assert_eq!(number(-150.0).to_string(), "-150");
        assert_eq!(number(1.5).to_string(), "1.5");
        assert_eq!(number(f64::NAN), Value::Null);
    }

    #[test]
    fn rounding_removes_float_noise() {
        assert_eq!(rounded(0.1 + 0.2, 0.1).to_string(), "0.3");
        assert_eq!(rounded(12.345_678, 0.01).to_string(), "12.35");
        assert_eq!(rounded(-30.04, 0.1).to_string(), "-30");
        assert_eq!(rounded(1234.0, 100.0).to_string(), "1200");
    }

    #[test]
    fn raw_values_are_recognised() {
        assert_eq!(raw(3), json!("raw:3"));
        assert!(is_raw("raw:3"));
        assert!(is_raw("raw:-1"));
        assert!(!is_raw("raw:"));
        assert!(!is_raw("raw:x"));
        assert!(!is_raw("auto"));
    }

    #[test]
    fn numbers_compare_by_value() {
        assert!(same(&json!(1), &json!(1.0)));
        assert!(same(
            &json!({"min": 1, "max": 2}),
            &json!({"max": 2.0, "min": 1.0})
        ));
        assert!(!same(&json!(1), &json!("1")));
        assert!(!same(&json!([1]), &json!([1, 2])));
    }
}
