use dashmap::DashMap;
use regex::Regex;
use std::sync::OnceLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

// =============================================================================
// Architecture completion values
// Sourced from RPM architecture identifiers; these are standardised identifiers
// not subject to copyright.
// =============================================================================

static ARCH_VALUES: &[&str] = &[
    "aarch64", "alpha", "alphaev5", "alphaev56", "alphaev6", "alphaev67", "alphapca56",
    "amd64", "armv3l", "armv4b", "armv4l", "armv5tejl", "armv5tel", "armv5tl", "armv6hl",
    "armv6l", "armv7hl", "armv7hnl", "armv7l", "armv8hl", "armv8l", "athlon", "em64t",
    "geode", "i370", "i386", "i486", "i586", "i686", "ia32e", "ia64", "loongarch64", "m68k",
    "m68kmint", "mips", "mips64", "mips64el", "mips64r6", "mips64r6el", "mipsel", "mipsr6",
    "mipsr6el", "pentium3", "pentium4", "ppc", "ppc32dy4", "ppc64", "ppc64iseries", "ppc64le",
    "ppc64p7", "ppc64pseries", "ppc8260", "ppc8560", "ppciseries", "ppcpseries", "riscv64",
    "rs6000", "s390", "s390x", "sgi", "sh", "sh3", "sh4", "sh4a", "sparc", "sparc64",
    "sparc64v", "sparcv8", "sparcv9", "sparcv9v", "x86_64", "x86_64_v2", "x86_64_v3",
    "x86_64_v4", "xtensa",
];

static ARCH_SHORTCUTS: &[&str] = &[
    "alpha", "arm", "arm32", "arm64", "ix86", "loongarch64", "mips", "mips32", "mips64",
    "mipseb", "mipsel", "power64", "riscv128", "riscv32", "sparc", "x86_64",
];

// =============================================================================
// Section / scriptlet / condition keywords for completion
// =============================================================================

static SECTION_KEYWORDS: &[&str] = &[
    "%package",
    "%description",
    "%prep",
    "%build",
    "%install",
    "%check",
    "%files",
    "%changelog",
    "%pre",
    "%post",
    "%preun",
    "%postun",
    "%pretrans",
    "%posttrans",
    "%preuntrans",
    "%postuntrans",
    "%verifyscript",
    "%triggerprein",
    "%triggerin",
    "%triggerun",
    "%triggerpostun",
    "%filetriggerin",
    "%filetriggerun",
    "%filetriggerpostun",
    "%transfiletriggerin",
    "%transfiletriggerun",
    "%transfiletriggerpostun",
    "%end",
];

static SCRIPTLET_MACROS: &[&str] = &[
    "%setup",
    "%autosetup",
    "%autopatch",
    "%patch",
    "%make_build",
    "%make_install",
    "%configure",
    "%cmake",
    "%cmake_build",
    "%cmake_install",
    "%meson",
    "%meson_build",
    "%meson_install",
    "%ninja_build",
    "%ninja_install",
    "%pyproject_buildrequires",
    "%pyproject_wheel",
    "%pyproject_install",
    "%gem_install",
    "%mvn_build",
    "%mvn_install",
    "%cargo_build",
    "%cargo_install",
    "%cargo_test",
    "%install_info",
    "%find_lang",
    "%doc",
    "%license",
    "%dir",
    "%ghost",
    "%config",
    "%attr",
    "%defattr",
    "%exclude",
    "%verify",
];

static CONDITION_KEYWORDS: &[&str] = &[
    "%if",
    "%ifarch",
    "%ifnarch",
    "%ifos",
    "%ifnos",
    "%elif",
    "%elifarch",
    "%elifos",
    "%else",
    "%endif",
    "%include",
    "%global",
    "%define",
    "%undefine",
    "%bcond_with",
    "%bcond_without",
    "%bcond",
];

// =============================================================================
// Preamble tag hover descriptions
// Written independently; factual content sourced from the RPM Spec documentation
// at https://rpm-software-management.github.io/rpm/manual/spec.html
// No text was copied verbatim.
// =============================================================================

