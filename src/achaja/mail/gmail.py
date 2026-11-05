"""Porzadki w Gmailu przez konektor Gmail z konta Claude (narzedzia mcp__claude_ai_Gmail__*) - pomocnik skilla `gmail`.

Sama skrzynke czyta i sprzata Claude narzedziami konektora; ten modul robi to, czego konektor nie ma:
  python -m achaja gmail unsubscribe --header "<List-Unsubscribe>" [--post "<List-Unsubscribe-Post>"] [--sender <adres>]
        wypisanie z listy: one-click POST (RFC 8058) gdy nadawca go wspiera; mailto -> wypisuje adres
        (wyslanie maila robi Claude za zgoda); zwykly link -> tylko podaje go (nie otwiera stron)
  python -m achaja gmail keep <adres|@domena> [--note "..."]   zawsze zachowuj (nie pytaj, nie usuwaj)
  python -m achaja gmail junk <adres|@domena> [--note "..."]   zawsze do kosza bez pytania
  python -m achaja gmail forget <adres|@domena>                 usun zasade
  python -m achaja gmail rules                                  zasady i historia wypisan
  python -m achaja gmail log --trashed N --spam N --unsubscribed N --kept N   zapis wyniku przegladu
Kody wyjscia: 0 ok, 1 blad, 3 wymaga dzialania Claude/uzytkownika (mailto lub link do recznego wypisania).
"""
import argparse
import ipaddress
import re
import socket
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

from achaja.core.common import STATE, read_json, write_json

DIR = STATE / "gmail"
RULES = DIR / "rules.json"
UNSUB = DIR / "unsubscribed.json"
RUNS = DIR / "runs.json"


def norm(sender):
    sender = (sender or "").strip().lower()
    match = re.search(r"<([^>]+)>", sender)
    return match.group(1) if match else sender


def parse_header(value):
    """List-Unsubscribe: <https://...>, <mailto:...> -> (lista https, lista mailto)."""
    urls = re.findall(r"<\s*([^>\s]+)\s*>", value or "") or [v.strip() for v in (value or "").split(",") if v.strip()]
    https = [u for u in urls if u.lower().startswith("https://")]
    mailto = [u for u in urls if u.lower().startswith("mailto:")]
    return https, mailto


def public_host(url):
    """Tylko adresy publiczne - link z maila nie moze trafic do sieci domowej (router, Home Assistant)."""
    host = urllib.parse.urlsplit(url).hostname
    if not host:
        return False
    try:
        infos = socket.getaddrinfo(host, 443, proto=socket.IPPROTO_TCP)
    except OSError:
        return False
    for info in infos:
        ip = ipaddress.ip_address(info[4][0])
        if ip.is_private or ip.is_loopback or ip.is_link_local or ip.is_reserved or ip.is_multicast:
            return False
    return True


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):
        return None


def one_click(url):
    if not public_host(url):
        return False, "adres niepubliczny - pomijam"
    request = urllib.request.Request(url, data=b"List-Unsubscribe=One-Click", method="POST", headers={
        "Content-Type": "application/x-www-form-urlencoded", "User-Agent": "Achaja-unsubscribe/1.0"})
    try:
        with urllib.request.build_opener(NoRedirect).open(request, timeout=20) as resp:
            return 200 <= resp.status < 300, f"HTTP {resp.status}"
    except urllib.error.HTTPError as exc:
        return False, f"HTTP {exc.code}"
    except (urllib.error.URLError, OSError) as exc:
        return False, f"blad polaczenia: {exc}"


def remember_unsub(sender, how, ok):
    data = read_json(UNSUB, [])
    data.insert(0, {"sender": norm(sender), "how": how, "ok": ok, "at": time.strftime("%Y-%m-%d %H:%M")})
    DIR.mkdir(parents=True, exist_ok=True)
    write_json(UNSUB, data[:500])


