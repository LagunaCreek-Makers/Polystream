import json
import os
from pathlib import Path
import tempfile
import unittest
from check_offline import check_configuration


class PreflightTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        for name, content in [('whisper', b'engine'), ('llama', b'engine'), ('piper', b'engine'),
                              ('base.bin', b'lmgg123'), ('qwen.gguf', b'GGUF123'),
                              ('voice.onnx', b'not-a-real-model')]:
            path = self.root / name
            path.write_bytes(content)
            path.chmod(0o755)
        (self.root / 'voice.onnx.json').write_text(json.dumps({'audio': {'sample_rate': 22050}}))
        self.config = dict(asr_port=8178, translation_port=8179, threads=4, gpu_layers=99,
                           whisper_server='whisper', llama_server='llama', piper_binary='piper',
                           whisper_model='base.bin', translation_model='qwen.gguf',
                           voices={'es': 'voice.onnx', 'de': 'voice.onnx'})

    def test_optional_religious_profile(self):
        self.config['translation_domain']='latter_day_saint'
        self.assertEqual(check_configuration(self.config,self.root),[])
        self.config['translation_domain']='unsupported'
        self.assertTrue(check_configuration(self.config,self.root))

    def test_complete_layout(self):
        # A structural pass must never claim that fake files support inference.
        self.assertEqual(check_configuration(self.config, self.root), [])

    def test_missing_components_are_reported_together(self):
        (self.root / 'piper').unlink()
        (self.root / 'qwen.gguf').unlink()
        errors = check_configuration(self.config, self.root)
        self.assertEqual(len(errors), 2)

    def test_bad_voice_rate_and_model_format(self):
        (self.root / 'qwen.gguf').write_bytes(b'HTML error page')
        (self.root / 'voice.onnx.json').write_text('{"audio":{"sample_rate":true}}')
        self.assertEqual(len(check_configuration(self.config, self.root)), 3)

    def test_unknown_fields_and_ports(self):
        self.config['azure_key'] = 'unused'
        self.assertTrue(check_configuration(self.config, self.root))
        del self.config['azure_key']
        self.config['asr_port'] = self.config['translation_port']
        self.assertTrue(check_configuration(self.config, self.root))
        self.config['threads'] = True
        self.assertTrue(check_configuration(self.config, self.root))

    def test_missing_voice_mapping(self):
        self.config['voices'] = {'es': 'voice.onnx'}
        self.assertEqual(len(check_configuration(self.config, self.root)), 1)

    @unittest.skipIf(os.name == 'nt', 'Windows has no POSIX executable bit')
    def test_executable_permissions(self):
        (self.root / 'piper').chmod(0o644)
        self.assertIn('not executable', str(check_configuration(self.config, self.root)))


if __name__ == '__main__': unittest.main()
