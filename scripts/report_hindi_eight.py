from pathlib import Path
import json,math,re,statistics,sys
sys.path.insert(0,str(Path('meeting-stress/plot-packages').resolve()))
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
names=['hindi-first-replay','hindi-persistent-replay','eight-language-first-replay'];output=Path('language-rollout/hindi-eight-report');output.mkdir(exist_ok=True)
lines=['# Hindi and eight-language speech validation','', 'Input: 100.702-second title-page development clip. Shared Whisper Base recognition, Qwen3 4B Q4_K_M translation, four CPU threads per engine, one GPU translation slot. Persistent Piper 1.8 CPU workers use four threads each where configured. Other voices use existing native Piper.','', 'Timing: source phrase start to first queued PCM; physical audio devices/acoustic latency not measured. Short replay is not a sustained capacity guarantee. All languages still require appropriate native-speaker/semantic review.','', '| Run | Language | Completed PCM clauses | Median (s) | p95 (s) | Max (s) |','|---|---|---:|---:|---:|---:|'];stats=[];fig,axes=plt.subplots(len(names),1,figsize=(11,3.6*len(names)),squeeze=False)
for ax,name in zip(axes[:,0],names):
    base=Path('language-rollout')/name;state=json.loads((base/'status.json').read_text());assert state['phase'] in ('completed','failed')
    complete=state['phase']=='completed';folder=base/'01-threads-4-slots-1/replay';meeting=next(folder.glob('Meeting_*'));rows=[json.loads(line) for line in (meeting/'delays.jsonl').read_text(encoding='utf-8').splitlines()];outputs=[]
    srts={f.name:re.findall(r'(?m)^\d+\n(\d\d:\d\d:\d\d,\d{3} --> \d\d:\d\d:\d\d,\d{3})',f.read_text(encoding='utf-8')) for f in meeting.glob('*.srt')};source=next(v for k,v in srts.items() if k.startswith('English'));aligned=all(v==source for v in srts.values())
    summary=json.loads((folder/'summary.json').read_text()) if complete else None
    for code in state['language_cases'][0].split(','):
        measured=[row for row in rows if row['language']==code];delays=sorted(row['delay_seconds'] for row in measured)
        if complete:assert len(delays)==summary['accepted_clauses']==len(source) and aligned
        result=dict(language=code,count=len(delays),median=statistics.median(delays) if delays else None,p95=delays[math.ceil(.95*len(delays))-1] if delays else None,max=max(delays) if delays else None,last_measured_source=max((row['source_seconds'] for row in measured),default=None));outputs.append(result)
        if delays:
            lines.append(f"| {name} | {code} | {len(delays)} | {result['median']:.2f} | {result['p95']:.2f} | {result['max']:.2f} |")
            ax.plot([row['source_seconds'] for row in measured],[row['delay_seconds'] for row in measured],label=code)
    drain=summary['wall_seconds']-summary['input_seconds'] if complete else None
    stats.append(dict(run=name,status=state['phase'],outputs=outputs,drain_seconds=drain,srt_timestamps_aligned=aligned if complete else None,last_progress=state.get('progress'),error=state.get('error')))
    lines += ['', f"{name}: {state['phase']}; drain {f'{drain:.2f}s' if drain is not None else 'unmeasured'}; SRT timestamps {'aligned' if complete and aligned else 'partial/unvalidated'}. {state.get('error','No cutoff or retry. All accepted clauses measured.')}", '']
    if not complete:lines += ['Incomplete portions are not counted as passing. Plot lines end at each language\'s last measured source clause; no extrapolation into unmeasured input.','']
    ax.set(title=name,xlabel='Source time (s)',ylabel='First queued PCM delay (s)',xlim=(0,state['input_seconds']));ax.grid(alpha=.2);ax.legend(ncol=4)
fig.suptitle('Hindi runtime comparison and eight simultaneous output routes');fig.tight_layout();fig.savefig(output/'delay-comparison.png',dpi=160)
lines += ['', '## Quality and distribution limits','', 'Hindi uses the Pratham medium voice. Upstream model card identifies CC BY-NC-SA 4.0 dataset terms; intended use is noncommercial and shared. Keep model/dataset attribution and component-specific notices. This does not change the application source license.','', 'Official Hindi title-page reference: https://www.churchofjesuschrist.org/study/scriptures/bofm/bofm-title?lang=hin . Initial names मॉरमन की पुस्तक and नफी appear, but full-replay inspection found omitted names and questionable wording (for example financial-account wording खाता in narrative context). A name substring pass is not semantic approval. No scripture completion or invented recovery is performed. Native review and held-out passages remain pending.','', 'The six-case Hindi text/speech regression is separate from live-input latency. Initial cold worker load is included in that test; it must not be compared directly to source-start speech delays.','', 'A Windows Nirmala TTC fallback is loaded for Devanagari when installed. Complete Indic shaping and physical UI/audio review have not been established by these automated tests.','', 'Raw evidence is retained in workspace language-rollout under each run name. Existing unsuccessful six-stream/Basque and Italian-old-runtime attempts remain preserved separately.']
(output/'validation-report.md').write_text('\n'.join(lines),encoding='utf-8');(output/'statistics.json').write_text(json.dumps(stats,indent=2));print(json.dumps([{k:v for k,v in run.items() if k not in ('last_progress',)} for run in stats],indent=2))
