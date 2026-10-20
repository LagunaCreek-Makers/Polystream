#!/usr/bin/env python3
"""Plan/resume translation-only evaluation against the prepared conference corpus."""
import argparse
from collections import defaultdict, deque
import hashlib
import json
from pathlib import Path
import subprocess
from conference_corpus import atomic_json

PAIRS = [('en','es'),('en','de'),('es','de'),('de','es')]


def file_sha256(path):
    digest=hashlib.sha256()
    with path.open('rb') as stream:
        while chunk:=stream.read(8*1024*1024):digest.update(chunk)
    return digest.hexdigest()


def select_records(path, split='all', limit=None, max_chars=2000, selected_ids=None):
    groups = defaultdict(deque)
    with path.open(encoding='utf-8') as source:
        for line in source:
            row = json.loads(line)
            if selected_ids is not None and row['id'] not in selected_ids:continue
            if split != 'all' and row['split'] != split: continue
            if any(len(text)>max_chars for text in row['texts'].values()):continue
            groups[(row['year'],row['month'])].append(row)
    # Deterministic round-robin coverage of all conferences, even in a small pilot.
    selected = []
    while any(groups.values()) and (limit is None or len(selected) < limit):
        for key in sorted(groups):
            if groups[key]: selected.append(groups[key].popleft())
            if limit is not None and len(selected) >= limit: break
    return selected


def cases_from_records(records):
    for row in records:
        for source,target in PAIRS:
            yield dict(id=f'{row["id"]}-{source}-{target}',source=source,target=target,
                       text=row['texts'][source],reference_source=row['texts'][source],
                       reference_translation=row['texts'][target],reference=row['talk_path']+'#'+row['paragraph'],
                       source_url=row['urls'][source],reference_url=row['urls'][target],
                       category='general_conference_'+row['split'],
                       review_focus=['Alignment is unreviewed','Meaning','Religious terminology','Names/numbers','Negation'])


def result_rows(path):
    if not path.is_file():return
    with path.open(encoding='utf-8') as source:
        for line in source:
            try:row=json.loads(line)
            except ValueError:continue
            if isinstance(row,dict):yield row


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--corpus',type=Path,required=True)
    parser.add_argument('--config',type=Path,required=True)
    parser.add_argument('--binary',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--split',choices=('all','train','dev','test'),default='all')
    parser.add_argument('--limit-paragraphs',type=int,help='Optional pilot limit; omitted means every candidate paragraph')
    parser.add_argument('--max-chars',type=int,default=2000,help='Maximum paragraph length in every language (15..2000)')
    parser.add_argument('--paragraph-ids',type=Path,help='Optional JSON array of paragraph IDs for an exact published selection')
    parser.add_argument('--run',action='store_true',help='Actually invoke the Rust runner; default prints a plan')
    parser.add_argument('--cache',action='store_true',help='Enable exact translation cache; default measures uncached inference')
    args=parser.parse_args()
    if args.limit_paragraphs is not None and args.limit_paragraphs < 1: parser.error('Pilot limit must be positive')
    if not 15 <= args.max_chars <= 2000: parser.error('max-chars must be 15..2000')
    selected_ids=None
    if args.paragraph_ids:
        selection=json.loads(args.paragraph_ids.read_text(encoding='utf-8'))
        if not isinstance(selection,list) or not selection or not all(isinstance(value,str) for value in selection):parser.error('Selection must be a nonempty JSON array of IDs')
        selected_ids=set(selection)
    records=select_records(args.corpus/'aligned.jsonl',args.split,args.limit_paragraphs,args.max_chars,selected_ids)
    if selected_ids is not None and {row['id'] for row in records} != selected_ids:parser.error('Some selected IDs are absent or filtered out; check corpus version, split, length and limit')
    cases=list(cases_from_records(records))
    if not cases: parser.error('No eligible cases')
    expected_ids={case['id'] for case in cases}
    print(json.dumps(dict(paragraphs=len(records),direction_cases=len(cases),batches=(len(cases)+99)//100,
                         mode='translation_only',corpus_scope='selected' if args.paragraph_ids else ('pilot' if args.limit_paragraphs else 'all_candidates'),
                         split=args.split,cache=args.cache,run=args.run),indent=2),flush=True)
    if not args.run:return
    root=args.output.resolve();root.mkdir(parents=True,exist_ok=True)
    config=json.loads(args.config.read_text(encoding='utf-8'))
    model=Path(config['translation_model'])
    if not model.is_absolute():model=args.config.resolve().parent/model
    if not model.is_file():parser.error('Configured translation model must exist to pin evaluation identity')
    identity=dict(config_sha256=file_sha256(args.config),binary_sha256=file_sha256(args.binary),
                  corpus_sha256=file_sha256(args.corpus/'aligned.jsonl'),model_sha256=file_sha256(model),
                  split=args.split,limit=args.limit_paragraphs,cache=args.cache,max_chars=args.max_chars,
                  selection_sha256=file_sha256(args.paragraph_ids) if args.paragraph_ids else None)
    fingerprint=hashlib.sha256(json.dumps(identity,sort_keys=True).encode()).hexdigest()
    state_path=root/'state.json'
    if state_path.exists():
        if json.loads(state_path.read_text())['fingerprint'] != fingerprint: parser.error('Run inputs changed; use a new output directory')
    else:atomic_json(state_path,dict(fingerprint=fingerprint,identity=identity,total_cases=len(cases),mode='translation_only'))
    completed=set();attempt=0
    for results in sorted(root.glob('attempt-*/results.jsonl')):
        for row in result_rows(results):
            if row.get('status')=='ok' and row.get('id') in expected_ids: completed.add(row['id'])
    pending=[case for case in cases if case['id'] not in completed]
    for offset in range(0,len(pending),100):
        while (root/f'attempt-{attempt:05}').exists() or (root/f'cases-{attempt:05}.json').exists():attempt+=1
        batch=root/f'cases-{attempt:05}.json'
        batch.write_text(json.dumps(pending[offset:offset+100],ensure_ascii=False),encoding='utf-8')
        command=[str(args.binary.resolve()),str(args.config.resolve()),str(batch),str(root/f'attempt-{attempt:05}'),'--translation-only']
        if not args.cache:command.append('--no-translation-cache')
        result=subprocess.run(command,check=False)
        if result.returncode:print(f'Batch {attempt} has failures; successful rows are retained for resume',flush=True)
        attempt+=1
        results_file=root/f'attempt-{attempt-1:05}'/'results.jsonl'
        if not results_file.exists() or not any(row.get('status')=='ok' for row in result_rows(results_file)):
            print('Stopping after a batch without successes; fix the engine/configuration and resume.',flush=True)
            break
    # Count successful IDs, not result files: reruns never inflate completion counts.
    successes=set();errors=0
    for results in root.glob('attempt-*/results.jsonl'):
        for row in result_rows(results):
            if row.get('status')=='ok' and row.get('id') in expected_ids:successes.add(row['id'])
            elif row.get('id') in expected_ids:errors+=1
    status=dict(fingerprint=fingerprint,total_cases=len(cases),successful_unique_cases=len(successes),
                remaining_cases=len(cases)-len(successes),failed_attempts=errors,mode='translation_only',
                alignment_reviewed=False,audio_tested=False)
    atomic_json(root/'completion.json',status)
    if status['remaining_cases']:raise SystemExit(1)


if __name__=='__main__':main()
