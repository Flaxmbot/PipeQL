"""Fluent query builder for PipeQL (``pipeql_python.builder``).

Composes a PipeQL **source string** stage by stage, then compiles it through the
same native facade as any hand-written query — so a builder query and a literal
string query are provably identical. No dual parser, no semantic drift.

Object inserts/updates (``insert``, ``update``, ``upsert``, ``do_update``)
accept ``{column: value}`` dicts and auto-generate ``$b0``, ``$b1``, ...
bind parameters — the ``$data`` ergonomics without a driver, in every SDK.

Example::

    from pipeql_python.builder import PipeQL

    q = (PipeQL.from_("notes")
         .filter("is_archived == 0")
         .sort(["created_at desc"])
         .take(10))

    q.source()                      # "from notes | filter ... | take 10"
    result = q.compile("postgres")  # {"sql": ..., "params": [...]}
    q.values                        # {"b0": ..., "b1": ...} for object inserts

    # Runs through any existing PipeqlDriver (duck-typed):
    rows = db.query(q)
"""

from __future__ import annotations

from typing import Any, Dict, Iterable, List, Optional, Union

from .pipeql_python import compile as _compile

__all__ = ["PipeQL", "Value"]


class Value:
    """A typed value for object inserts/updates.

    Wraps plain Python values so the builder can distinguish ``None`` (null)
    from a missing key, and to mirror the Rust builder's ``Value`` enum.
    """

    __slots__ = ("raw",)

    def __init__(self, raw: Any):
        self.raw = raw

    def __repr__(self) -> str:  # pragma: no cover - debugging aid
        return f"Value({self.raw!r})"


def _normalize_value(v: Any) -> Any:
    """Unwrap ``Value`` wrappers so plain values pass through unchanged."""
    return v.raw if isinstance(v, Value) else v


def _as_list(cols: Union[str, Iterable[str]]) -> str:
    """Render a column list argument as a PipeQL bracketed body."""
    if isinstance(cols, str):
        return cols
    return ", ".join(str(c) for c in cols)


