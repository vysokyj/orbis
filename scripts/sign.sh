#!/bin/sh
# Sign the orbis binary: with a Developer ID Application identity from the
# keychain (hardened runtime) when there is one, else ad hoc.
#   scripts/sign.sh [--release] <binary>
# --release adds Apple's secure timestamp (needs the network; notarization
# requires it). ORBIS_SIGN_IDENTITY picks an identity (name or SHA-1) instead
# of the first Developer ID Application one; "-" forces ad hoc. The first
# signature with a keychain key asks once to allow codesign ("Always Allow").
case ${1:-} in -h|--help) sed -n '2,8p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;; esac
set -eu
timestamp=--timestamp=none
if [ "${1:-}" = --release ]; then timestamp=--timestamp; shift; fi
bin=$1
identity=${ORBIS_SIGN_IDENTITY:-$(security find-identity -v -p codesigning 2>/dev/null | sed -n 's/^ *[0-9]*) \([0-9A-F]\{40\}\) "Developer ID Application: .*/\1/p' | head -n 1)}
if [ -z "$identity" ] || [ "$identity" = - ]; then
    codesign --force --sign - "$bin"
    echo "signed ad hoc: $bin" >&2
else
    codesign --force --sign "$identity" --options runtime "$timestamp" "$bin"
    echo "signed with Developer ID: $bin" >&2
fi
