"""Mirror the OctaneRender Standalone manual (docs.otoy.com/standaloneSE) as Markdown.

Usage: python3 scripts/dev/fetch-octane-docs.py docs/octane/manual <cache dir>
Needs `beautifulsoup4` and `markdownify`. The page list comes from the
manual's HelpNDoc `_toc.json` (fetched into the cache dir when missing).
"""
import json, os, re, sys, time, urllib.request
from concurrent.futures import ThreadPoolExecutor
from bs4 import BeautifulSoup
from markdownify import markdownify

BASE = "https://docs.otoy.com/standaloneSE/"
out, cache = sys.argv[1], sys.argv[2]
os.makedirs(out, exist_ok=True); os.makedirs(cache, exist_ok=True)

def path(n):  # breadcrumb
    p = []
    while n:
        p.append(n["text"]); n = by_id.get(n.get("parent"))
    return " > ".join(reversed(p))

def get(href):
    f = os.path.join(cache, href)
    if not os.path.exists(f):
        req = urllib.request.Request(BASE + href, headers={"User-Agent": "Mozilla/5.0 (Macintosh) orbis-docs-mirror"})
        with urllib.request.urlopen(req, timeout=60) as r:
            open(f, "wb").write(r.read())
    return open(f, encoding="utf-8", errors="replace").read()

toc = json.loads(get("_toc.json"))
by_id = {n["id"]: n for n in toc}


def convert(n):
    href = n.get("a_attr", {}).get("href")
    if not href: return None
    soup = BeautifulSoup(get(href), "html.parser")
    main = soup.select_one("#topic-content .main-content") or soup.select_one("#topic-content")
    for img in main.find_all("img"):
        img.replace_with(f"[image: {img.get('alt') or os.path.basename(img.get('src',''))}]")
    md = markdownify(str(main), heading_style="ATX", strip=["script", "style"])
    md = re.sub(r"\]\(([A-Za-z0-9_]+)\.html(#[^)]*)?\)", lambda m: f"]({m.group(1)}.md{m.group(2) or ''})", md)
    md = re.sub(r"\n{3,}", "\n\n", md).strip()
    name = href.replace(".html", ".md")
    open(os.path.join(out, name), "w").write(f"# {n['text']}\n\n_{path(n)}_ — source: {BASE}{href}\n\n{md}\n")
    return name

with ThreadPoolExecutor(8) as ex:
    names = list(ex.map(convert, toc))
lines = ["# OctaneRender Standalone manual (Markdown mirror)", "",
         f"Downloaded from {BASE} on {time.strftime('%Y-%m-%d')} by `scripts/fetch-octane-docs.py`; images are left out.", ""]
for n, name in zip(toc, names):
    if not name: continue
    depth = path(n).count(" > ")
    lines.append(f"{'  ' * depth}- [{n['text']}]({name})")
open(os.path.join(out, "README.md"), "w").write("\n".join(lines) + "\n")
print(len([x for x in names if x]), "pages")
