#!/usr/bin/env python3
"""Score local results with SacreBLEU chrF; does not establish semantic correctness."""
import argparse
from collections import Counter, defaultdict
import json
import re
from pathlib import Path
import statistics


TERMS = {
    'es': {'covenants':['convenio'], 'priesthood':['sacerdocio'], 'sacrament':['santa cena'],
           'atonement':['expiación','expiatorio'], 'holy_ghost':['espíritu santo'],
           'ward':['barrio'], 'stake':['estaca'], 'bishop':['obispo']},
    'de': {'covenants':['bündnis'], 'priesthood':['priestertum'], 'sacrament':['abendmahl'],
           'atonement':['sühn'], 'holy_ghost':['heiliger geist','heiligen geist','heiligem geist'],
           'ward':['gemeinde'], 'stake':['pfahl'], 'bishop':['bischof']}
}


def review_flags(row):
    reference=row.get('reference_translation','').casefold()
    output=row.get('translation','').casefold()
    flags=[]
    def has_term(text,forms):
        return any(re.search(r'(?<!\w)'+re.escape(form)+r'\w*',text) for form in forms)
    for term,forms in TERMS.get(row['target'],{}).items():
        if has_term(reference,forms) and not has_term(output,forms):
            flags.append('terminology:'+term)
    # Arabic-digit strings only. Spelled-out numbers/locale formatting require review.
    if Counter(re.findall(r'\d+',reference)) != Counter(re.findall(r'\d+',output)):
        flags.append('digit_difference')
    negative = {'es':r'\b(no|nunca|jamás|sin|ni)\b', 'de':r'\b(nicht|niemals|nie|ohne|weder|kein\w*)\b'}.get(row['target'])
    if negative and re.search(negative,reference) and not re.search(negative,output):
        flags.append('possible_missing_negation')
    if len(output)>max(120,len(reference)*3):flags.append('large_expansion')
    return flags


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--results',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    try:from sacrebleu.metrics import CHRF
    except ImportError:parser.error('Install evaluation-only dependency: python -m pip install sacrebleu==2.6.0')
    metric=CHRF(char_order=6,word_order=0,beta=2)
    latest={};failed=0
    for path in sorted(args.results.glob('attempt-*/results.jsonl')):
        for line in path.read_text(encoding='utf-8').splitlines():
            try:row=json.loads(line)
            except ValueError:continue
            if row.get('status')=='ok':latest[row['id']]=row
            else:failed+=1
    groups=defaultdict(list)
    conferences=defaultdict(list)
    for row in latest.values():
        groups[row['source']+'-'+row['target']].append(row)
        match=re.match(r'gc-(\d{4})-(04|10)-',row['id'])
        if match:conferences[match[1]+'-'+match[2]].append(row)
    scores={}
    for pair,rows in sorted(groups.items()):
        score=metric.corpus_score([r['translation'] for r in rows],[[r['reference_translation'] for r in rows]])
        times=sorted(r['translation_ms'] for r in rows)
        scores[pair]=dict(cases=len(rows),chrf=score.score,median_translation_ms=statistics.median(times),
                          p95_translation_ms=times[max(0,(95*len(times)+99)//100-1)],
                          cache_hits=sum(bool(r.get('translation_cache_hit')) for r in rows))
    flagged=[dict(id=row['id'],flags=flags) for row in latest.values() if (flags:=review_flags(row))]
    flag_counts=dict(Counter(flag for row in flagged for flag in row['flags']))
    conference_counts={key:len(rows) for key,rows in sorted(conferences.items())}
    report=dict(review_flag_counts=flag_counts,cases_for_review=flagged,flags_are_diagnostics_not_errors=True,conferences_with_successful_cases=len(conferences),cases_by_conference=conference_counts,successful_unique_cases=len(latest),failed_attempts=failed,by_pair=scores,
                metric_signature=str(metric.get_signature()) if latest else None,
                semantic_accuracy_verified=False,alignment_reviewed=False,audio_tested=False)
    state_path=args.results/'state.json'
    if state_path.exists():
        total=json.loads(state_path.read_text(encoding='utf-8'))['total_cases']
        report['requested_cases']=total
        report['remaining_unique_cases']=total-len(latest)
    with args.output.open('x',encoding='utf-8') as output:json.dump(report,output,indent=2);output.write('\n')
    print(json.dumps(report,indent=2))


if __name__=='__main__':main()
