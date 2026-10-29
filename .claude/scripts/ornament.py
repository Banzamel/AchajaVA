"""Ozdobna kreska z klejnotem: cienka zlota linia wygaszana ku koncom, posrodku karmazynowy klejnot
w zlotej oprawie. Gdy Achaja slucha, mysli albo mowi, klejnot pulsuje w kolorze stanu, a po kresce
przebiega blysk. Uzywana w naglowku okna (app.py) i w bocznym menu ustawien (setup/configure.py).
"""
import math
import time

import theme


class Band:
    def __init__(self, master, height, background_key="bg"):
        import tkinter as tk
        self.c = tk.Canvas(master, height=height, highlightthickness=0, borderwidth=0)
        self.h = height
        self.bg_key = background_key
        self.static = None  # (szerokosc, paleta) ostatniego spokojnego rysunku - bez ruchu nie rysujemy ponownie
        self.c.bind("<Configure>", lambda e: setattr(self, "static", None))

    def draw(self, p, color=None, active=False):
        """p: paleta; color: kolor stanu; active: klejnot pulsuje i blysk biegnie po kresce."""
        c, h = self.c, self.h
        w = c.winfo_width()
        if w < 10:
            return
        if not active and self.static == (w, id(p)):
            return
        self.static = None if active else (w, id(p))
        bg, gold = p[self.bg_key], p["gold"]
        c.configure(background=bg)
        c.delete("all")
        mid, cx = h / 2, w / 2
        gem = h * 0.42  # polowa przekatnej klejnotu
        now = time.time()
        glint = ((now * 0.6) % 1) if active else None  # pozycja blysku 0..1 od klejnotu ku koncom

        # kreska: segmenty coraz jasniejsze ku srodkowi, z obu stron klejnotu
        steps = 48
        span = cx - gem * 2.2
        for side in (-1, 1):
            for i in range(steps):
                f0, f1 = i / steps, (i + 1) / steps  # 0 przy klejnocie, 1 na koncu
                strength = (1 - f0) ** 1.4
                if glint is not None:
                    strength = min(1.0, strength + 0.9 * max(0.0, 1 - abs(f0 - glint) * 10))
                col = theme.blend(bg, theme.blend(gold, color or gold, 0.6) if glint is not None
                                  and abs(f0 - glint) < 0.1 else gold, strength)
                x0, x1 = cx + side * (gem * 2.2 + f0 * span), cx + side * (gem * 2.2 + f1 * span)
                c.create_line(x0, mid, x1, mid, fill=col, width=max(1, round(h * 0.09)))
            # drobne zlote kropki przy klejnocie
            for k, rad in ((1.55, 0.1), (1.95, 0.07)):
                x = cx + side * gem * k
                c.create_oval(x - h * rad, mid - h * rad, x + h * rad, mid + h * rad, fill=gold, outline="")

        # klejnot: zlota oprawa, karmazynowe wnetrze, jasna faseta; przy pracy pulsuje kolorem stanu
        pulse = 0.5 + 0.5 * math.sin(now * 4) if active else 0.0
        setting = gem * (1 + 0.12 * pulse)
        if active:  # poswiata
            glow = setting * 1.9
            c.create_oval(cx - glow, mid - glow * 0.75, cx + glow, mid + glow * 0.75,
                          fill=theme.blend(color or gold, bg, 0.75 - 0.25 * pulse), outline="")
        c.create_polygon(cx, mid - setting, cx + setting, mid, cx, mid + setting, cx - setting, mid,
                         fill=gold, outline="")
        inner = setting * 0.68
        stone = theme.blend(p["accent"], color, 0.45 * pulse) if active and color else p["accent"]
        c.create_polygon(cx, mid - inner, cx + inner, mid, cx, mid + inner, cx - inner, mid, fill=stone, outline="")
        c.create_polygon(cx, mid - inner, cx + inner * 0.5, mid - inner * 0.5, cx, mid, cx - inner * 0.5,
                         mid - inner * 0.5, fill=theme.blend(stone, "#ffffff", 0.35), outline="")
