"""Optional Piper 1.8 setup for Italian/Japanese; does not replace native Piper."""
import argparse,json,subprocess,sys
from pathlib import Path
PACKS={'ar':'ar_JO-kareem-medium','hi':'hi_IN-pratham-medium','it':'it_IT-serena-medium','ja':'ja_JP-hi_fi_captain-medium'}
def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('language',choices=PACKS)
    parser.add_argument('--download',action='store_true',help='Explicitly install the isolated runtime and voice')
    parser.add_argument('--python',type=Path,help='Python interpreter to create the isolated environment (tested with Python 3.12)')
    args=parser.parse_args();root=Path(__file__).resolve().parents[1]
    if args.language=='ar':
        print('Experimental Arabic Kareem: repository metadata MIT; dataset provenance/license unresolved (See URL). Keep model card and review redistribution before sharing this pack.',flush=True)
    if args.language=='hi':
        print('Hindi Pratham: upstream dataset CC BY-NC-SA 4.0. Keep model/dataset attribution and license notices.',flush=True)
    if args.language=='ja':
        print('Experimental Japanese: upstream Hi-Fi-Captain dataset CC BY-NC-SA 4.0. Not in the default distributable pack.',flush=True)
    subprocess.run([sys.executable,str(root/'scripts/fetch_models.py'),'--download' if args.download else '--verify','--manifest',str(root/f'models-{args.language}.json')],check=True)
    environment=root/'engines/piper-modern/venv';python=environment/'Scripts/python.exe'
    if args.download:
        if not python.is_file():
            subprocess.run([str(args.python or sys.executable),'-m','venv',str(environment)],check=True)
        subprocess.run([str(python),'-m','pip','install','--only-binary=:all:','-r',str(root/'scripts/requirements-piper-modern.txt')],check=True)
    if not python.is_file():raise SystemExit('Optional runtime missing. Run this installer with --download and a Python supporting venv/pip (tested: 3.12).')
    subprocess.run([str(python),'-c',"import importlib.metadata as m; assert m.version('piper-tts') == '1.8.0'; import piper,pyopenjtalk"],check=True)
    voice='models/piper/'+PACKS[args.language]+'.onnx';destination=root/f'offline.{args.language}.json'
    if destination.exists():
        config=json.loads(destination.read_text(encoding='utf-8-sig'))
        if config.get('voices',{}).get(args.language)!=voice:raise SystemExit('Existing voice selection differs; preserving it. Configure this optional pack explicitly.')
        existing=config.get('piper_python_voices',{}).get(args.language)
        if existing and (root/existing).resolve()!=python.resolve():raise SystemExit('Existing runtime selection differs; preserving it.')
    else:
        config=json.loads((root/'offline.cuda.json').read_text(encoding='utf-8-sig'))
        config.setdefault('voices',{})[args.language]=voice
    config.setdefault('piper_python_voices',{})[args.language]='engines/piper-modern/venv/Scripts/python.exe'
    data=json.dumps(config,indent=2)
    if not destination.exists() or destination.read_text(encoding='utf-8-sig')!=data:
        with destination.open('w',encoding='utf-8') as output:output.write(data)
    print('Optional voice and Piper 1.8 verified. Existing models/engine preferences and native Piper are preserved.')
    print('Native-speaker review remains pending. Persistent voice workers are enabled; benchmark sustained operation for the selected combination.')
if __name__=='__main__':main()
