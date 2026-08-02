use std::collections::{BTreeMap, HashMap};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_TREE_DEPTH: usize = 5;
const LARGEST_FILE_COUNT: usize = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Language {
    Rust,
    Latex,
    Markdown,
    Shell,
    Python,
    Html,
    Css,
    JavaScript,
    TypeScript,
    Json,
    Toml,
    Yaml,
    C,
    Cpp,
    Java,
    Go,
    Ruby,
    Sql,
    Text,
    Pdf,
    Image,
    Audio,
    Video,
    Archive,
    Binary,
    Unknown,
}

impl Language {
    fn from_path(path: &Path) -> Self {
        let extension = path
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_ascii_lowercase();

        match extension.as_str() {
            "rs" => Self::Rust,
            "tex" | "sty" | "cls" => Self::Latex,
            "md" | "markdown" => Self::Markdown,
            "sh" | "bash" | "zsh" | "fish" => Self::Shell,
            "py" => Self::Python,
            "html" | "htm" => Self::Html,
            "css" => Self::Css,
            "js" | "mjs" | "cjs" => Self::JavaScript,
            "ts" | "tsx" => Self::TypeScript,
            "json" => Self::Json,
            "toml" => Self::Toml,
            "yaml" | "yml" => Self::Yaml,
            "c" | "h" => Self::C,
            "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => Self::Cpp,
            "java" => Self::Java,
            "go" => Self::Go,
            "rb" => Self::Ruby,
            "sql" => Self::Sql,
            "txt" | "csv" | "tsv" | "log" | "vtt" | "srt" => Self::Text,
            "pdf" => Self::Pdf,
            "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "bmp" | "ico" => {
                Self::Image
            }
            "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" => Self::Audio,
            "mp4" | "mkv" | "avi" | "mov" | "webm" => Self::Video,
            "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => Self::Archive,
            "exe" | "dll" | "so" | "dylib" | "o" | "obj" | "class" | "wasm" => Self::Binary,
            _ => Self::Unknown,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Rust => "Rust",
            Self::Latex => "LaTeX",
            Self::Markdown => "Markdown",
            Self::Shell => "Shell",
            Self::Python => "Python",
            Self::Html => "HTML",
            Self::Css => "CSS",
            Self::JavaScript => "JavaScript",
            Self::TypeScript => "TypeScript",
            Self::Json => "JSON",
            Self::Toml => "TOML",
            Self::Yaml => "YAML",
            Self::C => "C",
            Self::Cpp => "C++",
            Self::Java => "Java",
            Self::Go => "Go",
            Self::Ruby => "Ruby",
            Self::Sql => "SQL",
            Self::Text => "Text",
            Self::Pdf => "PDF",
            Self::Image => "Image",
            Self::Audio => "Audio",
            Self::Video => "Video",
            Self::Archive => "Archive",
            Self::Binary => "Binary",
            Self::Unknown => "Unknown",
        }
    }

    fn is_textual(self) -> bool {
        !matches!(
            self,
            Self::Pdf
                | Self::Image
                | Self::Audio
                | Self::Video
                | Self::Archive
                | Self::Binary
        )
    }
}

#[derive(Debug)]
struct FileInfo {
    path: PathBuf,
    relative_path: PathBuf,
    language: Language,
    size: u64,
    lines: usize,
}

#[derive(Debug, Default)]
struct LatexStats {
    parts: usize,
    chapters: usize,
    sections: usize,
    subsections: usize,
    subsubsections: usize,
    definitions: usize,
    theorems: usize,
    lemmas: usize,
    propositions: usize,
    corollaries: usize,
    proofs: usize,
    examples: usize,
    remarks: usize,
    figures: usize,
    tables: usize,
    equations: usize,
    citations: usize,
}

