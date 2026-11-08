//! The listener loop: audio blocks from the microphone into Vosk — a grammar of the wake phrases
//! while idle (finished utterances only: partial results caught "haha", "ja ha"), the full
//! vocabulary while recording (the end word, "cancel", "new conversation") — plus the window's
//! requests and the periodic checks. Returns "closed" when the Claude session ended.

use std::sync::mpsc::{channel, RecvTimeoutError};
use std::time::Duration;

use serde_json::Value;

use super::control::{Ctl, Mode};
use super::ticks::{tick, Timers};
use super::wake::{ends_with, wake_heard};
use super::{replies, session, Bridge, Request};
use crate::config;
use crate::error::{AppError, Result};
use crate::json::now;
use crate::paths::Paths;
use crate::process::pid_alive;
use crate::state::{self, FOLLOWUP};
use crate::stt::vosk::{Model, Recognizer, Vosk};
use crate::voice::capture::Microphone;

fn json(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or(Value::Null)
}

fn handle(ctl: &mut Ctl, req: Request) {
    match req {
        Request::Record if ctl.mode == Mode::Idle => {
            crate::voice::speak::request_stop(ctl.paths);
            state::clear(ctl.paths, FOLLOWUP);
            ctl.start_recording("Nagrywanie z przycisku.");
        }
        Request::Send if ctl.mode == Mode::Recording => ctl.send(),
        Request::Cancel if ctl.mode == Mode::Recording => ctl.cancel(),
        Request::Clear => ctl.new_conversation(),
        Request::Type(text) if !text.trim().is_empty() => {
            if ctl.mode == Mode::Recording {
                ctl.cancel();
            }
            // while Claude works it queues the message and sends it afterwards
            ctl.submit(&crate::text::squash(&text));
            ctl.bridge.log("Wyslano wiadomosc z okna czatu.");
        }
        Request::Keys(keys) => {
            let k: Vec<&str> = keys.iter().map(String::as_str).collect();
            ctl.press(&k);
        }
        _ => {}
    }
}

