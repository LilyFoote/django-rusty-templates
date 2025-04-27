from django.template import engines


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
