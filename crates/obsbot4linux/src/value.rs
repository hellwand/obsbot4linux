// SPDX-License-Identifier: GPL-3.0-or-later

//! The value syntax of the command line (design spec 6.2, "Value syntax"): one syntax per
//! setting type, the same for `set` leaves and `do` parameters. Only the text is parsed
//! here; bounds, steps, writable values, read-only leaves and `requires` are the daemon's
//! to check, and its refusal keeps its code (the daemon stays authoritative, spec 6.1).

use crate::VERSION;
use crate::errors::CliError;
use obsbot_proto::values::number;
use obsbot_proto::{Action, Description, Params, Patch, Setting, SettingType};
use serde_json::{Map, Value};

/// Splits `PATH=VALUE` at its first `=`. The value may be empty and may start with `-`.
pub fn split_assignment(arg: &str) -> Result<(&str, &str), CliError> {
    match arg.split_once('=') {
        Some(("", _)) => Err(CliError::usage(format!(
            "'{arg}' names no path before the '='"
        ))),
        Some(pair) => Ok(pair),
        None => Err(CliError::usage(format!("expected NAME=VALUE, got '{arg}'"))),
    }
}

fn number_of(path: &str, text: &str) -> Result<Value, CliError> {
    match text.trim().parse::<f64>() {
        Ok(x) if x.is_finite() => Ok(number(x)),
        _ => Err(CliError::usage(format!("{path}: '{text}' is not a number"))),
    }
}

/// Parses the text of one value for `setting` (`path` names it in messages):
///
/// - bool: `on`, `off`, `true`, `false`;
/// - enum: the value's name; `-` may stand for `_` (`upper-body`);
/// - number: a decimal number (`1.5`, `-20`);
/// - range, pose, point, box: the components in their order, comma-separated
///   (`min,max`; `pan,tilt,zoom`; `x,y`; `left,top,right,bottom`);
/// - string: the text as given, empty included.
///
/// A list cannot be written, and a type this command line does not know cannot be parsed.
pub fn parse_value(path: &str, setting: &Setting, text: &str) -> Result<Value, CliError> {
    match &setting.kind {
        SettingType::Bool => match text {
            "on" | "true" => Ok(Value::Bool(true)),
            "off" | "false" => Ok(Value::Bool(false)),
            _ => Err(CliError::usage(format!(
                "{path}: expected on, off, true or false, got '{text}'"
            ))),
        },
        SettingType::Enum => {
            let underscored = text.replace('-', "_");
            let known = |v: &str| setting.values.iter().any(|x| x == v);
            if !known(text) && known(&underscored) {
                Ok(Value::String(underscored))
            } else {
                Ok(Value::String(text.to_string()))
            }
        }
        SettingType::Number => number_of(path, text),
        SettingType::String => Ok(Value::String(text.to_string())),
        SettingType::List => Err(CliError::usage(format!(
            "{path} is a list and cannot be written"
        ))),
        kind => match kind.components() {
            Some(names) => {
                let parts: Vec<&str> = text.split(',').collect();
                if parts.len() != names.len() {
                    return Err(CliError::usage(format!(
                        "{path}: expected {}, got '{text}'",
                        names.join(",")
                    )));
                }
                let mut object = Map::new();
                for (name, part) in names.iter().zip(parts) {
                    object.insert(name.to_string(), number_of(path, part)?);
                }
                Ok(Value::Object(object))
            }
            None => Err(CliError::usage(format!(
                "{path} has a setting type that obsbot4linux {VERSION} cannot write"
            ))),
        },
    }
}

/// `set PATH=VALUE …` → one patch with dotted keys. A path the description does not have
/// is sent with its text as a string, so that the daemon answers it (`unsupported`, or
/// `invalid` for a group). Naming a path twice is a usage error.
pub fn parse_patch(desc: &Description, args: &[String]) -> Result<Patch, CliError> {
    let mut patch = Patch::new();
    for arg in args {
        let (path, text) = split_assignment(arg)?;
        if patch.0.contains_key(path) {
            return Err(CliError::usage(format!("{path} is named twice")));
        }
        let value = match desc.settings.get(path) {
            Some(setting) => parse_value(path, setting, text)?,
            None => Value::String(text.to_string()),
        };
        patch.0.insert(path.to_string(), value);
    }
    Ok(patch)
}

