#!/usr/bin/env bash
# Start a real tau server on a throwaway home, pair one device, print its token.
#
# usage: run-server.sh [PORT] [DEVICE]
#   PORT    TCP port on 127.0.0.1; 0 (default) lets the OS choose.
#   DEVICE  device name to pair (default: laptop).
#
# env:
#   TAU_BIN         the tau executable (default: built from $TAU_FLAKE#tau)
#   TAU_FLAKE       flake ref for the server (default github:vicrdguez/tau/render-core)
#   TAU_EXTENSIONS  space-separated extensions to install, e.g. "w1 w2 websearch", each a name
#                   (built from $TAU_FLAKE#extension-<name>) or name=path (an extension package
#                   already built); default: none
#   TAU_STATE       state dir to use (default: a fresh mktemp -d). Keep its path short:
#                   $TAU_STATE/.tau/admin.sock must fit in 104 bytes on macOS, else
#                   tau dies with "listen EINVAL ... admin.sock".
#   TAU_FAUX=1      run tau-faux.mjs (same server, model `faux/echo` answers
#                   "echo: <text>") instead of the stock `tau`, which has no fake model
#
# Prints shell assignments on stdout (eval-able):
#   TAU_STATE=... TAU_PID=... TAU_PORT=... TAU_URL=... TAU_DEVICE=... TAU_TOKEN=...
# The server keeps running. Stop it with:  kill -TERM "$TAU_PID"
# Admin commands against it:               HOME="$TAU_STATE" "$TAU_BIN" device list
set -euo pipefail

port=${1:-0}
device=${2:-laptop}
flake=${TAU_FLAKE:-github:vicrdguez/tau/render-core}
state=${TAU_STATE:-$(mktemp -d "${TMPDIR:-/tmp}/tau-e2e.XXXXXX")}
state=$(cd "$state" && pwd -P)

tau=${TAU_BIN:-$(nix build --no-link --print-out-paths "$flake#tau")/bin/tau}
tau_out=$(dirname "$(dirname "$tau")")

# tau has no --home flag: it uses $HOME/.tau (os.homedir()). Point HOME at the temp dir.
mkdir -p "$state/.tau/extensions" "$state/pi-agent"
for spec in ${TAU_EXTENSIONS:-}; do
	case $spec in
	*=*) name=${spec%%=*} ext=${spec#*=} ;;
	*) name=$spec ext=$(nix build --no-link --print-out-paths "$flake#extension-$name") ;;
	esac
	ln -sfn "$ext" "$state/.tau/extensions/$name"
done

# Keep the server away from the owner's pi logins and provider keys:
# credentials come from $PI_CODING_AGENT_DIR/auth.json and *_API_KEY env vars.
log="$state/server.log"
server=("$tau")
if [ "${TAU_FAUX:-}" = 1 ]; then
	# Resolve bare imports (tau/server, @earendil-works/*) from the package's node_modules.
	mkdir -p "$state/faux"
	cp "$(dirname "$0")/tau-faux.mjs" "$state/faux/"
	ln -sfn "$tau_out/lib/tau/node_modules" "$state/faux/node_modules"
	node=$(sed -n 's/^exec "\([^"]*\)".*/\1/p' "$tau")
	server=("$node" "$state/faux/tau-faux.mjs")
fi
env -i PATH="$PATH" HOME="$state" PI_CODING_AGENT_DIR="$state/pi-agent" TMPDIR="${TMPDIR:-/tmp}" \
	${TAU_FAUX_SCRIPT:+TAU_FAUX_SCRIPT="$TAU_FAUX_SCRIPT"} \
	"${server[@]}" --port "$port" >"$log" 2>&1 &
pid=$!

# Readiness: the server prints "tau: listening on http://127.0.0.1:<port>" after the
# admin socket is open and the HTTP listener is bound.
for _ in $(seq 1 300); do
	if line=$(grep -m1 '^tau: listening on ' "$log" 2>/dev/null); then break; fi
	if ! kill -0 "$pid" 2>/dev/null; then
		echo "tau exited early:" >&2
		cat "$log" >&2
		exit 1
	fi
	sleep 0.05
done
[ -n "${line:-}" ] || { echo "tau did not start in 15 s" >&2; kill "$pid"; exit 1; }
url=${line#tau: listening on }
port=${url##*:}

code=$(HOME="$state" "$tau" device pair "$device" | sed -n 's/^Pairing code: //p')
reply=$(curl -sS -X POST "$url/v1/pair" -H 'Content-Type: application/json' -d "{\"code\":\"$code\"}")
token=$(printf '%s' "$reply" | sed -n 's/.*"token":"\([^"]*\)".*/\1/p')
[ -n "$token" ] || { echo "pairing failed: $reply" >&2; kill "$pid"; exit 1; }

printf 'TAU_BIN=%q\nTAU_STATE=%q\nTAU_PID=%s\nTAU_PORT=%s\nTAU_URL=%q\nTAU_DEVICE=%q\nTAU_TOKEN=%q\n' \
	"$tau" "$state" "$pid" "$port" "$url" "$device" "$token"
