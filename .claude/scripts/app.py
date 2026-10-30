"""Achaja jako zwykly program okienkowy: jedno okno z rozmowa, agentami i dziennikiem.

Uruchamiaj przez pythonw (bez okna konsoli):
  .venv\\Scripts\\pythonw.exe .claude\\scripts\\app.py              nowa rozmowa
  .venv\\Scripts\\pythonw.exe .claude\\scripts\\app.py --continue   wznawia ostatnia rozmowe
  ... app.py --minimized                                         start w zasobniku (autostart)

Nasluch (listener.run) dziala w watku tej aplikacji, a sesja Claude Code w ukrytej konsoli -
nie widac zadnych okien terminala. Konsole mozna pokazac z menu (np. przy pytaniu o zgode
pokazuje sie sama). Rozmowe wypelniaja hooki sesji (state/chat.jsonl), agentow - state/agents/.
Program sam sprawdza wydania na GitHubie, pobiera nowa wersje w tle i instaluje ja na koncu
(przyciskiem albo przy zamknieciu) - update.py. Zmiana config.json restartuje nasluch.
"""
import argparse
import ctypes
import json
import os
import queue
import re
import subprocess
import threading
import time

import avatar
import ornament
import listener
import theme
import update
from theme import FONT, FONT_BOLD
from common import (AGENTS_STATE, CHAT_LOG, CREATE_NO_WINDOW, DETACHED_PROCESS, PYTHON, PYTHONW, ROOT, SCRIPTS,
                    STATE, __version__, load_config, pid_alive, read_json, single_instance, write_json)

MARKER = re.compile(r"^\s*\[(?:G[ŁL]OS|VOICE)\]\s*:?\s*", re.IGNORECASE | re.MULTILINE)
SHOW_FLAG = STATE / "app_show.json"  # druga kopia programu prosi pierwsza o pokazanie okna
WINDOW_FILE = STATE / "app_window.json"
ICON = ROOT / "setup" / "assets" / "achaja.ico"
CONFIG = ROOT / "config.json"
UPDATE_FIRST, UPDATE_EVERY = 20, 15 * 60  # sekundy: pierwsze sprawdzenie po starcie, potem co 15 minut
ACCENT = "#a82a2f"

TEXT = {
    "pl": {
        "chat": "Rozmowa", "agents": "Agenci", "log": "Dziennik", "send": "Wyślij", "talk": "Mów",
        "send_rec": "Wyślij nagranie", "cancel": "Anuluj", "pause": "Wstrzymaj nasłuch", "resume": "Wznów nasłuch",
        "menu": "Achaja", "new": "Nowa rozmowa", "repeat": "Powtórz ostatnią odpowiedź",
        "console": "Pokaż / ukryj konsolę Claude", "settings": "Ustawienia…", "restart": "Uruchom nasłuch ponownie",
        "quit": "Zakończ", "show": "Pokaż okno", "you": "Ty", "achaja": "Achaja",
        "st_idle": "Czuwam — powiedz słowo wybudzenia", "st_rec": "Nagrywam… zakończ słowem końcowym",
        "st_busy": "Pracuję…", "st_wait": "Czekam na Twoją zgodę — zakładka Terminal", "st_speak": "Mówię…",
        "st_paused": "Nasłuch wstrzymany", "st_start": "Uruchamiam…", "st_closed": "Sesja Claude została zamknięta",
        "st_error": "Błąd nasłuchu — zobacz Dziennik",
        "placeholder": "Napisz do Achai (Enter wysyła, Shift+Enter nowa linia)",
        "wait_note": "Claude pyta o zgodę — otworzyłam konsolę, odpowiedz tam albo w zakładce Terminal.",
        "stop_agent": "■  Przerwij", "agent_remove": "Usuń z listy", "agent_finished": "zakończył {}",
        "agent_stopping": "Przerywam agenta {}…", "working": "pracuje {} min", "no_agents": "Brak agentów.",
        "quit_ask": "Zakończyć Achaję? Nasłuch i ukryta sesja Claude zostaną zamknięte.",
        "already": "Achaja już działa w oknie konsoli (start-achaja.cmd).",
        "edit_config": "Edytuj plik konfiguracji", "check_update": "Sprawdź aktualizacje", "about": "O programie",
        "about_text": "Achaja {}\nGłosowa asystentka dla Claude Code.\n\ngithub.com/Banzamel/AchajaVA",
        "upd_downloading": "Pobieram nową wersję {}… {}%", "upd_ready": "Nowa wersja {} jest gotowa — zainstaluję ją przy zamknięciu Achai.",
        "upd_ready_manual": "Nowa wersja {} jest gotowa do instalacji.", "upd_install": "Zainstaluj teraz",
        "upd_source": "Na GitHubie jest nowa wersja {} (kopia z gita: git pull).", "upd_open": "Otwórz stronę",
        "upd_none": "Masz najnowszą wersję ({}).", "upd_error": "Nie udało się sprawdzić aktualizacji: {}",
        "upd_noasset": "Wydanie {} nie ma jeszcze instalatora — spróbuję później.",
        "config_changed": "Zmieniono ustawienia — uruchamiam nasłuch ponownie.",
        "copy": "Kopiuj", "copy_sel": "Kopiuj zaznaczenie", "copy_all": "Kopiuj całą odpowiedź",
        "copied": "Skopiowano", "empty_title": "Cześć, tu Achaja",
        "empty": "Powiedz słowo wybudzenia albo napisz wiadomość poniżej.",
        "agent_done": "zakończył pracę", "agent_pick": "Wybierz agenta z listy", "menu_tip": "Menu",
        "term": "Terminal", "st_wake": "Słucham…", "st_dialog": "Claude czeka na decyzję — zakładka Terminal",
        "busy_for": "Myślę… {}", "live_rec": "Mów… zakończ słowem „{}”",
        "dialog_note": "Claude czeka na decyzję na ekranie — odpowiedz w zakładce Terminal (strzałki, Enter, Esc).",
        "dialog_say": "Claude czeka na decyzję na ekranie, w zakładce Terminal.",
        "rec_failed": "Claude nie włączył nagrywania — zobacz zakładkę Terminal.",
        "term_empty": "Sesja Claude jeszcze nie działa.", "term_console": "Okno konsoli",
        "hint_idle": "🎙  Powiedz „{}”, żeby zacząć mówić",
        "hint_rec": "🎙  Wyślij: „{}”  ·  przerwij: „{}”  ·  od nowa: „{}”",
        "hint_dialog": "Claude czeka na wybór — odpowiedz w zakładce Terminal",
        "hint_paused": "Nasłuch wstrzymany — kliknij „Wznów nasłuch”", "hint_start": "Uruchamiam sesję Claude…",
    },
    "en": {
        "chat": "Conversation", "agents": "Agents", "log": "Log", "send": "Send", "talk": "Talk",
        "send_rec": "Send recording", "cancel": "Cancel", "pause": "Pause listening", "resume": "Resume listening",
        "menu": "Achaja", "new": "New conversation", "repeat": "Repeat the last answer",
        "console": "Show / hide the Claude console", "settings": "Settings…", "restart": "Restart the listener",
        "quit": "Quit", "show": "Show window", "you": "You", "achaja": "Achaja",
        "st_idle": "Listening for the wake word", "st_rec": "Recording… finish with the end word",
        "st_busy": "Working…", "st_wait": "Waiting for your approval — Terminal tab", "st_speak": "Speaking…",
        "st_paused": "Listening paused", "st_start": "Starting…", "st_closed": "The Claude session was closed",
        "st_error": "Listener error — see the Log",
        "placeholder": "Write to Achaja (Enter sends, Shift+Enter adds a line)",
        "wait_note": "Claude is asking for approval — I opened the console, answer there.",
        "stop_agent": "■  Stop", "agent_remove": "Remove from list", "agent_finished": "finished {}",
        "agent_stopping": "Stopping agent {}…", "working": "working {} min", "no_agents": "No agents.",
        "quit_ask": "Quit Achaja? The listener and the hidden Claude session will be closed.",
        "already": "Achaja is already running in a console window (start-achaja.cmd).",
        "edit_config": "Edit the configuration file", "check_update": "Check for updates", "about": "About",
        "about_text": "Achaja {}\nA voice assistant for Claude Code.\n\ngithub.com/Banzamel/AchajaVA",
        "upd_downloading": "Downloading the new version {}… {}%", "upd_ready": "Version {} is ready — it installs when you quit Achaja.",
        "upd_ready_manual": "Version {} is ready to install.", "upd_install": "Install now",
        "upd_source": "Version {} is on GitHub (git checkout: git pull).", "upd_open": "Open page",
        "upd_none": "You have the latest version ({}).", "upd_error": "Could not check for updates: {}",
        "upd_noasset": "Release {} has no installer yet — I will try later.",
        "config_changed": "Settings changed — restarting the listener.",
        "copy": "Copy", "copy_sel": "Copy selection", "copy_all": "Copy the whole answer",
        "copied": "Copied", "empty_title": "Hi, this is Achaja",
        "empty": "Say the wake word or type a message below.",
        "agent_done": "finished", "agent_pick": "Pick an agent from the list", "menu_tip": "Menu",
        "term": "Terminal", "st_wake": "Listening…", "st_dialog": "Claude is waiting for a choice — Terminal tab",
        "busy_for": "Thinking… {}", "live_rec": "Speak… finish with “{}”",
        "dialog_note": "Claude is waiting for a choice on screen — answer in the Terminal tab (arrows, Enter, Esc).",
        "dialog_say": "Claude is waiting for a choice on screen, in the Terminal tab.",
        "rec_failed": "Claude did not start recording — see the Terminal tab.",
        "term_empty": "The Claude session is not running yet.", "term_console": "Console window",
        "hint_idle": "🎙  Say “{}” to start talking",
        "hint_rec": "🎙  Send: “{}”  ·  cancel: “{}”  ·  start over: “{}”",
        "hint_dialog": "Claude is waiting for a choice — answer in the Terminal tab",
        "hint_paused": "Listening paused — click “Resume listening”", "hint_start": "Starting the Claude session…",
    },
}
# kolor stanu (klucz palety z theme.py) - awatar, opis stanu
STATE_COLORS = {"idle": "green", "wake": "red", "rec": "red", "busy": "orange", "wait": "orange",
                "dialog": "orange", "speak": "accent", "paused": "grey", "start": "grey", "closed": "grey",
                "error": "red"}
