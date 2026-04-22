; Syntax highlighting for RPMBash (RPM-aware bash scriptlets)
; Derived from the tree-sitter-rpmspec grammar query file.
; https://github.com/cryptomilk/tree-sitter-rpmspec (MIT License)
;
; Bash highlighting is inherited via the grammar's tree-sitter.json configuration.
; RPM constructs are delegated back to rpmspec via injection.parent.
; This file provides fallback highlighting when used standalone.

(rpm_macro_expansion) @embedded
(rpm_macro_simple) @embedded
(rpm_conditional_keyword) @keyword.conditional
(rpm_else) @keyword.conditional
(rpm_endif) @keyword.conditional

; Override bash's "}" @punctuation.bracket for RPM macro expansion closing brace
(rpm_macro_expansion
  "}" @punctuation.special)
