// SPDX-License-Identifier: GPL-3.0-or-later

//! Shell completions for bash and zsh (design spec 6.2, "Completions"), generated from the
//! command-line definition ([`crate::args::command`]) and the description files embedded at
//! build time (`descriptions/*.json`, real models only). They cover subcommands, flags,
//! `get`/`describe` paths, `set` paths with `=`, enum and bool values after `=`, action
//! names and their parameters, and live topics. They are static: Tab never contacts the
//! daemon, and `--device` values are not completed. `scripts/regenerate.sh` writes them to
//! `packaging/completions/`; a test fails when the committed files are stale.

use crate::write::KEEP;
use obsbot_proto::state::DAEMON_PATHS;
use obsbot_proto::{Description, Setting, SettingType};
use std::collections::{BTreeMap, BTreeSet};

include!(concat!(env!("OUT_DIR"), "/descriptions.rs"));

/// The descriptions embedded at build time: (model key, description).
pub fn descriptions() -> Vec<(&'static str, Description)> {
    EMBEDDED
        .iter()
        .map(|(key, json)| {
            let desc = serde_json::from_str(json)
                .unwrap_or_else(|e| panic!("descriptions/{key}.json does not parse: {e}"));
            (*key, desc)
        })
        .collect()
}

/// Every word the completions offer.
#[derive(Debug, Default, PartialEq)]
pub struct Words {
    pub subcommands: Vec<String>,
    /// Long flags of the command itself (`--json`, `--device`, `--help`, `--version`).
    pub global_flags: Vec<String>,
    /// Subcommand → its own long flags.
    pub flags: BTreeMap<String, Vec<String>>,
    /// `get`: the daemon fields, every leaf and every group.
    pub get_paths: Vec<String>,
    /// `describe`: every leaf and every action.
    pub describe_names: Vec<String>,
    /// `set`: every writable leaf, with `=`.
    pub set_paths: Vec<String>,
    pub actions: Vec<String>,
    /// Action → its parameters with `=` (not `keep`, which the command line sends itself).
    pub params: BTreeMap<String, Vec<String>>,
    /// `<path>` or `<action> <param>` → the words after `=` (enum writable values; bool
    /// `on off true false`).
    pub values: BTreeMap<String, Vec<String>>,
    pub topics: Vec<String>,
}

fn value_words(s: &Setting) -> Vec<String> {
    match s.kind {
        SettingType::Bool => ["on", "off", "true", "false"].map(String::from).to_vec(),
        SettingType::Enum => s.writable.clone(),
        _ => Vec::new(),
    }
}

fn sorted(set: BTreeSet<String>) -> Vec<String> {
    set.into_iter().collect()
}

/// The words of the command-line definition and of `descriptions`, merged over all models
/// (a path means the same on every model, spec 4.2).
pub fn words_of(descriptions: &[(&str, Description)]) -> Words {
    let mut cmd = crate::args::command();
    cmd.build();
    let long = |a: &clap::Arg| a.get_long().map(|l| format!("--{l}"));
    let global_flags: Vec<String> = cmd.get_arguments().filter_map(long).collect();
    let mut flags = BTreeMap::new();
    let mut subcommands = Vec::new();
    for sub in cmd.get_subcommands() {
        subcommands.push(sub.get_name().to_string());
        let own: Vec<String> = sub
            .get_arguments()
            .filter_map(long)
            .filter(|f| !global_flags.contains(f))
            .collect();
        if !own.is_empty() {
            flags.insert(sub.get_name().to_string(), own);
        }
    }
    let mut get = BTreeSet::new();
    let mut describe = BTreeSet::new();
    let mut set = BTreeSet::new();
    let mut actions = BTreeSet::new();
    let mut params: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut values: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut topics = BTreeSet::new();
    get.extend(DAEMON_PATHS.iter().map(|p| p.to_string()));
    for (_, desc) in descriptions {
        for (path, s) in &desc.settings {
            get.insert(path.clone());
            describe.insert(path.clone());
            let mut prefix = path.as_str();
            while let Some((group, _)) = prefix.rsplit_once('.') {
                get.insert(group.to_string());
                prefix = group;
            }
            if let Some(topic) = &s.live {
                topics.insert(topic.clone());
            }
            if !s.readonly {
                set.insert(format!("{path}="));
                let words = value_words(s);
                if !words.is_empty() {
                    values.entry(path.clone()).or_default().extend(words);
                }
            }
        }
        for (name, a) in &desc.actions {
            actions.insert(name.clone());
            describe.insert(name.clone());
            let entry = params.entry(name.clone()).or_default();
            for (param, s) in &a.params {
                if param == KEEP {
                    continue;
                }
                entry.insert(format!("{param}="));
                let words = value_words(s);
                if !words.is_empty() {
                    values
                        .entry(format!("{name} {param}"))
                        .or_default()
                        .extend(words);
                }
            }
        }
    }
    Words {
        subcommands,
        global_flags,
        flags,
        get_paths: sorted(get),
        describe_names: sorted(describe),
        set_paths: sorted(set),
        actions: sorted(actions),
        params: params
            .into_iter()
            .filter(|(_, p)| !p.is_empty())
            .map(|(k, p)| (k, sorted(p)))
            .collect(),
        values: values.into_iter().map(|(k, v)| (k, sorted(v))).collect(),
        topics: sorted(topics),
    }
}

