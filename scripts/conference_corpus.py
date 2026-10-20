#!/usr/bin/env python3
"""Prepare a resumable local conference corpus; downloads require --download."""
import argparse
import atexit
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime, timezone
import hashlib
import gzip
from html.parser import HTMLParser
import json
from pathlib import Path
import re
import signal
import threading
import time
import urllib.error
import urllib.parse
import urllib.request

HOST = 'https://www.churchofjesuschrist.org'
LANGUAGES = {'en': 'eng', 'es': 'spa', 'de': 'deu'}
# Verified official video-only page, not a spoken talk transcript. Keep the gap explicit.
TEXT_EXCLUSIONS = {'/study/general-conference/2020/10/33video': 'Video presentation: no page transcript. Official source: https://www.churchofjesuschrist.org/study/general-conference/2020/10/33video?lang=eng'}
STOP = threading.Event()
VOID = {'area', 'base', 'br', 'col', 'embed', 'hr', 'img', 'input', 'link', 'meta', 'param', 'source', 'track', 'wbr'}


class ConferenceHTML(HTMLParser):
    def __init__(self):
        super().__init__()
        self.links, self.paragraphs = set(), {}
        self.lang, self.kind = None, None
        self.stack, self.body_depth, self.paragraph, self.parts = [], None, None, []
        self.skip_depth = None

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == 'a' and attrs.get('href'): self.links.add(attrs['href'])
        if attrs.get('data-content-type') and (self.kind is None or attrs.get('id') == 'main'):
            self.kind = attrs['data-content-type']
            self.lang = attrs.get('lang')
        if tag in VOID: return
        self.stack.append(tag)
        depth = len(self.stack)
        if 'body-block' in attrs.get('class', '').split(): self.body_depth = depth
        if self.body_depth is None: return
        if self.skip_depth is None and (tag in ('sup', 'aside', 'figure', 'script', 'style', 'button') or 'verse-number' in attrs.get('class','').split()):
            self.skip_depth = depth
        if tag == 'p' and re.fullmatch(r'p(?:\d+|_[A-Za-z0-9]+)', attrs.get('id', '')) and self.skip_depth is None:
            self.paragraph = (attrs['id'], depth)
            self.parts = []

    def handle_endtag(self, tag):
        if tag in VOID: return
        depth = len(self.stack)
        if self.paragraph and tag == 'p' and depth == self.paragraph[1]:
            ident = self.paragraph[0]
            text = ' '.join(''.join(self.parts).split())
            if ident in self.paragraphs: raise ValueError('Duplicate paragraph identifier')
            if text: self.paragraphs[ident] = text
            self.paragraph = None
        if depth == self.skip_depth: self.skip_depth = None
        if depth == self.body_depth: self.body_depth = None
        if self.stack: self.stack.pop()

    def handle_data(self, data):
        if self.paragraph and self.skip_depth is None: self.parts.append(data)


def parse_talk(html, expected_lang):
    parser = ConferenceHTML(); parser.feed(html)
    if parser.lang != expected_lang: raise ValueError(f'Language mismatch: expected {expected_lang}, got {parser.lang}')
    if parser.kind not in ('general-conference-talk', 'general-conference-business'): raise ValueError(f'Not a talk: {parser.kind}')
    if not parser.paragraphs: raise ValueError('No body paragraphs; page layout may have changed')
    return parser.paragraphs


def talk_paths(html, year, month):
    parser = ConferenceHTML(); parser.feed(html)
    prefix = f'/study/general-conference/{year}/{month}/'
    paths = set()
    for link in parser.links:
        parsed = urllib.parse.urlparse(urllib.parse.urljoin(HOST, link))
        if parsed.netloc != 'www.churchofjesuschrist.org': continue
        path = parsed.path.rstrip('/')
        if not path.startswith(prefix): continue
        slug = path[len(prefix):]
        if not re.fullmatch(r'[a-zA-Z0-9-]+', slug): continue
        if slug.endswith('-session') or slug.startswith('session-'): continue
        paths.add(path)
    return sorted(paths)


def atomic_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix('.tmp')
    temporary.write_text(json.dumps(value, ensure_ascii=False), encoding='utf-8')
    temporary.replace(path)


