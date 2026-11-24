from pygments.lexers import get_lexer_by_name
from pygments.token import Name, Operator

from vhrd_zensical.rust import RustLexer


def marked(src):
    return [(t, v) for t, v in RustLexer().get_tokens(src) if v.strip()]


def test_any_macro_call_is_marked():
    words = marked('sl_analysis::lint!(A, "x");\nlet v = my_vec! [1];\nm! { }\n')
    assert [v for t, v in words if t is Name.Function.Magic] == ["lint!", "my_vec!", "m!"]


def test_the_standard_macros_stay_marked():
    assert (Name.Function.Magic, "println!") in marked('println!("{}", 1);\n')


def test_not_equal_and_negation_stay_operators():
    words = marked("if a != b && !c { }\n")
    assert not [v for t, v in words if t is Name.Function.Magic]
    assert (Name, "a") in words and (Operator, "!") in words


def test_pygments_finds_it_by_its_alias():
    assert isinstance(get_lexer_by_name("vz-rust"), RustLexer)
