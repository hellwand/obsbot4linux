#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# check-model.sh: checks one device against its own description (design spec 9.4).
#
#   scripts/check-model.sh --device <id> [--asleep] [--destructive] [--assume-pass]
#                          [--results <file>]
#
# The rows come from `obsbot4linux describe --json`: every writable enum value, both values of
# each bool, the minimum and the maximum of each number (one row each) and the full span of
# each range (one row: `min.min,max.max`). A setting row passes when the state shows the
# written value within 70 s; the value is then restored, after every row, and the restore must
# show too. The row says how it was seen: `reply` (the state the reply published already shows
# it, the same rev) or `later` (a later state), with the lag from the reply. The last column
# says which kind a row is: `readback`, `no_effect (readback only)`, `readback:false (person
# in view)` or `action (person in view)`; the rows that need a person in view ask the person
# running the script, `--assume-pass` answers every question with pass (CI runs the script so
# against the fake devices). Rows whose `warning` holds `restart` or `destructive` run only
# with `--destructive`, last; they are not restored row by row (the camera restarts for each).
# A leaf with `requires` gets its required values set first, each as its own `set` the output
# names, and restored after its rows. A leaf whose snapshot value cannot be written back (an
# enum value outside `writable`, as one stored by the camera's own app, or `raw:<n>`) gets no
# row: its row says so and the leaf keeps the owner's value.
#
# Actions and asleep rows ask first. What an action stores on the camera is not restored:
# stored presets, views, power-on and return poses. The state before the first action row is
# printed, and the rows that work on a list item use a preset slot or a view the snapshot
# shows free or absent, saved by the script itself and deleted by it again; an action that
# needs such an item and finds none is not run. `--asleep` adds the asleep rows: sleep, write
# a value that differs from the snapshot (after the `requires` set while awake), wake, look
# for it. A `.turn` action runs asleep only with its `.stop`. The state is snapshot first; on
# exit, an interrupt or a termination every leaf the script wrote is set back to its snapshot
# value, the script's own presets and views are deleted, a turn is stopped and the pose is
# moved back. Output: one line per row and a results file (tab-separated).
#
# Needs bash, jq and the command line `obsbot4linux` (or the one named by $OBSBOT4LINUX); the
# daemon is reached as the command line reaches it ($OBSBOT4LINUX_SOCKET or the default).
# Exit: 0 every row that ran passed; 1 a row failed or a leaf could not be restored; 2 usage;
# 3 or 4 as the command line (3 also: the device is not available or asleep and not woken).

set -u

CLI=${OBSBOT4LINUX:-obsbot4linux}
# The slowest periodic tier (60 s) plus 10 s (9.4); the asleep waits are 30 s. Tests shorten them.
WAIT=${CHECK_MODEL_WAIT:-70}
SLEEP_WAIT=${CHECK_MODEL_SLEEP_WAIT:-30}
RETURN_WAIT=${CHECK_MODEL_RETURN_WAIT:-60}
POLL=${CHECK_MODEL_POLL:-0.2}
# Where the person's answers come from (tests: a file with one answer per line).
ANSWERS=${CHECK_MODEL_TTY:-/dev/tty}

DEVICE="" ASLEEP=0 DESTRUCTIVE=0 ASSUME=0 RESULTS=""

usage() {
	echo "usage: check-model.sh --device <id> [--asleep] [--destructive] [--assume-pass] [--results <file>]" >&2
	exit 2
}

while [ $# -gt 0 ]; do
	case $1 in
	--device)
		[ $# -ge 2 ] || usage
		DEVICE=$2
		shift 2
		;;
	--asleep)
		ASLEEP=1
		shift
		;;
	--destructive)
		DESTRUCTIVE=1
		shift
		;;
	--assume-pass)
		ASSUME=1
		shift
		;;
	--results)
		[ $# -ge 2 ] || usage
		RESULTS=$2
		shift 2
		;;
	-h | --help) usage ;;
	*)
		echo "check-model.sh: unknown argument: $1" >&2
		usage
		;;
	esac
done
[ -n "$DEVICE" ] || usage
command -v jq >/dev/null || {
	echo "check-model.sh: jq is needed" >&2
	exit 2
}
if [ "$ASSUME" != 1 ]; then
	exec 3<"$ANSWERS" || {
		echo "check-model.sh: cannot read the answers from $ANSWERS" >&2
		exit 2
	}
fi

# Text that came from the camera (a preset or view name, a firmware string) reaches the output
# only through this filter: JSON escapes the C0 controls, the filter replaces DEL and C1.
JQ_CLEAN='def clean: if type == "string" then gsub("[\u007f-\u009f]"; "?") else . end; def cleanall: walk(clean);'
# Numbers are rounded to nine decimals, so that a value's text does not depend on how a jq
# version prints a float; `mid` is the point three quarters up a range (on its step or
# precision, inside the range, not at a limit).
# shellcheck disable=SC2016 # jq variables, not shell ones
JQ_NUM='def num: if type == "number" then ((. * 1e9 | round) / 1e9) else . end;
    def mid($lo; $hi; $step; $prec):
        ($lo + ($hi - $lo) * 3 / 4) as $m
        | if $step != null then ((($m - $lo) / $step | floor) * $step + $lo)
          elif $prec != null then (($m / $prec | round) * $prec)
          else $m end;'

