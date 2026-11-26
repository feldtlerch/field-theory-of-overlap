"""Exercise the screen locator comparison against checked paper counter data."""
import pathlib
import shutil
import tempfile

import identities
import prepare
from ledger import auxiliary as paper_auxiliary, check_auxiliary_destinations, check_paragraph_tag_modes
from ledger import screen_references
from prepare import declared_fragments


def selftest():
    class Destination:
        def destination(self, anchor):
            return {'anchor.rep': 78}.get(anchor)

    check_auxiliary_destinations(('bd:rep',),
                                 {'bd:rep': {'anchor': 'anchor.rep', 'page': '78'}}, Destination())
    for targets in ({'bd:rep': {'anchor': 'anchor.rep', 'page': '79'}}, {}):
        try:
            check_auxiliary_destinations(('bd:rep',), targets, Destination())
        except ValueError as error:
            assert 'auxiliary/PDF page discrepancy' in str(error) or 'unresolved auxiliary target' in str(error), error
        else:
            raise AssertionError('auxiliary destination refusal')
    try:
        check_auxiliary_destinations(('bd:rep',),
                                     {'bd:rep': {'anchor': 'missing', 'page': '78'}}, Destination())
    except ValueError as error:
        assert 'missing PDF destination' in str(error), error
    else:
        raise AssertionError('missing auxiliary PDF destination')
    auxiliary = {
        'app:self-limit': {'default': 'G', 'page': '106', 'anchor': 'appendix.G'},
        'sec:front': {'default': '0', 'page': '1', 'anchor': 'section.0'},
        'subsec:part-e': {'default': '17.0', 'page': '80', 'anchor': 'subsection.17.0'},
        'subsec:coda:tenses': {'default': '22.1', 'page': '104', 'anchor': 'subsection.22.1'},
    }
    screen = '<a href="../paper-pilot/en.md#method-self-limit">→ appendix G</a><!-- see app:self-limit -->'
    assert len(screen_references(screen, auxiliary)) == 1
    for label, number in [('sec:front', '0'), ('subsec:part-e', '17.0'),
                          ('subsec:coda:tenses', '22.1')]:
        regular = f'<a href="../paper-pilot/en.md#target">→ §{number}</a><!-- see {label} -->'
        assert len(screen_references(regular, auxiliary)) == 1
        for changed in [regular.replace(f'§{number}', '§00'),
                        regular.replace(f'§{number}', '§17.00'),
                        regular.replace(f'§{number}', '§22.1.0'),
                        regular.replace(f'§{number}', '§')]:
            try:
                screen_references(changed, auxiliary)
            except ValueError as error:
                assert 'malformed' in str(error), error
            else:
                raise AssertionError(changed)
    for changed, reason in [
        (screen.replace('appendix G', 'appendix H'), 'number differs'),
        (screen.replace('app:self-limit', 'app:absent'), 'target absent'),
        (screen.replace('→', '=>'), 'malformed'),
    ]:
        try:
            screen_references(changed, auxiliary)
        except ValueError as error:
            assert reason in str(error), (reason, error)
        else:
            raise AssertionError(reason)

    with tempfile.TemporaryDirectory() as directory:
        root = pathlib.Path(directory)
        (root/'main.aux').write_text('\\zref@newlabel{bd:rep}{\\default{20.1}\\page{78}}\n', encoding='utf-8')
        assert paper_auxiliary(root, 'main', ('bd:rep',))['bd:rep']['default'] == '20.1'
        for changed, reason in [
            ('', 'Auxiliary definition count differs'),
            ('\\zref@newlabel{bd:rep}{}\n'*2, 'Auxiliary definition count differs'),
        ]:
            (root/'main.aux').write_text(changed, encoding='utf-8')
            try:
                paper_auxiliary(root, 'main', ('bd:rep',))
            except ValueError as error:
                assert reason in str(error), (reason, error)
            else:
                raise AssertionError(reason)

    with tempfile.TemporaryDirectory() as directory:
        root = pathlib.Path(directory)
        (root/'generated/documents').mkdir(parents=True)
        (root/'units/statements').mkdir(parents=True)
        (root/'generated/documents/inclusions.tsv').write_text(
            'documents/paper/parts/partF.tex\tsec:fixture\tgenerated/documents/fixture.tex\tparagraph\t315\n',
            encoding='utf-8')
        (root/'units/statements/fixture.toml').write_text(
            '[unit]\nid = "sec:fixture"\n[unit.texts]\nen = "@tag{bd:rep} is a boundary."\n',
            encoding='utf-8')
        fragment = root/'generated/documents/fixture.tex'
        fragment.write_text('\\PaperNativeParagraph{sec:fixture}{\\ftag{bd:rep} is a boundary.}\n',
                            encoding='utf-8')
        identity = dict(document='main', kind='identity', key='bd:rep',
                        site='parts/partF.tex:315')
        check_paragraph_tag_modes(root, [identity])
        for rows, reason in [
            ([identity, dict(identity, kind='locator')], 'print mode differs'),
            ([], 'shipped association differs'),
            ([identity, identity], 'shipped association differs'),
        ]:
            try:
                check_paragraph_tag_modes(root, rows)
            except ValueError as error:
                assert reason in str(error), (reason, error)
            else:
                raise AssertionError(reason)
        fragment.write_text('\\PaperNativeParagraph{sec:fixture}{\\ftag{bd:fab} is a boundary.}\n',
                            encoding='utf-8')
        try:
            check_paragraph_tag_modes(root, [identity])
        except ValueError as error:
            assert 'command sequence differs' in str(error), error
        else:
            raise AssertionError('command sequence differs')

        (root/'units/statements/fixture.toml').write_text(
            '[unit]\nid = "sec:fixture"\n[unit.documentary-targets]\n'
            'representation = "bd:rep"\nfabrication = "bd:fab"\n'
            '[unit.texts]\nen = "@handle{representation} and @handle{fabrication}."\n',
            encoding='utf-8')
        fragment.write_text('\\PaperNativeParagraph{sec:fixture}{\\handle{bd:rep} and \\handle{bd:fab}.}\n',
                            encoding='utf-8')
        check_paragraph_tag_modes(root, [])
        fragment.write_text('\\PaperNativeParagraph{sec:fixture}{\\handle{bd:fab} and \\handle{bd:rep}.}\n',
                            encoding='utf-8')
        try:
            check_paragraph_tag_modes(root, [])
        except ValueError as error:
            assert 'handle command sequence differs' in str(error), error
        else:
            raise AssertionError('swapped handles')

    with tempfile.TemporaryDirectory() as directory:
        root = pathlib.Path(directory)
        (root/'generated/documents').mkdir(parents=True)
        (root/'units/statements').mkdir(parents=True)
        owner = root/'units/statements/andreoni-joint.toml'
        shutil.copyfile(pathlib.Path(__file__).resolve().parents[2]/
                        'units/statements/andreoni-joint.toml', owner)
        (root/'generated/documents/fixture.tex').write_text('\\ftag{J:occ}\n', encoding='utf-8')
        inclusions = root/'generated/documents/inclusions.tsv'
        for marker in (424, 425):
            inclusions.write_text(
                f'documents/paper/parts/appendices.tex\tsec:log:andreoni-joint\t'
                f'generated/documents/fixture.tex\tparagraph\t{marker}\n', encoding='utf-8')
            rows = [dict(document='main', kind=kind, key='J:occ',
                         site=f'parts/appendices.tex:{marker}') for kind in ('identity', 'locator')]
            check_paragraph_tag_modes(root, rows)
            try:
                check_paragraph_tag_modes(root, rows[:1])
            except ValueError as error:
                assert 'print mode differs' in str(error), error
            else:
                raise AssertionError('located mode flip')
        original = owner.read_bytes()
        owner.write_bytes(original.replace(b'@tag{J:occ}', b'@tag{J:occ} @tag{J:occ}'))
        try:
            check_paragraph_tag_modes(root, rows)
        except ValueError as error:
            assert 'tag command sequence differs' in str(error), error
        else:
            raise AssertionError('copied located occurrence')
        owner.write_bytes(original)
        owner.write_bytes(owner.read_bytes().replace(b'joint no antecedent', b'joint no premise'))
        try:
            check_paragraph_tag_modes(root, rows)
        except ValueError as error:
            assert 'Located paragraph tag owner changed' in str(error), error
        else:
            raise AssertionError('Located paragraph tag owner changed')

    with tempfile.TemporaryDirectory() as directory:
        root = pathlib.Path(directory)
        (root/'generated/documents').mkdir(parents=True)
        (root/'documents/paper/parts').mkdir(parents=True)
        source = 'documents/paper/parts/fixture.tex'
        fragment = root/'generated/documents/fixture.tex'
        fragment.write_text('\\PaperNativeParagraph{sec:fixture}{\\handle{bd:rep} is a boundary.}\n',
                            encoding='utf-8')
        (root/'generated/documents/inclusions.tsv').write_text(
            source+'\tsec:fixture\tgenerated/documents/fixture.tex\tparagraph\t1\n',
            encoding='utf-8')
        original_bindings = identities.bindings
        original_prepare_bindings = prepare.bindings
        original_assembly_json = prepare.assembly_json
        identities.bindings = lambda unused: {
            'bd:rep': {'source_status': 'paper', 'spelling': 'bd:rep'}}
        prepare.bindings = identities.bindings
        prepare.assembly_json = lambda unused: '{"native_targets":[]}'
        try:
            for suffix, reason in [
                ('\\ftag{bd:rep}\n', None),
                ('\\ptg{bd:rep}\n', None),
                ('\\hdr{bd:rep}\n', 'Retained authored auxiliary target'),
                ('\\ftoolabel{bd:rep}\n', 'Retained authored auxiliary target'),
            ]:
                (root/source).write_text('% generated: sec:fixture\n'+suffix, encoding='utf-8')
                if reason is None:
                    declared_fragments(root, (source,))
                else:
                    try:
                        declared_fragments(root, (source,))
                    except ValueError as error:
                        assert reason in str(error), (reason, error)
                    else:
                        raise AssertionError(reason)
            fragment.write_text('\\PaperNativeParagraph{sec:fixture}{\\handle{bd:fab} is a boundary.}\n',
                                encoding='utf-8')
            try:
                declared_fragments(root, (source,))
            except ValueError as error:
                assert 'Fragment handle binding differs' in str(error), error
            else:
                raise AssertionError('Fragment handle binding differs')
            fragment.write_text('\\PaperNativeParagraph{sec:fixture}{\\handle{bd:rep} is a boundary.}\n',
                                encoding='utf-8')
            other_source = 'documents/paper/parts/other.tex'
            (root/other_source).write_text('% generated: sec:other\n', encoding='utf-8')
            (root/'generated/documents/other.tex').write_text(
                '\\PaperNativeParagraph{sec:other}{\\handle{bd:rep} is another boundary.}\n',
                encoding='utf-8')
            (root/'generated/documents/inclusions.tsv').write_text(
                source+'\tsec:fixture\tgenerated/documents/fixture.tex\tparagraph\t1\n'
                +other_source+'\tsec:other\tgenerated/documents/other.tex\tparagraph\t1\n',
                encoding='utf-8')
            try:
                declared_fragments(root, (source, other_source))
            except ValueError as error:
                assert 'Duplicate auxiliary handle' in str(error), error
            else:
                raise AssertionError('Duplicate auxiliary handle')
        finally:
            identities.bindings = original_bindings
            prepare.bindings = original_prepare_bindings
            prepare.assembly_json = original_assembly_json


if __name__ == '__main__':
    selftest()
