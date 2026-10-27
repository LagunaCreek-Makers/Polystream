"""Run a dual-language meeting rehearsal, retrying speech-only on a 600s cutoff."""
import argparse,datetime,json,subprocess,sys,time
from pathlib import Path
def main():
    p=argparse.ArgumentParser();p.add_argument('--root',type=Path,required=True);p.add_argument('--input',type=Path,required=True)
    p.add_argument('--output',type=Path,required=True);p.add_argument('--music-intervals',type=Path,required=True)
    p.add_argument('--threads',type=int,required=True);p.add_argument('--slots',type=int,required=True);a=p.parse_args()
    out=a.output;out.mkdir(parents=True,exist_ok=False)
    state={'phase':'starting','started_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'threads':a.threads,'slots':a.slots,'attempts':[]}
    def save():
        state['updated_utc']=datetime.datetime.now(datetime.timezone.utc).isoformat()
        temp=out/'status.tmp';temp.write_text(json.dumps(state,indent=2))
        for _ in range(20):
            try:temp.replace(out/'status.json');return
            except PermissionError:time.sleep(.05)
    try:
        for stage in ('full-soundtrack','speech-only'):
            state.update(phase='running',stage=stage);save()
            command=[sys.executable,str(Path(__file__).with_name('benchmark_dual.py')),'--root',str(a.root),'--input',str(a.input),'--output',str(out/stage),'--cases',f'{a.threads}:{a.slots}']
            if stage=='speech-only':command+=['--music-intervals',str(a.music_intervals)]
            with (out/f'{stage}.log').open('w') as log:
                result=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT,creationflags=subprocess.CREATE_NO_WINDOW)
            attempt=json.loads((out/stage/'status.json').read_text());state['attempts'].append(attempt);save()
            if result.returncode==0:
                state.update(phase='completed',passed=True);save();return
            failures='\n'.join(f.read_text(errors='replace') for f in (out/stage).rglob('failure.txt'))
            if stage=='full-soundtrack' and 'Delay exceeded ten minutes' in failures:
                state['phase']='retrying_without_music';save();continue
            raise RuntimeError(f'{stage} failed: {attempt.get("error",failures)}')
    except BaseException as error:
        state.update(phase='failed',passed=False,error=str(error));save();raise
if __name__=='__main__':main()
