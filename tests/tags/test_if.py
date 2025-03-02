from django.template import engines


def test_render_if_true():
    template = "{% if foo %}{{ foo }}{% endif %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    foo = "Foo"
    assert django_template.render({"foo": foo}) == foo
    assert rust_template.render({"foo": foo}) == foo


def test_render_if_false():
    template = "{% if foo %}{{ foo }}{% endif %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({}) == ""
    assert rust_template.render({}) == ""
