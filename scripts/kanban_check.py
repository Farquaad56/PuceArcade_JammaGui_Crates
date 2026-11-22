# Compare le corps de chaque carte Hermes au fichier tasks/<id>.md (usage : python check_bodies.py [From])
import json, subprocess, sys, pathlib
root = pathlib.Path(r"H:\__Emulator__\PuceArcade_JammaGui_Crates")
frm = sys.argv[1] if len(sys.argv) > 1 else "J000"
cards = json.loads((root / "tasks/board.json").read_text(encoding="utf-8"))
lst = subprocess.run(["hermes", "kanban", "--board", "jamma", "list", "--json"], capture_output=True).stdout.decode("utf-8")
tasks = json.loads(lst)
tasks = tasks if isinstance(tasks, list) else tasks.get("tasks", [])
by_title = {t["title"]: t["id"] for t in tasks}
active = False; ok = bad = 0
for c in cards:
    if c["id"] == frm: active = True
    if not active: continue
    tid = by_title.get(c["title"])
    if not tid: print(c["id"], "introuvable"); bad += 1; continue
    j = json.loads(subprocess.run(["hermes", "kanban", "--board", "jamma", "show", tid, "--json"], capture_output=True).stdout.decode("utf-8"))
    body = (j.get("task") or j).get("body", "")
    want = (root / c["file"]).read_text(encoding="utf-8")
    n = lambda s: s.replace("\r\n", "\n").strip()
    if n(body) == n(want): ok += 1
    else:
        bad += 1
        a, b = n(body), n(want)
        k = next((i for i in range(min(len(a), len(b))) if a[i] != b[i]), min(len(a), len(b)))
        print(f"{c['id']} ({c['status']}) DIFFERENT a {k}: hermes={a[k-20:k+30]!r} fichier={b[k-20:k+30]!r}")
print(f"identiques={ok} differents={bad}")