impl LatexStats {
    fn add_assign(&mut self, other: &Self) {
        self.parts += other.parts;
        self.chapters += other.chapters;
        self.sections += other.sections;
        self.subsections += other.subsections;
        self.subsubsections += other.subsubsections;
        self.definitions += other.definitions;
        self.theorems += other.theorems;
        self.lemmas += other.lemmas;
        self.propositions += other.propositions;
        self.corollaries += other.corollaries;
        self.proofs += other.proofs;
        self.examples += other.examples;
        self.remarks += other.remarks;
        self.figures += other.figures;
        self.tables += other.tables;
        self.equations += other.equations;
        self.citations += other.citations;
    }
}

#[derive(Debug, Default)]
struct RustStats {
    functions: usize,
    public_functions: usize,
    structs: usize,
    enums: usize,
    traits: usize,
    impl_blocks: usize,
    modules: usize,
    tests: usize,
    macros: usize,
    unsafe_uses: usize,
}

impl RustStats {
    fn add_assign(&mut self, other: &Self) {
        self.functions += other.functions;
        self.public_functions += other.public_functions;
        self.structs += other.structs;
        self.enums += other.enums;
        self.traits += other.traits;
        self.impl_blocks += other.impl_blocks;
        self.modules += other.modules;
        self.tests += other.tests;
        self.macros += other.macros;
        self.unsafe_uses += other.unsafe_uses;
    }
}

#[derive(Debug, Default)]
struct GitInfo {
    available: bool,
    branch: String,
    commit_count: usize,
    tracked_files: usize,
    modified: usize,
    staged: usize,
    untracked: usize,
    remote: String,
    latest_commit: String,
}

#[derive(Debug)]
struct Repository {
    root: PathBuf,
    files: Vec<FileInfo>,
    directories: Vec<PathBuf>,
    total_size: u64,
    total_lines: usize,
    latex: LatexStats,
    rust: RustStats,
    git: GitInfo,
}

impl Repository {
    fn new(root: PathBuf) -> Self {
        Self {
            root,
            files: Vec::new(),
            directories: Vec::new(),
            total_size: 0,
            total_lines: 0,
            latex: LatexStats::default(),
            rust: RustStats::default(),
            git: GitInfo::default(),
        }
    }
}

#[derive(Debug)]
struct Config {
    root: PathBuf,
    tree_depth: usize,
    show_hidden: bool,
}

fn usage(program: &str) {
    println!(
        "Repository Analyzer

Usage:
  {program} [PATH] [OPTIONS]

Arguments:
  PATH                  Repository path; defaults to current directory

Options:
  --depth N             Maximum displayed tree depth
  --show-hidden         Include hidden files and directories
  -h, --help            Show this help

Examples:
  {program}
  {program} .
  {program} ~/projects/physiome
  {program} . --depth 3
  {program} . --show-hidden
"
    );
}

fn parse_args() -> Result<Config, String> {
    let args: Vec<String> = env::args().collect();
    let program = args.first().map(String::as_str).unwrap_or("repo_analyzer");

    let mut root = PathBuf::from(".");
    let mut tree_depth = DEFAULT_TREE_DEPTH;
    let mut show_hidden = false;
    let mut root_set = false;

    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                usage(program);
                std::process::exit(0);
            }
            "--show-hidden" => {
                show_hidden = true;
                i += 1;
            }
            "--depth" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--depth requires a number".to_string())?;

                tree_depth = value
                    .parse::<usize>()
                    .map_err(|_| format!("Invalid depth: {value}"))?;

                i += 2;
            }
            value if value.starts_with('-') => {
                return Err(format!("Unknown option: {value}"));
            }
            value => {
                if root_set {
                    return Err(format!("Unexpected extra path: {value}"));
                }

                root = PathBuf::from(value);
                root_set = true;
                i += 1;
            }
        }
    }

    Ok(Config {
        root,
        tree_depth,
        show_hidden,
    })
}

