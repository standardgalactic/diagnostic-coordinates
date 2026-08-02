#!/usr/bin/env bash
set -euo pipefail

##############################################################################
# monotonic_commit.sh
#
# Commits every file under version-* directories individually.
# Pushes once at the end.
#
# Usage:
#
#     ./monotonic_commit.sh
#
##############################################################################

MODEL="${MODEL:-granite4.1:3b}"
LOG="commit-$(date +%Y%m%d-%H%M%S).log"

git rev-parse --is-inside-work-tree >/dev/null

echo "==========================================" | tee "$LOG"
echo "Monotonic Commit Pipeline" | tee -a "$LOG"
echo "Repository : $(basename "$(pwd)")" | tee -a "$LOG"
echo "==========================================" | tee -a "$LOG"

###########################################################################
# Classification
###########################################################################

classify() {

    local file="$1"

    case "$file" in
        *.rs)   echo "[rust]" ;;
        *.py)   echo "[python]" ;;
        *.sh)   echo "[bash]" ;;
        *.tex)  echo "[paper]" ;;
        *.bib)  echo "[bibliography]" ;;
        *.md)   echo "[docs]" ;;
        *.html) echo "[website]" ;;
        *.css)  echo "[css]" ;;
        *.js)   echo "[javascript]" ;;
        *.json|*.toml|*.yaml|*.yml)
                 echo "[config]" ;;
        *.png|*.jpg|*.jpeg|*.gif|*.svg|*.pdf)
                 echo "[assets]" ;;
        *)
            local headtext
            headtext="$(head -50 "$file" 2>/dev/null || true)"

            if grep -Eq '\\documentclass|\\chapter|\\section' <<<"$headtext"; then
                echo "[paper]"
            elif grep -Eq '^#!/usr/bin/env bash|^#!/bin/bash' <<<"$headtext"; then
                echo "[bash]"
            elif grep -Eq 'fn main|impl |struct |enum |use ' <<<"$headtext"; then
                echo "[rust]"
            else
                echo "[misc]"
            fi
            ;;
    esac
}

###########################################################################
# Simple description
###########################################################################

describe() {

    basename "$1" |
        sed 's/\.[^.]*$//' |
        tr '_-' ' ' |
        sed 's/[[:space:]]\+/ /g'
}

###########################################################################
# Optional Ollama summary
###########################################################################

# Uncomment if desired.
#
# summarize() {
#
#     local file="$1"
#
#     {
#         echo "Write a Git commit message."
#         echo
#         echo "Maximum 8 words."
#         echo "Imperative mood."
#         echo
#         echo "Filename:"
#         echo "$file"
#         echo
#         head -200 "$file"
#
#     } | ollama run "$MODEL" | head -1
# }

###########################################################################
# Main
###########################################################################

count=0

find version-* -type f | sort | while read -r file
do

    #
    # Skip generated junk if desired
    #

    case "$file" in
        */.git/*) continue ;;
        */commit-*.log) continue ;;
    esac

    #
    # Skip already tracked files.
    # Remove these three lines if you instead want to recommit modified files.
    #

    if git ls-files --error-unmatch "$file" >/dev/null 2>&1; then
        continue
    fi

    tag=$(classify "$file")

    msg="$tag $(describe "$file")"

    #
    # Uncomment for Ollama summaries
    #
    # msg="$tag $(summarize "$file")"

    echo
    echo "--------------------------------------------------" | tee -a "$LOG"
    echo "$file" | tee -a "$LOG"
    echo "$msg" | tee -a "$LOG"

    git add "$file"

    git commit -m "$msg" | tee -a "$LOG"

    count=$((count+1))

done

echo
echo "==========================================" | tee -a "$LOG"
echo "$count commits created." | tee -a "$LOG"
echo "Pushing..." | tee -a "$LOG"

git push | tee -a "$LOG"

echo
echo "Done."
echo "Log written to $LOG"