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
    pipeql: `from orders\n| join customers on orders.customer_id == customers.id\n| filter orders.status == 'active' and orders.total >= $min\n| group [region] (total = sum(orders.total), cnt = count(*))\n| filter total > $threshold\n| select [region, total, cnt]\n| sort [total desc]\n| take 10`,
    postgres: `SELECT region, SUM(orders.total) AS total, COUNT(*) AS cnt FROM orders\nINNER JOIN customers ON (orders.customer_id = customers.id)\nWHERE ((orders.status = $1) AND (orders.total >= $2))\nGROUP BY region\nHAVING (sum(orders.total) > $3)\nORDER BY total DESC\nLIMIT 10;`,
    sqlite: `SELECT region, SUM(orders.total) AS total, COUNT(*) AS cnt FROM orders\nINNER JOIN customers ON (orders.customer_id = customers.id)\nWHERE ((orders.status = ?) AND (orders.total >= ?))\nGROUP BY region\nHAVING (sum(orders.total) > ?)\nORDER BY total DESC\nLIMIT 10;`,
    duckdb: `SELECT region, SUM(orders.total) AS total, COUNT(*) AS cnt FROM orders\nINNER JOIN customers ON (orders.customer_id = customers.id)\nWHERE ((orders.status = ?) AND (orders.total >= ?))\nGROUP BY region\nHAVING (sum(orders.total) > ?)\nORDER BY total DESC\nLIMIT 10;`,
    mysql: `SELECT region, SUM(orders.total) AS total, COUNT(*) AS cnt FROM orders\nINNER JOIN customers ON (orders.customer_id = customers.id)\nWHERE ((orders.status = ?) AND (orders.total >= ?))\nGROUP BY region\nHAVING (sum(orders.total) > ?)\nORDER BY total DESC\nLIMIT 10;`,
    params: ["active", "min", "threshold"]
  },
  insert: {
    title: "Insert Record",
    pipeql: `into notes\n| insert [\n  title = $title,\n  content = $content,\n  category = 'Personal',\n  is_pinned = 0\n]`,
    postgres: `INSERT INTO notes (title, content, category, is_pinned)\nVALUES ($1, $2, $3, $4)\nRETURNING *;`,
    sqlite: `INSERT INTO notes (title, content, category, is_pinned)\nVALUES (?, ?, ?, ?);`,
    duckdb: `INSERT INTO notes (title, content, category, is_pinned)\nVALUES (?, ?, ?, ?);`,
    mysql: `INSERT INTO notes (title, content, category, is_pinned)\nVALUES (?, ?, ?, ?);`,
    params: ["title", "content", "Personal", "0"]
  },
  update: {
    title: "Update Record",
    pipeql: `from notes\n| filter id == $id and is_archived == 0\n| update [\n  title = $title,\n  is_pinned = 1,\n  updated_at = current_timestamp\n]`,
    postgres: `UPDATE notes\nSET title = $1, is_pinned = $2, updated_at = CURRENT_TIMESTAMP\nWHERE ((id = $3) AND (is_archived = $4));`,
    sqlite: `UPDATE notes\nSET title = ?, is_pinned = ?, updated_at = CURRENT_TIMESTAMP\nWHERE ((id = ?) AND (is_archived = ?));`,
    duckdb: `UPDATE notes\nSET title = ?, is_pinned = ?, updated_at = CURRENT_TIMESTAMP\nWHERE ((id = ?) AND (is_archived = ?));`,
    mysql: `UPDATE notes\nSET title = ?, is_pinned = ?, updated_at = CURRENT_TIMESTAMP\nWHERE ((id = ?) AND (is_archived = ?));`,
    params: ["title", "1", "id", "0"]
  },
  delete: {
    title: "Delete Record",
    pipeql: `from notes\n| filter id == $id\n| delete`,
    postgres: `DELETE FROM notes WHERE (id = $1);`,
    sqlite: `DELETE FROM notes WHERE (id = ?);`,
    duckdb: `DELETE FROM notes WHERE (id = ?);`,
    mysql: `DELETE FROM notes WHERE (id = ?);`,
    params: ["id"]
  },
  table: {
    title: "Create Table DDL",
    pipeql: `table notes [\n  id int primary auto,\n  title string not null,\n  content string not null,\n  category string default 'Personal',\n  is_pinned int default 0,\n  created_at timestamp default current_timestamp\n]`,
    postgres: `CREATE TABLE IF NOT EXISTS notes (\n  id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,\n  title TEXT NOT NULL,\n  content TEXT NOT NULL,\n  category TEXT DEFAULT 'Personal',\n  is_pinned INTEGER DEFAULT 0,\n  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP\n);`,
    sqlite: `CREATE TABLE IF NOT EXISTS notes (\n  id INTEGER PRIMARY KEY AUTOINCREMENT,\n  title TEXT NOT NULL,\n  content TEXT NOT NULL,\n  category TEXT DEFAULT 'Personal',\n  is_pinned INTEGER DEFAULT 0,\n  created_at DATETIME DEFAULT CURRENT_TIMESTAMP\n);`,
    duckdb: `CREATE TABLE IF NOT EXISTS notes (\n  id INTEGER GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY,\n  title VARCHAR NOT NULL,\n  content VARCHAR NOT NULL,\n  category VARCHAR DEFAULT 'Personal',\n  is_pinned INTEGER DEFAULT 0,\n  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP\n);`,
    mysql: `CREATE TABLE IF NOT EXISTS notes (\n  id INT AUTO_INCREMENT PRIMARY KEY,\n  title VARCHAR(255) NOT NULL,\n  content VARCHAR(255) NOT NULL,\n  category VARCHAR(255) DEFAULT 'Personal',\n  is_pinned BOOLEAN DEFAULT 0,\n  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP\n);`,
    params: []
  },
  upsert: {
    title: "Upsert (Insert or Update)",
    pipeql: `into users\n| upsert [\n  name = $name,\n  email = $email\n]\n| conflict [email]\n| do update [\n  name = $name\n]`,
    postgres: `INSERT INTO users (name, email) VALUES ($1, $2)\nON CONFLICT (email) DO UPDATE SET name = $1\nRETURNING *;`,
    sqlite: `INSERT INTO users (name, email) VALUES (?, ?)\nON CONFLICT (email) DO UPDATE SET name = ?;`,
    duckdb: `INSERT INTO users (name, email) VALUES (?, ?)\nON CONFLICT (email) DO UPDATE SET name = ?;`,
    mysql: `INSERT INTO users (name, email) VALUES (?, ?)\nON DUPLICATE KEY UPDATE name = ?;`,
    params: ["name", "email"]
  },
  union: {
    title: "Union (Combine Results)",
    pipeql: `from active_users\n| select [id, name]\n| union all\nfrom archived_users\n| select [id, name]`,
    postgres: `SELECT id, name FROM active_users\nUNION ALL\nSELECT id, name FROM archived_users;`,
    sqlite: `SELECT id, name FROM active_users\nUNION ALL\nSELECT id, name FROM archived_users;`,
    duckdb: `SELECT id, name FROM active_users\nUNION ALL\nSELECT id, name FROM archived_users;`,
    mysql: `SELECT id, name FROM active_users\nUNION ALL\nSELECT id, name FROM archived_users;`,
    params: []
  },
  subquery: {
    title: "Subquery (IN)",
    pipeql: `from orders\n| filter customer_id in (\n  from customers\n  | filter region == 'EU'\n  | select [id]\n)`,
    postgres: `SELECT * FROM orders\nWHERE (customer_id IN (\n  SELECT id FROM customers\n  WHERE (region = $1)\n));`,
    sqlite: `SELECT * FROM orders\nWHERE (customer_id IN (\n  SELECT id FROM customers\n  WHERE (region = ?)\n));`,
    duckdb: `SELECT * FROM orders\nWHERE (customer_id IN (\n  SELECT id FROM customers\n  WHERE (region = ?)\n));`,
    mysql: `SELECT * FROM orders\nWHERE (customer_id IN (\n  SELECT id FROM customers\n  WHERE (region = ?)\n));`,
    params: ["EU"]
  }
};
