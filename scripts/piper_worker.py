"""Local persistent Piper 1.8 worker. stdin JSON lines; stdout length-framed PCM16."""
import argparse,contextlib,json,struct,sys
from pathlib import Path

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--model',type=Path,required=True)
    parser.add_argument('--threads',type=int,default=4)
    args=parser.parse_args()
    if not 1<=args.threads<=128:raise ValueError('threads outside 1..128')
    output=sys.stdout.buffer
    with contextlib.redirect_stdout(sys.stderr):
        import onnxruntime
        from piper import PiperVoice
        from piper.config import PiperConfig
        settings=onnxruntime.SessionOptions();settings.intra_op_num_threads=args.threads;settings.inter_op_num_threads=1
        config=json.loads(Path(str(args.model)+'.json').read_text(encoding='utf-8'))
        voice=PiperVoice(config=PiperConfig.from_dict(config),session=onnxruntime.InferenceSession(str(args.model),sess_options=settings,providers=['CPUExecutionProvider']),download_dir=args.model.parent)
    for raw in sys.stdin.buffer:
        if len(raw)>65536:raise ValueError('Oversize speech request')
        request=json.loads(raw)
        text=request['text']
        if not isinstance(text,str) or not text.strip() or len(text)>8000:raise ValueError('Invalid speech request')
        total=0
        with contextlib.redirect_stdout(sys.stderr):
            for chunk in voice.synthesize(text):
                if chunk.sample_rate!=config['audio']['sample_rate'] or chunk.sample_width!=2 or chunk.sample_channels!=1:raise ValueError('Unexpected audio format')
                pcm=chunk.audio_int16_bytes;total+=len(pcm)
                if total>96000*2*120:raise ValueError('Oversize speech response')
                if pcm:output.write(struct.pack('<I',len(pcm)));output.write(pcm);output.flush()
        if not total:raise ValueError('Empty synthesis response')
        output.write(struct.pack('<I',0));output.flush()
if __name__=='__main__':main()
