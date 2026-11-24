from pygments.lexers import get_lexer_by_name
from pygments.token import Generic, Name, Operator, Text

from vhrd_zensical.shell import ConsoleLexer


def marked(src):
    return [(t, v) for t, v in ConsoleLexer().get_tokens(src) if v.strip()]


def test_command_subcommands_flags_and_value():
    assert marked("$ spl init --tier small-workspace\n") == [
        (Generic.Prompt, "$ "),
        (Name.Function, "spl"),
        (Name.Tag, "init"),
        (Name.Attribute, "--tier"),
        (Text, "small-workspace"),
    ]


def test_a_pipe_and_a_runner_start_new_commands():
    words = marked("$ spl tools needed --spec | xargs -r cargo binstall -y\n")
    assert (Operator, "|") in words
    assert [v for t, v in words if t is Name.Function] == ["spl", "xargs", "cargo"]
    assert [v for t, v in words if t is Name.Tag] == ["tools", "needed", "binstall"]


def test_paths_and_arguments_after_a_flag_stay_text():
    words = marked("$ cd ~/git/app_a && cargo nextest run -p a_app\n")
    assert (Text, "~/git/app_a") in words and (Text, "a_app") in words
    assert [v for t, v in words if t is Name.Function] == ["cd", "cargo"]


def test_lines_without_a_prompt_are_output():
    assert marked("$ tool --version\ntool 0.4.2\n")[-1] == (Generic.Output, "tool 0.4.2\n")


def test_pygments_finds_it_by_its_alias():
    assert isinstance(get_lexer_by_name("vz-console"), ConsoleLexer)
