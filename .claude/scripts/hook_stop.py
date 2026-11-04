"""Hook Stop: wyciaga sekcje [GŁOS] z odpowiedzi Achai i czyta ja na glos.

Jesli sekcji brak, w sesji Achai raz zatrzymuje koniec tury i prosi Claude o dopisanie [GŁOS];
gdy nadal jej nie ma - czyta pierwsze zdania odpowiedzi (awaryjnie). Pusta odpowiedz -> komunikat glosem.
Konczy tez zgode bramki (state/approval.json) - obowiazuje tylko w jednej turze.
Mowa startuje w osobnym procesie, zeby hook nie blokowal sesji.
"""
import json
import os
import re
import subprocess
import sys
import time

from common import (CREATE_NO_WINDOW, DETACHED_PROCESS, PYTHONW, SCRIPTS, STATE,
                    append_chat, load_config, write_json)

MARKER = re.compile(r"^\s*\[(?:G[ŁL]OS|VOICE)\]\s*:?\s*", re.IGNORECASE | re.MULTILINE)


def voice_text(message):
    matches = list(MARKER.finditer(message or ""))
    if matches:
        return message[matches[-1].end():].strip()
    sentences = re.split(r"(?<=[.!?])\s+", (message or "").strip())
    return " ".join(sentences[:2])


NO_VOICE = {
    "pl": ("Odpowiedz nie ma sekcji [GŁOS], a uzytkownik nie patrzy na ekran. Dopisz teraz na samym koncu "
           "jedna linie [GŁOS] z krotkim podsumowaniem do przeczytania na glos (bez powtarzania calej odpowiedzi)."),
    "en": ("The reply has no [VOICE] section and the user is not looking at the screen. Add one [VOICE] line at "
           "the very end now with a short summary to be read aloud (do not repeat the whole answer)."),
}
EMPTY = {"pl": "Nie dostałam odpowiedzi. Powtórz, proszę, polecenie.",
         "en": "I didn't get an answer. Please say that again."}


def lang():
    try:
        return "en" if load_config().get("language") == "en" else "pl"
    except Exception:
        return "pl"


def main():
    data = json.loads(sys.stdin.buffer.read().decode("utf-8-sig") or "{}")
    if os.environ.get("ACHAJA_JOB"):
        return  # zadanie w tle: wynik mowi jobs.py, kiedy nikt nie nagrywa
    message = data.get("last_assistant_message", "") or ""
    session = bool(os.environ.get("ACHAJA_SESSION"))
    if session and message.strip() and not MARKER.search(message) and not data.get("stop_hook_active"):
        # brak [GŁOS]: tura nie konczy sie po cichu - Claude dopisuje podsumowanie (raz; potem awaryjnie)
        json.dump({"decision": "block", "reason": NO_VOICE[lang()]}, sys.stdout)
        return
    if session:
        write_json(STATE / "achaja_state.json", {"state": "idle", "since": time.time()})
        (STATE / "approval.json").unlink(missing_ok=True)  # zgoda bramki obowiazuje tylko w tej turze
        if message.strip():
            append_chat("assistant", message)
    text = voice_text(message)
    if not text and session:
        text = EMPTY[lang()]
    if text:
        (STATE / "last_voice.txt").write_text(text, encoding="utf-8")
        if "?" in text and session:
            # tryb rozmowy: listener wlaczy nagrywanie, gdy Achaja skonczy mowic
            write_json(STATE / "followup.json", {"at": time.time()})
        subprocess.Popen(
            [str(PYTHONW), str(SCRIPTS / "speak.py"), text],
            cwd=str(SCRIPTS), creationflags=DETACHED_PROCESS | CREATE_NO_WINDOW, close_fds=True,
        )


if __name__ == "__main__":
    main()