fn preamble_tag_docs(tag: &str) -> Option<&'static str> {
    match tag {
        "Name" => Some(
            "The proper name of the package. Must not include whitespace. \
             A hyphen `-` is allowed (unlike `Version` and `Release`). \
             Avoid numeric comparison operators (`<`, `>`, `=`).",
        ),
        "Version" => Some(
            "The version of the packaged software. Made up of alphanumeric characters \
             optionally separated by `.`, `_`, or `+`. \
             Use `~` to sort lower than base (e.g. `1.0~rc1 < 1.0`) \
             and `^` to sort higher (e.g. `2.0^a > 2.0`).",
        ),
        "Release" => Some(
            "Distinguishes between different builds of the same software version. \
             Follows the same character and modifier rules as `Version`.",
        ),
        "Epoch" => Some(
            "Optional numeric value that overrides the normal version/release sort order. \
             Avoid using it if at all possible. Absent epoch is equal to epoch `0`.",
        ),
        "License" => Some(
            "Short (< 70 chars) SPDX expression or identifier for the package license, \
             e.g. `Apache-2.0` or `GPL-2.0-only`.",
        ),
        "SourceLicense" => Some(
            "License of the sources when it differs from the main package license. \
             If omitted, the source package shares the `License` tag value.",
        ),
        "Summary" => Some("Short (< 70 chars) one-line summary of the package."),
        "Group" => Some(
            "Optional short classification group for the package, \
             e.g. `Development/Libraries`.",
        ),
        "URL" => Some("URL with further information about the package, typically the upstream website."),
        "BugURL" => Some("URL for reporting bugs in the package."),
        "Source" | "Source0" => Some(
            "Declares the primary source archive used to build the package. \
             All sources are included in source RPMs. Additional sources use \
             `Source1:`, `Source2:`, etc. Numbers need not be consecutive.",
        ),
        "Patch" | "Patch0" => Some(
            "Declares a patch applied on top of the sources. All patches are \
             included in source RPMs. For new packages, prefer unnumbered patches \
             applied via `%autosetup` or `%autopatch`.",
        ),
        "BuildRequires" => Some(
            "Capabilities required to build the package. Resolved before the build \
             starts rather than at install time. \
             Example: `BuildRequires: gcc >= 10`",
        ),
        "BuildConflicts" => Some(
            "Capabilities that must NOT be installed during the package build. \
             Example: `BuildConflicts: somelib-devel`",
        ),
        "Requires" => Some(
            "Capabilities this package requires to function at install time. \
             Use `Requires(pre)`, `Requires(post)`, `Requires(preun)`, \
             `Requires(postun)`, etc. to scope requirements to a scriptlet phase.",
        ),
        "Provides" => Some(
            "Capabilities provided by this package. \
             `name = [epoch:]version-release` is added automatically.",
        ),
        "Conflicts" => Some(
            "Capabilities this package conflicts with — typically packages \
             that have conflicting paths or functionality.",
        ),
        "Obsoletes" => Some(
            "Packages that this package replaces or renames. \
             RPM will remove obsoleted packages when installing this one.",
        ),
        "Recommends" => Some(
            "Weak dependency: installed alongside this package if available, \
             but not required. (since rpm >= 4.13)",
        ),
        "Suggests" => Some(
            "Weaker than `Recommends`: a hint that the listed package may be useful, \
             but it is neither required nor recommended for automatic installation.",
        ),
        "Supplements" => Some(
            "Reverse weak dependency: indicates this package extends or supplements \
             the listed package.",
        ),
        "Enhances" => Some(
            "Reverse `Suggests`: hints that this package enhances the listed package.",
        ),
        "BuildArch" | "BuildArchitectures" => Some(
            "Architecture the resulting binary package targets. \
             Use `noarch` for platform-independent packages such as scripts or \
             pure documentation. On sub-packages, `noarch` allows arch-specific \
             parent packages to ship architecture-neutral sub-packages.",
        ),
        "ExcludeArch" => Some(
            "Architectures on which this package cannot be built, \
             e.g. due to endianness issues or missing platform support.",
        ),
        "ExclusiveArch" => Some(
            "Architectures on which this package can only be built. \
             All other architectures are implicitly excluded.",
        ),
        "ExcludeOS" => Some("Operating systems on which this package cannot be built."),
        "ExclusiveOS" => Some("Operating systems on which this package can only be built."),
        "Vendor" => Some(
            "Optional vendor/distributor name. \
             Typically filled in automatically by build system macros.",
        ),
        "Packager" => Some(
            "Optional maintainer name or contact information. \
             Typically filled in automatically by build system macros.",
        ),
        "Distribution" => Some(
            "Optional distribution name. \
             Typically filled in automatically by build system macros.",
        ),
        "Buildsystem" => Some(
            "Automatically populates the build scriptlets for a named build system, \
             e.g. `Buildsystem: autotools`. See the declarative build documentation.",
        ),
        "AutoReq" => Some(
            "Controls automatic dependency generation for `Requires`. \
             Accepted values: `1`/`0` or `yes`/`no`. Default is `yes`.",
        ),
        "AutoProv" => Some(
            "Controls automatic dependency generation for `Provides`. \
             Accepted values: `1`/`0` or `yes`/`no`. Default is `yes`.",
        ),
        "AutoReqProv" => Some(
            "Controls automatic dependency generation for both `Requires` and `Provides`. \
             Equivalent to setting `AutoReq` and `AutoProv` individually.",
        ),
        _ => None,
    }
}

