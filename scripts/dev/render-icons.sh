#!/usr/bin/env bash
# Render the plugin's Content Library thumbnails (91×91) from assets/icon/
# (needs rsvg-convert). The PNGs are committed: `orbis install` embeds them.
#   globe.svg → Scripts/ORBIS.png                    (the folder)
#   orbx.svg  → Scripts/ORBIS/Export to Octane.png   (the script: what it makes)
set -euo pipefail
cd "$(dirname "$0")/../.."
rsvg-convert -w 91 -h 91 assets/icon/globe.svg -o daz/Scripts/ORBIS.png
rsvg-convert -w 91 -h 91 assets/icon/orbx.svg -o "daz/Scripts/ORBIS/Export to Octane.png"