def cmd_unsubscribe(args):
    https, mailto = parse_header(args.header)
    sender = args.sender or "?"
    done = [u for u in read_json(UNSUB, []) if u.get("ok") and u.get("sender") == norm(sender) and sender != "?"]
    if done:
        print(f"JUZ WYPISANO {done[0]['at']} ({done[0]['how']}) - nadawca nadal wysyla; oznacz jako spam.")
        return 0
    if https and "one-click" in (args.post or "").lower():
        for url in https:
            ok, info = one_click(url)
            if ok:
                remember_unsub(sender, "one-click", True)
                print(f"WYPISANO (one-click, {info}).")
                return 0
            print(f"one-click nie zadzialal: {info}")
    if mailto:
        parts = urllib.parse.urlsplit(mailto[0])
        query = urllib.parse.parse_qs(parts.query)
        remember_unsub(sender, "mailto", False)
        print(f"MAILTO: do={urllib.parse.unquote(parts.path)} temat={query.get('subject', ['unsubscribe'])[0]} "
              f"tresc={query.get('body', ['unsubscribe'])[0]}")
        print("Wyslij to narzedziem send_message konektora Gmail (bramka zapyta o zgode), potem: gmail log.")
        return 3
    if https:
        remember_unsub(sender, "link", False)
        print(f"RECZNIE: nadawca ma tylko link do strony wypisania ({urllib.parse.urlsplit(https[0]).hostname}). "
              "Nie otwieraj go sama - powiedz uzytkownikowi albo oznacz jako spam.")
        return 3
    print("BRAK naglowka List-Unsubscribe - nie da sie wypisac automatycznie; oznacz jako spam albo dodaj junk.")
    return 3


def cmd_rule(kind, args):
    rules = read_json(RULES, {"keep": {}, "junk": {}})
    key = norm(args.sender)
    for k in ("keep", "junk"):
        rules.setdefault(k, {}).pop(key, None)
    if kind != "forget":
        rules[kind][key] = {"note": args.note or "", "at": time.strftime("%Y-%m-%d")}
    DIR.mkdir(parents=True, exist_ok=True)
    write_json(RULES, rules)
    print({"keep": "Zachowuje", "junk": "Do kosza", "forget": "Usunieto zasade"}[kind] + f": {key}")
    return 0


def cmd_rules(_args):
    rules = read_json(RULES, {"keep": {}, "junk": {}})
    for kind, title in (("keep", "ZAWSZE ZACHOWAJ"), ("junk", "ZAWSZE DO KOSZA")):
        print(f"{title}:")
        for sender, info in sorted(rules.get(kind, {}).items()):
            print(f"  {sender}" + (f"  - {info['note']}" if info.get("note") else ""))
    print("WYPISANIA (ostatnie 20):")
    for u in read_json(UNSUB, [])[:20]:
        print(f"  {u['at']}  {u['sender']}  {u['how']}  {'OK' if u['ok'] else 'czeka'}")
    runs = read_json(RUNS, [])
    if runs:
        print(f"OSTATNI PRZEGLAD: {runs[0]}")
    return 0


def cmd_log(args):
    runs = read_json(RUNS, [])
    runs.insert(0, {"at": time.strftime("%Y-%m-%d %H:%M"), "trashed": args.trashed, "spam": args.spam,
                    "unsubscribed": args.unsubscribed, "kept": args.kept})
    DIR.mkdir(parents=True, exist_ok=True)
    write_json(RUNS, runs[:200])
    print("Zapisano.")
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(prog="achaja gmail", description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd")
    p = sub.add_parser("unsubscribe")
    p.add_argument("--header", required=True)
    p.add_argument("--post", default="")
    p.add_argument("--sender", default="")
    for name in ("keep", "junk", "forget"):
        p = sub.add_parser(name)
        p.add_argument("sender")
        p.add_argument("--note", default="")
    sub.add_parser("rules")
    p = sub.add_parser("log")
    for name in ("trashed", "spam", "unsubscribed", "kept"):
        p.add_argument(f"--{name}", type=int, default=0)
    args = parser.parse_args(argv)
    if args.cmd == "unsubscribe":
        return cmd_unsubscribe(args)
    if args.cmd in ("keep", "junk", "forget"):
        return cmd_rule(args.cmd, args)
    if args.cmd == "rules":
        return cmd_rules(args)
    if args.cmd == "log":
        return cmd_log(args)
    print(__doc__)
    return 1


if __name__ == "__main__":
    sys.exit(main())
