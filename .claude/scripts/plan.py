"""Przypomnienia i zadania o czasie - Achaja mowi przypomnienie albo sama wysyla sobie polecenie.

Uzycie:
  plan.py add --say "tekst"    KIEDY     przypomnienie czytane na glos
  plan.py add --prompt "tekst" KIEDY     polecenie dla Achai o czasie (np. "co nowego w poczcie",
                                          "niech agent od sklepu sprawdzi bledy") - jak wpisane w okno czatu
  plan.py list                           co jest zaplanowane
  plan.py remove <id>|all                usuwa

KIEDY (jedno z):
  --in 30m | 2h | 1d | 1h30m             za tyle czasu
  --at 15:30 | "2026-11-04 15:30"        o godzinie (dzis, a gdy juz minela - jutro) albo w dniu
  --daily 08:00                          codziennie
  --weekdays 08:00                       w dni robocze
  --weekly "pon 09:00"                   co tydzien (pon wt sr czw pt sob nd / mon tue wed thu fri sat sun)

Wykonuje listener (w oknie Achai): przypomnienie mowi od razu (poczeka, az skonczysz nagrywac),
polecenie wysyla, gdy Achaja nic nie robi. Polecenie dostaje prefiks [ZAPLANOWANE] - nie jest
zgoda na akcje z bramki (git push, usuwanie, wysylka...), wiec Achaja i tak zapyta.
Przypomnienie, ktore przegapilo swoj czas (komputer byl wylaczony), mowi przy starcie z dopiskiem;
powtarzane zadanie spoznione o ponad godzine czeka na nastepny termin.
"""
import argparse
import re
import sys
import time
from datetime import datetime, timedelta

from common import STATE, read_json, write_json

PLAN = STATE / "plan.json"
SCHEDULED = "[ZAPLANOWANE]"  # prefiks polecen - hook_gate.py nie traktuje ich jako zgody
LATE_REPEAT = 3600
DAYS = {"pon": 0, "pn": 0, "mon": 0, "wt": 1, "wto": 1, "tue": 1, "sr": 2, "sro": 2, "wed": 2, "czw": 3,
        "thu": 3, "pt": 4, "pia": 4, "fri": 4, "sob": 5, "so": 5, "sat": 5, "nd": 6, "nie": 6, "ndz": 6, "sun": 6}
DAY_NAMES = ("pon", "wt", "sr", "czw", "pt", "sob", "nd")


def hhmm(text):
    m = re.fullmatch(r"\s*(\d{1,2})[:.](\d{2})\s*", text or "")
    if not m or int(m.group(1)) > 23 or int(m.group(2)) > 59:
        raise ValueError(f"Godzina w formacie GG:MM (jest: '{text}').")
    return int(m.group(1)), int(m.group(2))


def next_time(repeat, after):
    """Najblizszy termin powtarzanego zadania po chwili 'after' (datetime)."""
    kind, _, spec = repeat.partition(" ")
    if kind == "weekly":
        day, _, clock = spec.partition(" ")
        h, m = hhmm(clock)
        cand = after.replace(hour=h, minute=m, second=0, microsecond=0) + timedelta(days=(int(day) - after.weekday()) % 7)
        return cand if cand > after else cand + timedelta(days=7)
    h, m = hhmm(spec)
    cand = after.replace(hour=h, minute=m, second=0, microsecond=0)
    while cand <= after or (kind == "weekdays" and cand.weekday() > 4):
        cand += timedelta(days=1)
    return cand


def parse_when(args, now):
    """(pierwszy termin, powtarzanie albo None)."""
    if args.in_:
        parts = re.findall(r"(\d+)\s*([dhm])", args.in_.lower())
        if not parts or re.sub(r"[\d\sdhm]", "", args.in_.lower()):
            raise ValueError("--in w formacie 30m, 2h, 1d albo 1h30m.")
        secs = sum(int(n) * {"d": 86400, "h": 3600, "m": 60}[u] for n, u in parts)
        return now + timedelta(seconds=secs), None
    if args.at:
        text = args.at.strip()
        if re.fullmatch(r"\d{1,2}[:.]\d{2}", text):
            h, m = hhmm(text)
            cand = now.replace(hour=h, minute=m, second=0, microsecond=0)
            return (cand if cand > now else cand + timedelta(days=1)), None
        try:
            return datetime.strptime(text.replace("T", " "), "%Y-%m-%d %H:%M"), None
        except ValueError:
            raise ValueError("--at w formacie GG:MM albo 'RRRR-MM-DD GG:MM'.")
    if args.daily:
        repeat = f"daily {'%02d:%02d' % hhmm(args.daily)}"
    elif args.weekdays:
        repeat = f"weekdays {'%02d:%02d' % hhmm(args.weekdays)}"
    elif args.weekly:
        day, _, clock = args.weekly.strip().partition(" ")
        key = day.lower().replace("ś", "s").replace("ą", "a").replace("ę", "e")[:3].rstrip(".")
        key = key if key in DAYS else key[:2]
        if key not in DAYS:
            raise ValueError("--weekly 'pon 09:00' (pon wt sr czw pt sob nd).")
        repeat = f"weekly {DAYS[key]} {'%02d:%02d' % hhmm(clock)}"
    else:
        raise ValueError("Podaj kiedy: --in, --at, --daily, --weekdays albo --weekly.")
    return next_time(repeat, now), repeat


