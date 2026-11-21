"""Regression fixtures for Rust string boundaries in the generated reference."""
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("reference", ROOT / "tools/docs/readme_tables.py")
REF = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(REF)


class RustReferenceStrings(unittest.TestCase):
    def test_escaped_quotes_and_continued_narration(self):
        src = r'''let label = "They called it \"a haven\".";
let message = "The gate stood open, \
               but the road was quiet.";
let later = "A light in the dark.";
'''
        self.assertEqual(list(REF.inventory_literals(src)), [
            (1, 'They called it "a haven".'),
            (2, "The gate stood open, but the road was quiet."),
            (4, "A light in the dark."),
        ])
        expr = r'''format!("{} walked back, \
                            past the gate.", person)'''
        self.assertEqual(REF.templates(expr), ["{person} walked back, past the gate."])

    def test_multiline_sql_never_quotes_the_code_after_it(self):
        src = r'''let query = "SELECT kind FROM real_history \
                 WHERE tick > ?1";
let item = rows.filter_map(|row| row.ok());
let tail = "{told} AND id NOT IN (\
                 SELECT history_id FROM entity_ref)";
let begin = "BEGIN; SELECT count(*) FROM real_history;";
let caption = "The valley remembers.";
let explanation = "Choose SELECT to show the records.";
'''
        self.assertEqual(list(REF.inventory_literals(src)), [
            (7, "The valley remembers."),
            (8, "Choose SELECT to show the records."),
        ])


if __name__ == "__main__":
    unittest.main()
