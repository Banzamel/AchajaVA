# Changelog

What changed in each version of Achaja. The newest version is on top; `npm version` moves the
"Unreleased" notes into a version section, and the release workflow publishes that section as the
GitHub release description.

## [Unreleased]

### Phone alerts from every mailbox, calendar reminders

- **Important mail on the phone works without the Gmail connector.** The mail accounts from *Settings → Mail* now reach the phone on their own, also when the mail watcher is off: Achaja reads new mail without marking it read, sorts it with the same small model and moves nothing. Gmail through the claude.ai connector stays a separate source — use either or both.
- **Google Calendar reminders on the phone** a set number of minutes before each event (15 by default), through the claude.ai Google Calendar connector. Without the connector the option is greyed out in the settings, which now show the status of both connectors.
- **Mail work is counted too:** sorting mail and the phone checks of Gmail, the mailboxes and the calendar appear in the token statistics.
- Fixed: a voice command's tokens were not recorded (the Stop hook ran before Claude Code had written the answer). The response time now comes from Claude Code's own record of the turn.
- Fixed: "1 uruchomień" in the spoken statistics.
- Fixed: in the narrow window the settings page did not scroll and its Close / Save buttons slid out of view.

## [0.15.0] - 2026-11-14

### Tokens, cost and limits

- **Token statistics in the Claude tab** (*Statistics* next to *Sessions*): today and the last 7 days — commands and jobs, failures, tokens (input, output, cache), the estimated cost and the response time (average and p90), a 14-day cost trend, the split by kind (talking to Achaja, background jobs, project agents) and by model with the session's effort, and the most expensive sessions, jobs and agents of the week. Achaja counts every voice command (from the session's transcript), every background job and every project agent run into `data\state	okens.jsonl`.
- **The cost is an estimate in USD at API prices** — the Claude plan is a subscription. The November 2026 prices are built in; `pricing` in `config.json` changes or adds them.
- **Ask by voice:** "how many tokens today?" — the new `achaja claude stats` (`--details` for the full table).
- **A spoken warning at 95 %** of the plan's 5-hour or 7-day limit (once per window; `limits.warn_at`).
- **Recurring background jobs wait** while the 5-hour or 7-day limit is past 95 % (`limits.pause_jobs_at`) or today's estimated cost is past `limits.daily_cost_usd` (off by default); the Plan tab shows the skipped run. Voice commands are never blocked.
- **Spending limit per run:** a background job stops past $5 at API prices (`jobs.max_budget_usd`), a project agent past `agents.max_budget_usd` (off by default).
- **Another try after a passing failure:** a background job that failed because the API was overloaded runs once more after 5 minutes (`jobs.retry_minutes`); Edge speech, Home Assistant reads and connecting to the mail server try again after a dropped connection or a server error (never a Home Assistant action or sending mail, which could happen twice).

## [0.14.0] - 2026-11-08

### Achaja rebuilt as one native Windows program

