"""Byte I/O, explicit refusals and timing shared by paper preparation."""
import contextlib
import hashlib
import pathlib
import sys
import time
import tomllib

INPUTS = ['tools/paper/common.py']


def require(condition, message):
    if not condition:
        raise ValueError(message)


def read(path):
    return pathlib.Path(path).read_bytes().decode('utf-8')


def write(path, value):
    pathlib.Path(path).write_bytes(value.encode('utf-8'))


def digest(data):
    return hashlib.sha256(data).hexdigest()


@contextlib.contextmanager
def timed(name):
    start = time.monotonic()
    try:
        yield
    finally:
        print(f'{name}: wall={time.monotonic()-start:.6f}s', file=sys.stderr)


def runtime(repository, support, path='documents/paper-build/runtime.toml'):
    settings = tomllib.loads(read(repository/path))
    lock = tomllib.loads(read(repository/settings['support-lock']))
    engine = support/'runtime/bin/tectonic'
    require(digest(engine.read_bytes()) == lock['executable']['sha256'], 'Engine differs from support lock')
    return settings, lock, engine, support/'bundle'
