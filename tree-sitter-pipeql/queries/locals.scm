; PipeQL local-scope queries.
; Pipeline steps are data-flow scopes: each step sees the columns of the
; source and of the derived steps before it. Locals are informational.
(derive_step
  (assignment_list
    (assignment
      (identifier) @local.definition)))

(aggregate
  (identifier) @local.definition)

(column_ref) @local.reference
