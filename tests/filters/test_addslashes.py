"""
Test cases adapted from
https://github.com/django/django/blob/main/tests/template_tests/filter_tests/test_addslashes.py
"""

import pytest
from django.utils.safestring import mark_safe


@pytest.mark.xfail(reason="autoescape not there yet")
def test_addslashes01(self):
    """
    @setup(
        {
            "addslashes01": (
                "{% autoescape off %}{{ a|addslashes }} {{ b|addslashes }}"
                "{% endautoescape %}"
            )
        }
    )
    """
    output = self.engine.render_to_string(
        "addslashes01", {"a": "<a>'", "b": mark_safe("<a>'")}
    )
    self.assertEqual(output, r"<a>\' <a>\'")


@pytest.mark.xfail(reason="autoescape not there yet")
def test_addslashes02(self):
    """@setup({"addslashes02": "{{ a|addslashes }} {{ b|addslashes }}"})"""
    template = "{{ a|addslashes }} {{ b|addslashes }}"

    django_template = engines["django"].from_string(template)  # noqa
    rust_template = engines["rusty"].from_string(template)  # noqa

    output = self.engine.render_to_string(
        "addslashes02", {"a": "<a>'", "b": mark_safe("<a>'")}
    )
    self.assertEqual(output, r"&lt;a&gt;\&#x27; <a>\'")


def test_quotes(assert_render):
    template = "{{ a|addslashes }} and {{ b|addslashes }}"
    expected = r"\"double quotes\" and \'single quotes\'"
    context = {"a": mark_safe('"double quotes"'), "b": mark_safe("'single quotes'")}

    assert_render(template, context, expected)


def test_backslashes(assert_render):
    template = "{{ a|addslashes }}"
    context = {"a": r"\ : backslashes, too"}
    expected = r"\\ : backslashes, too"

    assert_render(template, context, expected)


def test_non_string_input(assert_render):
    template = "{{ a|addslashes }}"
    context = {"a": 123}

    assert_render(template, context, "123")