NOTE="not restored by this script: stored presets, views, power-on and return poses"

# One request through the command line; its JSON on stdout, its exit code returned.
cli() {
	local sub=$1
	shift
	"$CLI" "$sub" --json --device "$DEVICE" "$@"
}

now_ms() { date +%s%3N; }

# The code of a failed reply (one of the protocol's own codes), else `error`.
errcode() {
	local code
	code=$(jq -r '.error.code // empty' <<<"$1" 2>/dev/null | head -n 1 | tr -cd 'a-z_')
	echo "${code:-error}"
}

DESC=$(cli describe) || exit $?
ID=$(jq -r .id <<<"$DESC")
MODEL=$(jq -r .model <<<"$DESC")
SHORT=$("$CLI" devices --device "$DEVICE" | head -n 1 | cut -f 1 -d ' ')
FULL=$(cli status) || exit $?
SNAP=$(jq -c .state <<<"$FULL")

# The state the rows start from: the camera must be there and awake (the 0.13 script woke it).
if [ "$(jq -r .available <<<"$SNAP")" != true ]; then
	echo "check-model.sh: $SHORT is not available" >&2
	exit 3
fi
if [ "$(jq -r .asleep <<<"$SNAP")" = true ]; then
	jq -e '.actions.wake' <<<"$DESC" >/dev/null || {
		echo "check-model.sh: $SHORT is asleep and has no wake action" >&2
		exit 3
	}
fi

[ -n "$RESULTS" ] || RESULTS="check-model-$MODEL-$(date +%Y-%m-%d).tsv"
{
	echo "# check-model.sh $(date -u +%Y-%m-%dT%H:%M:%SZ)"
	echo "# device $SHORT model $MODEL firmware $(jq -c "$JQ_CLEAN"'.firmware // "not reported" | clean' <<<"$DESC") support $(jq -r .support <<<"$DESC")"
	printf '# kind\tpath\tvalue\tresult\tlag_ms\tseen\thow\n'
} >"$RESULTS"

FAILED=0
CHANGED=()
MOVED=0
TURN_STOPS=()
OWN_PRESET="" PRESET_SAVED=0 OWN_VIEW=""
FINISHING=0

# The value of a dotted path in a state.
leaf() { jq -c --arg p "$2" 'getpath($p | split("."))' <<<"$1"; }

# Whether a state shows `value` at `path`: the leaf equals it (numbers within half the
# precision, per component for a pose or a range) and the path is not pending.
shows() {
	jq -e --arg p "$2" --argjson v "$3" --argjson prec "$4" '
        def near($a; $b): if ($a | type) == "number" and ($b | type) == "number"
            then (($a - $b) | fabs) <= ($prec / 2 + 1e-9) else $a == $b end;
        def same($a; $b): if ($a | type) == "object" and ($b | type) == "object"
            then [$b | keys[] as $k | near($a[$k]; $b[$k])] | all else near($a; $b) end;
        (.pending | has($p) | not) and same(getpath($p | split("."))?; $v)
    ' <<<"$1" >/dev/null 2>&1
}

# The command-line text of a JSON value (spec 6.2 value syntax). Numbers are rounded to nine
# decimals, so that the text does not depend on how a jq version prints a float.
text() {
	jq -r "$JQ_NUM"'if type == "object" then
               (if has("pan") then "\(.pan | num),\(.tilt | num),\(.zoom | num)"
                else "\(.min | num),\(.max | num)" end)
           elif type == "boolean" then (if . then "on" else "off" end)
           else num | tostring end' <<<"$1"
}

full_state() {
	local out
	out=$(cli status) || return 1
	jq -c .state <<<"$out"
}

# One line of output and one of the results file: kind path value result [lag_ms seen how].
# Control characters of any field are replaced, so that a field cannot make another line.
record() {
	local i f out=()
	for i in 1 2 3 4 5 6 7; do
		f=${!i:-}
		out+=("${f//[[:cntrl:]]/ }")
	done
	printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "${out[@]}" >>"$RESULTS"
	printf '%-8s %-34s %-18s %s%s%s\n' "${out[0]}" "${out[1]}" "${out[2]}" "${out[3]}" \
		"${out[4]:+  lag ${out[4]} ms (${out[5]})}" "${out[6]:+  [${out[6]}]}"
	case $4 in fail*) FAILED=1 ;; esac
}

