#!/usr/bin/env python3
"""Reject committed rendered media or unapproved binary data, not a private audit.
FIXLOG 2026-11-19: generated PNGs belong in ephemeral CI workspaces only.
"""
import hashlib
from pathlib import PurePosixPath
import subprocess
import sys

FIXTURES = {
 'crates/hf-sim/tests/fixtures/saves/v2.hfsave/world.bin': '19b4d97937f26c964fc7cf07fc7d45a03296b92b836c73ea46b4f8d3402905a9',
 'crates/hf-sim/tests/fixtures/saves/v2.hfsave/chronicle.sqlite': 'ee9485781a5d51d8cb8b3ff2495780eb49b5f68cc10df6eaa3648ace78c613ae',
}
MEDIA = {'.png','.ogg','.jpg','.jpeg','.gif','.webp','.wav','.mp3','.mp4','.zip'}

def allowed(path, mode, data):
    if mode not in ('100644','100755') or PurePosixPath(path).suffix.lower() in MEDIA:
        return False
    if path in FIXTURES:
        return hashlib.sha256(data).hexdigest() == FIXTURES[path]
    try:
        data.decode('utf-8')
        return b'\0' not in data
    except UnicodeError:
        return False

def main():
    listing=subprocess.check_output(['git','--no-replace-objects','ls-tree','-rz','HEAD'])
    p=subprocess.Popen(['git','--no-replace-objects','cat-file','--batch'],stdin=subprocess.PIPE,stdout=subprocess.PIPE)
    seen=set();bad=0
    for rec in listing.split(b'\0'):
        if not rec:continue
        meta,name=rec.split(b'\t');mode,kind,oid=meta.decode().split();path=name.decode('utf-8')
        if kind!='blob':bad+=1;continue
        p.stdin.write((oid+'\n').encode());p.stdin.flush();header=p.stdout.readline().split()
        data=p.stdout.read(int(header[2]));assert p.stdout.read(1)==b'\n'
        bad+=not allowed(path,mode,data);seen.add(path)
    p.stdin.close();assert p.wait()==0
    if bad or not set(FIXTURES).issubset(seen):
        print('source-only: unapproved committed content or required fixture missing')
        return 1
    print('source-only: text/code plus two pinned synthetic binary fixtures; no committed media')
    return 0

if __name__=='__main__':
    try:sys.exit(main())
    except (OSError,ValueError,AssertionError,subprocess.SubprocessError):
        print('source-only: inspection failed');sys.exit(1)
