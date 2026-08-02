#!/usr/bin/env bash
set -euo pipefail

INPUT="${1:-template.tex}"
OUTPUT="${2:-template.index.tsv}"

[[ -f "$INPUT" ]] || {
    echo "Cannot find $INPUT"
    exit 1
}

awk '

BEGIN {
    FS=""
    n=0
}

function emit(type,title,line) {
    n++
    printf "%d\t%s\t%s\t%d\n", n, type, title, line
}

{
    text=$0

    #
    # Remove comments
    #
    sub(/%.*/, "", text)

    #
    # Chapters
    #
    if (match(text,/\\chapter\*?\{[^}]+\}/)) {

        title=text
        sub(/^.*\\chapter\*?\{/,"",title)
        sub(/\}.*/,"",title)

        emit("chapter",title,NR)
        next
    }

    #
    # Sections
    #
    if (match(text,/\\section\*?\{[^}]+\}/)) {

        title=text
        sub(/^.*\\section\*?\{/,"",title)
        sub(/\}.*/,"",title)

        emit("section",title,NR)
        next
    }

    #
    # Subsections
    #
    if (match(text,/\\subsection\*?\{[^}]+\}/)) {

        title=text
        sub(/^.*\\subsection\*?\{/,"",title)
        sub(/\}.*/,"",title)

        emit("subsection",title,NR)
        next
    }

    #
    # Subsubsections
    #
    if (match(text,/\\subsubsection\*?\{[^}]+\}/)) {

        title=text
        sub(/^.*\\subsubsection\*?\{/,"",title)
        sub(/\}.*/,"",title)

        emit("subsubsection",title,NR)
        next
    }

    #
    # Parts
    #
    if (match(text,/\\part\*?\{[^}]+\}/)) {

        title=text
        sub(/^.*\\part\*?\{/,"",title)
        sub(/\}.*/,"",title)

        emit("part",title,NR)
        next
    }

    #
    # Appendices
    #
    if (match(text,/\\appendix/)) {
        emit("appendix","",NR)
        next
    }

    #
    # Environments
    #
    if (match(text,/\\begin\{definition\}/))
        emit("definition","",NR)

    if (match(text,/\\begin\{theorem\}/))
        emit("theorem","",NR)

    if (match(text,/\\begin\{lemma\}/))
        emit("lemma","",NR)

    if (match(text,/\\begin\{corollary\}/))
        emit("corollary","",NR)

    if (match(text,/\\begin\{proposition\}/))
        emit("proposition","",NR)

    if (match(text,/\\begin\{example\}/))
        emit("example","",NR)

    if (match(text,/\\begin\{remark\}/))
        emit("remark","",NR)

    if (match(text,/\\begin\{proof\}/))
        emit("proof","",NR)

    #
    # Figures
    #
    if (match(text,/\\begin\{figure/))
        emit("figure","",NR)

    #
    # Tables
    #
    if (match(text,/\\begin\{table/))
        emit("table","",NR)

    #
    # Bibliography
    #
    if (match(text,/\\bibliography/))
        emit("bibliography","",NR)

    if (match(text,/\\printbibliography/))
        emit("bibliography","",NR)

}
' "$INPUT" > "$OUTPUT"

echo
echo "Created:"
echo "    $OUTPUT"
echo

column -t -s $'\t' "$OUTPUT"