fn should_skip_directory(name: &str, show_hidden: bool) -> bool {
    const ALWAYS_SKIP: &[&str] = &[
        ".git",
        "target",
        "node_modules",
        ".idea",
        ".vscode",
        "__pycache__",
        ".pytest_cache",
        ".mypy_cache",
        ".cache",
        "dist",
        "build",
        "coverage",
    ];

    if ALWAYS_SKIP.contains(&name) {
        return true;
    }

    !show_hidden && name.starts_with('.')
}

fn should_skip_file(name: &str, show_hidden: bool) -> bool {
    if !show_hidden && name.starts_with('.') {
        return true;
    }

    matches!(
        name,
        ".DS_Store"
            | "Thumbs.db"
            | "desktop.ini"
            | "Cargo.lock"
            | "package-lock.json"
            | "yarn.lock"
    )
}

fn count_lines(path: &Path, language: Language) -> usize {
    if !language.is_textual() {
        return 0;
    }

    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return 0,
    };

    BufReader::new(file).lines().count()
}

fn walk_repository(
    current: &Path,
    root: &Path,
    repository: &mut Repository,
    show_hidden: bool,
) -> io::Result<()> {
    let mut entries = fs::read_dir(current)?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    entries.sort_by_key(|entry| entry.file_name().to_string_lossy().to_ascii_lowercase());

    for entry in entries {
        let path = entry.path();
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(error) => {
                eprintln!("Warning: cannot inspect {}: {error}", path.display());
                continue;
            }
        };

        if file_type.is_symlink() {
            continue;
        }

        if file_type.is_dir() {
            if should_skip_directory(&name, show_hidden) {
                continue;
            }

            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_path_buf();

            repository.directories.push(relative);
            walk_repository(&path, root, repository, show_hidden)?;
            continue;
        }

        if !file_type.is_file() || should_skip_file(&name, show_hidden) {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(error) => {
                eprintln!("Warning: cannot read metadata for {}: {error}", path.display());
                continue;
            }
        };

        let language = Language::from_path(&path);
        let lines = count_lines(&path, language);
        let size = metadata.len();
        let relative_path = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_path_buf();

        repository.total_size += size;
        repository.total_lines += lines;

        if language == Language::Latex {
            let stats = analyze_latex_file(&path);
            repository.latex.add_assign(&stats);
        }

        if language == Language::Rust {
            let stats = analyze_rust_file(&path);
            repository.rust.add_assign(&stats);
        }

        repository.files.push(FileInfo {
            path,
            relative_path,
            language,
            size,
            lines,
        });
    }

    Ok(())
}

fn strip_latex_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%' {
            let mut slash_count = 0;
            let mut previous = index;

            while previous > 0 && bytes[previous - 1] == b'\\' {
                slash_count += 1;
                previous -= 1;
            }

            if slash_count % 2 == 0 {
                return &line[..index];
            }
        }

        index += 1;
    }

    line
}

fn analyze_latex_file(path: &Path) -> LatexStats {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return LatexStats::default(),
    };

    let mut stats = LatexStats::default();

    for line in BufReader::new(file).lines().map_while(Result::ok) {
        let line = strip_latex_comment(&line);

        stats.parts += count_occurrences(line, "\\part{")
            + count_occurrences(line, "\\part*{");
        stats.chapters += count_occurrences(line, "\\chapter{")
            + count_occurrences(line, "\\chapter*{");

        stats.subsubsections += count_occurrences(line, "\\subsubsection{")
            + count_occurrences(line, "\\subsubsection*{");

        stats.subsections += count_occurrences(line, "\\subsection{")
            + count_occurrences(line, "\\subsection*{");

        let section_count =
            count_occurrences(line, "\\section{") + count_occurrences(line, "\\section*{");

        let subsection_count =
            count_occurrences(line, "\\subsection{") + count_occurrences(line, "\\subsection*{");

        let subsubsection_count = count_occurrences(line, "\\subsubsection{")
            + count_occurrences(line, "\\subsubsection*{");

        stats.sections += section_count
            .saturating_sub(subsection_count)
            .saturating_sub(subsubsection_count);

        stats.definitions += count_occurrences(line, "\\begin{definition");
        stats.theorems += count_occurrences(line, "\\begin{theorem");
        stats.lemmas += count_occurrences(line, "\\begin{lemma");
        stats.propositions += count_occurrences(line, "\\begin{proposition");
        stats.corollaries += count_occurrences(line, "\\begin{corollary");
        stats.proofs += count_occurrences(line, "\\begin{proof");
        stats.examples += count_occurrences(line, "\\begin{example");
        stats.remarks += count_occurrences(line, "\\begin{remark");
        stats.figures += count_occurrences(line, "\\begin{figure");
        stats.tables += count_occurrences(line, "\\begin{table");

        stats.equations += count_occurrences(line, "\\begin{equation")
            + count_occurrences(line, "\\begin{align")
            + count_occurrences(line, "\\[");
        stats.citations += count_occurrences(line, "\\cite{")
            + count_occurrences(line, "\\citep{")
            + count_occurrences(line, "\\citet{");
    }

    stats
}

