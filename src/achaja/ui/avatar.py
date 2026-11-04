"""Awatar Achai - rdzen AI: uklad scalony z neonowa ramka i nozkami, od niego sciezki jak na plytce
procesora (zalamane pod katem 45 stopni, zakonczone przelotkami), na ukladzie neonowe oczy i usta.
Rysowany przez Pillow, ok. 16 klatek na sekunde.

Stan widac z daleka: idle - spokojny oddech poswiaty i mruganie, wake/rec - slucha (fale rozchodza sie
od ukladu), busy - mysli (pakiety danych biegna po sciezkach, linia skanu przesuwa sie po ukladzie,
wzrok w gore), speak - mowi (usta jak korektor graficzny), wait/dialog - czeka na decyzje (poswiata),
paused - spi (zamkniete oczy), start/closed/error - wyblakla.
"""
import math
import random
import time

from achaja.ui import theme

K = 3  # rysujemy 3x wieksze i zmniejszamy - gladkie krawedzie
TRACES = 8  # sciezki wychodzace z ukladu


def rgb(color):
    return tuple(int(color[i:i + 2], 16) for i in (1, 3, 5))


class Avatar:
    def __init__(self, master, size):
        import tkinter as tk
        from PIL import ImageTk, Image
        self.size = size
        self.photo = ImageTk.PhotoImage(Image.new("RGB", (size, size)), master=master)
        self.widget = tk.Label(master, image=self.photo, borderwidth=0, highlightthickness=0)
        self.blink_at = time.time() + 2

    def draw(self, key, color, p):
        """key: stan (jak w app.STATE_COLORS), color: kolor stanu, p: paleta motywu."""
        from PIL import Image, ImageDraw
        now = time.time()
        s = self.size * K
        c = s / 2
        bg = p["bg"]
        img = Image.new("RGB", (s, s), rgb(bg))
        d = ImageDraw.Draw(img)
        faded = key in ("start", "closed", "error", "paused")

        def tone(col):  # wyblakle kolory, gdy Achaja spi albo sesja nie dziala
            return theme.blend(col, p["grey"], 0.6) if faded else col

        neon = tone(p["accent"])
        state = tone(color)
        lw = max(1, round(K * 1.1))

        def circle(x, y, rad, fill=None, outline=None, width=1):
            d.ellipse([x - rad, y - rad, x + rad, y + rad], fill=rgb(fill) if fill else None,
                      outline=rgb(outline) if outline else None, width=width)

        breath = 0.5 + 0.5 * math.sin(now * 1.6)
        half = s * 0.2  # polowa boku ukladu

        # poswiata / fale stanu
        if key in ("rec", "wake", "speak", "wait", "dialog", "start"):
            period = 0.9 if key in ("rec", "wake") else 1.4
            for shift in (0, 0.5):
                f = ((now / period) + shift) % 1
                rad = half * 1.5 + f * (s * 0.5 - half * 1.5)
                circle(c, c, rad, outline=theme.blend(state, bg, 0.3 + 0.7 * f), width=round(K * 1.4))
        circle(c, c, s * 0.34, fill=theme.blend(state, bg, 0.93 - (0.04 * breath if key == "idle" else 0)))

        # sciezki: od nozek ukladu na zewnatrz, zalamanie 45 stopni, przelotka na koncu; trzy neony na zmiane
        hues = [(tone(theme.blend(p[t], p[n], 0.45)), tone(p[n]))
                for t, n in (("trace", "accent"), ("trace2", "cyan"), ("trace3", "violet"))]
        paths = []
        for i in range(TRACES):
            a = math.pi / 4 * i + math.pi / 8  # miedzy osiami - nie zaslaniaja nozek
            ux, uy = math.cos(a), math.sin(a)
            p0 = (c + ux * half * 1.15, c + uy * half * 1.15)
            p1 = (c + ux * s * 0.33, c + uy * s * 0.33)
            turn = a + (math.pi / 4 if i % 2 else -math.pi / 4)
            p2 = (p1[0] + math.cos(turn) * s * 0.09, p1[1] + math.sin(turn) * s * 0.09)
            paths.append((p0, p1, p2))
            line, via = hues[i % 3]
            d.line([p0, p1, p2], fill=rgb(line), width=lw, joint="curve")
            circle(*p2, s * 0.022, fill=bg, outline=theme.blend(via, bg, 0.2), width=lw)

        if key in ("busy", "speak", "rec", "wake"):  # pakiety danych na sciezkach
            speed = 1.4 if key == "busy" else 0.9
            for i, (p0, p1, p2) in enumerate(paths):
                f = (now * speed + i * 0.37) % 1
                if f < 0.7:
                    t = f / 0.7
                    x, y = p0[0] + (p1[0] - p0[0]) * t, p0[1] + (p1[1] - p0[1]) * t
                else:
                    t = (f - 0.7) / 0.3
                    x, y = p1[0] + (p2[0] - p1[0]) * t, p1[1] + (p2[1] - p1[1]) * t
                circle(x, y, s * 0.03, fill=theme.blend(state, bg, 0.6))
                circle(x, y, s * 0.015, fill=state)

        # uklad: nozki na czterech bokach, ciemny korpus, neonowa ramka
        pin = s * 0.045
        for k in range(3):
            o = (k - 1) * half * 0.6
            for sx, sy, horiz in ((0, -1, False), (0, 1, False), (-1, 0, True), (1, 0, True)):
                if horiz:
                    x0 = c + sx * half
                    d.line([(x0, c + o), (x0 + sx * pin, c + o)], fill=rgb(neon), width=lw * 2)
                else:
                    y0 = c + sy * half
                    d.line([(c + o, y0), (c + o, y0 + sy * pin)], fill=rgb(neon), width=lw * 2)
        box = [c - half, c - half, c + half, c + half]
        d.rounded_rectangle(box, radius=s * 0.04, fill=rgb(p["surface2"]), outline=rgb(neon),
                            width=round(K * 1.6))

        if key == "busy":  # linia skanu po ukladzie
            y = c - half * 0.85 + (now * 1.2 % 1) * half * 1.7
            d.line([(c - half * 0.8, y), (c + half * 0.8, y)], fill=rgb(theme.blend(state, p["surface2"], 0.45)),
                   width=lw)

        # oczy: neonowe pastylki w kolorze stanu
        eye_dx, eye_y = half * 0.42, c - half * 0.18
        if now > self.blink_at + 0.14:
            self.blink_at = now + random.uniform(2.5, 6)
        blinking = self.blink_at <= now <= self.blink_at + 0.14
        eye_col = state if key != "idle" else neon
        for sx in (-1, 1):
            x = c + sx * eye_dx
            if key == "paused" or blinking:
                d.line([(x - half * 0.16, eye_y), (x + half * 0.16, eye_y)], fill=rgb(eye_col), width=lw * 2)
                continue
            open_ = 1.2 if key in ("rec", "wake") else 1.0
            gy = -half * 0.08 if key == "busy" else 0
            ew, eh = half * 0.15, half * 0.22 * open_
            circle(x, eye_y + gy, eh * 1.5, fill=theme.blend(eye_col, p["surface2"], 0.75))  # poswiata
            d.rounded_rectangle([x - ew, eye_y + gy - eh, x + ew, eye_y + gy + eh], radius=ew, fill=rgb(eye_col))

        # usta
        my = c + half * 0.45
        if key == "speak":  # korektor graficzny
            bars = 5
            bw = half * 0.12
            for i in range(bars):
                hgt = half * (0.06 + 0.22 * abs(math.sin(now * (7 + i * 1.7) + i)))
                x = c + (i - (bars - 1) / 2) * bw * 1.6
                d.rectangle([x - bw / 2, my - hgt, x + bw / 2, my + hgt], fill=rgb(state))
        elif key in ("rec", "wake", "wait", "dialog"):
            circle(c, my, half * 0.09, fill=state)
        elif key in ("closed", "error", "busy"):
            d.line([(c - half * 0.22, my), (c + half * 0.22, my)], fill=rgb(eye_col), width=lw * 2)
        else:
            d.arc([c - half * 0.3, my - half * 0.25, c + half * 0.3, my + half * 0.1], 25, 155, fill=rgb(eye_col),
                  width=lw * 2)

        self.photo.paste(img.resize((self.size, self.size), Image.LANCZOS))
