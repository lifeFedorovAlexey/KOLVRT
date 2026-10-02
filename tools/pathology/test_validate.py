"""Mutation tests: the validator must reject realistic research-data defects."""
from copy import deepcopy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from validate import ROOT, read_json, validate_database

SCHEMA = ROOT / 'schemas/pathology/case.schema.json'


class ValidatorTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.directory = Path(self.tmp.name)
        self.case = read_json(ROOT / 'research/pathology/KOL-PATH-0001.json')

    def save(self, case=None, filename='KOL-PATH-0001.json'):
        target = self.directory / filename
        target.write_text(json.dumps(self.case if case is None else case, ensure_ascii=False), encoding='utf-8')
        return target

    def errors(self):
        return validate_database(self.directory, SCHEMA, 1)

    def test_complete_repository(self):
        self.assertEqual(validate_database(ROOT / 'research/pathology', SCHEMA, 30), [])

    def test_valid_record(self):
        self.save()
        self.assertEqual(self.errors(), [])

    def test_every_required_field(self):
        for field in read_json(SCHEMA)['required']:
            with self.subTest(field=field):
                candidate = deepcopy(self.case)
                del candidate[field]
                self.save(candidate)
                self.assertTrue(self.errors())

    def test_empty_whitespace_and_wrong_types(self):
        for field, values in {
            'kolvrt_native_semantics': ['', '   ', 17, None],
            'sources': [[], {}, None],
            'tests_required': [[], [''], ['   '], 'test'],
            'benchmarks_required': [[], [''], ['   '], False],
        }.items():
            for value in values:
                with self.subTest(field=field, value=value):
                    candidate = deepcopy(self.case)
                    candidate[field] = value
                    self.save(candidate)
                    self.assertTrue(self.errors())

    def test_unknown_fields_and_enums(self):
        for field, value in [('kolvrt_decision', 'LOOKS_GOOD'), ('category', ['BUG']),
                             ('compatibility_required', True), ('confidence', 0.9), ('typo', 'value')]:
            with self.subTest(field=field):
                candidate = deepcopy(self.case)
                candidate[field] = value
                self.save(candidate)
                self.assertTrue(self.errors())

    def test_duplicate_id_and_filename(self):
        self.save()
        self.save(filename='KOL-PATH-0002.json')
        joined = '\n'.join(self.errors())
        self.assertIn('duplicate ID', joined)
        self.assertIn('filename', joined)

    def test_duplicate_json_key(self):
        path = self.save()
        path.write_text('{"id":"KOL-PATH-0001","id":"KOL-PATH-0002"}', encoding='utf-8')
        self.assertIn('duplicate JSON key', '\n'.join(self.errors()))

    def test_malformed_json(self):
        self.save().write_text('{broken', encoding='utf-8')
        self.assertTrue(self.errors())

    def test_nonfinite_json(self):
        self.save().write_text('{"value":NaN}', encoding='utf-8')
        self.assertIn('non-finite', '\n'.join(self.errors()))

    def test_evidence_must_resolve(self):
        self.case['evidence']['root_cause'] = ['S99999']
        self.save()
        self.assertIn('evidence', '\n'.join(self.errors()))

    def test_duplicate_source_id(self):
        self.case['sources'].append(deepcopy(self.case['sources'][0]))
        self.save()
        self.assertIn('duplicate source', '\n'.join(self.errors()))

    def test_primary_source_required(self):
        self.case['sources'][0]['kind'] = 'SECONDARY_ANALYSIS'
        self.save()
        self.assertIn('primary source', '\n'.join(self.errors()))

    def test_bad_url_date_and_commit(self):
        for field, value in [('url', 'file:///tmp/paper'), ('date', '2026-02-30'),
                             ('date', '2099-01-01'), ('commit', 'not-a-commit'),
                             ('research_date', '2026-09-30')]:
            with self.subTest(field=field, value=value):
                candidate = deepcopy(self.case)
                candidate['sources'][0][field] = value
                self.save(candidate)
                self.assertTrue(self.errors())

    def test_compat_decision_consistency(self):
        for decision, compat, scope in [('COMPAT_ONLY', 'NO', 'NONE'),
                                       ('NATIVE_FIX', 'NO', 'some-module'),
                                       ('COMPAT_ONLY', 'YES', 'NONE')]:
            with self.subTest(compat=compat, scope=scope):
                self.case.update(kolvrt_decision=decision, compatibility_required=compat, compatibility_scope=scope)
                self.save()
                self.assertTrue(self.errors())

    def test_unresolved_needs_questions(self):
        self.case.update(kolvrt_decision='RESEARCH_REQUIRED', open_questions=[])
        self.save()
        self.assertIn('open questions', '\n'.join(self.errors()))

    def test_unknown_provenance_needs_explanation(self):
        self.case['sources'][0]['provenance_limitations'] = ''
        self.save()
        self.assertTrue(self.errors())

    def test_empty_and_missing_directory(self):
        self.assertTrue(self.errors())
        self.assertTrue(validate_database(self.directory / 'absent', SCHEMA, 1))

    def test_cli_failure_is_nonzero(self):
        result = subprocess.run([sys.executable, str(ROOT / 'tools/pathology/validate.py'),
                                 '--directory', str(self.directory)], capture_output=True, text=True)
        self.assertEqual(result.returncode, 1)
        self.assertIn('found 0', result.stderr)


if __name__ == '__main__':
    unittest.main()
