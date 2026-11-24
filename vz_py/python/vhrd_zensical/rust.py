"""Rust with every macro call marked, not only the standard library's.

Pygments' Rust lexer knows the standard macros by name (`vec!`, `println!`): any other call, such as
`sl_analysis::lint!(...)`, comes out as a plain name and a `!` operator. This lexer (alias `vz-rust`) marks a name
directly followed by `!` and an opening `(`, `[` or `{` as a macro call (`Name.Function.Magic`), as the standard
ones are; `a != b` and `!x` stay operators.

pymdownx.highlight maps the `rust` fence to it:

    [project.markdown_extensions.pymdownx.highlight]
    extend_pygments_lang = [{ name = "rust", lang = "vz-rust" }]

Pygments finds it through the package's `pygments.lexers` entry point.
"""

from __future__ import annotations

from pygments.lexers.rust import RustLexer as PygmentsRustLexer
from pygments.token import Name, Operator, Punctuation, Whitespace

OPENERS = {"(", "[", "{"}


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
            if kind is Name and _is_call(tokens, i + 1):
                yield pos, Name.Function.Magic, value + "!"
                i += 2
                continue
            yield pos, kind, value
            i += 1


def _is_call(tokens, bang):
    """Whether the token at `bang` is a `!` that an opening bracket follows, across whitespace"""
    if bang >= len(tokens) or tokens[bang][1:] != (Operator, "!"):
        return False
    for _, kind, value in tokens[bang + 1 :]:
        if kind in Whitespace or not value.strip():
            continue
        return kind in Punctuation and value in OPENERS
    return False