class PipeQL:
    """Fluent PipeQL query builder.

    Every stage method appends to the composed source and returns ``self`` for
    chaining. Use :meth:`source` for the PipeQL text, :meth:`compile` to
    compile it, or :attr:`values` for object-insert bound values.
    """

    def __init__(self, source: str):
        self._source = source
        self._values: Dict[str, Any] = {}

    # -- constructors -------------------------------------------------------

    @classmethod
    def from_(cls, table: str) -> "PipeQL":
        """Start a read pipeline: ``from <table>`` (``from`` is a keyword)."""
        return cls(f"from {table}")

    @classmethod
    def into_(cls, table: str) -> "PipeQL":
        """Start an insert/upsert pipeline: ``into <table>``."""
        return cls(f"into {table}")

    @classmethod
    def raw(cls, source: str) -> "PipeQL":
        """Start from an explicit PipeQL source string."""
        return cls(source)

    # -- pipeline stages ----------------------------------------------------

    def _stage(self, stage: str) -> "PipeQL":
        self._source += f" | {stage}"
        return self

    def filter(self, expr: str) -> "PipeQL":
        """``| filter <expr>``"""
        return self._stage(f"filter {expr}")

    def select(self, cols: Union[str, Iterable[str]]) -> "PipeQL":
        """``| select [<cols>]``"""
        return self._stage(f"select [{_as_list(cols)}]")

    def derive(self, cols: Union[str, Iterable[str]]) -> "PipeQL":
        """``| derive [<cols>]``"""
        return self._stage(f"derive [{_as_list(cols)}]")

    def sort(self, cols: Union[str, Iterable[str]]) -> "PipeQL":
        """``| sort [<cols>]``"""
        return self._stage(f"sort [{_as_list(cols)}]")

    def take(self, n: int) -> "PipeQL":
        """``| take <n>``"""
        return self._stage(f"take {n}")

    def skip(self, n: int) -> "PipeQL":
        """``| skip <n>``"""
        return self._stage(f"skip {n}")

    def join(self, table: str, on: str) -> "PipeQL":
        """``| join <table> on <on>``"""
        return self._stage(f"join {table} on {on}")

    def left_join(self, table: str, on: str) -> "PipeQL":
        """``| left join <table> on <on>``"""
        return self._stage(f"left join {table} on {on}")

    def right_join(self, table: str, on: str) -> "PipeQL":
        """``| right join <table> on <on>``"""
        return self._stage(f"right join {table} on {on}")

    def full_join(self, table: str, on: str) -> "PipeQL":
        """``| full join <table> on <on>``"""
        return self._stage(f"full join {table} on {on}")

    def inner_join(self, table: str, on: str) -> "PipeQL":
        """``| inner join <table> on <on>``"""
        return self._stage(f"inner join {table} on {on}")

    def group(self, cols: Union[str, Iterable[str]], aggs: str) -> "PipeQL":
        """``| group [<cols>] (<aggs>)``"""
        return self._stage(f"group [{_as_list(cols)}] ({aggs})")

    def union(self, other: Union[str, "PipeQL"]) -> "PipeQL":
        """``| union <other>`` where ``other`` is a source string or builder."""
        return self._stage(f"union {_other_source(other)}")

    def union_all(self, other: Union[str, "PipeQL"]) -> "PipeQL":
        """``| union all <other>``"""
        return self._stage(f"union all {_other_source(other)}")

    def raw_stage(self, stage: str) -> "PipeQL":
        """Append an explicit stage string."""
        return self._stage(stage)

    # -- mutations ----------------------------------------------------------

    def insert(self, values: Union[Dict[str, Any], str, Iterable[str]]) -> "PipeQL":
        """``| insert [...]`` with auto-generated ``$b0, $b1, ...`` params.

        Accepts a ``{column: value}`` dict (values become bound params), a
        single assignment string (``"title = $title"``), or a list of
        assignment strings.
        """
        return self._assignments("insert", values)

    def update(self, values: Union[Dict[str, Any], str, Iterable[str]]) -> "PipeQL":
        """``| update [...]`` (requires a preceding filter stage)."""
        return self._assignments("update", values)

    def update_all(self, values: Union[Dict[str, Any], str, Iterable[str]]) -> "PipeQL":
        """``| update all [...]`` — explicit opt-in for a full-table update that
        bypasses the filter guard."""
        return self._assignments("update all", values)

    def delete(self) -> "PipeQL":
        """``| delete``"""
        return self._stage("delete")

    def delete_all(self) -> "PipeQL":
        """``| delete all`` — explicit opt-in for a full-table delete that
        bypasses the filter guard."""
        return self._stage("delete all")

    def upsert(self, values: Union[Dict[str, Any], str, Iterable[str]]) -> "PipeQL":
        """``| upsert [...]``"""
        return self._assignments("upsert", values)

    def conflict(self, cols: Union[str, Iterable[str]]) -> "PipeQL":
        """``| conflict [<cols>]``"""
        return self._stage(f"conflict [{_as_list(cols)}]")

    def do_update(self, values: Union[Dict[str, Any], str, Iterable[str]]) -> "PipeQL":
        """``| do update [...]``"""
        return self._assignments("do update", values)

    def _assignments(self, kind: str, values: Union[Dict[str, Any], str, Iterable[str]]) -> "PipeQL":
        if isinstance(values, dict):
            # {column: value} — every value becomes a bound $bN param (incl. None)
            body = []
            for key, val in values.items():
                pname = f"b{len(self._values)}"
                self._values[pname] = _normalize_value(val)
                body.append(f"{key} = ${pname}")
        elif isinstance(values, str):
            # Already a PipeQL assignment string, e.g. "title = $title"
            body = [values]
        else:
            body = [str(v) for v in values]
        return self._stage(f"{kind} [{', '.join(body)}]")

    # -- output --------------------------------------------------------------

    def source(self) -> str:
        """The composed PipeQL source string."""
        return self._source

    @property
    def values(self) -> Dict[str, Any]:
        """Bound values from object inserts/updates, keyed by ``$bN`` name."""
        return dict(self._values)

    def compile(self, dialect: str = "postgres", **params: Any) -> Dict[str, Any]:
        """Compile through the standard facade.

        ``params`` are merged with builder-generated values so a compiled
        query can be executed directly: ``q.compile("sqlite", id=5)["args"]``
        is only present on the driver wrapper; this returns the native result
        plus the merged values under ``"values"``.
        """
        result = _compile(self._source, dialect)
        merged = {**self._values, **params}
        result["values"] = merged
        return result

    def __str__(self) -> str:
        return self._source

    def __repr__(self) -> str:  # pragma: no cover - debugging aid
        return f"PipeQL({self._source!r})"


def _other_source(other: Union[str, "PipeQL"]) -> str:
    if isinstance(other, PipeQL):
        return other.source()
    return other
