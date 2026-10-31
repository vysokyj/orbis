"""Mirror NVIDIA's Iray and MDL documentation (raytracing-docs.nvidia.com):
the PDF editions, each also as plain text to grep.

Usage: python3 scripts/dev/fetch-nvidia-docs.py [docs/nvidia]
Needs `pypdf` (text). Downloads go through macOS's /usr/bin/curl: the
site doesn't send its intermediate certificate, which OpenSSL clients
(Python, MacPorts curl) then refuse and the system's TLS fetches. The
PDF names carry a release date (manual.261007.A4.pdf); they are found on
each section's index page, so a new release is picked up. Already
downloaded files are kept (delete one to fetch it again).
"""
import html, logging, os, re, subprocess, sys, time, urllib.parse
from pypdf import PdfReader

logging.getLogger("pypdf").setLevel(logging.ERROR)  # font-encoding warnings

BASE = "https://raytracing-docs.nvidia.com/"
# Section index pages that link a PDF edition, and fixed PDFs.
SECTIONS = {
    "iray-manual": "iray/manual/index.html",                  # Iray programmer's manual: render modes, options, environment, tone mapping
    "iray-lpe": "iray/light_path_expressions/index.html",     # light path expressions (canvases)
    "iray-server": "iray_server/index.html",                  # Iray Server user and admin guide
    "mdl-introduction": "mdl/introduction/index.html",        # MDL introduction
    "mdl-handbook": "mdl/handbook/mdl_handbook/index.html",   # MDL handbook: the df:: layering and modifiers by example
    "mdl-core-definitions": "mdl/core_definitions/index.html",  # nvidia::core_definitions
}
FIXED = {
    "mdl-spec": "iray/mdl/specification/MDL_spec_latest.pdf",  # MDL language specification: df::, base::, semantics
    "iray-relnotes": "iray/relnotes.pdf",
    "iray-overview": "iray/presentations/iray_overview/nvidia_iray_rendering_system.pdf",
}
# HTML-only references, kept as text: the ::base module (anisotropy_conversion,
# blend_color_layers, flake_noise_texture, …) that DAZ's MDL calls.
PAGES = {
    "mdl-base-module": "mdl/base_module/pages/base.html",
}

out = sys.argv[1] if len(sys.argv) > 1 else "docs/nvidia"
os.makedirs(os.path.join(out, "pdf"), exist_ok=True)
os.makedirs(os.path.join(out, "txt"), exist_ok=True)


def get(url):
    return subprocess.run(["/usr/bin/curl", "-fsSL", "--max-time", "300", "-A", "Mozilla/5.0 (Macintosh) orbis-docs-mirror", url],
                          check=True, capture_output=True).stdout


urls = dict((k, BASE + v) for k, v in FIXED.items())
for name, page in SECTIONS.items():
    index = get(BASE + page).decode("utf-8", "replace")
    pdfs = re.findall(r'href="([^"#]+\.A4\.pdf)"', index) or re.findall(r'href="([^"#]+\.pdf)"', index)
    if not pdfs:
        print(f"{name}: no PDF on {page}", file=sys.stderr)
        continue
    urls[name] = urllib.parse.urljoin(BASE + page, pdfs[0])

lines = ["# NVIDIA Iray and MDL documentation (local mirror)", "",
         f"Downloaded from {BASE} on {time.strftime('%Y-%m-%d')} by `scripts/dev/fetch-nvidia-docs.py`.",
         "NVIDIA's copyrighted text: git-ignored, never committed. `txt/` is the PDFs' text (pypdf) to grep;",
         "page breaks are form feeds.", ""]
for name, url in sorted(urls.items()):
    pdf = os.path.join(out, "pdf", f"{name}.pdf")
    if not os.path.exists(pdf):
        open(pdf, "wb").write(get(url))
    txt = os.path.join(out, "txt", f"{name}.txt")
    reader = PdfReader(pdf)
    if not os.path.exists(txt) or os.path.getmtime(txt) < os.path.getmtime(pdf):
        open(txt, "w").write("\f".join(p.extract_text() or "" for p in reader.pages))
    lines.append(f"- [{name}](pdf/{name}.pdf) ([text](txt/{name}.txt)), {len(reader.pages)} pages — {url}")
    print(name, len(reader.pages), "pages")
for name, page in sorted(PAGES.items()):
    txt = os.path.join(out, "txt", f"{name}.txt")
    if not os.path.exists(txt):
        text = html.unescape(re.sub(r"<[^>]+>", " ", get(BASE + page).decode("utf-8", "replace")))
        open(txt, "w").write(re.sub(r"\n\s*\n+", "\n\n", re.sub(r"[ \t]+", " ", text)))
    lines.append(f"- [{name}](txt/{name}.txt) (HTML as text) — {BASE}{page}")
    print(name, "page")
open(os.path.join(out, "README.md"), "w").write("\n".join(lines) + "\n")
