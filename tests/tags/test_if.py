import pytest
from django.template import engines
from django.template.exceptions import TemplateSyntaxError
from hypothesis import given
from hypothesis.strategies import (
    lists,
    one_of,
    none,
    floats,
    booleans,
    integers,
    text,
    tuples,
    just,
    characters,
)


def test_render_if_true():
    template = "{% if foo %}{{ foo }}{% endif %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    foo = "Foo"
    assert django_template.render({"foo": foo}) == foo
    assert rust_template.render({"foo": foo}) == foo


def test_render_if_false():
    template = "{% if foo %}{{ foo }}{% endif %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({}) == ""
    assert rust_template.render({}) == ""


def test_render_elif():
    template = "{% if False %}foo{% elif True %}bar{% else %}baz{% endif %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render() == "bar"
    assert rust_template.render() == "bar"



def test_render_if_true_literal():
    template = "{% if True %}foo{% endif %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({}) == "foo"
    assert rust_template.render({}) == "foo"


@pytest.mark.parametrize("a", [True, False, "foo", 1, "", 0])
@pytest.mark.parametrize("b", [True, False, "foo", 1, "", 0])
def test_render_and(a, b):
    template = "{% if a and b %}foo{% else %}bar{% endif %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    expected = "foo" if a and b else "bar"

    assert django_template.render({"a": a, "b": b}) == expected
    assert rust_template.render({"a": a, "b": b}) == expected


def test_render_and_literals():
    template = """{% if a and "b" and 'c' and 1 and 2.0 %}foo{% endif %}"""
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({"a": "a"}) == "foo"
    assert rust_template.render({"a": "a"}) == "foo"


def test_render_or_literals():
    template = """{% if a or "" or '' or 0 or 0.0 %}foo{% endif %}"""
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({"a": ""}) == ""
    assert rust_template.render({"a": ""}) == ""


@pytest.mark.parametrize("a", [True, False, "foo", 1, "", 0])
@pytest.mark.parametrize("b", [True, False, "foo", 1, "", 0])
def test_render_or(a, b):
    template = "{% if a or b %}foo{% else %}bar{% endif %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    expected = "foo" if a or b else "bar"

    assert django_template.render({"a": a, "b": b}) == expected
    assert rust_template.render({"a": a, "b": b}) == expected


@pytest.mark.parametrize("a", [True, False, "foo", 1, "", 0])
def test_render_not(a):
    template = "{% if not a %}foo{% else %}bar{% endif %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    expected = "foo" if not a else "bar"

    assert django_template.render({"a": a}) == expected
    assert rust_template.render({"a": a}) == expected


@pytest.mark.parametrize("a", [True, False, "foo", 1, "", 0])
@pytest.mark.parametrize("b", [True, False, "foo", 1, "", 0])
def test_render_equal(a, b):
    template = "{% if a == b %}foo{% else %}bar{% endif %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    expected = "foo" if a == b else "bar"

    assert django_template.render({"a": a, "b": b}) == expected
    assert rust_template.render({"a": a, "b": b}) == expected


@pytest.mark.parametrize("a", [True, False, "foo", 1, "", 0])
@pytest.mark.parametrize("b", [True, False, "foo", 1, "", 0])
def test_render_not_equal(a, b):
    template = "{% if a != b %}foo{% else %}bar{% endif %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    expected = "foo" if a != b else "bar"

    assert django_template.render({"a": a, "b": b}) == expected
    assert rust_template.render({"a": a, "b": b}) == expected


@pytest.mark.parametrize("a", [True, False, "foo", 1, "", 0])
@pytest.mark.parametrize("b", [True, False, "foo", 1, "", 0])
def test_render_less_than(a, b):
    template = "{% if a < b %}foo{% else %}bar{% endif %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    try:
        expected = "foo" if a < b else "bar"
    except TypeError:
        expected = "bar"

    assert django_template.render({"a": a, "b": b}) == expected
    assert rust_template.render({"a": a, "b": b}) == expected


