"""Report only observed long-test data, preserving any cancelled attempts."""
import argparse,json,math,re,sys
from pathlib import Path
def main():
    p=argparse.ArgumentParser();p.add_argument('results',type=Path);p.add_argument('--plot-packages',type=Path);a=p.parse_args()
    root=a.results;state=json.loads((root/'status.json').read_text());assert state['phase'] in ('completed','failed')
    if a.plot_packages:sys.path.insert(0,str(a.plot_packages))
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    fig,axes=plt.subplots(2,1,figsize=(12,7));statistics={};lines=['# Sustained simultaneous translation validation','',f"State: **{state['phase']}**. Settings: {state['threads']} CPU threads per inference engine, {state['slots']} GPU translation slot(s).",'', '| Attempt | Language | Completed phrases | Median / p95 / max delay (s) |','|---|---|---:|---|']
    for stage,style in [('full-soundtrack','-'),('speech-only','--')]:
        path=root/stage
        if not path.exists():continue
        for case in sorted(path.glob('*-threads-*-slots-*')):
            summary_file=case/'replay/summary.json';summary=json.loads(summary_file.read_text()) if summary_file.exists() else {}
            transcript_dirs=list((case/'replay').glob('Meeting_*'))
            if not transcript_dirs:continue
            transcripts=transcript_dirs[0];metrics_file=transcripts/'delays.jsonl'
            metrics=[json.loads(line) for line in metrics_file.read_text().splitlines()] if metrics_file.exists() else []
            for code,name,color in [('es','Spanish','tab:orange'),('de','German','tab:blue')]:
                rows=[r for r in metrics if r['language']==code];delays=sorted(r['delay_seconds'] for r in rows)
                if not delays:continue
                n=len(delays);median=(delays[(n-1)//2]+delays[n//2])/2;p95=delays[math.ceil(n*.95)-1]
                statistics[f'{stage}/{code}']={'timed_phrases':n,'median_seconds':median,'p95_seconds':p95,'max_seconds':max(delays)}
                completed=next((o['completed'] for o in summary.get('outputs',[]) if o['language']==code),None)
                lines.append(f"| {stage} | {name} | {completed if completed is not None else 'incomplete'} | {median:.2f} / {p95:.2f} / {max(delays):.2f} |")
                xs=[];ys=[];last=None
                for r in rows:
                    if last is not None and r['source_seconds']-last>15:xs.append(float('nan'));ys.append(float('nan'))
                    xs.append(r['source_seconds']/60);ys.append(r['delay_seconds']);last=r['source_seconds']
                axes[0].plot(xs,ys,style,label=f'{name} ({stage})',color=color,linewidth=1)
            progress_file=case/'progress.jsonl'
            progress=[json.loads(line) for line in progress_file.read_text().splitlines()] if progress_file.exists() else []
            for code,name,color in [('es','Spanish','tab:orange'),('de','German','tab:blue')]:
                points=[(r['wall_seconds']/60,o['oldest_seconds']) for r in progress for o in r.get('outputs',[]) if o['language']==code]
                if points:axes[1].plot([p[0] for p in points],[p[1] for p in points],style,label=f'{name} ({stage})',color=color,linewidth=1)
            if summary:
                lines+=['',f"{stage}: source {summary['input_seconds']/60:.2f} min; wall {summary['wall_seconds']/60:.2f} min; drain {summary['wall_seconds']-summary['input_seconds']:.2f} s; accepted/completed source phrases {summary['accepted_clauses']}; excluded music {summary.get('music_excluded_seconds',0):.2f} s."]
                srt_counts={}
                for srt in transcripts.glob('*.srt'):
                    text=srt.read_text(encoding='utf-8');cues=re.findall(r'(?m)^\d+$',text);times=re.findall(r'(?m)^\d\d:\d\d:\d\d,\d{3} --> \d\d:\d\d:\d\d,\d{3}$',text)
                    assert len(cues)==len(times),f'Malformed SRT: {srt}'
                    srt_counts[srt.name]=len(cues)
                lines.append('Saved SRT cue counts: '+json.dumps(srt_counts)+'.')
                assert len(set(srt_counts.values()))==1,'Language subtitle cue counts differ'
    for ax in axes:ax.grid(alpha=.25);ax.legend();ax.set_ylabel('Seconds')
    axes[0].set(title='Source phrase start → first queued PCM (excludes acoustic latency)',xlabel='Original meeting position (minutes)',xlim=(0,45))
    axes[1].set(title='Current age of oldest recognized phrase pending per language',xlabel='Elapsed time including backlog drain (minutes)')
    fig.tight_layout();fig.savefig(root/'delay-throughout-session.png',dpi=160)
    lines+=['','All timing comes from observed software replay. Unmeasured/cancelled sections are not extrapolated. Subtitle timestamps use the original source timeline. Physical devices, native-speaker accuracy and crash/power-loss recovery are not established by this test.',
        'The cutoff is 600 seconds. A full-soundtrack cutoff triggers one speech-only retry with caption-marked music excluded while preserving the original wall-clock breaks. System/GPU resource logs are whole-system samples, including other workloads.','']
    (root/'performance-report.md').write_text('\n'.join(lines),encoding='utf-8');(root/'performance-statistics.json').write_text(json.dumps(statistics,indent=2))
if __name__=='__main__':main()
