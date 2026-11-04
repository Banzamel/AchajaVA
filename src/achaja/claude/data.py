"""Dane Claude Code na tym komputerze: konto, limity planu, sesje i miejsce na dysku.

Uzycie:
  python -m achaja claude account                       zalogowane konto (e-mail, organizacja, plan)
  python -m achaja claude usage                          wykorzystanie limitow planu (5 godzin, 7 dni...)
  python -m achaja claude summary                        konto + limity + miejsce, krotko (do powiedzenia)
  python -m achaja claude projects                       projekty z sesjami Claude: rozmiar, liczba sesji, ostatnia praca
  python -m achaja claude sessions <projekt>             sesje projektu (tytul, data, rozmiar, czy aktywna)
  python -m achaja claude storage                        ile zajmuja foldery ~/.claude
  python -m achaja claude delete-session <projekt> <id>  usuwa zapis sesji (nie da sie go potem wznowic)
  python -m achaja claude delete-project <projekt>       usuwa wszystkie sesje projektu (pliki projektu zostaja!)
  python -m achaja claude clean --days 30 [--apply]      stare sesje, logi debug, migawki powloki i listy zadan;
                                                bez --apply tylko pokazuje, co by usunal
Usuwanie dotyczy tylko zapisow Claude Code w ~/.claude - nigdy plikow w folderach projektow.
Aktywnych sesji (zapis w ostatnich 2 minutach) i biezacej sesji Achai nie usuwa.

Limity: ten sam odczyt co /usage w Claude Code (api.anthropic.com/api/oauth/usage) z tokenem
zalogowanego konta z ~/.claude/.credentials.json. Token nie jest wypisywany ani zapisywany.
"""
import argparse
import json
import os
import re
import shutil
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

from achaja.core.common import AGENTS_STATE, ROOT, STATE, __version__, read_json, write_json

HOME = Path(os.environ.get("CLAUDE_CONFIG_DIR") or Path.home() / ".claude")
GLOBAL_CONFIG = (HOME / ".claude.json") if os.environ.get("CLAUDE_CONFIG_DIR") else Path.home() / ".claude.json"
PROJECTS = HOME / "projects"
USAGE_URL = "https://api.anthropic.com/api/oauth/usage"
USAGE_CACHE = STATE / "claude_usage.json"
USAGE_TTL = 60
ACTIVE_SECONDS = 120
CLEAN_DIRS = ("debug", "shell-snapshots", "todos", "file-history")  # porzadki: pliki starsze niz N dni
PLAN_NAMES = {"max": "Max", "pro": "Pro", "team": "Team", "enterprise": "Enterprise", "free": "Free"}


class DataError(Exception):
    pass


# ---------- konto i limity ----------

