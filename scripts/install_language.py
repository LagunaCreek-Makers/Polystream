"""Install pinned optional voices; preserve existing model and engine settings."""
import argparse,json,subprocess,sys
from pathlib import Path
PACKS={'hi':'hi_IN-pratham-medium','ru':'ru_RU-denis-medium','eu':'eu_ES-antton-medium','it':'it_IT-serena-medium'}
def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('language',choices=PACKS)
    parser.add_argument('--download',action='store_true')
    args=parser.parse_args();root=Path(__file__).resolve().parents[1]
    subprocess.run([sys.executable,str(root/'scripts/fetch_models.py'),'--download' if args.download else '--verify','--manifest',str(root/f'models-{args.language}.json')],check=True)
    if args.language=='hi': print('Hindi Pratham: upstream dataset CC BY-NC-SA 4.0. Keep attribution and license notices when sharing.')
    voice='models/piper/'+PACKS[args.language]+'.onnx'
    destination=root/f'offline.{args.language}.json'
    if destination.exists():
        config=json.loads(destination.read_text(encoding='utf-8-sig'))
        if config.get('voices',{}).get(args.language)!=voice:
            raise SystemExit('Existing optional configuration differs; preserving it. Configure the installed voice explicitly.')
    else:
        config=json.loads((root/'offline.cuda.json').read_text(encoding='utf-8-sig'))
        config.setdefault('voices',{})[args.language]=voice
        with destination.open('x',encoding='utf-8') as output:json.dump(config,output,indent=2)
    print('Pack verified. Select the language in PolyStream. Attribution is retained in MODEL_CARD.')
if __name__=='__main__':main()
