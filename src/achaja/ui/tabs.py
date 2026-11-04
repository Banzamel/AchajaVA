"""Zakladki okna Achai: "Plan" (przypomnienia, polecenia i zadania w tle) i "Claude" (konto, limity planu,
sesje Claude Code na tym komputerze, miejsce na dysku, porzadki). Uzywa ich ui/app.py.

Usuwanie z zakladki Claude robi uzytkownik przyciskiem i potwierdza je w oknie dialogowym - to nie przechodzi
przez sesje Claude, wiec nie dotyczy go bramka (hooks/gate.py).
"""
import os
import subprocess
import threading
import time

from achaja.claude import data as claude_data
from achaja.plan import jobs
from achaja.plan import plan
from achaja.core.common import CREATE_NO_WINDOW, DETACHED_PROCESS
from achaja.ui.theme import FONT, FONT_BOLD

TEXT = {
    "pl": {
        "plan": "Plan", "claude": "Claude", "refresh": "Odśwież", "delete": "Usuń", "run_now": "Uruchom teraz",
        "plan_empty": "Nic nie jest zaplanowane. Powiedz np. „przypomnij mi o 15…” albo „codziennie o 8 przejrzyj pocztę”.",
        "plan_when": "Kiedy", "plan_what": "Co", "plan_pick": "Wybierz pozycję, żeby zobaczyć szczegóły i ostatni wynik.",
        "plan_del_ask": "Usunąć z planu: {}?", "plan_started": "Uruchomiłam zadanie „{}”: {}",
        "no_result": "To zadanie jeszcze się nie wykonało.",
        "acct_none": "Nie widzę zalogowanego konta claude.ai", "usage_err": "Limity: {}",
        "usage_title": "Wykorzystanie planu", "sessions": "Sesje Claude na tym komputerze",
        "col_name": "Projekt / sesja", "col_size": "Rozmiar", "col_date": "Ostatnio",
        "clean": "Wyczyść starsze niż {} dni", "open_dir": "Folder",
        "total": "Razem {} w ~/.claude · sesje {}", "loading": "Wczytuję…",
        "del_session": "Usunąć zapis sesji „{}”? Nie da się jej potem wznowić.",
        "del_project": "Usunąć wszystkie sesje Claude projektu\n{}?\nPliki projektu zostaną nietknięte.",
        "del_many": "Usunąć zaznaczone: sesje ({}) i wszystkie sesje projektów ({})?\n"
                    "Nie da się ich potem wznowić. Pliki projektów zostaną nietknięte.",
        "clean_ask": "Usunąć {} sesji starszych niż {} dni i {} starych plików pomocniczych ({})?",
        "clean_none": "Nie ma nic starszego niż {} dni.", "freed": "Zwolniono {}.",
        "active": "aktywna", "current": "bieżąca Achai", "agent": "agent {}",
    },
    "en": {
        "plan": "Plan", "claude": "Claude", "refresh": "Refresh", "delete": "Delete", "run_now": "Run now",
        "plan_empty": "Nothing scheduled. Say e.g. “remind me at 3…” or “every day at 8 go through my mail”.",
        "plan_when": "When", "plan_what": "What", "plan_pick": "Pick an item to see its details and last result.",
        "plan_del_ask": "Remove from the plan: {}?", "plan_started": "Started the task “{}”: {}",
        "no_result": "This task hasn't run yet.",
        "acct_none": "No claude.ai account signed in", "usage_err": "Usage: {}",
        "usage_title": "Plan usage", "sessions": "Claude sessions on this computer",
        "col_name": "Project / session", "col_size": "Size", "col_date": "Last",
        "clean": "Clean older than {} days", "open_dir": "Folder",
        "total": "{} in ~/.claude · sessions {}", "loading": "Loading…",
        "del_session": "Delete the session “{}”? It can't be resumed afterwards.",
        "del_project": "Delete all Claude sessions of\n{}?\nThe project's own files stay untouched.",
        "del_many": "Delete the selected: sessions ({}) and all sessions of projects ({})?\n"
                    "They can't be resumed afterwards. The projects' own files stay untouched.",
        "clean_ask": "Delete {} sessions older than {} days and {} old helper files ({})?",
        "clean_none": "Nothing older than {} days.", "freed": "Freed {}.",
        "active": "active", "current": "Achaja's current", "agent": "agent {}",
    },
}
CLEAN_DAYS = 30


