"""Awatar Achai - slowianska wojowniczka: jasne warkocze, karmazynowa opaska ze zlotym rombem i wianek.
Rysowany przez Pillow, ok. 16 klatek na sekunde.

Stan widac z daleka: idle - spokojny oddech i mruganie, wake/rec - slucha (fale dzwieku po bokach),
busy - mysli (zlote iskry krazace wokol, wzrok w gore), speak - mowi (ruchome usta), wait/dialog - czeka
na decyzje (poswiata), paused - spi (zamkniete oczy), start/closed/error - wyblakla.
"""
import math
import random
import time

import theme

K = 3  # rysujemy 3x wieksze i zmniejszamy - gladkie krawedzie
INK = "#2a1a14"


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
            return theme.blend(col, p["grey"], 0.55) if faded else col

        breath = 1 + 0.025 * math.sin(now * 1.6)
        r = s * 0.27 * (breath if key in ("idle", "paused") else 1)
        cy = c - s * 0.02  # twarz troche wyzej - pod nia warkocze

        def circle(x, y, rad, fill):
            d.ellipse([x - rad, y - rad, x + rad, y + rad], fill=rgb(fill))

        # poswiata / fala stanu
        if key in ("rec", "wake", "speak", "wait", "dialog", "start"):
            period = 0.9 if key in ("rec", "wake") else 1.4
            for shift in (0, 0.5):
                f = ((now / period) + shift) % 1
                circle(c, c, r * 1.25 + f * (s * 0.5 - r * 1.25), theme.blend(color, bg, 0.45 + 0.55 * f))
        else:
            circle(c, c, s * 0.47, theme.blend(color, bg, 0.82))
        if key in ("rec", "wake"):  # slucha: luki fali dzwieku po obu stronach
            for i in range(3):
                f = (now * 1.8 - i * 0.33) % 1
                rad = r * (1.45 + 0.17 * i)
                col = rgb(theme.blend(color, bg, 0.15 + 0.8 * f))
                box = [c - rad, cy - rad, c + rad, cy + rad]
                d.arc(box, -30, 30, fill=col, width=round(s * 0.022))
                d.arc(box, 150, 210, fill=col, width=round(s * 0.022))

        hair, hair_dark = tone(p["hair"]), tone(theme.blend(p["hair"], INK, 0.35))
        # warkocze: splecione ogniwa od skroni w dol, na koncu czerwona wstazka
        for sx in (-1, 1):
            for i in range(6):
                x = c + sx * (r * 0.92 + (r * 0.07 if i % 2 else -r * 0.05))
                y = cy + r * 0.05 + i * r * 0.2
                rad = r * (0.17 - i * 0.012)
                d.ellipse([x - rad, y - rad * 1.15, x + rad, y + rad * 1.15], fill=rgb(hair),
                          outline=rgb(hair_dark), width=max(1, round(K * 0.8)))
            tip_y = cy + r * 1.25
            x = c + sx * r * 0.92
            d.polygon([(x - r * 0.13, tip_y), (x + r * 0.13, tip_y), (x + sx * r * 0.06, tip_y + r * 0.22)],
                      fill=rgb(tone(p["accent"])))

        # wlosy (czubek glowy), twarz, rumience
        circle(c, cy - r * 0.06, r * 1.03, hair)
        skin = tone(p["skin"])
        d.ellipse([c - r * 0.86, cy - r * 0.62, c + r * 0.86, cy + r * 0.92], fill=rgb(skin))
        for sx in (-1, 1):
            circle(c + sx * r * 0.48, cy + r * 0.3, r * 0.12, theme.blend(skin, p["accent"], 0.25))

        # karmazynowa opaska na czole ze zlotym rombem
        # cienka przepaska wygieta wzdluz czola (luk okregu), na srodku zloty romb, po bokach kropki haftu
        bcx, bcy, brad = c, cy + r * 0.3, r * 0.92
        d.arc([bcx - brad, bcy - brad, bcx + brad, bcy + brad], 208, 332, fill=rgb(tone(p["accent"])),
              width=round(r * 0.15))
        gold = tone(p["gold"])
        top = bcy - brad + r * 0.07
        d.polygon([(c, top - r * 0.13), (c + r * 0.1, top), (c, top + r * 0.13), (c - r * 0.1, top)], fill=rgb(gold))
        for deg in (232, 250, 290, 308):
            a = math.radians(deg)
            circle(bcx + math.cos(a) * (brad - r * 0.07), bcy + math.sin(a) * (brad - r * 0.07), r * 0.03, gold)

        # wianek: kwiaty i listki na czubku glowy (przy mysleniu delikatnie migocza)
        flowers = (p["accent"], p["gold"], p["flower2"], p["accent"], p["gold"])
        for i, col in enumerate(flowers):
            ang = math.radians(-150 + i * 30)
            fx, fy = c + math.cos(ang) * r * 0.98, cy - r * 0.06 + math.sin(ang) * r * 0.98
            leaf_ang = ang + 0.5
            lx, ly = fx + math.cos(leaf_ang) * r * 0.16, fy + math.sin(leaf_ang) * r * 0.16
            d.ellipse([lx - r * 0.08, ly - r * 0.05, lx + r * 0.08, ly + r * 0.05], fill=rgb(tone(p["green"])))
            pulse = 1 + (0.15 * math.sin(now * 5 + i) if key == "busy" else 0)
            for j in range(5):  # platki
                pa = j * 2 * math.pi / 5 + now * (0.6 if key == "busy" else 0)
                circle(fx + math.cos(pa) * r * 0.1 * pulse, fy + math.sin(pa) * r * 0.1 * pulse,
                       r * 0.085 * pulse, tone(col))
            circle(fx, fy, r * 0.06, tone(p["gold"] if col != p["gold"] else p["on_accent"]))

        if key == "busy":  # mysli: zlote iskry krazace wokol
            for i in range(3):
                a = now * 3.2 - i * 0.55
                rad = s * (0.035 - i * 0.008)
                circle(c + math.cos(a) * r * 1.6, cy + math.sin(a) * r * 1.6, rad, theme.blend(p["gold"], bg, i * 0.25))

        # oczy
        eye_dx, eye_y = r * 0.34, cy + r * 0.02
        if now > self.blink_at + 0.14:
            self.blink_at = now + random.uniform(2.5, 6)
        blinking = self.blink_at <= now <= self.blink_at + 0.14
        if key == "paused" or blinking:
            for sx in (-1, 1):
                x = c + sx * eye_dx
                d.arc([x - r * 0.13, eye_y - r * 0.1, x + r * 0.13, eye_y + r * 0.08], 20, 160, fill=rgb(INK),
                      width=round(r * 0.05))
        else:
            open_ = 1.15 if key in ("rec", "wake") else 1.0
            gx, gy = 0, 0
            if key == "busy":  # wzrok w gore, powoli wedruje
                gx, gy = r * 0.04 * math.sin(now * 0.9), -r * 0.05
            for sx in (-1, 1):
                x = c + sx * eye_dx
                d.ellipse([x - r * 0.12 * open_, eye_y - r * 0.14 * open_, x + r * 0.12 * open_,
                           eye_y + r * 0.14 * open_], fill=rgb("#ffffff"))
                circle(x + gx, eye_y + gy + r * 0.01, r * 0.085 * open_, tone(p["flower2"]))
                circle(x + gx, eye_y + gy + r * 0.01, r * 0.045 * open_, INK)
                circle(x + gx + r * 0.03, eye_y + gy - r * 0.03, r * 0.02, "#ffffff")
                # brwi - pewne siebie
                d.line([x - r * 0.14, eye_y - r * 0.24, x + r * 0.13, eye_y - r * 0.27 + sx * 0], fill=rgb(hair_dark),
                       width=round(r * 0.045))

        # usta
        my, lips = cy + r * 0.52, rgb(tone(p["accent"]))
        if key == "speak":
            h = r * (0.03 + 0.1 * abs(math.sin(now * 9) + 0.4 * math.sin(now * 23)) / 1.4)
            d.ellipse([c - r * 0.14, my - h, c + r * 0.14, my + h], fill=lips)
        elif key in ("rec", "wake", "wait", "dialog"):
            circle(c, my, r * 0.065, tone(p["accent"]))
        elif key == "busy":
            d.line([c - r * 0.1, my + r * 0.02, c + r * 0.12, my - r * 0.02], fill=lips, width=round(r * 0.06))
        elif key in ("closed", "error"):
            d.line([c - r * 0.12, my, c + r * 0.12, my], fill=lips, width=round(r * 0.06))
        else:
            d.arc([c - r * 0.2, my - r * 0.18, c + r * 0.2, my + r * 0.06], 25, 155, fill=lips, width=round(r * 0.065))

        self.photo.paste(img.resize((self.size, self.size), Image.LANCZOS))