# klawisze pod podgladem terminala - odpowiedzi w oknach wyboru Claude Code
TERM_KEYS = (("↑", ["up"]), ("↓", ["down"]), ("Enter", ["enter"]), ("Esc", ["esc"]),
             ("1", ["text:1"]), ("2", ["text:2"]), ("3", ["text:3"]))
TERM_HISTORY = 300  # linie konsoli nad widocznym oknem w zakladce Terminal
LAYOUT = 2  # zmiana ukladu okna - stara zapamietana geometria (szerokie okno) nie jest wtedy uzywana


class Bridge:
    """Stan dzielony z watkiem nasluchu (listener.run)."""

    def __init__(self, hidden):
        self.stop = threading.Event()
        self.requests = queue.Queue()
        self.paused = False
        self.hidden = hidden
        self.state = "start"
        self.pid = None
        self.heard = ""  # co slyszy Vosk w trakcie nagrywania
        self.problem = None  # "dialog" / "rec_failed" - ustawia nasluch, odbiera tick()

    def on_state(self, state, pid):
        self.state, self.pid = state, pid


def split_voice(text):
    """Odpowiedz Achai -> (tresc, sekcja [GŁOS])."""
    matches = list(MARKER.finditer(text or ""))
    if not matches:
        return text.strip(), ""
    m = matches[-1]
    return text[:m.start()].strip(), text[m.end():].strip()


def trim_chat(keep=400):
    try:
        lines = CHAT_LOG.read_text(encoding="utf-8").splitlines()
    except OSError:
        return
    if len(lines) > keep * 2:
        CHAT_LOG.write_text("\n".join(lines[-keep:]) + "\n", encoding="utf-8")


def round_rect(x1, y1, x2, y2, r):
    """Punkty wielokata (smooth=True) dajacego prostokat z zaokraglonymi rogami."""
    return [x1 + r, y1, x2 - r, y1, x2, y1, x2, y1 + r, x2, y2 - r, x2, y2,
            x2 - r, y2, x1 + r, y2, x1, y2, x1, y2 - r, x1, y1 + r, x1, y1]


def wrapped_lines(text, font, width):
    """Ile linii zajmie tekst zawijany po slowach (jak Text z wrap="word") w polu o szerokosci width."""
    total = 0
    for para in text.split("\n"):
        lines, line = 1, ""
        for word in para.split(" "):
            candidate = f"{line} {word}" if line else word
            if font.measure(candidate) <= width:
                line = candidate
                continue
            if line:
                lines += 1
            while font.measure(word) > width and len(word) > 1:  # slowo dluzsze niz linia - lamane po znakach
                cut = max(1, next((i for i in range(len(word), 0, -1) if font.measure(word[:i]) <= width), 1))
                word = word[cut:]
                lines += 1
            line = word
        total += lines
    return max(1, total)


