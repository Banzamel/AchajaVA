"""Achaja - glosowa asystentka dla Claude Code.

Podpakiety:
  core/    wspolne sciezki i konfiguracja, aktualizacje, kopie zapasowe, sterowanie konsola Claude
  voice/   nasluch (slowo wybudzenia), mowa, wyjscia audio
  ui/      okno aplikacji i jego zakladki, wyglad
  hooks/   hooki sesji Claude Code (glos, bramka potwierdzen, ochrona zamkow i bram)
  agents/  agenci projektow
  mail/    poczta: segregacja w tle, odczyt, odpowiedzi
  home/    Home Assistant przez REST
  plan/    przypomnienia, polecenia o czasie i zadania w tle
  claude/  konto, limity planu i sesje Claude Code na tym komputerze

Polecenia: python -m achaja <polecenie> ... (lista: python -m achaja).
"""
