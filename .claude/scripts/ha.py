"""Home Assistant przez REST API - to, czego nie ma serwer MCP (np. tryb pracy klimatyzacji).

Uzycie:
  ha.py state <entity_id|nazwa|obszar>                    stan i atrybuty (obszar: lista jego urzadzen)
  ha.py climate <nazwa|entity_id> [--mode M] [--temp N]   wlacza, ustawia tryb i temperature, sprawdza stan
        M: heat | cool | auto | heat_cool | dry | fan_only | off  (tylko tryby z hvac_modes urzadzenia)
  ha.py call <domena> <usluga> '<json>'                   dowolna usluga z dozwolonych domen
Wspolne opcje: --dry-run (pokazuje zadania zmieniajace stan, nic nie wysyla), --confirmed (zamki, alarmy,
bramy i garaze - bez tej flagi skrypt odmawia, Achaja najpierw pyta glosowo).

Adres: config.json home_assistant.url (albo adres z .mcp.json). Token: tylko state/ha_token.txt - nigdy
nie jest wypisywany, takze w bledach.
Kod wyjscia: 0 ok, 1 blad, 2 stan po zmianie inny niz zadany, 3 wymagane --confirmed.
"""
import argparse
import json
import sys
import time
import unicodedata
import urllib.error
import urllib.request

from common import ROOT, STATE, load_config

ALLOWED = ("climate", "light", "switch", "cover", "fan", "media_player", "vacuum", "script",
           "lock", "alarm_control_panel")  # dwie ostatnie tylko z --confirmed
GUARDED = ("lock", "alarm_control_panel")
GUARDED_CLASSES = ("garage", "gate", "door")  # device_class rolet/bram
GUARDED_WORDS = ("brama", "bramy", "furtk", "garaz", "gate", "garage", "zamek", "lock", "alarm")
MODES = ("heat", "cool", "auto", "heat_cool", "dry", "fan_only", "off")
SETTLE = 2.5  # sekundy na reakcje urzadzenia przed odczytem stanu
TOKEN = ""


def fold(s):
    """Porownanie bez wielkosci liter i polskich znakow ('ł' nie rozklada sie w NFKD)."""
    s = (s or "").lower().replace("ł", "l")
    return "".join(c for c in unicodedata.normalize("NFKD", s) if not unicodedata.combining(c))


def safe(text):
    """Tekst bez tokenu - na wypadek, gdyby trafil do komunikatu bledu."""
    text = str(text)
    return text.replace(TOKEN, "***") if TOKEN else text


class HAError(Exception):
    pass


class HA:
    def __init__(self, dry_run=False):
        global TOKEN
        self.dry_run = dry_run
        url = (load_config().get("home_assistant") or {}).get("url")
        if not url:
            try:
                mcp = json.loads((ROOT / ".mcp.json").read_text(encoding="utf-8"))
                url = mcp["mcpServers"]["home-assistant"]["url"].split("/api/")[0]
            except (OSError, ValueError, KeyError):
                raise HAError("Brak adresu Home Assistant (config.json: home_assistant.url).")
        self.url = url.rstrip("/")
        try:
            TOKEN = (STATE / "ha_token.txt").read_text(encoding="utf-8").strip()
        except OSError:
            raise HAError("Brak tokenu Home Assistant (state/ha_token.txt).")
        if not TOKEN:
            raise HAError("Plik state/ha_token.txt jest pusty.")
        self._states = None

    def request(self, method, path, body=None):
        data = json.dumps(body).encode("utf-8") if body is not None else None
        req = urllib.request.Request(self.url + path, data=data, method=method,
                                     headers={"Authorization": f"Bearer {TOKEN}",
                                              "Content-Type": "application/json"})
        try:
            with urllib.request.urlopen(req, timeout=15) as resp:
                raw = resp.read().decode("utf-8")
        except urllib.error.HTTPError as exc:
            detail = exc.read().decode("utf-8", errors="replace")[:300]
            raise HAError(safe(f"Home Assistant: {method} {path} -> HTTP {exc.code} {detail}"))
        except (urllib.error.URLError, OSError) as exc:
            raise HAError(safe(f"Brak polaczenia z Home Assistant ({self.url}): {getattr(exc, 'reason', exc)}"))
        if not raw:
            return None
        try:
            return json.loads(raw)
        except ValueError:
            return raw

    def service(self, domain, name, data):
        """Wywolanie uslugi; w --dry-run tylko wypisuje zadanie."""
        path = f"/api/services/{domain}/{name}"
        if self.dry_run:
            print(f"[dry-run] POST {path} {json.dumps(data, ensure_ascii=False)}")
            return []
        return self.request("POST", path, data)

    def states(self):
        if self._states is None:
            self._states = self.request("GET", "/api/states")
        return self._states

    def state(self, entity_id):
        return self.request("GET", f"/api/states/{entity_id}")

    def template(self, text):
        return self.request("POST", "/api/template", {"template": text})

    def area_entities(self, name):
        """Encje obszaru o nazwie (albo id) pasujacej do name; [] gdy nie ma takiego obszaru."""
        listing = self.template("{% for a in areas() %}{{ a }}\t{{ area_name(a) }}\n{% endfor %}") or ""
        wanted = fold(name)
        for line in str(listing).splitlines():
            area_id, _, area_name = line.partition("\t")
            if wanted in (fold(area_id), fold(area_name)):
                ids = self.template(f"{{{{ area_entities('{area_id}') | tojson }}}}")
                return json.loads(ids) if isinstance(ids, str) else (ids or [])
        return []

    def find(self, query, domain=None):
        """Encje pasujace do entity_id, nazwy albo obszaru (najpierw dokladne trafienia)."""
        everything = [s for s in self.states() if not domain or s["entity_id"].startswith(domain + ".")]
        if any(s["entity_id"] == query for s in everything):
            return [s for s in everything if s["entity_id"] == query]
        wanted = fold(query)
        exact = [s for s in everything if fold(s["attributes"].get("friendly_name")) == wanted]
        if exact:
            return exact
        partial = [s for s in everything
                   if wanted in fold(s["attributes"].get("friendly_name")) or wanted in fold(s["entity_id"])]
        if partial:
            return partial
        in_area = set(self.area_entities(query))
        return [s for s in everything if s["entity_id"] in in_area]


