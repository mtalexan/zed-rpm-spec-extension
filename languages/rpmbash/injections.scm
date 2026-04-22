; Injection queries for RPMBash
; Derived from the tree-sitter-rpmspec grammar query file.
; https://github.com/cryptomilk/tree-sitter-rpmspec (MIT License)
;
; RPM constructs found inside bash scriptlets are delegated back to the parent
; rpmspec grammar for parsing and highlighting.

; =============================================================================
; RPM MACRO EXPANSIONS -> rpmspec
; =============================================================================

; Brace expansion: %{...}, %{?name}, %{name:arg}, etc.
((rpm_macro_expansion) @injection.content
  (#set! injection.parent))

; Simple expansion: %name, %version, etc.
((rpm_macro_simple) @injection.content
  (#set! injection.parent))

; =============================================================================
; RPM MACRO DEFINITIONS -> rpmspec
; =============================================================================

((rpm_global) @injection.content
  (#set! injection.parent))

((rpm_define) @injection.content
  (#set! injection.parent))

((rpm_undefine) @injection.content
  (#set! injection.parent))

; =============================================================================
; RPM SPECIAL PREP MACROS -> rpmspec
; =============================================================================

((rpm_setup) @injection.content
  (#set! injection.parent))

((rpm_autosetup) @injection.content
  (#set! injection.parent))

((rpm_patch) @injection.content
  (#set! injection.parent))

((rpm_autopatch) @injection.content
  (#set! injection.parent))