def lang_of(app):
    lang = app.cfg.get("language", "pl")
    return lang if lang in TEXT else "en"


def ask(app, text):
    from tkinter import messagebox
    return messagebox.askyesno("Achaja", text, parent=app.root)


def when(ts):
    return time.strftime("%d.%m %H:%M", time.localtime(ts))


class PlanTab:
    def __init__(self, app, nb):
        tk, ttk, px = app.tk, app.ttk, app.theme.px
        self.app, self.t = app, TEXT[lang_of(app)]
        self.frame = frame = ttk.Frame(nb, padding=(px(8), 0, px(8), px(4)))
        nb.add(frame, text=self.t["plan"])
        top = ttk.Frame(frame, style="Card.TFrame", padding=px(6))
        top.pack(fill="x")
        self.tree = ttk.Treeview(top, columns=("when",), show="tree headings", height=6, selectmode="browse")
        self.tree.heading("#0", text=self.t["plan_what"], anchor="w")
        self.tree.heading("when", text=self.t["plan_when"], anchor="w")
        self.tree.column("#0", width=px(260), stretch=True)
        self.tree.column("when", width=px(110), stretch=False)
        self.tree.pack(fill="x")
        self.tree.bind("<<TreeviewSelect>>", lambda e: self.show())
        bar = ttk.Frame(frame)
        bar.pack(fill="x", pady=(px(6), 0))
        self.run_btn = ttk.Button(bar, text=self.t["run_now"], style="Key.TButton", command=self.run_now)
        self.run_btn.pack(side="left")
        ttk.Button(bar, text=self.t["delete"], style="Key.TButton", command=self.remove).pack(side="left",
                                                                                           padx=(px(6), 0))
        card = ttk.Frame(frame, style="Card.TFrame", padding=(px(10), px(8), px(4), px(8)))
        card.pack(fill="both", expand=True, pady=(px(8), 0))
        self.text = tk.Text(card, wrap="word", state="disabled", padx=px(4), pady=px(2), font=(FONT, 9))
        sb = ttk.Scrollbar(card, command=self.text.yview, style="Card.Vertical.TScrollbar")
        self.text.configure(yscrollcommand=sb.set)
        sb.pack(side="right", fill="y")
        self.text.pack(side="left", fill="both", expand=True)
        self.rows, self.shown = None, None

    def paint(self):
        self.app.theme.text(self.text)

    def refresh(self):
        items = plan.items()
        rows = [(str(i["id"]), i) for i in items]
        key = [(r, i["next"]) for r, i in rows]
        if key != self.rows:
            sel = self.tree.selection()
            self.tree.delete(*self.tree.get_children())
            for iid, i in rows:
                icon = {"say": "🔔", "prompt": "💬", "job": "⚙"}.get(i["kind"], "•")
                label = f"{icon}  {i.get('name') or i['text']}"
                self.tree.insert("", "end", iid=iid, text=label, values=(when(i["next"]) + ("  ↻" if i.get("repeat") else ""),))
            if sel and self.tree.exists(sel[0]):
                self.tree.selection_set(sel[0])
            self.rows = key
            self.shown = None
        self.show()

    def selected(self):
        sel = self.tree.selection()
        return plan.find(sel[0]) if sel else None

    def show(self):
        item = self.selected()
        key = (item["id"], item["next"], self.result_mtime(item)) if item else ("none", len(self.rows or []))
        if key == self.shown:
            return
        self.shown = key
        if item:
            text = plan.describe(item)
            if item["kind"] == "job":
                path = jobs.JOBS / f"{item['id']}-last.md"
                text += "\n\n" + (path.read_text(encoding="utf-8", errors="replace") if path.exists()
                                  else self.t["no_result"])
        else:
            text = self.t["plan_pick"] if self.rows else self.t["plan_empty"]
        self.run_btn.state(["!disabled"] if item and item["kind"] == "job" else ["disabled"])
        self.text.configure(state="normal")
        self.text.delete("1.0", "end")
        self.text.insert("end", text)
        self.text.configure(state="disabled")

    @staticmethod
    def result_mtime(item):
        try:
            return (jobs.JOBS / f"{item['id']}-last.md").stat().st_mtime
        except OSError:
            return 0

    def run_now(self):
        item = self.selected()
        if item and item["kind"] == "job":
            self.app.add_note(self.t["plan_started"].format(item.get("name"), jobs.start(item)))

    def remove(self):
        item = self.selected()
        if item and ask(self.app, self.t["plan_del_ask"].format(item.get("name") or item["text"])):
            plan.cmd_remove(str(item["id"]))
            self.rows = None
            self.refresh()