fn analyze_rust_file(path: &Path) -> RustStats {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return RustStats::default(),
    };

    let mut stats = RustStats::default();

    for line in BufReader::new(file).lines().map_while(Result::ok) {
        let trimmed = line.trim();

        if trimmed.starts_with("//") {
            continue;
        }

        if trimmed.starts_with("pub fn ")
            || trimmed.starts_with("pub(crate) fn ")
            || trimmed.starts_with("pub(super) fn ")
        {
            stats.public_functions += 1;
            stats.functions += 1;
        } else if trimmed.starts_with("fn ")
            || trimmed.starts_with("async fn ")
            || trimmed.starts_with("pub async fn ")
        {
            stats.functions += 1;
        }

        if trimmed.starts_with("struct ") || trimmed.starts_with("pub struct ") {
            stats.structs += 1;
        }

        if trimmed.starts_with("enum ") || trimmed.starts_with("pub enum ") {
            stats.enums += 1;
        }

        if trimmed.starts_with("trait ") || trimmed.starts_with("pub trait ") {
            stats.traits += 1;
        }

        if trimmed.starts_with("impl ") || trimmed.starts_with("impl<") {
            stats.impl_blocks += 1;
        }

        if trimmed.starts_with("mod ") || trimmed.starts_with("pub mod ") {
            stats.modules += 1;
        }

        if trimmed == "#[test]" || trimmed.starts_with("#[tokio::test") {
            stats.tests += 1;
        }

        if trimmed.starts_with("macro_rules!") {
            stats.macros += 1;
        }

        if trimmed.contains("unsafe ") || trimmed.starts_with("unsafe {") {
            stats.unsafe_uses += 1;
        }
    }

    stats
}

fn count_occurrences(text: &str, pattern: &str) -> usize {
    text.match_indices(pattern).count()
}

fn run_git(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn analyze_git(root: &Path) -> GitInfo {
    let inside = run_git(root, &["rev-parse", "--is-inside-work-tree"]);

    if inside.as_deref() != Some("true") {
        return GitInfo::default();
    }

    let branch = run_git(root, &["branch", "--show-current"]).unwrap_or_default();

    let commit_count = run_git(root, &["rev-list", "--count", "HEAD"])
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);

    let tracked_files = run_git(root, &["ls-files"])
        .map(|value| value.lines().count())
        .unwrap_or(0);

    let status = run_git(root, &["status", "--porcelain"]).unwrap_or_default();

    let mut modified = 0;
    let mut staged = 0;
    let mut untracked = 0;

    for line in status.lines() {
        if line.starts_with("??") {
            untracked += 1;
            continue;
        }

        let bytes = line.as_bytes();

        if bytes.first().copied().unwrap_or(b' ') != b' ' {
            staged += 1;
        }

        if bytes.get(1).copied().unwrap_or(b' ') != b' ' {
            modified += 1;
        }
    }

    let remote = run_git(root, &["remote", "get-url", "origin"]).unwrap_or_default();

    let latest_commit = run_git(
        root,
        &["log", "-1", "--pretty=format:%h %ad %s", "--date=short"],
    )
    .unwrap_or_default();

    GitInfo {
        available: true,
        branch,
        commit_count,
        tracked_files,
        modified,
        staged,
        untracked,
        remote,
        latest_commit,
    }
}

