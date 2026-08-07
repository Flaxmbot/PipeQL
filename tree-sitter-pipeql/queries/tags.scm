; PipeQL tag queries (definitions for ctags-style symbol lookup).
(derive_step
  (assignment_list
    (assignment
      (identifier) @name
      "="
      (expression) @value)))

(aggregate
  (identifier) @name)