class ClaudeTab:
    def __init__(self, app, nb):
        tk, ttk, px = app.tk, app.ttk, app.theme.px
        self.app = app
        self.lang = lang_of(app)
        self.t = TEXT[self.lang]
        self.frame = frame = ttk.Frame(nb, padding=(px(8), 0, px(8), px(4)))
        nb.add(frame, text=self.t["claude"])
        card = ttk.Frame(frame, style="Card.TFrame", padding=(px(12), px(8), px(10), px(8)))
        card.pack(fill="x")
        head = ttk.Frame(card, style="Plain.TFrame")
        head.pack(fill="x")
        ttk.Button(head, text=self.t["refresh"], style="Card.Key.TButton", command=self.refresh).pack(side="right")
        self.acct = ttk.Label(head, style="CardTitle.TLabel", text=self.t["loading"])
        self.acct.pack(side="left")
        self.acct_sub = ttk.Label(card, style="CardMuted.TLabel", text="")
        self.acct_sub.pack(fill="x", anchor="w")
        self.bars = tk.Canvas(card, height=px(40), highlightthickness=0, borderwidth=0)
        self.bars.pack(fill="x", pady=(px(6), 0))
        self.bars.bind("<Configure>", lambda e: self.draw_bars())
        # przyciski najpierw na dole - inaczej przy niskim oknie lista je wypycha i "Usun" znika
        bar = ttk.Frame(frame)
        bar.pack(side="bottom", fill="x", pady=(px(6), 0))
        self.del_btn = ttk.Button(bar, text=self.t["delete"], style="Key.TButton", command=self.delete)
        self.del_btn.pack(side="left")
        ttk.Button(bar, text=self.t["clean"].format(CLEAN_DAYS), style="Key.TButton",
                   command=self.clean).pack(side="left", padx=(px(6), 0))
        ttk.Button(bar, text=self.t["open_dir"], style="Key.TButton", command=self.open_dir).pack(side="right")
        lower = ttk.Frame(frame, style="Card.TFrame", padding=px(6))
        lower.pack(fill="both", expand=True, pady=(px(8), 0))
        ttk.Label(lower, style="CardTitle.TLabel", text=self.t["sessions"]).pack(anchor="w", padx=px(4))
        self.total = ttk.Label(lower, style="CardMuted.TLabel", text="")
        self.total.pack(anchor="w", padx=px(4), pady=(0, px(4)))
        wrap = ttk.Frame(lower, style="Plain.TFrame")
        wrap.pack(fill="both", expand=True)
        # kilka naraz: Ctrl+klik, Shift+klik; Delete albo prawy przycisk -> Usun
        self.tree = ttk.Treeview(wrap, columns=("size", "date"), show="tree headings", selectmode="extended")
        self.tree.heading("#0", text=self.t["col_name"], anchor="w")
        self.tree.heading("size", text=self.t["col_size"], anchor="e")
        self.tree.heading("date", text=self.t["col_date"], anchor="w")
        self.tree.column("#0", width=px(220), stretch=True)
        self.tree.column("size", width=px(70), anchor="e", stretch=False)
        self.tree.column("date", width=px(85), stretch=False)
        sb = ttk.Scrollbar(wrap, command=self.tree.yview, style="Card.Vertical.TScrollbar")
        self.tree.configure(yscrollcommand=sb.set)
        sb.pack(side="right", fill="y")
        self.tree.pack(side="left", fill="both", expand=True)
        self.tree.bind("<<TreeviewOpen>>", self.on_open)
        self.tree.bind("<<TreeviewSelect>>", lambda e: self.update_delete())
        self.tree.bind("<Delete>", lambda e: self.delete())
        self.tree.bind("<Button-3>", self.context_menu)
        self.update_delete()
        self.account, self.windows, self.usage_error = None, [], ""
        self.busy, self.loaded_at, self.projects = False, 0, {}

    def paint(self):
        self.bars.configure(background=self.app.theme.p["surface"])
        self.draw_bars()

    # --- dane w tle, rysowanie w watku okna (app.calls) ---
    def refresh(self, usage_only=False):
        if self.busy:
            return
        self.busy = True

        def work():
            try:
                account = claude_data.account()
                try:
                    windows, err = claude_data.usage(), ""
                except claude_data.DataError as exc:
                    windows, err = [], str(exc)
                projects = None if usage_only else claude_data.projects()
                total = None if usage_only else claude_data.size_of(claude_data.HOME)
            except Exception as exc:  # brak ~/.claude itp. - pokazujemy, co sie da
                account, windows, err, projects, total = {}, [], str(exc), None, None
            self.app.calls.put(lambda: self.apply(account, windows, err, projects, total))

        threading.Thread(target=work, daemon=True).start()

    def apply(self, account, windows, err, projects, total):
        self.busy, self.loaded_at = False, time.time()
        self.account, self.windows, self.usage_error = account, windows, err
        if account.get("email"):
            self.acct.configure(text=account["email"])
            sub = " · ".join(x for x in (f"Claude {account['plan']}" if account.get("plan") else "",
                                         account.get("org") or "") if x)
        else:
            self.acct.configure(text=self.t["acct_none"])
            sub = ""
        if err:
            sub = (sub + " · " if sub else "") + self.t["usage_err"].format(err)
        self.acct_sub.configure(text=sub)
        self.draw_bars()
        if projects is not None:
            self.fill_projects(projects, total)

    def draw_bars(self):
        c, th = self.bars, self.app.theme
        p, px = th.p, th.px
        c.delete("all")
        width = max(c.winfo_width(), px(200))
        row = px(30)
        c.configure(height=max(row * len(self.windows), px(4)))
        for i, w in enumerate(self.windows):
            y = i * row
            pct = max(0.0, min(100.0, w["percent"]))
            color = p["green"] if pct < 70 else p["orange"] if pct < 90 else p["red"]
            c.create_text(0, y + px(7), anchor="w", text=claude_data.window_label(w["key"], self.lang),
                          fill=p["text"], font=(FONT_BOLD, 9))
            c.create_text(width, y + px(7), anchor="e", fill=p["muted"], font=(FONT, 8),
                          text=f"{pct:.0f}%  " + claude_data.reset_text(w.get("resets_at"), self.lang))
            top, h = y + px(16), px(7)
            c.create_rectangle(0, top, width, top + h, fill=p.get("surface2", p["border"]), outline="")
            if pct > 0:
                c.create_rectangle(0, top, max(px(3), width * pct / 100), top + h, fill=color, outline="")

    def fill_projects(self, projects, total):
        tree = self.tree
        tree.delete(*tree.get_children())
        self.projects = {p["id"]: p for p in projects}
        sessions_size = sum(p["size"] for p in projects)
        self.total.configure(text=self.t["total"].format(claude_data.human(total or 0),
                                                         claude_data.human(sessions_size)))
        for p in projects:
            name = os.path.basename(str(p["path"]).rstrip("\\/")) or p["path"]
            tree.insert("", "end", iid=f"p:{p['id']}", text=f"{name}  ({p['sessions']})",
                        values=(claude_data.human(p["size"]), when(p["last"])))
            tree.insert(f"p:{p['id']}", "end", iid=f"wait:{p['id']}", text=self.t["loading"])

    def on_open(self, event):
        iid = self.tree.focus()
        if not iid.startswith("p:") or not self.tree.exists(f"wait:{iid[2:]}"):
            return
        pid = iid[2:]

        def work():
            items = claude_data.sessions(pid)
            self.app.calls.put(lambda: self.fill_sessions(pid, items))

        threading.Thread(target=work, daemon=True).start()

    def fill_sessions(self, pid, items):
        parent = f"p:{pid}"
        if not self.tree.exists(parent):
            return
        self.tree.delete(*self.tree.get_children(parent))
        for s in items:
            flags = [self.t["current"]] if s["current"] else [self.t["active"]] if s["active"] else []
            if s["agent"]:
                flags.append(self.t["agent"].format(s["agent"]))
            label = ("● " if s["active"] or s["current"] else "") + s["title"][:70] + (
                f"  [{', '.join(flags)}]" if flags else "")
            self.tree.insert(parent, "end", iid=f"s:{pid}:{s['id']}", text=label,
                             values=(claude_data.human(s["size"]), when(s["mtime"])))

    def tick(self, visible):
        if visible and time.time() - self.loaded_at > 60:
            self.refresh(usage_only=bool(self.projects))

    # --- akcje uzytkownika ---
    def targets(self):
        """Zaznaczone projekty i sesje; sesje projektu, ktory sam jest zaznaczony, sa juz w nim."""
        sel = self.tree.selection()
        projects = [i[2:] for i in sel if i.startswith("p:")]
        sessions = [tuple(i.split(":", 2)[1:]) for i in sel if i.startswith("s:")]
        return projects, [(pid, sid) for pid, sid in sessions if pid not in projects]

    def update_delete(self):
        projects, sessions = self.targets()
        n = len(projects) + len(sessions)
        self.del_btn.configure(text=self.t["delete"] + (f" ({n})" if n > 1 else ""))
        self.del_btn.state(["!disabled"] if n else ["disabled"])

    def context_menu(self, event):
        import tkinter as tk
        row = self.tree.identify_row(event.y)
        if not row:
            return
        if row not in self.tree.selection():  # prawy klik poza zaznaczeniem - zaznacz ten wiersz
            self.tree.selection_set(row)
        menu = tk.Menu(self.tree, tearoff=0)
        self.app.theme.menu(menu)
        menu.add_command(label=self.del_btn.cget("text"), command=self.delete)
        menu.tk_popup(event.x_root, event.y_root)

    def delete(self):
        projects, sessions = self.targets()
        if not projects and not sessions:
            return
        if len(projects) + len(sessions) == 1:
            if projects:
                question = self.t["del_project"].format(self.projects.get(projects[0], {}).get("path", projects[0]))
            else:
                pid, sid = sessions[0]
                question = self.t["del_session"].format(self.tree.item(f"s:{pid}:{sid}", "text")[:80])
        else:
            question = self.t["del_many"].format(len(sessions), len(projects))
        if not ask(self.app, question):
            return
        freed, errors = 0, []
        for pid in projects:
            try:
                freed += claude_data.delete_project(pid)
            except claude_data.DataError as exc:
                errors.append(str(exc))
        for pid, sid in sessions:
            try:
                freed += claude_data.delete_session(pid, sid)
            except claude_data.DataError as exc:  # np. aktywna sesja albo biezaca rozmowa Achai
                errors.append(str(exc))
        for err in errors:
            self.app.add_note(err)
        self.app.add_note(self.t["freed"].format(claude_data.human(freed)))
        self.refresh()

    def clean(self):
        n, f, b = claude_data.clean(CLEAN_DAYS)
        if not n and not f:
            self.app.add_note(self.t["clean_none"].format(CLEAN_DAYS))
            return
        if ask(self.app, self.t["clean_ask"].format(n, CLEAN_DAYS, f, claude_data.human(b))):
            _, _, freed = claude_data.clean(CLEAN_DAYS, apply=True)
            self.app.add_note(self.t["freed"].format(claude_data.human(freed)))
            self.refresh()

    def open_dir(self):
        sel = self.tree.selection()
        path = claude_data.HOME
        if sel and sel[0].startswith("p:"):
            path = claude_data.PROJECTS / sel[0][2:]
        subprocess.Popen(["explorer.exe", str(path)], creationflags=DETACHED_PROCESS | CREATE_NO_WINDOW,
                         close_fds=True)
