from django.template import engines


def test_load_filters():
    template = "{% load custom_filters %}{{ text|cut:'ello' }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    text = "Hello World!"
    expected = "H World!"
    assert django_template.render({"text": text}) == expected
    assert rust_template.render({"text": text}) == expected