- **No Python any more.** Achaja is now a single program (Tauri + Rust, the window in React): the wake-word listener, speech, the Google Cast speaker, the mail watcher, Home Assistant, project agents, the plan and background jobs all run natively. Starting is faster, there is no `.venv`, no portable Python and no console flicker.
- **Everything in one folder.** The program and all its data live in the install folder (`data\` inside it: settings, mail log, tokens, plan, the speech model, the window's browser profile). Nothing is written to `%APPDATA%`, `%LOCALAPPDATA%` or your profile. New installs go to `C:\Banzamel\Achaja`; an existing Achaja keeps its folder (unless it was inside the user profile, then it moves to `C:\Banzamel\Achaja`).
- **Your settings come along.** The installer copies `config.json`, mail accounts, the Home Assistant token, the plan, agent sessions and the speech model of the Python version into `data\`, removes the old version, and only then tidies what it left. The running Python Achaja updates itself straight into this version.
- **One window for everything.** Conversation, Terminal, Agents, Plan, Claude and Log tabs, the animated avatar and the processor-traces band, light and dark theme, the tray icon — and the settings are now a view of the same window (the ⚙ next to the tabs, *Settings* in the menu or the tray, the *Achaja – settings* shortcut), with the same sections and the step-by-step wizard on the first start. No separate settings window any more. The window has its own title bar like the other Banzamel apps.
- **Starting with Windows** is a switch in *Settings → General* (it was an installer option).
- **Updates** are signed and come from GitHub Releases; a downloaded version still installs when you quit Achaja, or at once with *Install now*.
- **Backups** now go to `backups\` in the Achaja folder by default (set `backup.folder` to keep using another place).
- The Claude session now works in Achaja's `data` folder, so the first start after the update begins a new conversation. The tools are called through `data\achaja.cmd` (`achaja.cmd plan list`…), the session hooks through `Achaja.exe hook …`.
- Removed: the separate console mode (`start-achaja.cmd`) and the "Agent: <project>" console windows — the Log tab shows what that console printed, the Terminal tab shows Claude's console live (and *Console window* still shows the real one), and the Agents tab shows each agent's console live, one panel per agent.

## [0.13.2] - 2026-11-06

*Quitting closes everything*

- **Quitting Achaja now closes everything.** The Claude session, the mail watcher, phone notifications, speech, project agents and background jobs all stop with the app, so nothing keeps running in the background. Closing the window with the tray option on still only hides it; use Quit. Mail checks and phone notifications resume on the next start.
- **Fixed:** the first phone notification check after installing failed because its state folder did not exist yet.

## [0.13.1] - 2026-11-06

*Phone notification settings*

- **Choose how your phone is notified.** The Home Assistant page of the settings has a new section, *Important mail on your phone*: turn it on or off, pick the phone (*Find phones* lists the ones with the HA app), choose push and voice, push only or voice only, decide whether voice ignores silent mode, and set the voice hours.

## [0.13.0] - 2026-11-06

*Important mail on your phone, Remote Control*

- **New: important mail on your phone.** Every five minutes Achaja checks Gmail through the Gmail connector of your Claude account and sends a push plus a spoken message through the Home Assistant mobile app — only for what matters: personal replies, interview invitations, recruitment tasks, deadlines, rejections and decisions. Automated mail, ads and confirmations stay silent.
- **Only with the connector.** Without Gmail connected in Claude (Settings → Connectors) the feature does nothing. Mailboxes configured in Achaja are optional and are included only when they are set up.
- **Read-only by design.** The checking session sees two reading tools and nothing else: no shell, no files, no sending. The phone gets a short note (sender, matter, deadline) with links, addresses and long numbers removed. Achaja never replies, deletes, marks mail or opens links here.
- **Speaks through silent mode.** Spoken messages use the alarm stream by default and only inside `voice_hours` (08:00–22:00); outside them you get just the push. Set it all up in `phone_alerts` in `config.json`; test it with `python -m achaja phone test`.
- **Remote Control.** The main session now starts named and with Remote Control, so you can drive it from your phone or claude.ai/code. Turn it off with `achaja.remote_control: false`.

## [0.12.0] - 2026-11-05

*Gmail clean-up through the Claude connector*

- **New: Gmail clean-up.** Say *"clean up my Gmail"* and Achaja reviews the inbox through the Gmail connector of your Claude account: spam and phishing are marked as spam right away, ads and pointless newsletters are collected for removal, and important messages are summarised. Then she asks *"Delete forty ads and unsubscribe you from six lists?"* — nothing goes to the trash without your *yes*, and starred threads or threads you replied to are never touched. Deleted mail goes to the Gmail trash (recoverable for 30 days).
- **Unsubscribing from mailing lists** — only from legitimate senders, never from spam (that would confirm your address). One-click unsubscribe (RFC 8058) runs automatically; senders that only accept an email get one after your approval; senders with just a web page are listed for you, links are never opened.
- **Remembers your decisions:** *"always keep mail from X"* / *"always delete mail from X"*.
- **Daily clean-up in the background** as a scheduled job — by default it only marks spam and leaves deleting and unsubscribing for your decision.
- Works only when Gmail is connected in Claude (Settings → Connectors); without it Achaja tells you how to connect it and does nothing else.

## [0.11.2] - 2026-11-04

*delete several Claude sessions at once*

- **Claude tab: select several sessions** with Ctrl+click or Shift+click and delete them together — one confirmation for all, *Delete (n)* shows how many. The Delete key and a right-click menu work too. Active sessions and Achaja's current conversation are skipped with a note; the rest are still deleted.
- Fixed: the *Delete* button disappeared in a short window (the session list pushed it out). The buttons now stay at the bottom.
- Fits the default window size: the update banner's button is never cut off (the text wraps beside it), and while recording *Pause listening* steps aside to make room for *Cancel*.

## [0.11.1] - 2026-11-04

*clearer hint and a separator under the tabs*

- The *Say "Achaja"* hint is centred and softened (faded background and frame, italic text), so it reads as information rather than a button.
- A neon circuit-style line under the tabs separates the menu from the conversation.
- Includes the new look from 0.11.0: graphite background, neon processor traces, the AI-core avatar.

## [0.11.0] - 2026-11-04

*new look: neon circuit traces*

- **New look.** Calm graphite background and light text that is easy on the eyes; neon appears only in thin lines, frames and small accents. The *Talk* button and your chat bubbles are dark with a neon frame instead of solid colour.
- **Circuit-board header.** Under Achaja's name runs a strip of processor-style traces — 45° bends, vias and a chip in the middle — in green, cyan and violet. While Achaja listens, thinks or speaks, data packets run along the traces.
- **New avatar: an AI core.** A chip with neon eyes; an equaliser for a mouth while speaking, data running along its traces while thinking. State colours: listening — magenta, thinking — violet, speaking — cyan, waiting — amber.
- Fixed the *Say "Achaja"* hint under the Talk button: its rounded ends were clipped and it stuck to the tabs.
- New icon and installer graphics in the same style.

## [0.10.2] - 2026-11-04

*listening works again after the 0.10.1 update*

- **Fixed: Achaja stopped listening after updating to 0.10.1.** One import still used the old module name, so the wake word, the Claude session and the mail watcher did not start. Please update — existing installs do it by themselves.
- After an update, Achaja no longer re-attaches to the Claude session started by the previous version (it had the old rules and hooks); it closes it and continues the same conversation in a fresh session.

## [0.10.1] - 2026-11-04

*tidier project layout*

- **Tidier project layout.** Achaja's code moved from `.claude/scripts` to a Python package in `src/achaja/` (core, voice, ui, hooks, agents, mail, home, plan, claude) with one entry point, `python -m achaja <command>`; `.claude/` now holds only what Claude Code reads — the rules, skills and settings. Updating cleans up the old folder and rewrites the hooks by itself; a git checkout needs `install.ps1` run once.
- The confirmation gate recognises both the new (`python -m achaja mail reply …`) and the old way of calling Achaja's tools.
- Fixed: the 0.10.0 installer was not attached to its release (a race between two release workflows).
- Includes everything from 0.10.0: background jobs on a schedule, the Plan and Claude tabs, and automatic backups.

## [0.10.0] - 2026-11-04

*﻿# Achaja 0.10.0 — background jobs, Claude usage and sessions, backups*

- **New: background jobs on a schedule.** *“Every weekday at eight go through my mail and answer clients the way we agreed”*, *“every morning tell me what's in my calendar”*. A job runs on its own in a separate Claude session, with the rules you agreed with Achaja. By default it only prepares drafts and tells you what is waiting for your yes; it acts by itself only in what you allowed when creating it (for example sending replies) — and the confirmation gate holds it to that. The report lands in the chat and the new **Plan** tab, and Achaja speaks the summary (or stays quiet when nothing matters).
- **New: Claude tab.** The signed-in account and plan, usage bars for the 5-hour and 7-day windows with the time they reset, and every Claude Code session on the PC grouped by project, with its size and whether it is active. Delete a session or a whole project's sessions, or clean up everything older than 30 days — only Claude Code's own records, never your project files. By voice: *“how much of my limit is left?”*.
- **New: Plan tab.** Reminders, scheduled commands and background jobs in one list, with each job's last report and *Run now*.
- **New: automatic backups.** Settings and data are zipped once a day to `Documents\Achaja - kopie` (the newest 10 are kept); *Achaja → Back up now* any time, restore by voice after your yes.
- The window opens a little wider to fit the new tabs.

## [0.9.0] - 2026-11-03

*﻿# Achaja 0.9.0 — reminders, a confirmation gate, safer agents*

- **New: reminders and scheduled tasks.** *“Remind me at three to call mum”*, *“every weekday at eight tell me what's in my mail”*, *“on Friday have the shop agent check the error log”*. Achaja speaks a reminder on time (it waits until you finish talking) and runs a scheduled task when it is idle. *“What have I scheduled?”* lists them, *“delete reminder 3”* removes one. A reminder missed while the PC was off is spoken at the next start.
- **New: the confirmation gate is enforced in code.** Pushing code, deleting files, sending mail, deploys, shutting the PC down, locks and gates, installing an update and MCP tools that send or delete now run only in a turn that started with your own *“yes”* — not just because the instructions say so. Text from a mail, a web page or an agent's report can't approve anything, and neither can a scheduled task.
- **New: a stuck agent stops itself.** A project agent that shows no progress for 45 minutes (`agents.hang_minutes`) is stopped and Achaja tells you it hung, instead of waiting forever. Agents also can no longer read or change Achaja's own files (tokens and logs in `state/`, settings, `config.json`).
- **New: no silent answers.** If a reply comes without its spoken summary, Achaja adds one before it speaks; an empty reply is announced (*“I didn't get an answer”*) instead of silence.
- **New: “what's new” after an update.** The first start of a new version says so and lists the changes in the chat window.
- Includes the security fixes from [0.8.3](https://github.com/Banzamel/AchajaVA/releases/tag/v0.8.3); the mail fence now also removes hidden Unicode characters (text direction controls, zero-width and tag characters).

## [0.8.3] - 2026-11-03

*﻿# Achaja 0.8.3 — door and gate lock, mail fence*

- **Fixed: Home Assistant confirmation for locks, gates and garages could be skipped** (reported privately by @mattabate — thank you). A command aimed at a whole area, floor, device or label (instead of a single entity), `entity_id: all`, a comma-separated list or a Home Assistant script went through without asking. Now `ha.py` expands every target to the entities it covers and asks first if any of them is a lock, alarm, gate or garage — and asks when it can't tell.
- **New: the same guard for the Home Assistant MCP tools.** Before, only Achaja's instructions protected *“turn on everything in the garage”*. A hook now checks `HassTurnOn`, `HassTurnOff`, `HassToggle` and `HassSetPosition` (and scripts exposed as tools) and stops them when they would reach a lock or a gate — Achaja asks you, and after *“yes”* does it with confirmation.
- **Fixed: the mail fence protected only the message body.** A subject or sender name containing `</email>` could slip instructions to the mail classifier. Every field written by the sender is now neutralised; a mail read aloud in conversation is wrapped in a random tag the sender can't guess; and the spoken mail report never says the wake word (a mail couldn't wake Achaja through the speaker).
- Updating now refreshes Achaja's Claude Code settings (needed for the new hook) and keeps your language.

## [0.8.2] - 2026-11-02

*﻿# Achaja 0.8.2 — restored mail stays restored*

- **Fixed: a mail you restored from spam went straight back to spam** ([#3](https://github.com/Banzamel/AchajaVA/issues/3), thanks @mattabate). When you said *“that wasn't spam”*, Achaja moved the message back to the inbox — where the server gives it a new number, so the mail watcher took it for new mail, classified it again and the same rule sent it back to spam seconds later. Now the watcher checks its log first: a message you corrected keeps your choice and is not judged again. The same goes for moving a mail to *important* and for dragging a message back to the inbox in your mail program.

## [0.8.1] - 2026-10-29

*pausing frees your headphones*

- **Fixed: Bluetooth headphones stayed in “headset” mode while listening was paused.** Pausing only ignored the sound, the microphone stayed open — and while any program holds a Bluetooth headset's microphone, Windows keeps it in hands-free mode, so YouTube, music and games played quietly or not at all. Now *Pause listening* closes the microphone completely and *Resume* opens it again. Pressing *Talk* while paused opens it just for that recording. If the microphone cannot be opened (headphones disconnected), Achaja tries again a few seconds later.

## [0.8.0] - 2026-10-29

*a Slavic warrior look, air-conditioning modes, clearer settings*

### A new look

- **Colours of a Slavic warrior maiden** — crimson and gold on dark oak at night, on linen by day; a new crimson app icon and installer images.
- **Achaja's avatar is a warrior maiden** with golden braids, a crimson headband with a golden diamond and a flower wreath. She still shows what she is doing: sound waves while listening, golden sparks while thinking, a moving mouth while speaking, closed eyes when paused.
- **An ornamental line with a jewel** under the header (and in the settings): the jewel glows in the colour of the current state and a glint runs along the line while Achaja listens, thinks or speaks.

### Home Assistant

- **Air-conditioning mode can be changed.** Home Assistant's MCP integration only sets the temperature and turns devices on or off — after turning on, the device picked its own mode (e.g. *auto* instead of *heat*). The new `.claude/scripts/ha.py` uses the Home Assistant REST API: `ha.py climate "living room" --mode heat --temp 23` sets the mode and temperature, then re-reads the state and reports when the device ended up different from what was asked.
- `ha.py state` shows a device's state and attributes (e.g. which modes it supports), `ha.py call` runs other services MCP lacks (climate, light, switch, cover, fan, media player, vacuum, script).
- Modes the device does not support are refused with the list of supported ones; `--dry-run` only prints the requests.
- **Locks, alarms, gates and garages are refused without `--confirmed`** — Achaja asks you aloud first.
- The token is read only from `state/ha_token.txt` and never printed. Standard library only, no new dependencies.

### The window

- **Answers can be copied and selected** — a *Copy* link under every answer, and any fragment can be selected with the mouse and copied with Ctrl+C or the right-click menu.
- **Voice command hint** under the Talk button: while waiting it says which word wakes Achaja, while recording which words send, cancel or start over.
- **A readable app menu** with larger items and icons in the app's colours (instead of the small system menu).
- A **round Send button with a paper plane**, like in messengers — dimmed until you type something.
- **Agents tab**: *Stop* now really stops a working agent and says so; a finished agent shows ✓ with its finish time and can be **removed from the list** (its log goes to `state/agents/archive`). Previously the stop button silently did nothing for finished agents, which looked like a bug.
- **Terminal tab**: the arrow / Enter / Esc / 1-2-3 keys appear only while Claude is waiting for a choice; *Console window* stays.
- Buttons on cards and banners no longer have dark corners.

### Settings

- **Settings opened from the menu are no longer a wizard**: a side menu with icons (General, Voice and microphone, Project agents, Mail, Home Assistant) — open any section, change it and **Save changes**; the window stays open for the next section. **Close** (or Esc) leaves without saving. The step-by-step wizard stays only for the first setup after installation.
- **Language has its own window** (side menu → *Language*), shown by name (polski / English). If the speech model for that language is not installed, Achaja says so and does not allow the change — the listener could not start without it.
- **Drop-downs only allow choosing from the list** (microphone, output, model…) and have a new arrow.

## [0.7.2] - 2026-10-29

*the wake word works again*

- **Fixed: saying “Achaja” did nothing.** The wake phrases have several spellings of the same word (*aha ja*, *a ha ja*, *acha ja*). The speech model splits its confidence between them, so the differing part got only 0.5 and the 0.8 threshold rejected every wake-up. The threshold for the differing part is now divided by the number of variants; the shared part (*ja*) still needs full confidence, and loose matches such as *ha ja* are still rejected.
- **Desktop shortcut is created by default** — the installer option is now checked (updates keep your previous choice).

## [0.7.1] - 2026-10-29

*compact window, live terminal and an avatar*

### Listening that actually works

- **Choice dialogs in the hidden Claude console are detected.** A dialog such as *New MCP server found* used to block the hidden session: the wake word started "recording", nothing happened and after a moment Achaja went back to idle. Now Achaja does not press any key into a dialog, says that Claude is waiting for a choice, and opens the **Terminal** tab where you answer it.
- **The Achaja folder is marked as trusted in Claude Code** during installation (`setup/trust.py`, with a backup of `~/.claude.json`). Without it Claude Code ignored the project's MCP choices and asked about the Home Assistant server on every start.
- After pressing record Achaja **checks that Claude really shows REC**; if not, it tells you instead of pretending to listen.
- Messages typed in the chat box **no longer get stuck** in Claude's input field — Achaja waits until the text is in the field, sends Enter and repeats it when Claude ignored it.
- Fixed an invalid-handle error after reading the console: quitting closes the hidden session again, *Stop agent* works and the microphone is assigned to Claude.
- The Claude Code texts used to recognise recording, dialogs and work can be changed in `config.json` (`wake.screen_text`) when Claude Code changes its interface.

### A window like a phone screen

- **Compact layout** (440×780 by default): avatar, name and coloured status on top, a big **Talk** button below.
- **Animated Achaja avatar** — breathes and blinks while waiting, sound waves while listening, orbiting dots while thinking, a moving mouth while speaking, an orange glow when waiting for your decision.
- **Live strip** — while recording it shows the text Claude is transcribing; while working it shows what Claude is doing right now (e.g. *Reticulating… (4s)*) and the status counts the time (*Thinking… 0:42*).
- **Terminal tab** — a live view of the hidden Claude console including the last 300 lines of history, with ↑ ↓ Enter Esc 1 2 3 keys for its dialogs. Because of it the real console window no longer pops up on permission prompts by default (`app.show_console_on_prompt`).
- **Agents tab** — the agent list on top, the selected agent's progress below.

## [0.6.0] - 2026-10-20

*new look, light and dark theme*

### A fresh window

- **Light and dark theme** with one violet palette, rounded buttons, fields, tabs and scrollbars. By default the theme follows Windows and switches live; in dark mode the title bar is dark too. Choose it in the wizard (*Window appearance*) or with `app.theme` in `config.json`: `auto`, `light` or `dark`.
- **Conversation as chat bubbles** — yours on the right, Achaja's on the left, the spoken part marked ♪ in the same bubble. Right-click a bubble to copy it. An empty conversation shows a welcome screen, and the input box highlights when focused.
- **Pulsing status indicator** while Achaja is recording, working, speaking or waiting for approval. **Talk** is the main button; the menu moved under **☰** (or F10).
- **Agents on cards** — a working agent has a green dot and its running time, and the tab shows how many agents are working.
- **Log** with dimmed times and errors in red.
- The settings wizard has the same look, a *Step X of Y* header and a progress bar.

### Updates

- Achaja now checks GitHub Releases **every 15 minutes** (first check 20 seconds after start). A new version shows a full-width banner with an **Install now** button.

## [0.5.0] - 2026-10-14

*Achaja.exe, one window, automatic updates*

### Achaja is now a normal Windows program

- **`Achaja.exe`** with its own icon starts Achaja — **no console windows** anymore. The Claude Code session runs in a hidden console that shows up by itself only when Claude asks for your approval, and hides again afterwards.
- **One window** with three tabs: **Conversation** (your prompts and Achaja's answers, the spoken part marked ♪, and a box to type instead of talking), **Agents** (live progress and the final report of every project agent, *Stop agent*), **Log**. A status line shows listening / recording / working / speaking / waiting for approval, with **Talk** and **Pause listening** buttons.
- **Tray icon** — closing the window keeps Achaja running; *Quit* also closes the hidden session. If the session dies, it restarts and continues the same conversation.
- **Smoother listening** — keys and screen reads go to the hidden console from inside the app instead of starting a helper Python process every second.
- **Agents no longer open their own windows** — their progress is in the *Agents* tab.

### Automatic updates

- Achaja checks GitHub Releases 20 seconds after start and then every 6 hours (or *Achaja → Check for updates*), downloads the new installer in the background (size and SHA-256 checked) and **installs it when you quit Achaja** — or right away with *Install now*, after which it starts again. Modules, settings and data are kept.
- By voice: "check for updates", "update yourself" (Achaja asks first).

### Settings

- *Achaja → Settings…* opens the step-by-step wizard (also `Achaja.exe --settings`), now with update and window options; *Edit the configuration file* opens `config.json` in Notepad. Saving either one restarts the listener with the new settings.

## [0.4.0] - 2026-10-07

*listener in sync with Claude Code*

### A listener that knows what Claude Code is doing

- **Reads the Claude Code screen** — before pressing Space the listener checks the status bar for `● REC`, and it follows along when recording stops or starts without it. No more listener and Claude getting out of step.
- **Reliable submit and cancel** — the end word sends with Enter when Claude has already stopped recording; *cancel* / *new conversation* press Esc only while recording, so they never interrupt Achaja's work.
- **Fewer false wake-ups** — the wake word counts only in finished utterances, with a minimum confidence per word (`wake.wake_confidence`) and spoken on its own (`wake.wake_isolated`).
- **"Słucham" / "Yes?" after the wake word** (`voice.wake_replies`) — played instantly from a trimmed clip in memory; recording starts only after it, so the reply never ends up in your prompt.

[Unreleased]: https://github.com/Banzamel/AchajaVA/compare/v0.15.0...HEAD
[0.15.0]: https://github.com/Banzamel/AchajaVA/compare/v0.14.0...v0.15.0
[0.14.0]: https://github.com/Banzamel/AchajaVA/compare/v0.13.2...v0.14.0
[0.13.2]: https://github.com/Banzamel/AchajaVA/releases/tag/v0.13.2
[0.13.1]: https://github.com/Banzamel/AchajaVA/releases/tag/v0.13.1
[0.13.0]: https://github.com/Banzamel/AchajaVA/releases/tag/v0.13.0
[0.12.0]: https://github.com/Banzamel/AchajaVA/releases/tag/v0.12.0
[0.11.2]: https://github.com/Banzamel/AchajaVA/releases/tag/v0.11.2
[0.11.1]: https://github.com/Banzamel/AchajaVA/releases/tag/v0.11.1
[0.11.0]: https://github.com/Banzamel/AchajaVA/releases/tag/v0.11.0
[0.10.2]: https://github.com/Banzamel/AchajaVA/releases/tag/v0.10.2
[0.10.1]: https://github.com/Banzamel/AchajaVA/releases/tag/v0.10.1
[0.10.0]: https://github.com/Banzamel/AchajaVA/releases/tag/v0.10.0
[0.9.0]: https://github.com/Banzamel/AchajaVA/releases/tag/v0.9.0
[0.8.3]: https://github.com/Banzamel/AchajaVA/releases/tag/v0.8.3
[0.8.2]: https://github.com/Banzamel/AchajaVA/releases/tag/v0.8.2
[0.8.1]: https://github.com/Banzamel/AchajaVA/releases/tag/v0.8.1
[0.8.0]: https://github.com/Banzamel/AchajaVA/releases/tag/v0.8.0
[0.7.2]: https://github.com/Banzamel/AchajaVA/releases/tag/v0.7.2
[0.7.1]: https://github.com/Banzamel/AchajaVA/releases/tag/v0.7.1
[0.6.0]: https://github.com/Banzamel/AchajaVA/releases/tag/v0.6.0
[0.5.0]: https://github.com/Banzamel/AchajaVA/releases/tag/v0.5.0
[0.4.0]: https://github.com/Banzamel/AchajaVA/releases/tag/v0.4.0
