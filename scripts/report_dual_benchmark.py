"""Create a comparison report from completed benchmark evidence."""
import argparse,json,sys,math,statistics
from pathlib import Path
def main():
    parser=argparse.ArgumentParser();parser.add_argument('results',type=Path);parser.add_argument('--plot-packages',type=Path)
    args=parser.parse_args();root=args.results;state=json.loads((root/'status.json').read_text())
    assert state['phase']=='completed','Only complete matrices can be compared'
    cases=state['cases'];baselines=[c for c in cases if (c['threads'],c['slots'])==(4,1)]
    def score(case): return max(o['p95_seconds'] for o in case['summary']['outputs'])
    baseline_mean=statistics.mean(map(score,baselines));band=max(map(score,baselines))-min(map(score,baselines))
    winner=min(cases,key=score)
    # A short-clip setting is only promoted if its gain exceeds both repeat
    # variability and five percent, and no language exceeds baseline p95 bounds.
    target_bounds={o['language']:max(next(x for x in c['summary']['outputs'] if x['language']==o['language'])['p95_seconds'] for c in baselines) for o in baselines[0]['summary']['outputs']}
    qualifies=baseline_mean-score(winner)>max(band,.05*baseline_mean) and all(o['p95_seconds']<=target_bounds[o['language']] for o in winner['summary']['outputs'])
    choice={'threads':winner['threads'],'slots':winner['slots']} if qualifies else {'threads':4,'slots':1}
    lines=['# Simultaneous translation CPU/GPU comparison','',f"Source: {state['input_seconds']:.3f} seconds, shared English recognition → Spanish and German.",
        'Each case reloads and warms the models, disables translation caching, and preserves 4,096 context tokens per GPU slot.',
        '', '| Case | Threads/engine | GPU slots | Spanish median / p95 / max (s) | German median / p95 / max (s) | Drain (s) |', '|---|---:|---:|---|---|---:|']
    plot_rows=[]
    for c in cases:
        summary=c['summary'];assert summary['status']=='passed';assert all(o['completed']==38 for o in summary['outputs'])
        outputs={o['language']:o for o in summary['outputs']}
        def fmt(code):
            o=outputs[code];return f"{o['median_seconds']:.2f} / {o['p95_seconds']:.2f} / {o['max_seconds']:.2f}"
        lines.append(f"| {c['case']} | {c['threads']} | {c['slots']} | {fmt('es')} | {fmt('de')} | {summary['wall_seconds']-summary['input_seconds']:.2f} |")
        plot_rows.append((f"{c['threads']} threads / {c['slots']} slots\n(case {c['case']})",outputs))
    lines+=['',f"Repeated 4-thread/1-slot worst-language p95 range: {min(map(score,baselines)):.2f}–{max(map(score,baselines)):.2f} seconds.",
        f"Selected for sustained validation: **{choice['threads']} CPU threads per inference engine, {choice['slots']} GPU translation slot(s)**.",
        ('The gain cleared the repeat-variability/5% gate without worsening either language beyond the baseline bounds.' if qualifies else 'The tested alternatives did not establish a clear improvement beyond the repeated-baseline variation/5% gate. Retain the existing defaults.'),
        '', '## Resource observations','', '| Case | System CPU mean / sampled max | System RAM sampled max (GiB) | GPU sampled max (MiB) |', '|---|---|---:|---:|']
    for c in cases:
        r=c['resources'];cpu=r.get('system_cpu_percent',{});mem=r.get('system_used_ram_bytes',{});gpu=r.get('gpu_used_mib',{})
        lines.append(f"| {c['case']} | {cpu.get('mean',0):.1f}% / {cpu.get('max',0):.1f}% | {mem.get('max',0)/2**30:.2f} | {gpu.get('max',0):.0f} |")
    lines+=['','Counters are whole-system/whole-GPU, sampled approximately every two seconds. Other workloads are included and short peaks may be missed. This is not isolated process utilization.',
        'Delay is source phrase start to first queued PCM, excluding physical audio hardware latency. These 100-second tests do not establish long-meeting stability or native-speaker accuracy.',
        'Synthesis and the duration of the spoken translations can limit catch-up even when recognition/translation computation is fast. Additional GPU slots do not accelerate already-generated audio playback.',
        'The earlier status-file-sharing failure is preserved in the sibling thread-slot-benchmark directory and is not counted as a completed matrix case.','']
    (root/'comparison.md').write_text('\n'.join(lines),encoding='utf-8')
    (root/'selected-settings.json').write_text(json.dumps(dict(choice,short_test_only=True,clear_gain=qualifies,baseline_p95_span_seconds=band),indent=2))
    if args.plot_packages: sys.path.insert(0,str(args.plot_packages))
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    fig,ax=plt.subplots(figsize=(11,4));xs=list(range(len(plot_rows)))
    for code,name,offset in [('es','Spanish',-.18),('de','German',.18)]:
        ax.bar([x+offset for x in xs],[row[1][code]['p95_seconds'] for row in plot_rows],.35,label=name)
    ax.set_xticks(xs,[row[0] for row in plot_rows]);ax.set(ylabel='95th percentile delay (seconds)',title='Shared-input dual translation: thread and GPU-slot comparison')
    ax.grid(axis='y',alpha=.25);ax.legend();fig.tight_layout();fig.savefig(root/'comparison.png',dpi=160)
    print(json.dumps(choice))
if __name__=='__main__':main()
