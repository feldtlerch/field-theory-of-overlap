"""Exercise the screen locator comparison against checked paper counter data."""
from ledger import screen_references


def selftest():
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


if __name__ == '__main__':
    selftest()
