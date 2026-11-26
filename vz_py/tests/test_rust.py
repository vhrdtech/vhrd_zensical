from pygments.lexers import get_lexer_by_name
from pygments.token import Keyword, Name, Operator

from vhrd_zensical.rust import RustLexer


def marked(src):
    return [(t, v) for t, v in RustLexer().get_tokens(src) if v.strip()]


def kinds(src, kind):
    return [v for t, v in marked(src) if t is kind]


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


def test_a_name_before_a_bracket_is_a_call():
    src = "writer.put(&defs);\nlet w = ww_tape_fs::writer::create(path)?;\nlet v = it.collect::<Vec<_>>();\n"
    assert kinds(src, Name.Function) == ["put", "create", "collect"]
    assert (Name, "ww_tape_fs") in marked(src) and (Name, "writer") in marked(src)


def test_upper_case_names_are_types():
    src = "let g = GridDef::new(axis);\nfn f(a: &FixedAxis, b: Vec<FixedAxis>) {}\nimpl Foo for Bar {}\nAct::Put { t } => x\n"
    assert kinds(src, Name.Class) == ["GridDef", "FixedAxis", "FixedAxis", "Foo", "Bar", "Act", "Put"]
    assert kinds("struct grid_def { a: u32 }\n", Name.Class) == ["grid_def"]


def test_a_tuple_struct_before_a_bracket_is_a_type():
    assert kinds("let e = UNib32(extent);\n", Name.Class) == ["UNib32"]


def test_upper_case_throughout_is_a_constant():
    assert kinds("let n = MAX_LEN + 1;\n", Name.Constant) == ["MAX_LEN"]


def test_a_struct_literal_value_is_not_a_type():
    words = marked("GridDef { downsample: true, axis: axis, extent: UNib32(extent), n: len(x) }\n")
    assert (Keyword.Constant, "true") in words
    assert [t for t, v in words if v == "axis"] == [Name, Name]
    assert (Name.Class, "UNib32") in words and (Name.Function, "len") in words
    assert kinds("GridDef { axis: axis }\n", Name.Class) == ["GridDef"]


def test_a_reference_after_a_colon_is_an_operator_and_keyword():
    words = marked("fn f(x: &mut FixedAxis) {}\n")
    assert (Operator, "&") in words and (Keyword, "mut") in words and (Name.Class, "FixedAxis") in words