def fetch(url):
    if STOP.is_set(): raise InterruptedError('Collection interrupted')
    request = urllib.request.Request(url, headers={'User-Agent': 'OfflineReligiousTranslationEvaluation/0.5', 'Accept-Encoding':'gzip'})
    for attempt in range(2):
        if STOP.is_set():raise InterruptedError('Collection interrupted')
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                final = urllib.parse.urlparse(response.url)
                if final.scheme != 'https' or final.netloc != 'www.churchofjesuschrist.org': raise ValueError('Unexpected redirect host')
                body = response.read(4 * 1024 * 1024 + 1)
                if len(body) > 4 * 1024 * 1024: raise ValueError('Page exceeds 4 MiB')
                if response.headers.get('Content-Encoding') == 'gzip': body = gzip.decompress(body)
                if len(body) > 4 * 1024 * 1024: raise ValueError('Expanded page exceeds 4 MiB')
                return body.decode('utf-8')
        except (OSError, ValueError):
            if attempt: raise
            time.sleep(1)


def validate_path(path, year=None, month=None):
    if not isinstance(path,str) or not re.fullmatch(r'/study/general-conference/\d{4}/(?:04|10)/[A-Za-z0-9-]+',path):
        raise ValueError('Unsafe conference path')
    if year is not None and not path.startswith(f'/study/general-conference/{year}/{month}/'):
        raise ValueError('Cached index path does not match its conference')


def load_index(root, year, month, download):
    if STOP.is_set(): raise InterruptedError('Collection interrupted')
    cache = root / 'indexes' / f'{year}-{month}.json'
    if cache.exists():
        record=json.loads(cache.read_text(encoding='utf-8'))
        if not record.get('paths'):raise ValueError('Empty cached index')
        for path in record['paths']:validate_path(path,year,month)
        return record
    if not download: raise ValueError('Index is not cached; use --download during online preparation')
    url = f'{HOST}/study/general-conference/{year}/{month}?lang=eng'
    paths = talk_paths(fetch(url), year, month)
    if not paths: raise ValueError('No talk links found')
    record = dict(url=url, paths=paths, retrieved_utc=datetime.now(timezone.utc).isoformat())
    atomic_json(cache, record)
    return record


def load_talk(root, path, language, download):
    if STOP.is_set(): raise InterruptedError('Collection interrupted')
    validate_path(path)
    relative = path.removeprefix('/study/general-conference/')
    cache = root / 'talks' / (relative + f'-{language}.json')
    if cache.exists():
        record = json.loads(cache.read_text(encoding='utf-8'))
        if record['language'] != language or record['path'] != path: raise ValueError('Cached talk identity mismatch')
        payload = json.dumps(record['paragraphs'], ensure_ascii=False, sort_keys=True).encode()
        if hashlib.sha256(payload).hexdigest() != record['sha256']: raise ValueError('Cached talk checksum mismatch')
        return record
    if not download: raise ValueError('Talk is not cached; use --download during online preparation')
    url = f'{HOST}{path}?lang={LANGUAGES[language]}'
    paragraphs = parse_talk(fetch(url), LANGUAGES[language])
    digest = hashlib.sha256(json.dumps(paragraphs, ensure_ascii=False, sort_keys=True).encode()).hexdigest()
    record = dict(path=path, language=language, url=url, paragraphs=paragraphs, sha256=digest,
                  retrieved_utc=datetime.now(timezone.utc).isoformat())
    atomic_json(cache, record)
    return record