@pytest.mark.parametrize("a", [True, False, "foo", 1, "", 0])
@pytest.mark.parametrize("b", [True, False, "foo", 1, "", 0])
def test_render_greater_than(a, b):
    template = "{% if a > b %}foo{% else %}bar{% endif %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    try:
        expected = "foo" if a > b else "bar"
    except TypeError:
        expected = "bar"

    assert django_template.render({"a": a, "b": b}) == expected
    assert rust_template.render({"a": a, "b": b}) == expected


@pytest.mark.parametrize("a", [True, False, "foo", 1, "", 0])
@pytest.mark.parametrize("b", [True, False, "foo", 1, "", 0])
def test_render_less_than_equal(a, b):
    template = "{% if a <= b %}foo{% else %}bar{% endif %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    try:
        expected = "foo" if a <= b else "bar"
    except TypeError:
        expected = "bar"

    assert django_template.render({"a": a, "b": b}) == expected
    assert rust_template.render({"a": a, "b": b}) == expected


@pytest.mark.parametrize("a", [True, False, "foo", 1, "", 0])
@pytest.mark.parametrize("b", [True, False, "foo", 1, "", 0])
def test_render_greater_than_equal(a, b):
    template = "{% if a >= b %}foo{% else %}bar{% endif %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    try:
        expected = "foo" if a >= b else "bar"
    except TypeError:
        expected = "bar"

    assert django_template.render({"a": a, "b": b}) == expected
    assert rust_template.render({"a": a, "b": b}) == expected


@pytest.mark.parametrize("a", ["foo", 1, "", 0])
@pytest.mark.parametrize("b", ["foobar", "bar", [1, 2], ["foobar", 1]])
def test_render_in(a, b):
    template = "{% if a in b %}foo{% else %}bar{% endif %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    try:
        expected = "foo" if a in b else "bar"
    except TypeError:
        expected = "bar"

    assert django_template.render({"a": a, "b": b}) == expected
    assert rust_template.render({"a": a, "b": b}) == expected


@pytest.mark.parametrize("a", ["foo", 1, "", 0])
@pytest.mark.parametrize("b", ["foobar", "bar", [1, 2], ["foobar", 1]])
def test_render_not_in(a, b):
    template = "{% if a not in b %}foo{% else %}bar{% endif %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    try:
        expected = "foo" if a not in b else "bar"
    except TypeError:
        expected = "bar"

    assert django_template.render({"a": a, "b": b}) == expected
    assert rust_template.render({"a": a, "b": b}) == expected


@pytest.mark.parametrize("a", [True, False, "foo", 1, "", 0, None])
@pytest.mark.parametrize("b", [True, False, "foo", 1, "", 0, None])
def test_render_is(a, b):
    template = "{% if a is b %}foo{% else %}bar{% endif %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    expected = "foo" if a is b else "bar"

    assert django_template.render({"a": a, "b": b}) == expected
    assert rust_template.render({"a": a, "b": b}) == expected


@pytest.mark.parametrize("a", [True, False, "foo", 1, "", 0, None])
@pytest.mark.parametrize("b", [True, False, "foo", 1, "", 0, None])
def test_render_is_not(a, b):
    template = "{% if a is not b %}foo{% else %}bar{% endif %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    expected = "foo" if a is not b else "bar"

    assert django_template.render({"a": a, "b": b}) == expected
    assert rust_template.render({"a": a, "b": b}) == expected


def test_invalid_and_position():
    template = "{% if and %}{{ foo }}{% endif %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Not expecting 'and' in this position in if tag."

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    expected = """\
  × Not expecting 'and' in this position
   ╭────
 1 │ {% if and %}{{ foo }}{% endif %}
   ·       ─┬─
   ·        ╰── here
   ╰────
"""
    assert str(exc_info.value) == expected


def test_invalid_or_position():
    template = "{% if or %}{{ foo }}{% endif %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Not expecting 'or' in this position in if tag."

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    expected = """\
  × Not expecting 'or' in this position
   ╭────
 1 │ {% if or %}{{ foo }}{% endif %}
   ·       ─┬
   ·        ╰── here
   ╰────
"""
    assert str(exc_info.value) == expected


