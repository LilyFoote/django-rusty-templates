import pytest
from django.template import engines
from django.template.exceptions import TemplateSyntaxError


def test_escape():
    template = "{{ html|escape }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    html = "<p>Hello World!</p>"
    escaped = "&lt;p&gt;Hello World!&lt;/p&gt;"
    assert django_template.render({"html": html}) == escaped
    assert rust_template.render({"html": html}) == escaped


def test_escape_with_argument():
    template = "{{ html|escape:invalid }}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "escape requires 1 arguments, 2 provided"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    expected = """\
  × escape filter does not take an argument
   ╭────
 1 │ {{ html|escape:invalid }}
   ·                ───┬───
   ·                   ╰── unexpected argument
   ╰────
"""
    assert str(exc_info.value) == expected


def test_escape_missing_value():
    template = "{{ html|escape }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({}) == ""
    assert rust_template.render({}) == ""


def test_already_escaped():
    template = "{{ html|escape|escape }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    html = "<p>Hello World!</p>"
    escaped = "&lt;p&gt;Hello World!&lt;/p&gt;"
    assert django_template.render({"html": html}) == escaped
    assert rust_template.render({"html": html}) == escaped


def test_escape_integer():
    template = "{{ num|default:100|escape }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({}) == "100"
    assert rust_template.render({}) == "100"


def test_escape_float():
    template = "{{ num|default:1.6|escape }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({}) == "1.6"
    assert rust_template.render({}) == "1.6"


def test_escape_autoescape_off():
    template = "{% autoescape off %}{{ html|escape }}{% endautoescape %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    html = "<p>Hello World!</p>"
    escaped = "&lt;p&gt;Hello World!&lt;/p&gt;"
    assert django_template.render({"html": html}) == escaped
    assert rust_template.render({"html": html}) == escaped


def test_escape_autoescape_off_lower():
    template = "{% autoescape off %}{{ html|lower|escape }}{% endautoescape %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    html = "<p>Hello World!</p>"
    escaped = "&lt;p&gt;hello world!&lt;/p&gt;"
    assert django_template.render({"html": html}) == escaped
    assert rust_template.render({"html": html}) == escaped
