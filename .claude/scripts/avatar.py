"""Awatar Achai: animowana buzka w naglowku okna (rysowana przez Pillow, ok. 16 klatek na sekunde).

Stan widac z daleka: idle - spokojny oddech i mruganie, wake/rec - slucha (fale dzwieku po bokach),
busy - mysli (kropki krazace wokol, wzrok w gore), speak - mowi (ruchome usta), wait/dialog - czeka
na decyzje (pomaranczowa poswiata), paused - spi (zamkniete oczy), start/closed/error - szara.
"""
import math
import random
import time

import theme

K = 3  # rysujemy 3x wieksze i zmniejszamy - gladkie krawedzie
DARK = "#1c1a24"


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
        grey = key in ("start", "closed", "error", "paused")
        face = p["grey"] if grey else p["accent"]
        breath = 1 + 0.025 * math.sin(now * 1.6)
        r = s * 0.33 * (breath if key in ("idle", "paused") else 1)

        def circle(x, y, rad, fill):
            d.ellipse([x - rad, y - rad, x + rad, y + rad], fill=rgb(fill))

        # poswiata / fala stanu
        if key in ("rec", "wake", "speak", "wait", "dialog", "start"):
            period = 0.9 if key in ("rec", "wake") else 1.4
            for shift in (0, 0.5):
                f = ((now / period) + shift) % 1
                circle(c, c, r + f * (s * 0.5 - r), theme.blend(color, bg, 0.45 + 0.55 * f))
        else:
            circle(c, c, r * 1.16, theme.blend(color, bg, 0.72))
        if key == "busy":  # mysli: trzy kropki krazace wokol twarzy
            for i in range(3):
                a = now * 3.2 - i * 0.55
                rad = s * (0.05 - i * 0.011)
                circle(c + math.cos(a) * r * 1.3, c + math.sin(a) * r * 1.3, rad, theme.blend(color, bg, i * 0.25))
        if key in ("rec", "wake"):  # slucha: luki fali dzwieku po obu stronach
            for i in range(3):
                f = (now * 1.8 - i * 0.33) % 1
                rad = r * (1.18 + 0.2 * i)
                col = rgb(theme.blend(color, bg, 0.15 + 0.8 * f))
                box = [c - rad, c - rad, c + rad, c + rad]
                d.arc(box, -32, 32, fill=col, width=round(s * 0.025))
                d.arc(box, 148, 212, fill=col, width=round(s * 0.025))

        # twarz: ciemniejszy spod i jasniejsza gora daja odrobine glebi
        circle(c, c, r, theme.blend(face, DARK, 0.25))
        circle(c, c - r * 0.06, r * 0.94, face)
        circle(c - r * 0.3, c - r * 0.42, r * 0.22, theme.blend(face, "#ffffff", 0.18))

        # oczy
        eye_dx, eye_y = r * 0.36, c - r * 0.1
        if now > self.blink_at + 0.14:
            self.blink_at = now + random.uniform(2.5, 6)
        blinking = self.blink_at <= now <= self.blink_at + 0.14
        white = "#ffffff"
        if key == "paused" or blinking:
            for sx in (-1, 1):
                x = c + sx * eye_dx
                d.line([x - r * 0.13, eye_y, x + r * 0.13, eye_y], fill=rgb(white), width=round(r * 0.07))
        else:
            open_ = 1.15 if key in ("rec", "wake") else 1.0
            gx, gy = 0, 0
            if key == "busy":  # wzrok w gore, powoli wedruje
                gx, gy = r * 0.05 * math.sin(now * 0.9), -r * 0.06
            for sx in (-1, 1):
                x = c + sx * eye_dx
                d.ellipse([x - r * 0.13 * open_, eye_y - r * 0.18 * open_, x + r * 0.13 * open_,
                           eye_y + r * 0.18 * open_], fill=rgb(white))
                circle(x + gx, eye_y + gy + r * 0.02, r * 0.075 * open_, DARK)

        # usta
        my, ink = c + r * 0.36, rgb(DARK)
        if key == "speak":
            h = r * (0.04 + 0.12 * abs(math.sin(now * 9) + 0.4 * math.sin(now * 23)) / 1.4)
            d.ellipse([c - r * 0.16, my - h, c + r * 0.16, my + h], fill=ink)
        elif key in ("rec", "wake", "wait", "dialog"):
            circle(c, my, r * 0.08, DARK)
        elif key == "busy":
            d.line([c - r * 0.12, my + r * 0.02, c + r * 0.14, my - r * 0.03], fill=ink, width=round(r * 0.07))
        elif key in ("closed", "error"):
            d.line([c - r * 0.15, my, c + r * 0.15, my], fill=ink, width=round(r * 0.07))
        else:
            d.arc([c - r * 0.24, my - r * 0.24, c + r * 0.24, my + r * 0.06], 25, 155, fill=ink,
                  width=round(r * 0.07))

        self.photo.paste(img.resize((self.size, self.size), Image.LANCZOS))