/// The words of the command-line definition and the embedded descriptions.
pub fn words() -> Words {
    words_of(&descriptions())
}

/// Words joined by spaces, broken into lines of at most 96 characters after `indent`.
fn wrap(words: &[String], indent: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for w in words {
        if !line.is_empty() && indent.len() + line.len() + 1 + w.len() > 96 {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(w);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
        .iter()
        .map(|l| format!("{indent}{l}"))
        .collect::<Vec<_>>()
        .join("\n")
}

const HEADER: &str = "Generated by scripts/regenerate.sh from the command-line definition and \
descriptions/*.json; do not edit.";

/// The bash completion script (`packaging/completions/obsbot4linux.bash`).
pub fn bash() -> String {
    let w = words();
    let mut values = String::new();
    for (key, words) in &w.values {
        values.push_str(&format!(
            "        '{key}') REPLY='{}' ;;\n",
            words.join(" ")
        ));
    }
    let mut params = String::new();
    for (key, words) in &w.params {
        params.push_str(&format!(
            "        '{key}') REPLY='{}' ;;\n",
            words.join(" ")
        ));
    }
    let list = |words: &[String]| format!("'\n{}\n'", wrap(words, "    "));
    let flags_of = |sub: &str| {
        let mut all = w.flags.get(sub).cloned().unwrap_or_default();
        all.extend(w.global_flags.iter().cloned());
        all.join(" ")
    };
    format!(
        r#"# bash completion for obsbot4linux.
# {HEADER}

_obsbot4linux_values() {{
    case $1 in
{values}        *) REPLY='' ;;
    esac
}}

_obsbot4linux_params() {{
    case $1 in
{params}        *) REPLY='' ;;
    esac
}}

_obsbot4linux_get_paths={get}

_obsbot4linux_describe_names={describe}

_obsbot4linux_set_paths={set}

_obsbot4linux_actions={actions}

# Completes the value after `=` in $1 from the words in $2.
_obsbot4linux_assign() {{
    local key=${{1%%=*}} val=${{1#*=}}
    local -a found
    mapfile -t found < <(compgen -W "$2" -- "$val")
    if [[ $COMP_WORDBREAKS == *=* ]]; then
        COMPREPLY=("${{found[@]}}")
    else
        COMPREPLY=("${{found[@]/#/$key=}}")
    fi
}}

_obsbot4linux() {{
    local line=${{COMP_LINE:0:COMP_POINT}}
    local -a words
    read -r -a words <<< "$line"
    local cur=''
    if [[ $line != *[[:space:]] ]]; then
        cur=${{words[${{#words[@]}}-1]}}
        unset 'words[${{#words[@]}}-1]'
    fi
    local prev=${{words[${{#words[@]}}-1]}}
    local sub='' action='' i=1 w
    while (( i < ${{#words[@]}} )); do
        w=${{words[i]}}
        case $w in
            --device) (( i += 2 )); continue ;;
            -*) ;;
            *)
                if [[ -z $sub ]]; then
                    sub=$w
                elif [[ $sub == do && -z $action ]]; then
                    action=$w
                fi
                ;;
        esac
        (( i++ ))
    done
    local cands='' nospace='' REPLY=''
    if [[ $prev == --device ]]; then
        COMPREPLY=()
        return
    elif [[ $cur == -* ]]; then
        case $sub in
{sub_flags}            *) cands='{global}' ;;
        esac
    elif [[ -z $sub ]]; then
        cands='{subcommands}'
    else
        case $sub in
            get) cands=$_obsbot4linux_get_paths ;;
            describe) cands=$_obsbot4linux_describe_names ;;
            set)
                if [[ $cur == *=* ]]; then
                    _obsbot4linux_values "${{cur%%=*}}"
                    _obsbot4linux_assign "$cur" "$REPLY"
                    return
                fi
                cands=$_obsbot4linux_set_paths
                nospace=1
                ;;
            do)
                if [[ -z $action ]]; then
                    cands=$_obsbot4linux_actions
                elif [[ $cur == *=* ]]; then
                    _obsbot4linux_values "$action ${{cur%%=*}}"
                    _obsbot4linux_assign "$cur" "$REPLY"
                    return
                else
                    _obsbot4linux_params "$action"
                    cands=$REPLY
                    nospace=1
                fi
                ;;
            watch) cands='{topics}' ;;
        esac
    fi
    mapfile -t COMPREPLY < <(compgen -W "$cands" -- "$cur")
    if [[ -n $nospace ]]; then
        compopt -o nospace 2>/dev/null
    fi
}}

complete -F _obsbot4linux obsbot4linux
"#,
        get = list(&w.get_paths),
        describe = list(&w.describe_names),
        set = list(&w.set_paths),
        actions = list(&w.actions),
        sub_flags = w
            .flags
            .keys()
            .map(|sub| format!("            {sub}) cands='{}' ;;\n", flags_of(sub)))
            .collect::<String>(),
        global = w.global_flags.join(" "),
        subcommands = w.subcommands.join(" "),
        topics = w.topics.join(" "),
    )
}

/// The zsh completion script (`packaging/completions/_obsbot4linux`).
pub fn zsh() -> String {
    let w = words();
    let mut values = String::new();
    for (key, words) in &w.values {
        values.push_str(&format!("    '{key}') reply=({}) ;;\n", words.join(" ")));
    }
    let mut params = String::new();
    for (key, words) in &w.params {
        params.push_str(&format!("    '{key}') reply=({}) ;;\n", words.join(" ")));
    }
    let array = |words: &[String]| format!("(\n{}\n  )", wrap(words, "    "));
    let flags_of = |sub: &str| {
        let mut all = w.flags.get(sub).cloned().unwrap_or_default();
        all.extend(w.global_flags.iter().cloned());
        all.join(" ")
    };
    format!(
        r#"#compdef obsbot4linux
# zsh completion for obsbot4linux.
# {HEADER}

_obsbot4linux_values() {{
  case $1 in
{values}    *) reply=() ;;
  esac
}}

_obsbot4linux_params() {{
  case $1 in
{params}    *) reply=() ;;
  esac
}}

_obsbot4linux() {{
  local cur=${{words[CURRENT]}} prev=${{words[CURRENT-1]}} sub= action= i=2
  local -a reply
  local -a get_paths={get}
  local -a describe_names={describe}
  local -a set_paths={set}
  local -a actions={actions}
  while (( i < CURRENT )); do
    case ${{words[i]}} in
      --device) (( i += 2 )); continue ;;
      -*) ;;
      *)
        if [[ -z $sub ]]; then
          sub=${{words[i]}}
        elif [[ $sub == do && -z $action ]]; then
          action=${{words[i]}}
        fi
        ;;
    esac
    (( i++ ))
  done
  if [[ $prev == --device ]]; then
    return 1
  fi
  if [[ $cur == -* ]]; then
    case $sub in
{sub_flags}      *) compadd -- {global} ;;
    esac
    return
  fi
  if [[ -z $sub ]]; then
    compadd -- {subcommands}
    return
  fi
  case $sub in
    get) compadd -- $get_paths ;;
    describe) compadd -- $describe_names ;;
    set)
      if [[ $cur == *=* ]]; then
        _obsbot4linux_values ${{cur%%=*}}
        compadd -P "${{cur%%=*}}=" -- $reply
      else
        compadd -S '' -- $set_paths
      fi
      ;;
    do)
      if [[ -z $action ]]; then
        compadd -- $actions
      elif [[ $cur == *=* ]]; then
        _obsbot4linux_values "$action ${{cur%%=*}}"
        compadd -P "${{cur%%=*}}=" -- $reply
      else
        _obsbot4linux_params $action
        compadd -S '' -- $reply
      fi
      ;;
    watch) compadd -- {topics} ;;
  esac
}}