// =============================================================================
// Built-in RPM macro hover descriptions
// =============================================================================

fn builtin_macro_docs(name: &str) -> Option<&'static str> {
    match name {
        "name" => Some("Expands to the package `Name` tag value."),
        "version" => Some("Expands to the package `Version` tag value."),
        "release" => Some("Expands to the package `Release` tag value."),
        "epoch" => Some("Expands to the package `Epoch` tag value (empty string if unset)."),
        "summary" => Some("Expands to the package `Summary` tag value."),
        "license" => Some("Expands to the package `License` tag value."),
        "url" => Some("Expands to the package `URL` tag value."),
        "buildroot" => Some(
            "The root directory used during the install phase. \
             All `%install` scriptlet actions should install under this path.",
        ),
        "_prefix" => Some("Installation prefix, typically `/usr`."),
        "_exec_prefix" => Some("Exec prefix, typically `/usr`."),
        "_bindir" => Some("Directory for user-executable binaries, typically `/usr/bin`."),
        "_sbindir" => Some("Directory for system-admin binaries, typically `/usr/sbin`."),
        "_libexecdir" => Some(
            "Directory for program executables launched by other programs, \
             typically `/usr/libexec`.",
        ),
        "_libdir" => Some(
            "Directory for object code libraries, typically `/usr/lib` or `/usr/lib64`.",
        ),
        "_includedir" => Some("Directory for C header files, typically `/usr/include`."),
        "_datadir" => Some(
            "Read-only architecture-independent data, typically `/usr/share`.",
        ),
        "_datarootdir" => Some("Data root directory, typically `/usr/share`."),
        "_mandir" => Some("Manual page directory, typically `/usr/share/man`."),
        "_infodir" => Some("GNU Info page directory, typically `/usr/share/info`."),
        "_docdir" => Some("Documentation directory, typically `/usr/share/doc`."),
        "_sysconfdir" => Some("System configuration directory, typically `/etc`."),
        "_localstatedir" => Some(
            "Persistent local state data directory, typically `/var`.",
        ),
        "_sharedstatedir" => Some(
            "Architecture-independent modifiable data, typically `/var/lib`.",
        ),
        "_rundir" => Some("Runtime data directory, typically `/run`."),
        "_tmppath" => Some("Temporary directory path used during builds."),
        "_builddir" => Some("The build directory, typically `%{_topdir}/BUILD`."),
        "_sourcedir" => Some("The sources directory, typically `%{_topdir}/SOURCES`."),
        "_specdir" => Some("The spec files directory, typically `%{_topdir}/SPECS`."),
        "_rpmdir" => Some(
            "Output directory for built RPM packages, typically `%{_topdir}/RPMS`.",
        ),
        "_srcrpmdir" => Some(
            "Output directory for built SRPM packages, typically `%{_topdir}/SRPMS`.",
        ),
        "_topdir" => Some("Top-level RPM build directory, typically `~/rpmbuild`."),
        "_rpmconfigdir" => Some("RPM configuration directory, typically `/usr/lib/rpm`."),
        "_rpmmacrodir" => Some(
            "RPM macros directory, typically `/usr/lib/rpm/macros.d`.",
        ),
        "_unitdir" => Some(
            "systemd unit file directory, typically `/usr/lib/systemd/system`.",
        ),
        "_userunitdir" => Some(
            "systemd user unit file directory, typically `/usr/lib/systemd/user`.",
        ),
        "_udevrulesdir" => Some(
            "udev rules directory, typically `/usr/lib/udev/rules.d`.",
        ),
        "optflags" => Some(
            "Compiler optimisation flags for the target architecture \
             (e.g. `-O2 -g -Wall`).",
        ),
        "make_build" => Some(
            "Runs `make` with the appropriate parallel job flags (`-j$(nproc)`).",
        ),
        "make_install" => Some("Runs `make install DESTDIR=%{buildroot}`."),
        "configure" => Some(
            "Runs `./configure` with standard RPM directory layout arguments.",
        ),
        "python3" => Some("Path to the Python 3 interpreter."),
        "python3_sitelib" => Some("Python 3 pure-module site-packages directory."),
        "python3_sitearch" => Some(
            "Python 3 architecture-specific site-packages directory.",
        ),
        "python3_version" => Some(
            "Python 3 major.minor version string (e.g. `3.12`).",
        ),
        "_arch" => Some(
            "The build target architecture (e.g. `x86_64`, `aarch64`).",
        ),
        "nil" => Some(
            "Always expands to the empty string. \
             Useful for conditional empty values.",
        ),
        _ => None,
    }
}

