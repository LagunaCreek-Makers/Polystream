"""Install and verify the optional French pack without replacing user settings."""
import argparse,json,subprocess,sys
from pathlib import Path

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--download',action='store_true')
    args=parser.parse_args()
    root=Path(__file__).resolve().parents[1]
    subprocess.run([sys.executable,str(root/'scripts/fetch_models.py'),'--download' if args.download else '--verify','--manifest',str(root/'models-french.json')],check=True)
    voice='models/piper/fr_FR-siwis-medium.onnx'
    destination=root/'offline.french.json'
    if destination.exists():
        config=json.loads(destination.read_text(encoding='utf-8-sig'))
        if config.get('voices',{}).get('fr')!=voice:
            raise SystemExit('Existing French configuration differs; preserving it. Add the installed French voice explicitly.')
    else:
        config=json.loads((root/'offline.cuda.json').read_text(encoding='utf-8-sig'))
        config.setdefault('voices',{})['fr']=voice
        with destination.open('x',encoding='utf-8') as output:
            json.dump(config,output,indent=2)
    print('French pack verified. Select French in PolyStream; existing engine settings are preserved.')
    print('SIWIS dataset: CC BY 4.0. Attribution retained in the installed MODEL_CARD.')
if __name__=='__main__':main()