_obsbot4linux "$@"
"#,
        get = array(&w.get_paths),
        describe = array(&w.describe_names),
        set = array(&w.set_paths),
        actions = array(&w.actions),
        sub_flags = w
            .flags
            .keys()
            .map(|sub| format!("      {sub}) compadd -- {} ;;\n", flags_of(sub)))
            .collect::<String>(),
        global = w.global_flags.join(" "),
        subcommands = w.subcommands.join(" "),
        topics = w.topics.join(" "),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_real_models_are_embedded() {
        let keys: Vec<&str> = descriptions().iter().map(|(k, _)| *k).collect();
        assert!(keys.contains(&"tiny2lite"), "{keys:?}");
        assert!(!keys.iter().any(|k| k.starts_with("fake")), "{keys:?}");
    }

    #[test]
    fn the_words_come_from_the_definition_and_the_descriptions() {
        let w = words();
        assert_eq!(
            w.subcommands,
            ["status", "get", "set", "do", "describe", "devices", "watch"]
        );
        assert_eq!(
            w.global_flags,
            ["--json", "--device", "--help", "--version"]
        );
        assert_eq!(w.flags["devices"], ["--report"]);
        assert_eq!(w.flags["watch"], ["--live"]);
        assert!(w.set_paths.contains(&"zoom.factor=".to_string()));
        assert!(
            !w.set_paths.contains(&"gimbal.pan=".to_string()),
            "read-only leaves are not set"
        );
        assert!(w.get_paths.contains(&"zone.area".to_string()), "groups");
        assert!(
            w.get_paths.contains(&"pending".to_string()),
            "daemon fields"
        );
        assert!(w.describe_names.contains(&"preset.save".to_string()));
        assert_eq!(w.values["zoom.move speed"], ["fast", "medium", "slow"]);
        assert_eq!(w.values["image.mirror"], ["false", "off", "on", "true"]);
        assert_eq!(
            w.values["zoom.field_of_view"],
            ["medium", "narrow", "wide"],
            "only writable values"
        );
        assert_eq!(
            w.params["gimbal.turn"],
            ["pan_speed=", "tilt_speed="],
            "no keep"
        );
        assert!(!w.params.contains_key("gimbal.stop"));
        assert_eq!(w.topics, ["auto", "pose", "target"]);
    }

    #[test]
    fn a_second_model_adds_its_words_without_code() {
        let fake_b: Description =
            serde_json::from_str(include_str!("../../../descriptions/fake/fake-b.json")).unwrap();
        let mut all = descriptions();
        all.push(("fake-b", fake_b));
        let w = words_of(&all);
        assert!(w.set_paths.contains(&"beacon.color=".to_string()));
        assert_eq!(w.params["beacon.flash"], ["times="]);
        // A path both models have: the values of both.
        assert!(w.values["tracking.mode"].contains(&"person".to_string()));
        assert!(w.values["tracking.mode"].contains(&"upper_body".to_string()));
    }
}
