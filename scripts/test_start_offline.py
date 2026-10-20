"""Offline policy tests; no models, GPU, audio hardware or network required."""
import subprocess
import unittest
from pathlib import Path
from unittest.mock import patch
import start_offline as launcher


class HardwarePolicyTests(unittest.TestCase):
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
