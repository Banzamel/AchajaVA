//! Voice outputs from config.json (voice.outputs): a local device (headphones) or a Google Cast
//! speaker, the current choice in state\voice_output.json ("--once" returns to the default after
//! one answer), and the `achaja output` command.

use serde_json::{json, Value};

use crate::config::Config;
use crate::json::{read_json, remove, write_json};
use crate::paths::Paths;
use crate::state::VOICE_OUTPUT;
use crate::text::fold;

#[derive(Debug, Clone, PartialEq)]
pub struct Output {
    pub name: String,
    pub kind: String, // "local" | "cast"
    pub device: String,
    pub cast_name: String,
    pub host: String,
    pub aliases: Vec<String>,
}

fn parse(name: &str, v: &Value) -> Output {
    let s = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    Output {
        name: name.to_string(),
        kind: if s("type") == "cast" { "cast".into() } else { "local".into() },
        device: s("device"),
        cast_name: s("name"),
        host: s("host"),
        aliases: v
            .get("aliases")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
            .unwrap_or_default(),
    }
}

/// All outputs (in config order) and the default one's name.
pub fn outputs(cfg: &Config) -> (Vec<Output>, String) {
    let mut list: Vec<Output> = match cfg.get("voice.outputs") {
        Some(Value::Object(map)) => map.iter().map(|(n, v)| parse(n, v)).collect(),
        _ => Vec::new(),
    };
    if list.is_empty() {
        let device = cfg.str_or("voice.output_device", "");
        list.push(Output { name: "słuchawki".into(), kind: "local".into(), device, ..empty() });
    }
    let default = cfg.str_or("voice.default_output", &list[0].name);
    (list, default)
}

fn empty() -> Output {
    Output { name: String::new(), kind: String::new(), device: String::new(), cast_name: String::new(), host: String::new(), aliases: Vec::new() }
}

/// The output to speak on now.
pub fn current(paths: &Paths, cfg: &Config) -> Output {
    let (list, default) = outputs(cfg);
    let chosen = read_json(&paths.state_file(VOICE_OUTPUT), Value::Null)
        .get("output")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| default.clone());
    list.iter()
        .find(|o| o.name == chosen)
        .or_else(|| list.iter().find(|o| o.name == default))
        .cloned()
        .unwrap_or_else(|| list[0].clone())
}

/// After a one-time switch (--once) the next answer goes to the default again.
pub fn consume_once(paths: &Paths) {
    let file = paths.state_file(VOICE_OUTPUT);
    if read_json(&file, Value::Null).get("once").and_then(Value::as_bool) == Some(true) {
        remove(&file);
    }
}

/// The output a spoken name refers to ("głośnik", "speaker", part of the speaker's name).
pub fn find<'a>(list: &'a [Output], wanted: &str) -> Option<&'a Output> {
    let w = fold(wanted);
    list.iter().find(|o| {
        fold(&o.name) == w
            || fold(&o.name).contains(&w)
            || (!o.cast_name.is_empty() && fold(&o.cast_name).contains(&w))
            || o.aliases.iter().any(|a| fold(a) == w || fold(a).contains(&w))
    })
}

pub fn set(paths: &Paths, name: &str, once: bool) {
    let _ = write_json(&paths.state_file(VOICE_OUTPUT), &json!({ "output": name, "once": once }));
}

pub fn reset(paths: &Paths) {
    remove(&paths.state_file(VOICE_OUTPUT));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_by_alias_without_polish_letters() {
        let cfg = Config::new(json!({"voice": {"outputs": {
            "słuchawki": {"type": "local"},
            "głośnik": {"type": "cast", "name": "Kitchen", "host": "1.2.3.4", "aliases": ["google home"]}
        }}}));
        let (list, default) = outputs(&cfg);
        assert_eq!(default, "słuchawki");
        assert_eq!(find(&list, "glosnik").map(|o| o.host.as_str()), Some("1.2.3.4"));
        assert_eq!(find(&list, "Google Home").map(|o| o.kind.as_str()), Some("cast"));
        assert_eq!(find(&list, "kitchen").map(|o| o.name.as_str()), Some("głośnik"));
    }
}