def describe_repeat(repeat):
    if not repeat:
        return "jednorazowo"
    kind, _, spec = repeat.partition(" ")
    if kind == "weekly":
        day, _, clock = spec.partition(" ")
        return f"co tydzien ({DAY_NAMES[int(day)]} {clock})"
    return {"daily": "codziennie", "weekdays": "w dni robocze"}[kind] + f" o {spec}"


def load():
    data = read_json(PLAN, {})
    return data if isinstance(data.get("items"), list) else {"items": [], "next_id": 1}


def cmd_add(args):
    if bool(args.say) == bool(args.prompt):
        raise ValueError("Podaj --say (przypomnienie) albo --prompt (polecenie dla Achai).")
    now = datetime.now()
    first, repeat = parse_when(args, now)
    if first <= now:
        raise ValueError("Ten termin juz minal.")
    data = load()
    item = {"id": data.get("next_id", 1), "kind": "say" if args.say else "prompt",
            "text": " ".join((args.say or args.prompt).split()), "next": first.timestamp(), "repeat": repeat}
    data["items"].append(item)
    data["next_id"] = item["id"] + 1
    write_json(PLAN, data)
    print(f"Zaplanowano [{item['id']}] {first:%d.%m %H:%M} ({describe_repeat(repeat)}): "
          f"{'przypomnienie' if item['kind'] == 'say' else 'polecenie'} - {item['text']}")


def cmd_list():
    items = sorted(load()["items"], key=lambda i: i["next"])
    if not items:
        print("Nic nie jest zaplanowane.")
    for i in items:
        print(f"[{i['id']}] {datetime.fromtimestamp(i['next']):%d.%m %H:%M} | {describe_repeat(i.get('repeat'))} | "
              f"{'przypomnienie' if i['kind'] == 'say' else 'polecenie'}: {i['text']}")


def cmd_remove(target):
    data = load()
    before = len(data["items"])
    data["items"] = [] if target == "all" else [i for i in data["items"] if str(i["id"]) != str(target)]
    write_json(PLAN, data)
    removed = before - len(data["items"])
    print(f"Usunieto {removed}." if removed else f"Nie ma zadania {target}.")


def take_due(kinds, now=None):
    """Dla listenera: zadania do wykonania teraz (tylko podanych rodzajow) - przesuwa je na kolejny termin
    albo usuwa. Zwraca liste (rodzaj, tekst)."""
    now = now or time.time()
    data = load()
    due, keep, changed = [], [], False
    for item in data["items"]:
        stale_prompt = item["kind"] == "prompt" and not item.get("repeat") and now - item["next"] > 6 * 3600
        if item["next"] > now or (item["kind"] not in kinds and not (stale_prompt and "say" in kinds)):
            keep.append(item)
            continue
        changed = True
        late = now - item["next"]
        if item.get("repeat"):
            if late <= LATE_REPEAT:
                due.append((item["kind"], item["text"]))
            item["next"] = next_time(item["repeat"], datetime.fromtimestamp(now)).timestamp()
            keep.append(item)
        else:
            kind, text, at = item["kind"], item["text"], datetime.fromtimestamp(item["next"])
            if kind == "prompt" and late > 6 * 3600:  # stare polecenie nie rusza samo - tylko o nim mowimy
                kind, text = "say", f"Przegapiłam zaplanowane polecenie z {at:%d.%m %H:%M}: {text}"
            elif kind == "say" and late > 300:
                text = f"Spóźnione przypomnienie z {at:%H:%M}: {text}"
            due.append((kind, text))
    if changed:
        data["items"] = keep
        write_json(PLAN, data)
    return due


def main(argv=None):
    ap = argparse.ArgumentParser(description="Przypomnienia i zadania o czasie")
    sub = ap.add_subparsers(dest="cmd", required=True)
    a = sub.add_parser("add")
    a.add_argument("--say")
    a.add_argument("--prompt")
    a.add_argument("--in", dest="in_")
    a.add_argument("--at")
    a.add_argument("--daily")
    a.add_argument("--weekdays")
    a.add_argument("--weekly")
    sub.add_parser("list")
    sub.add_parser("remove").add_argument("target")
    args = ap.parse_args(argv)
    try:
        if args.cmd == "add":
            cmd_add(args)
        elif args.cmd == "list":
            cmd_list()
        else:
            cmd_remove(args.target)
    except ValueError as exc:
        print(f"BLAD: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
