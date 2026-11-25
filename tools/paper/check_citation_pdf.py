"""Bind every registered screen citation to printed text and a bibliography destination; printed link targets are not inspected."""
import html
import pathlib
import re
import sys
import tempfile

from common import require
from pdf_text import Pdf


REGISTERED = {
    ('paper-pilot/en.labels.md', 'a:andreoni', 'parenthetical'): 'warm-glow giving ',
    ('paper-pilot/en.labels.md', 'a:sencapability', 'parenthetical'): 'The flourishing pole is Sen’s capability approach ',
}
COMMENT = re.compile(r'<!-- bibcite ([^<>]+) -->')
LINK = r'<a href="#bibliography-[^"]+">([^<]+)</a>'


def inventory(labels_root):
    found = []
    for path in sorted(labels_root.rglob('*.labels.md')):
        source = path.read_text(encoding='utf-8')
        matches = list(COMMENT.finditer(source))
        require(len(matches) == source.count('<!-- bibcite '), 'Malformed bibliography citation comment')
        for match in matches:
            before = source[source.rfind('\n', 0, match.start()) + 1:match.start()]
            parenthetical = re.search(r'\(' + LINK + r'\)$', before)
            textual = re.search(LINK + r'$', before) if parenthetical is None else None
            require(parenthetical is not None or textual is not None,
                    'Bibliography citation screen form differs')
            kind = 'parenthetical' if parenthetical is not None else 'textual'
            label = html.unescape((parenthetical or textual)[1])
            found.append((path.relative_to(labels_root).as_posix(), match[1], kind, label))
    return found


def require_registered(found):
    require(len(found) == len(REGISTERED), 'Converted citation inventory differs')
    for path, key, kind, _ in found:
        require((path, key, kind) in REGISTERED, 'Converted citation is not registered')


def check(root, pdf, labels_root=None):
    labels_root = labels_root or root / 'generated/documents'
    found = inventory(labels_root)
    require_registered(found)
    printed = Pdf(pdf)
    pages = printed.pages()
    for path, key, kind, label in found:
        registered = REGISTERED[(path, key, kind)]
        destination = printed.destination('cite.0@' + key)
        require(destination is not None, 'Citation bibliography destination is missing')
        occurrence = registered + ('(' + label + ')' if kind == 'parenthetical' else label)
        require(any(occurrence in page for page in pages[:destination - 1]),
                'Screen citation label differs from its printed occurrence')


def selftest():
    with tempfile.TemporaryDirectory() as temporary:
        labels = pathlib.Path(temporary)
        primary = labels / 'paper-pilot/en.labels.md'
        primary.parent.mkdir()
        primary.write_text('(<a href="#bibliography-andreoni-1990">Andreoni 1990</a>)'
                           '<!-- bibcite a:andreoni -->\n'
                           '(<a href="#bibliography-sen-1999">A. Sen 1999</a>)'
                           '<!-- bibcite a:sencapability -->\n', encoding='utf-8')
        require_registered(inventory(labels))
        secondary = labels / 'other/en.labels.md'
        secondary.parent.mkdir()
        for extra in ('(<a href="#bibliography-other-1991">Other 1991</a>)',
                      '<a href="#bibliography-other-1991">Other (1991)</a>'):
            secondary.write_text(extra + '<!-- bibcite a:other -->\n', encoding='utf-8')
            try:
                require_registered(inventory(labels))
            except ValueError as error:
                require(str(error) == 'Converted citation inventory differs', str(error))
            else:
                raise AssertionError('Additional citation escaped the inventory')


if __name__ == '__main__':
    if sys.argv[1:] == ['selftest']:
        selftest()
    else:
        require(len(sys.argv) in (3, 4), 'Expected repository, PDF and optional label root')
        root, pdf = map(pathlib.Path, sys.argv[1:3])
        labels_root = pathlib.Path(sys.argv[3]) if len(sys.argv) == 4 else None
        check(root, pdf, labels_root)
