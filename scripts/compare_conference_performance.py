#!/usr/bin/env python3
"""Controlled local probes for request ordering and exact-repeat cache behavior."""
import argparse
import json
import os
import platform
from pathlib import Path
import re
import statistics
import socket
import subprocess
import time
import urllib.request
from conference_corpus import atomic_json
from check_offline import local_path
from run_conference import cases_from_records, select_records


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self,*args,**kwargs):return None


def probe(server,config_file,model,binary,cases,root,threads,cache,gpu_layers=0):
    root.mkdir()
    with socket.socket() as check:
        check.bind(("127.0.0.1",0))
        port=check.getsockname()[1]
    effective=json.loads(config_file.read_text(encoding='utf-8'))
    for key in ('whisper_server','whisper_model','llama_server','translation_model','piper_binary'):
        value=Path(effective[key])
        if not value.is_absolute():effective[key]=str((config_file.parent/value).resolve())
    effective['voices']={lang:str((config_file.parent/Path(value)).resolve()) for lang,value in effective['voices'].items()}
    effective['translation_port']=port
    if effective['asr_port']==port:effective['asr_port']=port%65535+1
    effective_config=root/'effective-config.json'
    effective_config.write_text(json.dumps(effective),encoding='utf-8')
    opener=urllib.request.build_opener(urllib.request.ProxyHandler({}),NoRedirect())
    case_file=root/'cases.json';case_file.write_text(json.dumps(cases,ensure_ascii=False),encoding='utf-8')
    argv=[str(server),'-m',str(model),'--host','127.0.0.1','--port',str(port),'-t',str(threads),'-ngl',str(gpu_layers),'-c','4096','--parallel','1','--alias','local-translation','--jinja']
    with (root/'server.log').open('w') as log:
        process=subprocess.Popen(argv,stdout=log,stderr=subprocess.STDOUT,cwd=server.parent)
        try:
            for _ in range(240):
                if process.poll() is not None:raise RuntimeError('Local engine exited; inspect probe log')
                try:
                    with opener.open(f'http://127.0.0.1:{port}/health',timeout=1) as response:
                        if response.status==200:break
                except OSError:time.sleep(0.25)
            else:raise RuntimeError('Local engine startup timed out')
            command=[str(binary),str(effective_config),str(case_file),str(root/'results'),'--translation-only']
            if not cache:command.append('--no-translation-cache')
            started=time.monotonic()
            result=subprocess.run(command,check=False)
            wall_ms=(time.monotonic()-started)*1000
        finally:
            process.terminate()
            try:process.wait(timeout=5)
            except subprocess.TimeoutExpired:process.kill();process.wait()
    rows=[json.loads(line) for line in (root/'results/results.jsonl').read_text(encoding='utf-8').splitlines()]
    text=(root/'server.log').read_text(encoding='utf-8')
    prefill=re.findall(r'prompt eval time\s*=\s*([\d.]+) ms /\s*(\d+) tokens',text)
    timings=[row['translation_ms'] for row in rows if row['status']=='ok']
    summary=dict(cases=len(rows),failed=sum(row['status']!='ok' for row in rows),cache_hits=sum(bool(row.get('translation_cache_hit')) for row in rows),
                 wall_ms=wall_ms,median_translation_ms=statistics.median(timings) if timings else None,
                 prompt_eval_ms=sum(float(ms) for ms,_ in prefill),prompt_tokens_evaluated=sum(int(tokens) for _,tokens in prefill))
    return summary,{row['id']:row.get('translation') for row in rows if row['status']=='ok'},rows


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--corpus',type=Path,required=True)
    parser.add_argument('--config',type=Path,required=True)
    parser.add_argument('--binary',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--threads',type=int,default=4)
    parser.add_argument('--gpu-layers',type=int,default=0,help='0 for CPU; configure CUDA-enabled engine before requesting offload')
    args=parser.parse_args()
    config_file=args.config.resolve();config=json.loads(config_file.read_text())
    def config_path(key):
        path=Path(config[key]);return (path if path.is_absolute() else config_file.parent/path).resolve()
    if type(config['translation_port']) is not int or not 1 <= config['translation_port'] <= 65535:parser.error('Invalid local translation port')
    if not 0 <= args.gpu_layers <= 2**32-1:parser.error('Invalid GPU layer count')
    if not 1 <= args.threads <= 128:parser.error('Threads must be 1..128')
    server=local_path(config_file.parent,config['llama_server'],True);model=config_path('translation_model');binary=args.binary.resolve()
    records=select_records(args.corpus/'aligned.jsonl','dev',2,180)
    if len(records)<2:parser.error('Need two short development-set paragraphs')
    all_cases=list(cases_from_records(records))
    interleaved=sorted(all_cases,key=lambda row:(next(i for i,record in enumerate(records) if row['id'].startswith(record['id']+'-')),row['source']+'-'+row['target']))
    grouped=sorted(all_cases,key=lambda row:(row['source']+'-'+row['target'],row['id']))
    root=args.output.resolve();root.mkdir()
    first,translations_a,_=probe(server,config_file,model,binary,interleaved,root/'interleaved',args.threads,False,args.gpu_layers)
    second,translations_b,_=probe(server,config_file,model,binary,grouped,root/'grouped',args.threads,False,args.gpu_layers)
    repeated=[dict(all_cases[0],id=f'repeat-{i}') for i in range(4)]
    no_cache,_,_=probe(server,config_file,model,binary,repeated,root/'repeat-uncached',args.threads,False,args.gpu_layers)
    cached,_,cached_rows=probe(server,config_file,model,binary,repeated,root/'repeat-cached',args.threads,True,args.gpu_layers)
    report=dict(scope='Two 2024 development paragraphs, eight ordering cases, four exact repeats per cache arm',
        threads=args.threads,gpu_layers_requested=args.gpu_layers,hardware=dict(os=platform.system(),architecture=platform.machine(),logical_cpus=os.cpu_count()),
        interleaved=first,grouped=second,identical_outputs_across_orderings=translations_a==translations_b,
        repeat_uncached=no_cache,repeat_cached=cached,
        median_cache_hit_ms=statistics.median([r['translation_ms'] for r in cached_rows if r.get('translation_cache_hit')]) if cached['cache_hits'] else None,
        semantic_accuracy_verified=False,audio_tested=False,repeated_workload_is_not_representative_of_all_talks=True)
    atomic_json(root/'performance-summary.json',report)
    print(json.dumps(report,indent=2))


if __name__=='__main__':main()