def credentials():
    try:
        data = json.loads((HOME / ".credentials.json").read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return {}
    return data.get("claudeAiOauth") or {}


def account():
    """{email, name, org, plan, tier} zalogowanego konta (puste pola, gdy nie wiadomo)."""
    acct = read_json(GLOBAL_CONFIG, {}).get("oauthAccount") or {}
    cred = credentials()
    plan = str(cred.get("subscriptionType") or acct.get("subscriptionType") or "")
    tier = str(cred.get("rateLimitTier") or "")
    m = re.search(r"max_(\d+)x", tier)
    plan_name = PLAN_NAMES.get(plan.lower(), plan.title()) if plan else ""
    if plan_name == "Max" and m:
        plan_name += f" {m.group(1)}x"
    return {"email": acct.get("emailAddress") or "", "name": acct.get("displayName") or "",
            "org": acct.get("organizationName") or "", "plan": plan_name, "logged_in": bool(cred.get("accessToken"))}


def window_label(key, lang="pl"):
    pl = lang == "pl"
    base = {"five_hour": "5 godzin" if pl else "5 hours", "seven_day": "7 dni" if pl else "7 days"}
    for prefix, name in (("five_hour", base["five_hour"]), ("seven_day", base["seven_day"])):
        if key == prefix:
            return name
        if key.startswith(prefix + "_"):
            return f"{name} · {key[len(prefix) + 1:].replace('_', ' ').title()}"
    return key.replace("_", " ")


def usage(force=False):
    """[{key, label, percent, resets_at}] - wykorzystanie okien limitu planu. Wynik z ostatniej minuty z pamieci."""
    cache = read_json(USAGE_CACHE, {})
    if not force and time.time() - cache.get("at", 0) < USAGE_TTL and "windows" in cache:
        return cache["windows"]
    cred = credentials()
    token = cred.get("accessToken")
    if not token:
        raise DataError("Brak zalogowanego konta claude.ai w Claude Code (uruchom claude i zaloguj sie).")
    if cred.get("expiresAt") and cred["expiresAt"] / 1000 < time.time():
        raise DataError("Token Claude Code wygasl - odswiezy sie przy najblizszym uzyciu claude.")
    req = urllib.request.Request(USAGE_URL, headers={
        "Authorization": f"Bearer {token}", "anthropic-beta": "oauth-2025-04-20", "Accept": "application/json",
        "User-Agent": f"Achaja/{__version__} (claude-code usage)"})
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            raw = json.load(resp)
    except urllib.error.HTTPError as exc:
        raise DataError(f"Limity: HTTP {exc.code} z api.anthropic.com.")
    except (urllib.error.URLError, OSError, ValueError) as exc:
        raise DataError(f"Limity: brak polaczenia ({getattr(exc, 'reason', exc)}).")
    windows = []
    for key, val in (raw or {}).items():
        if isinstance(val, dict) and val.get("utilization") is not None and re.match(r"(five_hour|seven_day)", key):
            windows.append({"key": key, "percent": float(val["utilization"]), "resets_at": val.get("resets_at")})
    order = {"five_hour": 0, "seven_day": 1}
    windows.sort(key=lambda w: (order.get(w["key"], 2), w["key"]))
    write_json(USAGE_CACHE, {"at": time.time(), "windows": windows})
    return windows


def reset_text(iso, lang="pl"):
    """'za 2 h 15 min' / 'w pt 14:00' - kiedy okno sie odnowi."""
    if not iso:
        return ""
    try:
        from datetime import datetime
        when = datetime.fromisoformat(str(iso).replace("Z", "+00:00")).astimezone()
    except ValueError:
        return ""
    left = when.timestamp() - time.time()
    if left <= 0:
        return ""
    if left < 24 * 3600:
        h, m = int(left // 3600), int(left % 3600 // 60)
        return (f"odnowi się za {h} h {m} min" if lang == "pl" else f"resets in {h} h {m} min") if h else (
            f"odnowi się za {m} min" if lang == "pl" else f"resets in {m} min")
    days = ("pon", "wt", "śr", "czw", "pt", "sob", "nd") if lang == "pl" else ("Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun")
    return (f"odnowi się w {days[when.weekday()]} {when:%H:%M}" if lang == "pl"
            else f"resets {days[when.weekday()]} {when:%H:%M}")


# ---------- sesje i miejsce ----------

def size_of(path):
    total = 0
    try:
        if path.is_file():
            return path.stat().st_size
        for root, _, files in os.walk(path):
            for f in files:
                try:
                    total += os.path.getsize(os.path.join(root, f))
                except OSError:
                    pass
    except OSError:
        pass
    return total


def human(n):
    for unit in ("B", "KB", "MB", "GB"):
        if n < 1024 or unit == "GB":
            return f"{n:.0f} {unit}" if unit == "B" else f"{n:.1f} {unit}"
        n /= 1024


def session_info(path):
    """Tytul (pierwsza wiadomosc uzytkownika albo podsumowanie), katalog pracy, liczba wiadomosci."""
    title = summary = cwd = ""
    messages = 0
    try:
        with open(path, encoding="utf-8", errors="replace") as f:
            for i, line in enumerate(f):
                if i > 4000:
                    break
                try:
                    rec = json.loads(line)
                except ValueError:
                    continue
                cwd = cwd or rec.get("cwd") or ""
                if rec.get("type") == "summary" and rec.get("summary"):
                    summary = summary or rec["summary"]
                if rec.get("type") in ("user", "assistant") and not rec.get("isMeta"):
                    messages += 1
                if not title and rec.get("type") == "user" and not rec.get("isMeta"):
                    content = (rec.get("message") or {}).get("content")
                    if isinstance(content, list):
                        content = " ".join(c.get("text", "") for c in content if isinstance(c, dict))
                    text = " ".join(str(content or "").split())
                    if text and not text.startswith(("<", "[Request interrupted", "Caveat:")):
                        title = text[:120]
    except OSError:
        pass
    return {"title": summary or title or "(bez tytułu)", "cwd": cwd, "messages": messages}


def achaja_current():
    """Najnowsza sesja w projekcie Achai - biezaca rozmowa, nie do usuwania."""
    best = None
    for d in PROJECTS.glob("*") if PROJECTS.is_dir() else []:
        files = sorted(d.glob("*.jsonl"), key=lambda p: p.stat().st_mtime, reverse=True)
        if files and Path(session_info(files[0])["cwd"] or "/nonexistent").resolve() == ROOT.resolve():
            if best is None or files[0].stat().st_mtime > best.stat().st_mtime:
                best = files[0]
    return best.stem if best else None


def agent_sessions():
    return {sid: name for name, sid in read_json(AGENTS_STATE / "sessions.json", {}).items()}


def projects():
    """[{id, path, size, sessions, last}] - foldery w ~/.claude/projects, najnowsze najpierw."""
    out = []
    for d in PROJECTS.iterdir() if PROJECTS.is_dir() else []:
        if not d.is_dir():
            continue
        files = list(d.glob("*.jsonl"))
        newest = max(files, key=lambda p: p.stat().st_mtime) if files else None
        cwd = session_info(newest)["cwd"] if newest else ""
        out.append({"id": d.name, "path": cwd or d.name, "size": size_of(d), "sessions": len(files),
                    "last": newest.stat().st_mtime if newest else d.stat().st_mtime})
    return sorted(out, key=lambda p: p["last"], reverse=True)


def find_project(query):
    items = projects()
    for p in items:
        if query in (p["id"], p["path"]):
            return p
    q = query.lower()
    hits = [p for p in items if q in p["path"].lower() or q in p["id"].lower()]
    if len(hits) == 1:
        return hits[0]
    if not hits:
        raise DataError(f"Nie ma projektu '{query}' w {PROJECTS}.")
    raise DataError(f"Kilka projektow pasuje do '{query}': " + ", ".join(p["path"] for p in hits[:6]))


def sessions(project_id):
    """[{id, title, size, mtime, messages, active, current, agent}] sesji projektu, najnowsze najpierw."""
    d = PROJECTS / project_id
    current, agents, now = achaja_current(), agent_sessions(), time.time()
    out = []
    for f in d.glob("*.jsonl"):
        info = session_info(f)
        extra = d / f.stem  # podagenci, wyniki narzedzi
        mtime = f.stat().st_mtime
        out.append({"id": f.stem, "title": info["title"], "messages": info["messages"],
                    "size": f.stat().st_size + (size_of(extra) if extra.is_dir() else 0), "mtime": mtime,
                    "active": now - mtime < ACTIVE_SECONDS, "current": f.stem == current,
                    "agent": agents.get(f.stem, "")})
    return sorted(out, key=lambda s: s["mtime"], reverse=True)


def storage():
    """[(nazwa, rozmiar)] folderow ~/.claude, najwieksze najpierw."""
    rows = []
    for p in HOME.iterdir() if HOME.is_dir() else []:
        if p.name == ".credentials.json":
            continue
        rows.append((p.name, size_of(p)))
    return sorted(rows, key=lambda r: r[1], reverse=True)


def _remove(path):
    if path.is_dir():
        shutil.rmtree(path, ignore_errors=True)
    elif path.exists():
        path.unlink()


def session_files(project_id, sid):
    """Wszystko, co Claude Code trzyma o jednej sesji."""
    paths = [PROJECTS / project_id / f"{sid}.jsonl", PROJECTS / project_id / sid, HOME / "file-history" / sid,
             HOME / "session-env" / sid]
    todos = HOME / "todos"
    if todos.is_dir():
        paths += [p for p in todos.glob(f"{sid}*")]
    return [p for p in paths if p.exists()]


def forget_agent(sid):
    path = AGENTS_STATE / "sessions.json"
    data = read_json(path, {})
    kept = {k: v for k, v in data.items() if v != sid}
    if kept != data:
        write_json(path, kept)  # agent zacznie nowa rozmowe zamiast wznawiac usunieta


def delete_session(project_id, sid, force=False):
    match = next((s for s in sessions(project_id) if s["id"] == sid), None)
    if not match:
        raise DataError(f"Nie ma sesji {sid} w projekcie {project_id}.")
    if match["current"]:
        raise DataError("To biezaca rozmowa Achai - nie usuwam jej.")
    if match["active"] and not force:
        raise DataError("Ta sesja wlasnie pracuje (zapis w ostatnich 2 minutach) - nie usuwam jej.")
    freed = 0
    for p in session_files(project_id, sid):
        freed += size_of(p)
        _remove(p)
    forget_agent(sid)
    return freed


def delete_project(project_id):
    items = sessions(project_id)
    busy = [s for s in items if s["current"] or s["active"]]
    if busy:
        raise DataError("W tym projekcie jest aktywna sesja albo biezaca rozmowa Achai - usun pojedyncze sesje.")
    freed = sum(delete_session(project_id, s["id"], force=True) for s in items)
    d = PROJECTS / project_id
    freed += size_of(d)
    _remove(d)
    return freed


def clean(days, apply=False):
    """Sesje i porzadki starsze niz 'days' dni. Zwraca (liczba sesji, liczba innych plikow, bajty)."""
    limit = time.time() - days * 86400
    n_sessions = n_files = freed = 0
    for p in projects():
        for s in sessions(p["id"]):
            if s["mtime"] < limit and not s["current"] and not s["active"]:
                n_sessions += 1
                freed += s["size"]
                if apply:
                    delete_session(p["id"], s["id"])
        d = PROJECTS / p["id"]
        if apply and d.is_dir() and not any(d.iterdir()):
            d.rmdir()
    for name in CLEAN_DIRS:
        base = HOME / name
        for child in base.iterdir() if base.is_dir() else []:
            try:
                if child.stat().st_mtime < limit:
                    n_files += 1
                    freed += size_of(child)
                    if apply:
                        _remove(child)
            except OSError:
                pass
    return n_sessions, n_files, freed


def summary(lang="pl"):
    """Kilka zdan do powiedzenia: konto, limity, miejsce."""
    a = account()
    parts = []
    if a["email"]:
        parts.append(f"Konto {a['email']}" + (f", plan {a['plan']}" if a["plan"] else "") + ".")
    try:
        for w in usage():
            parts.append(f"Limit {window_label(w['key'])}: wykorzystane {round(w['percent'])} procent"
                         + (f", {reset_text(w['resets_at'])}" if reset_text(w["resets_at"]) else "") + ".")
    except DataError as exc:
        parts.append(str(exc))
    total = size_of(PROJECTS)
    parts.append(f"Sesje Claude zajmują {human(total)} w {len(projects())} projektach.")
    return " ".join(parts)


def main(argv=None):
    ap = argparse.ArgumentParser(description="Dane Claude Code: konto, limity, sesje, miejsce")
    sub = ap.add_subparsers(dest="cmd", required=True)
    for name in ("account", "usage", "summary", "projects", "storage"):
        sub.add_parser(name)
    sub.add_parser("sessions").add_argument("project")
    d = sub.add_parser("delete-session")
    d.add_argument("project")
    d.add_argument("session")
    sub.add_parser("delete-project").add_argument("project")
    c = sub.add_parser("clean")
    c.add_argument("--days", type=int, default=30)
    c.add_argument("--apply", action="store_true")
    args = ap.parse_args(argv)
    try:
        if args.cmd == "account":
            a = account()
            print(f"E-mail: {a['email'] or '-'}\nNazwa: {a['name'] or '-'}\nOrganizacja: {a['org'] or '-'}\n"
                  f"Plan: {a['plan'] or '-'}\nZalogowane: {'tak' if a['logged_in'] else 'nie'}")
        elif args.cmd == "usage":
            for w in usage(force=True):
                print(f"{window_label(w['key'])}: {w['percent']:.0f}%  {reset_text(w['resets_at'])}")
        elif args.cmd == "summary":
            print(summary())
        elif args.cmd == "projects":
            for p in projects():
                print(f"{human(p['size']):>9}  {p['sessions']:>4} sesji  {time.strftime('%d.%m.%Y %H:%M', time.localtime(p['last']))}"
                      f"  {p['path']}   [{p['id']}]")
        elif args.cmd == "sessions":
            p = find_project(args.project)
            for s in sessions(p["id"]):
                flag = "BIEZACA ACHAI" if s["current"] else "AKTYWNA" if s["active"] else (
                    f"agent {s['agent']}" if s["agent"] else "")
                print(f"{s['id']}  {time.strftime('%d.%m.%Y %H:%M', time.localtime(s['mtime']))}  "
                      f"{human(s['size']):>9}  {s['messages']:>4} wiad.  {flag:13}  {s['title']}")
        elif args.cmd == "storage":
            for name, size in storage():
                print(f"{human(size):>9}  {name}")
        elif args.cmd == "delete-session":
            p = find_project(args.project)
            print(f"Usunieto sesje {args.session} ({human(delete_session(p['id'], args.session))}).")
        elif args.cmd == "delete-project":
            p = find_project(args.project)
            print(f"Usunieto sesje projektu {p['path']} ({human(delete_project(p['id']))}). Pliki projektu zostaly.")
        elif args.cmd == "clean":
            n, f, b = clean(args.days, apply=args.apply)
            verb = "Usunieto" if args.apply else "Do usuniecia (dodaj --apply)"
            print(f"{verb}: {n} sesji starszych niz {args.days} dni i {f} innych plikow - {human(b)}.")
    except DataError as exc:
        print(f"BLAD: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
