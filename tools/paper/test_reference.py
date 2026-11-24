"""Exercise the screen locator comparison against checked paper counter data."""
from ledger import screen_references


def selftest():
    auxiliary = {'app:self-limit': {'default': 'G', 'page': '106', 'anchor': 'appendix.G'}}
    screen = '<a href="../paper-pilot/en.md#method-self-limit">→ appendix G</a><!-- see app:self-limit -->'
    assert len(screen_references(screen, auxiliary)) == 1
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
