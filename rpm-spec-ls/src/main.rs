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
        let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b'-';
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
                    trigger_characters: Some(vec![" ".to_string()]),
                    resolve_provider: Some(false),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "rpm-spec-ls initialized")
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
        let prev_char = if position.character > 0 {
            line.chars().nth(position.character as usize - 1)
        } else {
            None
        };

        // Offer arch completions after %ifarch / %ifnarch followed by a space
        if (trimmed.starts_with("%ifarch") || trimmed.starts_with("%ifnarch"))
            && prev_char == Some(' ')
        {
            return Ok(Some(CompletionResponse::Array(Self::arch_completions())));
        }

        Ok(None)
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

        // --- Check for preamble tag hover (word at column 0) ---
        if position.character == 0 || {
            // Allow the cursor to be anywhere on the tag word when it starts at col 0
            let line = doc.lines().nth(position.line as usize).unwrap_or("");
            let ch0 = line.chars().next();
            ch0.map(|c| c.is_ascii_alphabetic()).unwrap_or(false)
        } {
            if let Some((word, start, end)) = Self::word_at(&doc, position) {
                let line_text = doc.lines().nth(position.line as usize).unwrap_or("");
                // Only treat as a preamble tag if the word starts at column 0
                let word_col = line_text.find(&word).unwrap_or(usize::MAX);
                if word_col == 0 {
                    if let Some(desc) = preamble_tag_docs(&word) {
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
        }

        // --- Check for %{varName} or %global/%define variable hover ---
        if let Some((word, start, end)) = Self::word_at(&doc, position) {
            let offset = Self::offset_of(&doc, position).unwrap_or(0);

            // Check the two characters before the word for "%{"
            let is_macro_ref = offset >= 2
                && doc.as_bytes().get(offset.saturating_sub(word.len()).saturating_sub(2))
                    == Some(&b'%')
                && doc.as_bytes().get(offset.saturating_sub(word.len()).saturating_sub(1))
                    == Some(&b'{');

            // Also accept cursor directly on the varName after %global
            let line_text = doc.lines().nth(position.line as usize).unwrap_or("");
            let is_global_def = line_text.trim_start().starts_with("%global")
                || line_text.trim_start().starts_with("%define");

            if is_macro_ref || is_global_def {
                // Collect all const values defined for this variable
                let defs = Self::collect_definitions(&doc);
                let values: Vec<&str> = defs
                    .iter()
                    .filter(|(name, val, _)| name == &word && !val.is_empty())
                    .filter(|(_, val, _)| val.chars().all(|c| c.is_alphanumeric() || " ._+-~^".contains(c)))
                    .map(|(_, val, _)| val.as_str())
                    .collect();

                if !values.is_empty() {
                    let body = if values.len() == 1 {
                        format!("```yaml\n{word}: {}\n```", values[0])
                    } else {
                        format!(
                            "```yaml\n{}:\n  - {}\n```",
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
