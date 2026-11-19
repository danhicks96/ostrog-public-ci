"""Run actual workflow shell with a synthetic CLI; never calculate game digests."""
import os,shutil,subprocess,tempfile,textwrap,unittest
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
WORKFLOWS=['tools/publish/public-ci.yml','.github/workflows/ci.yml']
FAKE='''#!/usr/bin/env python3
import os,sys
mode=os.environ.get('FAKE_DIGEST_MODE','clean');kind=sys.argv[1];seed=sys.argv[sys.argv.index('--seed')+1]
value=('a' if seed=='3' else 'b')*64
if mode=='failure' and seed=='7':sys.exit(7)
if kind=='digest':
 print(value if mode!='malformed_tick' else 'not-a-digest')
else:
 print('normal CLI diagnostics')
 if mode!='missing':print('digest '+(value if mode!='malformed_run' else 'c'*63))
 if mode=='duplicate':print('digest '+value)
 if mode=='run_failure':sys.exit(9)
'''
def shell(path):
 text=path.read_text().split('      - name: digests\n',1)[1]
 return textwrap.dedent(text.split('        run: |\n',1)[1].split('      - uses:',1)[0]).replace('${{ matrix.os }}','fixture')
@unittest.skipIf(os.name == 'nt', 'POSIX executable fixture; Linux workflow guard covers the shared collector')
class DigestJobs(unittest.TestCase):
 def run_job(self,workflow,mode):
  with tempfile.TemporaryDirectory() as tmp:
   r=Path(tmp);binary=r/'target/release/ostrog';binary.parent.mkdir(parents=True);binary.write_text(FAKE);binary.chmod(0o755)
   (r/'tools').mkdir()
   if (ROOT/'tools/ci_digests.py').exists():shutil.copyfile(ROOT/'tools/ci_digests.py',r/'tools/ci_digests.py')
   result=subprocess.run(['bash','-e','-o','pipefail','-c',shell(ROOT/workflow)],cwd=r,env={**os.environ,'FAKE_DIGEST_MODE':mode},capture_output=True,text=True)
   content=(r/'digests-fixture.txt').read_text() if (r/'digests-fixture.txt').exists() else ''
   return result,content
 def test_valid_values_unchanged(self):
  expected=''.join(f'seed {s} tick0 {v*64}\nseed {s} day3 digest {v*64}\n' for s,v in [('3','a'),('7','b')])
  for w in WORKFLOWS:
   if not (ROOT/w).exists():continue
   with self.subTest(workflow=w):
    result,content=self.run_job(w,'clean');self.assertEqual(result.returncode,0,result.stderr);self.assertEqual(content,expected)
 def test_failed_or_malformed_computations_fail_job(self):
  for w in WORKFLOWS:
   if not (ROOT/w).exists():continue
   for mode in ['failure','run_failure','malformed_tick','malformed_run','missing','duplicate']:
    with self.subTest(workflow=w,mode=mode):
     result,content=self.run_job(w,mode);self.assertNotEqual(result.returncode,0,'workflow falsely passed '+mode)
     self.assertNotIn('seed 7 day3',content)
if __name__=='__main__':unittest.main()
