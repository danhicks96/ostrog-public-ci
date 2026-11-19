"""Required policy for this fallback rehearsal, not asset/rights approval.
FIXLOG 2026-11-19 fallback integration: missing policy must never enable legacy writes.
"""
import hashlib,json,os,io,tomllib
from contextlib import contextmanager
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
MANIFEST=ROOT/'tools/art/fallback/manifest.json'
DIGEST='bb100781cc098863bfadfdce11ee592831e6a81b1d75b95583c355249b822c5c'

def safe_file(path):
 p=Path(path)
 if p.is_absolute() or '..' in p.parts or '\\' in path or ':' in path:raise ValueError('unsafe replacement path')
 full=ROOT/p
 if any(x.is_symlink() for x in [full,*full.parents]):raise ValueError('symlink replacement path')
 return full

def load():
 data=MANIFEST.read_bytes()
 if hashlib.sha256(data).hexdigest()!=DIGEST:raise ValueError('missing or changed required replacement policy')
 rows=json.loads(data)
 if len(rows)!=113 or len({r['path'] for r in rows})!=113:raise ValueError('incomplete replacement policy')
 return {r['path']:r for r in rows}

def audit():
 rows=load()
 for path,row in rows.items():
  if hashlib.sha256(safe_file(path).read_bytes()).hexdigest()!=row['sha256']:raise ValueError('replacement asset drift')
 return rows

def reject_legacy():
 load()
 raise ValueError('retired sheet pipeline disabled in fallback rehearsal')

def provenance(row):
 return {'path':row['path'],'kind':Path(row['path']).parent.name,'source':'generated',
 'generator':'tools/art/fallback/generate.py','licence':'unresolved; rehearsal only',
 'rights_status':'unreviewed','selection':'fallback-rehearsal-only','sha256':row['sha256'],
 'origin':row['source']}

def check_provenance():
 rows=audit();seen={}
 for table in ['tools/art/provenance.toml','tools/art/provenance_people.toml']:
  for entry in tomllib.loads((ROOT/table).read_text()).get('asset',[]):
   if entry['path'] in rows:
    if entry['path'] in seen or entry!=provenance(rows[entry['path']]):raise ValueError('replacement provenance drift')
    seen[entry['path']]=entry
 if set(seen)!=set(rows):raise ValueError('missing replacement provenance')

@contextmanager
def atomic_text(path):
 # Buffer until all row generation succeeds; retain prior table on exceptions.
 stream=io.StringIO();yield stream
 target=Path(path);tmp=target.with_suffix(target.suffix+'.replacement-tmp')
 tmp.write_text(stream.getvalue());os.replace(tmp,target)

if __name__=='__main__':
 check_provenance();print('fallback policy: 113 exact assets and provenance rows; unreviewed')
