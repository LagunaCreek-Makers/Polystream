"""Development-only prompt bias comparison on original audio and negative controls."""
from pathlib import Path
import io,json,socket,subprocess,sys,time,urllib.request,uuid,wave
root=Path(r'C:\Users\Paul Lott\Documents\GitHub\LagunaCreekMakers\live-translate-rs')
out=Path('language-rollout/asr-long-context');out.mkdir(exist_ok=False)
sys.path.insert(0,str(root/'scripts'))
from start_offline import engine_commands
from check_offline import local_path
config=json.loads((root/'offline.priority-eight.json').read_text(encoding='utf-8'))
http=urllib.request.build_opener(urllib.request.ProxyHandler({}))
with socket.socket() as probe:probe.bind(('127.0.0.1',config['asr_port']))
command=engine_commands(config,lambda p,executable=False:local_path(root,p,executable),'cuda',4)[0]
log=(out/'whisper.log').open('w');child=subprocess.Popen(command,cwd=Path(command[0]).parent,stdout=log,stderr=subprocess.STDOUT,creationflags=subprocess.CREATE_NO_WINDOW)
try:
    deadline=time.monotonic()+120
    while True:
        if child.poll() is not None:raise RuntimeError('Owned engine exited')
        try:
            with http.open(f'http://127.0.0.1:{config["asr_port"]}/health',timeout=2) as r:
                if r.status==200:break
        except OSError:pass
        if time.monotonic()>deadline:raise TimeoutError('Readiness')
        time.sleep(.25)
    source=Path('language-rollout/meeting-first-15-minutes.pcm').read_bytes()
    cases=[('oaks-context',514.0,523.0,'Dallin Harris Oaks'),('kearon-soares-context',590.0,601.0,'Patrick Kearon'),('ordinary-context',34.0,43.0,None),('ordinary',36.5,40.5,None),('silence',None,None,None)]
    prompt='The Church of Jesus Christ of Latter-day Saints. Dallin H. Oaks, Patrick Kearon, Ulisses Soares. Book of Mormon, Nephi.'
    with (out/'results.jsonl').open('w',encoding='utf-8') as results:
        for variant in ('baseline','domain_hint'):
            for name,start,end,expected in cases:
                pcm=source[int(start*32000):int(end*32000)] if start is not None else bytes(128000)
                buf=io.BytesIO()
                with wave.open(buf,'wb') as wav:wav.setnchannels(1);wav.setsampwidth(2);wav.setframerate(16000);wav.writeframes(pcm)
                fields=dict(language='en',translate='false',temperature='0.0',temperature_inc='0.0',response_format='json',prompt=prompt if variant=='domain_hint' else '')
                boundary=uuid.uuid4().hex;body=bytearray()
                for key,value in fields.items():body.extend(f'--{boundary}\r\nContent-Disposition: form-data; name="{key}"\r\n\r\n{value}\r\n'.encode())
                body.extend(f'--{boundary}\r\nContent-Disposition: form-data; name="file"; filename="clip.wav"\r\nContent-Type: audio/wav\r\n\r\n'.encode());body.extend(buf.getvalue());body.extend(f'\r\n--{boundary}--\r\n'.encode())
                request=urllib.request.Request(f'http://127.0.0.1:{config["asr_port"]}/inference',bytes(body),{'Content-Type':f'multipart/form-data; boundary={boundary}'})
                began=time.monotonic()
                with http.open(request,timeout=35) as r:response=json.load(r)
                row=dict(variant=variant,case=name,source_start=start,source_end=end,prompt=fields['prompt'],text=response.get('text'),expected_name=expected,seconds=time.monotonic()-began)
                results.write(json.dumps(row,ensure_ascii=False)+'\n');results.flush();print(json.dumps(row,ensure_ascii=True),flush=True)
finally:
    if child.poll() is None:
        child.terminate()
        try:child.wait(timeout=10)
        except subprocess.TimeoutExpired:child.kill();child.wait(timeout=10)
    log.close()