pub fn run(paths: &Paths, bridge: &Bridge, resume: bool) -> Result<&'static str> {
    let cfg = config::load(paths)?;
    let model_dir = config::model_dir(paths, &cfg);
    if !model_dir.is_dir() {
        return Err(AppError::msg(format!("Brak modelu mowy {} - pobierz go w ustawieniach (Ogolne).", model_dir.display())));
    }
    let vosk = Vosk::load(&crate::stt::library(paths))?;
    let model = Model::new(&vosk, &model_dir)?;
    let (wake, ends) = (cfg.strs("wake.phrases"), cfg.strs("wake.end"));
    let (cancel, clear) = (cfg.strs("wake.cancel"), cfg.strs("wake.clear"));
    let mut grammar = wake.clone();
    grammar.push("[unk]".into());
    let wake_rec = Recognizer::new(&model, 16000.0, Some(&serde_json::to_string(&grammar)?))?;
    wake_rec.set_words(true); // per-word confidence: rejects "haha", "ja ha"
    let free_rec = Recognizer::new(&model, 16000.0, None)?;
    state::clear(paths, FOLLOWUP);
    let pid = session::attach_or_launch(paths, &cfg, bridge, resume)?;
    bridge.pid.store(pid, std::sync::atomic::Ordering::Relaxed);
    session::start_background(paths, &cfg, bridge);
    let (tx, rx) = channel();
    let wanted = cfg.str_or("wake.microphone", "");
    if !wanted.is_empty() && !crate::voice::devices::input_exists(&wanted) {
        bridge.log(&format!("Nie znaleziono mikrofonu '{wanted}' - uzywam domyslnego."));
    }
    let mut mic = Microphone::new(&wanted, tx);
    let default_reply = if cfg.language() == "pl" { "Słucham." } else { "Yes?" };
    let mut replies_text = cfg.strs("voice.wake_replies");
    if cfg.get("voice.wake_replies").is_none() {
        replies_text = vec![default_reply.to_string()];
    }
    let clips = replies::prepare(paths, &cfg, bridge.sender(), replies_text.clone());
    let (end_match, end_stable) = (cfg.f64_or("wake.end_match", 0.7), cfg.f64_or("wake.end_stable_seconds", 0.6));
    let (wake_conf, isolated, debug) = (cfg.f64_or("wake.wake_confidence", 0.8), cfg.bool_or("wake.wake_isolated", true), cfg.bool_or("wake.debug", true));
    let mut ctl = Ctl::new(paths, cfg.clone(), bridge, pid);
    let mut timers = Timers::new();
    let (mut partial_text, mut partial_since, mut mic_retry, mut shown) = (String::new(), 0.0, 0.0, String::new());
    bridge.log("Stan: czekam na 'Achaja'");
    while !bridge.stopped() {
        let chunk = match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(c) => Some(c),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => None,
        };
        if !pid_alive(pid) {
            bridge.log("Okno Achai zostalo zamkniete - koncze nasluch.");
            return Ok("closed");
        }
        // pausing releases the microphone: Bluetooth headsets go back to stereo (music, games);
        // recording from the button during a pause opens it for a while
        let need_mic = !bridge.paused() || ctl.mode != Mode::Idle;
        if need_mic != mic.is_open() && now() >= mic_retry {
            if need_mic {
                while rx.try_recv().is_ok() {}
                wake_rec.reset();
                match mic.open() {
                    Ok(()) => bridge.log(&format!("Nasluch wznowiony - mikrofon otwarty ({}).", mic.name)),
                    Err(e) => {
                        mic_retry = now() + 5.0;
                        bridge.log(&format!("BLAD mikrofonu: {e}"));
                    }
                }
            } else {
                mic.close();
                bridge.log("Nasluch wstrzymany - mikrofon zwolniony (sluchawki graja normalnie).");
            }
        }
        for req in bridge.take_requests() {
            handle(&mut ctl, req);
        }
        let label = if ctl.mode == Mode::Recording { "recording" } else { "idle" };
        if label != shown {
            shown = label.to_string();
            bridge.set_state(label);
        }
        tick(&mut ctl, &mut timers);
        if std::mem::take(&mut ctl.reset_free) {
            free_rec.reset();
            partial_text.clear();
        }
        let Some(chunk) = chunk else { continue };
        if ctl.mode == Mode::Idle {
            if bridge.paused() || !wake_rec.accept(&chunk) {
                continue;
            }
            let result = json(&wake_rec.result());
            if debug && !result["text"].as_str().unwrap_or("").is_empty() {
                let confs: Vec<String> = result["result"].as_array().into_iter().flatten().map(|w| format!("{}:{:.2}", w["word"].as_str().unwrap_or(""), w["conf"].as_f64().unwrap_or(0.0))).collect();
                bridge.log(&format!("  czuwanie: {}  ({})", result["text"].as_str().unwrap_or(""), confs.join(" ")));
            }
            if wake_heard(&result, &wake, wake_conf, isolated) {
                crate::voice::speak::request_stop(paths);
                state::clear(paths, FOLLOWUP);
                bridge.set_state("wake");
                shown.clear(); // after the reply the state is shown again, also if recording fails
                replies::reply(paths, &cfg, bridge, &replies_text, &clips);
                while rx.try_recv().is_ok() {} // what the microphone heard during the reply goes
                ctl.start_recording("Uslyszalem Achaje.");
                if std::mem::take(&mut ctl.reset_free) {
                    free_rec.reset();
                    partial_text.clear();
                }
            }
            continue;
        }
        let t = now();
        let text = if !free_rec.accept(&chunk) {
            let partial = json(&free_rec.partial())["partial"].as_str().unwrap_or("").trim().to_string();
            if partial != partial_text {
                partial_text = partial.clone();
                partial_since = t;
                bridge.set_heard(&partial);
            }
            if !partial.is_empty() {
                ctl.last_speech = t;
            }
            // the quick way: the end word last and nothing new for end_stable seconds
            if partial.is_empty() || t - partial_since < end_stable || !ends_with(&partial, &ends, end_match) {
                continue;
            }
            free_rec.reset();
            partial
        } else {
            json(&free_rec.result())["text"].as_str().unwrap_or("").trim().to_string()
        };
        partial_text.clear();
        if text.is_empty() {
            continue;
        }
        ctl.last_speech = now();
        bridge.set_heard(&text);
        bridge.log(&format!("  slysze: {text}"));
        if ends_with(&text, &ends, end_match) {
            ctl.send();
        } else if cancel.contains(&text) {
            ctl.cancel();
        } else if clear.contains(&text) {
            ctl.new_conversation();
        }
    }
    Ok("stopped")
}
