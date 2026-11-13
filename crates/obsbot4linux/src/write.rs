// SPDX-License-Identifier: GPL-3.0-or-later

//! The writing commands (design spec 6.2): `set` sends one `set` with the whole patch, `do`
//! sends one `do`. Neither chains a second request (spec 2): `do gimbal.move` leaves the
//! tracking as it is, and a user who wants it off runs `set tracking.mode=static` first.

use crate::errors::CliError;
use crate::{Ctx, render, session, value};
use obsbot_proto::{Action, Params, RevOk};
use serde_json::Value;

/// The parameter that keeps a continuing action going after its connection closes
/// (spec 4.4, `gimbal.turn`).
pub const KEEP: &str = "keep";

/// `set <path>=<value> …`: one patch. `--json`: `{"rev"}`; human output prints nothing.
pub fn set(ctx: &mut Ctx, assignments: &[String]) -> Result<(), CliError> {
    let (mut client, device) =
        session::connect_device(ctx.socket.as_deref(), ctx.device.as_deref())?;
    let patch = value::parse_patch(&device.description, assignments)?;
    let rev = client
        .set(&device.id, patch)
        .map_err(session::client_error)?;
    if ctx.json {
        let _ = writeln!(ctx.out, "{{\"rev\":{rev}}}");
    }
    Ok(())
}

/// An action with a `keep` parameter lasts as long as its connection unless `keep` is true
/// (spec 4.4). The command line's connection closes right after the reply, so it always
/// sends `keep: true` (spec 6.2, "`gimbal.turn`"); any other `keep` is a usage error.
pub fn keep(action: Option<&Action>, params: &mut Params) -> Result<(), CliError> {
    if !action.is_some_and(|a| a.params.contains_key(KEEP)) {
        return Ok(());
    }
    match params.get(KEEP) {
        None | Some(Value::Bool(true)) => {
            params.insert(KEEP.into(), Value::Bool(true));
            Ok(())
        }
        Some(_) => Err(CliError::usage(
            "keep: the command line always sends keep=true, because its connection closes \
             right after the reply; stop the action with its stop action",
        )),
    }
}

/// `do <action> [<param>=<value> …]`: one action. `--json`: `{"rev","result"?}`; human
/// output prints the result's fields, if the action has a result, and nothing otherwise.
pub fn act(ctx: &mut Ctx, action: &str, args: &[String]) -> Result<(), CliError> {
    let (mut client, device) =
        session::connect_device(ctx.socket.as_deref(), ctx.device.as_deref())?;
    let spec = device.description.actions.get(action);
    let mut params = value::parse_params(spec, args)?;
    keep(spec, &mut params)?;
    let (rev, result) = client
        .act(&device.id, action, Value::Object(params))
        .map_err(session::client_error)?;
    if ctx.json {
        let reply = RevOk { rev, result };
        let _ = writeln!(
            ctx.out,
            "{}",
            serde_json::to_string(&reply).expect("a reply serialises")
        );
    } else if let Some(result) = result {
        for (name, v) in &result {
            let described = spec
                .and_then(|a| a.result.as_ref())
                .and_then(|r| r.get(name));
            let text = match described {
                Some(setting) => render::value_text(setting, v),
                None => v.to_string(),
            };
            let _ = writeln!(ctx.out, "{name}: {text}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use obsbot_proto::{Asleep, Setting};
    use serde_json::json;

    fn turn() -> Action {
        Action::new("Turn", Asleep::Unknown)
            .param(
                "pan_speed",
                Setting::number("Pan speed", -178.0, 178.0).optional(),
            )
            .param("keep", Setting::boolean("Keep").optional())
    }

    /// Spec 6.2: the CLI always sends keep:true with gimbal.turn.
    #[test]
    fn a_continuing_action_always_gets_keep_true() {
        let mut p = Params::new();
        p.insert("pan_speed".into(), json!(20));
        keep(Some(&turn()), &mut p).unwrap();
        assert_eq!(Value::Object(p), json!({"pan_speed": 20, "keep": true}));
        let mut p = Params::new();
        p.insert("keep".into(), json!(true));
        keep(Some(&turn()), &mut p).unwrap();
        assert_eq!(p["keep"], json!(true));
    }

    #[test]
    fn keep_false_is_refused_and_other_actions_are_left_alone() {
        let mut p = Params::new();
        p.insert("keep".into(), json!(false));
        assert_eq!(keep(Some(&turn()), &mut p).unwrap_err().code.exit(), 2);
        let move_ = Action::new("Move", Asleep::Unknown);
        let mut p = Params::new();
        keep(Some(&move_), &mut p).unwrap();
        assert!(p.is_empty(), "no keep for an action without it");
        keep(None, &mut p).unwrap();
        assert!(p.is_empty());
    }
}
