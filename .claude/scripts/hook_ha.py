"""Hook PreToolUse dla serwera MCP home-assistant: zamki, alarmy, bramy i garaze tylko po potwierdzeniu.

Narzedzia MCP (HassTurnOn/Off, HassSetPosition...) przyjmuja nazwe, obszar, pietro, domene i device_class -
jedno polecenie dla obszaru "garaz" albo domeny "cover" moze otworzyc brame. Hook rozwija cel na encje
(przez REST API, jak ha.py) i odmawia, gdy wsrod nich jest cos chronionego albo nie da sie tego sprawdzic.
Po "tak" uzytkownika Achaja wykonuje to przez ha.py z --confirmed (MCP nie ma flagi potwierdzenia).
Narzedzia tylko do odczytu (Get*) przechodza bez sprawdzania.
"""
import json
import sys

import ha

READ_ONLY_PREFIXES = ("Get", "get_", "list_")
# narzedzia, ktore moga ruszyc zamek, brame lub alarm (reszta: swiatla, media, timery, listy - bez sprawdzania)
RISKY_PREFIXES = ("HassTurnOn", "HassTurnOff", "HassToggle", "HassSetPosition")
RISKY_WORDS = ("lock", "alarm", "cover", "valve", "open", "close", "gate", "garage", "door")


def as_list(value):
    if value is None:
        return []
    items = value if isinstance(value, (list, tuple)) else [value]
    return [str(v).strip() for v in items if str(v).strip()]


def deny(reason):
    json.dump({"hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "permissionDecision": "deny",
        "permissionDecisionReason":
            f"WYMAGA POTWIERDZENIA: {reason}. Zamki, alarmy, bramy i garaze steruje sie tylko po 'tak' "
            "uzytkownika: zapytaj glosowo, a po 'tak' wykonaj to przez ha.py z --confirmed "
            "(np. ha.py call cover open_cover '{\"entity_id\": \"cover.brama\"}' --confirmed). "
            "Jesli to zwykle urzadzenie (nie zamek, brama ani alarm), wskaz je dokladna nazwa albo uzyj "
            "ha.py call z dokladnym entity_id - wtedy bez pytania."}},
        sys.stdout)
    return 0


def floor_entities(conn, floor):
    areas = "floor_areas(" + repr(floor) + ")"
    out = conn.template("{{ ((" + areas + " | map('area_entities') | sum(start=[])) + (" + areas
                        + " | map('area_devices') | sum(start=[]) | map('device_entities') | sum(start=[])))"
                        " | unique | list | tojson }}")
    return json.loads(out) if isinstance(out, str) else (out or [])


def entities_for(conn, args):
    """Encje, ktore moze objac wywolanie; None = nie da sie ustalic."""
    domains = [d.lower() for d in as_list(args.get("domain"))]
    states = conn.states()

    def in_domain(eid):
        return not domains or eid.split(".")[0] in domains

    pool = None
    for area in as_list(args.get("area")):
        ids = set(conn.area_entities(area))
        if not ids:
            return None  # obszar o nazwie, ktorej nie znamy - nie zgadujemy
        pool = ids if pool is None else pool & ids
    for floor in as_list(args.get("floor")):
        try:
            ids = set(floor_entities(conn, floor))
        except (ha.HAError, ValueError):
            return None
        if not ids:
            return None
        pool = ids if pool is None else pool & ids
    candidates = [s for s in states if in_domain(s["entity_id"]) and (pool is None or s["entity_id"] in pool)]
    names = as_list(args.get("name"))
    if names:
        wanted = [ha.fold(n) for n in names]
        candidates = [s for s in candidates
                      if any(w in ha.fold(s["attributes"].get("friendly_name")) or w in ha.fold(s["entity_id"])
                             for w in wanted)]
        if not candidates:
            return None  # nazwa spoza listy encji (np. alias) - nie wiadomo, co HA wybierze
    return candidates


def check(tool, args):
    """Powod odmowy albo None."""
    text = ha.fold(" ".join(as_list(args.get("name")) + as_list(args.get("area")) + as_list(args.get("floor"))))
    if any(w in text for w in ha.GUARDED_WORDS):
        return f"{tool} dla '{text}'"
    if set(d.lower() for d in as_list(args.get("domain"))) & set(ha.GUARDED):
        return f"{tool} dla domeny {', '.join(as_list(args.get('domain')))}"
    if set(c.lower() for c in as_list(args.get("device_class"))) & set(ha.GUARDED_CLASSES):
        return f"{tool} dla urzadzen typu {', '.join(as_list(args.get('device_class')))}"
    try:
        conn = ha.HA()
        found = entities_for(conn, args)
    except ha.HAError:
        found = None
    if found is None:
        return f"{tool}: nie udalo sie sprawdzic, ktore urzadzenia obejmie to polecenie"
    for s in found:
        if ha.guarded(s):
            return f"{tool} obejmuje {s['entity_id']} ({s['attributes'].get('friendly_name', '')})"
    return None


def main():
    data = json.loads(sys.stdin.buffer.read().decode("utf-8-sig") or "{}")
    tool = str(data.get("tool_name", "")).split("__")[-1]
    args = data.get("tool_input") or {}
    if not tool or tool.startswith(READ_ONLY_PREFIXES) or not isinstance(args, dict):
        return 0
    if any(w in ha.fold(tool) for w in ha.GUARDED_WORDS):  # skrypt HA udostepniony jako narzedzie
        return deny(f"skrypt {tool}")
    if not (tool.startswith(RISKY_PREFIXES) or any(w in tool.lower() for w in RISKY_WORDS)):
        return 0
    reason = check(tool, args)
    return deny(reason) if reason else 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as exc:  # blad hooka nie moze otworzyc bramy - odmawiamy
        sys.exit(deny(f"blad sprawdzania ({type(exc).__name__})"))
