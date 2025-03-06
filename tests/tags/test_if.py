import pytest
from django.template import engines
from django.template.exceptions import TemplateSyntaxError


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


@pytest.mark.parametrize("a", [True, False, "foo", 1, "", 0])
@pytest.mark.parametrize("b", [True, False, "foo", 1, "", 0])
def test_render_and(a, b):
    template = "{% if a and b %}foo{% else %}bar{% endif %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    expected = "foo" if a and b else "bar"

    assert django_template.render({"a": a, "b": b}) == expected
    assert rust_template.render({"a": a, "b": b}) == expected


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

    assert str(exc_info.value) == """\
  × Not expecting 'and' in this position
   ╭────
 1 │ {% if and %}{{ foo }}{% endif %}
   ·       ─┬─
   ·        ╰── here
   ╰────
"""


def test_invalid_or_position():
    template = "{% if or %}{{ foo }}{% endif %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Not expecting 'or' in this position in if tag."

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert str(exc_info.value) == """\
  × Not expecting 'or' in this position
   ╭────
 1 │ {% if or %}{{ foo }}{% endif %}
   ·       ─┬
   ·        ╰── here
   ╰────
"""


def test_no_condition():
    template = "{% if %}{{ foo }}{% endif %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Unexpected end of expression in if tag."

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert str(exc_info.value) == """\
  × Missing boolean expression
   ╭────
 1 │ {% if %}{{ foo }}{% endif %}
   · ────┬───
   ·     ╰── here
   ╰────
"""


def test_unexpected_end_of_expression():
    template = "{% if not %}{{ foo }}{% endif %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Unexpected end of expression in if tag."

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert str(exc_info.value) == """\
  × Unexpected end of expression
   ╭────
 1 │ {% if not %}{{ foo }}{% endif %}
   ·       ─┬─
   ·        ╰── after this
   ╰────
"""


def test_invalid_in_position():
    template = "{% if in %}{{ foo }}{% endif %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Not expecting 'in' in this position in if tag."

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert str(exc_info.value) == """\
  × Not expecting 'in' in this position
   ╭────
 1 │ {% if in %}{{ foo }}{% endif %}
   ·       ─┬
   ·        ╰── here
   ╰────
"""


def test_invalid_not_in_position():
    template = "{% if not in %}{{ foo }}{% endif %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Not expecting 'not in' in this position in if tag."

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert str(exc_info.value) == """\
  × Not expecting 'not in' in this position
   ╭────
 1 │ {% if not in %}{{ foo }}{% endif %}
   ·       ───┬──
   ·          ╰── here
   ╰────
"""


def test_invalid_is_position():
    template = "{% if is %}{{ foo }}{% endif %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Not expecting 'is' in this position in if tag."

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert str(exc_info.value) == """\
  × Not expecting 'is' in this position
   ╭────
 1 │ {% if is %}{{ foo }}{% endif %}
   ·       ─┬
   ·        ╰── here
   ╰────
"""


def test_invalid_is_not_position():
    template = "{% if is not %}{{ foo }}{% endif %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Not expecting 'is not' in this position in if tag."

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert str(exc_info.value) == """\
  × Not expecting 'is not' in this position
   ╭────
 1 │ {% if is not %}{{ foo }}{% endif %}
   ·       ───┬──
   ·          ╰── here
   ╰────
"""


def test_no_operator():
    template = "{% if foo bar spam %}{{ foo }}{% endif %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Unused 'bar' at end of if expression."

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert str(exc_info.value) == """\
  × Unused expression 'bar' in if tag
   ╭────
 1 │ {% if foo bar spam %}{{ foo }}{% endif %}
   ·           ─┬─
   ·            ╰── here
   ╰────
"""
