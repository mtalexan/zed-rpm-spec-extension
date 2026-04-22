; Outline / structure for RPM Spec files
; Shows spec sections in the Outline panel.

; %package sub-package declarations
(package
  "%package" @context
  name: (_) @name) @item

; %description sections
(description
  "%description" @name) @item

; Build scriptlet sections
(prep_scriptlet
  (section_prep) @name) @item

(generate_buildrequires
  (section_generate_buildrequires) @name) @item

(conf_scriptlet
  (section_conf) @name) @item

(build_scriptlet
  (section_build) @name) @item

(install_scriptlet
  (section_install) @name) @item

(check_scriptlet
  (section_check) @name) @item

(clean_scriptlet
  (section_clean) @name) @item

; Files section
(files
  "%files" @name) @item

; Changelog section
(changelog
  "%changelog" @name) @item
