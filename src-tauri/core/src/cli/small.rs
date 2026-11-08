//! The smaller commands: plan, jobs, output, speak, devices, audio.

use clap::Subcommand;

use crate::config;
use crate::error::{AppError, Result};
use crate::paths::Paths;
use crate::plan::{self, jobs, store, AddArgs};
use crate::voice::outputs;

#[derive(Debug, Subcommand)]
pub enum PlanCmd {
    /// Dodaje przypomnienie (--say), polecenie (--prompt) albo zadanie w tle (--job)
    Add(AddArgs),
    /// Co jest zaplanowane
    List,
    /// Usuwa pozycje (numer albo all)
    Remove { target: String },
    /// Zadanie w tle uruchom teraz
    Run { target: String },
}

#[derive(Debug, Subcommand)]
pub enum JobsCmd {
    /// Ostatnie wyniki zadan
    List,
    /// Pelny ostatni wynik zadania
    Last { target: String },
    #[command(hide = true)]
    Exec { spec: String },
}

#[derive(Debug, Subcommand)]
pub enum OutputCmd {
    /// Przelacza wyjscie (na stale albo --once)
    Set {
        name: String,
        #[arg(long)]
        once: bool,
    },
    /// Powrot do domyslnego
    Reset,
    /// Glosniki Google Cast w sieci
    Discover,
}

pub fn plan(p: &Paths, cmd: PlanCmd) -> Result<i32> {
    match cmd {
        PlanCmd::Add(a) => plan::print_added(&plan::add(p, &a)?),
        PlanCmd::List => {
            let items = store::items(p);
            if items.is_empty() {
                println!("Nic nie jest zaplanowane.");
            }
            items.iter().for_each(|i| println!("{}", plan::describe(i)));
        }
        PlanCmd::Remove { target } => {
            let n = store::remove(p, &target);
            println!("{}", if n > 0 { format!("Usunieto {n}.") } else { format!("Nie ma zadania {target}.") });
        }
        PlanCmd::Run { target } => {
            let item = store::find(p, &target).filter(|i| i.kind == "job").ok_or_else(|| AppError::msg(format!("Nie ma zadania w tle {target}.")))?;
            println!("Uruchomiono zadanie '{}' w tle: {}", item.name.clone().unwrap_or_default(), jobs::start(p, &item));
        }
    }
    Ok(0)
}

pub fn jobs(p: &Paths, cmd: JobsCmd) -> Result<i32> {
    match cmd {
        JobsCmd::Exec { spec } => return Ok(jobs::execute(p, std::path::Path::new(&spec))),
        JobsCmd::List => {
            for h in jobs::history(p).iter().take(20) {
                let when = plan::schedule::from_ts(h["at"].as_f64().unwrap_or(0.0)).format("%d.%m %H:%M");
                let ok = if h["ok"].as_bool().unwrap_or(false) { "OK" } else { "BLAD" };
                println!("{when}  [{}] {}  {ok}  {}", h["id"], h["name"].as_str().unwrap_or(""), h["voice"].as_str().unwrap_or(""));
            }
        }
        JobsCmd::Last { target } => {
            let hist = jobs::history(p);
            let t = target.to_lowercase();
            let hit = hist.iter().find(|h| h["id"].to_string() == target || h["name"].as_str().unwrap_or("").to_lowercase().contains(&t));
            let text = hit.and_then(|h| h["id"].as_u64()).and_then(|id| jobs::last_report(p, id));
            println!("{}", text.unwrap_or_else(|| "Brak wyniku.".into()));
        }
    }
    Ok(0)
}

pub fn output(p: &Paths, cmd: Option<OutputCmd>) -> Result<i32> {
    let cfg = config::load(p)?;
    let (list, default) = outputs::outputs(&cfg);
    match cmd {
        None => {
            println!("Aktualne wyjscie: {} (domyslne: {default})", outputs::current(p, &cfg).name);
            for o in &list {
                println!("  {}: {} {}{}", o.name, o.kind, if o.kind == "cast" { format!("{} ({})", o.cast_name, o.host) } else { o.device.clone() }, if o.aliases.is_empty() { String::new() } else { format!(" | aliasy: {}", o.aliases.join(", ")) });
            }
        }
        Some(OutputCmd::Set { name, once }) => {
            let o = outputs::find(&list, &name).ok_or_else(|| AppError::msg(format!("Nie ma wyjscia '{name}'. Dostepne: {}", list.iter().map(|o| o.name.as_str()).collect::<Vec<_>>().join(", "))))?;
            outputs::set(p, &o.name, once);
            println!("Wyjscie glosu: {}{}", o.name, if once { " (tylko nastepna wypowiedz)" } else { "" });
        }
        Some(OutputCmd::Reset) => {
            outputs::reset(p);
            println!("Wyjscie glosu: {default} (domyslne)");
        }
        Some(OutputCmd::Discover) => {
            for s in crate::cast::discover(8) {
                println!("{} | {} | {}", s.name, s.model, s.host);
            }
        }
    }
    Ok(0)
}

pub fn speak(p: &Paths, file: Option<String>, text: Vec<String>) -> Result<i32> {
    let text = match file {
        Some(f) => std::fs::read_to_string(f)?,
        None => text.join(" "),
    };
    crate::voice::speak::run(p, &text)?;
    Ok(0)
}

pub fn devices() -> Result<i32> {
    let d = crate::voice::devices::list();
    println!("Mikrofony:\n  {}\nWyjscia:\n  {}", d.input.join("\n  "), d.output.join("\n  "));
    Ok(0)
}

pub fn audio(args: &[String]) -> Result<i32> {
    match args {
        [cmd] if cmd == "list" => println!("{}", crate::audio_routing::list()),
        [pid, kind, wanted] if kind == "input" || kind == "output" => {
            let pid: u32 = pid.parse().map_err(|_| AppError::msg("Zly numer procesu."))?;
            println!("{}", crate::audio_routing::assign(pid, kind, wanted)?);
        }
        _ => return Err(AppError::msg("Uzycie: achaja audio <pid> input|output <nazwa|default>  albo  achaja audio list")),
    }
    Ok(0)
}