class Bubble:
    """Dymek rozmowy: plotno z zaokraglonym tlem osadzone w polu rozmowy, a w nim pola tekstowe tylko do
    odczytu - tekst da sie zaznaczac myszka i kopiowac (Ctrl+C, prawy przycisk). Szerokosc wg tresci."""

    def __init__(self, app, body, voice, mine):
        from tkinter import font as tkfont
        p, px, tk = app.theme.p, app.theme.px, app.tk
        self.app, self.mine = app, mine
        self.copy_text = "\n\n".join(x for x in (body, voice) if x)
        fill = p["accent"] if mine else p["surface"]
        self.c = c = tk.Canvas(app.chat, highlightthickness=0, borderwidth=0, background=p["bg"])
        self.shape = c.create_polygon(0, 0, 0, 0, smooth=True, fill=fill, outline="")
        self.parts = []  # (pole tekstowe, okno na plotnie, czcionka, tekst)
        for text, font, color in ((body, (FONT, 10), p["on_accent"] if mine else p["text"]),
                                  ("♪  " + voice if voice else "", (FONT, 10, "italic"), p["accent_text"])):
            if not text:
                continue
            t = tk.Text(c, wrap="word", font=font, borderwidth=0, highlightthickness=0, padx=0, pady=0,
                        background=fill, foreground=color, cursor="xterm", height=1, width=1,
                        selectbackground=p["select"], selectforeground=p["text"],
                        inactiveselectbackground=p["select"], insertwidth=0)
            t.insert("1.0", text)
            t.configure(state="disabled")
            # wylaczone pole nie bierze fokusu - bez tego Ctrl+C nie trafialby do zaznaczenia
            t.bind("<Button-1>", lambda e, t=t: t.focus_set(), add="+")
            t.bind("<Control-c>", lambda e: self.copy_selection() or "break")
            t.bind("<MouseWheel>", app.on_chat_wheel)
            t.bind("<Button-3>", self.menu)
            self.parts.append((t, c.create_window(0, 0, anchor="nw", window=t), tkfont.Font(font=font), text))
        self.pad = (px(14), px(9))
        c.bind("<MouseWheel>", app.on_chat_wheel)
        c.bind("<Button-3>", self.menu)

    def layout(self, maxw):
        c, (pad_x, pad_y), px = self.c, self.pad, self.app.theme.px
        inner = max(120, maxw - 2 * pad_x)
        # dymek waski dla krotkiej odpowiedzi: szerokosc najdluzszej linii, najwyzej tyle, ile miesci sie w oknie
        natural = max((max(font.measure(line) for line in text.split("\n")) + px(4)
                       for _, _, font, text in self.parts), default=0)
        width = min(inner, max(natural, px(24)))
        y = pad_y
        for t, win, font, text in self.parts:
            # liczba linii z szerokosci czcionki - pole jeszcze niewyswietlone nie zna swojego zawijania
            height = wrapped_lines(text, font, width - px(2)) * font.metrics("linespace")
            c.coords(win, pad_x, y)
            c.itemconfigure(win, width=width, height=height)
            y += height + px(6)
        w, h = width + 2 * pad_x, y - px(6) + pad_y
        c.coords(self.shape, *round_rect(0, 0, w - 1, h - 1, min(px(14), h // 2)))
        c.configure(width=w, height=h)

    def selection(self):
        for t, *_ in self.parts:
            if t.tag_ranges("sel"):
                return t.get("sel.first", "sel.last")
        return ""

    def menu(self, event):
        m = self.app.tk.Menu(self.c, tearoff=False)
        self.app.theme.menu(m)
        if self.selection():
            m.add_command(label=self.app.t["copy_sel"], command=self.copy_selection)
        m.add_command(label=self.app.t["copy_all"], command=self.copy)
        m.tk_popup(event.x_root, event.y_root)

    def copy_selection(self):
        text = self.selection()
        if text:
            self.c.clipboard_clear()
            self.c.clipboard_append(text)

    def copy(self):
        self.c.clipboard_clear()
        self.c.clipboard_append(self.copy_text)


class SendButton:
    """Okragly przycisk wysylania z papierowym samolotem; przygaszony, gdy pole jest puste."""

    def __init__(self, app, parent, size, command):
        self.app, self.size, self.command = app, size, command
        self.c = app.tk.Canvas(parent, width=size, height=size, highlightthickness=0, borderwidth=0, cursor="hand2")
        self.active = self.hover = False
        self.c.bind("<Enter>", lambda e: self.set(hover=True))
        self.c.bind("<Leave>", lambda e: self.set(hover=False))
        self.c.bind("<Button-1>", lambda e: self.active and self.command())
        self.paint()

    def set(self, active=None, hover=None):
        if active is not None:
            self.active = active
        if hover is not None:
            self.hover = hover
        self.paint()

    def paint(self):
        p, c, s = self.app.theme.p, self.c, self.size
        c.configure(background=p["surface"])
        c.delete("all")
        if self.active:
            fill, ink = (p["accent_hover"] if self.hover else p["accent"]), p["on_accent"]
        else:
            fill, ink = p["surface2"], p["faint"]
        c.create_oval(1, 1, s - 2, s - 2, fill=fill, outline="")
        pts = [(0.30, 0.27), (0.76, 0.50), (0.30, 0.73), (0.37, 0.53), (0.55, 0.50), (0.37, 0.47)]
        c.create_polygon([v * s for pt in pts for v in pt], fill=ink, outline="")


class PopupMenu:
    """Menu aplikacji: duze, czytelne pozycje z ikonami w kolorach motywu (menu systemowe Windows jest male
    i ignoruje ciemny motyw). items: (ikona, opis, funkcja) albo None jako separator."""

    def __init__(self, app, items):
        self.app, self.items = app, items
        self.win = None
        self.closed_at = 0.0

    def toggle(self, right, top):
        if self.win:
            self.close()
        elif time.time() - self.closed_at > 0.25:  # klik w ☰ przy otwartym menu najpierw je zamyka (FocusOut)
            self.open(right, top)

    def open(self, right, top):
        tk, th = self.app.tk, self.app.theme
        p, px = th.p, th.px
        self.win = win = tk.Toplevel(self.app.root)
        win.overrideredirect(True)
        win.attributes("-topmost", True)
        frame = tk.Frame(win, background=p["border"], padx=1, pady=1)
        frame.pack()
        body = tk.Frame(frame, background=p["surface"], padx=px(4), pady=px(4))
        body.pack()
        for item in self.items:
            if item is None:
                tk.Frame(body, height=1, background=p["border"]).pack(fill="x", padx=px(10), pady=px(4))
                continue
            icon, label, command = item
            row = tk.Frame(body, background=p["surface"], cursor="hand2")
            row.pack(fill="x")
            parts = [row, tk.Label(row, text=icon, width=2, font=(FONT, 12), background=p["surface"],
                                   foreground=p["accent_text"]),
                     tk.Label(row, text=label, anchor="w", font=(FONT, 11), background=p["surface"],
                              foreground=p["text"])]
            parts[1].pack(side="left", padx=(px(10), px(6)), pady=px(6))
            parts[2].pack(side="left", fill="x", expand=True, padx=(0, px(18)))
            for w in parts:
                w.bind("<Enter>", lambda e, ws=parts: [x.configure(background=p["accent_soft"]) for x in ws])
                w.bind("<Leave>", lambda e, ws=parts: [x.configure(background=p["surface"]) for x in ws])
                w.bind("<Button-1>", lambda e, c=command: self.run(c))
        win.update_idletasks()
        win.geometry(f"+{right - win.winfo_reqwidth()}+{top + px(4)}")
        win.bind("<FocusOut>", lambda e: self.close())
        win.bind("<Escape>", lambda e: self.close())
        win.focus_force()

    def run(self, command):
        self.close()
        self.app.root.after(10, command)

    def close(self):
        if self.win:
            self.win.destroy()
            self.win = None
            self.closed_at = time.time()


class App:
    def __init__(self, args):
        import tkinter as tk
        from tkinter import ttk
        self.tk, self.ttk = tk, ttk
        self.args = args
        self.cfg = load_config()
        self.opts = self.cfg.get("app", {})
        lang = self.cfg.get("language", "pl")
        self.t = TEXT[lang if lang in TEXT else "en"]
        self.bridge = None
        self.thread = None
        self.logs = queue.Queue()
        self.calls = queue.Queue()  # wywolania z watku ikony w zasobniku
        self.chat_pos = 0
        self.auto_shown = False
        self.hidden_once = None
        self.tray = None
        self.quitting = False
        self.ticks = 0
        self.last_start = 0.0
        self.update_info = self.update_path = None
        self.update_msg = None  # (tekst, przycisk) dla paska aktualizacji - ustawia watek, rysuje tick()
        self.update_busy = False
        try:
            self.config_mtime = CONFIG.stat().st_mtime
        except OSError:
            self.config_mtime = 0
        listener.LOG_SINKS.append(self.logs.put)
        self.messages = []  # ("msg", rola, godzina, tresc, glos) albo ("note", tekst) - do przerysowania
        self.bubbles = []
        self.chat_width = 0
        self.relayout_job = None
        self.state_key = "start"
        self.agent_running = {}
        self.screen = None  # ostatni odczyt konsoli Claude (listener.parse_screen) - watek watch_screen
        self.markers = listener.screen_text(self.cfg)
        self.visible = True
        self.term_shown = None
        self.term_open = False  # zakladka Terminal na wierzchu - czytamy wtedy tez historie konsoli
        self.dialog_seen = False

        self.root = root = tk.Tk()
        root.withdraw()  # pokazujemy dopiero gotowe okno (bez mrugniecia jasnym tlem)
        root.title("Achaja")
        saved = read_json(WINDOW_FILE, {})
        root.geometry(saved.get("geometry") if saved.get("layout") == LAYOUT else "440x780")
        root.minsize(360, 520)
        if ICON.exists():
            root.iconbitmap(default=str(ICON))
        self.theme = theme.Theme(root, self.opts.get("theme", "auto"))
        root.protocol("WM_DELETE_WINDOW", self.on_close)

        self.build_menu()
        self.build_header()
        px = self.theme.px
        self.nb = nb = ttk.Notebook(root)
        nb.pack(fill="both", expand=True, padx=px(8), pady=(px(2), px(8)))
        self.build_chat(nb)
        self.build_terminal(nb)
        self.build_agents(nb)
        self.build_log(nb)
        self.paint()
        threading.Thread(target=self.watch_screen, daemon=True).start()

        self.start_tray()
        if args.minimized:
            root.after(0, self.hide_window)
        else:
            root.deiconify()
            root.after(50, lambda: self.theme.titlebar(root))
        self.start_listener(args.resume)
        root.after(200, self.poll)
        root.after(60, self.animate)
        if self.opts.get("update_check", True):
            root.after(UPDATE_FIRST * 1000, self.scheduled_update_check)

    # --- budowa okna ---
    def build_menu(self):
        t = self.t
        self.menu = PopupMenu(self, [
            ("＋", t["new"], lambda: self.request("clear")),
            ("↻", t["repeat"], self.repeat),
            None,
            ("⚙", t["settings"], self.open_settings),
            ("✎", t["edit_config"], self.edit_config),
            None,
            ("▣", t["console"], self.toggle_console),
            ("⟳", t["restart"], self.restart_listener),
            ("⇩", t["check_update"], lambda: self.check_update(manual=True)),
            ("ⓘ", t["about"], self.about),
            None,
            ("⏻", t["quit"], self.quit),
        ])
        self.root.bind("<F10>", lambda e: self.show_menu() or "break")

    def show_menu(self):
        b = self.menu_btn
        self.menu.toggle(b.winfo_rootx() + b.winfo_width(), b.winfo_rooty() + b.winfo_height())

    def build_header(self):
        ttk, t, px = self.ttk, self.t, self.theme.px
        # awatar + imie + stan (slucha / nagrywa / mysli / mowi) - czytelne z daleka
        self.head = head = ttk.Frame(self.root, padding=(px(12), px(10), px(8), px(4)))
        head.pack(fill="x")
        # haftowany pas pod naglowkiem - fala swiatla, gdy Achaja slucha, mysli albo mowi
        self.band = ornament.Band(self.root, px(18))
        self.band.c.pack(fill="x", padx=px(12), pady=(0, px(6)))
        self.avatar = avatar.Avatar(head, px(72))
        self.avatar.widget.pack(side="left")
        self.menu_btn = ttk.Button(head, text="☰", style="Ghost.TButton", width=2, command=self.show_menu)
        self.menu_btn.pack(side="right", anchor="n")
        info = ttk.Frame(head)
        info.pack(side="left", fill="x", expand=True, padx=(px(10), 0))
        ttk.Label(info, text=t["achaja"], style="Brand.TLabel").pack(anchor="w")
        self.status = ttk.Label(info, text=t["st_start"], style="State.TLabel")
        self.status.pack(anchor="w", fill="x")
        # pasek "na zywo": dyktowany tekst albo to, czym Claude sie teraz zajmuje
        self.live = ttk.Frame(self.root, style="Banner.TFrame", padding=(px(12), px(7)))
        self.live_label = ttk.Label(self.live, style="Live.TLabel", justify="left")
        self.live_label.pack(fill="x")
        self.live.bind("<Configure>", lambda e: self.live_label.configure(wraplength=max(100, e.width - px(28))))
        self.live_shown = None
        self.bar = bar = ttk.Frame(self.root, padding=(px(12), px(2), px(12), px(4)))
        bar.pack(fill="x")
        self.talk_btn = ttk.Button(bar, text="🎤  " + t["talk"], style="Accent.TButton", command=self.talk)
        self.talk_btn.pack(side="left", fill="x", expand=True)
        self.cancel_btn = ttk.Button(bar, text=t["cancel"], command=lambda: self.request("cancel"))
        self.pause_btn = ttk.Button(bar, text=t["pause"], command=self.toggle_pause)
        self.pause_btn.pack(side="left", padx=(px(6), 0))
        # podpowiedz (nieklikalna): jakie slowa dzialaja teraz - "Achaja" przy czuwaniu, "wykonaj"... przy nagrywaniu
        chip = ttk.Frame(self.root, style="Pill.TFrame", padding=(px(12), px(3)))
        chip.pack(anchor="w", padx=px(12), pady=(0, px(4)))
        self.hint = ttk.Label(chip, style="Chip.TLabel")
        self.hint.pack()
        # pasek aktualizacji - pokazuje sie, gdy jest nowa wersja
        self.upd_bar = ttk.Frame(self.root, style="Banner.TFrame", padding=(px(14), px(6), px(6), px(6)))
        self.upd_label = ttk.Label(self.upd_bar, style="Banner.TLabel")
        self.upd_label.pack(side="left")
        self.upd_btn = ttk.Button(self.upd_bar, style="Banner.Accent.TButton", command=self.update_action)
        self.upd_shown = None

    def build_chat(self, nb):
        tk, ttk, t, px = self.tk, self.ttk, self.t, self.theme.px
        frame = ttk.Frame(nb, padding=(px(8), 0, px(8), px(4)))
        nb.add(frame, text=t["chat"])
        card = ttk.Frame(frame, style="Input.TFrame", padding=px(6))
        card.pack(side="bottom", fill="x", pady=(px(8), 0))  # najpierw dol - przy malym oknie sciska sie rozmowa
        box = ttk.Frame(frame)
        box.pack(fill="both", expand=True)
        self.chat = tk.Text(box, wrap="word", state="disabled", padx=px(14), pady=px(6), cursor="arrow",
                            font=(FONT, 10))
        sb = ttk.Scrollbar(box, command=self.chat.yview)
        self.chat.configure(yscrollcommand=sb.set)
        sb.pack(side="right", fill="y")
        self.chat.pack(side="left", fill="both", expand=True)
        self.chat.bind("<Configure>", self.on_chat_resize)
        self.chat.bind("<MouseWheel>", self.on_chat_wheel)

        self.entry_card = card
        self.send_btn = SendButton(self, card, px(38), self.send_text)  # okragly, jak w komunikatorach
        self.send_btn.c.pack(side="right", anchor="s")
        self.entry = tk.Text(card, height=2, wrap="word", font=(FONT, 10), padx=px(6), pady=px(4))
        self.entry.pack(side="left", fill="both", expand=True, padx=(0, px(8)))
        self.entry.bind("<Return>", self.on_enter)
        self.entry.bind("<KeyRelease>", lambda e: self.send_btn.set(active=self.entry_text() != ""))
        self.entry.bind("<FocusIn>", lambda e: (self.placeholder(False), card.state(["focus"])))
        self.entry.bind("<FocusOut>", lambda e: (self.placeholder(True), card.state(["!focus"])))

        trim_chat()
        try:
            lines = CHAT_LOG.read_text(encoding="utf-8").splitlines()
            self.chat_pos = CHAT_LOG.stat().st_size
        except OSError:
            lines = []
        for line in lines[-60:]:
            self.parse_chat_line(line)

    def build_agents(self, nb):
        tk, ttk, t, px = self.tk, self.ttk, self.t, self.theme.px
        self.agents_tab = frame = ttk.Frame(nb, padding=(px(8), 0, px(8), px(4)))
        nb.add(frame, text=t["agents"])
        # waskie okno: lista agentow u gory, pod nia przebieg pracy wybranego
        top_card = ttk.Frame(frame, style="Card.TFrame", padding=px(6))
        top_card.pack(fill="x")
        self.agent_list = tk.Listbox(top_card, height=4, exportselection=False)
        self.agent_list.pack(fill="x")
        self.agent_list.bind("<<ListboxSelect>>", lambda e: self.show_agent(force=True))
        right = ttk.Frame(frame, style="Card.TFrame", padding=(px(10), px(8), px(4), px(8)))
        right.pack(fill="both", expand=True, pady=(px(8), 0))
        top = ttk.Frame(right, style="Plain.TFrame")
        top.pack(fill="x", pady=(0, px(6)))
        self.agent_btn = ttk.Button(top, style="Card.Key.TButton", command=self.agent_action)
        self.agent_btn.pack(side="right")
        self.agent_title = ttk.Label(top, style="CardTitle.TLabel", text=t["agent_pick"])
        self.agent_title.pack(side="left")
        self.agent_sub = ttk.Label(top, style="CardMuted.TLabel")
        self.agent_sub.pack(side="left", padx=(px(8), 0))
        self.agent_text = tk.Text(right, wrap="word", state="disabled", padx=px(4), pady=px(2),
                                  font=(self.theme.mono, 9))
        sb = ttk.Scrollbar(right, command=self.agent_text.yview, style="Card.Vertical.TScrollbar")
        self.agent_text.configure(yscrollcommand=sb.set)
        sb.pack(side="right", fill="y")
        self.agent_text.pack(side="left", fill="both", expand=True)
        self.agent_names, self.agent_shown = [], (False, -1)  # pierwszy odczyt zawsze rysuje
        self.agents_label = t["agents"]

    def build_terminal(self, nb):
        """Podglad ukrytej konsoli Claude na zywo + klawisze do okien wyboru (zgody, nowy serwer MCP)."""
        tk, ttk, t, px = self.tk, self.ttk, self.t, self.theme.px
        self.term_tab = frame = ttk.Frame(nb, padding=(px(8), 0, px(8), px(4)))
        nb.add(frame, text=t["term"])
        keys = ttk.Frame(frame)
        keys.pack(side="bottom", fill="x", pady=(px(6), 0))
        ttk.Button(keys, text="⧉  " + t["term_console"], style="Key.TButton", command=self.toggle_console).pack(
            side="right")
        # klawisze do okien wyboru Claude (zgoda, nowy serwer MCP) - widoczne tylko, gdy takie okno czeka
        self.term_keys = ttk.Frame(keys)
        for label, seq in TERM_KEYS:
            ttk.Button(self.term_keys, text=label, style="Key.TButton", width=len(label) + 1,
                       command=lambda seq=seq: self.request(("key", seq))).pack(side="left", padx=(0, px(3)))
        card = ttk.Frame(frame, style="Card.TFrame", padding=(px(10), px(8), px(4), px(8)))
        card.pack(fill="both", expand=True)
        self.term_text = tk.Text(card, wrap="word", state="disabled", padx=px(2), pady=px(2),
                                 font=(self.theme.mono, 8))
        sb = ttk.Scrollbar(card, command=self.term_text.yview, style="Card.Vertical.TScrollbar")
        self.term_text.configure(yscrollcommand=sb.set)
        sb.pack(side="right", fill="y")
        self.term_text.pack(side="left", fill="both", expand=True)

    def watch_screen(self):
        """Watek: co chwile czyta ekran ukrytej konsoli Claude (stan, dyktowany tekst, okna wyboru)."""
        import console
        while not self.quitting:
            pid = self.claude_pid()
            lines = None
            if pid:
                try:
                    lines = console.read_screen(pid, TERM_HISTORY if self.term_open else 0)
                except OSError:
                    pass
            self.screen = listener.parse_screen(lines, self.markers) if lines else None
            time.sleep(0.35 if self.visible else 1.5)

    def show_terminal(self, screen):
        if not screen:
            text = self.t["term_empty"]
        else:  # dlugie kreski konsoli (120 znakow) zawijalyby sie w waskim oknie
            text = "\n".join(re.sub(r"─{4,}", "────", line.rstrip()) for line in screen["lines"]).lstrip("\n")
        if text == self.term_shown:
            return
        self.term_shown = text
        at_end = self.term_text.yview()[1] > 0.98
        self.term_text.configure(state="normal")
        self.term_text.delete("1.0", "end")
        self.term_text.insert("end", text)
        self.term_text.configure(state="disabled")
        if at_end:
            self.term_text.see("end")

    def build_log(self, nb):
        tk, ttk, px = self.tk, self.ttk, self.theme.px
        frame = ttk.Frame(nb, padding=(px(8), 0, px(8), px(4)))
        nb.add(frame, text=self.t["log"])
        card = ttk.Frame(frame, style="Card.TFrame", padding=(px(12), px(8), px(4), px(8)))
        card.pack(fill="both", expand=True)
        self.log_text = tk.Text(card, wrap="word", state="disabled", padx=px(4), pady=px(2),
                                font=(self.theme.mono, 9))
        sb = ttk.Scrollbar(card, command=self.log_text.yview, style="Card.Vertical.TScrollbar")
        self.log_text.configure(yscrollcommand=sb.set)
        sb.pack(side="right", fill="y")
        self.log_text.pack(side="left", fill="both", expand=True)

    def paint(self):
        """Kolory zwyklych widzetow tk (ttk bierze je z motywu) - po starcie i po zmianie motywu."""
        th = self.theme
        p = th.p
        th.text(self.chat, surface=False)
        th.text(self.entry)
        self.send_btn.paint()
        th.text(self.agent_text)
        th.text(self.term_text)
        th.text(self.log_text)
        th.listbox(self.agent_list)
        self.menu.close()  # otworzy sie w nowych kolorach
        self.chat.tag_configure("who", foreground=p["muted"], font=(FONT_BOLD, 9), spacing1=th.px(12))
        self.chat.tag_configure("meta", foreground=p["faint"], font=(FONT, 8), spacing1=th.px(12))
        self.chat.tag_configure("bubble", spacing1=th.px(3), spacing3=th.px(2))
        self.chat.tag_configure("copylink", foreground=p["accent_text"], font=(FONT, 8), spacing1=th.px(2))
        self.chat.tag_bind("copylink", "<Enter>", lambda e: self.chat.configure(cursor="hand2"))
        self.chat.tag_bind("copylink", "<Leave>", lambda e: self.chat.configure(cursor="arrow"))
        self.chat.tag_configure("right", justify="right")
        self.chat.tag_configure("left", justify="left")
        self.chat.tag_configure("note", foreground=p["note"], font=(FONT, 9, "italic"), justify="center",
                                spacing1=th.px(10), spacing3=th.px(4))
        self.chat.tag_configure("empty_title", foreground=p["text"], font=(FONT_BOLD, 16), justify="center",
                                spacing1=th.px(90))
        self.chat.tag_configure("empty", foreground=p["muted"], font=(FONT, 10), justify="center",
                                spacing1=th.px(6))
        self.entry.tag_configure("ph", foreground=p["faint"])
        self.log_text.tag_configure("time", foreground=p["faint"])
        self.log_text.tag_configure("err", foreground=p["red"])
        self.render_chat()
        self.agent_list.delete(0, "end")  # odswiezy sie z nowymi kolorami
        self.placeholder(self.root.focus_get() is not self.entry)

    # --- rozmowa ---
    def placeholder(self, on):
        has_ph = bool(self.entry.tag_ranges("ph"))
        if on and not self.entry.get("1.0", "end").strip():
            self.entry.insert("1.0", self.t["placeholder"], "ph")
        elif not on and has_ph:
            self.entry.delete("1.0", "end")

    def on_enter(self, event):
        if event.state & 0x1:  # Shift+Enter - nowa linia
            return None
        self.send_text()
        return "break"

    def entry_text(self):
        return "" if self.entry.tag_ranges("ph") else self.entry.get("1.0", "end").strip()

    def send_text(self):
        text = self.entry_text()
        if not text:
            return
        self.entry.delete("1.0", "end")
        self.send_btn.set(active=False)
        self.request(("type", text))

    def parse_chat_line(self, line):
        try:
            item = json.loads(line)
        except ValueError:
            return None
        when = time.strftime("%H:%M", time.localtime(item.get("at", time.time())))
        if item.get("role") == "user":
            msg = ("msg", "user", when, item.get("text", "").strip(), "")
        else:
            msg = ("msg", "ai", when, *split_voice(item.get("text", "")))
        self.messages.append(msg)
        return msg

    def add_chat_line(self, line):
        msg = self.parse_chat_line(line)
        if msg:
            self.show_message(msg)

    def add_note(self, text):
        msg = ("note", text)
        self.messages.append(msg)
        self.show_message(msg)

    def show_message(self, msg):
        if len(self.messages) > 300 or len(self.messages) == 1:
            self.messages = self.messages[-200:]
            self.render_chat()  # przyciecie historii albo pierwsza wiadomosc zamiast ekranu powitalnego
            return
        self.chat.configure(state="normal")
        self.draw_message(msg)
        self.chat.configure(state="disabled")
        self.scroll_chat_end()

    def render_chat(self):
        for b in self.bubbles:
            b.c.destroy()
        self.bubbles = []
        self.chat.configure(state="normal")
        self.chat.delete("1.0", "end")
        if not self.messages:
            self.chat.insert("end", self.t["empty_title"] + "\n", "empty_title")
            self.chat.insert("end", self.t["empty"] + "\n", "empty")
        for msg in self.messages:
            self.draw_message(msg)
        self.chat.configure(state="disabled")
        self.scroll_chat_end()

    def draw_message(self, msg):
        chat = self.chat
        if msg[0] == "note":
            chat.insert("end", msg[1] + "\n", "note")
            return
        _, role, when, body, voice = msg
        if not body and not voice:
            return
        mine = role == "user"
        align = "right" if mine else "left"
        chat.insert("end", self.t["you"] if mine else self.t["achaja"], ("who", align))
        chat.insert("end", "  " + when + "\n", ("meta", align))
        bubble = Bubble(self, body, voice, mine)
        bubble.layout(self.bubble_width())
        self.bubbles.append(bubble)
        start = chat.index("end-1c")
        chat.window_create("end", window=bubble.c)
        chat.insert("end", "\n")
        chat.tag_add(align, start, "end-1c")
        chat.tag_add("bubble", start, "end-1c")
        if not mine:  # widoczne "Kopiuj" pod odpowiedzia Achai (zaznaczony fragment: Ctrl+C albo prawy przycisk)
            tag = f"copy{len(self.bubbles)}"
            chat.insert("end", "⧉ " + self.t["copy"] + "\n", ("copylink", tag, align))
            chat.tag_bind(tag, "<Button-1>", lambda e, b=bubble, tag=tag: self.copy_answer(b, tag))

    def copy_answer(self, bubble, tag):
        bubble.copy()
        ranges = self.chat.tag_ranges(tag)
        if not ranges:
            return
        self.chat.configure(state="normal")  # chwilowo "Skopiowano" w miejscu linku
        self.chat.delete(ranges[0], ranges[1])
        self.chat.insert(ranges[0], "✓ " + self.t["copied"] + "\n", ("copylink", tag, "left"))
        self.chat.configure(state="disabled")

        def back():
            r = self.chat.tag_ranges(tag)
            if r:
                self.chat.configure(state="normal")
                self.chat.delete(r[0], r[1])
                self.chat.insert(r[0], "⧉ " + self.t["copy"] + "\n", ("copylink", tag, "left"))
                self.chat.configure(state="disabled")
        self.root.after(1500, back)

    def bubble_width(self):
        width = self.chat.winfo_width()
        if width < 50:  # okno jeszcze niepokazane
            width = max(self.root.winfo_width(), 400) - self.theme.px(40)
        return int((width - self.theme.px(40)) * 0.8)

    def scroll_chat_end(self):
        self.chat.update_idletasks()
        self.chat.yview_moveto(1.0)

    def on_chat_resize(self, event):
        if abs(event.width - self.chat_width) < 4:
            return
        self.chat_width = event.width
        if self.relayout_job:
            self.root.after_cancel(self.relayout_job)
        self.relayout_job = self.root.after(80, self.relayout_chat)

    def relayout_chat(self):
        self.relayout_job = None
        at_end = self.chat.yview()[1] > 0.98
        width = self.bubble_width()
        for b in self.bubbles:
            b.layout(width)
        if at_end:
            self.scroll_chat_end()

    def on_chat_wheel(self, event):
        # dymek to jedna "linia" pola tekstowego - przewijamy pikselami, nie liniami
        self.chat.yview_scroll(int(-event.delta / 120 * self.theme.px(60)), "pixels")
        return "break"

    def read_new_chat(self):
        try:
            size = CHAT_LOG.stat().st_size
        except OSError:
            return
        if size < self.chat_pos:  # plik przyciety
            self.chat_pos = 0
        if size == self.chat_pos:
            return
        with open(CHAT_LOG, "rb") as f:
            f.seek(self.chat_pos)
            data = f.read()
        end = data.rfind(b"\n") + 1  # tylko pelne linie
        self.chat_pos += end
        for line in data[:end].decode("utf-8", errors="replace").splitlines():
            self.add_chat_line(line)

    # --- agenci ---
    def refresh_agents(self):
        running = {n: r for n, r in read_json(AGENTS_STATE / "running.json", {}).items() if pid_alive(r["pid"])}
        self.agent_running = running
        logs = sorted(AGENTS_STATE.glob("*.log"), key=os.path.getmtime, reverse=True)
        names = list(running) + [p.stem for p in logs if p.stem not in running]
        labels = []
        for n in names:
            if n in running:
                labels.append(f"●  {n}  · " + self.t["working"].format(self.agent_minutes(n)))
            else:
                try:
                    when = time.strftime("%d.%m %H:%M", time.localtime(os.path.getmtime(AGENTS_STATE / f"{n}.log")))
                except OSError:
                    when = ""
                labels.append(f"✓  {n}  · " + self.t["agent_finished"].format(when))
        if labels != list(self.agent_list.get(0, "end")):
            selected = self.selected_agent()
            self.agent_list.delete(0, "end")
            p = self.theme.p
            for i, label in enumerate(labels):
                self.agent_list.insert("end", label)
                self.agent_list.itemconfigure(i, foreground=p["green"] if names[i] in running else p["text"])
            self.agent_names = names
            if selected in names:
                self.agent_list.selection_set(names.index(selected))
            elif names:
                self.agent_list.selection_set(0)
        tab = f"{self.t['agents']}  {len(running)}" if running else self.t["agents"]
        if tab != self.agents_label:
            self.agents_label = tab
            self.nb.tab(self.agents_tab, text=tab)
        self.show_agent()

    def agent_minutes(self, name):
        return round((time.time() - self.agent_running[name]["started"]) / 60)

    def selected_agent(self):
        sel = self.agent_list.curselection()
        return self.agent_names[sel[0]] if sel and sel[0] < len(self.agent_names) else None

    def show_agent(self, force=False):
        name = self.selected_agent()
        working = name in self.agent_running
        sub = "● " + self.t["working"].format(self.agent_minutes(name)) if working else (
            self.t["agent_done"] if name else "")
        self.agent_title.configure(text=name or self.t["agent_pick"])
        self.agent_btn.configure(text=self.t["stop_agent"] if working else self.t["agent_remove"])
        self.agent_btn.state(["!disabled"] if name else ["disabled"])
        self.agent_sub.configure(text=sub, foreground=self.theme.p["green" if working else "muted"])
        path = AGENTS_STATE / f"{name}.log" if name else None
        try:
            mtime = path.stat().st_mtime if path else 0
        except OSError:
            mtime = 0
        if not force and (name, mtime) == self.agent_shown:
            return
        self.agent_shown = (name, mtime)
        text = path.read_text(encoding="utf-8", errors="replace") if mtime else self.t["no_agents"]
        at_end = self.agent_text.yview()[1] > 0.98
        self.agent_text.configure(state="normal")
        self.agent_text.delete("1.0", "end")
        self.agent_text.insert("end", text)
        self.agent_text.configure(state="disabled")
        if at_end or force:
            self.agent_text.see("end")

    def agent_action(self):
        """Pracujacy agent: przerwij. Zakonczony: usun z listy (log trafia do state/agents/archive)."""
        name = self.selected_agent()
        if not name:
            return
        if name in self.agent_running:
            self.add_note(self.t["agent_stopping"].format(name))

            def work():
                result = subprocess.run([str(PYTHON), str(SCRIPTS / "agent.py"), "stop", name], capture_output=True,
                                        text=True, encoding="utf-8", errors="replace", stdin=subprocess.DEVNULL,
                                        creationflags=CREATE_NO_WINDOW)
                self.logs.put(f"{time.strftime('%H:%M:%S')} {(result.stdout or result.stderr).strip()}")
            threading.Thread(target=work, daemon=True).start()
            return
        archive = AGENTS_STATE / "archive"
        archive.mkdir(exist_ok=True)
        try:
            os.replace(AGENTS_STATE / f"{name}.log", archive / f"{name}.log")
        except OSError as exc:
            self.logs.put(f"{time.strftime('%H:%M:%S')} BLAD: nie udalo sie usunac {name} z listy: {exc}")
        self.agent_list.delete(0, "end")  # odswiezy sie bez niego
        self.refresh_agents()

    # --- nasluch ---
    def start_listener(self, resume):
        paused = self.bridge.paused if self.bridge else False
        self.bridge = Bridge(hidden=self.opts.get("hide_console", True))
        self.bridge.paused = paused
        self.last_start = time.time()
        self.hidden_once = None
        args = listener.parse_args(["--continue"] if resume else [])
        bridge = self.bridge

        def work():
            try:
                if listener.run(args, bridge) == "closed":
                    bridge.state = "closed"
            except BaseException as exc:  # SystemExit z launch_achaja tez
                self.logs.put(f"{time.strftime('%H:%M:%S')} BLAD: {exc!r}")
                bridge.state = "error"

        self.thread = threading.Thread(target=work, daemon=True)
        self.thread.start()

    def restart_listener(self):
        if self.bridge:
            self.bridge.stop.set()
        if self.thread:
            self.thread.join(timeout=3)
        self.start_listener(resume=True)  # podpina sie do tej samej sesji Claude, jesli dziala

    def request(self, req):
        if self.bridge:
            self.bridge.requests.put(req)

    def talk(self):
        self.request("send" if self.bridge and self.bridge.state == "recording" else "record")

    def toggle_pause(self):
        if self.bridge:
            self.bridge.paused = not self.bridge.paused

    def repeat(self):
        path = STATE / "last_voice.txt"
        if path.exists():
            subprocess.Popen([str(PYTHONW), str(SCRIPTS / "speak.py"), "--file", str(path)], cwd=str(SCRIPTS),
                             creationflags=DETACHED_PROCESS | CREATE_NO_WINDOW, close_fds=True)

    def open_settings(self):
        subprocess.Popen([str(PYTHONW), str(ROOT / "setup" / "configure.py")], cwd=str(ROOT),
                         creationflags=DETACHED_PROCESS | CREATE_NO_WINDOW, close_fds=True)

    def claude_pid(self):
        pid = self.bridge.pid if self.bridge else None
        return pid if pid and pid_alive(pid) else None

    def toggle_console(self):
        import console
        pid = self.claude_pid()
        if pid:
            console.set_visible(pid, not console.is_visible(pid))
            self.auto_shown = False

    def edit_config(self):
        subprocess.Popen(["notepad.exe", str(CONFIG)], creationflags=DETACHED_PROCESS, close_fds=True)

    def about(self):
        from tkinter import messagebox
        messagebox.showinfo("Achaja", self.t["about_text"].format(__version__), parent=self.root)

    # --- aktualizacje ---
    def scheduled_update_check(self):
        self.check_update(manual=False)
        self.root.after(UPDATE_EVERY * 1000, self.scheduled_update_check)

    def check_update(self, manual):
        """Sprawdza wydania w tle; nowa wersja jest od razu pobierana (update_auto_download)."""
        if self.update_busy:
            return
        self.update_busy = True
        t = self.t

        def work():
            try:
                info = update.check()
                if not info:
                    if manual:
                        self.update_msg = (t["upd_none"].format(__version__), None)
                    return
                self.update_info = info
                if not update.installed():
                    self.update_msg = (t["upd_source"].format(info["version"]), "open")
                    return
                if not info.get("url"):
                    if manual:
                        self.update_msg = (t["upd_noasset"].format(info["version"]), None)
                    return
                path = update.downloaded(info)
                if not path and (manual or self.opts.get("update_auto_download", True)):
                    def progress(frac):
                        self.update_msg = (t["upd_downloading"].format(info["version"], int(frac * 100)), None)
                    path = update.download(info, progress)
                if path:
                    self.update_path = path
                    key = "upd_ready" if self.opts.get("update_install_on_quit", True) else "upd_ready_manual"
                    self.update_msg = (t[key].format(info["version"]), "install")
                    self.logs.put(f"{time.strftime('%H:%M:%S')} Pobrano nowa wersje {info['version']}: {path}")
            except Exception as exc:
                self.logs.put(f"{time.strftime('%H:%M:%S')} Aktualizacja: {exc!r}")
                if manual:
                    self.update_msg = (t["upd_error"].format(exc), None)
            finally:
                self.update_busy = False

        threading.Thread(target=work, daemon=True).start()

    def show_update_bar(self):
        msg = self.update_msg
        if msg == self.upd_shown:
            return
        self.upd_shown = msg
        if not msg:
            self.upd_bar.pack_forget()
            return
        text, action = msg
        self.upd_label.configure(text=text)
        if action:
            self.upd_btn.configure(text=self.t["upd_install"] if action == "install" else self.t["upd_open"])
            self.upd_btn.pack(side="right")
        else:
            self.upd_btn.pack_forget()
        if not self.upd_bar.winfo_ismapped():
            px = self.theme.px
            self.upd_bar.pack(fill="x", padx=px(16), pady=(0, px(6)), after=self.band.c)

    def update_action(self):
        if self.update_msg and self.update_msg[1] == "install" and self.update_path:
            self.quit(ask=False, install_now=True)
        elif self.update_info:
            os.startfile(self.update_info["page"])

    # --- petla okna ---
    def poll(self):
        try:
            self.tick()
        except Exception as exc:
            self.logs.put(f"{time.strftime('%H:%M:%S')} BLAD okna: {exc!r}")
        self.root.after(300, self.poll)

    def tick(self):
        import console
        while not self.calls.empty():
            self.calls.get_nowait()()
        lines = []
        while not self.logs.empty():
            lines.append(self.logs.get_nowait())
        if lines:
            at_end = self.log_text.yview()[1] > 0.98
            self.log_text.configure(state="normal")
            for line in "\n".join(lines).splitlines():
                stamp = re.match(r"\d\d:\d\d:\d\d ", line)
                if stamp:
                    self.log_text.insert("end", stamp.group(), "time")
                    line = line[stamp.end():]
                bad = re.search(r"BLAD|BŁĄD|ERROR|Error|Traceback|Exception", line)
                self.log_text.insert("end", line + "\n", "err" if bad else ())
            if int(self.log_text.index("end-1c").split(".")[0]) > 3000:
                self.log_text.delete("1.0", "1000.0")
            self.log_text.configure(state="disabled")
            if at_end:
                self.log_text.see("end")

        if SHOW_FLAG.exists():
            SHOW_FLAG.unlink(missing_ok=True)
            self.show_window()
        self.read_new_chat()
        self.show_update_bar()
        try:
            mtime = CONFIG.stat().st_mtime
        except OSError:
            mtime = self.config_mtime
        if mtime != self.config_mtime:  # zapis w kreatorze ustawien albo w edytorze
            self.config_mtime = mtime
            self.cfg = load_config()
            self.opts = self.cfg.get("app", {})
            self.markers = listener.screen_text(self.cfg)
            if self.theme.apply(self.opts.get("theme", "auto")):
                self.paint()
            self.add_note(self.t["config_changed"])
            self.restart_listener()
        self.ticks += 1
        if self.ticks % 3 == 1:  # agenci mniej wiecej co sekunde
            self.refresh_agents()
        if self.ticks % 10 == 0 and self.theme.stale():  # zmiana trybu jasny/ciemny w Windows
            self.theme.apply()
            self.paint()

        b = self.bridge
        pid = self.claude_pid()
        # sesja Claude z poprzedniego uruchomienia mogla byc widoczna - chowamy ja raz po podpieciu
        if pid and b.hidden and self.hidden_once != pid:
            self.hidden_once = pid
            console.set_visible(pid, False)
        work = read_json(STATE / "achaja_state.json", {})
        waiting = work.get("state") == "waiting" and work.get("kind") == "permission_prompt"
        if pid and waiting and not self.auto_shown and self.opts.get("show_console_on_prompt", False):
            if not console.is_visible(pid):
                console.set_visible(pid, True)
                self.auto_shown = True
                self.add_note(self.t["wait_note"])
        elif self.auto_shown and not waiting and pid and b.hidden:
            console.set_visible(pid, False)
            self.auto_shown = False

        # stan z nasluchu, hookow sesji i z ekranu konsoli (ekran widzi tez okna wyboru, ktorych hooki nie zglaszaja)
        self.visible = self.root.winfo_viewable()
        self.term_open = self.visible and self.nb.select() == str(self.term_tab)
        screen = self.screen if pid else None
        busy = work.get("state") == "busy" or bool(screen and screen["busy"])
        if b.state in ("closed", "error", "start"):
            key = b.state
        elif b.state == "recording" or (screen and screen["rec"]):
            key = "rec"
        elif b.state == "wake":
            key = "wake"
        elif waiting:
            key = "wait"
        elif screen and screen["dialog"]:
            key = "dialog"
        elif listener.speaking():
            key = "speak"
        elif busy:
            key = "busy"
        elif b.paused:
            key = "paused"
        else:
            key = "idle"
        self.state_key = key
        status = self.t["st_" + key]
        if key == "busy":
            secs = int(time.time() - work.get("since", time.time())) if work.get("state") == "busy" else 0
            status = self.t["busy_for"].format(f"{secs // 60}:{secs % 60:02d}" if secs else "").strip()
        self.status.configure(text=status, foreground=self.theme.p[STATE_COLORS[key]])
        self.pause_btn.configure(text=self.t["resume"] if b.paused else self.t["pause"])
        self.hint.configure(text=self.command_hint(key))
        if key == "rec":
            self.talk_btn.configure(text="➤  " + self.t["send_rec"])
            if not self.cancel_btn.winfo_ismapped():
                self.cancel_btn.pack(side="left", padx=(self.theme.px(6), 0), after=self.talk_btn)
        else:
            self.talk_btn.configure(text="🎤  " + self.t["talk"])
            if self.cancel_btn.winfo_ismapped():
                self.cancel_btn.pack_forget()

        # pasek na zywo: co dyktujesz (tekst z pola Claude, a zanim sie pojawi - to, co slyszy Vosk)
        if key == "rec":
            end = (self.cfg["wake"].get("end") or ["wykonaj"])[0]
            heard = (screen and screen["input"]) or b.heard
            live = f"„{heard}”" if heard else self.t["live_rec"].format(end)
        elif key == "busy":
            live = (screen and screen["activity"]) or ""
        else:
            live = ""
        if live != self.live_shown:
            self.live_shown = live
            self.live_label.configure(text=live)
            if live and not self.live.winfo_ismapped():
                px = self.theme.px
                self.live.pack(fill="x", padx=px(12), pady=(0, px(6)), after=self.band.c)
            elif not live and self.live.winfo_ismapped():
                self.live.pack_forget()
        if self.term_open:
            self.show_terminal(screen)
        dialog_keys = key in ("dialog", "wait")
        if dialog_keys != self.term_keys.winfo_ismapped():
            self.term_keys.pack(side="left") if dialog_keys else self.term_keys.pack_forget()

        # okno wyboru w konsoli (np. nowy serwer MCP przy starcie) - bez odpowiedzi Achaja nie nagra ani nie odpowie
        problem, b.problem = b.problem, None
        if key == "dialog" or problem:
            if not self.dialog_seen:
                self.dialog_seen = True
                self.nb.select(self.term_tab)
                self.add_note(self.t["rec_failed"] if problem == "rec_failed" else self.t["dialog_note"])
                if key == "dialog" and not waiting:  # o zgodzie na narzedzie mowi juz hook Notification
                    listener.say(self.t["dialog_say"])
        else:
            self.dialog_seen = False
        # zamknieta / zabita sesja Claude wraca sama (z ta sama rozmowa), ale nie czesciej niz co 30 s
        if (key == "closed" and self.opts.get("restart_closed_session", True) and not self.quitting
                and time.time() - self.last_start > 30):
            self.add_note(self.t["st_closed"])
            self.start_listener(resume=True)

    def command_hint(self, key):
        """Komendy glosowe dla obecnego stanu - slowa z config.json (wybrany jezyk)."""
        w, t = self.cfg["wake"], self.t
        first = lambda name, default: (w.get(name) or [default])[0]  # noqa: E731
        if key in ("rec", "wake"):
            return t["hint_rec"].format(first("end", "wykonaj"), first("cancel", "anuluj"), first("clear", "nowa rozmowa"))
        if key in ("wait", "dialog"):
            return t["hint_dialog"]
        if key == "paused":
            return t["hint_paused"]
        if key in ("start", "closed", "error"):
            return t["hint_start"]
        return t["hint_idle"].format(t["achaja"])

    def animate(self):
        """Awatar: slucha, mysli, mowi albo czeka - animacja tylko przy widocznym oknie."""
        if self.visible:
            p, key = self.theme.p, self.state_key
            try:
                self.avatar.draw(key, p[STATE_COLORS[key]], p)
                self.band.draw(p, p[STATE_COLORS[key]], active=key in ("rec", "wake", "busy", "speak"))
            except Exception as exc:
                self.logs.put(f"{time.strftime('%H:%M:%S')} BLAD awatara: {exc!r}")
        self.root.after(60, self.animate)

    # --- okno, zasobnik, koniec ---
    def start_tray(self):
        if not self.opts.get("tray", True):
            return
        try:
            import pystray
            from PIL import Image
        except ImportError:
            return  # bez pystray okno po prostu minimalizuje sie na pasek zadan
        t = self.t
        image = Image.open(str(ICON)) if ICON.exists() else Image.new("RGB", (64, 64), ACCENT)

        def call(fn):
            return lambda icon, item: self.calls.put(fn)

        menu = pystray.Menu(
            pystray.MenuItem(t["show"], call(self.show_window), default=True),
            pystray.MenuItem(t["talk"], call(self.talk)),
            pystray.MenuItem(lambda item: t["resume"] if self.bridge and self.bridge.paused else t["pause"],
                             call(self.toggle_pause)),
            pystray.Menu.SEPARATOR,
            pystray.MenuItem(t["quit"], call(lambda: self.quit(ask=False))),
        )
        self.tray = pystray.Icon("Achaja", image, "Achaja", menu)
        self.tray.run_detached()

    def show_window(self):
        self.root.deiconify()
        self.root.lift()
        self.root.focus_force()

    def hide_window(self):
        if self.tray:
            self.root.withdraw()
        else:
            self.root.iconify()

    def on_close(self):
        if self.tray and self.opts.get("close_to_tray", True):
            self.hide_window()
        else:
            self.quit()

    def quit(self, ask=True, install_now=False):
        """install_now: po zamknieciu instaluje pobrana wersje i uruchamia Achaje ponownie.
        Bez tego pobrana wersja instaluje sie przy zwyklym zamknieciu (update_install_on_quit)."""
        from tkinter import messagebox
        if ask and not messagebox.askyesno("Achaja", self.t["quit_ask"], parent=self.root):
            return
        self.quitting = True
        write_json(WINDOW_FILE, {"geometry": self.root.geometry(), "layout": LAYOUT})
        pid = self.claude_pid()
        if self.bridge:
            self.bridge.stop.set()
        if pid and self.bridge and self.bridge.hidden:
            import console
            if not console.is_visible(pid):  # ukryta sesja bez okna zostalaby w tle na zawsze
                subprocess.run(["taskkill", "/PID", str(pid), "/T", "/F"], capture_output=True,
                               stdin=subprocess.DEVNULL, creationflags=CREATE_NO_WINDOW)
        if self.tray:
            self.tray.stop()
        self.root.destroy()
        if self.update_path and (install_now or self.opts.get("update_install_on_quit", True)):
            update.run_installer(self.update_path, relaunch=install_now)

    def run(self):
        self.root.mainloop()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--continue", dest="resume", action="store_true")
    ap.add_argument("--minimized", action="store_true")
    args = ap.parse_args()

    try:
        ctypes.windll.shell32.SetCurrentProcessExplicitAppUserModelID("Banzamel.Achaja")  # wlasna ikona na pasku
        ctypes.windll.shcore.SetProcessDpiAwareness(1)
    except (AttributeError, OSError):
        pass

    if not single_instance("AchajaListener"):
        if pid_alive(read_json(STATE / "app.json", {}).get("pid", 0)):
            write_json(SHOW_FLAG, {"at": time.time()})  # pierwsza kopia sama pokaze swoje okno
        else:
            from tkinter import Tk, messagebox
            root = Tk()
            root.withdraw()
            messagebox.showinfo("Achaja", TEXT.get(load_config().get("language"), TEXT["en"])["already"])
        return
    write_json(STATE / "app.json", {"pid": os.getpid()})
    App(args).run()


if __name__ == "__main__":
    main()
