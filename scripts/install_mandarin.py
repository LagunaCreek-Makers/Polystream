"""Explicit experimental Mandarin voice setup; never changes the default model pack."""
import argparse,json,subprocess,sys
from pathlib import Path
def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--download',action='store_true');a=p.parse_args()
    root=Path(__file__).resolve().parents[1]
    print('Experimental local evaluation: upstream Huayan dataset license is listed as Unknown. Not a validated/redistributable voice pack.',flush=True)
    subprocess.run([sys.executable,str(root/'scripts/fetch_models.py'),'--download' if a.download else '--verify','--manifest',str(root/'models-mandarin-experimental.json')],check=True)
    config=json.loads((root/'offline.cuda.json').read_text(encoding='utf-8'))
    config['voices']['zh-Hans']='models/piper/zh_CN-huayan-medium.onnx'
    destination=root/'offline.mandarin.experimental.json'
    if destination.exists():
        existing=json.loads(destination.read_text(encoding='utf-8'))
        if existing.get('voices',{}).get('zh-Hans')!=config['voices']['zh-Hans']:
            raise SystemExit('Existing experimental configuration differs; refusing to overwrite it')
    else:
        with destination.open('x',encoding='utf-8') as file:json.dump(config,file,indent=2)
    print('Ready for experimental testing. Launch Start-Translation-GPU.cmd --config offline.mandarin.experimental.json')
    print('Choose Mandarin Chinese (experimental) as an output. English remains the source. Native-speaker review is still required.')
if __name__=='__main__':main()
