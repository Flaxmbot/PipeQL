/*
 * Tree-sitter grammar for PipeQL.
 *
 * Statements: read pipelines (`from ...`), inserts (`into ... | insert [...]`),
 * and schema DDL (`table ...`), with injection-safe `$param` placeholders and
 * lossless comments.
 */

export default grammar({
  name: "pipeql",

  extras: ($) => [/\s/, $.line_comment, $.block_comment],

  conflicts: ($) => [[$.pipeline, $._separator]],

  rules: {
    // A statement is a read pipeline, an insert, or a table DDL declaration.
    statement: ($) => choice(
      $.pipeline,
      $.insert_statement,
      $.table_statement,
    ),

    // A pipeline is a source table followed by zero or more steps,
    // separated by `|` or a newline (consecutive separators collapse).
    // The separator scaffolding is hidden so the tree shows only semantics.
    pipeline: ($) => seq(
      $.source,
      repeat($._step_with_sep),
      optional($._trailing_separators),
    ),

    _step_with_sep: ($) => seq($._separator, $.step),

    _separator: ($) => prec(1, repeat1(choice("|", "\n"))),

    _trailing_separators: ($) => repeat1(choice("|", "\n")),

    source: ($) => seq(
      "from",
      $.identifier,
      optional($.identifier), // table alias
    ),

    insert_statement: ($) => seq(
      "into",
      $.identifier,
      $._separator,
      "insert",
      $.assignment_list,
    ),

    table_statement: ($) => seq(
      "table",
      $.identifier,
      "[",
      commaSep($.column_def),
      "]",
    ),

    column_def: ($) => seq(
      $.identifier,
      $.column_type,
      repeat($.column_modifier),
    ),

    column_type: ($) => choice(
      "int", "integer",
      "float", "real",
      "string", "text",
      "bool", "boolean",
      "timestamp", "datetime",
    ),

    column_modifier: ($) => choice(
      "primary",
      "auto",
      "unique",
      seq("not", "null"),
      seq("default", $.expression),
    ),

    step: ($) => choice(
      $.filter_step,
      $.select_step,
      $.derive_step,
      $.join_step,
      $.group_step,
      $.sort_step,
      $.take_step,
      $.skip_step,
      $.update_step,
      $.delete_step,
    ),

    filter_step: ($) => seq("filter", $.expression),

    select_step: ($) => seq("select", $.select_list),

    select_list: ($) => seq(
      "[",
      commaSep($.select_item),
      "]",
    ),

    select_item: ($) => seq(
      $.expression,
      optional(seq("as", $.identifier)),
    ),

    derive_step: ($) => seq("derive", $.assignment_list),

    update_step: ($) => seq("update", $.assignment_list),

    delete_step: ($) => "delete",

    assignment_list: ($) => seq(
      "[",
      commaSep($.assignment),
      "]",
    ),

    assignment: ($) => seq(
      $.identifier,
      "=",
      $.expression,
    ),

    join_step: ($) => seq(
      optional(choice("left", "right", "full", "inner")),
      "join",
      $.identifier,
      optional($.identifier), // alias
      "on",
      $.expression,
    ),

    group_step: ($) => seq(
      "group",
      $.group_spec,
    ),

    group_spec: ($) => seq(
      "[",
      commaSep($.expression),
      "]",
      optional(seq(
        "(",
        commaSep($.aggregate),
        ")",
      )),
    ),

    aggregate: ($) => seq(
      $.identifier,
      "=",
      $.identifier, // function name
      "(",
      commaSep($.expression),
      ")",
    ),

    sort_step: ($) => seq(
      "sort",
      "[",
      commaSep($.sort_item),
      "]",
    ),

    sort_item: ($) => seq(
      $.expression,
      optional(choice("asc", "desc")),
    ),

    take_step: ($) => seq("take", $.number),

    skip_step: ($) => seq("skip", $.number),

    // --- expressions ---
    expression: ($) => choice(
      prec(2, $.or_expression),
    ),

    or_expression: ($) => seq(
      $.and_expression,
      repeat(seq("or", $.and_expression)),
    ),

    and_expression: ($) => seq(
      $.not_expression,
      repeat(seq("and", $.not_expression)),
    ),

    not_expression: ($) => choice(
      prec(3, seq("not", $.not_expression)),
      $.comparison,
    ),

    comparison: ($) => choice(
      prec(1, seq($.additive, optional($.comparison_rest))),
      prec(1, $.in_expression),
      prec(1, $.is_expression),
    ),

    comparison_rest: ($) => seq(
      choice("==", "=", "!=", "<", "<=", ">", ">=", "<>"),
      $.additive,
    ),

    in_expression: ($) => seq(
      $.additive,
      optional("not"),
      "in",
      seq("[", commaSep($.expression), "]"),
    ),

    is_expression: ($) => seq(
      $.additive,
      "is",
      optional("not"),
      "null",
    ),

    additive: ($) => seq(
      $.multiplicative,
      repeat(seq(choice("+", "-"), $.multiplicative)),
    ),

    multiplicative: ($) => seq(
      $.unary,
      repeat(seq(choice("*", "/"), $.unary)),
    ),

    unary: ($) => choice(
      $.primary,
      seq("-", $.primary),
    ),

    primary: ($) => choice(
      $.literal,
      $.parameter,
      $.column_ref,
      $.function_call,
      $.star,
      $.parenthesized,
    ),

    parenthesized: ($) => seq("(", $.expression, ")"),

    star: ($) => "*",

    literal: ($) => choice(
      $.number,
      $.string,
      $.boolean,
      "null",
    ),

    number: ($) => token(choice(/\d+/, /\d*\.\d+/)),

    string: ($) => seq("'", optional(/[^']*/), "'"),

    boolean: ($) => choice("true", "false"),

    parameter: ($) => choice(
      seq("$", $.identifier),
      seq("${", $.identifier, "}"),
    ),

    column_ref: ($) => seq(
      $.identifier,
      repeat(seq(".", $.identifier)),
    ),

    function_call: ($) => seq(
      $.identifier,
      "(",
      commaSep($.expression),
      ")",
    ),

    identifier: ($) => /[a-zA-Z_][a-zA-Z0-9_]*/,

    line_comment: ($) => seq("--", /[^\n]*/),

    block_comment: ($) => seq("/*", /[^*]*\*+([^/*][^*]*\*+)*/, "/"),
  },
});

function commaSep(rule) {
  return seq(optional(rule), repeat(seq(",", rule)));
}