def aligned_records(path, documents):
    year, month, slug = path.removeprefix('/study/general-conference/').split('/')
    paragraphs = [set(documents[lang]['paragraphs']) for lang in LANGUAGES]
    shared = set.intersection(*paragraphs)
    # Shared publisher IDs are alignment candidates, not proof of semantic equivalence.
    for ident in documents['en']['paragraphs']:
        if ident not in shared: continue
        texts = {lang: documents[lang]['paragraphs'][ident] for lang in LANGUAGES}
        if any(len(text) > 2000 or len(text) < 15 for text in texts.values()): continue
        if len(set(texts.values())) != 3: continue
        yield dict(id=f'gc-{year}-{month}-{hashlib.sha256(slug.encode()).hexdigest()[:12]}-{ident}',
                   year=int(year), month=month, talk_path=path, paragraph=ident,
                   split='train' if int(year) <= 2023 else ('dev' if int(year) == 2024 else 'test'),
                   alignment='shared_publisher_id_unreviewed', texts=texts,
                   urls={lang: documents[lang]['url'] + '#' + ident for lang in LANGUAGES},
                   document_hashes={lang: documents[lang]['sha256'] for lang in LANGUAGES})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--start-year', type=int, default=2017)
    parser.add_argument('--end-year', type=int, default=2026)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--download', action='store_true')
    parser.add_argument('--workers', type=int, default=4)
    args = parser.parse_args()
    if not 1971 <= args.start_year <= args.end_year <= datetime.now(timezone.utc).year: parser.error('Invalid year range')
    if not 1 <= args.workers <= 8: parser.error('Use 1–8 workers')
    signal.signal(signal.SIGINT, lambda *_: STOP.set())
    root = args.output.resolve(); root.mkdir(parents=True, exist_ok=True)
    lock=root/'.collection.lock'
    try:
        with lock.open('x',encoding='utf-8') as stream:stream.write(datetime.now(timezone.utc).isoformat())
    except FileExistsError:parser.error('This corpus directory is locked by another collection; finish/stop it first. Remove a stale lock only after the old collector has ended.')
    atexit.register(lambda:lock.unlink(missing_ok=True))
    coverage = dict(schema_version=1, start_year=args.start_year, end_year=args.end_year,
                    generated_utc=datetime.now(timezone.utc).isoformat(), conferences=[], failures=[],
                    text_inference_cases_completed=0, audio_cases_completed=0)
    indexes = []
    with ThreadPoolExecutor(max_workers=args.workers) as pool:
        futures = {pool.submit(load_index, root, year, month, args.download):(year,month)
                   for year in range(args.start_year,args.end_year+1) for month in ('04','10')}
        for future in as_completed(futures):
            year,month = futures[future]
            try:
                index = future.result(); indexes.append((year,month,index))
                print(f'Index {year}-{month}: {len(index["paths"])} entries', flush=True)
            except Exception as error: coverage['failures'].append(dict(stage='index', year=year,month=month,error=str(error)))
    excluded=[dict(path=path,reason=reason) for path,reason in TEXT_EXCLUSIONS.items() if any(path in index['paths'] for _,_,index in indexes)]
    coverage['excluded_entries']=excluded
    documents = {}; total = sum(sum(path not in TEXT_EXCLUSIONS for path in index['paths'])*3 for _,_,index in indexes)
    with ThreadPoolExecutor(max_workers=args.workers) as pool:
        futures = {pool.submit(load_talk,root,path,lang,args.download):(path,lang)
                   for _,_,index in sorted(indexes, reverse=True) for path in index['paths'] if path not in TEXT_EXCLUSIONS for lang in LANGUAGES}
        for count,future in enumerate(as_completed(futures),1):
            path,lang = futures[future]
            try: documents.setdefault(path,{})[lang] = future.result()
            except Exception as error: coverage['failures'].append(dict(stage='talk',path=path,language=lang,error=str(error)))
            if count % 100 == 0 or count == total:
                print(f'Pages {count}/{total}; failures {len(coverage["failures"])}',flush=True)
                atomic_json(root/'coverage.partial.json',coverage | {'pages_processed':count,'pages_expected':total})
    temporary = root/'aligned.tmp'
    record_count = 0
    with temporary.open('w',encoding='utf-8') as output:
        for year,month,index in sorted(indexes):
            aligned_talks = 0; paragraphs = 0
            for path in index['paths']:
                if set(documents.get(path,{})) != set(LANGUAGES): continue
                records = list(aligned_records(path,documents[path]))
                if records: aligned_talks += 1
                for record in records: output.write(json.dumps(record,ensure_ascii=False)+'\n')
                paragraphs += len(records)
            coverage['conferences'].append(dict(year=year,month=month,listed_entries=len(index['paths']),
                                                trilingual_talks_with_candidates=aligned_talks,candidate_paragraphs=paragraphs))
            record_count += paragraphs
    temporary.replace(root/('aligned.partial.jsonl' if STOP.is_set() else 'aligned.jsonl'))
    coverage['candidate_paragraphs'] = record_count
    coverage['pages_expected'] = total
    coverage['pages_cached'] = sum(len(doc) for doc in documents.values())
    coverage['interrupted'] = STOP.is_set()
    coverage['complete_download'] = not coverage['failures'] and not STOP.is_set()
    coverage_file=root/('coverage.interrupted.json' if STOP.is_set() else 'coverage.json')
    atomic_json(coverage_file,coverage)
    print(f'Prepared {record_count} candidate paragraphs. Coverage: {coverage_file}',flush=True)
    if not coverage['complete_download']: raise SystemExit(1)


if __name__ == '__main__': main()
