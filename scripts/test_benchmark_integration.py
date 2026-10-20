"""Black-box Rust runner test with fake local engines, not model accuracy evidence."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import threading
import unittest
import wave
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


@unittest.skipUnless(os.environ.get('LIVE_TRANSLATE_BENCHMARK') and os.name != 'nt',
                     'Set LIVE_TRANSLATE_BENCHMARK to built example on Linux')
class BenchmarkIntegrationTests(unittest.TestCase):
    def test_success_failure_audio_and_no_overwrite(self):
        class Engine(BaseHTTPRequestHandler):
            def do_POST(self):
                body = self.rfile.read(int(self.headers['Content-Length']))
                self.server.paths.append(self.path)
                if self.path == '/inference':
                    assert b'RIFF' in body
                    response = {'text': 'Faith brings hope.'}
                else:
                    request = json.loads(body)
                    assert request['chat_template_kwargs']['enable_thinking'] is False
                    incomplete = request['messages'][-1]['content'] == 'TRUNCATE'
                    response = {'choices': [{'finish_reason': 'length' if incomplete else 'stop',
                                             'message': {'content': 'La fe trae esperanza.'}}]}
                raw = json.dumps(response).encode()
                self.send_response(200)
                self.send_header('Content-Length', str(len(raw)))
                self.end_headers()
                self.wfile.write(raw)
            def log_message(self, *args): pass
        servers = []
        for _ in range(2):
            server = ThreadingHTTPServer(('127.0.0.1', 0), Engine)
            server.paths = []
            threading.Thread(target=server.serve_forever, daemon=True).start()
            servers.append(server)
            self.addCleanup(server.server_close)
            self.addCleanup(server.shutdown)
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            piper = root / 'piper'
            piper.write_text('#!/usr/bin/env python3\nimport sys\nsys.stdin.read()\nsys.stdout.buffer.write(b"\\x00\\x10" * 800)\n')
            piper.chmod(0o755)
            (root / 'voice.onnx').write_bytes(b'fake model')
            (root / 'voice.onnx.json').write_text('{"audio":{"sample_rate":16000}}')
            cfg = dict(asr_port=servers[0].server_port, translation_port=servers[1].server_port,
                       piper_binary=str(piper), voices={'es':'voice.onnx','de':'voice.onnx'}, whisper_server='unused',
                       whisper_model='unused', llama_server='unused', translation_model='unused',threads=4,gpu_layers=0)
            (root / 'config.json').write_text(json.dumps(cfg))
            with wave.open(str(root / 'input.wav'), 'wb') as wav:
                wav.setparams((1, 2, 16000, 0, 'NONE', 'not compressed'))
                wav.writeframes(b'\x00\x10' * 1600)
            cases = [dict(id='text',source='en',target='es',text='Faith brings hope.'),
                     dict(id='audio',source='en',target='es',audio='input.wav'),
                     dict(id='bad',source='en',target='es',text='TRUNCATE')]
            (root / 'cases.json').write_text(json.dumps(cases))
            command = [str(Path(os.environ['LIVE_TRANSLATE_BENCHMARK']).resolve()), str(root/'config.json'), str(root/'cases.json'), str(root/'results')]
            result = subprocess.run(command, capture_output=True, text=True, timeout=20)
            self.assertNotEqual(result.returncode, 0, result.stdout)
            rows = [json.loads(line) for line in (root/'results/results.jsonl').read_text().splitlines()]
            self.assertEqual([row['status'] for row in rows], ['ok','ok','error'])
            self.assertIn('incomplete', rows[2]['error'])
            self.assertGreater(rows[0]['first_pcm_ms'], 0)
            self.assertIn('recognition_ms', rows[1])
            self.assertGreater(rows[1]['processing_to_input_ratio'], 0)
            self.assertEqual(servers[0].paths, ['/inference'])
            self.assertEqual(len(servers[1].paths), 2)
            self.assertTrue(rows[1]['translation_cache_hit'])
            summary = json.loads((root/'results/summary.json').read_text())
            self.assertEqual(summary['failed'], 1)
            self.assertFalse(summary['acoustic_latency_measured'])
            self.assertFalse((root/'results/case-bad.wav').exists())
            with wave.open(str(root/'results/case-audio.wav'), 'rb') as wav:
                self.assertEqual(wav.getnframes(), 800)
            before = (root/'results/results.jsonl').read_bytes()
            repeat = subprocess.run(command, capture_output=True, text=True, timeout=5)
            self.assertNotEqual(repeat.returncode, 0)
            self.assertEqual((root/'results/results.jsonl').read_bytes(), before)

            # Exercise every bundled domain case through the Rust manifest/parser path.
            # Fake translations prove plumbing only, never linguistic correctness.
            from build_evaluation import build_cases
            corpus = json.loads((Path(__file__).resolve().parents[1] / 'evaluation/religious-parallel.json').read_text(encoding='utf-8'))
            (root / 'religious.json').write_text(json.dumps(build_cases(corpus)))
            domain_command = command[:2] + [str(root/'religious.json'), str(root/'religious-results')]
            domain = subprocess.run(domain_command, capture_output=True, text=True, timeout=30)
            self.assertEqual(domain.returncode, 0, domain.stderr)
            domain_rows = [json.loads(line) for line in (root/'religious-results/results.jsonl').read_text().splitlines()]
            self.assertEqual(len(domain_rows), 52)
            self.assertTrue(all(row['status'] == 'ok' for row in domain_rows))
            self.assertEqual(sum(row['category'] == 'official_scripture' for row in domain_rows), 20)
            self.assertTrue(all(row['human_review'] is None for row in domain_rows))

            # Translation-only mode works without Piper or installed voices.
            cfg['piper_binary'] = 'missing-piper'
            cfg['voices'] = {}
            (root / 'translation-config.json').write_text(json.dumps(cfg))
            repeat_cases=[dict(id='repeat-a',source='en',target='es',text='Faith brings hope.'),
                          dict(id='repeat-b',source='en',target='es',text='Faith brings hope.')]
            (root / 'repeats.json').write_text(json.dumps(repeat_cases))
            translation_command=[command[0],str(root/'translation-config.json'),str(root/'repeats.json'),str(root/'translation-only'),'--translation-only']
            translated=subprocess.run(translation_command,capture_output=True,text=True,timeout=10)
            self.assertEqual(translated.returncode,0,translated.stderr)
            translation_rows=[json.loads(line) for line in (root/'translation-only/results.jsonl').read_text().splitlines()]
            self.assertEqual([row['translation_cache_hit'] for row in translation_rows],[False,True])
            self.assertTrue(all('output_wav' not in row and 'first_pcm_ms' not in row for row in translation_rows))
            self.assertTrue(all('translation_ready_ms' in row for row in translation_rows))
            translation_command[3]=str(root/'translation-uncached')
            translation_command.append('--no-translation-cache')
            uncached=subprocess.run(translation_command,capture_output=True,text=True,timeout=10)
            self.assertEqual(uncached.returncode,0,uncached.stderr)
            uncached_rows=[json.loads(line) for line in (root/'translation-uncached/results.jsonl').read_text().splitlines()]
            self.assertTrue(all(not row['translation_cache_hit'] for row in uncached_rows))