# Asks the person; with --assume-pass the answer is pass (the question is still printed).
ask() {
	if [ "$ASSUME" = 1 ]; then
		echo "  ask: $1 -> pass (--assume-pass)"
		return 0
	fi
	local answer
	printf '%s [y/n] ' "$1" >&2
	read -r answer <&3
	[ "$answer" = y ]
}

# Waits up to $1 seconds for the state to show $3 at $2 (precision $4).
wait_for() {
	local until=$(($(date +%s) + $1)) st
	while :; do
		st=$(full_state) || return 1
		if shows "$st" "$2" "$3" "$4"; then return 0; fi
		[ "$(date +%s)" -lt "$until" ] || return 1
		sleep "$POLL"
	done
}

# Waits up to $1 seconds for the leaf `asleep` to be $2.
wait_asleep() {
	local until=$(($(date +%s) + $1))
	while :; do
		[ "$(leaf "$(full_state)" asleep)" = "$2" ] && return 0
		[ "$(date +%s)" -lt "$until" ] || return 1
		sleep "$POLL"
	done
}

# The camera must be awake for the rows; a sleeping one is woken when the person agrees.
if [ "$(jq -r .asleep <<<"$SNAP")" = true ]; then
	ask "$SHORT is asleep: wake it for the check?" || exit 3
	cli "do" wake >/dev/null || exit 3
	wait_asleep "$SLEEP_WAIT" false || {
		echo "check-model.sh: $SHORT is still asleep after ${SLEEP_WAIT} s" >&2
		exit 3
	}
	SNAP=$(full_state) || exit 3
fi

# The setting rows of one leaf: the values to write, in order.
values_of() {
	jq -c --arg p "$1" --argjson snap "$(leaf "$SNAP" "$1")" '.settings[$p] as $s |
        if $s.type == "bool" then (if $snap == true then [false, true] else [true, false] end)
        elif $s.type == "enum" then $s.writable
        elif $s.type == "number" then [$s.min, $s.max] | map(select(. != null))
        elif $s.type == "range" then [{min: $s.min.min, max: $s.max.max}]
        else [] end | .[]' <<<"$DESC"
}

precision_of() {
	jq -r --arg p "$1" '.settings[$p].precision // 0 |
        if type == "object" then ([.[]] | max) else . end' <<<"$DESC"
}

# Which kind a setting row is (spec 9.4).
how_of() {
	jq -r --arg p "$1" '.settings[$p] |
        if .readback == false then "readback:false (person in view)"
        elif .no_effect then "no_effect (readback only)"
        else "readback" end' <<<"$DESC"
}

readback_of() { jq -r --arg p "$1" '.settings[$p].readback' <<<"$DESC"; }

# Puts one leaf back to its snapshot value after a row; a leaf that reads back must show it.
# A restore that fails is a warning row here; the trap tries again and names what is left.
restore_row() {
	local path=$1 snap reply
	snap=$(leaf "$SNAP" "$path")
	[ "$snap" = null ] && return 0
	if ! reply=$(cli set "$path=$(text "$snap")" 2>&1); then
		restore_warning "$path" "$snap" "restore refused ($(errcode "$reply"))"
		return 1
	fi
	[ "$(readback_of "$path")" = true ] || return 0
	wait_for "$WAIT" "$path" "$snap" "$(precision_of "$path")" || {
		restore_warning "$path" "$snap" "restore not shown within ${WAIT} s"
		return 1
	}
}

restore_warning() {
	if [ "$FINISHING" = 1 ]; then
		echo "  warn: $1 $2: $3"
	else
		record restore "$1" "$2" "warn: $3"
	fi
}

# The setup a leaf needs: its `requires` (spec 4.2), as `path=value` arguments.
setup_of() {
	jq -r --arg p "$1" '.settings[$p].requires // [] | .[] |
        "\(.path)=\(.equals | if type == "boolean" then (if . then "on" else "off" end) else tostring end)"' <<<"$DESC"
}

# Writes the setup of a leaf, each required value as its own `set` the output names; the row
# of kind $2 fails when one is refused.
setup_leaf() {
	local path=$1 kind=$2 req reqs
	mapfile -t reqs < <(setup_of "$path")
	for req in "${reqs[@]}"; do
		[ -n "$req" ] || continue
		echo "  setup for $path: set $req"
		CHANGED+=("${req%%=*}")
		cli set "$req" >/dev/null 2>&1 || {
			record "$kind" "$path" - "fail: setup $req refused"
			return 1
		}
	done
}

restore_setup() {
	local req reqs
	mapfile -t reqs < <(setup_of "$1")
	for req in "${reqs[@]}"; do
		[ -n "$req" ] && restore_row "${req%%=*}"
	done
	return 0
}

