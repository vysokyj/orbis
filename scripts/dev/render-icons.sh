#!/usr/bin/env bash
# Render the plugin's Content Library thumbnails from assets/icon/orbis.svg
# (needs rsvg-convert). The PNGs are committed: `orbis install` embeds them.
set -euo pipefail
cd "$(dirname "$0")/../.."
rsvg-convert -w 91 -h 91 assets/icon/orbis.svg -o "daz/Scripts/ORBIS/Export to Octane.png"
rsvg-convert -w 91 -h 91 assets/icon/orbis.svg -o daz/Scripts/ORBIS.png
