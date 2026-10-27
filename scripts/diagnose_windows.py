"""Run actual app adapters against local engines; never records the microphone."""
import argparse
import json
import re
from pathlib import Path
import subprocess
import sys
import time
import urllib.request


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument('--config', type=Path)
    parser.add_argument('--device', choices=['cpu', 'cuda'], default='cpu')
    parser.add_argument('--audio', type=Path, help='Optional human recording: PCM16 mono 16 kHz WAV')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--repeat', type=int, default=1, help='Repeat uncached cases for throughput checks (1..20)')
    args = parser.parse_args()
    if not 1 <= args.repeat <= 20:
        parser.error('--repeat must be 1..20')
    root = args.root.resolve()
    sys.path.insert(0, str(root / 'scripts'))
    from start_offline import engine_commands
    from check_offline import check_configuration, local_path
    config_path = (args.config or root / 'offline.json').resolve()
    config = json.loads(config_path.read_text(encoding='utf-8'))
    errors = check_configuration(config, config_path.parent)
    if errors:
        raise SystemExit('\n'.join(errors))
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    report = {'device': args.device, 'microphone_tested': False,
              'audio_source': 'human recording' if args.audio else 'Windows synthetic English voice',
              'listening_reviewed': False, 'status': 'failed'}
    processes, logs = [], []
    http = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    try:
        # Refuse to reuse an unknown local server.
        import socket
        for port in (config['asr_port'], config['translation_port']):
            with socket.socket() as probe:
                probe.bind(('127.0.0.1', port))
        commands = engine_commands(config, lambda p, executable=False: local_path(config_path.parent, p, executable), args.device, config['threads'])
        if args.device == 'cuda':
            commands[1].extend(['-lv', '4'])
        for name, command in zip(('whisper', 'llama'), commands):
            log = (out / (name + '.log')).open('w', encoding='utf-8')
            logs.append(log)
            processes.append(subprocess.Popen(command, cwd=Path(command[0]).parent, stdout=log, stderr=subprocess.STDOUT))
        for process, port in zip(processes, (config['asr_port'], config['translation_port'])):
            deadline = time.monotonic() + 240
            while True:
                if process.poll() is not None:
                    raise RuntimeError(f'Engine exited: {process.returncode}; see logs')
                try:
                    with http.open(f'http://127.0.0.1:{port}/health', timeout=2) as response:
                        if response.status == 200:
                            break
                except OSError:
                    pass
                if time.monotonic() > deadline:
                    raise TimeoutError(f'Engine on {port} not ready; see logs')
                time.sleep(0.25)
        if args.device == 'cuda':
            from warmup_offline import warmup
            print('Warming up CUDA inference before timed cases...', flush=True)
            report['warmup_seconds'] = warmup(config)
        audio = args.audio.resolve() if args.audio else out / 'english-input.wav'
        if not args.audio:
            fixture = out / 'make-fixture.ps1'
            fixture.write_text("""Add-Type -AssemblyName System.Speech
$voice = New-Object System.Speech.Synthesis.SpeechSynthesizer
$voice.SelectVoiceByHints([System.Speech.Synthesis.VoiceGender]::NotSet, [System.Speech.Synthesis.VoiceAge]::NotSet, 0, [Globalization.CultureInfo]::GetCultureInfo('en-US'))
$format = New-Object System.Speech.AudioFormat.SpeechAudioFormatInfo(16000, [System.Speech.AudioFormat.AudioBitsPerSample]::Sixteen, [System.Speech.AudioFormat.AudioChannel]::Mono)
$voice.SetOutputToWaveFile((Join-Path $PSScriptRoot 'english-input.wav'), $format)
$voice.Speak('Good morning. The meeting starts at nine.')
$voice.Dispose()
""", encoding='utf-8')
            subprocess.run(['powershell.exe', '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', str(fixture)], check=True, timeout=45)
        cases = []
        for target in ('es', 'de'):
            cases.extend([
                {'id': 'typed-' + target, 'source': 'en', 'target': target, 'text': 'Good morning. The meeting starts at nine.'},
                {'id': 'audio-' + target, 'source': 'en', 'target': target, 'audio': str(audio)},
            ])
        cases = [dict(case, id=f"{case['id']}-{iteration}") for iteration in range(args.repeat) for case in cases]
        (out / 'cases.json').write_text(json.dumps(cases), encoding='utf-8')
        voice_exe = root / 'target/debug/examples/local_voice_check.exe'
        bench_exe = root / 'target/debug/examples/offline_benchmark.exe'
        for language in ('es', 'de'):
            subprocess.run([str(voice_exe), str(config_path), language, str(out / ('voice-' + language + '.wav'))], check=True, timeout=60)
        result = subprocess.run([str(bench_exe), str(config_path), str(out / 'cases.json'), str(out / 'pipeline'), '--no-translation-cache'], timeout=300 * args.repeat)
        rows = [json.loads(line) for line in (out / 'pipeline/results.jsonl').read_text(encoding='utf-8').splitlines()]
        report['cases'] = rows
        report['status'] = 'passed' if result.returncode == 0 and len(rows) == 4 * args.repeat and all(row['status'] == 'ok' for row in rows) else 'failed'
        if args.device == 'cuda':
            for log in logs:
                log.flush()
            # These are evidence for manual review, not a claim based only on driver detection.
            report['gpu_log_evidence'] = {name: [line for line in (out / (name + '.log')).read_text(encoding='utf-8', errors='replace').splitlines() if any(word in line.lower() for word in ('cuda', 'offload', 'gpu'))] for name in ('whisper', 'llama')}
            llama_log = '\n'.join(report['gpu_log_evidence']['llama'])
            whisper_log = '\n'.join(report['gpu_log_evidence']['whisper'])
            offload = re.search(r'offloaded (\d+)/(\d+) layers to GPU', llama_log)
            report['gpu_verified'] = bool(offload and int(offload[1]) > 0 and re.search(r'using CUDA\d+ backend', whisper_log))
            if not report['gpu_verified']:
                report['status'] = 'failed'
                report['error'] = 'Inference completed but CUDA offload evidence is missing; inspect engine logs.'
    except Exception as error:
        report['error'] = str(error)
    finally:
        for process in reversed(processes):
            if process.poll() is None:
                process.terminate()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
        for log in logs:
            log.close()
        (out / 'diagnostic.json').write_text(json.dumps(report, indent=2, ensure_ascii=False), encoding='utf-8')
    print(json.dumps(report, indent=2, ensure_ascii=False))
    return 0 if report['status'] == 'passed' else 1


if __name__ == '__main__':
    raise SystemExit(main())
