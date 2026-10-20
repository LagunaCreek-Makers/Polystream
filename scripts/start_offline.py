#!/usr/bin/env python3
"""Start only local model engines. Does not download or contact external hosts."""
import argparse
import csv
import io
import sys
import json
import os
from pathlib import Path
import socket
import subprocess
import time
from check_offline import check_configuration


def detect_gpus():
    """Query the local driver only. Missing/broken drivers result in CPU selection."""
    try:
        result = subprocess.run(
            ['nvidia-smi', '--query-gpu=uuid,memory.free', '--format=csv,noheader,nounits'],
            capture_output=True, text=True, timeout=3, check=True)
        gpus = []
        for row in csv.reader(io.StringIO(result.stdout)):
            if len(row) != 2: continue
            uuid, free = (field.strip() for field in row)
            if uuid.startswith('GPU-') and int(free) >= 0:
                gpus.append((uuid, int(free)))
        return gpus
    except (OSError, subprocess.SubprocessError, ValueError):
        return []


def select_device(requested, minimum_mib, gpus, visible_devices=None):
    if requested in ('cpu', 'cuda'): return requested, None
    # Respect administrator/user GPU restrictions; do not reinterpret numeric masks.
    if visible_devices is not None:
        return 'cpu', None
    candidates = [(uuid, free) for uuid, free in gpus if free >= minimum_mib]
    if not candidates: return 'cpu', None
    uuid, _ = max(candidates, key=lambda gpu: gpu[1])
    return 'cuda', uuid


def engine_commands(config, local_path, device, threads):
    layers = int(config['gpu_layers']) if device == 'cuda' else 0
    if device == 'cuda' and layers <= 0:
        raise ValueError('CUDA mode requires gpu_layers > 0 in offline.json')
    commands = [
        [str(local_path(config['whisper_server'], True)), '-m', str(local_path(config['whisper_model'])), '--host', '127.0.0.1', '--port', str(config['asr_port']), '-t', str(threads)],
        [str(local_path(config['llama_server'], True)), '-m', str(local_path(config['translation_model'])), '--host', '127.0.0.1', '--port', str(config['translation_port']), '-t', str(threads), '-ngl', str(layers), '-c', '4096', '--parallel', '1', '--alias', 'local-translation', '--jinja'],
    ]
    if device == 'cpu': commands[0].append('--no-gpu')
    return commands


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config', type=Path, default=Path(__file__).resolve().parents[1] / 'offline.json')
    parser.add_argument('--app', type=Path, help='Optional compiled Rust application to launch after engines are listening')
    parser.add_argument('--print-commands', action='store_true', help='Print argv arrays without starting any process')
    parser.add_argument('--device', choices=('auto', 'cuda', 'cpu'), default='auto',
                        help='Auto selects a GPU with sufficient free VRAM, otherwise CPU')
    parser.add_argument('--min-free-vram-mib', type=int, default=6144,
                        help='Auto-selection threshold for the supplied model pack; not a memory guarantee')
    parser.add_argument('--threads', type=int, help='CPU threads per inference engine (1..128)')
    args = parser.parse_args()
    if args.min_free_vram_mib < 1: parser.error('VRAM threshold must be positive')
    config_file = args.config.resolve()
    config = json.loads(config_file.read_text(encoding='utf-8'))
    root = config_file.parent
    def local_path(value, executable=False):
        path = Path(value)
        if not path.is_absolute(): path = root / path
        if executable and os.name == 'nt' and not path.exists() and not path.suffix:
            path = path.with_suffix('.exe')
        return path.resolve()
    ports = [int(config['asr_port']), int(config['translation_port'])]
    if not all(1 <= p <= 65535 for p in ports) or ports[0] == ports[1]:
        parser.error('Use distinct nonzero local ports')
    threads = args.threads if args.threads is not None else int(config['threads'])
    if not 1 <= threads <= 128: parser.error('Threads must be in 1..128')
    gpus = detect_gpus() if args.device == 'auto' else []
    device, uuid = select_device(args.device, args.min_free_vram_mib, gpus,
                                 os.environ.get('CUDA_VISIBLE_DEVICES'))
    print(f'Inference mode: {device}; {threads} CPU threads per engine', file=sys.stderr)
    if args.device == 'auto' and device == 'cpu':
        print('No unrestricted GPU with sufficient reported free VRAM; using CPU. '
              'Use --device cuda for a configured CUDA environment.', file=sys.stderr)
    environment = os.environ.copy()
    if uuid: environment['CUDA_VISIBLE_DEVICES'] = uuid
    if uuid: print(f'CUDA_VISIBLE_DEVICES={uuid}', file=sys.stderr)
    try:
        commands = engine_commands(config, local_path, device, threads)
    except ValueError as error:
        parser.error(str(error))
    if args.print_commands:
        for command in commands: print(json.dumps(command))
        return
    try:
        errors = check_configuration(config, root)
    except (OSError, ValueError, TypeError) as error:
        parser.error(f'Installation check failed: {error}')
    if errors:
        parser.error('Installation is incomplete:\n' + '\n'.join(errors))
    # Never reuse an unknown process already bound to a configured inference port.
    for port in ports:
        probe = socket.socket()
        try:
            probe.bind(('127.0.0.1', port))
        except OSError:
            parser.error(f'Local port {port} is occupied; stop the old engine or change offline.json')
        finally:
            probe.close()
    processes = []
    try:
        for command in commands:
            processes.append(subprocess.Popen(command, cwd=Path(command[0]).parent, env=environment))
        deadline = time.monotonic() + 180
        for process, port in zip(processes, ports):
            while True:
                if process.poll() is not None:
                    raise SystemExit(f'Local engine exited with code {process.returncode}; inspect its console output. For CUDA/VRAM failures, restart with --device cpu')
                try:
                    with socket.create_connection(('127.0.0.1', port), timeout=0.25): break
                except OSError:
                    if time.monotonic() > deadline: raise SystemExit('Timed out starting local models')
                    time.sleep(0.1)
        print('Local engines are listening. No internet connection or service account is used.')
        if args.app:
            app = args.app.resolve()
            if not app.is_file(): raise SystemExit(f'Application not found: {app}')
            subprocess.run([str(app)], cwd=root, check=True)
        else:
            print('Launch the Rust app in another terminal. Press Ctrl+C here to stop these engines.')
            while all(p.poll() is None for p in processes): time.sleep(0.25)
    except KeyboardInterrupt:
        pass
    finally:
        for process in reversed(processes):
            if process.poll() is None: process.terminate()
        for process in processes:
            try: process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill(); process.wait()


if __name__ == '__main__': main()