# One setting row: write, wait for the value, restore. `reply` when the state of the same rev
# as the reply's already shows the value; `later` otherwise, with the time from the reply.
setting_row() {
	local path=$1 value=$2 prec how reply replied rrev out srev st seen=later lag=0
	prec=$(precision_of "$path")
	how=$(how_of "$path")
	CHANGED+=("$path")
	if ! reply=$(cli set "$path=$(text "$value")" 2>&1); then
		record setting "$path" "$value" "fail: $(errcode "$reply")" "" "" "$how"
		return
	fi
	replied=$(now_ms)
	if [ "$(readback_of "$path")" = false ]; then
		if ask "$(jq -r --arg p "$path" '.settings[$p].label' <<<"$DESC") = $(text "$value"): is it so on the camera? (person in view)"; then
			record setting "$path" "$value" "pass (confirmed)" "" "" "$how"
		else
			record setting "$path" "$value" "fail (not confirmed)" "" "" "$how"
		fi
		restore_row "$path"
		return
	fi
	rrev=$(jq -r '.rev // empty' <<<"$reply" 2>/dev/null)
	out=$(cli status 2>/dev/null) || out=""
	st=$(jq -c .state <<<"$out" 2>/dev/null) || st=""
	srev=$(jq -r '.rev // empty' <<<"$out" 2>/dev/null)
	if [ -n "$st" ] && shows "$st" "$path" "$value" "$prec"; then
		if [ -n "$rrev" ] && [ "$srev" = "$rrev" ]; then seen=reply; fi
		lag=$(($(now_ms) - replied))
	elif wait_for "$WAIT" "$path" "$value" "$prec"; then
		lag=$(($(now_ms) - replied))
	else
		record setting "$path" "$value" "fail: not shown within ${WAIT} s" "" "" "$how"
		restore_row "$path"
		return
	fi
	[ "$seen" = reply ] && lag=0
	record setting "$path" "$value" pass "$lag" "$seen" "$how"
	restore_row "$path"
}

# Whether the snapshot value of a leaf is one the script cannot write back: an enum value
# outside `writable` (a start mode that the camera's own app stored) or `raw:<n>`. Such a leaf
# gets no row, so the owner's value is not lost.
cannot_restore() {
	jq -e --arg p "$1" --argjson v "$(leaf "$SNAP" "$1")" '.settings[$p] as $s
        | ($v | type == "string" and startswith("raw:"))
          or ($s.type == "enum" and $v != null and ($s.writable | any(. == $v) | not))' <<<"$DESC" >/dev/null
}

not_restorable_row() {
	record "$1" "$2" "$(leaf "$SNAP" "$2")" "not run: the value it reads cannot be written back" "" "" "$(how_of "$2")"
}

check_leaf() {
	local path=$1 value
	if cannot_restore "$path"; then
		not_restorable_row setting "$path"
		return
	fi
	setup_leaf "$path" setting || {
		restore_setup "$path"
		return
	}
	while IFS= read -r value; do
		[ -n "$value" ] && setting_row "$path" "$value"
	done < <(values_of "$path")
	restore_setup "$path"
}

