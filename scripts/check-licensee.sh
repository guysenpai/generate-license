#!/usr/bin/env bash
# Checks that the files written by generate-license are recognized by
# licensee, the license detector GitHub uses. Needs licensee and ruby.
#
# usage: scripts/check-licensee.sh [path/to/generate-license]
set -euo pipefail

bin="$(realpath "${1:-target/debug/generate-license}")"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
checked=0
failed=0

# check <project license | -> <licenses, sorted> <generate-license arguments...>
#
# Generates into an empty directory, then compares what licensee reports:
# the license of the project as a whole ("-" to skip) and the licenses of
# the individual files.
check() {
  local want_project="$1" want_licenses="$2"
  shift 2
  local dir got_project got_licenses
  dir="$(mktemp -d "$work/XXXXXX")"
  checked=$((checked + 1))

  if ! (cd "$dir" && "$bin" "$@" --holder "Test Holder" 2>"$work/stderr"); then
    echo "FAIL $*: generate-license failed"
    cat "$work/stderr"
    failed=$((failed + 1))
    return
  fi
  # licensee exits with 1 when it finds nothing: the comparison reports it.
  got_licenses="$( (licensee detect --json "$dir" || true) |
    ruby -rjson -e 'puts JSON.parse(STDIN.read)["licenses"].map { |l| l["spdx_id"] }.sort.join(" ")')"
  got_project="$( (licensee detect "$dir" || true) | sed -n 's/^License: *//p')"

  if [ "$got_licenses" != "$want_licenses" ]; then
    echo "FAIL $*: expected licenses '$want_licenses', got '$got_licenses'"
    failed=$((failed + 1))
  elif [ "$want_project" != "-" ] && [ "$got_project" != "$want_project" ]; then
    echo "FAIL $*: expected project license '$want_project', got '$got_project'"
    failed=$((failed + 1))
  else
    echo "ok   $*"
  fi
}

# Every identifier whose text comes from choosealicense.com. licensee names
# a license by the spdx-id of that source file, which for the GNU licenses
# is the deprecated identifier without -only or -or-later.
for id in $("$bin" --list | awk '{ print $1 }'); do
  case "$id" in
    CC-BY-NC-* | CC-BY-ND-*) continue ;; # unknown to licensee
  esac
  expected="${id%-only}"
  expected="${expected%-or-later}"
  if [ "$expected" = "LGPL-3.0" ]; then
    # Recognized as LGPL-3.0 thanks to the COPYING.LESSER and COPYING pair.
    check LGPL-3.0 "GPL-3.0 LGPL-3.0" "$id" --year 2026
  else
    check "$expected" "$expected" "$id" --year 2026
  fi
done

# The licenses with a copyright line, without the year.
for id in MIT MIT-0 ISC 0BSD BSD-2-Clause BSD-3-Clause Zlib; do
  check "$id" "$id" "$id" --no-year
done

# A dual license: both files must be recognized.
check - "Apache-2.0 MIT" MIT OR Apache-2.0 --year 2026

if [ "$checked" -ne 34 ]; then
  echo "FAIL expected 34 cases (26 + 7 + 1), ran $checked"
  failed=$((failed + 1))
fi
echo "$checked cases, $failed failed"
[ "$failed" -eq 0 ]
