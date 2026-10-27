"""Offline policy tests; no models, GPU, audio hardware or network required."""
import subprocess
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import start_offline as launcher


class HardwarePolicyTests(unittest.TestCase):
    def test_two_slots_preserve_context_per_request(self):
        cfg = dict(whisper_server='whisper', whisper_model='base', llama_server='llama',
                   translation_model='qwen', asr_port=8178, translation_port=8179, gpu_layers=99)
        for slots in (1, 2):
            commands=launcher.engine_commands(cfg, lambda path, *args: Path(path), 'cuda', 8, slots)
            self.assertEqual(commands[1][commands[1].index('-c')+1], str(4096*slots))
            self.assertEqual(commands[1][commands[1].index('--parallel')+1], str(slots))
            self.assertEqual(commands[0][commands[0].index('-t')+1], '8')
        for invalid in (0,3,True,'2'):
            with self.assertRaises(ValueError):
                launcher.engine_commands(cfg, lambda path, *args: Path(path), 'cuda', 8, invalid)

    def test_context_limits_and_configured_context(self):
        cfg=dict(whisper_server='whisper', whisper_model='base', llama_server='llama',translation_model='qwen',asr_port=8178,translation_port=8179,gpu_layers=99)
        command=launcher.engine_commands(cfg,lambda value,*args:Path(value),'cuda',4,2,2048)[1]
        self.assertEqual(command[command.index('-c')+1],'4096')
        for invalid in (0,1023,16385,True,'4096'):
            with self.assertRaises(ValueError):launcher.engine_commands(cfg,lambda value,*args:Path(value),'cpu',4,1,invalid)

    def test_saved_hardware_preferences_drive_real_argv(self):
        cfg=dict(whisper_server='gpu-whisper',whisper_model='base.bin',llama_server='gpu-llama',translation_model='qwen.gguf',asr_port=8178,translation_port=8179,gpu_layers=99,threads=4)
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder)
            (root/'custom.json').write_text(json.dumps(cfg))
            (root/'offline.json').write_text(json.dumps(dict(whisper_server='cpu-whisper',llama_server='cpu-llama')))
            prefs=dict(device='cpu',threads=8,translation_slots=2,gpu_layers=10,context_tokens=2048)
            (root/'engine-settings.json').write_text(json.dumps(prefs))
            args=[sys.executable,str(Path(launcher.__file__)), '--config',str(root/'custom.json'),'--device','cuda','--print-commands']
            result=subprocess.run(args,capture_output=True,text=True,check=True)
            commands=[json.loads(line) for line in result.stdout.splitlines()]
            self.assertIn('--no-gpu',commands[0])
            self.assertIn('cpu-whisper',commands[0][0])
            self.assertEqual(commands[1][commands[1].index('-ngl')+1],'0')
            self.assertEqual(commands[1][commands[1].index('-t')+1],'8')
            self.assertEqual(commands[1][commands[1].index('-c')+1],'4096')
            prefs['threads']=0;(root/'engine-settings.json').write_text(json.dumps(prefs))
            result=subprocess.run(args,capture_output=True,text=True)
            self.assertNotEqual(result.returncode,0)
            self.assertIn('Invalid threads',result.stderr)

    def test_selected_custom_config_drives_models_ports_and_adapter_arguments(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder)
            base=dict(whisper_server='old-whisper',whisper_model='old-base',llama_server='old-llama',translation_model='old-qwen',asr_port=8178,translation_port=8179,gpu_layers=99,threads=4)
            (root/'offline.json').write_text(json.dumps(base))
            custom=dict(base,whisper_server='python.exe',whisper_model='asr.json',llama_server='translate.exe',translation_model='translation.json',asr_port=9101,translation_port=9102,asr_launch_args=['C:/my adapter.py'])
            (root/'selected.json').write_text(json.dumps(custom))
            prefs=dict(device='cuda',threads=4,translation_slots=1,gpu_layers=99,context_tokens=4096,config=str(root/'selected.json'),stock_engines=False)
            (root/'engine-settings.json').write_text(json.dumps(prefs))
            result=subprocess.run([sys.executable,str(Path(launcher.__file__)),'--config',str(root/'offline.json'),'--print-commands'],capture_output=True,text=True,check=True)
            commands=[json.loads(line) for line in result.stdout.splitlines()]
            self.assertIn('python.exe',commands[0][0]);self.assertEqual(commands[0][1],'C:/my adapter.py')
            self.assertIn('asr.json',commands[0][commands[0].index('-m')+1]);self.assertIn('9101',commands[0])
            self.assertIn('translate.exe',commands[1][0]);self.assertIn('translation.json',commands[1][commands[1].index('-m')+1]);self.assertIn('9102',commands[1])
            prefs['config']='missing.json';(root/'engine-settings.json').write_text(json.dumps(prefs))
            failed=subprocess.run([sys.executable,str(Path(launcher.__file__)),'--config',str(root/'offline.json'),'--print-commands'],capture_output=True,text=True)
            self.assertNotEqual(failed.returncode,0);self.assertIn('Invalid engine-settings.json',failed.stderr)

    def test_selected_stock_preset_uses_new_weights_and_stock_hardware_engines(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder)
            cfg=dict(whisper_server='old-whisper',whisper_model='base.bin',llama_server='old-llama',translation_model='4b.gguf',asr_port=8178,translation_port=8179,gpu_layers=99,threads=4)
            (root/'offline.json').write_text(json.dumps(cfg))
            (root/'offline.cuda.json').write_text(json.dumps(dict(whisper_server='cuda-whisper',llama_server='cuda-llama')))
            (root/'chosen.json').write_text(json.dumps(dict(cfg,translation_model='8b.gguf')))
            (root/'engine-settings.json').write_text(json.dumps(dict(device='cuda',threads=4,translation_slots=1,gpu_layers=99,context_tokens=4096,config=str(root/'chosen.json'),stock_engines=True)))
            result=subprocess.run([sys.executable,str(Path(launcher.__file__)),'--config',str(root/'offline.json'),'--print-commands'],capture_output=True,text=True,check=True)
            commands=[json.loads(line) for line in result.stdout.splitlines()]
            self.assertIn('cuda-whisper',commands[0][0]);self.assertIn('cuda-llama',commands[1][0])
            self.assertIn('8b.gguf',commands[1][commands[1].index('-m')+1]);self.assertNotIn('4b.gguf',commands[1])

    def test_auto_selects_most_free_gpu(self):
        self.assertEqual(launcher.select_device('auto', 6144,
                         [('GPU-a', 6200), ('GPU-b', 9000)]), ('cuda', 'GPU-b'))

    def test_auto_falls_back_when_busy_or_missing(self):
        for gpus in ([], [('GPU-a', 6143)]):
            self.assertEqual(launcher.select_device('auto', 6144, gpus), ('cpu', None))

    def test_auto_respects_visibility_masks(self):
        for mask in ('', '0', '-1', 'GPU-a'):
            self.assertEqual(launcher.select_device('auto', 6144,
                             [('GPU-a', 9000)], mask), ('cpu', None))

    def test_explicit_modes_do_not_depend_on_detection(self):
        for mode in ('cpu', 'cuda'):
            self.assertEqual(launcher.select_device(mode, 6144, []), (mode, None))

    def test_cpu_disables_both_gpu_backends(self):
        cfg = dict(whisper_server='whisper', whisper_model='base', llama_server='llama',
                   translation_model='qwen', asr_port=8178, translation_port=8179, gpu_layers=99)
        cpu = launcher.engine_commands(cfg, lambda path, *args: Path(path), 'cpu', 4)
        cuda = launcher.engine_commands(cfg, lambda path, *args: Path(path), 'cuda', 6)
        self.assertIn('--no-gpu', cpu[0])
        self.assertEqual(cpu[1][cpu[1].index('-ngl') + 1], '0')
        self.assertNotIn('--no-gpu', cuda[0])
        self.assertEqual(cuda[1][cuda[1].index('--parallel')+1],'1')
        self.assertEqual(cuda[1][cuda[1].index('-ngl') + 1], '99')
        for command in cpu + cuda:
            self.assertEqual(command[command.index('--host') + 1], '127.0.0.1')
        cfg['gpu_layers'] = 0
        with self.assertRaises(ValueError):
            launcher.engine_commands(cfg, lambda path, *args: Path(path), 'cuda', 4)

    @patch('start_offline.subprocess.run')
    def test_local_driver_output(self, run):
        run.return_value.stdout = 'GPU-a, 8192\nGPU-b, 4096\n'
        self.assertEqual(launcher.detect_gpus(), [('GPU-a', 8192), ('GPU-b', 4096)])
        self.assertEqual(run.call_args.kwargs['timeout'], 3)

    @patch('start_offline.subprocess.run')
    def test_driver_failures_fall_back(self, run):
        for error in (FileNotFoundError(), subprocess.TimeoutExpired('nvidia-smi', 3),
                      subprocess.CalledProcessError(1, 'nvidia-smi')):
            run.side_effect = error
            self.assertEqual(launcher.detect_gpus(), [])
        run.side_effect = None
        run.return_value.stdout = 'GPU-a, N/A\n'
        self.assertEqual(launcher.detect_gpus(), [])


if __name__ == '__main__': unittest.main()
