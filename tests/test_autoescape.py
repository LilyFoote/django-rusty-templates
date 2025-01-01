import pytest
from django.template import engines
from django.utils.safestring import mark_safe


def test_mark_safe():
    html = mark_safe("<p>Hello World!</p>")
    template = "{{ html }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    expected = "<p>Hello World!</p>"
    assert django_template.render({"html": html}) == expected
    assert rust_template.render({"html": html}) == expected


def test_autoescape():
    html = "<p>Hello World!</p>"
    template = "{{ html }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    expected = "&lt;p&gt;Hello World!&lt;/p&gt;"
    assert django_template.render({"html": html}) == expected
    assert rust_template.render({"html": html}) == expected


def test_autoescape_not_string():
    class Html:
        def __init__(self, html):
            self.html = html

        def __str__(self):
            return self.html

    html = Html("<p>Hello World!</p>")
    template = "{{ html }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    expected = "&lt;p&gt;Hello World!&lt;/p&gt;"
    assert django_template.render({"html": html}) == expected
    assert rust_template.render({"html": html}) == expected


def test_autoescape_invalid_str_method():
    class Broken:
        def __str__(self):
            1/0

    broken = Broken()
    template = "{{ broken }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    with pytest.raises(ZeroDivisionError):
        django_template.render({"broken": broken})
    with pytest.raises(ZeroDivisionError):
        rust_template.render({"broken": broken})


def test_autoescape_invalid_html_method():
    class Broken(str):
        def __html__(self):
            1/0

    broken = Broken("")
    template = "{{ broken }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    with pytest.raises(ZeroDivisionError):
        django_template.render({"broken": broken})
    with pytest.raises(ZeroDivisionError):
        rust_template.render({"broken": broken})


def test_mark_safe_filter_lower():
    html = mark_safe("<p>Hello World!</p>")
    template = "{{ html|lower }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    expected = "<p>hello world!</p>"
    assert django_template.render({"html": html}) == expected
    assert rust_template.render({"html": html}) == expected


def test_autoescape_filter_lower():
    html = "<p>Hello World!</p>"
    template = "{{ html|lower }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    expected = "&lt;p&gt;hello world!&lt;/p&gt;"
    assert django_template.render({"html": html}) == expected
    assert rust_template.render({"html": html}) == expected
