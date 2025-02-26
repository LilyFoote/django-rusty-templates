import pytest
from django.template import engines
from django.template.exceptions import TemplateSyntaxError


def test_safe():
    template = "{{ html|safe }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    html = "<p>Hello World!</p>"
    assert django_template.render({"html": html}) == html
    assert rust_template.render({"html": html}) == html


def test_safe_with_argument():
    template = "{{ html|safe:invalid }}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "safe requires 1 arguments, 2 provided"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert str(exc_info.value) == """\
  × safe filter does not take an argument
   ╭────
 1 │ {{ html|safe:invalid }}
   ·              ───┬───
   ·                 ╰── unexpected argument
   ╰────
"""


def test_safe_missing_value():
    template = "{{ html|safe }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({}) == ""
    assert rust_template.render({}) == ""


def test_safe_already_safe():
    template = "{{ html|safe|safe }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    html = "<p>Hello World!</p>"
    assert django_template.render({"html": html}) == html
    assert rust_template.render({"html": html}) == html


def test_safe_integer():
    template = "{{ num|default:100|safe }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({}) == "100"
    assert rust_template.render({}) == "100"


def test_safe_float():
    template = "{{ num|default:1.6|safe }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({}) == "1.6"
    assert rust_template.render({}) == "1.6"


def test_safe_escaped():
    template = "{{ html|escape|safe }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    html = "<p>Hello World!</p>"
    escaped = "&lt;p&gt;Hello World!&lt;/p&gt;"
    assert django_template.render({"html": html}) == escaped
    assert rust_template.render({"html": html}) == escaped


def test_safe_autoescape_off():
    template = "{% autoescape off %}{{ html|safe }}{% endautoescape %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    html = "<p>Hello World!</p>"
    assert django_template.render({"html": html}) == html
    assert rust_template.render({"html": html}) == html


def test_safe_autoescape_off_lower():
    template = "{% autoescape off %}{{ html|lower|safe }}{% endautoescape %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    html = "<p>Hello World!</p>"
    assert django_template.render({"html": html}) == html.lower()
    assert rust_template.render({"html": html}) == html.lower()
