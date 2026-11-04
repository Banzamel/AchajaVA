"""Pas jak sciezki na plytce procesora: posrodku uklad scalony z nozkami, od niego w obie strony glowna
sciezka i odgalezienia zalamane pod katem 45 stopni, zakonczone przelotkami (via). Sciezki gasna ku koncom.
Gdy Achaja slucha, mysli albo mowi, rdzen ukladu pulsuje w kolorze stanu, a po sciezkach biegna pakiety danych.
Uzywany w naglowku okna (app.py) i w bocznym menu ustawien (setup/configure.py).
"""
import math
import time

from achaja.ui import theme

# odgalezienia: (start 0..1 wzdluz sciezki, kierunek -1 gora / 1 dol, koniec 0..1)
BRANCHES = ((0.06, -1, 0.42), (0.22, 1, 0.66), (0.5, -1, 0.9), (0.72, 1, 0.97))


class Band:
    def __init__(self, master, height, background_key="bg"):
        import tkinter as tk
        self.c = tk.Canvas(master, height=height, highlightthickness=0, borderwidth=0)
        self.h = height
        self.bg_key = background_key
        self.static = None  # (szerokosc, paleta) ostatniego spokojnego rysunku - bez ruchu nie rysujemy ponownie
        self.c.bind("<Configure>", lambda e: setattr(self, "static", None))

    def _paths(self, cx, chip, span, mid, lane):
        """Lamane sciezek: lista (punkty, czy konczy sie przelotka, kolor: 0 zielen, 1 cyjan, 2 fiolet)."""
        paths = []
        for side in (-1, 1):
            def x(f):
                return cx + side * (chip + f * span)
            paths.append(([(x(0), mid), (x(1), mid)], False, 0))
            for i, (start, direction, end) in enumerate(BRANCHES):
                y = mid + direction * lane
                x0 = x(start)
                x1 = x0 + side * lane  # 45 stopni: tyle samo w bok, co w gore/dol
                paths.append(([(x0, mid), (x1, y), (x(end), y)], True, (i + (side > 0)) % 3))
        return paths

    def draw(self, p, color=None, active=False):
        """p: paleta; color: kolor stanu; active: rdzen pulsuje, po sciezkach biegna pakiety."""
        c, h = self.c, self.h
        w = c.winfo_width()
        if w < 10:
            return
        if not active and self.static == (w, id(p)):
            return
        self.static = None if active else (w, id(p))
        bg, neon = p[self.bg_key], p["accent"]
        traces = (p["trace"], p["trace2"], p["trace3"])
        neons = (p["accent"], p["cyan"], p["violet"])
        color = color or neon
        c.configure(background=bg)
        c.delete("all")
        now = time.time()
        mid, cx = h / 2, w / 2
        chip = h * 0.55  # polowa szerokosci ukladu
        lane = h * 0.3  # odstep odgalezien od glownej sciezki
        span = cx - chip - h * 0.2
        lw = max(1, round(h * 0.07))

        def faded(col, f):
            return theme.blend(bg, col, max(0.0, (1 - f) ** 1.3))

        for points, via, hue in self._paths(cx, chip, span, mid, lane):
            line = theme.blend(traces[hue], neons[hue], 0.55 if active else 0.4)
            glow = theme.blend(bg, traces[hue], 0.45)
            # kazdy odcinek dzielony na kawalki - gradient wygaszania ku koncom; pod spodem szersza poswiata
            for (xa, ya), (xb, yb) in zip(points, points[1:]):
                n = max(1, int(abs(xb - xa) / 6))
                for i in range(n):
                    t0, t1 = i / n, (i + 1) / n
                    px0, py0 = xa + (xb - xa) * t0, ya + (yb - ya) * t0
                    px1, py1 = xa + (xb - xa) * t1, ya + (yb - ya) * t1
                    f = abs((px0 + px1) / 2 - cx) / (cx or 1)
                    c.create_line(px0, py0, px1, py1, fill=faded(glow, f), width=lw * 3)
                    c.create_line(px0, py0, px1, py1, fill=faded(line, f), width=lw)
            if via:
                vx, vy = points[-1]
                f = abs(vx - cx) / (cx or 1)
                rv = h * 0.1
                c.create_oval(vx - rv, vy - rv, vx + rv, vy + rv, fill=bg, outline=faded(neons[hue], f * 0.85),
                              width=lw)

            if active:  # pakiet danych: jasna kropka z poswiata biegnie od ukladu ku koncom
                seg = [math.dist(a, b) for a, b in zip(points, points[1:])]
                total = sum(seg) or 1
                phase = (now * (0.55 if not via else 0.8) + (points[-1][1] - mid) * 0.01 + points[0][0] * 0.003) % 1
                d = phase * total
                for (xa, ya), (xb, yb), length in zip(points, points[1:], seg):
                    if d <= length:
                        t = d / (length or 1)
                        qx, qy = xa + (xb - xa) * t, ya + (yb - ya) * t
                        f = abs(qx - cx) / (cx or 1)
                        for rad, mix in ((h * 0.22, 0.75), (h * 0.12, 0.0)):
                            col = theme.blend(color, bg, min(1.0, mix + f * 0.8))
                            c.create_oval(qx - rad, qy - rad, qx + rad, qy + rad, fill=col, outline="")
                        break
                    d -= length

        # uklad scalony: ciemny korpus z neonowa ramka, nozki u gory i u dolu, rdzen w kolorze stanu
        pulse = 0.5 + 0.5 * math.sin(now * 4) if active else 0.0
        half_h = h * 0.36
        if active:
            g = chip * (1.25 + 0.2 * pulse)
            c.create_rectangle(cx - g, mid - half_h * 1.5, cx + g, mid + half_h * 1.5,
                               fill=theme.blend(color, bg, 0.82 - 0.1 * pulse), outline="")
        for i in range(4):
            px_ = cx - chip * 0.66 + i * chip * 0.44
            for sy in (-1, 1):
                c.create_line(px_, mid + sy * half_h, px_, mid + sy * (half_h + h * 0.13), fill=neon, width=lw)
        c.create_rectangle(cx - chip, mid - half_h, cx + chip, mid + half_h, fill=p["surface2"], outline=neon,
                           width=lw)
        core = h * (0.13 + 0.04 * pulse)
        core_col = theme.blend(neon, color, 0.5 + 0.5 * pulse) if active else neon
        c.create_rectangle(cx - core, mid - core, cx + core, mid + core, fill=core_col, outline="")
