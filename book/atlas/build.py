#!/usr/bin/env python3
"""Build the ferroplan Atlas: merge content/*.json into template.html.

  python3 book/atlas/build.py            -> book/src/atlas.html (full page for mdBook / GitHub Pages)
  python3 book/atlas/build.py --fragment OUT  -> body-only page (for the claude.ai artifact preview)
"""
import json, pathlib, subprocess, sys, re, datetime

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parent.parent
ORDER = ["sec-frontends", "sec-pipeline", "sec-example", "sec-heuristics", "sec-search",
         "sec-modes", "sec-session", "sec-sat", "sec-data", "sec-tooling"]

def load():
    root = json.loads((HERE / "content" / "root.json").read_text())
    secs = {}
    for f in sorted((HERE / "content").glob("*.json")):
        if f.name == "root.json":
            continue
        data = json.loads(f.read_text())
        for s in (data if isinstance(data, list) else [data]):
            secs[s["id"]] = s
    root["children"] = [secs.pop(k) for k in ORDER if k in secs] + list(secs.values())
    return root

def check(root):
    ids, dup, links = set(), [], []
    def walk(n):
        if n["id"] in ids: dup.append(n["id"])
        ids.add(n["id"])
        for l in n.get("links", []): links.append((n["id"], l["to"]))
        for c in n.get("children", []): walk(c)
    walk(root)
    dangling = [(a, b) for a, b in links if b not in ids]
    for d in dup: print("duplicate id:", d, file=sys.stderr)
    for a, b in dangling: print(f"dangling link: {a} -> {b}", file=sys.stderr)
    return len(ids), len(links), len(dangling)

def main():
    root = load()
    n, nl, nd = check(root)
    ver = re.search(r'^version = "([^"]+)"', (REPO / "Cargo.toml").read_text(), re.M).group(1)
    try:
        sha = subprocess.check_output(["git", "-C", str(REPO), "rev-parse", "--short", "HEAD"], text=True).strip()
    except Exception:
        sha = "?"
    root["version"] = f"v{ver}"
    root["generated"] = f"Generated from ferroplan {ver} ({sha}) on {datetime.date.today().isoformat()}"
    tpl = (HERE / "template.html").read_text()
    blob = json.dumps(root, ensure_ascii=False, separators=(",", ":")).replace("</", "<\\/")
    page = tpl.replace("/*__ATLAS_DATA__*/null", blob)
    if len(sys.argv) > 2 and sys.argv[1] == "--fragment":
        out = pathlib.Path(sys.argv[2])
        out.write_text(page)
    else:
        head, rest = page.split("<style>", 1)
        page = ('<!doctype html>\n<html lang="en">\n<head>\n<meta charset="utf-8">\n'
                '<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">\n'
                + head + "<style>\n[hidden]{display:none!important}\n" + rest.replace("</style>", "</style>\n</head>\n<body>", 1)
                + "\n</body>\n</html>\n")
        out = REPO / "book" / "src" / "atlas.html"
        out.write_text(page)
    print(f"{out}: {n} nodes, {nl} links ({nd} dangling), {len(page)//1024} KiB")

main()
