import pytest
from django.template import engines, TemplateSyntaxError


def test_upper_string():
    template = "{{ var|upper }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    var = "foo"
    uppered = "FOO"
    assert django_template.render({"var": var}) == uppered
    assert rust_template.render({"var": var}) == uppered


def test_upper_undefined():
    template = "{{ var|upper }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render() == ""
    assert rust_template.render() == ""


def test_upper_integer():
    template = "{{ var|upper }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    var = "3"
    uppered = "3"
    assert django_template.render({"var": var}) == uppered
    assert rust_template.render({"var": var}) == uppered


def test_upper_with_argument():
    template = "{{ var|upper:arg }}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "upper requires 1 arguments, 2 provided"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    expected = """\
  × upper filter does not take an argument
   ╭────
 1 │ {{ var|upper:arg }}
   ·              ─┬─
   ·               ╰── unexpected argument
   ╰────
"""
    assert str(exc_info.value) == expected


def test_upper_unicode():
    template = "{{ var|upper }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    var = "\xeb"
    uppered = "\xcb"
    assert django_template.render({"var": var}) == uppered
    assert rust_template.render({"var": var}) == uppered


def test_upper_html():
    template = "{{ var|upper }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    var = "<b>foo</b>"
    uppered = "&lt;B&gt;FOO&lt;/B&gt;"
    assert django_template.render({"var": var}) == uppered
    assert rust_template.render({"var": var}) == uppered


def test_upper_html_safe():
    template = "{{ var|upper|safe }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    var = "<b>foo</b>"
    uppered = "<B>FOO</B>"
    assert django_template.render({"var": var}) == uppered
    assert rust_template.render({"var": var}) == uppered


def test_upper_add_strings():
    template = "{{ var|upper|add:'bar' }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    var = "foo"
    uppered = "FOObar"
    assert django_template.render({"var": var}) == uppered
    assert rust_template.render({"var": var}) == uppered


def test_upper_add_numbers():
    template = "{{ var|upper|add:4 }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    var = "2"
    uppered = "6"
    assert django_template.render({"var": var}) == uppered
    assert rust_template.render({"var": var}) == uppered
