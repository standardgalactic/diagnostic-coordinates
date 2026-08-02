#!/usr/bin/env bash
set -euo pipefail

# ============================================================
# essay_pipeline.sh — section-by-section long-form generator
# ============================================================
#
# Redesign of the original single-pass draft/critique/revise/polish
# script. Instead of feeding the whole essay back into the model on
# every pass, this splits the outline into top-level sections,
# generates + reviews each one independently, and carries a small
# persistent memory.md (definitions / claims / open questions)
# forward instead of the full prior text. That keeps every prompt
# small regardless of how long the finished document gets, which is
# what makes book-length runs practical.
#
# NOTE on Ollama context size: `ollama run` has no --ctx-size flag.
# Runtime context is set either via the interactive `/set parameter
# num_ctx N` command (works when piped to stdin, which is what this
# script does — same mechanism the original script used), or baked
# into a custom Modelfile with `PARAMETER num_ctx N`, or via the
# server-side OLLAMA_CONTEXT_LENGTH env var when you start `ollama
# serve`. This script keeps the /set parameter approach since it's
# the one that actually works per-invocation without a custom model.
#
# ------------------------------------------------------------
# Usage:
#   ./essay_pipeline.sh outline.md [output.md] [workdir]
#
# Outline format: top-level "# " headings mark section boundaries.
# Everything under a "# " heading (including "##" subheadings) is
# that section's outline. Content before the first "# " is ignored.
# ------------------------------------------------------------

# -----------------------------
# Config (override via env vars)
# -----------------------------
MODEL_DRAFT="${MODEL_DRAFT:-granite4.1:8b}"
MODEL_REVIEW="${MODEL_REVIEW:-granite4.1:8b}"
MODEL_POLISH="${MODEL_POLISH:-granite4.1:8b}"
MODEL_MEMORY="${MODEL_MEMORY:-granite4.1:8b}"

OLLAMA_NUM_CTX="${OLLAMA_NUM_CTX:-8192}"
TEMP_DRAFT="${TEMP_DRAFT:-0.7}"
TEMP_REVIEW="${TEMP_REVIEW:-0.3}"
TEMP_POLISH="${TEMP_POLISH:-0.4}"
TEMP_MEMORY="${TEMP_MEMORY:-0.2}"

STYLE="${STYLE:-formal academic prose}"
TARGET_WORDS_PER_SECTION="${TARGET_WORDS_PER_SECTION:-900}"
REVISION_LAYERS="${REVISION_LAYERS:-1}"     # critique/revise loops per section
RUN_CONSISTENCY_CHECK="${RUN_CONSISTENCY_CHECK:-1}"
FORCE="${FORCE:-0}"                         # 1 = regenerate finished sections
KEEP_ARTIFACTS="${KEEP_ARTIFACTS:-1}"
DO_FINAL_POLISH="${DO_FINAL_POLISH:-0}"     # whole-document polish pass; off
                                             # by default since it reintroduces
                                             # the full-context problem this
                                             # redesign exists to avoid

OUTLINE="${1:-}"
OUTPUT="${2:-book.md}"
WORKDIR="${3:-.pipeline_work}"

usage() {
  cat <<EOF
Usage: $0 outline.md [output.md] [workdir]

Outline: top-level "# " headings become sections; everything under
a heading (incl. "##" subheadings) is that section's outline.

Env knobs:
  MODEL_DRAFT, MODEL_REVIEW, MODEL_POLISH, MODEL_MEMORY
  OLLAMA_NUM_CTX
  TEMP_DRAFT, TEMP_REVIEW, TEMP_POLISH, TEMP_MEMORY
  STYLE, TARGET_WORDS_PER_SECTION, REVISION_LAYERS
  RUN_CONSISTENCY_CHECK   (1/0)
  FORCE                   (1 = rebuild sections that already finished)
  KEEP_ARTIFACTS          (0 = delete workdir after assembly)
  DO_FINAL_POLISH         (1 = run one extra polish pass on the full doc)
EOF
}

if [[ -z "$OUTLINE" || ! -f "$OUTLINE" ]]; then
  usage
  exit 1
fi

mkdir -p "$WORKDIR/sections_in" "$WORKDIR/sections_out" "$WORKDIR/logs"
MEMORY_FILE="$WORKDIR/memory.md"
if [[ ! -f "$MEMORY_FILE" ]]; then
  cat > "$MEMORY_FILE" <<'EOF'
# Project Memory

## Definitions
(none yet)

## Established Claims
(none yet)

## Open Questions
(none yet)
EOF
fi

# -----------------------------
# Helpers
# -----------------------------

# run_ollama MODEL TEMP PROMPT_FILE OUT_FILE LOG_FILE
# Streams live to the terminal via tee, appends the same output to
# LOG_FILE, and writes the final response to OUT_FILE.
run_ollama() {
  local model="$1" temp="$2" prompt_file="$3" out_file="$4" log_file="$5"
  {
    echo "/set parameter num_ctx $OLLAMA_NUM_CTX"
    echo "/set parameter temperature $temp"
    cat "$prompt_file"
  } | ollama run "$model" 2>"$WORKDIR/.ollama_err" \
    | tee "$out_file" | tee -a "$log_file" >/dev/null \
    || {
      echo "WARN: /set parameter not accepted by $model, retrying plain" >&2
      cat "$prompt_file" | ollama run "$model" \
        | tee "$out_file" | tee -a "$log_file" >/dev/null
    }
  cp "$prompt_file" "$WORKDIR/logs/$(basename "$log_file" .log).prompt.txt"
}

