#!/usr/bin/env bash

MODEL="granite4.1:3b"
OUTLINE="$1"
OUTPUT="${2:-essay.md}"

if [ -z "$OUTLINE" ]; then
    echo "Usage: $0 outline.txt [output.md]"
    exit 1
fi

PROMPT=$(cat <<EOF
You are an expert academic writer.

Expand the following outline into a complete essay.

Requirements:
- Preserve the overall structure.
- Use clear headings.
- Write coherent paragraphs.
- Add examples where appropriate.
- Do not invent citations.
- Write in formal academic prose.
- Return only the essay.

Outline:

$(cat "$OUTLINE")
EOF
)

ollama run "$MODEL" <<< "$PROMPT" > "$OUTPUT"

echo "Essay written to $OUTPUT":
