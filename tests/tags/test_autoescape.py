import pytest
from django.template import engines
from django.template.exceptions import TemplateSyntaxError


def test_autoescape_off():
    html = "<p>Hello World!</p>"
    template = "{% autoescape off %}{{ html }}{% endautoescape %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({"html": html}) == html
    assert rust_template.render({"html": html}) == html


def test_missing_argument():
    template = "{% autoescape %}{{ html }}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "'autoescape' tag requires exactly one argument."

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert str(exc_info.value) == """\
  × 'autoescape' tag missing an 'on' or 'off' argument.
   ╭────
 1 │ {% autoescape %}{{ html }}
   ·              ▲
   ·              ╰── here
   ╰────
"""


def test_invalid_argument():
    template = "{% autoescape foo %}{{ html }}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "'autoescape' argument should be 'on' or 'off'"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert str(exc_info.value) == """\
  × 'autoescape' argument should be 'on' or 'off'.
   ╭────
 1 │ {% autoescape foo %}{{ html }}
   ·               ─┬─
   ·                ╰── here
   ╰────
"""


def test_extra_argument():
    template = "{% autoescape on off %}{{ html }}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "'autoescape' tag requires exactly one argument."

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert str(exc_info.value) == """\
  × 'autoescape' tag requires exactly one argument.
   ╭────
 1 │ {% autoescape on off %}{{ html }}
   ·               ───┬──
   ·                  ╰── here
   ╰────
"""


def test_missing_endautoescape():
    template = "{% autoescape off %}{{ html }}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Unclosed tag on line 1: 'autoescape'. Looking for one of: endautoescape."

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert str(exc_info.value) == """\
  × Unclosed 'autoescape' tag. Looking for one of: endautoescape
   ╭────
 1 │ {% autoescape off %}{{ html }}
   · ──────────┬─────────
   ·           ╰── started here
   ╰────
"""


def test_wrong_end_tag():
    template = "{% autoescape off %}{{ html }}{% endverbatim %}{% endautoescape %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Invalid block tag on line 1: 'endverbatim', expected 'endautoescape'. Did you forget to register or load this tag?"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert str(exc_info.value) == """\
  × Unexpected tag endverbatim, expected endautoescape
   ╭────
 1 │ {% autoescape off %}{{ html }}{% endverbatim %}{% endautoescape %}
   · ──────────┬─────────          ────────┬────────
   ·           │                           ╰── unexpected tag
   ·           ╰── start tag
   ╰────
"""


def test_endautoescape_argument():
    html = "<p>Hello World!</p>"
    template = "{% autoescape off %}{{ html }}{% endautoescape extra %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({"html": html}) == html
    assert rust_template.render({"html": html}) == html


def test_nested_autoescape():
    html = "<p>Hello World!</p>"
    template = "{{ html }}{% autoescape off %}{{ html }}{% autoescape on %}{{ html }}{% endautoescape %}{% endautoescape %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    escaped = "&lt;p&gt;Hello World!&lt;/p&gt;"
    assert django_template.render({"html": html}) == f"{escaped}{html}{escaped}"
    assert rust_template.render({"html": html}) == f"{escaped}{html}{escaped}"


def test_autoescape_text():
    template = "{% autoescape off %}<p>Hello World!</p>{% endautoescape %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    html = "<p>Hello World!</p>"
    assert django_template.render({}) == html
    assert rust_template.render({}) == html


def test_autoescape_comment():
    template = "{% autoescape off %}{# comment #}{% endautoescape %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({}) == ""
    assert rust_template.render({}) == ""


def test_autoescape_url():
    template = "{% autoescape off %}{% url 'home' %}{% endautoescape %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({}) == "/"
    assert rust_template.render({}) == "/"


def test_unexpected_end_tag():
    template = "{% endautoescape %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Invalid block tag on line 1: 'endautoescape'. Did you forget to register or load this tag?"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert str(exc_info.value) == """\
  × Unexpected tag endautoescape
   ╭────
 1 │ {% endautoescape %}
   · ─────────┬─────────
   ·          ╰── unexpected tag
   ╰────
"""
