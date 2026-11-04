"""Hook PreToolUse: bramka Achai - akcje nieodwracalne i zewnetrzne tylko po "tak" uzytkownika.

Zasada z CLAUDE.md ("zapytaj glosowo i czekaj na 'tak'") jest tu wymuszana w kodzie: git push, usuwanie plikow,
wyslanie maila, deploy, wylaczenie komputera, zamki i bramy (ha.py --confirmed), instalacja aktualizacji
i narzedzia MCP, ktore wysylaja lub usuwaja, przechodza tylko w turze, ktora zaczal prompt uzytkownika
brzmiacy jak zgoda ("tak", "potwierdzam", "wyslij"...). Zgode zapisuje hooks/event.py (UserPromptSubmit)
w state/approval.json, a hooks/stop.py kasuje ja na koncu tury - nie przechodzi na kolejne tury.
Tekst z maila, strony czy wyniku agenta nie jest promptem uzytkownika, wiec nie moze udzielic zgody.
Dziala w sesji Achai (ACHAJA_SESSION) i w zadaniach w tle (ACHAJA_JOB): tam przechodza tylko kategorie
ze stalych zgod zadania (ACHAJA_JOB_ALLOW, plan.py --allow); "system" nigdy. Agenci projektow maja wlasne zasady.
"""
import json
import os
import re
import sys
import time

from achaja.core.common import STATE, load_config, read_json

APPROVAL = STATE / "approval.json"
APPROVAL_TTL = 30 * 60
SCHEDULED = "[ZAPLANOWANE]"  # prefiks promptow z plan.py (plan.SCHEDULED) - nigdy nie sa zgoda

_ARGS = r"[^\n|;&]*"


def cli(command, module, old):
    """Narzedzie Achai wywolane dowolnym sposobem: "-m achaja <polecenie>", "-m achaja.<pakiet>.<modul>"
    albo stara sciezka <plik>.py - zawsze z odstepem przed podpoleceniem."""
    return rf"(?:\bachaja\s+{command}|\bachaja\.\w+\.{module}\b|\b{old}\.py[\"']?)\s+"
# (wzorzec, opis, kategoria) - kategorie to tez nazwy stalych zgod zadan w tle (plan.py --allow);
# "system" nigdy nie jest zgoda stala: tylko "tak" uzytkownika w rozmowie
COMMANDS = [
    (rf"\bgit\b{_ARGS}\bpush\b", "git push", "push"),
    (rf"\bgit\b{_ARGS}\breset\b{_ARGS}--hard\b", "git reset --hard", "delete"),
    (rf"\bgit\b{_ARGS}\bclean\b{_ARGS}\s-\w*f", "git clean", "delete"),
    (rf"\bgit\b{_ARGS}\bbranch\b{_ARGS}\s-D\b", "usuniecie galezi", "delete"),
    (r"\b(?:remove-item|rm|rmdir|rd|del|erase|ri|shred|unlink)\b\s", "usuwanie plikow", "delete"),
    (r"\b(?:clear-recyclebin|format-volume|diskpart)\b|\bformat(?:\.com)?\s+[a-z]:", "czyszczenie dysku", "system"),
    (r"\b(?:stop-computer|restart-computer|shutdown)\b", "wylaczenie komputera", "system"),
    (r"\b(?:npm|pnpm|yarn)\s+publish\b|\btwine\s+upload\b|\bfirebase\s+deploy\b|\bvercel\b[^\n]*--prod"
     r"|\bkubectl\s+(?:apply|delete)\b|\bterraform\s+(?:apply|destroy)\b|\bgh\s+release\s+create\b",
     "deploy lub publikacja", "deploy"),
    (cli("mail", "mail", "mail") + r"(?:send|reply)\b(?![^\n]*--(?:draft|dry-run))", "wyslanie maila", "mail"),
    (r"\bsend-mailmessage\b", "wyslanie maila", "mail"),
    (cli("ha", "ha", "ha") + r"[^\n]*--confirmed", "zamek, brama, garaz lub alarm", "home"),
    (cli("update", "update", "update") + r"install\b", "instalacja aktualizacji", "system"),
    (cli("plan", "plan", "plan") + r"add\b[^\n]*--allow", "stala zgoda dla zadania w tle", "system"),
    (cli("claude", "data", "claude_data") + r"(?:delete-session\b|delete-project\b|clean\b[^\n]*--apply)",
     "usuniecie sesji Claude", "delete"),
    (cli("backup", "backup", "backup") + r"restore\b", "przywrocenie kopii zapasowej", "system"),
]
MCP_RISKY = re.compile(r"send|trash|delete|remove|forward|reply|publish|merge|push|transfer|pay|purchase|order",
                       re.IGNORECASE)
