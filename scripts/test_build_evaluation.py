import copy
import json
from pathlib import Path
import tempfile
import unittest
from build_evaluation import build_cases


class ReligiousCorpusTests(unittest.TestCase):
    def setUp(self):
        self.corpus = json.loads((Path(__file__).resolve().parents[1] / 'evaluation/religious-parallel.json').read_text(encoding='utf-8'))

    def test_alignment_and_scope(self):
        cases = build_cases(self.corpus)
        self.assertEqual(len(cases), 52)
        self.assertEqual(len({c['id'] for c in cases}), 52)
        self.assertEqual({(c['source'], c['target']) for c in cases}, {('en','es'),('en','de'),('es','de'),('de','es')})
        self.assertEqual(sum(c['category'] == 'official_scripture' for c in cases), 20)
        for case in cases:
            self.assertTrue(case['text'])
            self.assertTrue(case['reference_translation'])

    def test_integrity_is_checked(self):
        self.corpus['passages'][0]['languages']['en']['text'] += ' altered'
        with self.assertRaisesRegex(ValueError, 'checksum'): build_cases(self.corpus)

    def test_duplicate_and_path_ids_rejected(self):
        self.corpus['passages'].append(copy.deepcopy(self.corpus['passages'][0]))
        with self.assertRaisesRegex(ValueError, 'Duplicate'): build_cases(self.corpus)
        self.corpus['passages'].pop()
        self.corpus['passages'][0]['id'] = '../escape'
        with self.assertRaisesRegex(ValueError, 'Unsafe'): build_cases(self.corpus)

    def test_recordings_are_required_and_reused_across_targets(self):
        with tempfile.TemporaryDirectory() as temp:
            recordings = Path(temp)
            with self.assertRaisesRegex(ValueError, 'Missing human recording'): build_cases(self.corpus, recordings)
            for passage in self.corpus['passages']:
                for lang in ('en', 'es', 'de'):
                    (recordings / f'{passage["id"]}-{lang}.wav').write_bytes(b'placeholder')
            cases = build_cases(self.corpus, recordings)
            self.assertTrue(all('audio' in case and 'text' not in case for case in cases))
            self.assertEqual(cases[0]['audio'], cases[1]['audio'])
