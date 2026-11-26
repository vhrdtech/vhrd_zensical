"""Rust coloured as GitHub colours it: calls, types and macro calls marked by their shape.

Pygments' Rust lexer leaves most names plain: a call such as `writer.put(...)` or `writer::create(...)`, a type
it does not know (`FixedAxis`, `GridDef`), a macro outside the standard library's (`sl_analysis::lint!(...)`). It
also takes the word after any `:` for a type, so a struct literal's values come out as types (`downsample: true`,
`extent: UNib32(extent)`), and `&mut` after a colon reads as a type named `mut`.

This lexer (alias `vz-rust`) gives each name its kind from its shape, as GitHub does:

- a name directly followed by `!` and an opening `(`, `[` or `{` is a macro call (`Name.Function.Magic`); `a != b`
  and `!x` stay operators;
- a name that starts upper-case is a type (`Name.Class`), a path segment before `::` and a tuple struct or variant
  before `(` included; one that is upper-case throughout (`MAX_LEN`) is a constant (`Name.Constant`);
- any other name followed by `(`, or by a turbofish `::<`, is a call (`Name.Function`);
- any other name stays plain, lower-case module path segments included;
- the word after a `:` is a keyword (`true`, `mut`, `impl`) or a name by these rules, never a type for its place
  alone; a `&` there is an operator.

The names Pygments already knows (a function's own name, `Option`, `Self`, `u32`) keep its colours.

pymdownx.highlight maps the `rust` fence to it:

    [project.markdown_extensions.pymdownx.highlight]
    extend_pygments_lang = [{ name = "rust", lang = "vz-rust" }]

Pygments finds it through the package's `pygments.lexers` entry point.
"""

from __future__ import annotations

from functools import lru_cache

from pygments.lexers.rust import RustLexer as PygmentsRustLexer
from pygments.token import Keyword, Name, Operator, Punctuation, Whitespace

OPENERS = {"(", "[", "{"}

# Pygments' keywords after which it names a declared type (`struct GridDef`): its `Name.Class` stays
DECLARATIONS = {"struct", "enum", "type", "union"}


class RustLexer(PygmentsRustLexer):
    name = "Rust (vz)"
    aliases = ["vz-rust"]
    filenames = []
    mimetypes = []

    def get_tokens_unprocessed(self, text, stack=("root",)):
        tokens = list(super().get_tokens_unprocessed(text, stack))
        i = 0
        while i < len(tokens):
            pos, kind, value = tokens[i]
            if kind is Name and _is_macro_call(tokens, i + 1):
                yield pos, Name.Function.Magic, value + "!"
                i += 2
                continue
            if kind is Name:
                kind = _by_shape(tokens, i)
            elif kind is Name.Class and not _declares(tokens, i):
                kind = _word(value)
                if kind is Name:
                    kind = _by_shape(tokens, i)
            elif kind is Keyword.Pseudo and value == "&":
                kind = Operator
            yield pos, kind, value
            i += 1


def _by_shape(tokens, at):
    """The kind of the plain name at `at`, from its case and the token after it"""
    value = tokens[at][2]
    after = _next_values(tokens, at, 3)
    if value[0].isupper():
        if after[:1] != ["("] and len(value) > 1 and value.upper() == value:
            return Name.Constant
        return Name.Class
    if after[:1] == ["("] or after == [":", ":", "<"] or after[:2] == ["::", "<"]:
        return Name.Function
    return Name


@lru_cache(maxsize=256)
def _word(value):
    """The kind Pygments gives `value` on its own, outside the place after a `:`"""
    kinds = [kind for kind, _ in PygmentsRustLexer().get_tokens(value) if kind not in Whitespace]
    return kinds[0] if len(kinds) == 1 else Name


def _declares(tokens, at):
    """Whether the type name at `at` is the one a `struct`, `enum`, `type` or `union` declares"""
    for _, kind, value in reversed(tokens[:at]):
        if kind in Whitespace or not value.strip():
            continue
        return kind is Keyword and value in DECLARATIONS
    return False


def _next_values(tokens, at, count):
    """The values of up to `count` tokens after `at` that are not whitespace"""
    values = []
    for _, kind, value in tokens[at + 1 :]:
        if kind in Whitespace or not value.strip():
            continue
        values.append(value)
        if len(values) == count:
            break
    return values


def _is_macro_call(tokens, bang):
    """Whether the token at `bang` is a `!` that an opening bracket follows, across whitespace"""
    if bang >= len(tokens) or tokens[bang][1:] != (Operator, "!"):
        return False
    for _, kind, value in tokens[bang + 1 :]:
        if kind in Whitespace or not value.strip():
            continue
        return kind in Punctuation and value in OPENERS
    return False
