import pytest
from django.template import engines
from django.template.exceptions import TemplateSyntaxError


def test_load_empty():
    template = "{% load %}"

    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({}) == ""
    assert rust_template.render({}) == ""


def test_load_missing():
    template = "{% load missing_filters %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == """\
'missing_filters' is not a registered tag library. Must be one of:
cache
custom_filters
i18n
l10n
more_filters
no_filters
no_tags
static
tz"""

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert str(exc_info.value) == """\
  × 'missing_filters' is not a registered tag library.
   ╭────
 1 │ {% load missing_filters %}
   ·         ───────┬───────
   ·                ╰── here
   ╰────
  help: Must be one of:
        cache
        custom_filters
        i18n
        l10n
        more_filters
        no_filters
        no_tags
        static
        tz
"""


def test_load_missing_filter():
    template = "{% load missing from custom_filters %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "'missing' is not a valid tag or filter in tag library 'custom_filters'"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert str(exc_info.value) == """\
  × 'missing' is not a valid tag or filter in tag library 'custom_filters'
   ╭────
 1 │ {% load missing from custom_filters %}
   ·         ───┬───      ───────┬──────
   ·            │                ╰── library
   ·            ╰── tag or filter
   ╰────
"""


def test_unknown_filter():
    template = "{{ foo|bar }}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Invalid filter: 'bar'"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert str(exc_info.value) == """\
  × Invalid filter: 'bar'
   ╭────
 1 │ {{ foo|bar }}
   ·        ─┬─
   ·         ╰── here
   ╰────
"""


def test_load_no_filters():
    template = "{% load no_filters %}"

    with pytest.raises(AttributeError):
        engines["django"].from_string(template)

    with pytest.raises(AttributeError):
        engines["rusty"].from_string(template)


def test_load_no_tags():
    template = "{% load no_tags %}"

    with pytest.raises(AttributeError):
        engines["django"].from_string(template)

    with pytest.raises(AttributeError):
        engines["rusty"].from_string(template)
