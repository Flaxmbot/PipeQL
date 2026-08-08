"""PipeQL Python SDK (v1.1.6).

Re-exports the native ``pipeql_python`` extension functions and the
zero-boilerplate ``driver`` adapters.

>>> import pipeql_python as p
>>> p.compile("from users | select [id]", "sqlite")["sql"]
'SELECT id FROM users;'
>>> from pipeql_python import driver
"""

try:
    from .pipeql_python import (
        compile,
        compile_with_catalog,
        parse,
        supported_dialects,
        version,
    )
except ImportError:
    from pipeql_python import (
        compile,
        compile_with_catalog,
        parse,
        supported_dialects,
        version,
    )
from pathlib import Path
from . import builder, driver
from .builder import PipeQL, Value

_prompt_path = Path(__file__).parent / "ai" / "system_prompt.md"
if _prompt_path.exists():
    SYSTEM_PROMPT = _prompt_path.read_text(encoding="utf-8")
else:
    SYSTEM_PROMPT = ""

__all__ = [
    "compile",
    "compile_with_catalog",
    "parse",
    "supported_dialects",
    "version",
    "builder",
    "driver",
    "PipeQL",
    "Value",
    "SYSTEM_PROMPT",
]
