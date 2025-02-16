import pytest
from django.template import engines
from django.template.exceptions import TemplateSyntaxError
from django.template.base import VariableDoesNotExist


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
        django_template = engines["django"].from_string(template)

    assert str(exc_info.value) == "escape requires 1 arguments, 2 provided"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        rust_template = engines["rusty"].from_string(template)

    assert str(exc_info.value) == """
  × escape filter does not take an argument
   ╭────
 1 │ {{ html|escape:invalid }}
   ·                ───┬───
   ·                   ╰── unexpected argument
   ╰────
"""


def test_escape_missing_value():
    template = "{{ html|escape }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({}) == ""
    assert rust_template.render({}) == ""
