"""Actual generator entry points in isolated toy fixtures; no old source pixels."""
import hashlib,json,os,shutil,subprocess,sys,tempfile,tomllib,unittest
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
class IntegrationTests(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup);self.r=Path(self.tmp.name)
  art=self.r/'tools/art';art.mkdir(parents=True);(art/'fallback').mkdir()
  for name in ['replacement_policy.py','items.py','items_derived.py','derive.py','provenance.py','pal.py','draw.py']:
   if (ROOT/'tools/art'/name).exists():
    shutil.copyfile(ROOT/'tools/art'/name,art/name)
  rows=json.loads((ROOT/'tools/art/fallback/manifest.json').read_text());self.paths=[r['path'] for r in rows]
  for row in rows:
   p=self.r/row['path'];p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(b'toy replacement bytes')
   row['sha256']=hashlib.sha256(p.read_bytes()).hexdigest();row['source']='synthetic-toy'
  self.m=art/'fallback/manifest.json';self.m.write_text(json.dumps(rows));digest=hashlib.sha256(self.m.read_bytes()).hexdigest()
  p=art/'replacement_policy.py';s=p.read_text().replace('bb100781cc098863bfadfdce11ee592831e6a81b1d75b95583c355249b822c5c',digest);p.write_text(s)
  palette=self.r/'godot/assets/palette';palette.mkdir();shutil.copyfile(ROOT/'godot/assets/palette/ostrog.json',palette/'ostrog.json')
  (art/'sources').mkdir();(art/'sources/derived_assets.json').write_text(json.dumps({p:{'generator':'retired','licence':'old claim'} for p in self.paths}))
  self.out=art/'provenance.toml';self.out.write_text('old table sentinel')
 def run_script(self,name,ok=True):
  result=subprocess.run([sys.executable,str(self.r/'tools/art'/name)],capture_output=True,env={**os.environ,'PYTHONDONTWRITEBYTECODE':'1'})
  self.assertEqual(result.returncode==0,ok,result.stderr.decode());return result
 def test_items_preserve_protected_and_generate_unrelated(self):
  self.run_script('items.py')
  self.assertTrue((self.r/'godot/assets/items/log.png').exists())
  self.assertTrue(all((self.r/p).read_bytes()==b'toy replacement bytes' for p in self.paths))
 def test_eight_large_defaults_match_text_templates_and_provenance(self):
  result=subprocess.run([sys.executable,str(self.r/'tools/art/items.py'),'--big'],capture_output=True,env={**os.environ,'PYTHONDONTWRITEBYTECODE':'1'})
  self.assertEqual(result.returncode,0,result.stderr.decode())
  self.run_script('provenance.py')
  rows={r['path']:r for r in tomllib.loads(self.out.read_text())['asset']}
  for name in ('bed_frame','chest','door','dye','rough_stone','stool','table','thatch'):
   path='godot/assets/items32/'+name+'.png'
   self.assertEqual(hashlib.sha256((self.r/path).read_bytes()).digest(),hashlib.sha256((ROOT/path).read_bytes()).digest())
   self.assertEqual(rows[path]['generator'],'tools/art/items.py')
   self.assertEqual(rows[path]['source'],'generated')
 def test_missing_policy_fails_before_write(self):
  self.m.rename(self.m.with_suffix('.parked'))
  for script in ['items.py','provenance.py','items_derived.py','derive.py']:
   if (self.r/'tools/art'/script).exists(): self.run_script(script,False)
  self.assertEqual(self.out.read_text(),'old table sentinel')
  self.assertFalse((self.r/'godot/assets/items/log.png').exists())
 def test_changed_policy_fails_before_write(self):
  rows=json.loads(self.m.read_text());self.m.write_text(json.dumps(rows[:-1]));self.run_script('provenance.py',False)
  self.assertEqual(self.out.read_text(),'old table sentinel')
 def test_changed_pixels_fail_before_write(self):
  (self.r/self.paths[0]).write_bytes(b'old synthetic pixels');self.run_script('items.py',False)
  self.assertFalse((self.r/'godot/assets/items/log.png').exists())
 def test_sheet_entry_points_stop_without_inputs(self):
  if not (self.r/'tools/art/items_derived.py').exists():
   self.assertFalse((ROOT/'tools/art/derive.py').exists())
   self.skipTest('retired entry points excluded from public rehearsal')
  self.run_script('items_derived.py',False);self.run_script('derive.py',False)
 def test_provenance_overrides_old_claims(self):
  self.run_script('provenance.py');s=self.out.read_text()
  self.assertNotIn('old claim',s);self.assertNotIn('retired',s);self.assertEqual(s.count('rights_status = "unreviewed"'),113)
 def test_restored_attribution_rejected(self):
  self.run_script('provenance.py')
  (self.r/'tools/art/provenance_people.toml').write_text('')
  self.run_script('replacement_policy.py')
  self.out.write_text(self.out.read_text().replace('unresolved; rehearsal only','old synthetic grant'))
  self.run_script('replacement_policy.py',False)
 def test_refresh_protected_folder_leaves_table(self):
  code='import provenance; provenance.refresh_folder("items", lambda _: "toy")'
  result=subprocess.run([sys.executable,'-c',code],cwd=self.r/'tools/art',capture_output=True,env={**os.environ,'PYTHONDONTWRITEBYTECODE':'1'})
  self.assertNotEqual(result.returncode,0);self.assertEqual(self.out.read_text(),'old table sentinel')
if __name__=='__main__':unittest.main()
