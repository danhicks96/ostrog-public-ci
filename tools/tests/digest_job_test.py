"""Run actual workflow shell with a synthetic CLI; never calculate game digests."""
import importlib.util,io,os,shutil,subprocess,tempfile,textwrap,unittest
from pathlib import Path
from unittest import mock
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

SPEC=importlib.util.spec_from_file_location('ci_digests_under_test',ROOT/'tools/ci_digests.py')
COLLECTOR=importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(COLLECTOR)

class DigestOutput(unittest.TestCase):
 def collect(self,*,cli_newline='\n',stdout_newline='\n',changed=False):
  def fake_cli(command,**kwargs):
   seed=command[command.index('--seed')+1]
   value=('a' if seed=='3' else 'b')*64
   if changed and seed=='7' and command[1]=='run':value='c'*64
   text=value if command[1]=='digest' else 'normal CLI diagnostics'+cli_newline+'digest '+value
   return subprocess.CompletedProcess(command,0,stdout=text+cli_newline)
  raw=io.BytesIO()
  output=io.TextIOWrapper(raw,encoding='utf-8',newline=stdout_newline)
  try:
   with mock.patch.object(COLLECTOR.sys,'argv',['ci_digests.py','synthetic-cli']),mock.patch.object(COLLECTOR.sys,'stdout',output),mock.patch.object(COLLECTOR.subprocess,'run',side_effect=fake_cli):
    COLLECTOR.main()
   output.flush()
   return raw.getvalue()
  finally:output.detach()

 def test_lf_bytes_with_windows_stdout_and_crlf_cli(self):
  expected=''.join(f'seed {s} tick0 {v*64}\nseed {s} day3 digest {v*64}\n' for s,v in [('3','a'),('7','b')]).encode('ascii')
  actual=self.collect(cli_newline='\r\n',stdout_newline='\r\n')
  self.assertEqual(actual,expected)
  self.assertEqual(len(actual),324)
  self.assertEqual(actual.count(b'\n'),4)
  self.assertNotIn(b'\r',actual)

 def test_raw_diff_accepts_equal_records_and_rejects_changed_digest(self):
  diff=shutil.which('diff')
  if diff is None:self.skipTest('raw diff executable not available')
  with tempfile.TemporaryDirectory() as tmp:
   left=Path(tmp)/'linux.txt';right=Path(tmp)/'windows.txt'
   left.write_bytes(self.collect())
   right.write_bytes(self.collect(cli_newline='\r\n',stdout_newline='\r\n'))
   equal=subprocess.run([diff,str(left),str(right)],capture_output=True)
   self.assertEqual(equal.returncode,0,equal.stdout+equal.stderr)
   right.write_bytes(self.collect(cli_newline='\r\n',stdout_newline='\r\n',changed=True))
   changed=subprocess.run([diff,str(left),str(right)],capture_output=True)
   self.assertEqual(changed.returncode,1,changed.stdout+changed.stderr)
   right.write_bytes(left.read_bytes().replace(b'\n',b'\r\n'))
   noncanonical=subprocess.run([diff,str(left),str(right)],capture_output=True)
   self.assertEqual(noncanonical.returncode,1,'comparison must remain byte-exact')


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