fn human_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];

    let mut value = bytes as f64;
    let mut unit_index = 0;

    while value >= 1024.0 && unit_index < UNITS.len() - 1 {
        value /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.2} {}", UNITS[unit_index])
    }
}

fn project_name(root: &Path) -> String {
    root.file_name()
        .and_then(OsStr::to_str)
        .filter(|name| !name.is_empty())
        .unwrap_or(".")
        .to_string()
}

fn print_header(title: &str) {
    println!();
    println!("{title}");
    println!("{}", "-".repeat(title.chars().count()));
}

fn print_overview(repository: &Repository) {
    println!("Repository Analysis");
    println!("===================");
    println!();
    println!("Name          : {}", project_name(&repository.root));
    println!("Root          : {}", repository.root.display());
    println!("Directories   : {}", repository.directories.len());
    println!("Files         : {}", repository.files.len());
    println!("Total lines   : {}", repository.total_lines);
    println!("Total size    : {}", human_size(repository.total_size));

    let deepest = repository
        .files
        .iter()
        .map(|file| file.relative_path.components().count())
        .max()
        .unwrap_or(0);

    println!("Deepest path  : {deepest} levels");
}

fn print_git(git: &GitInfo) {
    print_header("Git");

    if !git.available {
        println!("No Git repository detected.");
        return;
    }

    println!(
        "Branch        : {}",
        if git.branch.is_empty() {
            "(detached or unknown)"
        } else {
            &git.branch
        }
    );
    println!("Commits       : {}", git.commit_count);
    println!("Tracked files : {}", git.tracked_files);
    println!("Staged        : {}", git.staged);
    println!("Modified      : {}", git.modified);
    println!("Untracked     : {}", git.untracked);

    if !git.remote.is_empty() {
        println!("Origin        : {}", git.remote);
    }

    if !git.latest_commit.is_empty() {
        println!("Latest commit : {}", git.latest_commit);
    }
}

fn print_language_makeup(repository: &Repository) {
    let mut totals: BTreeMap<Language, (usize, usize, u64)> = BTreeMap::new();

    for file in &repository.files {
        let entry = totals.entry(file.language).or_insert((0, 0, 0));
        entry.0 += 1;
        entry.1 += file.lines;
        entry.2 += file.size;
    }

    let mut rows = totals.into_iter().collect::<Vec<_>>();
    rows.sort_by(|left, right| right.1.1.cmp(&left.1.1));

    print_header("Language Makeup");
    println!(
        "{:<16} {:>8} {:>12} {:>12}",
        "Language", "Files", "Lines", "Size"
    );

    for (language, (files, lines, size)) in rows {
        println!(
            "{:<16} {:>8} {:>12} {:>12}",
            language.name(),
            files,
            lines,
            human_size(size)
        );
    }
}

fn print_largest_files(repository: &Repository) {
    let mut files = repository.files.iter().collect::<Vec<_>>();
    files.sort_by(|left, right| right.size.cmp(&left.size));

    print_header("Largest Files");
    println!(
        "{:<12} {:>10}  {}",
        "Size", "Lines", "Path"
    );

    for file in files.into_iter().take(LARGEST_FILE_COUNT) {
        println!(
            "{:<12} {:>10}  {}",
            human_size(file.size),
            file.lines,
            file.relative_path.display()
        );
    }
}

