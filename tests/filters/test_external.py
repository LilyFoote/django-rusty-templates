import pytest
from django.template import engines
from django.template.base import VariableDoesNotExist


def test_load_and_render_filters():
    template = "{% load custom_filters %}{{ text|cut:'ello' }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    text = "Hello World!"
    expected = "H World!"
    assert django_template.render({"text": text}) == expected
    assert rust_template.render({"text": text}) == expected


def test_load_and_render_single_filter():
    template = "{% load cut from custom_filters %}{{ text|cut:'ello' }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    text = "Hello World!"
    expected = "H World!"
    assert django_template.render({"text": text}) == expected
    assert rust_template.render({"text": text}) == expected


def test_load_and_render_multiple_filters():
    template = """{% load cut double multiply from custom_filters %}
{{ text|cut:'ello' }}
{{ num|double }}
{{ num|multiply }}
{{ num|multiply:4 }}
"""
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    text = "Hello World!"
    expected = "\nH World!\n4\n6\n8\n"
    assert django_template.render({"text": text, "num": 2}) == expected
    assert rust_template.render({"text": text, "num": 2}) == expected


def test_load_and_render_multiple_filter_libraries():
    template = "{% load custom_filters more_filters %}{{ num|double|square }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({"num": 2}) == "16"
    assert rust_template.render({"num": 2}) == "16"


def test_resolve_filter_arg_error():
    template = """\
{% load multiply from custom_filters %}
{{ num|multiply:foo.bar.1b.baz }}
"""
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    with pytest.raises(VariableDoesNotExist) as exc_info:
        django_template.render({"num": 2, "foo": {"bar": 3}})

    assert str(exc_info.value) == "Failed lookup for key [1b] in 3"

    with pytest.raises(VariableDoesNotExist) as exc_info:
        rust_template.render({"num": 2, "foo": {"bar": 3}})

    assert str(exc_info.value) == """\
  × Failed lookup for key [1b] in 3
   ╭─[2:17]
 1 │ {% load multiply from custom_filters %}
 2 │ {{ num|multiply:foo.bar.1b.baz }}
   ·                 ───┬─── ─┬
   ·                    │     ╰── key
   ·                    ╰── 3
   ╰────
"""


def test_filter_error():
    template = "{% load custom_filters %}{{ num|divide_by_zero }}"

    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    with pytest.raises(ZeroDivisionError):
        django_template.render({"num": 1})

    with pytest.raises(ZeroDivisionError):
        rust_template.render({"num": 1})


def test_filter_error_with_argument():
    template = "{% load custom_filters %}{{ num|divide_by_zero:0 }}"

    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    with pytest.raises(ZeroDivisionError):
        django_template.render({"num": 1})

    with pytest.raises(ZeroDivisionError):
        rust_template.render({"num": 1})