def pick_one(ha, query, domain):
    found = ha.find(query, domain)
    if not found:
        raise HAError(f"Nie znalazlam urzadzenia '{query}'" + (f" ({domain})." if domain else "."))
    if len(found) > 1:
        names = ", ".join(f"{s['entity_id']} ({s['attributes'].get('friendly_name', '')})" for s in found[:8])
        raise HAError(f"Kilka urzadzen pasuje do '{query}': {names}. Podaj dokladniejsza nazwe albo entity_id.")
    return found[0]


def describe(s, full=True):
    a = s["attributes"]
    line = f"{s['entity_id']} ({a.get('friendly_name', '')}): {s['state']}"
    if not full:
        return line
    skip = ("friendly_name", "entity_picture", "icon", "supported_features", "supported_color_modes")
    attrs = {k: v for k, v in a.items() if k not in skip}
    return line + ("\n" + "\n".join(f"  {k}: {json.dumps(v, ensure_ascii=False)}" for k, v in attrs.items())
                   if attrs else "")


def guarded(entity):
    """Zamek, alarm, brama, garaz - wymaga potwierdzenia uzytkownika (--confirmed)."""
    eid, a = entity["entity_id"], entity.get("attributes", {})
    if eid.split(".")[0] in GUARDED or a.get("device_class") in GUARDED_CLASSES:
        return True
    text = fold(eid + " " + a.get("friendly_name", ""))
    return any(w in text for w in GUARDED_WORDS)


def cmd_state(ha, args):
    found = ha.find(args.target)
    if not found:
        raise HAError(f"Nie znalazlam '{args.target}' (ani urzadzenia, ani obszaru).")
    for s in found[:40]:
        print(describe(s, full=len(found) == 1))
    if len(found) > 40:
        print(f"... i {len(found) - 40} wiecej")


