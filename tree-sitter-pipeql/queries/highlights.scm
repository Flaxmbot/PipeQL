; PipeQL syntax highlighting queries

; statement + step keywords
[
  "from"
  "into"
  "insert"
  "upsert"
  "update"
  "union"
  "conflict"
  "do"
  "all"
  "table"
  "filter"
  "select"
  "derive"
  "join"
  "group"
  "sort"
  "take"
  "skip"
  "on"
  "as"
] @keyword

; `delete` is a bare-string rule, unlike the other step keywords.
(delete_step) @keyword

; upsert / union statements
(upsert_statement) @keyword
(union_statement) @keyword

; DDL column modifiers
[
  "primary"
  "auto"
  "unique"
  "default"
] @keyword

; DDL column types
[
  "int"
  "integer"
  "float"
  "real"
  "string"
  "text"
  "bool"
  "boolean"
  "timestamp"
  "datetime"
] @type

[
  "left"
  "right"
  "full"
  "inner"
] @keyword.operator

[
  "and"
  "or"
  "not"
  "in"
  "is"
] @keyword.operator

[
  "asc"
  "desc"
] @keyword.direction

; literals
[
  (number)
] @number

[
  (string)
] @string

[
  "true"
  "false"
] @boolean

"null" @constant.builtin

; identifiers
(identifier) @variable

(column_ref
  (identifier) @variable
  (identifier) @variable)

; function calls
(function_call
  (identifier) @function)

; parameters — must come after the generic identifier rule so that
; identifiers inside `$param` are highlighted as parameters, not variables.
(parameter
  (identifier) @parameter)

; operators and punctuation
[
  "=="
  "="
  "!="
  "<"
  "<="
  ">"
  ">="
  "<>"
  "+"
  "-"
  "*"
  "/"
] @operator

[
  "("
  ")"
  "["
  "]"
] @punctuation.bracket

"|" @punctuation.delimiter

; comments
(line_comment) @comment
(block_comment) @comment
