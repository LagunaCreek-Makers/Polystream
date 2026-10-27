"""Summarize observed capacity only; count projections are deliberately unvalidated."""
import argparse,hashlib,json,math,os,re,statistics,subprocess
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('results',type=Path);p.add_argument('--root',type=Path,required=True);a=p.parse_args()
state=json.loads((a.results/'status.json').read_text());base=json.loads((a.root/'offline.french.json').read_text(encoding='utf-8-sig'))
profile=dict(machine=state['machine'],device='cuda',threads=4,slots=1,context=4096,layers=99,translation_model=base['translation_model'],whisper_model=base['whisper_model'],cases=[],projections={})
profile['logical_processors']=state['logical_processors']
profile['gpu_identity']=subprocess.run(['nvidia-smi','--query-gpu=uuid,name,memory.total','--format=csv,noheader,nounits'],capture_output=True,text=True,check=True,timeout=5).stdout.strip()
profile['file_signatures']=[]
for key,path in [('translation_model',base['translation_model']),('whisper_model',base['whisper_model']),*base['voices'].items()]:
    file=a.root/path;metadata=file.stat();profile['file_signatures'].append(dict(key=key,path=path,bytes=metadata.st_size,modified=int(metadata.st_mtime)))
lines=['# Stream capacity calibration','',f"Run: {state['phase']}. Source: {state['input_seconds']:.3f} seconds of title-page speech. Single source pass per case, 4 threads/engine, 1 GPU translation slot.",'','| Streams | Language | Completed | Median / p95 / max delay (s) |','|---|---|---:|---|']
for case in state.get('cases',[]):
    folder=Path(case['folder']);summary=case['summary'];languages=[o['language'] for o in summary['outputs']]
    meeting=next((folder/'replay').glob('Meeting_*'));times={}
    for srt in meeting.glob('*.srt'):
        times[srt.name]=re.findall(r'(?m)^\d+\n(\d\d:\d\d:\d\d,\d{3} --> \d\d:\d\d:\d\d,\d{3})',srt.read_text(encoding='utf-8'))
    source=next(v for k,v in times.items() if k.startswith('English'))
    assert all(v==source for v in times.values()),'Subtitle misalignment'
    assert all(o['completed']==summary['accepted_clauses'] for o in summary['outputs']),'Incomplete outputs'
    metrics=[json.loads(line) for line in (meeting/'delays.jsonl').read_text().splitlines()]
    medians=[];p95s=[];maxima=[]
    for output in summary['outputs']:
        delays=sorted(r['delay_seconds'] for r in metrics if r['language']==output['language']);n=len(delays)
        assert n==output['completed']
        median=statistics.median(delays);p95=delays[math.ceil(n*.95)-1];maximum=max(delays)
        medians.append(median);p95s.append(p95);maxima.append(maximum)
        lines.append(f"| {len(languages)} | {output['language']} | {n} | {median:.2f} / {p95:.2f} / {maximum:.2f} |")
    profile['cases'].append(dict(languages=languages,median=max(medians),p95=max(p95s),max=max(maxima),drain_seconds=summary['wall_seconds']-summary['input_seconds']))
    lines.append(f"\nCase {len(languages)}: all {summary['accepted_clauses']} clauses per output; aligned SRTs; final drain {summary['wall_seconds']-summary['input_seconds']:.2f}s.\n")
# An envelope extrapolation from positive observed per-count increments, NOT a
# real-time queue stability model. Never present as a validated count/voice result.
cases=profile['cases']
if len(cases)>=3:
    step_m=max(max(0.,b['median']-c['median']) for c,b in zip(cases,cases[1:]))
    step_p=max(max(0.,b['p95']-c['p95']) for c,b in zip(cases,cases[1:]))
    last=cases[-1];count=len(last['languages'])
    for streams in range(count+1,9):
        delta=streams-count;profile['projections'][str(streams)]={'low':last['median']+step_m*delta,'high':max(last['p95']+step_p*delta,last['median']+step_m*delta),'validated':False}
    lines+=['','## Unvalidated count-only envelope','',json.dumps(profile['projections'],indent=2),'','These projections assume similar speech and voice costs. They use the largest positive observed median/p95 increment per added stream; they do not bound latency, model steady-state queue stability, include acoustic latency, or establish eight-language support on this hardware. Test the exact combination over a sustained meeting before relying on it.']
lines+=['','The earlier 45-minute Spanish/German test reached 125.45/115.46 seconds maximum despite short-test medians near six seconds. Short calibration is not a sustained-session guarantee. CPU/GPU usage alone does not determine delay. Whole-system resource measurements are included in the raw run. Hardware upgrade recommendations require stage-specific evidence.','',f"State: {state['phase']}. Incomplete/failed cases have no invented results. Native-speaker review and physical output devices are not established by this replay."]
(a.results/'capacity-profile.json').write_text(json.dumps(profile,indent=2));(a.results/'capacity-report.md').write_text('\n'.join(lines),encoding='utf-8')
print(json.dumps(profile,indent=2))
