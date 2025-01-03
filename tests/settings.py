TEMPLATES = [
    {
        "BACKEND": "django.template.backends.django.DjangoTemplates",
        "DIRS": ["tests/templates"],
        "OPTIONS": {
            "libraries": {
                "custom_filters": "tests.templatetags.custom_filters",
            },
        },
    },
    {
        "BACKEND": "django_rusty_templates.RustyTemplates",
        "DIRS": ["tests/templates"],
        "NAME": "rusty",
        "OPTIONS": {
            "libraries": {
                "custom_filters": "tests.templatetags.custom_filters",
            },
        },
    },
]

ROOT_URLCONF = "tests.urls"
