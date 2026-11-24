"""A shell session as a reader scans it: the command, its subcommands and its flags each in their own colour.

Pygments' `console` lexer leaves every word of a command line as plain text: only comments, strings and variables
stand out. This lexer (alias `vz-console`) takes the same `$ ` lines and output, and marks each command's words by
their place: the command (`Name.Function`), the subcommands that follow it before any flag (`Name.Tag`), the flags
(`Name.Attribute`). A pipe, `&&`, `||` or `;` starts a new command, and so does the command that `xargs`, `sudo`,
`env`, `time` or `exec` runs. Lines without a prompt are output (`Generic.Output`).

pymdownx.highlight maps the `console` fence to it, so a page keeps the fence GitHub knows:

    [project.markdown_extensions.pymdownx.highlight]
    extend_pygments_lang = [{ name = "console", lang = "vz-console" }]

Pygments finds it through the package's `pygments.lexers` entry point.
"""

from __future__ import annotations

import re

from pygments.lexer import Lexer
from pygments.lexers.shell import BashLexer
from pygments.token import Generic, Name, Operator, Punctuation, Text, Token

PROMPT = "$ "
# Commands that run the command after their own flags
RUNNERS = {"xargs", "sudo", "env", "time", "exec"}
# Operators after which a new command starts
SEPARATORS = {"|", "||", "&&", ";", "(", "$("}
SUBCOMMAND = re.compile(r"[a-z][a-z0-9-]*\Z")


class ConsoleLexer(Lexer):
    name = "Shell session (vz)"
    aliases = ["vz-console"]
    filenames = []
    mimetypes = []

    def get_tokens_unprocessed(self, text):
        bash = BashLexer(**self.options)
        pos = 0
        for line in text.splitlines(keepends=True):
            if line.startswith(PROMPT):
                yield pos, Generic.Prompt, PROMPT
                yield from _command(bash, line[len(PROMPT) :], pos + len(PROMPT))
            else:
                yield pos, Generic.Output, line
            pos += len(line)


def _command(bash, line, start):
    """One command line's tokens, its words marked by their place"""
    at_command, in_subcommands = True, False
    for i, kind, value in bash.get_tokens_unprocessed(line):
        word = value.strip()
        if not word or kind in Token.Comment or kind in Token.Literal.String or kind in Name.Variable:
            yield start + i, kind, value
            continue
        if word in SEPARATORS and (kind in Operator or kind in Punctuation or kind is Text):
            at_command, in_subcommands = True, False
            yield start + i, Operator, value
            continue
        if kind is not Text and kind not in Name.Builtin:
            yield start + i, kind, value
            continue
        if word.startswith("-"):
            in_subcommands = False
            yield start + i, Name.Attribute, value
        elif at_command:
            at_command, in_subcommands = word in RUNNERS, word not in RUNNERS
            yield start + i, Name.Function, value
        elif in_subcommands and SUBCOMMAND.match(word):
            yield start + i, Name.Tag, value
        else:
            in_subcommands = False
            yield start + i, Text, value