def cmd_climate(ha, args):
    dev = pick_one(ha, args.target, "climate")
    eid, a = dev["entity_id"], dev["attributes"]
    if guarded(dev) and not args.confirmed:
        return refuse(eid)
    modes = a.get("hvac_modes") or []
    if args.mode:
        if args.mode not in modes:
            raise HAError(f"{a.get('friendly_name', eid)} nie obsluguje trybu '{args.mode}'. "
                          f"Dostepne tryby: {', '.join(modes) or 'brak informacji'}.")
    if args.temp is not None:
        lo, hi = a.get("min_temp"), a.get("max_temp")
        if lo is not None and hi is not None and not lo <= args.temp <= hi:
            raise HAError(f"Temperatura {args.temp} poza zakresem urzadzenia ({lo}-{hi}).")
        if args.mode == "off":
            raise HAError("Nie ustawiam temperatury przy wylaczaniu (--mode off).")
    if not args.mode and args.temp is None:
        raise HAError("Podaj --mode albo --temp.")

    # tryb wlacza urzadzenie; temperatura dopiero po trybie (czesc klimatyzacji resetuje ja przy zmianie trybu)
    if args.mode:
        ha.service("climate", "set_hvac_mode", {"entity_id": eid, "hvac_mode": args.mode})
    elif dev["state"] == "off":
        ha.service("climate", "turn_on", {"entity_id": eid})
    if args.temp is not None:
        temp = int(args.temp) if float(args.temp).is_integer() else args.temp  # 23, nie 23.0
        ha.service("climate", "set_temperature", {"entity_id": eid, "temperature": temp})
    if ha.dry_run:
        print(f"[dry-run] potem GET /api/states/{eid} po {SETTLE} s i porownanie z zadanym stanem")
        return 0

    time.sleep(SETTLE)
    now = ha.state(eid)
    na = now["attributes"]
    print(f"{na.get('friendly_name', eid)}: tryb {now['state']}, temperatura zadana {na.get('temperature')}, "
          f"w pomieszczeniu {na.get('current_temperature')}")
    problems = []
    if args.mode and now["state"] != args.mode:
        problems.append(f"tryb jest '{now['state']}' zamiast '{args.mode}'")
    if args.temp is not None and na.get("temperature") is not None and float(na["temperature"]) != float(args.temp):
        problems.append(f"temperatura zadana to {na['temperature']} zamiast {args.temp}")
    if problems:
        print("UWAGA: " + "; ".join(problems) + ".")
        return 2
    print("OK: stan zgodny z zadanym.")
    return 0


def cmd_call(ha, args):
    if args.domain not in ALLOWED:
        raise HAError(f"Domena '{args.domain}' nie jest dozwolona. Dozwolone: {', '.join(ALLOWED)}.")
    try:
        data = json.loads(args.data) if args.data else {}
    except ValueError as exc:
        raise HAError(f"Niepoprawny JSON: {exc}")
    if not isinstance(data, dict):
        raise HAError("Dane uslugi musza byc obiektem JSON, np. {\"entity_id\": \"light.salon\"}.")
    ids = data.get("entity_id") or []
    ids = [ids] if isinstance(ids, str) else list(ids)
    if not args.confirmed:
        if args.domain in GUARDED:
            return refuse(args.domain)
        if not ids and args.domain == "cover":  # obszar / urzadzenie - nie wiadomo, czy nie ma tam bramy
            return refuse("rolety bez entity_id (moze obejmowac brame lub garaz)")
        by_id = {s["entity_id"]: s for s in ha.states()}
        for eid in ids:
            if guarded(by_id.get(eid, {"entity_id": eid})):
                return refuse(eid)
    result = ha.service(args.domain, args.service, data)
    if ha.dry_run:
        return 0
    changed = [f"{s['entity_id']}: {s['state']}" for s in result or [] if isinstance(s, dict)]
    print("OK" + (": " + ", ".join(changed[:20]) if changed else " (usluga wykonana)."))
    return 0


def refuse(what):
    print(f"WYMAGA POTWIERDZENIA: {what} - zamki, alarmy, bramy i garaze steruje tylko po 'tak' uzytkownika "
          f"(uruchom ponownie z --confirmed).")
    return 3


def main(argv=None):
    ap = argparse.ArgumentParser(description="Home Assistant przez REST API")
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--confirmed", action="store_true")
    sub = ap.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("state")
    p.add_argument("target")
    p = sub.add_parser("climate")
    p.add_argument("target")
    p.add_argument("--mode", choices=MODES)
    p.add_argument("--temp", type=float)
    p = sub.add_parser("call")
    p.add_argument("domain")
    p.add_argument("service")
    p.add_argument("data", nargs="?", default="{}")
    for name in ("climate", "call"):  # opcje wspolne dzialaja tez po poleceniu
        sub.choices[name].add_argument("--dry-run", action="store_true", dest="dry_run_sub")
        sub.choices[name].add_argument("--confirmed", action="store_true", dest="confirmed_sub")
    args = ap.parse_args(argv)
    args.dry_run = args.dry_run or getattr(args, "dry_run_sub", False)
    args.confirmed = args.confirmed or getattr(args, "confirmed_sub", False)
    try:
        ha = HA(dry_run=args.dry_run)
        return {"state": cmd_state, "climate": cmd_climate, "call": cmd_call}[args.cmd](ha, args) or 0
    except HAError as exc:
        print(f"BLAD: {safe(exc)}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
