"""Kopie zapasowe ustawien i danych Achai.

Uzycie:
  python -m achaja backup create            kopia teraz (zip w folderze kopii)
  python -m achaja backup list              kopie od najnowszej
  python -m achaja backup auto              kopia, jesli ostatnia jest starsza niz backup.every_hours (domyslnie 24)
  python -m achaja backup restore <plik>    przywraca kopie (najpierw robi kopie obecnego stanu); potem uruchom Achaje ponownie
  python -m achaja backup folder            sciezka folderu kopii

W kopii: config.json (konta pocztowe z haslami!), .mcp.json, .claude/settings.local.json, .claude/CLAUDE.local.md
i state/ (token Home Assistant, sesje agentow, harmonogram, dziennik poczty) - bez pamieci podrecznej mowy.
Folder: backup.folder w config.json, domyslnie Dokumenty\\Achaja - kopie. Trzymamy backup.keep najnowszych (10).
Kopia zawiera hasla i tokeny - nie wrzucaj jej do publicznych miejsc.
"""
import argparse
import sys
import time
import zipfile
from pathlib import Path

from achaja.core.common import ROOT, STATE, load_config, read_json, write_json

FILES = ("config.json", ".mcp.json", ".claude/settings.local.json", ".claude/CLAUDE.local.md")
SKIP_DIRS = {"tts_cache", "__pycache__"}
SKIP_SUFFIX = (".mp3", ".wav", ".pid", ".tmp")
LAST = STATE / "backup.json"


def settings():
    try:
        cfg = load_config().get("backup") or {}
    except Exception:
        cfg = {}
    folder = Path(cfg.get("folder") or Path.home() / "Documents" / "Achaja - kopie")
    return folder, int(cfg.get("keep", 10)), float(cfg.get("every_hours", 24)), cfg.get("auto", True)


def members():
    for rel in FILES:
        p = ROOT / rel
        if p.is_file():
            yield p, rel
    if STATE.is_dir():
        for p in sorted(STATE.rglob("*")):
            rel = p.relative_to(ROOT)
            if p.is_file() and not SKIP_DIRS & set(rel.parts) and not p.name.endswith(SKIP_SUFFIX):
                yield p, rel.as_posix()


def create(note=""):
    folder, keep, _, _ = settings()
    folder.mkdir(parents=True, exist_ok=True)
    name = folder / f"Achaja-{time.strftime('%Y%m%d-%H%M%S')}{('-' + note) if note else ''}.zip"
    count = 0
    with zipfile.ZipFile(name, "w", zipfile.ZIP_DEFLATED) as z:
        for path, rel in members():
            try:
                z.write(path, rel)
                count += 1
            except OSError:
                pass  # plik zablokowany przez inny proces (np. log w trakcie zapisu)
    write_json(LAST, {"at": time.time(), "file": str(name)})
    for old in backups()[keep:]:
        try:
            old.unlink()
        except OSError:
            pass
    return name, count


def backups():
    folder = settings()[0]
    return sorted(folder.glob("Achaja-*.zip"), key=lambda p: p.stat().st_mtime, reverse=True) if folder.is_dir() else []


def due():
    _, _, every, auto = settings()
    return bool(auto) and time.time() - read_json(LAST, {}).get("at", 0) >= every * 3600


def restore(path):
    path = Path(path)
    if not path.is_file():
        found = [b for b in backups() if b.name == path.name or b.stem == path.stem]
        if not found:
            raise SystemExit(f"Nie ma kopii {path}.")
        path = found[0]
    safety, _ = create(note="przed-przywroceniem")
    with zipfile.ZipFile(path) as z:
        for info in z.infolist():
            target = (ROOT / info.filename).resolve()
            if ROOT.resolve() not in target.parents:  # zip z "..\" - nie wychodzimy poza folder Achai
                continue
            if not (info.filename in FILES or info.filename.startswith("state/")):
                continue
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(z.read(info))
    return path, safety


def main(argv=None):
    ap = argparse.ArgumentParser(description="Kopie zapasowe Achai")
    sub = ap.add_subparsers(dest="cmd", required=True)
    for name in ("create", "list", "auto", "folder"):
        sub.add_parser(name)
    sub.add_parser("restore").add_argument("file")
    args = ap.parse_args(argv)
    if args.cmd == "create" or (args.cmd == "auto" and due()):
        name, count = create()
        print(f"Kopia zapisana: {name} ({count} plikow).")
    elif args.cmd == "auto":
        print("Ostatnia kopia jest aktualna.")
    elif args.cmd == "list":
        items = backups()
        print("\n".join(f"{time.strftime('%d.%m.%Y %H:%M', time.localtime(b.stat().st_mtime))}  "
                        f"{b.stat().st_size // 1024} KB  {b.name}" for b in items) or "Brak kopii.")
    elif args.cmd == "folder":
        print(settings()[0])
    elif args.cmd == "restore":
        path, safety = restore(args.file)
        print(f"Przywrocono {path.name}. Poprzedni stan zapisalam w {safety.name}. Uruchom Achaje ponownie.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
