"""
Adapted from
https://github.com/django/django/blob/5.1/tests/template_tests/filter_tests/test_slugify.py
"""

from django.utils.functional import lazy
from django.utils.safestring import mark_safe


def test_slugify01(assert_render):
    """
    Running slugify on a pre-escaped string leads to odd behavior,
    but the result is still safe.
    """
    template = "{% autoescape off %}{{ a|slugify }} {{ b|slugify }}{% endautoescape %}"
    context = {"a": "a & b", "b": mark_safe("a &amp; b")}
    assert_render(template, context, "a-b a-amp-b")


def test_slugify02(assert_render):
    template = "{{ a|slugify }} {{ b|slugify }}"
    context = {"a": "a & b", "b": mark_safe("a &amp; b")}
    assert_render(template, context, "a-b a-amp-b")


def test_slugify(assert_render):
    template = "{{ test|slugify }}"
    context = {
        "test": " Jack & Jill like numbers 1,2,3 and 4 and silly characters ?%.$!/"
    }
    expected = "jack-jill-like-numbers-123-and-4-and-silly-characters"
    assert_render(template, context, expected)


def test_unicode(assert_render):
    template = "{{ test|slugify }}"
    context = {"test": "Un \xe9l\xe9phant \xe0 l'or\xe9e du bois"}
    expected = "un-elephant-a-loree-du-bois"

    assert_render(template, context, expected)


def test_non_string_input(assert_render):
    template = "{{ test|slugify }}"
    context = {"test": 123}
    expected = "123"
    assert_render(template, context, expected)


def test_slugify_lazy_string(assert_render):
    lazy_str = lazy(lambda string: string, str)
    template = "{{ test|slugify }}"
    context = {
        "test": lazy_str(
            " Jack & Jill like numbers 1,2,3 and 4 and silly characters ?%.$!/"
        )
    }
    expected = "jack-jill-like-numbers-123-and-4-and-silly-characters"
    assert_render(template, context, expected)


def test_danish_name(assert_render):
    template = "{{ test|slugify }}"
    context = {"test": "Lærke Sørensen"}
    expected = "lrke-srensen"

    assert_render(template, context, expected)


def test_list(assert_render):
    template = "{{ test|slugify }}"
    context = {"test": ["hello world", "muu"]}
    expected = "hello-world-muu"

    assert_render(template, context, expected)


def test_dictionary(assert_render):
    template = "{{ test|slugify }}"
    context = {"test": {"key": "value"}}
    expected = "key-value"

    assert_render(template, context, expected)
