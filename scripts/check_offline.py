#!/usr/bin/env python3
"""Check local configuration and installation without starting inference or networking."""
import argparse
import json
import os
from pathlib import Path


def local_path(root, value, executable=False):
    path = Path(value)
    if not path.is_absolute(): path = root / path
    if executable and os.name == 'nt' and not path.exists() and not path.suffix:
        path = path.with_suffix('.exe')
    return path.resolve()


def check_configuration(config, root):
    errors = []
    required = {'asr_port', 'translation_port', 'piper_binary', 'voices', 'whisper_server',
                'whisper_model', 'llama_server', 'translation_model', 'threads', 'gpu_layers'}
    optional = {'translation_domain', 'piper_python_voices', 'piper_worker', 'asr_custom_adapter', 'translation_custom_adapter', 'asr_launch_args', 'translation_launch_args'}
    if not required.issubset(config) or set(config) - required - optional:
        errors.append(f'Configuration fields differ: missing={sorted(required - set(config))}, unknown={sorted(set(config) - required - optional)}')
        return errors
    for key in ('asr_custom_adapter', 'translation_custom_adapter'):
        if type(config.get(key, False)) is not bool: errors.append(f'{key} must be boolean')
    for key in ('asr_launch_args', 'translation_launch_args'):
        value=config.get(key, [])
        if not isinstance(value, list) or any(not isinstance(v,str) or '\x00' in v for v in value): errors.append(f'{key} must be a list of argument strings')
    if config.get('translation_domain','general') not in ('general','latter_day_saint'):
        errors.append('translation_domain must be general or latter_day_saint')
    for key, low, high in [('asr_port', 1, 65535), ('translation_port', 1, 65535),
                           ('threads', 1, 128), ('gpu_layers', 0, 2**32 - 1)]:
        if type(config[key]) is not int or not low <= config[key] <= high:
            errors.append(f'{key} must be an integer in {low}..{high}')
    if config['asr_port'] == config['translation_port']: errors.append('Engine ports must differ')
    def file_check(value, label, executable=False, magic=None):
        if not isinstance(value, str) or not value.strip():
            errors.append(f'{label}: expected a nonempty local path'); return None
        path = local_path(root, value, executable)
        if not path.is_file(): errors.append(f'{label}: missing file {path}'); return None
        if executable and os.name != 'nt' and not os.access(path, os.X_OK):
            errors.append(f'{label}: file is not executable: {path}')
        if path.stat().st_size == 0: errors.append(f'{label}: empty file {path}')
        if magic:
            with path.open('rb') as stream:
                if stream.read(len(magic)) != magic: errors.append(f'{label}: unexpected model format {path}')
        return path
    runtimes = config.get('piper_python_voices', {})
    if not isinstance(runtimes, dict): errors.append('piper_python_voices must be a mapping'); return errors
    if runtimes: file_check(config.get('piper_worker','scripts/piper_worker.py'), 'Piper worker script')
    for code, runtime in runtimes.items():
        if code not in config.get('voices', {}): errors.append(f'{code}: runtime has no configured voice')
        file_check(runtime, f'{code} Piper Python runtime', executable=True)
    for key in ('whisper_server', 'llama_server', 'piper_binary'):
        file_check(config[key], key, executable=True)
    # Legacy Whisper GGML has little-endian GGML magic; llama consumes GGUF.
    file_check(config['whisper_model'], 'whisper_model', magic=None if config.get('asr_custom_adapter',False) else b'lmgg')
    file_check(config['translation_model'], 'translation_model', magic=None if config.get('translation_custom_adapter',False) else b'GGUF')
    voices = config['voices']
    if not isinstance(voices, dict): errors.append('voices must be a mapping'); return errors
    for language in ('es', 'de'):
        voice = file_check(voices.get(language), f'{language} voice')
        if voice:
            metadata = file_check(str(voice) + '.json', f'{language} voice configuration')
            if metadata:
                try:
                    rate = json.loads(metadata.read_text(encoding='utf-8'))['audio']['sample_rate']
                    if type(rate) is not int or not 8000 <= rate <= 96000: raise ValueError('unsupported rate')
                except (ValueError, KeyError, TypeError) as error:
                    errors.append(f'{language} voice configuration: invalid audio.sample_rate ({error})')
    return errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config', type=Path, default=Path(__file__).resolve().parents[1] / 'offline.json')
    args = parser.parse_args()
    try:
        config_file = args.config.resolve()
        config = json.loads(config_file.read_text(encoding='utf-8'))
        if not isinstance(config, dict): raise ValueError('Configuration must be a JSON object')
        errors = check_configuration(config, config_file.parent)
    except (OSError, ValueError) as error:
        errors = [str(error)]
    print(json.dumps({'ready_for_startup': not errors, 'errors': errors,
                      'inference_tested': False, 'model_hashes_verified': False}, indent=2))
    raise SystemExit(1 if errors else 0)


if __name__ == '__main__': main()
