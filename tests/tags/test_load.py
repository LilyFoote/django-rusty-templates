import pytest
from django.template import engines
from django.template.exceptions import TemplateSyntaxError


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
static
tz"""

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert str(exc_info.value) == """
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
        static
        tz
"""


def test_unknown_filter():
    template = "{{ foo|bar }}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Invalid filter: 'bar'"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert str(exc_info.value) == """
  × Invalid filter: 'bar'
   ╭────
 1 │ {{ foo|bar }}
   ·        ─┬─
   ·         ╰── here
   ╰────
"""
