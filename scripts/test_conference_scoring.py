import unittest
from score_conference import review_flags


class ConferenceScoringTests(unittest.TestCase):
    def test_terms_negation_and_numbers_are_review_flags(self):
        row=dict(target='de',reference_translation='Der Bischof kommt nicht um 10 Uhr.',translation='Er kommt um 11 Uhr.')
        self.assertEqual(set(review_flags(row)),{'terminology:bishop','digit_difference','possible_missing_negation'})

    def test_accepted_inflections_and_rephrasing(self):
        row=dict(target='de',reference_translation='Wir schließen Bündnisse mit Gott.',translation='Mit Gott gehen wir Bündnisse ein.')
        self.assertEqual(review_flags(row),[])
        row=dict(target='es',reference_translation='No comienza a las 10.',translation='Nunca comienza a las 10.')
        self.assertEqual(review_flags(row),[])

    def test_flags_cannot_certify_accuracy(self):
        # Opposite meanings can have the same words/digits and raise no lexical flags.
        row=dict(target='es',reference_translation='Juan ayuda a Pedro.',translation='Pedro ayuda a Juan.')
        self.assertEqual(review_flags(row),[])

    def test_unrelated_word_substrings_do_not_become_church_terms(self):
        self.assertEqual(review_flags(dict(target='de',reference_translation='Er empfahl uns zu beten.',translation='Wir sollten beten.')),[])
        self.assertEqual(review_flags(dict(target='es',reference_translation='Es un ejemplo destacado.',translation='Es un buen ejemplo.')),[])