# The item an action row works on is one the script saved itself: a preset slot that the
# snapshot shows free (`preset.save` takes it, the rows after it use it, `preset.delete` ends
# it) and the view the script saved under the name `check`. An item of the owner is never
# renamed, recalled, used or deleted. A snapshot that shows no list at all leaves no slot.
OWN_PRESET=$(jq -r --argjson snap "$SNAP" '
    (.actions["preset.save"].params.slot // null) as $s
    | if $s == null or $s.min == null or $s.max == null or ($snap.presets | type) != "array" then empty
      else ([range($s.min; $s.max + 1; ($s.step // 1))] - [$snap.presets[].slot]) | first // empty end' <<<"$DESC")

# Looks for the script's own items in a state.
resync_own() {
	local st=$1
	PRESET_SAVED=0
	if [ -n "$OWN_PRESET" ] && jq -e --argjson s "$OWN_PRESET" '[.presets[]?.slot] | index($s) != null' <<<"$st" >/dev/null 2>&1; then
		PRESET_SAVED=1
	fi
	if [ -n "$OWN_VIEW" ] && ! jq -e --argjson i "$OWN_VIEW" '[.views[]?.id] | index($i) != null' <<<"$st" >/dev/null 2>&1; then
		OWN_VIEW=""
	fi
	if [ -z "$OWN_VIEW" ]; then
		OWN_VIEW=$(jq -rn --argjson a "$SNAP" --argjson b "$st" '
            if ($a.views | type) != "array" then empty
            else ([$a.views[].id]) as $old
                | [$b.views[]? | select(.name == "check") | select(.id as $i | $old | index($i) | not) | .id] | first // empty end' 2>/dev/null) || OWN_VIEW=""
	fi
}

# The parameters of an action's row: every required parameter, or the first parameter that
# is not a bool when none is required (a move needs one axis). The value is the parameter's
# default, else the point three quarters up its bounds (on its step or precision: inside the
# range, not at a limit, not zero for a symmetric one), its first writable value, `true` for a
# bool (`confirm=true` reaches only the rows of --destructive) or the text `check`. A preset
# slot or a view id is the script's own item, or `__none__` when there is none yet.
params_of() {
	local saved=${2:-$PRESET_SAVED} view=${3:-$OWN_VIEW} live_preset=""
	[ "$saved" = 1 ] && live_preset=$OWN_PRESET
	jq -r --arg a "$1" --arg own_preset "$OWN_PRESET" --arg live_preset "$live_preset" --arg own_view "$view" "$JQ_NUM"'
        def own($key):
            if $key == "slot" and ($a | startswith("preset.") or . == "power_on.use_preset") then
                (if $a == "preset.save" then $own_preset else $live_preset end)
            elif $key == "id" and ($a | startswith("view.")) then $own_view
            else null end;
        def value($key; $s):
            if own($key) != null then (own($key) | if . == "" then "__none__" else . end)
            elif $s.default != null then ($s.default | num | tostring)
            elif $s.type == "bool" then "true"
            elif $s.type == "enum" then $s.writable[0]
            elif $s.type == "string" then "check"
            elif $s.type == "pose" then
                [mid($s.min.pan; $s.max.pan; null; $s.precision.pan),
                 mid($s.min.tilt; $s.max.tilt; null; $s.precision.tilt),
                 mid($s.min.zoom; $s.max.zoom; null; $s.precision.zoom)] | map(num | tostring) | join(",")
            elif $s.min != null and $s.max != null then (mid($s.min; $s.max; $s.step; $s.precision) | num | tostring)
            elif $s.min != null then ($s.min | num | tostring)
            else "0" end;
        (.actions[$a].params // {} | to_entries) as $all
        | ([$all[] | select(.value.optional | not)]
           | if length == 0 then [$all[] | select(.value.type != "bool")][:1] else . end)[]
        | .key + "=" + value(.key; .value)' <<<"$DESC"
}

# The leaves whose values differ between two states (camera text is cleaned).
diff_states() {
	jq -rn --argjson a "$1" --argjson b "$2" "$JQ_CLEAN"'
        [$a, $b | del(.pending, .unread) | paths(scalars)] | unique[] as $p
        | select(($a | getpath($p)) != ($b | getpath($p)))
        | "    \($p | map(tostring) | join(".")): \($a | getpath($p) | clean | tojson) -> \($b | getpath($p) | clean | tojson)"'
}

# The stop of a continuing action: `<group>.stop` for a `.turn` action that has one.
stop_of() {
	case $1 in
	*.turn)
		local stop="${1%.*}.stop"
		jq -e --arg a "$stop" '.actions[$a]' <<<"$DESC" >/dev/null && echo "$stop"
		;;
	esac
	return 0
}

action_row() {
	local action=$1 before after reply params code stop
	local how="action (person in view)"
	mapfile -t params < <(params_of "$action")
	if [[ " ${params[*]} " == *=__none__* ]]; then
		record action "$action" - "not run: no item of its own (none is free or saved)" "" "" "$how"
		return
	fi
	stop=$(stop_of "$action")
	if ! ask "run do $action ${params[*]}? ($NOTE; person in view)"; then
		record action "$action" "${params[*]}" "not run (declined)" "" "" "$how"
		return
	fi
	MOVED=1
	[ -n "$stop" ] && TURN_STOPS+=("$stop")
	before=$(full_state)
	if ! reply=$(cli "do" "$action" "${params[@]}" 2>&1); then
		code=$(errcode "$reply")
		case $code in
		conflict | asleep) record action "$action" "${params[*]}" "not run: $code" "" "" "$how" ;;
		*) record action "$action" "${params[*]}" "fail: $code" "" "" "$how" ;;
		esac
		return
	fi
	sleep "$POLL"
	after=$(full_state)
	resync_own "$after"
	echo "  state change after do $action:"
	diff_states "$before" "$after"
	if ask "does the camera show the effect of $action (ledger: effect column)?"; then
		record action "$action" "${params[*]}" pass "" "" "$how"
	else
		record action "$action" "${params[*]}" "fail (not confirmed)" "" "" "$how"
	fi
}

# A row whose write or action restarts the camera: it passes when the device returns under
# the same id within 60 s with the expected state.
restart_row() {
	local kind=$1 name=$2 value=$3 reply until how
	if [ "$kind" = setting ]; then
		how=$(how_of "$name")
		CHANGED+=("$name")
		reply=$(cli set "$name=$(text "$value")" 2>&1) || {
			record "$kind" "$name" "$value" "fail: $(errcode "$reply")" "" "" "$how"
			return
		}
	else
		local params
		how="action (person in view)"
		mapfile -t params < <(params_of "$name")
		value="${params[*]}"
		ask "run do $name $value? the camera restarts (person in view)" || {
			record "$kind" "$name" "$value" "not run (declined)" "" "" "$how"
			return
		}
		reply=$(cli "do" "$name" "${params[@]}" 2>&1) || {
			record "$kind" "$name" "$value" "fail: $(errcode "$reply")" "" "" "$how"
			return
		}
	fi
	until=$(($(date +%s) + RETURN_WAIT))
	while :; do
		if [ "$("$CLI" --json devices 2>/dev/null | jq -r --arg id "$ID" '.devices[] | select(.id == $id) | .available')" = true ]; then
			if [ "$kind" = action ] || wait_for "$WAIT" "$name" "$value" "$(precision_of "$name")"; then
				record "$kind" "$name" "$value" pass "" returned "$how"
				return
			fi
		fi
		[ "$(date +%s)" -lt "$until" ] || break
		sleep "$POLL"
	done
	record "$kind" "$name" "$value" "fail: not back under $SHORT within ${RETURN_WAIT} s" "" "" "$how"
}

# The first value of a leaf's rows that differs from the snapshot; a write of the value the
# camera already shows would read `accepted` whether or not the camera took it.
differing_value() {
	local path=$1 prec value
	prec=$(precision_of "$path")
	while IFS= read -r value; do
		[ -n "$value" ] || continue
		if ! shows "$SNAP" "$path" "$value" "$prec"; then
			printf '%s\n' "$value"
			return 0
		fi
	done < <(
		values_of "$path"
		mid_of "$path"
	)
	return 1
}

# For a number that has both bounds: the point three quarters up (the third value to try when
# the snapshot already shows the minimum or the maximum).
mid_of() {
	jq -c --arg p "$1" "$JQ_NUM"'.settings[$p] | select(.type == "number" and .min != null and .max != null)
        | mid(.min; .max; .step; .precision) | num' <<<"$DESC"
}

# Saves the item an asleep row of an action needs, awake, when the script has none yet.
ensure_own() {
	case $1 in
	preset.rename | preset.delete | preset.recall | power_on.use_preset)
		[ "$PRESET_SAVED" = 1 ] && return 0
		[ -n "$OWN_PRESET" ] || return 1
		jq -e '.actions["preset.save"]' <<<"$DESC" >/dev/null || return 1
		echo "  setup for $1: do preset.save slot=$OWN_PRESET"
		cli "do" preset.save "slot=$OWN_PRESET" >/dev/null 2>&1 || return 1
		resync_own "$(full_state)"
		[ "$PRESET_SAVED" = 1 ]
		;;
	view.rename | view.delete | view.go)
		[ -n "$OWN_VIEW" ] && return 0
		jq -e '.actions["view.save"]' <<<"$DESC" >/dev/null || return 1
		echo "  setup for $1: do view.save name=check"
		cli "do" view.save name=check >/dev/null 2>&1 || return 1
		resync_own "$(full_state)"
		[ -n "$OWN_VIEW" ]
		;;
	esac
	return 0
}

# An asleep row: sleep, write, wake, look for the value (spec 9.4 `--asleep`).
asleep_row() {
	local kind=$1 name=$2 value="" reply code stop="" how params
	if [ "$kind" = setting ]; then
		how=$(how_of "$name")
		if cannot_restore "$name"; then
			not_restorable_row asleep "$name"
			return
		fi
		value=$(differing_value "$name") || {
			record asleep "$name" - "not run: no value that differs from the snapshot" "" "" "$how"
			return
		}
		setup_leaf "$name" asleep || {
			restore_setup "$name"
			return
		}
	else
		how="action (person in view)"
		# what the row would work on, before anything is saved: the question comes first
		value=$(params_of "$name" 1 "${OWN_VIEW:-<its own view>}" | tr '\n' ' ')
		case $name in
		*.turn)
			stop=$(stop_of "$name")
			[ -n "$stop" ] || {
				record asleep "$name" "$value" "not run: no stop action to end it" "" "" "$how"
				return
			}
			;;
		esac
		if [[ " $value " == *=__none__* ]]; then
			record asleep "$name" - "not run: no item of its own (none is free or saved)" "" "" "$how"
			return
		fi
		ask "asleep row: sleep, do $name $value${stop:+, then $stop}, wake. $NOTE; person in view. Run it?" || {
			record asleep "$name" "$value" "not run (declined)" "" "" "$how"
			return
		}
		ensure_own "$name" || {
			record asleep "$name" "$value" "not run: no item of its own (none is free or saved)" "" "" "$how"
			return
		}
		mapfile -t params < <(params_of "$name")
		value="${params[*]}"
		MOVED=1
		[ -n "$stop" ] && TURN_STOPS+=("$stop")
	fi
	if ! { cli "do" sleep >/dev/null && wait_asleep "$SLEEP_WAIT" true; }; then
		record asleep "$name" "$value" "fail: no asleep:true within ${SLEEP_WAIT} s" "" "" "$how"
		return
	fi
	if [ "$kind" = setting ]; then
		CHANGED+=("$name")
		reply=$(cli set "$name=$(text "$value")" 2>&1)
	else
		reply=$(cli "do" "$name" "${params[@]}" 2>&1)
		[ -n "$stop" ] && cli "do" "$stop" >/dev/null 2>&1
	fi
	code=$(jq -r '.error.code // "ok"' <<<"$reply" 2>/dev/null)
	if ! { cli "do" wake >/dev/null && wait_asleep "$SLEEP_WAIT" false; }; then
		record asleep "$name" "$value" "fail: no asleep:false within ${SLEEP_WAIT} s" "" "" "$how"
		return
	fi
	# A turn goes on after the wake (the command line sends keep:true): stop it again.
	[ -n "$stop" ] && cli "do" "$stop" >/dev/null 2>&1
	if [ "$code" != ok ]; then
		record asleep "$name" "$value" "ignored (reply $(errcode "$reply"))" "" "" "$how"
	elif [ "$kind" = setting ] && [ "$(readback_of "$name")" = true ]; then
		if wait_for "$WAIT" "$name" "$value" "$(precision_of "$name")"; then
			record asleep "$name" "$value" accepted "" "" "$how"
		else
			record asleep "$name" "$value" "ignored (not shown after wake)" "" "" "$how"
		fi
	elif ask "after the wake: does the camera show $name $value? (person in view)"; then
		record asleep "$name" "$value" accepted "" "" "$how"
	else
		record asleep "$name" "$value" "ignored (not confirmed)" "" "" "$how"
	fi
	if [ "$kind" = setting ]; then
		restore_row "$name"
		restore_setup "$name"
	else
		resync_own "$(full_state)"
	fi
}

# The script's own presets and views are deleted again (a list item is not a leaf).
# shellcheck disable=SC2329 # run by the trap
cleanup_own() {
	local st=$1
	if [ -n "$OWN_PRESET" ] && jq -e --argjson s "$OWN_PRESET" '[.presets[]?.slot] | index($s) != null' <<<"$st" >/dev/null 2>&1; then
		cli "do" preset.delete "slot=$OWN_PRESET" >/dev/null 2>&1
	fi
	if [ -n "$OWN_VIEW" ]; then
		cli "do" view.delete "id=$OWN_VIEW" >/dev/null 2>&1
	fi
}

# On exit, an interrupt or a termination: every leaf written goes back to its snapshot value,
# the last one written first; the script's own presets and views are deleted and a turn is
# stopped (spec 9.4). The pose of the snapshot goes back first, its zoom written back whatever
# the state shows (a move's zoom shows only at its group's next read), so that the leaves a pose
# change can switch (the field of view, the tracking mode) are judged after it; asleep, the
# pose is skipped and the output says so. A leaf that does not show its snapshot value
# afterwards is named and makes the exit code 1.
# shellcheck disable=SC2329 # run by the trap
finish() {
	local rc=$? i path st stop bad=0
	declare -A done_
	trap - EXIT INT TERM
	FINISHING=1
	WAIT=${CHECK_MODEL_FINISH_WAIT:-10}
	# after a restart row the device may still be settling: wait for it
	local until=$(($(date +%s) + WAIT))
	while :; do
		st=$(full_state 2>/dev/null) || st=""
		[ "$(leaf "${st:-null}" available 2>/dev/null)" = true ] && break
		[ "$(date +%s)" -lt "$until" ] || break
		sleep "$POLL"
	done
	[ -n "$st" ] || st=$SNAP
	for stop in "${TURN_STOPS[@]}"; do
		cli "do" "$stop" >/dev/null 2>&1
	done
	# The pose first: on the camera a zoom write can switch the field of view and a gimbal move
	# the tracking mode, which the loop below then puts back and judges on a fresh state.
	if [ "$MOVED" = 1 ] && [ "$(leaf "$st" asleep 2>/dev/null)" = true ]; then
		echo "  skipped: the pose of the snapshot (the camera is asleep)"
	elif [ "$MOVED" = 1 ]; then
		if jq -e '.actions["gimbal.move"]' <<<"$DESC" >/dev/null; then
			local pan tilt
			pan=$(leaf "$SNAP" gimbal.pan)
			tilt=$(leaf "$SNAP" gimbal.tilt)
			if [ "$pan" != null ] && [ "$tilt" != null ]; then
				cli "do" gimbal.move "pan=$pan" "tilt=$tilt" >/dev/null 2>&1
			fi
		fi
		# The zoom: a move (`zoom.move`, `preset.recall`, `view.go`, `zone.go_return`,
		# `hand.go_return`, `power_on.go`, `wake`) changes it without the state having to show
		# it yet (its group is read on its own tier, not after the move), so the state cannot
		# tell whether it is back. It is written back whatever it shows; the write reads it again.
		if jq -e '.settings["zoom.factor"] | . != null and (.readonly | not)' <<<"$DESC" >/dev/null; then
			restore_row zoom.factor 2>/dev/null
		fi
		st=$(full_state 2>/dev/null) || st=$SNAP
	fi
	for ((i = ${#CHANGED[@]} - 1; i >= 0; i--)); do
		path=${CHANGED[i]}
		shows "$st" "$path" "$(leaf "$SNAP" "$path")" "$(precision_of "$path")" 2>/dev/null && continue
		restore_row "$path" 2>/dev/null && st=$(full_state 2>/dev/null || echo "$st")
	done
	resync_own "$st"
	cleanup_own "$st"
	st=$(full_state 2>/dev/null) || st=""
	for path in "${CHANGED[@]}"; do
		[ -n "${done_[$path]:-}" ] && continue
		done_[$path]=1
		[ "$(leaf "$SNAP" "$path")" = null ] && continue
		[ "$(readback_of "$path")" = true ] || continue
		if ! shows "$st" "$path" "$(leaf "$SNAP" "$path")" "$(precision_of "$path")"; then
			echo "not restored: $path (snapshot $(leaf "$SNAP" "$path"))"
			bad=1
		fi
	done
	echo "results: $RESULTS"
	[ "$bad" = 0 ] || [ "$rc" != 0 ] || rc=1
	exit "$rc"
}
trap finish EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

# What may restart the camera or destroy something.
risky() { jq -e --arg k "$1" --arg n "$2" '.[$k][$n].warning // [] | any(. == "restart" or . == "destructive")' <<<"$DESC" >/dev/null; }

mapfile -t LEAVES < <(jq -r '.settings | to_entries[] | select(.value.readonly | not) | .key' <<<"$DESC")
# Actions: those that save first, so that a row that needs a list item finds the one the
# script saved; then group by group (the path before the last dot), a stop after the moves of
# its group and a delete last; `sleep` and `wake` run on their own after them.
mapfile -t ACTIONS < <(jq -r '.actions | keys | map(select(. != "sleep" and . != "wake"))
    | sort_by((if endswith(".save") then 0 elif endswith(".stop") then 2
               elif endswith(".delete") then 3 else 1 end) as $c
              | [(if $c == 0 then 0 else 1 end), (split(".") | .[:-1] | join(".")), $c])[]' <<<"$DESC")

for path in "${LEAVES[@]}"; do
	if risky settings "$path"; then continue; fi
	check_leaf "$path"
done

# What the action rows may change and the script does not restore, as it was before.
echo "stored on the camera before the action rows ($NOTE):"
jq -c "$JQ_CLEAN"'{presets, views, power_on: .power_on.start, hand_return: .hand.return, zone_return: .zone.return} | cleanall' <<<"$SNAP"
[ -n "$OWN_PRESET" ] && echo "  the script's own preset slot: $OWN_PRESET (free in the snapshot)"

for action in "${ACTIONS[@]}"; do
	if risky actions "$action"; then continue; fi
	action_row "$action"
done
# `sleep`, then `wake`; the rows after them need the camera awake.
for action in sleep wake; do
	if jq -e --arg a "$action" '.actions[$a]' <<<"$DESC" >/dev/null; then
		action_row "$action"
	fi
done
wait_asleep "$SLEEP_WAIT" false || record action wake - "fail: still asleep after ${SLEEP_WAIT} s"

if [ "$ASLEEP" = 1 ]; then
	for path in "${LEAVES[@]}"; do
		risky settings "$path" && continue
		[ "$(jq -r --arg p "$path" '.settings[$p].asleep' <<<"$DESC")" = unknown ] && asleep_row setting "$path"
	done
	for action in "${ACTIONS[@]}"; do
		risky actions "$action" && continue
		[ "$(jq -r --arg a "$action" '.actions[$a].asleep' <<<"$DESC")" = unknown ] && asleep_row action "$action"
	done
fi

for path in "${LEAVES[@]}"; do
	risky settings "$path" || continue
	[ "$DESTRUCTIVE" = 1 ] || {
		record setting "$path" - "not run (--destructive)" "" "" "$(how_of "$path")"
		continue
	}
done
for action in "${ACTIONS[@]}"; do
	risky actions "$action" || continue
	[ "$DESTRUCTIVE" = 1 ] || {
		record action "$action" - "not run (--destructive)" "" "" "action (person in view)"
		continue
	}
done
if [ "$DESTRUCTIVE" = 1 ]; then
	echo "presets and views before the restart and destructive rows:"
	full_state 2>/dev/null | jq -c "$JQ_CLEAN"'{presets, views} | cleanall' 2>/dev/null || true
	for path in "${LEAVES[@]}"; do
		risky settings "$path" || continue
		while IFS= read -r value; do
			[ -n "$value" ] && restart_row setting "$path" "$value"
		done < <(values_of "$path")
	done
	for action in "${ACTIONS[@]}"; do
		risky actions "$action" && restart_row action "$action" -
	done
fi

exit "$FAILED"
