#!/usr/bin/env python3
"""The player's guide to Ostrog's files is the register, in plain words (WP-09 slice 9.12)."""

import html
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "privacy"))
import files_guide  # noqa: E402
import files_register  # noqa: E402


class FilesGuideTest(unittest.TestCase):
    def setUp(self):
        self.sections = files_guide.parse(files_guide.REGISTER.read_text(encoding="utf-8"))

    def test_the_committed_guide_and_rows_match_the_register(self):
        for path, data in files_guide.outputs().items():
            self.assertTrue(path.exists(), path)
            self.assertEqual(path.read_bytes(), data, f"{path} is stale: run files_guide.py")

    def test_every_name_players_can_find_is_in_the_guide_and_the_game_rows(self):
        txt = files_guide.TXT.read_text(encoding="utf-8")
        page = files_guide.PAGE.read_text(encoding="utf-8")
        rows = json.loads(files_guide.JSON_OUT.read_text(encoding="utf-8"))
        game_names = {n for s in rows["sections"] for r in s["rows"] for n in r["names"]}
        register = files_register.load_register()
        for place in ("install", "userdata", "temp"):
            for name in register[place]:
                if name in ("install_id", "reports/"):
                    continue  # the playtest rows, held back until slices 9.2/9.3
                self.assertIn(name, txt, f"{place}:{name} missing from READ ME FIRST")
                self.assertIn(name, game_names, f"{place}:{name} missing from the game's rows")
                self.assertIn(
                    f"<code>{html.escape(name)}</code>", page, f"{place}:{name} missing from the download page"
                )

    def test_the_guide_reads_as_plain_text(self):
        raw = files_guide.TXT.read_bytes()
        self.assertNotIn(b"\n", raw.replace(b"\r\n", b""), "CRLF throughout, for Notepad")
        text = raw.decode("utf-8")
        for markup in ("`", "**", "](", "(verify", "Open items"):
            self.assertNotIn(markup, text)
        self.assertTrue(all(len(l) <= 78 for l in text.split("\r\n")), "wrapped to 78 columns")

    def test_nothing_describes_what_the_game_cannot_do_yet(self):
        text = files_guide.TXT.read_text(encoding="utf-8")
        rows = files_guide.JSON_OUT.read_text(encoding="utf-8")
        if not files_guide.PLAYTEST_IN_GAME:
            for held in ("install_id", "Playtest", "playtest"):
                self.assertNotIn(held, text)
                self.assertNotIn(held, rows)
        page = files_guide.PAGE.read_text(encoding="utf-8")
        if not files_guide.PLAYTEST_IN_GAME:
            self.assertNotIn("install_id", page)
        if not files_guide.BUG_REPORT_IN_GAME:
            self.assertNotIn("Send bug report", text)
            self.assertNotIn("Send bug report", rows)
            self.assertNotIn("Send bug report", page)

    def test_check_mode_fails_on_a_changed_register(self):
        with tempfile.TemporaryDirectory() as d:
            changed = Path(d) / "register.md"
            changed.write_text(
                files_guide.REGISTER.read_text(encoding="utf-8").replace(
                    "Which alerts you chose to see or pause on.",
                    "Which alerts you chose.",
                ),
                encoding="utf-8",
            )
            out = files_guide.outputs(changed)
            self.assertNotEqual(out[files_guide.TXT], files_guide.TXT.read_bytes())

    def test_the_download_page_keeps_its_hand_written_parts_and_release_fields(self):
        sections = files_guide.parse(files_guide.REGISTER.read_text(encoding="utf-8"))
        page = files_guide.PAGE.read_text(encoding="utf-8")
        start = page.index(files_guide.PAGE_BEGIN)
        end = page.index(files_guide.PAGE_END) + len(files_guide.PAGE_END)
        # Re-rendering over an empty block gives the same page: only the block is generated.
        empty = page[:start] + files_guide.PAGE_BEGIN + "\n" + files_guide.PAGE_END + page[end:]
        self.assertEqual(files_guide.render_page(empty, sections), page)
        # The publish job's fields are outside the block and survive; the download stays first.
        for field in ("@VERSION@", "@TAG@", "@ZIP@", "@SHA256@"):
            self.assertIn(field, page[:start] + page[end:])
        self.assertLess(page.index('class="get"'), start, "the download button comes before the list")
        with self.assertRaises(ValueError):
            files_guide.render_page("<p>no markers</p>", sections)
        bad = [dict(s, rows=[dict(s["rows"][0], what="mail me@example")]) for s in sections[:1]]
        bad += sections[1:]
        with self.assertRaises(ValueError):
            files_guide.render_page(empty, bad)

    def test_a_malformed_row_is_refused(self):
        bad = "## The install folder\n| `x` | only two | cells |\n"
        with self.assertRaises(ValueError):
            files_guide.parse(bad)


if __name__ == "__main__":
    unittest.main()
