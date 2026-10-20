#!/usr/bin/env python3
"""Explicit, one-time model setup. Normal app operation never downloads models."""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import urllib.request


def sha256(path):
    digest = hashlib.sha256()
    with path.open('rb') as source:
        while chunk := source.read(4 * 1024 * 1024): digest.update(chunk)
    return digest.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--download', action='store_true', help='Explicitly download ~2.8 GB from the pinned public sources; no account needed')
    parser.add_argument('--verify', action='store_true', help='Verify an installed/copied model pack without any network calls')
    parser.add_argument('--destination', type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    manifest = json.loads((Path(__file__).resolve().parents[1] / 'models-manifest.json').read_text())
    total = sum(item['size'] for item in manifest['files'])
    print(f"Model pack: {len(manifest['files'])} files, {total / 1e9:.2f} GB. Native engine programs are separate.")
    failures = 0
    for item in manifest['files']:
        relative = PurePosixPath(item['path'])
        if relative.is_absolute() or '..' in relative.parts: raise SystemExit('Invalid model destination')
        target = args.destination.resolve().joinpath(*relative.parts)
        if not args.download and not args.verify:
            print(item['path'], item['size'], item['license'])
            continue
        if target.exists():
            if target.is_file() and sha256(target) == item['sha256']:
                print('Verified:', item['path']); continue
            raise SystemExit(f'Existing file has a different checksum; refusing to replace {target}')
        if not args.download:
            print('Missing:', item['path']); failures += 1; continue
        target.parent.mkdir(parents=True, exist_ok=True)
        temporary = target.with_name(target.name + '.download')
        digest = hashlib.sha256(); size = 0
        try:
            with urllib.request.urlopen(item['url'], timeout=90) as response, temporary.open('wb') as output:
                while chunk := response.read(4 * 1024 * 1024):
                    output.write(chunk); digest.update(chunk); size += len(chunk)
            if digest.hexdigest() != item['sha256'] or size != item['size']:
                raise SystemExit(f'Model verification failed: {item["path"]}')
            os.replace(temporary, target)
            print('Installed:', item['path'])
        finally:
            temporary.unlink(missing_ok=True)
    if not args.download and not args.verify:
        print('No network activity. Use --download for online setup, or copy a prepared pack and use --verify.')
    if failures: raise SystemExit(1)


if __name__ == '__main__': main()