// =============================================================================
// Regex helpers (compiled once)
// =============================================================================

fn re_global_define() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?im)^%(?:global|define)\s+(\S+)\s+(.*)")
            .expect("invalid global/define regex")
    })
}

fn re_macro_ref() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"%\{([^}\s]+)\}").expect("invalid macro ref regex")
    })
}

fn re_undefine() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?im)^%undefine\s+(\S+)").expect("invalid undefine regex")
    })
}

/// Matches the first word of any spec section header line (e.g. `%prep`, `%package`).
fn re_section_header() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)^%(prep|build|install|check|files|changelog|pre|post|preun|postun|\
pretrans|posttrans|preuntrans|postuntrans|verifyscript|\
triggerprein|triggerin|triggerun|triggerpostun|\
filetriggerin|filetriggerun|filetriggerpostun|\
transfiletriggerin|transfiletriggerun|transfiletriggerpostun|\
package|description|end)\b",
        )
        .expect("invalid section header regex")
    })
}

// =============================================================================
// Backend
// =============================================================================

struct Backend {
    client: Client,
    /// Maps document URI strings to their full text content.
    documents: DashMap<String, String>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Backend {
            client,
            documents: DashMap::new(),
        }
    }

    /// Returns the 0-based byte offset in `text` for the given LSP position.
    fn offset_of(text: &str, position: Position) -> Option<usize> {
        let mut line = 0u32;
        let mut col = 0u32;
        for (idx, ch) in text.char_indices() {
            if line == position.line && col == position.character {
                return Some(idx);
            }
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }
        // Handle position at very end of file
        if line == position.line && col == position.character {
            return Some(text.len());
        }
        None
    }

    /// Converts a 0-based byte offset in `text` to an LSP Position.
    fn position_of(text: &str, offset: usize) -> Position {
        let mut line = 0u32;
        let mut last_newline = 0usize;
        for (idx, ch) in text.char_indices() {
            if idx >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                last_newline = idx + 1;
            }
        }
        let character = text[last_newline..offset].chars().count() as u32;
        Position { line, character }
    }

    /// Extracts the word at `position` in `text`, restricted to `[A-Za-z0-9_-]`.
    fn word_at(text: &str, position: Position) -> Option<(String, usize, usize)> {
        let offset = Self::offset_of(text, position)?;
        let bytes = text.as_bytes();
        let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
        let start = (0..offset)
            .rev()
            .find(|&i| !is_word(bytes[i]))
            .map(|i| i + 1)
            .unwrap_or(0);
        let end = (offset..bytes.len())
            .find(|&i| !is_word(bytes[i]))
            .unwrap_or(bytes.len());
        if start >= end {
            return None;
        }
        Some((text[start..end].to_string(), start, end))
    }

    /// Returns all `(varName, value, lineNumber)` definitions from %global/%define.
    fn collect_definitions(text: &str) -> Vec<(String, String, u32)> {
        let mut defs = Vec::new();
        for cap in re_global_define().captures_iter(text) {
            let var = cap[1].to_string();
            let val = cap[2].trim().to_string();
            // Compute line number of the match
            let mat = cap.get(0).unwrap();
            let line = text[..mat.start()].chars().filter(|&c| c == '\n').count() as u32;
            defs.push((var, val, line));
        }
        defs
    }

    fn arch_completions() -> Vec<CompletionItem> {
        let mut items = Vec::with_capacity(ARCH_VALUES.len() + ARCH_SHORTCUTS.len());
        for &arch in ARCH_VALUES {
            items.push(CompletionItem {
                label: arch.to_string(),
                kind: Some(CompletionItemKind::VALUE),
                ..Default::default()
            });
        }
        for &shortcut in ARCH_SHORTCUTS {
            let label = format!("%{{{shortcut}}}");
            items.push(CompletionItem {
                label,
                kind: Some(CompletionItemKind::CONSTANT),
                ..Default::default()
            });
        }
        items
    }

    /// Returns `true` if `line_idx` is within the preamble or a `%package` /
    /// `%description` sub-section — i.e. where preamble tag completions apply.
    fn cursor_in_preamble(doc: &str, line_idx: u32) -> bool {
        for (i, line) in doc.lines().enumerate() {
            if i as u32 >= line_idx {
                break;
            }
            let t = line.trim_start();
            if re_section_header().is_match(t) {
                let lower = t.to_ascii_lowercase();
                if !lower.starts_with("%package") && !lower.starts_with("%description") {
                    return false;
                }
            }
        }
        true
    }

    /// Completion items for preamble tags, with documentation and insert text.
    fn preamble_tag_items() -> Vec<CompletionItem> {
        static TAG_NAMES: &[&str] = &[
            "Name", "Version", "Release", "Epoch", "License", "SourceLicense",
            "Summary", "Group", "URL", "BugURL",
            "Source", "Source0", "Source1", "Source2",
            "Patch", "Patch0", "Patch1",
            "BuildRequires", "BuildConflicts",
            "Requires", "Provides", "Conflicts", "Obsoletes",
            "Recommends", "Suggests", "Supplements", "Enhances",
            "BuildArch", "BuildArchitectures",
            "ExcludeArch", "ExclusiveArch", "ExcludeOS", "ExclusiveOS",
            "Vendor", "Packager", "Distribution", "Buildsystem",
            "AutoReq", "AutoProv", "AutoReqProv",
        ];
        TAG_NAMES
            .iter()
            .map(|&label| {
                let base = label.trim_end_matches(|c: char| c.is_ascii_digit());
                let lookup = if base.is_empty() { label } else { base };
                let doc = preamble_tag_docs(lookup);
                CompletionItem {
                    label: label.to_string(),
                    kind: Some(CompletionItemKind::FIELD),
                    documentation: doc.map(|d| {
                        Documentation::MarkupContent(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: d.to_string(),
                        })
                    }),
                    insert_text: Some(format!("{label}: ")),
                    ..Default::default()
                }
            })
            .collect()
    }

    /// Completion items for section headers, scriptlet macros, and condition keywords.
    fn keyword_items() -> Vec<CompletionItem> {
        let mut items = Vec::new();
        for &kw in SECTION_KEYWORDS {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::MODULE),
                ..Default::default()
            });
        }
        for &kw in SCRIPTLET_MACROS {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                ..Default::default()
            });
        }
        for &kw in CONDITION_KEYWORDS {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                ..Default::default()
            });
        }
        items
    }

    /// Completion items for user-defined macros (`%global` / `%define`).
    fn user_macro_items(doc: &str) -> Vec<CompletionItem> {
        Self::collect_definitions(doc)
            .into_iter()
            .map(|(name, val, _)| {
                let label = format!("%{{{name}}}");
                let detail = if val.is_empty() { None } else { Some(val) };
                CompletionItem {
                    label,
                    kind: Some(CompletionItemKind::VARIABLE),
                    detail,
                    ..Default::default()
                }
            })
            .collect()
    }

    /// Parse the spec document into `(section_name, start_line, end_line)` triples.
    fn parse_sections(doc: &str) -> Vec<(String, u32, u32)> {
        let lines: Vec<&str> = doc.lines().collect();
        let total = lines.len() as u32;
        // Collect (name, start_line) pairs
        let mut headers: Vec<(String, u32)> = Vec::new();
        let mut found_first = false;
        for (i, &line) in lines.iter().enumerate() {
            let t = line.trim_start();
            if re_section_header().is_match(t) {
                if !found_first && i > 0 {
                    // There are preamble lines before the first section header
                    headers.push(("preamble".to_string(), 0));
                }
                found_first = true;
                let name = t.split_whitespace().next().unwrap_or(t).to_string();
                headers.push((name, i as u32));
            }
        }
        if !found_first && !lines.is_empty() {
            headers.push(("preamble".to_string(), 0));
        }
        // Convert to (name, start, end) triples
        let mut result = Vec::new();
        for i in 0..headers.len() {
            let (ref name, start) = headers[i];
            let end = headers.get(i + 1).map(|h| h.1).unwrap_or(total);
            result.push((name.clone(), start, end));
        }
        result
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![" ".to_string(), "%".to_string()]),
                    resolve_provider: Some(false),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "rpm-spec-lsp initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.documents.insert(
            params.text_document.uri.to_string(),
            params.text_document.text,
        );
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            self.documents
                .insert(params.text_document.uri.to_string(), change.text);
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.remove(&params.text_document.uri.to_string());
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri.to_string();
        let position = params.text_document_position.position;

        let doc = match self.documents.get(&uri) {
            Some(d) => d.clone(),
            None => return Ok(None),
        };

        let lines: Vec<&str> = doc.lines().collect();
        let line = match lines.get(position.line as usize) {
            Some(l) => *l,
            None => return Ok(None),
        };

        let trimmed = line.trim_start();
        let trigger = params
            .context
            .as_ref()
            .and_then(|c| c.trigger_character.as_deref());

        // Space trigger: arch completions after %ifarch / %ifnarch only
        if trigger == Some(" ") {
            if trimmed.starts_with("%ifarch") || trimmed.starts_with("%ifnarch") {
                return Ok(Some(CompletionResponse::Array(Self::arch_completions())));
            }
            return Ok(None);
        }

        let in_preamble = Self::cursor_in_preamble(&doc, position.line);

        match trigger {
            // `%` trigger: keywords + user macros
            Some("%") => {
                let mut items = Self::keyword_items();
                items.extend(Self::user_macro_items(&doc));
                Ok(Some(CompletionResponse::Array(items)))
            }
            // Manual invocation (no trigger): everything appropriate for context
            None => {
                let mut items = Self::keyword_items();
                items.extend(Self::user_macro_items(&doc));
                if in_preamble {
                    items.extend(Self::preamble_tag_items());
                }
                Ok(Some(CompletionResponse::Array(items)))
            }
            // Any other trigger character: no completion
            _ => Ok(None),
        }
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let position = params.text_document_position_params.position;

        let doc = match self.documents.get(&uri) {
            Some(d) => d.clone(),
            None => return Ok(None),
        };

        let line_text = doc.lines().nth(position.line as usize).unwrap_or("");

        // --- Preamble tag hover ---
        // Only fire when the word starts at column 0 (tag names are at line start).
        if let Some((word, start, end)) = Self::word_at(&doc, position) {
            if line_text.starts_with(word.as_str()) {
                // Normalise numbered variants: Source1 → Source, Patch3 → Patch
                let base = word.trim_end_matches(|c: char| c.is_ascii_digit());
                let key = if base.is_empty() { word.as_str() } else { base };
                if let Some(desc) = preamble_tag_docs(key) {
                    let start_pos = Self::position_of(&doc, start);
                    let end_pos = Self::position_of(&doc, end);
                    return Ok(Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: format!("### `{word}`\n\n{desc}"),
                        }),
                        range: Some(Range {
                            start: start_pos,
                            end: end_pos,
                        }),
                    }));
                }
            }
        }

        // --- User-defined macro hover: cursor inside %{varName} or on a definition ---
        if let Some((word, start, end)) = Self::word_at(&doc, position) {
            // Bug fix: use `start` (byte start of the word) not the cursor offset
            // so that the two bytes before the word are reliably checked for `%{`.
            let is_macro_ref = start >= 2
                && doc.as_bytes().get(start - 2) == Some(&b'%')
                && doc.as_bytes().get(start - 1) == Some(&b'{');

            let is_global_def = line_text.trim_start().starts_with("%global")
                || line_text.trim_start().starts_with("%define");

            if is_macro_ref || is_global_def {
                let defs = Self::collect_definitions(&doc);
                let values: Vec<&str> = defs
                    .iter()
                    .filter(|(name, val, _)| name == &word && !val.is_empty())
                    .map(|(_, val, _)| val.as_str())
                    .collect();

                if !values.is_empty() {
                    let body = if values.len() == 1 {
                        format!("```\n{word}: {}\n```", values[0])
                    } else {
                        format!(
                            "```\n{}:\n  - {}\n```",
                            word,
                            values.join("\n  - ")
                        )
                    };
                    let start_pos = Self::position_of(&doc, start);
                    let end_pos = Self::position_of(&doc, end);
                    return Ok(Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: body,
                        }),
                        range: Some(Range {
                            start: start_pos,
                            end: end_pos,
                        }),
                    }));
                }

                // --- Built-in macro hover fallback ---
                if let Some(desc) = builtin_macro_docs(&word) {
                    let start_pos = Self::position_of(&doc, start);
                    let end_pos = Self::position_of(&doc, end);
                    return Ok(Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: format!("### `%{{{word}}}`\n\n{desc}"),
                        }),
                        range: Some(Range {
                            start: start_pos,
                            end: end_pos,
                        }),
                    }));
                }
            }
        }

        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let position = params.text_document_position_params.position;

        let doc = match self.documents.get(&uri.to_string()) {
            Some(d) => d.clone(),
            None => return Ok(None),
        };

        let (word, _, _) = match Self::word_at(&doc, position) {
            Some(w) => w,
            None => return Ok(None),
        };

        let defs = Self::collect_definitions(&doc);
        let locations: Vec<Location> = defs
            .iter()
            .filter(|(name, _, _)| name == &word)
            .map(|(name, _, line_num)| {
                // Find the column of the variable name on that line
                let line_text = doc.lines().nth(*line_num as usize).unwrap_or("");
                let col = line_text.find(name.as_str()).unwrap_or(0) as u32;
                let end_col = col + name.len() as u32;
                Location {
                    uri: uri.clone(),
                    range: Range {
                        start: Position {
                            line: *line_num,
                            character: col,
                        },
                        end: Position {
                            line: *line_num,
                            character: end_col,
                        },
                    },
                }
            })
            .collect();

        if locations.is_empty() {
            return Ok(None);
        }
        Ok(Some(GotoDefinitionResponse::Array(locations)))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri.clone();
        let position = params.text_document_position.position;

        let doc = match self.documents.get(&uri.to_string()) {
            Some(d) => d.clone(),
            None => return Ok(None),
        };

        let (word, _, _) = match Self::word_at(&doc, position) {
            Some(w) => w,
            None => return Ok(None),
        };

        let mut locations = Vec::new();

        // Find all %{varName} usages
        for cap in re_macro_ref().captures_iter(&doc) {
            if cap[1] != word {
                continue;
            }
            let mat = cap.get(1).unwrap();
            let start_pos = Self::position_of(&doc, mat.start());
            let end_pos = Self::position_of(&doc, mat.end());
            locations.push(Location {
                uri: uri.clone(),
                range: Range {
                    start: start_pos,
                    end: end_pos,
                },
            });
        }

        // Find all %undefine varName usages
        for cap in re_undefine().captures_iter(&doc) {
            if cap[1] != word {
                continue;
            }
            let mat = cap.get(1).unwrap();
            let start_pos = Self::position_of(&doc, mat.start());
            let end_pos = Self::position_of(&doc, mat.end());
            locations.push(Location {
                uri: uri.clone(),
                range: Range {
                    start: start_pos,
                    end: end_pos,
                },
            });
        }

        if locations.is_empty() {
            return Ok(None);
        }
        Ok(Some(locations))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri.to_string();
        let doc = match self.documents.get(&uri) {
            Some(d) => d.clone(),
            None => return Ok(None),
        };

        let sections = Self::parse_sections(&doc);
        let symbols: Vec<DocumentSymbol> = sections
            .into_iter()
            .map(|(name, start_line, end_line)| {
                let range = Range {
                    start: Position { line: start_line, character: 0 },
                    end: Position {
                        line: end_line.saturating_sub(1),
                        character: 0,
                    },
                };
                #[allow(deprecated)]
                DocumentSymbol {
                    name,
                    detail: None,
                    kind: SymbolKind::NAMESPACE,
                    tags: None,
                    deprecated: None,
                    range,
                    selection_range: Range {
                        start: Position { line: start_line, character: 0 },
                        end: Position { line: start_line, character: 0 },
                    },
                    children: None,
                }
            })
            .collect();
        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri.clone();
        let position = params.text_document_position.position;
        let new_name = params.new_name;

        let doc = match self.documents.get(&uri.to_string()) {
            Some(d) => d.clone(),
            None => return Ok(None),
        };

        let (old_name, _, _) = match Self::word_at(&doc, position) {
            Some(w) => w,
            None => return Ok(None),
        };

        // Only rename symbols that have a %global/%define definition
        let defs = Self::collect_definitions(&doc);
        if !defs.iter().any(|(name, _, _)| name == &old_name) {
            return Ok(None);
        }

        let mut edits: Vec<TextEdit> = Vec::new();

        // Rename the definition name on each %global/%define line
        for (name, _, line_num) in &defs {
            if name != &old_name {
                continue;
            }
            let line_text = doc.lines().nth(*line_num as usize).unwrap_or("");
            if let Some(col) = line_text.find(name.as_str()) {
                edits.push(TextEdit {
                    range: Range {
                        start: Position { line: *line_num, character: col as u32 },
                        end: Position {
                            line: *line_num,
                            character: (col + name.len()) as u32,
                        },
                    },
                    new_text: new_name.clone(),
                });
            }
        }

        // Rename all %{oldName} references
        for cap in re_macro_ref().captures_iter(&doc) {
            if &cap[1] != old_name.as_str() {
                continue;
            }
            let mat = cap.get(1).unwrap();
            edits.push(TextEdit {
                range: Range {
                    start: Self::position_of(&doc, mat.start()),
                    end: Self::position_of(&doc, mat.end()),
                },
                new_text: new_name.clone(),
            });
        }

        // Rename all %undefine oldName occurrences
        for cap in re_undefine().captures_iter(&doc) {
            if &cap[1] != old_name.as_str() {
                continue;
            }
            let mat = cap.get(1).unwrap();
            edits.push(TextEdit {
                range: Range {
                    start: Self::position_of(&doc, mat.start()),
                    end: Self::position_of(&doc, mat.end()),
                },
                new_text: new_name.clone(),
            });
        }

        if edits.is_empty() {
            return Ok(None);
        }

        let mut changes = std::collections::HashMap::new();
        changes.insert(uri, edits);
        Ok(Some(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        }))
    }
}

// =============================================================================
// Entry point
// =============================================================================

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
