export const DOCS_SECTIONS = [
  {
    id: "getting-started",
    title: "Getting Started",
    items: [
      { id: "intro", label: "Introduction" },
      { id: "quickstart", label: "Quick Start & Installation" }
    ]
  },
  {
    id: "syntax",
    title: "Syntax Reference",
    items: [
      { id: "syntax", label: "Query Syntax (EBNF)" },
      { id: "mutations", label: "Mutations (DML)" },
      { id: "ddl", label: "Table Schema (DDL)" }
    ]
  },
  {
    id: "sdks",
    title: "Polyglot SDKs",
    items: [
      { id: "api-reference", label: "API Reference" },
      { id: "drivers", label: "Driver Adapters" }
    ]
  },
  {
    id: "tools",
    title: "Tools & IDE",
    items: [
      { id: "lsp", label: "LSP & VS Code" },
      { id: "tree-sitter", label: "Tree-sitter Grammar" }
    ]
  },
  {
    id: "deep-dive",
    title: "Deep Dive",
    items: [
      { id: "architecture", label: "Architecture" },
      { id: "contributing", label: "Contributing" }
    ]
  }
];

export const SAMPLE_QUERIES = {
  basic: {
    title: "Simple Pipeline",
    pipeql: `from orders\n| join customers on orders.customer_id == customers.id\n| filter orders.status == 'active' and orders.total >= $min\n| group [region] (total = sum(orders.total), cnt = count(*))\n| filter total > $threshold\n| select [region, total, cnt]\n| sort [total desc]\n| take 10`
  },
  insert: {
    title: "Insert",
    pipeql: `into notes\n| insert [\n  title = $title,\n  content = $content,\n  category = 'Personal',\n  is_pinned = 0\n]`
  },
  update: {
    title: "Update",
    pipeql: `from notes\n| filter id == $id and is_archived == 0\n| update [\n  title = $title,\n  is_pinned = 1,\n  updated_at = current_timestamp\n]`
  },
  delete: {
    title: "Delete",
    pipeql: `from notes\n| filter id == $id\n| delete`
  },
  update_all: {
    title: "Update All",
    pipeql: `from users\n| update all [\n  plan = 'free',\n  updated_at = current_timestamp\n]`
  },
  delete_all: {
    title: "Delete All",
    pipeql: `from users\n| delete all`
  },
  upsert: {
    title: "Upsert",
    pipeql: `into users\n| upsert [\n  name = $name,\n  email = $email\n]\n| conflict [email]\n| do update [\n  name = $name\n]`
  },
  union: {
    title: "Union",
    pipeql: `from active_users\n| select [id, name]\n| union all\nfrom archived_users\n| select [id, name]`
  },
  subquery: {
    title: "Subquery",
    pipeql: `from orders\n| filter customer_id in (\n  from customers\n  | filter region == 'EU'\n  | select [id]\n)`
  }
};
