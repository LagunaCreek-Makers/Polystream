"""Sequential GPU experiments; never reuse/stop unowned engine processes."""
import argparse,ctypes,datetime,json,math,os,socket,subprocess,sys,time,urllib.request
from pathlib import Path

class Memory(ctypes.Structure):
    _fields_=[('length',ctypes.c_ulong),('load',ctypes.c_ulong)]+[(n,ctypes.c_ulonglong) for n in ('total','available','total_page','available_page','total_virtual','available_virtual','extended')]

def system_sample():
    idle=ctypes.c_ulonglong();kernel=ctypes.c_ulonglong();user=ctypes.c_ulonglong()
    if not ctypes.windll.kernel32.GetSystemTimes(ctypes.byref(idle),ctypes.byref(kernel),ctypes.byref(user)):
        raise ctypes.WinError()
    memory=Memory();memory.length=ctypes.sizeof(memory)
    if not ctypes.windll.kernel32.GlobalMemoryStatusEx(ctypes.byref(memory)): raise ctypes.WinError()
    return (idle.value,kernel.value+user.value),memory.total-memory.available

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root',type=Path,required=True);parser.add_argument('--input',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--cases',nargs='+',default=['4:1','8:1','16:1','4:2','4:1'])
    parser.add_argument('--music-intervals',type=Path)
    args=parser.parse_args();root=args.root.resolve();out=args.output.resolve();source=args.input.resolve()
    pairs=[]
    for case in args.cases:
        threads,slots=map(int,case.split(':'))
        if not (1<=threads<=128 and slots in (1,2)): parser.error('Use threads:slots with threads 1..128, slots 1 or 2')
        pairs.append((threads,slots))
    out.mkdir(parents=True,exist_ok=False)
    sys.path.insert(0,str(root/'scripts'))
    from start_offline import engine_commands
    from check_offline import local_path,check_configuration
    from warmup_offline import warmup
    original=json.loads((root/'offline.cuda.json').read_text(encoding='utf-8'))
    errors=check_configuration(original,root)
    if errors: raise RuntimeError('\n'.join(errors))
    state={'phase':'starting','started_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'logical_processors':os.cpu_count(),'input_seconds':source.stat().st_size/32000.,'cases':[]}
    def save():
        state['updated_utc']=datetime.datetime.now(datetime.timezone.utc).isoformat()
        temp=out/'status.tmp';temp.write_text(json.dumps(state,indent=2),encoding='utf-8')
        # Windows readers may not share DELETE access. Telemetry contention must
        # never cancel an inference test; defer this update if the reader persists.
        for attempt in range(20):
            try:
                temp.replace(out/'status.json');return
            except PermissionError:
                time.sleep(.05)
        print('Progress file busy; deferring status update',file=sys.stderr,flush=True)
    http=urllib.request.build_opener(urllib.request.ProxyHandler({}));save()
    try:
        for index,(threads,slots) in enumerate(pairs):
            folder=out/f'{index+1:02d}-threads-{threads}-slots-{slots}';folder.mkdir()
            config=dict(original,threads=threads)
            # Snapshot with absolute model/engine paths; do not mutate installed config.
            for key in ('piper_binary','whisper_server','whisper_model','llama_server','translation_model'):
                config[key]=str(local_path(root,config[key],key.endswith('server') or key=='piper_binary'))
            config['voices']={k:str(local_path(root,v)) for k,v in config['voices'].items()}
            config_path=folder/'config.json';config_path.write_text(json.dumps(config,indent=2))
            state.update(phase='starting_engines',current_case=index+1,threads=threads,slots=slots);save()
            children=[];logs=[]
            try:
                for port in (config['asr_port'],config['translation_port']):
                    with socket.socket() as probe: probe.bind(('127.0.0.1',port))
                commands=engine_commands(config,lambda p,executable=False:local_path(root,p,executable),'cuda',threads)
                llama=commands[1];llama[llama.index('--parallel')+1]=str(slots)
                # Keep 4096 tokens PER slot so the comparison does not reduce context.
                llama[llama.index('-c')+1]=str(4096*slots)
                (folder/'commands.json').write_text(json.dumps(commands,indent=2))
                for name,command in zip(('whisper','llama'),commands):
                    log=(folder/f'{name}.log').open('w',encoding='utf-8');logs.append(log)
                    children.append(subprocess.Popen(command,cwd=Path(command[0]).parent,stdout=log,stderr=subprocess.STDOUT,creationflags=subprocess.CREATE_NO_WINDOW))
                state['owned_engine_pids']=[p.pid for p in children];save();deadline=time.monotonic()+180
                for child,port in zip(children,(config['asr_port'],config['translation_port'])):
                    while True:
                        if child.poll() is not None: raise RuntimeError('Engine exited; inspect per-case logs')
                        try:
                            with http.open(f'http://127.0.0.1:{port}/health',timeout=2) as response:
                                if response.status==200: break
                        except OSError: pass
                        if time.monotonic()>deadline: raise TimeoutError('Engine readiness timeout')
                        time.sleep(.25)
                state['phase']='warming';save();warmup(config)
                replay_log=(folder/'replay.log').open('w');logs.append(replay_log)
                replay_command=[str(root/'target/debug/examples/replay_dual.exe'),str(config_path),str(source),str(folder/'replay')]
                if args.music_intervals: replay_command.append(str(args.music_intervals.resolve()))
                replay=subprocess.Popen(replay_command,cwd=root,stdout=replay_log,stderr=subprocess.STDOUT,creationflags=subprocess.CREATE_NO_WINDOW)
                children.append(replay);state.update(phase='running',replay_pid=replay.pid);save()
                last,_=system_sample();samples=[];began=time.monotonic();deadline=began+state['input_seconds']+660
                with (folder/'resources.jsonl').open('w') as resource_log, (folder/'progress.jsonl').open('w') as progress_log:
                    while replay.poll() is None:
                        if any(p.poll() is not None for p in children[:-1]): raise RuntimeError('Engine exited during replay')
                        if time.monotonic()>deadline: raise TimeoutError('Replay safety timeout')
                        time.sleep(2)
                        counters,used=system_sample();idle_delta=counters[0]-last[0];total_delta=counters[1]-last[1];last=counters
                        sample={'wall_seconds':time.monotonic()-began,'system_cpu_percent':100*(1-idle_delta/total_delta) if total_delta>0 else None,'system_used_ram_bytes':used}
                        try:
                            gpu=subprocess.run(['nvidia-smi','--query-gpu=utilization.gpu,memory.used','--format=csv,noheader,nounits'],capture_output=True,text=True,timeout=3,creationflags=subprocess.CREATE_NO_WINDOW)
                            if gpu.returncode==0:
                                util,mib=gpu.stdout.splitlines()[0].split(',');sample.update(gpu_percent=float(util),gpu_used_mib=float(mib))
                        except (OSError,ValueError,subprocess.TimeoutExpired): pass
                        samples.append(sample);resource_log.write(json.dumps(sample)+'\n');resource_log.flush()
                        try:
                            state['progress']=json.loads((folder/'replay/progress.json').read_text())
                            progress_log.write(json.dumps(state['progress'])+'\n');progress_log.flush()
                        except (OSError,ValueError): pass
                        state['resources']=sample;save()
                if replay.returncode: raise RuntimeError(f'Replay exited {replay.returncode}; inspect {folder}')
                summary=json.loads((folder/'replay/summary.json').read_text())
                resources={}
                for key in ('system_cpu_percent','system_used_ram_bytes','gpu_percent','gpu_used_mib'):
                    values=[s[key] for s in samples if s.get(key) is not None]
                    if values: resources[key]={'mean':sum(values)/len(values),'max':max(values)}
                state['cases'].append({'case':index+1,'threads':threads,'slots':slots,'summary':summary,'resources':resources,'folder':str(folder)})
                state.pop('progress',None);save()
            finally:
                for child in reversed(children):
                    if child.poll() is None:
                        child.terminate()
                        try: child.wait(timeout=10)
                        except subprocess.TimeoutExpired: child.kill();child.wait(timeout=10)
                for log in logs: log.close()
                state.pop('replay_pid',None);state.pop('owned_engine_pids',None)
        state['phase']='completed';save()
    except BaseException as error:
        state.update(phase='failed',error=str(error));save();raise

if __name__=='__main__': main()