word_count() { wc -w < "$1" | tr -d ' '; }

progress() {
  # progress SECTION_NUM TOTAL STAGE
  echo "[section $1/$2] $3"
}

# -----------------------------
# Split outline into sections
# -----------------------------
rm -f "$WORKDIR"/sections_in/*.md
awk -v outdir="$WORKDIR/sections_in" '
  /^# / { n++; fname = sprintf("%s/%02d.md", outdir, n) }
  n > 0 { print > fname }
' "$OUTLINE"

TOTAL=$(ls "$WORKDIR"/sections_in/*.md 2>/dev/null | wc -l | tr -d ' ')
if [[ "$TOTAL" -eq 0 ]]; then
  echo "ERROR: no top-level '# ' headings found in $OUTLINE" >&2
  exit 1
fi
echo "Parsed $TOTAL section(s) from $OUTLINE"

# -----------------------------
# Per-section pipeline
# -----------------------------
for sec_in in "$WORKDIR"/sections_in/*.md; do
  n=$(basename "$sec_in" .md)
  final_out="$WORKDIR/sections_out/${n}_final.md"

  if [[ -f "$final_out" && "$FORCE" != "1" ]]; then
    progress "$n" "$TOTAL" "already finished, skipping (set FORCE=1 to redo)"
    continue
  fi

  logdir="$WORKDIR/logs/section_${n}"
  mkdir -p "$logdir"

  # Prior section's tail, for continuity — small, not the whole doc.
  prev_num=$((10#$n - 1))
  prev_tail=""
  if [[ "$prev_num" -ge 1 ]]; then
    prev_file=$(printf "%s/sections_out/%02d_final.md" "$WORKDIR" "$prev_num")
    [[ -f "$prev_file" ]] && prev_tail=$(tail -n 15 "$prev_file")
  fi

  # ---- Draft ----
  progress "$n" "$TOTAL" "draft"
  draft_prompt="$logdir/01_draft_prompt.txt"
  cat > "$draft_prompt" <<EOF
You are an expert academic writer working on one section of a
longer document. Only this section's outline is shown to you —
rely on the project memory below for established terminology and
prior claims, not on having seen earlier sections in full.

Hard requirements:
- Preserve the structure of this section's outline.
- Use clear headings/subheadings matching the outline.
- Write coherent paragraphs with transitions.
- Add concrete examples where appropriate.
- Do not invent citations or references.
- Stay consistent with the project memory below; do not
  contradict established claims or redefine existing terms.
- Target approximately ${TARGET_WORDS_PER_SECTION} words.
- Style: ${STYLE}.
- Return only this section's body in Markdown.

Project memory:
$(cat "$MEMORY_FILE")

End of previous section (for continuity only, do not repeat it):
${prev_tail}

This section's outline:
$(cat "$sec_in")
EOF
  run_ollama "$MODEL_DRAFT" "$TEMP_DRAFT" "$draft_prompt" \
    "$logdir/02_draft.md" "$logdir/section.log"
  current="$logdir/02_draft.md"

  # ---- Critique / revise loop ----
  for i in $(seq 1 "$REVISION_LAYERS"); do
    progress "$n" "$TOTAL" "critique (layer $i)"
    critique_prompt="$logdir/$(printf "%02d" $((2*i+1)))_critique_prompt.txt"
    critique_out="$logdir/$(printf "%02d" $((2*i+2)))_critique.md"
    cat > "$critique_prompt" <<EOF
You are a strict academic editor reviewing one section of a larger
document. Evaluate against:
1) Structural fidelity to this section's outline
2) Argument clarity and logical flow
3) Depth of analysis
4) Precision and academic tone
5) Redundancy, vagueness, unsupported claims
6) Consistency with the project memory (no redefinitions,
   no contradictions of established claims)

Return a prioritized bullet list of issues, concrete rewrite
instructions, and a short "must-fix first" section.

Project memory:
$(cat "$MEMORY_FILE")

Section outline:
$(cat "$sec_in")

Section text:
$(cat "$current")
EOF
    run_ollama "$MODEL_REVIEW" "$TEMP_REVIEW" "$critique_prompt" \
      "$critique_out" "$logdir/section.log"

    progress "$n" "$TOTAL" "revise (layer $i)"
    revise_prompt="$logdir/$(printf "%02d" $((2*i+3)))_revise_prompt.txt"
    revise_out="$logdir/$(printf "%02d" $((2*i+4)))_revised.md"
    cat > "$revise_prompt" <<EOF
Revise this section using the critique below.

Rules:
- Keep the best original content where strong.
- Fix all must-fix items first.
- Improve cohesion and depth.
- Do not invent citations.
- Preserve markdown heading structure.
- Stay consistent with the project memory.
- Return only the revised section.

Project memory:
$(cat "$MEMORY_FILE")

Critique:
$(cat "$critique_out")

Section to revise:
$(cat "$current")
EOF
    run_ollama "$MODEL_DRAFT" "$TEMP_DRAFT" "$revise_prompt" \
      "$revise_out" "$logdir/section.log"
    current="$revise_out"
  done

  # ---- Consistency check (against memory, not full doc) ----
  if [[ "$RUN_CONSISTENCY_CHECK" == "1" ]]; then
    progress "$n" "$TOTAL" "consistency check"
    consistency_prompt="$logdir/90_consistency_prompt.txt"
    consistency_out="$logdir/91_consistency.md"
    cat > "$consistency_prompt" <<EOF
Check this section against the project memory for:
- inconsistent terminology
- duplicate concepts already covered elsewhere (per memory)
- contradictions with established claims
- undefined technical terms
- missing transitions

Return only a short list of fixes needed, or "No issues found."

Project memory:
$(cat "$MEMORY_FILE")

Section text:
$(cat "$current")
EOF
    run_ollama "$MODEL_REVIEW" "$TEMP_REVIEW" "$consistency_prompt" \
      "$consistency_out" "$logdir/section.log"

    if ! grep -qi "no issues found" "$consistency_out"; then
      progress "$n" "$TOTAL" "applying consistency fixes"
      fix_prompt="$logdir/92_consistency_fix_prompt.txt"
      fix_out="$logdir/93_consistency_fixed.md"
      cat > "$fix_prompt" <<EOF
Apply these consistency fixes to the section. Return only the
corrected section in Markdown.

Fixes needed:
$(cat "$consistency_out")

Section:
$(cat "$current")
EOF
      run_ollama "$MODEL_DRAFT" "$TEMP_DRAFT" "$fix_prompt" \
        "$fix_out" "$logdir/section.log"
      current="$fix_out"
    fi
  fi

  # ---- Polish ----
  progress "$n" "$TOTAL" "polish"
  polish_prompt="$logdir/95_polish_prompt.txt"
  cat > "$polish_prompt" <<EOF
Final-pass copy edit of this section for:
- concision without loss of meaning
- sentence variety
- terminology consistency with the project memory
- typo/grammar fixes
- smooth transitions
- ${STYLE}

Do not invent citations. Return only the final section markdown.

Project memory:
$(cat "$MEMORY_FILE")

Section:
$(cat "$current")
EOF
  run_ollama "$MODEL_POLISH" "$TEMP_POLISH" "$polish_prompt" \
    "$final_out" "$logdir/section.log"

  # ---- Update project memory ----
  progress "$n" "$TOTAL" "updating memory"
  memory_prompt="$logdir/98_memory_prompt.txt"
  memory_out="$logdir/99_memory_updated.md"
  cat > "$memory_prompt" <<EOF
Update the project memory to incorporate this newly finished
section. Merge in new definitions, new established claims, and
new open questions. Keep existing entries unless this section
explicitly supersedes them. Keep it compact — this file is read
in full by every future prompt, so prune anything no longer
useful rather than letting it grow without bound.

Return only the full updated memory.md content, same three-section
format (Definitions / Established Claims / Open Questions).

Current memory:
$(cat "$MEMORY_FILE")

Newly finished section:
$(cat "$final_out")
EOF
  run_ollama "$MODEL_MEMORY" "$TEMP_MEMORY" "$memory_prompt" \
    "$memory_out" "$logdir/section.log"
  cp "$memory_out" "$MEMORY_FILE"

  wc=$(word_count "$final_out")
  progress "$n" "$TOTAL" "done (${wc} words)"
done

# -----------------------------
# Assemble final document
# -----------------------------
echo "Assembling final document..."
> "$OUTPUT"
for f in "$WORKDIR"/sections_out/*_final.md; do
  cat "$f" >> "$OUTPUT"
  echo -e "\n" >> "$OUTPUT"
done

if [[ "$DO_FINAL_POLISH" == "1" ]]; then
  echo "Running optional whole-document polish pass..."
  echo "(this reintroduces full-document context cost — only sensible for shorter documents)"
  final_polish_prompt="$WORKDIR/logs/final_polish_prompt.txt"
  cat > "$final_polish_prompt" <<EOF
Do one light final pass over this complete document: fix any
seams between sections (abrupt transitions, duplicated phrasing
across section boundaries), keep everything else as-is. Return
only the full corrected document in Markdown.

Document:
$(cat "$OUTPUT")
EOF
  run_ollama "$MODEL_POLISH" "$TEMP_POLISH" "$final_polish_prompt" \
    "$OUTPUT" "$WORKDIR/logs/final_polish.log"
fi

total_words=$(word_count "$OUTPUT")
echo "Final document written to: $OUTPUT (${total_words} words, ${TOTAL} sections)"
echo "Project memory: $MEMORY_FILE"
echo "Logs and intermediate artifacts: $WORKDIR"

if [[ "$KEEP_ARTIFACTS" == "0" ]]; then
  rm -rf "$WORKDIR"
  echo "Intermediate artifacts removed."
fi
