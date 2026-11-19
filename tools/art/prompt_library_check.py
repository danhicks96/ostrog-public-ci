#!/usr/bin/env python3
"""Check the successful-prompt library against source and derived-asset provenance."""
import hashlib
import json
import os
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
LIBRARY = os.path.join(ROOT, 'tools', 'art', 'sources', 'prompts.json')
DERIVED = os.path.join(ROOT, 'tools', 'art', 'sources', 'derived_assets.json')


def load_json(path):
    with open(path, encoding='utf-8') as fh:
        return json.load(fh)


def check(library_path=LIBRARY, derived_path=DERIVED):
    errors = []
    library = load_json(library_path)
    derived = load_json(derived_path)
    if library.get('version') != 1:
        errors.append('prompts.json: version must be 1')
    sheets = library.get('sheets')
    if not isinstance(sheets, dict):
        return ['prompts.json: sheets must be an object keyed by source sheet path']

    expected = {}
    for asset, row in sorted(derived.items()):
        sheet = row.get('sheet')
        if sheet not in sheets:
            errors.append('%s: derived sheet has no prompts.json row' % sheet)
            continue
        prompt = sheets[sheet]
        for field in ('model', 'service', 'prompt_file'):
            if prompt.get(field) != row.get(field):
                errors.append('%s: %s disagrees with %s' % (sheet, field, asset))
        source_hash = prompt.get('sheet_sha256', '')
        if not source_hash.startswith(row.get('sheet_sha256', '')):
            errors.append('%s: sheet hash disagrees with %s' % (sheet, asset))
        expected.setdefault(sheet, {}).setdefault(row.get('sprite'), []).append(asset)

    for sheet, row in sorted(sheets.items()):
        for field in ('prompt', 'model', 'service', 'date', 'prompt_file', 'sheet_sha256'):
            if not isinstance(row.get(field), str) or not row[field].strip():
                errors.append('%s: missing %s' % (sheet, field))
        source = os.path.join(ROOT, sheet)
        prompt_file = os.path.join(ROOT, row.get('prompt_file', ''))
        if not os.path.isfile(source) or not os.path.isfile(prompt_file):
            errors.append('%s: source sheet or prompt file is missing' % sheet)
            continue
        metadata = load_json(source + '.json')
        with open(prompt_file, encoding='utf-8') as fh:
            prompt_text = fh.read().strip()
        for field, actual in (('prompt', prompt_text), ('model', metadata.get('model')),
                              ('service', metadata.get('service')),
                              ('date', metadata.get('created'))):
            if row.get(field) != actual:
                errors.append('%s: %s disagrees with source metadata' % (sheet, field))
        if row.get('prompt') != metadata.get('prompt'):
            errors.append('%s: prompt disagrees with source metadata' % sheet)
        with open(source, 'rb') as fh:
            actual_hash = hashlib.sha256(fh.read()).hexdigest()
        if row.get('sheet_sha256') != actual_hash:
            errors.append('%s: sheet SHA-256 does not match source' % sheet)

        kept = row.get('kept_sprites')
        if not isinstance(kept, list) or not kept:
            errors.append('%s: kept_sprites must be a non-empty list' % sheet)
            continue
        declared = {}
        for sprite in kept:
            index = sprite.get('index')
            name = sprite.get('name')
            if not isinstance(index, int) or isinstance(index, bool) or not isinstance(name, str) or not name:
                errors.append('%s: invalid kept sprite %r' % (sheet, sprite))
                continue
            if index in declared:
                errors.append('%s: duplicate kept sprite %s' % (sheet, index))
            declared[index] = name
        wanted = set(expected.get(sheet, {}))
        if set(declared) != wanted:
            errors.append('%s: kept sprite indices %s, derived assets use %s' %
                          (sheet, sorted(declared), sorted(wanted)))
        for index, name in declared.items():
            assets = expected.get(sheet, {}).get(index, [])
            if name + '.png' not in [os.path.basename(asset) for asset in assets]:
                errors.append('%s sprite %s: kept name %s disagrees with %s' %
                              (sheet, index, name, ', '.join(assets) or 'no derived asset'))
    return errors


def main():
    errors = check()
    if errors:
        print('prompt library: %d error(s)' % len(errors))
        for error in errors[:25]:
            print('  ' + error)
        if len(errors) > 25:
            print('  ... and %d more' % (len(errors) - 25))
        return 1
    library = load_json(LIBRARY)
    derived = load_json(DERIVED)
    kept = sum(len(row['kept_sprites']) for row in library['sheets'].values())
    print('prompt library: %d sheets, %d kept sprites, %d derived assets covered' %
          (len(library['sheets']), kept, len(derived)))
    return 0


if __name__ == '__main__':
    sys.exit(main())