fn print_top_directories(repository: &Repository) {
    let mut stats: HashMap<String, (usize, usize, u64)> = HashMap::new();

    for file in &repository.files {
        let top = file
            .relative_path
            .components()
            .next()
            .map(|component| component.as_os_str().to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string());

        let entry = stats.entry(top).or_insert((0, 0, 0));
        entry.0 += 1;
        entry.1 += file.lines;
        entry.2 += file.size;
    }

    let mut rows = stats.into_iter().collect::<Vec<_>>();
    rows.sort_by(|left, right| right.1.2.cmp(&left.1.2));

    print_header("Top-Level Makeup");
    println!(
        "{:<28} {:>8} {:>12} {:>12}",
        "Directory", "Files", "Lines", "Size"
    );

    for (directory, (files, lines, size)) in rows {
        println!(
            "{:<28} {:>8} {:>12} {:>12}",
            directory,
            files,
            lines,
            human_size(size)
        );
    }
}

fn print_latex_stats(stats: &LatexStats) {
    let total = stats.parts
        + stats.chapters
        + stats.sections
        + stats.subsections
        + stats.subsubsections
        + stats.definitions
        + stats.theorems
        + stats.lemmas
        + stats.propositions
        + stats.corollaries
        + stats.proofs
        + stats.examples
        + stats.remarks
        + stats.figures
        + stats.tables
        + stats.equations
        + stats.citations;

    if total == 0 {
        return;
    }

    print_header("LaTeX Structure");
    println!("Parts          : {}", stats.parts);
    println!("Chapters       : {}", stats.chapters);
    println!("Sections       : {}", stats.sections);
    println!("Subsections    : {}", stats.subsections);
    println!("Subsubsections : {}", stats.subsubsections);
    println!("Definitions    : {}", stats.definitions);
    println!("Theorems       : {}", stats.theorems);
    println!("Lemmas         : {}", stats.lemmas);
    println!("Propositions   : {}", stats.propositions);
    println!("Corollaries    : {}", stats.corollaries);
    println!("Proofs         : {}", stats.proofs);
    println!("Examples       : {}", stats.examples);
    println!("Remarks        : {}", stats.remarks);
    println!("Figures        : {}", stats.figures);
    println!("Tables         : {}", stats.tables);
    println!("Equation blocks: {}", stats.equations);
    println!("Citation calls : {}", stats.citations);
}

fn print_rust_stats(stats: &RustStats) {
    let total = stats.functions
        + stats.structs
        + stats.enums
        + stats.traits
        + stats.impl_blocks
        + stats.modules
        + stats.tests
        + stats.macros
        + stats.unsafe_uses;

    if total == 0 {
        return;
    }

    print_header("Rust Structure");
    println!("Functions        : {}", stats.functions);
    println!("Public functions : {}", stats.public_functions);
    println!("Structs          : {}", stats.structs);
    println!("Enums            : {}", stats.enums);
    println!("Traits           : {}", stats.traits);
    println!("Impl blocks      : {}", stats.impl_blocks);
    println!("Modules          : {}", stats.modules);
    println!("Tests            : {}", stats.tests);
    println!("Macros           : {}", stats.macros);
    println!("Unsafe uses      : {}", stats.unsafe_uses);
}

fn print_tree(repository: &Repository, max_depth: usize) {
    let mut paths = Vec::new();

    for directory in &repository.directories {
        paths.push((directory.clone(), true));
    }

    for file in &repository.files {
        paths.push((file.relative_path.clone(), false));
    }

    paths.sort_by(|left, right| left.0.cmp(&right.0));

    print_header(&format!("Repository Tree (depth ≤ {max_depth})"));
    println!("{}/", project_name(&repository.root));

    for (path, is_directory) in paths {
        let depth = path.components().count();

        if depth == 0 || depth > max_depth {
            continue;
        }

        let name = path
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("?");

        let indent = "  ".repeat(depth);

        if is_directory {
            println!("{indent}{name}/");
        } else {
            println!("{indent}{name}");
        }
    }
}

