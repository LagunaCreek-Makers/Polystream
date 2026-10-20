import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


@unittest.skipIf(os.name=='nt','Test fixture uses a POSIX executable script')
class ConferenceResumeTests(unittest.TestCase):
    def test_resume_and_identity_guard(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp)
            row=dict(id='gc-2025-04-hash-p3',year=2025,month='04',split='test',paragraph='p3',
                     talk_path='/study/general-conference/2025/04/example',texts={'en':'Faith brings hope.','es':'La fe trae esperanza.','de':'Glaube bringt Hoffnung.'},
                     urls={lang:'https://example.test' for lang in ('en','es','de')})
            (root/'aligned.jsonl').write_text(json.dumps(row)+'\n')
            (root/'config.json').write_text('{"translation_model":"model.gguf"}')
            (root/'model.gguf').write_bytes(b'GGUF-fake-model')
            binary=root/'fake-runner'
            binary.write_text('''#!/usr/bin/env python3
import json,sys
from pathlib import Path
cases=json.loads(Path(sys.argv[2]).read_text())
out=Path(sys.argv[3]);out.mkdir()
(out/'results.jsonl').write_text(''.join(json.dumps({'id':c['id'],'status':'ok'})+'\\n' for c in cases))
''')
            binary.chmod(0o755)
            script=Path(__file__).resolve().parent/'run_conference.py'
            command=[sys.executable,str(script),'--corpus',str(root),'--config',str(root/'config.json'),'--binary',str(binary),'--output',str(root/'results')]
            plan=subprocess.run(command,capture_output=True,text=True,timeout=5)
            self.assertEqual(plan.returncode,0,plan.stderr)
            self.assertFalse((root/'results').exists())
            command.append('--run')
            first=subprocess.run(command,capture_output=True,text=True,timeout=5)
            self.assertEqual(first.returncode,0,first.stderr)
            self.assertEqual(json.loads((root/'results/completion.json').read_text())['successful_unique_cases'],4)
            with (root/'results/attempt-00000/results.jsonl').open('a') as stream:stream.write('{"interrupted":')
            resumed=subprocess.run(command,capture_output=True,text=True,timeout=5)
            self.assertEqual(resumed.returncode,0,resumed.stderr)
            self.assertEqual(len(list((root/'results').glob('attempt-*'))),1)
            (root/'model.gguf').write_bytes(b'GGUF-changed')
            changed=subprocess.run(command,capture_output=True,text=True,timeout=5)
            self.assertNotEqual(changed.returncode,0)
            self.assertIn('Run inputs changed',changed.stderr)
