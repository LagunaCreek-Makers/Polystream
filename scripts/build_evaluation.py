#!/usr/bin/env python3
"""Build offline text or recorded-audio cases from the bundled parallel passage set."""
import argparse
import hashlib
import json
from pathlib import Path

PAIRS = [('en', 'es'), ('en', 'de'), ('es', 'de'), ('de', 'es')]


def build_cases(corpus, recordings=None):
    cases, ids = [], set()
    for passage in corpus['passages']:
        ident = passage['id']
        if not ident or len(ident) > 50 or not all(c.isascii() and (c.isalnum() or c in '-_') for c in ident):
            raise ValueError('Unsafe passage identifier')
        if ident.lower() in ids: raise ValueError('Duplicate passage identifier')
        ids.add(ident.lower())
        for source, target in PAIRS:
            original, reference = passage['languages'][source], passage['languages'][target]
            for entry in (original, reference):
                if not entry['text'].strip() or len(entry['text']) > 2000: raise ValueError('Invalid passage length')
                if entry.get('sha256') and hashlib.sha256(entry['text'].encode()).hexdigest() != entry['sha256']:
                    raise ValueError('Passage checksum mismatch')
            case = dict(id=f'{ident}-{source}-{target}', source=source, target=target,
                        reference_source=original['text'], reference_translation=reference['text'],
                        reference=passage.get('reference'), category=passage['kind'],
                        source_url=original.get('url'), reference_url=reference.get('url'),
                        review_focus=passage.get('review_focus', ['Meaning', 'Negation', 'Scripture wording', 'Names']))
            if recordings is None: case['text'] = original['text']
            else:
                audio = recordings.resolve() / f'{ident}-{source}.wav'
                if not audio.is_file(): raise ValueError(f'Missing human recording: {audio}')
                case['audio'] = str(audio)
            cases.append(case)
    if not 1 <= len(cases) <= 200: raise ValueError('Use 1–200 generated cases')
    return cases


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--parallel', type=Path, default=Path(__file__).resolve().parents[1] / 'evaluation/religious-parallel.json')
    parser.add_argument('--output', type=Path, required=True, help='New case file; existing files are never replaced')
    parser.add_argument('--recordings', type=Path, help='Human PCM16/16kHz/mono WAVs named PASSAGE_ID-en|es|de.wav')
    args = parser.parse_args()
    try:
        cases = build_cases(json.loads(args.parallel.read_text(encoding='utf-8')), args.recordings)
        with args.output.open('x', encoding='utf-8') as stream:
            json.dump(cases, stream, ensure_ascii=False, indent=2)
            stream.write('\n')
    except (OSError, ValueError, KeyError, TypeError) as error: parser.error(str(error))
    print(f'Created {len(cases)} cases. No network access or inference performed.')


if __name__ == '__main__': main()