/// `do ACTION PARAM=VALUE …` → the action's parameters. A parameter the action does not have
/// (or an action the description does not have) is sent with its text as a string, so that
/// the daemon answers it. Naming a parameter twice is a usage error.
pub fn parse_params(action: Option<&Action>, args: &[String]) -> Result<Params, CliError> {
    let mut params = Params::new();
    for arg in args {
        let (name, text) = split_assignment(arg)?;
        if params.contains_key(name) {
            return Err(CliError::usage(format!("{name} is named twice")));
        }
        let value = match action.and_then(|a| a.params.get(name)) {
            Some(setting) => parse_value(name, setting, text)?,
            None => Value::String(text.to_string()),
        };
        params.insert(name.to_string(), value);
    }
    Ok(params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use obsbot_proto::{Asleep, Unit};
    use serde_json::json;

    fn ok(setting: &Setting, text: &str) -> Value {
        parse_value("p", setting, text).unwrap()
    }

    fn usage(setting: &Setting, text: &str) -> String {
        let e = parse_value("p", setting, text).unwrap_err();
        assert_eq!(e.code.as_str(), "usage");
        assert_eq!(e.code.exit(), 2);
        e.message
    }

    #[test]
    fn bool_takes_on_off_true_false() {
        let s = Setting::boolean("Mirror");
        assert_eq!(ok(&s, "on"), json!(true));
        assert_eq!(ok(&s, "true"), json!(true));
        assert_eq!(ok(&s, "off"), json!(false));
        assert_eq!(ok(&s, "false"), json!(false));
        assert_eq!(
            usage(&s, "yes"),
            "p: expected on, off, true or false, got 'yes'"
        );
        usage(&s, "On");
        usage(&s, "1");
    }

    #[test]
    fn enum_takes_the_name_and_a_minus_for_an_underscore() {
        let s = Setting::enumeration("Mode", &["static", "upper_body", "50hz"]);
        assert_eq!(ok(&s, "upper_body"), json!("upper_body"));
        assert_eq!(ok(&s, "upper-body"), json!("upper_body"));
        assert_eq!(ok(&s, "50hz"), json!("50hz"));
        // Not a value of the description: sent as typed, the daemon answers it.
        assert_eq!(ok(&s, "sideways"), json!("sideways"));
        assert_eq!(ok(&s, "side-ways"), json!("side-ways"));
    }

    #[test]
    fn a_value_with_a_minus_of_its_own_stays_as_it_is() {
        let s = Setting::enumeration("Odd", &["a-b", "a_b"]);
        assert_eq!(ok(&s, "a-b"), json!("a-b"));
        assert_eq!(ok(&s, "a_b"), json!("a_b"));
    }

    #[test]
    fn number_is_decimal_and_integral_numbers_stay_integers() {
        let s = Setting::number("Zoom", 1.0, 4.0).unit(Unit::Factor);
        assert_eq!(ok(&s, "1.5").to_string(), "1.5");
        assert_eq!(ok(&s, "2").to_string(), "2");
        assert_eq!(ok(&s, "2.0").to_string(), "2");
        assert_eq!(ok(&s, "-20").to_string(), "-20");
        // Out of bounds is the daemon's to refuse (out_of_range), not a usage error.
        assert_eq!(ok(&s, "9").to_string(), "9");
        assert_eq!(usage(&s, "1,5"), "p: '1,5' is not a number");
        usage(&s, "");
        usage(&s, "inf");
        usage(&s, "NaN");
        usage(&s, "0x10");
    }

    #[test]
    fn range_is_min_comma_max() {
        let s = Setting::range("Pan area", -150.0, 150.0).unit(Unit::Deg);
        assert_eq!(ok(&s, "-20,20"), json!({"min": -20, "max": 20}));
        assert_eq!(ok(&s, "-20.5,20"), json!({"min": -20.5, "max": 20}));
        // Review focus: a quoted value with spaces after the commas.
        assert_eq!(ok(&s, "-20, 20"), json!({"min": -20, "max": 20}));
        assert_eq!(usage(&s, "20"), "p: expected min,max, got '20'");
        usage(&s, "1,2,3");
        usage(&s, "a,b");
        usage(&s, "-20,");
    }

    #[test]
    fn pose_point_and_box_take_their_components_in_order() {
        let pose = Setting::new(SettingType::Pose, "Pose");
        assert_eq!(
            ok(&pose, "0,5,1.2"),
            json!({"pan": 0, "tilt": 5, "zoom": 1.2})
        );
        assert_eq!(usage(&pose, "0,5"), "p: expected pan,tilt,zoom, got '0,5'");
        let point = Setting::new(SettingType::Point, "Point");
        assert_eq!(ok(&point, "0.5,0.25"), json!({"x": 0.5, "y": 0.25}));
        let bx = Setting::new(SettingType::Box, "Box");
        assert_eq!(
            ok(&bx, "0,0,1,1"),
            json!({"left": 0, "top": 0, "right": 1, "bottom": 1})
        );
    }

    /// Spec 6.2: free text after `=` may start with `-`; a string is taken as given.
    #[test]
    fn string_is_taken_as_given() {
        let s = Setting::string("Name", 32, true);
        assert_eq!(ok(&s, "Desk"), json!("Desk"));
        assert_eq!(ok(&s, "-x, y=z"), json!("-x, y=z"));
        assert_eq!(ok(&s, ""), json!(""));
    }

    #[test]
    fn a_list_and_an_unknown_type_cannot_be_written() {
        let list = Setting::list("Presets", &[("slot", Setting::number("Slot", 0.0, 2.0))]);
        assert_eq!(usage(&list, "x"), "p is a list and cannot be written");
        let future = Setting::new(SettingType::Other("colour".into()), "Colour");
        assert!(usage(&future, "red").contains("cannot write"));
    }

    fn desc() -> Description {
        Description::default()
            .setting(
                "zone.area.pan",
                Setting::range("Pan", -150.0, 150.0).asleep(Asleep::Unknown),
            )
            .setting("led.brightness", Setting::number("LED", 0.0, 3.0))
            .setting("image.mirror", Setting::boolean("Mirror"))
    }

    #[test]
    fn a_patch_holds_one_dotted_key_per_argument() {
        let args: Vec<String> = [
            "zone.area.pan=-20,20",
            "led.brightness=2",
            "image.mirror=on",
        ]
        .map(String::from)
        .to_vec();
        let patch = parse_patch(&desc(), &args).unwrap();
        assert_eq!(
            serde_json::to_string(&patch).unwrap(),
            r#"{"image.mirror":true,"led.brightness":2,"zone.area.pan":{"max":20,"min":-20}}"#
        );
    }

    /// The daemon answers paths it does not have (`unsupported`) and groups (`invalid`).
    #[test]
    fn a_path_the_description_lacks_is_sent_as_text() {
        let args: Vec<String> = ["image.gamma=1", "zone=1"].map(String::from).to_vec();
        let patch = parse_patch(&desc(), &args).unwrap();
        assert_eq!(patch.0["image.gamma"], json!("1"));
        assert_eq!(patch.0["zone"], json!("1"));
    }

    #[test]
    fn bad_assignments_are_usage_errors() {
        let one = |a: &str| parse_patch(&desc(), &[a.to_string()]).unwrap_err().message;
        assert_eq!(
            one("led.brightness"),
            "expected NAME=VALUE, got 'led.brightness'"
        );
        assert_eq!(one("=2"), "'=2' names no path before the '='");
        assert_eq!(
            one("led.brightness=x"),
            "led.brightness: 'x' is not a number"
        );
        let twice = ["led.brightness=1", "led.brightness=2"]
            .map(String::from)
            .to_vec();
        assert_eq!(
            parse_patch(&desc(), &twice).unwrap_err().message,
            "led.brightness is named twice"
        );
    }

    #[test]
    fn parameters_use_the_same_syntax() {
        let action = Action::new("Save a view", Asleep::Ignored)
            .param("name", Setting::string("Name", 32, true))
            .param("pose", Setting::new(SettingType::Pose, "Pose").optional());
        let args: Vec<String> = ["name=Desk", "pose=0,5,1.2"].map(String::from).to_vec();
        let params = parse_params(Some(&action), &args).unwrap();
        assert_eq!(
            Value::Object(params),
            json!({"name": "Desk", "pose": {"pan": 0, "tilt": 5, "zoom": 1.2}})
        );
        // An unknown parameter or action goes to the daemon as text.
        let params = parse_params(Some(&action), &["colour=red".to_string()]).unwrap();
        assert_eq!(params["colour"], json!("red"));
        let params = parse_params(None, &["slot=1".to_string()]).unwrap();
        assert_eq!(params["slot"], json!("1"));
        // Only the first `=` splits: the value may contain more.
        let params = parse_params(None, &["note=a=b".to_string()]).unwrap();
        assert_eq!(params["note"], json!("a=b"));
        assert!(parse_params(None, &[]).unwrap().is_empty());
    }
}
