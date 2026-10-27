"""Owned-engine development comparison for actual failed clauses, never held-out."""
from pathlib import Path
import json, re, socket, subprocess, sys, time, urllib.request
root=Path(r'C:\Users\Paul Lott\Documents\GitHub\LagunaCreekMakers\live-translate-rs')
out=Path('language-rollout/qwen8b-runtime').resolve()
sys.path.insert(0,str(root/'scripts'))
from start_offline import engine_commands
from check_offline import local_path
config=json.loads((out/'config.json').read_text(encoding='utf-8'))
code=(root/'src/provider.rs').read_text(encoding='utf-8')
failures=next(Path('language-rollout/priority-eight-15min-fixed').glob('01*/replay/Meeting_*/failures.jsonl'))
cases=[json.loads(s) for s in failures.read_text(encoding='utf-8').splitlines() if '"language":"ar"' in s or '"language":"hi"' in s]
cases=[c for c in cases if c['source_text']!='♪']
cases += [dict(language=lang,clause_id='control-'+str(i),source_text=text) for lang in ('ar','hi') for i,text in enumerate(['The Book of Mormon.','The plates of Nephi.','Mormon wrote an account.','The meeting starts at nine.','Do not change the number fifteen.','She bought fifteen plates for dinner.','The bank account is closed.','I do not agree.'])]
config['translation_model']=str(Path('language-rollout/qwen3-8b/Qwen3-8B-Q4_K_M.gguf').resolve())
http=urllib.request.build_opener(urllib.request.ProxyHandler({}));children=[];logs=[]
try:
    for port in (config['asr_port'],config['translation_port']):
        with socket.socket() as probe:probe.bind(('127.0.0.1',port))
    for name,command in zip(('whisper','llama'),engine_commands(config,lambda p,executable=False:local_path(root,p,executable),'cuda',4)):
        log=(out/f'{name}.log').open('w');logs.append(log)
        children.append(subprocess.Popen(command,cwd=Path(command[0]).parent,stdout=log,stderr=subprocess.STDOUT,creationflags=subprocess.CREATE_NO_WINDOW))
    deadline=time.monotonic()+180
    for child,port in zip(children,(config['asr_port'],config['translation_port'])):
        while True:
            if child.poll() is not None:raise RuntimeError('Owned engine exited')
            try:
                with http.open(f'http://127.0.0.1:{port}/health',timeout=2) as r:
                    if r.status==200:break
            except OSError:pass
            if time.monotonic()>deadline:raise TimeoutError('Engine readiness')
            time.sleep(.25)
    subprocess.run([str(root/'target/debug/examples/offline_benchmark.exe'),str(out/'config.json'),str(out/'cases.json'),str(out/'results'),'--no-translation-cache'],cwd=root,check=True,timeout=300)
finally:
    for child in reversed(children):
        if child.poll() is None:
            child.terminate()
            try:child.wait(timeout=10)
            except subprocess.TimeoutExpired:child.kill();child.wait(timeout=10)
    for log in logs:log.close()
