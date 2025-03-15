import pytest
from django.template import engines
from django.template.base import VariableDoesNotExist
from django.template.exceptions import TemplateSyntaxError


def test_add_integers():
    template = "{{ foo|add:3 }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({"foo": 2}) == "5"
    assert rust_template.render({"foo": 2}) == "5"


def test_add_no_variable():
    template = "{{ foo|add:3 }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({}) == ""
    assert rust_template.render({}) == ""


def test_add_no_argument():
    template = "{{ foo|add:bar }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    with pytest.raises(VariableDoesNotExist) as exc_info:
        django_template.render({"foo": 1})

    expected = "Failed lookup for key [bar] in [{'True': True, 'False': False, 'None': None}, {'foo': 1}]"
    assert str(exc_info.value) == expected

    with pytest.raises(VariableDoesNotExist) as exc_info:
        rust_template.render({"foo": 1})

    expected = """\
  × Failed lookup for key [bar] in {"False": False, "None": None, "True":
  │ True, "foo": 1}
   ╭────
 1 │ {{ foo|add:bar }}
   ·            ─┬─
   ·             ╰── key
   ╰────
"""
    assert str(exc_info.value) == expected


def test_add_integer_strings():
    template = "{{ foo|add:'3' }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({"foo": "2"}) == "5"
    assert rust_template.render({"foo": "2"}) == "5"


def test_add_strings():
    template = "{{ foo|add:'def' }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({"foo": "abc"}) == "abcdef"
    assert rust_template.render({"foo": "abc"}) == "abcdef"


def test_add_lists():
    template = "{{ foo|add:bar}}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({"foo": [1], "bar": [2]}) == "[1, 2]"
    assert rust_template.render({"foo": [1], "bar": [2]}) == "[1, 2]"


def test_add_incompatible():
    template = "{{ foo|add:bar}}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({"foo": [1], "bar": 2}) == ""
    assert rust_template.render({"foo": [1], "bar": 2}) == ""


def test_add_float():
    template = "{{ foo|add:bar}}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({"foo": 1.2, "bar": 2.9}) == "3"
    assert rust_template.render({"foo": 1.2, "bar": 2.9}) == "3"


def test_add_float_literal():
    template = "{{ foo|add:2.9 }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({"foo": 1.2}) == "3"
    assert rust_template.render({"foo": 1.2}) == "3"


def test_add_incompatible_int():
    template = "{{ foo|add:2}}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({"foo": [1]}) == ""
    assert rust_template.render({"foo": [1]}) == ""


def test_add_incompatible_float():
    template = "{{ foo|add:2.9}}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({"foo": [1]}) == ""
    assert rust_template.render({"foo": [1]}) == ""


def test_add_missing_argument():
    template = "{{ foo|add }}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "add requires 2 arguments, 1 provided"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    expected = """\
  × Expected an argument
   ╭────
 1 │ {{ foo|add }}
   ·        ─┬─
   ·         ╰── here
   ╰────
"""
    assert str(exc_info.value) == expected


def test_add_safe():
    template = "{{ html|safe|add:'<p>More HTML</p>' }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    html = "<p>Some HTML</p>"
    expected = "<p>Some HTML</p><p>More HTML</p>"
    assert django_template.render({"html": html}) == expected
    assert rust_template.render({"html": html}) == expected


def test_add_integer_strings_autoescape_off():
    template = "{% autoescape off %}{{ foo|add:'3' }}{% endautoescape %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({"foo": "2"}) == "5"
    assert rust_template.render({"foo": "2"}) == "5"


def test_add_strings_autoescape_off():
    template = "{% autoescape off %}{{ foo|add:'def' }}{% endautoescape %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({"foo": "abc"}) == "abcdef"
    assert rust_template.render({"foo": "abc"}) == "abcdef"
