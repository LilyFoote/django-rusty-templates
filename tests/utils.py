from django.template.loader import get_template


def render(template, context, *, using):
    template = get_template(template, using=using)
    return template.render(context)
