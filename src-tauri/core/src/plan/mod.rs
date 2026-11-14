//! Reminders, timed commands and background jobs (`achaja plan ...`).

pub mod job_claude;
pub mod jobs;
pub mod schedule;
pub mod store;

use clap::Args;

use schedule::{describe_repeat, from_ts, parse_when, to_ts, When};
use store::{kind_name, load, save, Item, ALLOW};

use crate::error::{AppError, Result};
use crate::paths::Paths;

pub const SCHEDULED: &str = crate::hooks::yes::SCHEDULED;

#[derive(Debug, Args, Default)]
pub struct AddArgs {
    #[arg(long)]
    pub say: Option<String>,
    #[arg(long)]
    pub prompt: Option<String>,
    #[arg(long)]
    pub job: Option<String>,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub rules: Option<String>,
    #[arg(long)]
    pub allow: Option<String>,
    #[arg(long = "in")]
    pub in_: Option<String>,
    #[arg(long)]
    pub at: Option<String>,
    #[arg(long)]
    pub daily: Option<String>,
    #[arg(long)]
    pub weekdays: Option<String>,
    #[arg(long)]
    pub weekly: Option<String>,
}

pub fn describe(i: &Item) -> String {
    let mut line = format!(
        "[{}] {} | {} | {}: ",
        i.id,
        from_ts(i.next).format("%d.%m %H:%M"),
        describe_repeat(i.repeat.as_deref()),
        kind_name(&i.kind)
    );
    if i.kind == "job" {
        line += &format!("{} - {}", i.name.clone().unwrap_or_default(), i.text);
        if let Some(r) = i.rules.as_ref().filter(|r| !r.is_empty()) {
            line += &format!(" | zasady: {r}");
        }
        if let Some(a) = i.allow.as_ref().filter(|a| !a.is_empty()) {
            line += &format!(" | zgody: {}", a.join(", "));
        }
        return line;
    }
    line + &i.text
}

pub fn add(paths: &Paths, a: &AddArgs) -> Result<Item> {
    let given: Vec<(&str, &String)> = [("say", &a.say), ("prompt", &a.prompt), ("job", &a.job)].into_iter().filter_map(|(k, v)| v.as_ref().map(|v| (k, v))).collect();
    let [(kind, text)] = given.as_slice() else {
        return Err(AppError::msg("Podaj jedno: --say (przypomnienie), --prompt (polecenie) albo --job (zadanie w tle)."));
    };
    let mut allow: Vec<String> = a.allow.as_deref().unwrap_or("").split(',').map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).collect();
    allow.sort();
    allow.dedup();
    let bad: Vec<&String> = allow.iter().filter(|x| !ALLOW.contains(&x.as_str())).collect();
    if !bad.is_empty() {
        return Err(AppError::msg(format!("Nieznane zgody: {}. Dozwolone: {}.", bad.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "), ALLOW.join(", "))));
    }
    if *kind != "job" && (!allow.is_empty() || a.rules.is_some()) {
        return Err(AppError::msg("--rules i --allow sa tylko dla zadan w tle (--job)."));
    }
    let now = chrono::Local::now().naive_local();
    let when = When { in_: a.in_.clone(), at: a.at.clone(), daily: a.daily.clone(), weekdays: a.weekdays.clone(), weekly: a.weekly.clone() };
    let (first, repeat) = parse_when(&when, now)?;
    if first <= now {
        return Err(AppError::msg("Ten termin juz minal."));
    }
    let mut plan = load(paths);
    let text = crate::text::squash(text);
    let mut item = Item { id: plan.next_id, kind: kind.to_string(), text: text.clone(), next: to_ts(first), repeat, name: None, rules: None, allow: None };
    if *kind == "job" {
        let name = a.name.clone().unwrap_or_else(|| crate::text::truncate_chars(&text, 40));
        item.name = Some(name.trim().to_string());
        item.rules = Some(a.rules.clone().unwrap_or_default().trim().to_string());
        item.allow = Some(allow);
    }
    plan.next_id = item.id + 1;
    plan.items.push(item.clone());
    save(paths, &plan);
    Ok(item)
}

pub fn print_added(i: &Item) {
    let extra = i.allow.as_ref().filter(|a| !a.is_empty()).map(|a| format!(" | zgody: {}", a.join(", "))).unwrap_or_default();
    println!(
        "Zaplanowano [{}] {} ({}): {} - {}{extra}",
        i.id,
        from_ts(i.next).format("%d.%m %H:%M"),
        describe_repeat(i.repeat.as_deref()),
        kind_name(&i.kind),
        i.name.clone().unwrap_or_else(|| i.text.clone())
    );
}
