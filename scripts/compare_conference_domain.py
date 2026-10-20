#!/usr/bin/env python3
"""Compare the religious terminology profile with saved general-profile results."""
import argparse
from collections import Counter
import json
from pathlib import Path
from check_offline import local_path
from compare_conference_performance import probe
from conference_corpus import atomic_json
from run_conference import result_rows
from score_conference import review_flags


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline-results',type=Path,required=True)
    parser.add_argument('--config',type=Path,required=True,help='General-profile config used by the baseline')
    parser.add_argument('--binary',type=Path,required=True)
    parser.add_argument('--paragraph-ids',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--threads',type=int,default=4)
    parser.add_argument('--gpu-layers',type=int,default=0)
    args=parser.parse_args()
    try:from sacrebleu.metrics import CHRF
    except ImportError:parser.error('Install evaluation/requirements.txt before scoring')
    if not 1 <= args.threads <= 128 or not 0 <= args.gpu_layers <= 2**32-1:parser.error('Invalid threads/GPU layers')
    config_file=args.config.resolve();config=json.loads(config_file.read_text(encoding='utf-8'))
    if config.get('translation_domain','general') != 'general':parser.error('Baseline config must use the general domain')
    ids=json.loads(args.paragraph_ids.read_text(encoding='utf-8'))
    if not isinstance(ids,list) or not 1 <= len(ids) <= 50 or not all(isinstance(i,str) for i in ids):parser.error('Choose 1–50 paragraph IDs')
    case_ids=set(ids);cases={}
    for path in sorted(args.baseline_results.glob('cases-*.json')):
        for case in json.loads(path.read_text(encoding='utf-8')):
            if case['id'].rsplit('-',2)[0] in case_ids:cases[case['id']]=case
    if len(cases)!=len(case_ids)*4:parser.error('Baseline does not contain all four directions for every selected paragraph')
    before={}
    for path in sorted(args.baseline_results.glob('attempt-*/results.jsonl')):
        for row in result_rows(path):
            if row.get('status')=='ok' and row['id'] in cases:before[row['id']]=row
    root=args.output.resolve();root.mkdir()
    model=local_path(config_file.parent,config['translation_model'])
    server=local_path(config_file.parent,config['llama_server'],True)
    # Rebase paths before writing the updated profile in a different directory.
    for key in ('whisper_server','whisper_model','llama_server','translation_model','piper_binary'):
        config[key]=str(local_path(config_file.parent,config[key],key.endswith('server') or key=='piper_binary'))
    config['voices']={lang:str(local_path(config_file.parent,value)) for lang,value in config['voices'].items()}
    config['translation_domain']='latter_day_saint'
    updated=root/'religious-config.json';atomic_json(updated,config)
    summary,_,after=probe(server,updated,model,args.binary.resolve(),list(cases.values()),root/'religious',args.threads,False,args.gpu_layers)
    paired=[row for row in after if row['status']=='ok' and row['id'] in before]
    metric=CHRF(char_order=6,word_order=0,beta=2)
    reference=[[row['reference_translation'] for row in paired]]
    report=dict(requested_cases=len(cases),paired_successful_cases=len(paired),baseline_domain='general',updated_domain='latter_day_saint',
        baseline_chrf=metric.corpus_score([before[row['id']]['translation'] for row in paired],reference).score if paired else None,
        updated_chrf=metric.corpus_score([row['translation'] for row in paired],reference).score if paired else None,
        metric_signature=str(metric.get_signature()) if paired else None,
        baseline_review_flags=dict(Counter(flag for row in before.values() for flag in review_flags(row))),
        updated_review_flags=dict(Counter(flag for row in after if row['status']=='ok' for flag in review_flags(row))),
        updated_timing=summary,baseline_case_ids=list(before),independent_heldout_evaluation=False,semantic_accuracy_verified=False,
        note='Targeted comparison against saved baseline outputs; reference alignment remains unreviewed. Timings are not a controlled speed comparison across profiles.')
    atomic_json(root/'domain-summary.json',report)
    print(json.dumps(report,indent=2))


if __name__=='__main__':main()
