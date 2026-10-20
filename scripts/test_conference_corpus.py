import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from conference_corpus import aligned_records, load_talk, parse_talk, talk_paths, validate_path
from run_conference import cases_from_records, select_records


class ConferenceCorpusTests(unittest.TestCase):
    def test_talk_body_excludes_notes_images_and_headers(self):
        html='''<article data-content-type="general-conference-talk" lang="eng"><header><p id="p1">Speaker</p></header><div class="body-block"><p id="p3">Keep <a>faith<sup>1</sup></a>, and hope.</p><figure><p id="p4">Image caption</p></figure><p id="p5">Serve others.</p></div><aside><p id="p6">Footnote.</p></aside></article>'''
        modern=html.replace('p3','p_Ab12x').replace('p5','p_Z92aa')
        self.assertEqual(list(parse_talk(modern,'eng')),['p_Ab12x','p_Z92aa'])
        self.assertEqual(parse_talk(html,'eng'),{'p3':'Keep faith, and hope.','p5':'Serve others.'})
        with self.assertRaisesRegex(ValueError,'Language mismatch'):parse_talk(html,'deu')
        with self.assertRaisesRegex(ValueError,'Not a talk'):parse_talk(html.replace('general-conference-talk','session'),'eng')

    def test_unsafe_cache_paths_are_rejected(self):
        for path in ('/study/general-conference/2025/04/../../secret','https://evil.test/x','/study/general-conference/2025/04/a?x=1'):
            with self.assertRaises(ValueError):validate_path(path)
        with self.assertRaises(ValueError):validate_path('/study/general-conference/2024/10/example',2025,'04')

    def test_index_limits_scope_and_deduplicates(self):
        html='''<a href="/study/general-conference/2017/04/example?lang=eng">talk</a><a href="/study/general-conference/2017/04/example">same</a><a href="https://evil.test/study/general-conference/2017/04/foreign">bad</a><a href="/study/general-conference/2017/04/sunday-morning-session">session</a><a href="/study/general-conference/2016/04/old">old</a>'''
        self.assertEqual(talk_paths(html,2017,'04'),['/study/general-conference/2017/04/example'])

    def test_alignment_only_emits_shared_ids_and_holds_out_years(self):
        docs={lang:dict(paragraphs={'p3':text,'p4':'too short'},url='https://example.test',sha256='hash') for lang,text in
              [('en','Faith helps us serve.'),('es','La fe nos ayuda a servir.'),('de','Glaube hilft uns beim Dienen.')]}
        rows=list(aligned_records('/study/general-conference/2025/04/example',docs))
        self.assertEqual(len(rows),1)
        self.assertEqual(rows[0]['split'],'test')
        self.assertEqual(rows[0]['alignment'],'shared_publisher_id_unreviewed')
        cases=list(cases_from_records(rows))
        self.assertEqual(len(cases),4)
        self.assertEqual(len({row['id'] for row in cases}),4)
        del docs['de']['paragraphs']['p3']
        self.assertEqual(list(aligned_records('/study/general-conference/2025/04/example',docs)),[])

    def test_offline_cache_detects_modified_text(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);path=root/'talks/2025/04/example-en.json';path.parent.mkdir(parents=True)
            paragraphs={'p3':'Faith helps us serve.'}
            digest=hashlib.sha256(json.dumps(paragraphs,ensure_ascii=False,sort_keys=True).encode()).hexdigest()
            record=dict(path='/study/general-conference/2025/04/example',language='en',paragraphs=paragraphs,sha256=digest)
            path.write_text(json.dumps(record))
            self.assertEqual(load_talk(root,record['path'],'en',False)['paragraphs'],paragraphs)
            record['paragraphs']['p3']='Changed';path.write_text(json.dumps(record))
            with self.assertRaisesRegex(ValueError,'checksum'):load_talk(root,record['path'],'en',False)

    def test_pilot_balances_conferences_and_limits_length(self):
        with tempfile.TemporaryDirectory() as temp:
            path=Path(temp)/'aligned.jsonl'
            rows=[dict(year=year,month=month,split='test',texts={'en':'a'*length})
                  for year,month,length in [(2025,'04',50),(2025,'04',60),(2025,'10',40),(2026,'04',500)]]
            path.write_text(''.join(json.dumps(row)+'\n' for row in rows))
            selected=select_records(path,'test',2,100)
            self.assertEqual([(r['year'],r['month']) for r in selected],[(2025,'04'),(2025,'10')])