def test_no_condition():
    template = "{% if %}{{ foo }}{% endif %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Unexpected end of expression in if tag."

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    expected = """\
  × Missing boolean expression
   ╭────
 1 │ {% if %}{{ foo }}{% endif %}
   · ────┬───
   ·     ╰── here
   ╰────
"""
    assert str(exc_info.value) == expected


def test_unexpected_end_of_expression():
    template = "{% if not %}{{ foo }}{% endif %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Unexpected end of expression in if tag."

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    expected = """\
  × Unexpected end of expression
   ╭────
 1 │ {% if not %}{{ foo }}{% endif %}
   ·       ─┬─
   ·        ╰── after this
   ╰────
"""
    assert str(exc_info.value) == expected


def test_invalid_in_position():
    template = "{% if in %}{{ foo }}{% endif %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Not expecting 'in' in this position in if tag."

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    expected = """\
  × Not expecting 'in' in this position
   ╭────
 1 │ {% if in %}{{ foo }}{% endif %}
   ·       ─┬
   ·        ╰── here
   ╰────
"""
    assert str(exc_info.value) == expected


def test_invalid_not_in_position():
    template = "{% if not in %}{{ foo }}{% endif %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Not expecting 'not in' in this position in if tag."

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    expected = """\
  × Not expecting 'not in' in this position
   ╭────
 1 │ {% if not in %}{{ foo }}{% endif %}
   ·       ───┬──
   ·          ╰── here
   ╰────
"""
    assert str(exc_info.value) == expected


def test_invalid_is_position():
    template = "{% if is %}{{ foo }}{% endif %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Not expecting 'is' in this position in if tag."

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    expected = """\
  × Not expecting 'is' in this position
   ╭────
 1 │ {% if is %}{{ foo }}{% endif %}
   ·       ─┬
   ·        ╰── here
   ╰────
"""
    assert str(exc_info.value) == expected


def test_invalid_is_not_position():
    template = "{% if is not %}{{ foo }}{% endif %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Not expecting 'is not' in this position in if tag."

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    expected = """\
  × Not expecting 'is not' in this position
   ╭────
 1 │ {% if is not %}{{ foo }}{% endif %}
   ·       ───┬──
   ·          ╰── here
   ╰────
"""
    assert str(exc_info.value) == expected


def test_no_operator():
    template = "{% if foo bar spam %}{{ foo }}{% endif %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Unused 'bar' at end of if expression."

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    expected = """\
  × Unused expression 'bar' in if tag
   ╭────
 1 │ {% if foo bar spam %}{{ foo }}{% endif %}
   ·           ─┬─
   ·            ╰── here
   ╰────
"""
    assert str(exc_info.value) == expected


VALID_VARIABLE_NAMES = text(
    alphabet=characters(max_codepoint=91, categories=["Ll", "Lu", "Nd"]),
    min_size=1,
).filter(lambda s: not s[0].isdigit())


VALID_ATOM = one_of(
    none(),
    booleans(),
    floats(),
    integers(),
    text().map("'{}'".format),
    text().map('"{}"'.format),
    VALID_VARIABLE_NAMES,
)

VALID_ATOM = one_of(VALID_ATOM, VALID_ATOM.map("not {}".format))

VALID_OPERATOR = one_of(
    just("and"),
    just("or"),
    just("=="),
    just("!="),
    just("<"),
    just(">"),
    just("<="),
    just(">="),
    just("in"),
    just("not in"),
    just("is"),
    just("is not"),
)


VALID_ATOM_NO_INTEGERS = one_of(
    none(),
    booleans(),
    floats(),
    text().map("'{}'".format),
    text().map('"{}"'.format),
    VALID_VARIABLE_NAMES,
)

VALID_OPERATOR_NO_IS = one_of(
    just("and"),
    just("or"),
    just("=="),
    just("!="),
    just("<"),
    just(">"),
    just("<="),
    just(">="),
    just("in"),
    just("not in"),
)


