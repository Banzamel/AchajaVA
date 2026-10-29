"""Oznacza folder Achai jako zaufany w Claude Code (~/.claude.json: projects[<folder>].hasTrustDialogAccepted).

Bez tego Claude Code pomija .claude/settings.local.json (wlaczone serwery MCP) i przy kazdym starcie
ukrytej sesji Achai pyta o serwer z .mcp.json - w oknie, ktorego nikt nie widzi. Uruchamia go install.ps1.
  python setup/trust.py [folder]      domyslnie folder Achai
Przed zmiana robi kopie ~/.claude.json.achaja-backup. Nic nie zmienia, gdy folder jest juz zaufany.
"""
import json
import os
import shutil
import sys
from pathlib import Path


def main():
    folder = Path(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parents[1]).resolve()
    key = str(folder).replace("\\", "/")
    path = Path.home() / ".claude.json"
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        data = {}  # Claude Code jeszcze nie uruchamiany - utworzy reszte pliku sam
    except (OSError, ValueError) as exc:
        print(f"Nie udalo sie odczytac {path}: {exc}")
        return 1
    project = data.setdefault("projects", {}).setdefault(key, {})
    if project.get("hasTrustDialogAccepted"):
        print(f"Folder {key} jest juz zaufany w Claude Code.")
        return 0
    project["hasTrustDialogAccepted"] = True
    if path.exists():
        shutil.copy2(path, str(path) + ".achaja-backup")
    tmp = Path(str(path) + ".achaja-tmp")
    tmp.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")
    os.replace(tmp, path)
    print(f"Oznaczono {key} jako zaufany folder w Claude Code.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
