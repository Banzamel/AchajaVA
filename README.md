<p align="center"><img src="docs/achaja.png" alt="Achaja logo" width="128"></p>

# Achaja

**A hands-free voice assistant for Windows, built on Claude Code.**

Say the wake word, speak your request, finish with the end word — and listen to the answer. *Achaja* runs [Claude Code](https://claude.com/claude-code) for you in a hidden window: it runs commands on your PC, **hands work to agents in your own projects** and tells you what they did, keeps your **mailbox clean**, and controls your **home** through Home Assistant. You never have to look at the screen.

**Setup takes about 10 minutes:** download the installer, click through a settings window, say "Achaja".

⭐ **[Star it](https://github.com/Banzamel/AchajaVA)** to follow new releases — installed copies update themselves.

## Is this for me?

Yes, if you want to talk to your computer the way you would talk to an assistant in the room — while cooking, walking around, or with your hands on something else. You need:

- **A Windows 10 or 11 PC** with a microphone and headphones or a speaker.
- **A Claude subscription** (Pro or Max) with [Claude Code](https://code.claude.com/docs/en/setup) signed in — dictation needs a claude.ai account.
- Optional: **Home Assistant** for the home, **IMAP mail accounts** for the mail cleaner, a **Google Cast speaker** for answers out loud in the room.

### Your first five minutes

1. Download **`AchajaSetup-<version>.exe`** from [Releases](https://github.com/Banzamel/AchajaVA/releases) and run it (SmartScreen: *More info → Run anyway* — the installer is not code-signed yet).
2. Click through the installer (it follows the language of Windows, which also becomes Achaja's first language — Polish or English); Achaja starts.
3. The settings window opens: download the speech model, choose the microphone and the output, add project folders, mail accounts and your Home Assistant address — every step except the model can be skipped and changed later.
4. Save, and Achaja starts listening (later: **Achaja** in the Start menu). Say **"Achaja"**, wait for *"Słucham"* and the beep, then say: *"what time is it in Tokyo — execute"*.
5. The answer is spoken. From here, just talk.

## What it does

- **Listens without a keyboard.** An offline wake-word listener (no audio leaves the PC while idle); the request itself is transcribed by Claude Code, so it understands you as well as Claude does.
- **Answers out loud.** Every reply ends with a short spoken summary, in headphones or on a Google Cast speaker. When Achaja asks a question, it listens for your answer by itself.
- **Delegates to project agents.** *"Have the agent for the shop fix the login"* starts a separate Claude Code session in that project's folder, with its own rules — Achaja tells you when it's done and what changed.
- **Keeps the mailbox clean.** Ads, spam and phishing leave the inbox on the server; a spoken report tells you what is important. It replies only after your yes.
- **Reminds you and works in the background on a schedule.** *“Remind me at three to call mum”*, *“every weekday at eight go through my mail and answer clients the way we agreed”*, *“every morning tell me what's in my calendar”*.
- **Shows your Claude account and limits.** Plan usage bars (5 hours, 7 days), the signed-in e-mail, every Claude Code session on the PC with its size, and a one-click clean-up.
- **Backs itself up.** Settings and data are zipped once a day into the `backups` folder next to the program.
- **Runs the home.** Lights, climate (including the heating/cooling mode), covers, media, live state — through Home Assistant.
- **Asks before anything that matters.** Pushing code, deleting, sending mail, locks, gates and alarms wait for your spoken *yes* — enforced in code, not just in the instructions.
- **Updates itself.** New releases download in the background and install when you close Achaja.

## What it costs

| What | Cost | Needed for |
|---|---|---|
| [Claude Pro or Max](https://claude.com/pricing) | $20, $100 or $200 a month | Everything. Max is better if Achaja and its agents work all day |
| Achaja | free (MIT) | — |
| Speech (Vosk wake word, Edge neural voices) | free | Listening and talking |
| [Home Assistant](https://www.home-assistant.io) | free | Only the home module |
| A Windows PC | the one you own | Running Achaja |

## How it works

```
[microphone] --- wake-word listener (Vosk, offline, control words only)
     │
     ├─► presses Space in the Claude Code window → Claude's dictation writes the prompt
     │
     └─► after the end word, presses Space again → the prompt submits itself
                     │
                     ▼
        Claude Code (the "Achaja" session)
         ├─ runs commands on the computer
         ├─ delegates tasks to project agents (separate claude sessions)
         └─ controls the home (Home Assistant over MCP) and a TV (ADB)
                     │
                     ▼
        Stop hook → [GŁOS] section of the answer → speech (Edge neural voices)
                     → headphones or a Google Cast speaker
```

The key design decision: **the prompt itself is transcribed by Claude Code**, not by a local model. The small offline model only picks up a handful of control words, so dictation quality stays exactly as good as Claude Code's, and the listener sends nothing to the network.

Keystrokes are written straight into the Claude Code console input buffer (`WriteConsoleInputW`), so **the window does not need focus** — you can keep working while Achaja listens.

### The Achaja window

Achaja is one native Windows program — **`Achaja.exe`** (Tauri + Rust, the window in React; no Python, no console windows). `Achaja.exe --settings` opens the settings window.

- **Conversation** — chat bubbles with your prompts and Achaja's answers (the spoken `[GŁOS]` part marked ♪; *Copy* under every answer, or select text), filled by the session hooks; type into the box and press Enter to send a message without speaking.
- **Terminal** — the hidden Claude console live, with keys for its choice dialogs (approvals, a new MCP server) when one waits.
- **Agents** — every project agent with live progress and its final report, plus *Stop*.
- **Plan** — reminders, timed commands and background jobs with their last report; *Run now*, *Delete*.
- **Claude** — the account, plan usage bars and every Claude Code session on the PC (see below).
- **Log** — what the listener hears and does.
- The state in its colour next to an animated avatar (listening, recording, working, speaking, waiting for approval), a **Talk** button (start / send a recording without the wake word) and **Pause listening** (which also releases the microphone, so Bluetooth headsets go back to stereo). The ☰ button (or F10) opens the Achaja menu.
- **Light and dark theme** — follows the Windows app mode by default (switches live); pick it in the settings (*General → Window theme*) or set `app.theme` to `auto`, `light` or `dark`. Both windows have the same custom title bar as the other Banzamel apps.
- The Claude Code session runs in a **hidden console**. *Achaja → Show / hide the Claude console* toggles it; with `app.show_console_on_prompt` it shows up by itself while Claude asks for a permission.
- Closing the window keeps Achaja in the notification area (tray icon: show, talk, pause, quit); *Quit* closes everything — the hidden session, the mail watcher, phone alerts, speech, agents and background jobs. If the session dies, it restarts and continues the same conversation.

- **Updates** — Achaja checks GitHub Releases (20 s after start, then every 15 minutes, or *Achaja → Check for updates*), downloads a newer signed version in the background and installs it **when you quit Achaja**, or right away with *Install now* (then it starts again). The update keeps your settings and data. Voice: "check for updates", "update yourself" (Achaja asks first).
- **Settings** — *Achaja → Settings…* opens the settings window, *Edit the configuration file* opens `config.json` in Notepad; saving either one restarts the listener with the new settings.

Settings for the window live under `app` in `config.json` (`hide_console`, `show_console_on_prompt`, `tray`, `close_to_tray`, `restart_closed_session`, `update_check`, `update_auto_download`, `update_install_on_quit`, `theme`).

## All features

- **Hands-free control** — wake word, dictation, end word, automatic submit.
- **Spoken answers** — Claude ends every reply with a short summary in a `[GŁOS]` ("voice") section, which is read aloud.
- **Conversation mode** — when the answer ends with a question, recording of your reply starts automatically.
- **Acknowledgements** — "I'm on it" right after submit, and "still working" during longer tasks.
- **Project agents** — "have the agent for <project> do X" starts a separate Claude Code session **inside that project's folder** (with its own `CLAUDE.md`, skills and settings), shows live progress in its own window, and returns a report that Achaja summarizes aloud.
- **Audio outputs** — headphones or a Google Cast / Google Home speaker ("answer through the speaker").
- **Mail (optional)** — a background watcher on any number of IMAP mailboxes: ads, spam and threats (phishing, malware) are cleaned out on the server, a spoken report says what is important and how much was removed, and Achaja can reply (after your yes).
- **Home (optional)** — Home Assistant through its official MCP integration: lights, climate, covers, media, live state.
- **Windows audio tool** — assign a microphone or speaker to a single program (e.g. `claude.exe`) without changing the system default devices.

## Requirements

- **Windows 10/11** (keystroke injection and per-app audio routing use Windows APIs).
- **[Claude Code](https://claude.com/claude-code)** signed in with a **claude.ai account** — dictation does not work with an API key or through Bedrock/Vertex.
- A **microphone** and an audio output; an internet connection for speech synthesis (Edge's online voices).
- Optional: Home Assistant 2025.2+ (the "Model Context Protocol Server" integration), a Google Cast speaker.

## Install

Download `AchajaSetup-<version>.exe` from [Releases](https://github.com/Banzamel/AchajaVA/releases) and run it. It:

- installs for the current user only (no administrator rights) into `C:\Banzamel\Achaja` (or the folder you choose — never inside your user profile); **everything Achaja writes stays in that folder** (`data\`), nothing goes to `%APPDATA%` or `%LOCALAPPDATA%`;
- replaces an installed Python version of Achaja (0.13 or older), keeping its folder, settings, mail accounts, Home Assistant token, plan and speech model — and Python Achaja updates itself into this version;
- adds "Achaja" and "Achaja – settings" to the Start menu and a normal uninstaller to *Apps & features* (it asks whether to delete your settings and data too).

On the first start the settings window walks you through the steps: general (language, model, theme, *start with Windows*), voice (microphone and output picked from your devices, Google Cast speaker search), project folders, mail accounts (add / edit / *Test*) and mail rules, Home Assistant address and token. If the speech model of your language is missing it offers to download it (~50 MB). Every step can be changed later.

The file is not code-signed, so Windows SmartScreen shows a warning on the first run (*More info → Run anyway*). Unattended install: `AchajaSetup.exe /S` (`/P` shows only the progress).

### Language

There is **one** `config.json` (in `data\`). It carries a `language` key and a `languages` block with the control words, speech model, voice and acknowledgement phrases for each language; the settings window switches it (and downloads the matching Vosk model). The assistant rules are one file for every language:

| | Polski (default) | English |
|---|---|---|
| Wake | "Achaja" (heard as `aha ja`) | "Achaja" (heard as `a kaya`) |
| Submit | „wykonaj” | „execute” |
| Cancel / reset | „anuluj” / „nowa rozmowa” | „cancel” / „new conversation” |
| Voice | `pl-PL-ZofiaNeural` | `en-US-JennyNeural` |
| Spoken-summary marker | `[GŁOS]` | `[VOICE]` |

Any other language works too: add a block to `languages` with a model from [alphacephei.com/vosk/models](https://alphacephei.com/vosk/models) (unpacked into `datamodels`) and control words **the model actually knows** — an invented name is never recognized, so use a phrase that sounds like it (Polish "Achaja" is matched as `aha ja`). Set its `tts_voice` too — the rules file needs no translation.

Note that only the control words depend on this model. The prompt itself is transcribed by Claude Code, and Claude replies in the language you spoke.

## Usage

| You say | What happens |
|---|---|
| **wake word** → "Słucham" (`voice.wake_replies`) + *beep* | Claude Code starts recording your prompt (the reply is never recorded) |
| your command… **end word** + a short pause | prompt submitted; Achaja confirms: "I'm on it" |
| **"cancel"** | the recording is discarded |
| **"new conversation"** | clears the context (`/clear`) |
| wake word while Achaja is speaking | stops the speech and starts recording |
| a question from Achaja | recording of your answer starts automatically |


Example commands:

- "open the calculator and add the square roots of two and three"
- "look up how much X costs"
- "have the agent for **vision** check why login is broken"
- "what are the agents doing?", "what did the taxi agent do?", "stop the vision agent"
- "answer through the speaker", "back to headphones"
- "turn on the light in the office", "set the AC to 22 degrees" (Home Assistant)
- "remind me in half an hour to take the laundry out", "every weekday at eight tell me what's new in my mail", "what have I scheduled?"

## Project agents

```
Achaja ──► achaja agent run <project> ──► claude -p  (cwd = the project folder)
                                        │  its own .claude/CLAUDE.md, skills, permissions
                                        ├─ an "Agent: <project>" window with live progress
                                        └─ report → Achaja summarizes it aloud
```

- Project names are matched fuzzily (dictation mangles them) plus aliases from `config.json`.
- Each project keeps a continuous conversation (`--resume`); `--new` starts over.
- Agents run in the `auto` permission mode; risky actions are blocked and come back to you as a question.
- Commands: `run`, `status`, `last <project>`, `stop <project>`, `list`.

## Home Assistant (optional)

1. In HA add the **Model Context Protocol Server** integration (control: Assist).
2. Create a long-lived access token and enter it on the *Home Assistant* page of the settings window (it is kept in `data/state/ha_token.txt`, never shown again).
3. Enter your HA address there too; the settings window writes `data/.mcp.json` for the session. The token reaches the session as `${HA_TOKEN}` and is never stored in a config file.
4. In HA expose the devices Achaja may control (Settings → Voice assistants → Expose).

The MCP integration cannot change an air conditioner's **HVAC mode** (heat, cool, auto, dry, fan only) — after `HassTurnOn` the device picks the mode itself. For that and other services MCP lacks, Achaja uses `achaja.cmd ha` (Home Assistant REST API; address from `config.json` → `home_assistant.url`, token from `state/ha_token.txt`, never printed):

```
achaja.cmd ha climate "living room" --mode heat --temp 23   # sets, then re-reads and compares
achaja.cmd ha state climate.living_room                      # state and attributes (hvac_modes...)
achaja.cmd ha call light turn_on '{"entity_id": "light.desk"}'
```

Modes the device does not support are refused with the list of supported ones; `--dry-run` only prints the requests. Locks, alarms, gates and garages are refused without `--confirmed` — Achaja asks you aloud first.

`achaja.cmd ha areas` also helps tidy the home: `dump` lists areas and entities with no room assigned, `apply plan.json` assigns devices to areas — Assist then understands commands like "turn off the lights in the bedroom" much better.

## Mail (optional)

```
IMAP server ──IDLE──► achaja.mail.watch (background, one connection per account)
                        ├─ rules: server spam/virus headers, executable or macro attachments,
                        │         Windows Defender scan, important / ignored senders
                        ├─ the rest: claude -p --model haiku with NO tools → important | normal | ad | spam | threat
                        ├─ ad / spam / threat → Achaja's folders on the server (purged after delete_after_days) or deleted
                        └─ spoken report when you are not talking to Achaja: important mail with sender and gist,
                           the rest as numbers ("I removed 30 ads and 2 spam messages"); also at digest_at times
```

1. Add the accounts in the settings window (or in `mail.accounts` in `config.json` — an array: host, port, `security` ssl/starttls, user, password; any shared setting can be overridden per account) and turn mail on.
2. Check the connection: `achaja.cmd mail test`.
3. Start Achaja (the watcher starts with it) or run `achaja mail start`.

Mail is never marked as read. The first start reviews existing mail (all unread plus the last `scan_existing_days` days) and reports once at the end; `achaja mail rescan` repeats it on demand. Everything happens on the server: moved mail is purged `delete_after_days` (7 by default) after the move, "that wasn't spam" brings it back (`achaja mail move <n> inbox`), and locally only metadata from the last `log_days` days is kept. Replies (`achaja mail reply <n> --text-file F`, or `--draft`) go out over SMTP, get a copy in Sent and mark the original as answered; Achaja reads the draft aloud and sends it only after your yes. E-mail content only ever reaches a model with no tools and comes back as a category, so instructions hidden in an e-mail cannot trigger actions. For viruses the best first line of defence is the mail server itself (e.g. Rspamd + ClamAV) — Achaja trusts its `X-Spam-*` / `X-Virus-*` headers.

## Reminders, scheduled tasks and background jobs

```
achaja.cmd plan add --say "Call mum." --at 15:00
achaja.cmd plan add --prompt "what's new in my mail" --weekdays 08:00
achaja.cmd plan list
achaja.cmd plan remove 3
```

Achaja does this for you when you ask by voice. When: `--in 30m`, `--at 15:30` or `--at "2026-11-20 09:00"`, `--daily 08:00`, `--weekdays 08:00`, `--weekly "fri 16:00"`. A `--say` reminder is spoken on time (after you finish recording); a `--prompt` task reaches Achaja as a message marked `[ZAPLANOWANE]` when she is idle — it can delegate to agents or check the mail, but it never counts as your approval for gated actions. The listener in the Achaja window runs the schedule (`state/plan.json`).

**Background jobs** run on their own, in a separate headless Claude session in Achaja's folder (`achaja.plan.jobs`), with Achaja's rules, mail and Home Assistant:

```
achaja.cmd plan add --job "go through new mail and answer clients" --name "Morning mail" ^
    --rules "only answer questions about appointment dates, in a friendly tone; never promise prices" --weekdays 08:00
achaja.cmd plan run 4        # run it now
achaja.cmd jobs last 4       # its last report
```

`--rules` are the standing rules you agreed with Achaja. By default a job only prepares drafts and tells you what is waiting for your yes; `--allow mail` (also `mcp`, `delete`, `push`, `deploy`, `home`) lets it act on its own in that category — adding it needs your spoken *yes*, and the gate lets a job do nothing outside it. The report lands in the chat and the **Plan** tab; Achaja speaks its summary when you are not recording, or stays silent when nothing matters. A job running longer than `jobs.timeout_minutes` (30) is stopped.

## Claude account, usage and sessions

The **Claude** tab shows the signed-in account (e-mail, plan), usage bars for each plan window with the time it resets (the same reading as `/usage`, from `api.anthropic.com/api/oauth/usage` with Claude Code's own token — never printed or stored), and every Claude Code session on the PC grouped by project with its size, last activity and whether it is active. *Delete* removes a session or all sessions of a project, *Clean older than 30 days* tidies old sessions, debug logs and shell snapshots — only Claude Code's records in `~/.claude`, never project files, and never an active session or Achaja's current conversation. By voice: *“how much of my limit is left?”*, *“how much space do Claude sessions take?”* (`achaja claude summary | usage | projects | sessions <project> | clean --days 30`).

## Backups

Once a day (`backup.every_hours`) Achaja zips `config.json`, `.mcp.json`, local settings and `state\` into `backups\` in the Achaja folder (`backup.folder` to choose another place), keeping the newest 10. *Achaja → Back up now* or *“make a backup”* does it on demand; `achaja.cmd backup restore <file>` brings one back (after your yes, saving the current state first). Backups contain mail passwords and tokens — keep them private.

## Where things are

```
<install folder>\            e.g. C:\Banzamel\Achaja
  Achaja.exe                 the program: the window, and `Achaja.exe <command>` for the session's tools and hooks
  resources\claude\          Achaja's rules and skills for Claude Code, templates
  resources\vosk\            the offline recognizer of the control words (libvosk)
  data\                      everything Achaja writes (the Claude session works here too)
    config.json  .mcp.json   your settings (git-ignored in a checkout, never published)
    .claude\                 rules and skills (refreshed on start), hooks settings, your settings.local.json / CLAUDE.local.md
    achaja.cmd               one entry to the tools: achaja.cmd plan list, achaja.cmd mail summary…
    state\                   tokens, logs, plan, mail log, agent sessions, speech cache
    models\                  the Vosk speech model
    webview\                 the window's browser profile
  backups\                   daily backups
```

`Achaja.exe help` (or `achaja.cmd help`) lists the commands: `plan`, `jobs`, `mail`, `gmail`, `phone`, `ha`, `agent`, `claude`, `backup`, `update`, `output`, `speak`, `devices`, `audio` (a microphone or speaker for one program, e.g. `claude.exe`, without changing the system defaults).

## Configuration

Everything lives in `data\config.json` (created from the template on the first start): microphone, control words and their sensitivity, the model and effort of the Achaja session, voice and audio outputs, acknowledgement phrases, project folders and aliases, Home Assistant, mail accounts. Most commonly tuned:

| Key | Meaning |
|---|---|
| `wake.end_match`, `wake.end_stable_seconds` | end-word sensitivity and delay |
| `wake.auto_listen_after_question` | conversation mode |
| `achaja.model`, `achaja.effort` | speed vs. quality of answers |
| `voice.outputs`, `voice.default_output` | headphones, Cast speaker |
| `voice.ack_phrases`, `voice.working_phrases` | acknowledgements and reminders |
| `mail.accounts`, `mail.important_hint`, `mail.quiet_hours` | mailboxes, your own "what is important" rules, silent hours |

## How security works

Achaja runs on your PC and controls it by voice, so the rules are strict and enforced in code where possible, not only in the assistant's instructions.

- **Local listening.** The wake-word model runs offline (Vosk). Only your dictated request goes to Claude Code's dictation, and the spoken text to the speech service (Microsoft Edge read-aloud voices).
- **Your yes, out loud — enforced by a gate.** Irreversible or outward-facing actions (git push, deleting files, sending mail, deploys, shutting down, locks and gates, installing an update, MCP tools that send or delete) are stopped by a hook (`Achaja.exe hook gate`) unless the turn started with your own *yes*. The approval lasts one turn; text from a mail, a page, an agent's report or a scheduled task can't give it.
- **Background jobs get only the permissions you gave them.** A scheduled job can't ask you, so the gate lets it act only in the categories you allowed when creating it (`--allow`); everything else becomes a draft in its report.
- **Agents stay in their lane.** Project agents can't read or change Achaja's tokens, logs, settings or `config.json`, and an agent with no progress for 45 minutes is stopped.
- **Locks, gates, garages, alarms.** `achaja ha` refuses them without `--confirmed` — including commands aimed at a whole area, floor, device, label or script that would reach one. A hook (`Achaja.exe hook home-guard`) applies the same check to the Home Assistant MCP tools, and when it can't tell what a command covers, it asks.
- **Mail is data, never instructions.** E-mails are classified by a model with no tools; every field written by the sender (body, subject, sender name, attachment names) is neutralised, hidden Unicode characters included, before it reaches that model, mail read out in a conversation is wrapped in a random tag the sender can't guess, and the spoken report never says the wake word.
- **Secrets stay local.** `config.json`, `.mcp.json`, `state/` (tokens, logs, transcripts) and `.claude/settings*.json` are git-ignored; the Home Assistant token is never printed.
- **Only its own window.** The listener writes keystrokes only into the console of the Claude Code session it manages.

Found a security hole? Please [report it privately](https://github.com/Banzamel/AchajaVA/security/advisories/new), not in a public issue.

## FAQ

**Do I need to know how to code?**
No. The installer and the settings window do the setup; after that you only talk.

**Does it work on macOS or Linux?**
Not yet — keystroke injection and per-app audio routing use Windows APIs. Pull requests welcome.

**Is my audio sent anywhere?**
Not while Achaja waits for the wake word. After it, your request goes to Claude Code's dictation, and Achaja's spoken answers are synthesised by Microsoft Edge's online voices.

**Can it delete files, send mail or open my gate on its own?**
No. Those actions wait for your spoken yes; locks, gates, garages and alarms are refused in code until you confirm.

**What languages does it speak?**
Polish and English out of the box; any language with a Vosk model can be added in `config.json`.

**How do I get new features?**
Installed copies check GitHub Releases and update themselves when you close the window. Say *"check for updates"* any time.

**Is this an Anthropic product?**
No. It's an independent open source project built on Claude Code.

## Troubleshooting

| Symptom | What to check |
|---|---|
| No beep after the wake word | the *Log* tab — its `czuwanie:` (idle) lines show what was heard; tune `wake.phrases` |
| Beep works but Claude doesn't record | the prompt input must be empty; Claude Code must be in `voice tap` mode |
| The end word doesn't submit | the *Log* tab shows what it heard — lower `wake.end_match` |
| The prompt submits too early | raise `wake.end_match` or `wake.end_stable_seconds` |
| No speech is heard | `voice.outputs`, `data\state\speak_error.txt`; with Bluetooth headsets pick the "headset" (HFP) output |
| An agent doesn't answer | the *Agents* tab, `data\state\agents\<project>.log` |
| A console window still shows up | Windows Terminal set as the default terminal may ignore the hidden start; set *Windows Console Host* in Terminal → Settings → Startup → Default terminal application |

## Limitations

- Windows only (keystroke injection and per-app audio routing use Windows APIs).
- The screen must not be locked (a sleeping monitor is fine).
- Dictation requires a claude.ai account in Claude Code.
- The small Vosk model confuses similar words — that's why control words are matched fuzzily while the prompt itself is transcribed by Claude.

Pull requests welcome: other operating systems, other speech engines, new voice satellites.

## Tech stack

- **Tauri 2** with a **Rust** native side: the wake-word listener (cpal microphone capture, Vosk loaded at run time), speech (Edge's online neural voices over WebSocket, rodio playback), Google Cast (Cast v2 over TLS, mDNS discovery), the hidden Claude console (Win32 console API), IMAP/SMTP (async-imap, lettre, mail-parser), Home Assistant (reqwest, WebSocket), the plan, agents and hooks. All logic is in the crate `achaja-core` (`src-tauri/core`).
- **React 19 + TypeScript** (Vite) for both windows; tests on **Vitest** and `cargo test`.
- **NSIS** installer (Tauri's template with hooks for the install folder and the move from the Python version), signed updates through `tauri-plugin-updater`.

## Building

Requirements: Node.js 22 and Docker (no Rust or MSVC on the host). The image `tauri-build` (`docker/Dockerfile`: Rust 1.90, cargo-xwin, NSIS, clang/lld, Node 22) cross-compiles the Windows installer:

```bash
docker build -t tauri-build docker      # once
npm ci
npm test                                # Vitest
npm run dist                            # tests, cargo test of the core, installer -> release/AchajaSetup-<version>.exe
```

With Rust and MSVC on Windows, `npm run fetch-vosk` and `npm start` run the app in development; set `ACHAJA_DATA` to a test folder so your real data stays untouched.

### Publishing a release

Write the changes under `## [Unreleased]` in [CHANGELOG.md](CHANGELOG.md), then:

```bash
git checkout main && git pull
npm run release:minor                   # or release:patch / release:major: bumps, stamps the changelog, commits and tags
git push origin main --follow-tags
```

The tag starts the *Build & release* workflow: tests, the signed installer (`TAURI_SIGNING_PRIVATE_KEY` repository secret) and a GitHub release with `AchajaSetup-<version>.exe` and `latest.json`, described from the changelog. Installed copies — also the Python versions — find it on their next check.

## Who made this

Rafał Polak ([@Banzamel](https://github.com/Banzamel)). Questions and ideas: [open an issue](https://github.com/Banzamel/AchajaVA/issues).

## License

MIT — see [LICENSE](LICENSE).