def to_template(parts):
    flat = []
    for var, op in parts:
        flat.append(str(var))
        flat.append(str(op))

    condition = " ".join(flat[:-1])
    return f"{{% if {condition} %}}truthy{{% else %}}falsey{{% endif %}}"


@given(lists(tuples(VALID_ATOM, VALID_OPERATOR_NO_IS)).map(to_template))
def test_render_same_result_no_is(template):
    try:
        django_template = engines["django"].from_string(template)
    except TemplateSyntaxError:
        with pytest.raises(TemplateSyntaxError):
            engines["rusty"].from_string(template)
    else:
        rust_template = engines["rusty"].from_string(template)

        context = {}
        assert rust_template.render(context) == django_template.render(context)


@given(lists(tuples(VALID_ATOM_NO_INTEGERS, VALID_OPERATOR)).map(to_template))
def test_render_same_result_no_integers(template):
    # We can't test `is` with integers without triggering failures due to Python's
    # small integer cache optimisation.
    try:
        django_template = engines["django"].from_string(template)
    except TemplateSyntaxError:
        with pytest.raises(TemplateSyntaxError):
            engines["rusty"].from_string(template)
    else:
        rust_template = engines["rusty"].from_string(template)

        context = {}
        assert rust_template.render(context) == django_template.render(context)


def test_render_none_is_not_none_equal_none():
    template = "{% if None is not None == None %}truthy{% else %}falsey{% endif %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({}) == "falsey"
    assert rust_template.render({}) == "falsey"


def test_render_none_equal_none_is_not_not_none():
    template = "{% if None == None is not not None %}truthy{% else %}falsey{% endif %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({}) == "falsey"
    assert rust_template.render({}) == "falsey"


def test_number_less_than_false():
    template = "{% if 1 < False %}truthy{% else %}falsey{% endif %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({}) == "falsey"
    assert rust_template.render({}) == "falsey"


def test_escaped_unicode_escape():
    template = "{% if '\\\x80' %}truthy{% else %}falsey{% endif %}"

    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({}) == "truthy"
    assert rust_template.render({}) == "truthy"


def test_incomplete_escape():
    template = "{% if '\\ %}truthy{% else %}falsey{% endif %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Could not parse the remainder: ''\\' from ''\\'"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    expected = """\
  × Expected a complete string literal
   ╭────
 1 │ {% if '\\ %}truthy{% else %}falsey{% endif %}
   ·       ─┬
   ·        ╰── here
   ╰────
"""
    assert str(exc_info.value) == expected


def test_zero_less_than_not_none():
    template = "{% if 0.0 < not None %}truthy{% else %}falsey{% endif %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({}) == "truthy"
    assert rust_template.render({}) == "truthy"


def test_zero_not_in_zero():
    template = "{% if 0.0 not in 0.0 %}truthy{% else %}falsey{% endif %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({}) == "falsey"
    assert rust_template.render({}) == "falsey"


def test_text_is_not_not_variable():
    template = '{% if "õeS" is not not WQWJXO52RWIA0D %}truthy{% else %}falsey{% endif %}'
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({}) == "truthy"
    assert rust_template.render({}) == "truthy"


def test_none_equal_none_not_in_zero():
    template = '{% if None == None not in 0.0 %}truthy{% else %}falsey{% endif %}'
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({}) == "falsey"
    assert rust_template.render({}) == "falsey"


def test_if_tag_split_by_newline():
    template = "{% if '\n' %}truthy{% else %}falsey{% endif %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Invalid block tag on line 2: 'else'. Did you forget to register or load this tag?"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    expected = """\
  × Unexpected tag else
   ╭─[2:11]
 1 │ {% if '
 2 │ ' %}truthy{% else %}falsey{% endif %}
   ·           ─────┬────
   ·                ╰── unexpected tag
   ╰────
"""
    assert str(exc_info.value) == expected


def test_var_lte_var():
    template = "{% if B <= A %}truthy{% else %}falsey{% endif %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({}) == "falsey"
    assert rust_template.render({}) == "falsey"