MCP_SAFE = re.compile(r"draft|^get|^list|^search|^read|^query", re.IGNORECASE)

YES = {"tak", "yes", "potwierdzam", "zgoda", "zgadzam", "dawaj", "jasne", "oczywiscie", "ok", "okej", "okay",
       "dobrze", "rob", "zrob", "wyslij", "usun", "smialo", "sure", "confirm", "confirmed", "yep", "yeah", "go",
       "proceed", "send", "delete", "do", "wypchnij", "push", "otworz", "zamknij", "instaluj", "zainstaluj"}
NO = {"nie", "no", "not", "dont", "stop", "anuluj", "cancel", "czekaj", "wait", "nigdy", "never"}


def fold(text):
    import unicodedata
    text = (text or "").lower().replace("ł", "l")
    return "".join(c for c in unicodedata.normalize("NFKD", text) if not unicodedata.combining(c))


def is_yes(prompt, cfg=None):
    """Czy prompt uzytkownika to zgoda na akcje, o ktora Achaja zapytala."""
    if not prompt or prompt.lstrip().startswith(SCHEDULED):
        return False
    text = fold(prompt)
    try:
        ends = (cfg or load_config()).get("wake", {}).get("end") or []
    except Exception:
        ends = []
    for end in sorted((fold(e) for e in ends), key=len, reverse=True):
        text = re.sub(rf"\b{re.escape(end)}\W*$", "", text.strip())
    words = re.findall(r"[a-z]+", text.replace("'", ""))
    if len(words) > 1 and words[0] in ("no", "noo") and words[1] in ("tak", "dobrze", "jasne", "dawaj", "okej", "ok"):
        words = words[1:]  # polskie "no tak", "no dobrze"
    if not words or len(words) > 12 or NO & set(words):
        return False
    return words[0] in YES


def approved():
    data = read_json(APPROVAL, {})
    return bool(data.get("yes")) and time.time() - data.get("at", 0) < APPROVAL_TTL


def risky(tool, args):
    """(opis, kategoria) ryzykownej akcji albo None."""
    if tool in ("Bash", "PowerShell"):
        command = str(args.get("command", ""))
        for pattern, what, category in COMMANDS:
            if re.search(pattern, command, re.IGNORECASE):
                return what, category
        return None
    if tool.startswith("mcp__") and not tool.startswith("mcp__home-assistant__"):  # HA pilnuje hooks/home_guard.py
        name = tool.split("__")[-1]
        if MCP_RISKY.search(name) and not MCP_SAFE.search(name):
            return f"narzedzie {name}", "mcp"
    return None


def deny(what):
    json.dump({"hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "permissionDecision": "deny",
        "permissionDecisionReason":
            f"WYMAGA POTWIERDZENIA (bramka Achai): {what}. Nie obchodz tego innym poleceniem. Zapytaj "
            "uzytkownika glosowo (pytanie w sekcji [GŁOS]) i zakoncz ture; wykonasz to w nastepnej turze, "
            "jesli odpowie 'tak'."}}, sys.stdout)
    return 0


def deny_job(what):
    json.dump({"hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "permissionDecision": "deny",
        "permissionDecisionReason":
            f"ZADANIE W TLE NIE MA ZGODY: {what}. Nie obchodz tego innym poleceniem. Przygotuj szkic, jesli sie "
            "da (np. python -m achaja mail reply ... --draft), i opisz w raporcie, co czeka na decyzje uzytkownika."}}, sys.stdout)
    return 0


def main():
    data = json.loads(sys.stdin.buffer.read().decode("utf-8-sig") or "{}")
    job = os.environ.get("ACHAJA_JOB")
    if not os.environ.get("ACHAJA_SESSION") and not job:
        return 0
    args = data.get("tool_input") or {}
    hit = risky(str(data.get("tool_name", "")), args if isinstance(args, dict) else {})
    if not hit:
        return 0
    what, category = hit
    if job:  # zadanie w tle: tylko stale zgody nadane przy jego tworzeniu (plan.py --allow, za "tak")
        allowed = {a for a in os.environ.get("ACHAJA_JOB_ALLOW", "").split(",") if a}
        return 0 if category in allowed and category != "system" else deny_job(what)
    return 0 if approved() else deny(what)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as exc:  # blad bramki nie moze przepuscic akcji
        sys.exit(deny(f"blad sprawdzania ({type(exc).__name__})"))