fn detect_project_features(repository: &Repository) -> Vec<String> {
    let mut features = Vec::new();

    let has_file = |name: &str| {
        repository
            .files
            .iter()
            .any(|file| file.relative_path == PathBuf::from(name))
    };

    let has_extension = |extension: &str| {
        repository.files.iter().any(|file| {
            file.relative_path
                .extension()
                .and_then(OsStr::to_str)
                == Some(extension)
        })
    };

    if has_file("Cargo.toml") {
        features.push("Rust Cargo project".to_string());
    }

    if has_file("package.json") {
        features.push("Node.js project".to_string());
    }

    if has_file("pyproject.toml")
        || has_file("requirements.txt")
        || has_file("setup.py")
    {
        features.push("Python project".to_string());
    }

    if has_file("CMakeLists.txt") {
        features.push("CMake project".to_string());
    }

    if has_file("Makefile") {
        features.push("Make-based build".to_string());
    }

    if has_file("Dockerfile") || has_file("docker-compose.yml") {
        features.push("Containerized project".to_string());
    }

    if has_file("README.md") || has_file("README") {
        features.push("Contains project documentation".to_string());
    }

    if has_extension("tex") {
        features.push("Contains LaTeX manuscripts".to_string());
    }

    if repository
        .directories
        .iter()
        .any(|directory| directory == Path::new("tests"))
    {
        features.push("Contains a tests directory".to_string());
    }

    if repository
        .directories
        .iter()
        .any(|directory| directory == Path::new(".github"))
    {
        features.push("Contains GitHub configuration".to_string());
    }

    features
}

fn print_project_fingerprint(repository: &Repository) {
    let mut language_lines: HashMap<Language, usize> = HashMap::new();

    for file in &repository.files {
        *language_lines.entry(file.language).or_insert(0) += file.lines;
    }

    let primary_language = language_lines
        .iter()
        .max_by_key(|(_, lines)| *lines)
        .map(|(language, _)| language.name())
        .unwrap_or("Unknown");

    let documentation_lines = language_lines
        .get(&Language::Markdown)
        .copied()
        .unwrap_or(0)
        + language_lines
            .get(&Language::Latex)
            .copied()
            .unwrap_or(0)
        + language_lines
            .get(&Language::Text)
            .copied()
            .unwrap_or(0);

    let documentation_ratio = if repository.total_lines == 0 {
        0.0
    } else {
        documentation_lines as f64 / repository.total_lines as f64 * 100.0
    };

    print_header("Project Fingerprint");
    println!("Primary language    : {primary_language}");
    println!("Documentation ratio : {documentation_ratio:.1}%");

    let features = detect_project_features(repository);

    if features.is_empty() {
        println!("Detected features   : none");
    } else {
        println!("Detected features   :");

        for feature in features {
            println!("  - {feature}");
        }
    }
}

fn main() {
    let config = match parse_args() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Error: {error}");
            eprintln!("Run with --help for usage.");
            std::process::exit(2);
        }
    };

    let root = match fs::canonicalize(&config.root) {
        Ok(root) => root,
        Err(error) => {
            eprintln!(
                "Error: cannot access {}: {error}",
                config.root.display()
            );
            std::process::exit(1);
        }
    };

    if !root.is_dir() {
        eprintln!("Error: {} is not a directory.", root.display());
        std::process::exit(1);
    }

    let mut repository = Repository::new(root.clone());

    if let Err(error) = walk_repository(
        &root,
        &root,
        &mut repository,
        config.show_hidden,
    ) {
        eprintln!("Error while scanning repository: {error}");
        std::process::exit(1);
    }

    repository.files.sort_by(|left, right| {
        left.relative_path.cmp(&right.relative_path)
    });
    repository.directories.sort();
    repository.git = analyze_git(&root);

    print_overview(&repository);
    print_git(&repository.git);
    print_project_fingerprint(&repository);
    print_language_makeup(&repository);
    print_top_directories(&repository);
    print_largest_files(&repository);
    print_latex_stats(&repository.latex);
    print_rust_stats(&repository.rust);
    print_tree(&repository, config.tree_depth);
}
