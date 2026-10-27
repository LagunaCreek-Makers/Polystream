"""Prime local CUDA kernels before the application's short request deadlines apply."""
import io
import json
import time
import urllib.request
import wave


def warmup(config, timeout=300):
    http = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    start = time.monotonic()
    payload = json.dumps({'model': 'local-translation', 'messages': [
        {'role': 'system', 'content': 'Translate English into Spanish. Return only the translation.'},
        {'role': 'user', 'content': 'Hello.'}], 'max_tokens': 8,
        'temperature': 0, 'stream': False, 'chat_template_kwargs': {'enable_thinking': False}}).encode()
    request = urllib.request.Request(f"http://127.0.0.1:{config['translation_port']}/v1/chat/completions", data=payload, headers={'Content-Type': 'application/json'})
    with http.open(request, timeout=timeout) as response:
        result = json.load(response)
        if not result.get('choices'):
            raise RuntimeError('Translation warm-up returned no choices')
    audio = io.BytesIO()
    with wave.open(audio, 'wb') as wav:
        wav.setparams((1, 2, 16000, 0, 'NONE', 'not compressed'))
        wav.writeframes(b'\0' * 16000)
    boundary = 'live-translate-local-warmup'
    data = (f'--{boundary}\r\nContent-Disposition: form-data; name="file"; filename="warmup.wav"\r\nContent-Type: audio/wav\r\n\r\n'.encode()
            + audio.getvalue()
            + f'\r\n--{boundary}\r\nContent-Disposition: form-data; name="language"\r\n\r\nen\r\n--{boundary}--\r\n'.encode())
    request = urllib.request.Request(f"http://127.0.0.1:{config['asr_port']}/inference", data=data, headers={'Content-Type': 'multipart/form-data; boundary=' + boundary})
    with http.open(request, timeout=timeout) as response:
        result = json.load(response)
        if 'text' not in result:
            raise RuntimeError('Recognition warm-up returned no text field')
    return time.monotonic() - start
