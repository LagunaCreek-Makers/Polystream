from pathlib import Path
import json,math,re,statistics,sys
sys.path.insert(0,str(Path('meeting-stress/plot-packages').resolve()))
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
out=Path('language-rollout/italian-japanese-comparison');out.mkdir(exist_ok=True);stats=[];fig,axes=plt.subplots(1,2,figsize=(12,4.5),sharey=True)
lines=['# Italian / Japanese optional Piper validation','', '100.702-second shared English title-page development clip; Whisper Base, Qwen3 4B Q4_K_M, one GPU translation slot. Piper 1.8 CPU synthesis. Native-speaker/semantic/pronunciation review remains pending. Timing is source phrase start to first queued PCM, not physical acoustic latency.','', '| Runtime | Language | Clauses | Median (s) | p95 (s) | Maximum (s) |','|---|---|---:|---:|---:|---:|']
for ax,name,label in zip(axes,['italian-japanese-modern','italian-japanese-persistent'],['Per-phrase process baseline','Persistent worker; 4 CPU threads/voice']):
    state=json.loads((Path('language-rollout')/name/'status.json').read_text());assert state['phase']=='completed';summary=state['cases'][0]['summary'];meeting=Path(summary['transcripts'])
    srts={f.name:re.findall(r'(?m)^\d+\n(\d\d:\d\d:\d\d,\d{3} --> \d\d:\d\d:\d\d,\d{3})',f.read_text(encoding='utf-8')) for f in meeting.glob('*.srt')};source=next(v for k,v in srts.items() if k.startswith('English'));assert len(source)==38 and all(v==source for v in srts.values())
    metrics=[json.loads(line) for line in (meeting/'delays.jsonl').read_text().splitlines()];outputs=[]
    for language in ['it','ja']:
        rows=[v for v in metrics if v['language']==language];values=sorted(v['delay_seconds'] for v in rows);assert len(values)==38
        row=dict(language=language,clauses=38,median=statistics.median(values),p95=values[math.ceil(.95*len(values))-1],maximum=max(values));outputs.append(row)
        lines.append(f"| {label} | {language} | 38 | {row['median']:.2f} | {row['p95']:.2f} | {row['maximum']:.2f} |")
        ax.plot([v['source_seconds'] for v in rows],[v['delay_seconds'] for v in rows],label=language)
    drain=summary['wall_seconds']-summary['input_seconds'];stats.append(dict(runtime=label,outputs=outputs,drain_seconds=drain,aligned_srt=True));lines += ['',f'{label}: backlog drain {drain:.2f}s; 38 aligned English/Italian/Japanese SRT cues. No cutoff/retry and no unmeasured input in either completed attempt.','']
    ax.set(title=label,xlabel='Source time (s)',xlim=(0,summary['input_seconds']));ax.grid(alpha=.2);ax.legend()
axes[0].set_ylabel('First queued PCM delay (s)');fig.suptitle('Italian / Japanese — startup and bounded CPU-worker change');fig.tight_layout();fig.savefig(out/'delay-comparison.png',dpi=160)
lines += ['', 'The new path both reuses the loaded voice and sets four ONNX CPU threads per voice. This comparison does not isolate their separate contributions and is not a sustained-session guarantee. Existing native Piper voices are unchanged.','', 'The shared speech replay predates the final source-gated Japanese alias correction. Targeted final-adapter name tests are stored separately in italian-japanese-terminology; do not claim the full corrected transcript was replayed unless separately measured. Initial Japanese output contained a Nephi variant; source-gated normalization replaces only observed variants when English input explicitly names the corresponding person/people, in the Church domain. It does not fill missing quotations or establish general accuracy.','', 'Official Japanese reference: https://www.churchofjesuschrist.org/study/scriptures/bofm/bofm-title?lang=jpn . Italian reference: https://www.churchofjesuschrist.org/study/scriptures/bofm?lang=ita . These clips are development material, not held-out evaluation.','', 'Italian Serena dataset card: CC BY 4.0. Japanese Hi-Fi-Captain dataset card: CC BY-NC-SA 4.0; optional experimental pack, separate from default distribution. Modern Piper: OHF-Voice/piper1-gpl v1.8.0; package and third-party notices retained in the isolated environment.']
(out/'validation-report.md').write_text('\n'.join(lines),encoding='utf-8');(out/'statistics.json').write_text(json.dumps(stats,indent=2));print(json.dumps(stats,indent=2))
