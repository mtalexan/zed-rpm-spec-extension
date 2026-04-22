; Bracket matching for RPM Spec files

("%{" @open "}" @close)
("(" @open ")" @close)
("\"" @open "\"" @close)
