; Syntax highlighting for RPM Spec files
; Derived from the tree-sitter-rpmspec grammar query file.
; https://github.com/cryptomilk/tree-sitter-rpmspec (MIT License)

; =============================================================================
; PREAMBLE
; =============================================================================

; Tags like Name:, Version:, Release:, etc.
; @type.definition falls back to @type for themes that don't define it.
[
  (tag)
  (dependency_tag)
] @type.definition @type

; Dependency tag qualifier (e.g., post in Requires(post):)
(qualifier) @attribute.builtin @attribute

; Boolean dependency operators
[
  "if"
  "else"
  "unless"
  "with"
  "without"
] @keyword.operator

; Boolean dependency parentheses
(boolean_dependency
  "(" @punctuation.bracket
  ")" @punctuation.bracket)

; -----------------------------------------------------------------------------
; Dependency Types
; -----------------------------------------------------------------------------

; ELF dependencies: libc.so.6(GLIBC_2.2.5)(64bit)
(elf_dependency
  soname: (soname) @module
  symbol_version: (elf_symbol_version) @property
  arch: (elf_arch) @attribute)

; Path dependencies: /usr/bin/pkg-config
(path_dependency) @string.special.path @string.special

; Qualified dependencies: perl(Carp), pkgconfig(glib-2.0)
(qualified_dependency
  name: (_) @function
  qualifier: (dependency_qualifier
    content: (_) @variable.parameter))

