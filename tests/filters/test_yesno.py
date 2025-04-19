from django.template import Template, Context
from django.test import SimpleTestCase


class YesNoTests(SimpleTestCase):
    def test_yesno_default(self):
        # Test default mapping (yes, no, maybe)
        template = Template("{{ var|yesno }}")

        # Test with True value
        rendered = template.render(Context({"var": True}))
        self.assertEqual(rendered, "yes")
        
        # Test with False value
        rendered = template.render(Context({"var": False}))
        self.assertEqual(rendered, "no")
        
        # Test with None value
        rendered = template.render(Context({"var": None}))
        self.assertEqual(rendered, "maybe")

    def test_yesno_custom(self):
        # Test custom mapping
        template = Template("{{ var|yesno:'yeah,nope,perhaps' }}")

        # Test with True value
        rendered = template.render(Context({"var": True}))
        self.assertEqual(rendered, "yeah")
        
        # Test with False value
        rendered = template.render(Context({"var": False}))
        self.assertEqual(rendered, "nope")
        
        # Test with None value
        rendered = template.render(Context({"var": None}))
        self.assertEqual(rendered, "perhaps")

    def test_yesno_two_options(self):
        # Test with only two options - None uses the second option
        template = Template("{{ var|yesno:'yep,nah' }}")

        # Test with True value
        rendered = template.render(Context({"var": True}))
        self.assertEqual(rendered, "yep")
        
        # Test with False value
        rendered = template.render(Context({"var": False}))
        self.assertEqual(rendered, "nah")
        
        # Test with None value - should use second option when no third option
        rendered = template.render(Context({"var": None}))
        self.assertEqual(rendered, "nah")

    def test_yesno_empty_string(self):
        # Test with empty string (which is falsy)
        template = Template("{{ var|yesno }}")
        rendered = template.render(Context({"var": ""}))
        self.assertEqual(rendered, "no")

    def test_yesno_empty_list(self):
        # Test with empty list (which is falsy)
        template = Template("{{ var|yesno }}")
        rendered = template.render(Context({"var": []}))
        self.assertEqual(rendered, "no")

    def test_yesno_non_empty_list(self):
        # Test with non-empty list (which is truthy)
        template = Template("{{ var|yesno }}")
        rendered = template.render(Context({"var": ["item"]}))
        self.assertEqual(rendered, "yes")

    def test_yesno_non_empty_string(self):
        # Test with non-empty string (which is truthy)
        template = Template("{{ var|yesno }}")
        rendered = template.render(Context({"var": "value"}))
        self.assertEqual(rendered, "yes")

    def test_yesno_zero(self):
        # Test with zero (which is falsy)
        template = Template("{{ var|yesno }}")
        rendered = template.render(Context({"var": 0}))
        self.assertEqual(rendered, "no")

    def test_yesno_nonzero(self):
        # Test with non-zero number (which is truthy)
        template = Template("{{ var|yesno }}")
        rendered = template.render(Context({"var": 1}))
        self.assertEqual(rendered, "yes") 