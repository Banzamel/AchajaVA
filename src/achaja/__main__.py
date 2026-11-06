"""python -m achaja <polecenie> [argumenty] - jedno wejscie do narzedzi Achai.

Przyklad: python -m achaja plan list   (to samo co python -m achaja.plan.plan list)
"""
import runpy
import sys

COMMANDS = {
    "plan": ("achaja.plan.plan", "przypomnienia, polecenia o czasie i zadania w tle"),
    "jobs": ("achaja.plan.jobs", "wyniki zadan w tle"),
    "mail": ("achaja.mail.mail", "poczta: raport, odczyt, odpowiedzi, przywracanie"),
    "gmail": ("achaja.mail.gmail", "porzadki w Gmailu (konektor Claude): wypisywanie z list, zasady"),
    "phone": ("achaja.mail.phone", "wazna poczta na telefon (push i glos przez Home Assistant)"),
    "ha": ("achaja.home.ha", "Home Assistant: stan, klimatyzacja, uslugi"),
    "agent": ("achaja.agents.agent", "agenci projektow"),
    "claude": ("achaja.claude.data", "konto, limity planu i sesje Claude Code"),
    "backup": ("achaja.core.backup", "kopie zapasowe"),
    "update": ("achaja.core.update", "aktualizacje"),
    "output": ("achaja.voice.output", "wyjscie glosu (sluchawki, glosnik)"),
    "speak": ("achaja.voice.speak", "przeczytaj tekst na glos"),
    "app": ("achaja.ui.app", "okno Achai"),
    "listener": ("achaja.voice.listener", "nasluch w oknie konsoli"),
}


def main():
    if len(sys.argv) < 2 or sys.argv[1] not in COMMANDS:
        print("Uzycie: python -m achaja <polecenie> [argumenty]\n")
        for name, (_, about) in COMMANDS.items():
            print(f"  {name:9} {about}")
        return 0 if len(sys.argv) < 2 else 2
    module = COMMANDS[sys.argv[1]][0]
    sys.argv = [module] + sys.argv[2:]
    runpy.run_module(module, run_name="__main__", alter_sys=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