; ISA qualifiers - known architecture patterns
((dependency_qualifier
   content: (word) @attribute)
 (#match? @attribute "^(x86-64|x86-32|aarch64|arm|ppc-64|ppc-32|s390x)$"))

; Simple module dependencies: make, cmake-filesystem >= 3
(qualified_dependency
  name: (_) @module)
(dependency
  (word) @module)

; Source tag file paths
(preamble_tag
  value: (file) @string.special.path @string.special)

; Description and package sections
(description
  "%description" @type.definition @type)
(package
  "%package" @type.definition @type)

; Sourcelist section
(sourcelist
  "%sourcelist" @type.definition @type)
(sourcelist
  (file) @string.special.path @string.special)

; Patchlist section
(patchlist
  "%patchlist" @type.definition @type)
(patchlist
  (file) @string.special.path @string.special)

; =============================================================================
; SCRIPTLETS
; =============================================================================

; Build scriptlets (%prep, %build, %install, %check, %clean, %conf)
; @module.builtin falls back to @keyword for themes that don't define it.
(prep_scriptlet
  (section_prep) @module.builtin @keyword)
(generate_buildrequires
  (section_generate_buildrequires) @module.builtin @keyword)
(conf_scriptlet
  (section_conf) @module.builtin @keyword)
(build_scriptlet
  (section_build) @module.builtin @keyword)
(install_scriptlet
  (section_install) @module.builtin @keyword)
(check_scriptlet
  (section_check) @module.builtin @keyword)
(clean_scriptlet
  (section_clean) @module.builtin @keyword)

; Runtime scriptlets (%pre, %post, %preun, %postun, etc.)
[
  "%pre"
  "%post"
  "%preun"
  "%postun"
  "%pretrans"
  "%posttrans"
  "%preuntrans"
  "%postuntrans"
  "%verify"
] @module.builtin @keyword

; Scriptlet interpreter (-p <program>)
(script_interpreter
  "-p" @variable.parameter)
(interpreter_program) @string.special.path @string.special

; Scriptlet augment options (-a, -p for append/prepend)
(scriptlet_augment_option) @variable.parameter

; Trigger scriptlets
[
  "%triggerprein"
  "%triggerin"
  "%triggerun"
  "%triggerpostun"
] @module.builtin @keyword

; File trigger scriptlets
[
  "%filetriggerin"
  "%filetriggerun"
  "%filetriggerpostun"
  "%transfiletriggerin"
  "%transfiletriggerun"
  "%transfiletriggerpostun"
] @module.builtin @keyword

; -----------------------------------------------------------------------------
; Prep macros (%setup, %autosetup, %patch, %autopatch)
; -----------------------------------------------------------------------------

[
  (setup_macro argument: (macro_option) @variable.parameter)
  (autosetup_macro argument: (macro_option) @variable.parameter)
  (autopatch_macro argument: (macro_option) @variable.parameter)
  (patch_macro argument: (macro_option) @variable.parameter)
]

; Patch number arguments
[
  (autopatch_macro (macro_argument) @number)
  (patch_macro (macro_argument) @number)
]

; =============================================================================
; FILES SECTION
; =============================================================================

(files
  "%files" @type.definition @type)

; File directives
[
  "%artifact"
  "%attr"
  "%caps"
  "%config"
  "%defattr"
  "%dir"
  "%doc"
  "%docdir"
  "%exclude"
  "%ghost"
  "%license"
  "%missingok"
  "%readme"
] @keyword.type @keyword

; =============================================================================
; CHANGELOG
; =============================================================================

(changelog
  "%changelog" @type.definition @type)

; =============================================================================
; MACROS
; =============================================================================

; -----------------------------------------------------------------------------
; Macro definitions
; -----------------------------------------------------------------------------

(macro_definition
  "%" @punctuation.special
  ["define" "global"] @constant.builtin
  name: (identifier) @keyword.macro @keyword)

(macro_undefinition
  "%" @punctuation.special
  (builtin) @constant.builtin
  (identifier) @keyword.macro @keyword)

; -----------------------------------------------------------------------------
; Simple macro expansion (%name, %!name, %*, etc.)
; -----------------------------------------------------------------------------

(macro_simple_expansion
  "%" @punctuation.special
  (simple_macro) @constant.macro @constant)

(macro_simple_expansion
  "%" @punctuation.special
  (negated_macro) @constant.macro @constant)

(macro_simple_expansion
  "%" @punctuation.special
  (special_macro) @constant.macro @constant)

; -----------------------------------------------------------------------------
; Parametric macro expansion (%name [options] [arguments])
; -----------------------------------------------------------------------------

(macro_parametric_expansion
  name: (identifier) @function.macro @function)

(macro_parametric_expansion
  option: (macro_option) @variable.parameter)

(macro_parametric_expansion
  argument: (word) @variable.parameter)

(macro_parametric_expansion
  argument: (integer) @number)

(macro_parametric_expansion
  argument: (quoted_string) @string)

(macro_option) @variable.parameter

; -----------------------------------------------------------------------------
; Brace macro expansion (%{name}, %{name:arg}, etc.)
; -----------------------------------------------------------------------------

(macro_expansion
  "%{" @punctuation.special
  "}" @punctuation.special) @none

(macro_expansion
  (builtin) @constant.builtin
  argument: (_) @variable.parameter)

(macro_expansion
  (identifier) @constant.macro @constant)

(macro_expansion
  (identifier)
  argument: [
    (word) @variable.parameter
    (concatenation
      (word) @variable.parameter)
  ])

; Conditional expansion (%{?name}, %{!?name})
(conditional_expansion
  condition: (identifier) @constant.macro @constant)

; General macro rules
(special_variable_name) @constant
(builtin) @constant.builtin

; =============================================================================
; CONDITIONALS
; =============================================================================

[
  "%if"
  "%ifarch"
  "%ifnarch"
  "%ifos"
  "%ifnos"
  "%elif"
  "%elifarch"
  "%elifos"
  "%else"
  "%endif"
] @keyword.conditional

[
  "defined"
  "undefined"
] @keyword.operator

; =============================================================================
; LITERALS AND OPERATORS
; =============================================================================

(integer) @number
(float) @number.float @number
(version) @number.float @number

(quoted_string) @string

(url) @string.special.url @string.special

(comment) @comment

[
  "!="
  "<"
  "<="
  "="
  "=="
  ">"
  ">="
  "and"
  "&&"
  "or"
  "||"
] @operator
