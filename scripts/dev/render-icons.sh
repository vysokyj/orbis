#!/usr/bin/env bash
# Render the plugin's Content Library thumbnails (91×91) from assets/icon/
# (needs rsvg-convert). The PNGs are committed: `orbis install` embeds them.
#   globe.svg → Scripts/ORBIS.png                    (the folder)
#   orbx.svg  → Scripts/ORBIS/Export to Octane.png   (the script: what it makes)
#   banner.svg → banner.png, banner@2x.png           (the options dialog's header)
case ${1:-} in -h|--help) sed -n '2,6p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;; esac
set -euo pipefail
cd "$(dirname "$0")/../.."
rsvg-convert -w 91 -h 91 assets/icon/globe.svg -o daz/Scripts/ORBIS.png
rsvg-convert -w 91 -h 91 assets/icon/orbx.svg -o "daz/Scripts/ORBIS/Export to Octane.png"
# banner.svg → the options dialog's header (1× and 2×), installed beside bin/
rsvg-convert -w 520 -h 84 assets/icon/banner.svg -o daz/banner.png
rsvg-convert -w 1040 -h 168 assets/icon/banner.svg -o daz/banner@2x.png
