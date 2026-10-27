import json,math,re,statistics,sys
from pathlib import Path
root=Path(sys.argv[1]);state=json.loads((root/'status.json').read_text());results=[]
lines=['# Language rollout replay report','',f"State: {state['phase']}; input {state['input_seconds']:.3f}s. 4 threads/engine, one GPU translation slot.",'','Basque translation quality FAILED despite voice operation. This is mechanical timing evidence, not six validated languages. Russian terminology retest is separate. No physical device/acoustic latency or native-speaker quality review.','', '| Case | Language | Clauses | Median | p95 | Maximum |','|---|---|---:|---:|---:|---:|']
for case in state['cases']:
    summary=case['summary'];meeting=Path(summary['transcripts']);metrics=[json.loads(line) for line in (meeting/'delays.jsonl').read_text().splitlines()]
    srts={f.name:re.findall(r'(?m)^\d+\n(\d\d:\d\d:\d\d,\d{3} --> \d\d:\d\d:\d\d,\d{3})',f.read_text(encoding='utf-8')) for f in meeting.glob('*.srt')}
    source=next(v for k,v in srts.items() if k.startswith('English'));assert len(source)==summary['accepted_clauses'];assert all(v==source for v in srts.values())
    outputs=[]
    for o in summary['outputs']:
        d=sorted(m['delay_seconds'] for m in metrics if m['language']==o['language']);assert len(d)==o['completed']==summary['accepted_clauses']
        row=dict(language=o['language'],count=len(d),median=statistics.median(d),p95=d[math.ceil(.95*len(d))-1],max=max(d));outputs.append(row)
        lines.append(f"| {case['case']} | {row['language']} | {row['count']} | {row['median']:.2f} | {row['p95']:.2f} | {row['max']:.2f} |")
    drain=summary['wall_seconds']-summary['input_seconds'];results.append(dict(case=case['case'],outputs=outputs,drain=drain,aligned_srt=True))
    lines += ['',f"Case {case['case']} drain: {drain:.2f}s; SRT cue counts/timestamps aligned.",'']
if state['phase']=='failed':
    folder=Path(state['cases'][-1]['folder']).parent/f"{state['current_case']:02d}-threads-4-slots-1";meeting=next((folder/'replay').glob('Meeting_*'))
    metrics=[json.loads(line) for line in (meeting/'delays.jsonl').read_text().splitlines()]
    partial=[]
    lines += ['', '## Incomplete case '+str(state['current_case']), '', state.get('error',''), 'Pipeline error: incomplete translation rejected; not a 600-second cutoff. No automatic retry. Final polling observation is approximate, not the exact failure instant.', f"Last observed source consumption: {state.get('progress',{}).get('consumed_seconds')} seconds. No completed drain measurement. Unmeasured output remains unvalidated.", '']
    for code in state['language_cases'][state['current_case']-1].split(','):
        values=[m for m in metrics if m['language']==code];d=sorted(m['delay_seconds'] for m in values)
        if not d:continue
        row=dict(language=code,count=len(d),median=statistics.median(d),p95=d[math.ceil(.95*len(d))-1],max=max(d),last_measured_source=max(m['source_seconds'] for m in values));partial.append(row)
        lines.append(f"{code}: {len(d)} measured first-PCM clauses; median/p95/max {row['median']:.2f}/{row['p95']:.2f}/{row['max']:.2f}s; last measured source start {row['last_measured_source']:.2f}s.")
    results.append(dict(case=state['current_case'],status='failed-partial',outputs=partial,drain=None))
    state['cases'].append(dict(case=state['current_case'],summary=dict(transcripts=str(meeting),outputs=[dict(language=row['language']) for row in partial])))
lines += ['','Delays are source phrase start to first queued PCM. No cutoff or retry was needed for completed cases; see explicit failure evidence for partial cases. Failed initial Italian attempt remains preserved separately; zero PCM caused early failure, so remaining input was not tested in that attempt.','', 'The title-page clip is development material, not held-out evaluation. Short test results do not establish sustained meeting performance. Basque is disabled in the shipping UI; results remain diagnostic evidence only.']
(root/'language-report.md').write_text('\n'.join(lines),encoding='utf-8');(root/'language-statistics.json').write_text(json.dumps(results,indent=2))
sys.path.insert(0,str(Path('meeting-stress/plot-packages').resolve()))
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
fig,axes=plt.subplots(len(state['cases']),1,figsize=(10,4*len(state['cases'])),squeeze=False)
for ax,case in zip(axes[:,0],state['cases']):
    meeting=Path(case['summary']['transcripts']);metrics=[json.loads(line) for line in (meeting/'delays.jsonl').read_text().splitlines()]
    for o in case['summary']['outputs']:
        rows=[m for m in metrics if m['language']==o['language']];ax.plot([m['source_seconds'] for m in rows],[m['delay_seconds'] for m in rows],label=o['language']+(' (quality failed)' if o['language']=='eu' else ''))
    ax.set(title='Case '+str(case['case']),xlabel='Source time (s)',ylabel='First queued PCM delay (s)');ax.set_xlim(0,state['input_seconds']);ax.legend(ncol=3);ax.grid(alpha=.2)
fig.suptitle('Language replay — mechanical timing only' + ('; Basque quality failed' if any('eu' in v for v in state['language_cases']) else '; Russian quality review pending'));fig.tight_layout();fig.savefig(root/'language-delay.png',dpi=150)
print(json.dumps(results,indent=2))

