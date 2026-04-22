; Language injection queries for RPM Spec files
; Derived from the tree-sitter-rpmspec grammar query file.
; https://github.com/cryptomilk/tree-sitter-rpmspec (MIT License)
;
; Scriptlet bodies are injected as rpmbash (RPM-aware bash).
; Scriptlets with -p <interpreter> are injected as the appropriate language.

; =============================================================================
; BUILD SCRIPTLETS (%prep, %build, %install, %check, %clean, %conf)
; These always use rpmbash (no interpreter option)
; =============================================================================

(prep_scriptlet (script_block) @injection.content
  (#set! injection.language "rpmbash")
  (#set! injection.include-children))
(build_scriptlet (script_block) @injection.content
  (#set! injection.language "rpmbash")
  (#set! injection.include-children))
(install_scriptlet (script_block) @injection.content
  (#set! injection.language "rpmbash")
  (#set! injection.include-children))
(check_scriptlet (script_block) @injection.content
  (#set! injection.language "rpmbash")
  (#set! injection.include-children))
(clean_scriptlet (script_block) @injection.content
  (#set! injection.language "rpmbash")
  (#set! injection.include-children))
(conf_scriptlet (script_block) @injection.content
  (#set! injection.language "rpmbash")
  (#set! injection.include-children))
(generate_buildrequires (script_block) @injection.content
  (#set! injection.language "rpmbash")
  (#set! injection.include-children))

; =============================================================================
; RUNTIME SCRIPTLETS (no -p option) -> rpmbash
; =============================================================================

(runtime_scriptlet
  (script_block) @injection.content
  (#set! injection.language "rpmbash")
  (#set! injection.include-children))

; =============================================================================
; RUNTIME SCRIPTLETS WITH INTERPRETER (-p option)
; =============================================================================

; Lua: -p <lua>
(runtime_scriptlet_interpreter
  interpreter: (script_interpreter
    program: (interpreter_program) @_interp)
  (script_block) @injection.content
  (#eq? @_interp "<lua>")
  (#set! injection.language "lua")
  (#set! injection.include-children))

; Python: -p /path/python or -p /path/python3
(runtime_scriptlet_interpreter
  interpreter: (script_interpreter
    program: (interpreter_program) @_interp)
  (script_block) @injection.content
  (#match? @_interp "python")
  (#set! injection.language "python")
  (#set! injection.include-children))

; Perl: -p /path/perl
(runtime_scriptlet_interpreter
  interpreter: (script_interpreter
    program: (interpreter_program) @_interp)
  (script_block) @injection.content
  (#match? @_interp "perl")
  (#set! injection.language "perl")
  (#set! injection.include-children))

; Bash/sh: -p /bin/bash, -p /bin/sh, etc.
(runtime_scriptlet_interpreter
  interpreter: (script_interpreter
    program: (interpreter_program) @_interp)
  (script_block) @injection.content
  (#match? @_interp "(bash|/sh$)")
  (#set! injection.language "rpmbash")
  (#set! injection.include-children))

; =============================================================================
; TRIGGERS
; =============================================================================

; Default rpmbash for triggers
(trigger
  (script_block) @injection.content
  (#set! injection.language "rpmbash")
  (#set! injection.include-children))

; Lua interpreter for triggers
(trigger
  interpreter: (script_interpreter
    program: (interpreter_program) @_interp)
  (script_block (script_line) @injection.content)
  (#eq? @_interp "<lua>")
  (#not-match? @injection.content "^\\s*[%]")
  (#set! injection.language "lua")
  (#set! injection.include-children)
  (#set! injection.combined))

; Perl interpreter for triggers
(trigger
  interpreter: (script_interpreter
    program: (interpreter_program) @_interp)
  (script_block (script_line) @injection.content)
  (#match? @_interp "perl")
  (#not-match? @injection.content "^\\s*[%]")
  (#set! injection.language "perl")
  (#set! injection.include-children)
  (#set! injection.combined))

; =============================================================================
; FILE TRIGGERS
; =============================================================================

; Default rpmbash for file triggers
(file_trigger
  (script_block) @injection.content
  (#set! injection.language "rpmbash")
  (#set! injection.include-children))

; Lua interpreter for file triggers
(file_trigger
  interpreter: (script_interpreter
    program: (interpreter_program) @_interp)
  (script_block (script_line) @injection.content)
  (#eq? @_interp "<lua>")
  (#not-match? @injection.content "^\\s*[%]")
  (#set! injection.language "lua")
  (#set! injection.include-children)
  (#set! injection.combined))

; =============================================================================
; SHELL COMMAND EXPANSION %(...)
; =============================================================================

((shell_command) @injection.content
  (#set! injection.language "rpmbash")
  (#set! injection.include-children))

; =============================================================================
; LUA MACRO EXPANSION %{lua:...}
; =============================================================================

(macro_expansion
  (builtin) @_builtin
  argument: (script_code) @injection.content
  (#eq? @_builtin "lua:")
  (#set! injection.language "lua")
  (#set! injection.include-children)
  (#set! injection.combined))
