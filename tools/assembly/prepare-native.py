#!/usr/bin/env python3
"""Prepare the two content-locked native inputs. Gates never call this program."""
import argparse
import hashlib
import os
from pathlib import Path
import platform
import re
import shutil
import stat
import tarfile
import tempfile
import tomllib
import urllib.parse
import urllib.request


def require(condition, evidence):
    if not condition:
        raise ValueError(evidence)


def fields(value, names):
    require(type(value) is dict and set(value) == set(names), ("fields", names, value))


def digest(value):
    require(type(value) is str and re.fullmatch(r"[0-9a-f]{64}", value), ("digest", value))
    return value


def member_name(value):
    require(type(value) is str and value and "\\" not in value
            and all(part not in ("", ".", "..") for part in value.split("/"))
            and not any(ord(c) < 32 or 127 <= ord(c) <= 159 for c in value), ("member", value))
    return value


def url(value):
    require(type(value) is str, ("url", value))
    parsed = urllib.parse.urlsplit(value)
    require(parsed.scheme == "https" and parsed.netloc and not parsed.username
            and not parsed.password and not parsed.fragment
            and parsed.hostname != "relay.fullyjustified.net", ("unpinned URL", value))
    return value


def forbid_floats(value):
    if isinstance(value, dict):
        for child in value.values():
            forbid_floats(child)
    elif isinstance(value, list):
        for child in value:
            forbid_floats(child)
    else:
        require(not isinstance(value, float), ("float", value))


def lock_preflight(text, maximum=262_144):
    """Stream resource counts before tomllib allocates the file inventory."""
    def tokens():
        at = 0
        while at < len(text):
            c = text[at]
            if c in " \t\r":
                at += 1
                continue
            if c == "#":
                end = text.find("\n", at)
                at = len(text) if end < 0 else end
                continue
            start = at
            at += 1
            if c in "\"'":
                delimiter = c * 3 if text.startswith(c * 3, start) else c
                at = start + len(delimiter)
                while at < len(text):
                    if c == '"' and text[at] == "\\":
                        at += 2
                    elif text.startswith(delimiter, at):
                        at += len(delimiter)
                        if len(delimiter) == 3:
                            for _ in range(2):
                                if at < len(text) and text[at] == c:
                                    at += 1
                                else:
                                    break
                        break
                    else:
                        at += 1
            elif c not in "\n[]=.,{}":
                while at < len(text) and text[at] not in " \t\r\n#[]=.,{}\"'":
                    at += 1
            yield text[start:at]

    stream = iter(tokens())
    look = next(stream, None)
    records = 0
    schema = None

    def take():
        nonlocal look
        result = look
        look = next(stream, None)
        return result

    def key():
        result = []
        while True:
            token = take()
            if token is None:
                return None
            if token.startswith(('"', "'")):
                try:
                    value = tomllib.loads('key=' + token)['key']
                except tomllib.TOMLDecodeError:
                    return None
                if type(value) is not str:
                    return None
            elif re.fullmatch(r"[A-Za-z0-9_-]+", token):
                value = token
            else:
                return None
            result.append(value)
            if look != '.':
                return result
            take()

    def count():
        nonlocal records
        require(records < maximum, ("file-records", maximum + 1))
        records += 1

    def value(path, depth):
        nonlocal schema
        if depth > 128:
            return False
        token = take()
        if path == ['schema']:
            schema = token
        if token == '[':
            while look != ']':
                if look in ('\n', ','):
                    take()
                elif look is None:
                    return False
                else:
                    if path == ['bundle', 'files']:
                        count()
                    if not value([], depth + 1):
                        return False
            take()
        elif token == '{':
            while look != '}':
                child = key()
                if child is None or take() != '=' or not value(path + child, depth + 1):
                    return False
                if look == ',':
                    take()
                elif look != '}':
                    return False
            take()
        elif token in (None, '\n', ',', ']', '}'):
            return False
        else:
            while look is not None and look not in ('\n', ',', ']', '}'):
                take()
        return True

    table = []
    while look is not None:
        if look == '\n':
            take()
        elif look == '[':
            take()
            array = look == '['
            if array:
                take()
            table = key()
            if table is None or take() != ']' or (array and take() != ']'):
                break
            if array and table == ['bundle', 'files']:
                count()
        else:
            child = key()
            if child is None or take() != '=' or not value(table + child, 0):
                break
    return schema


def read_lock(path):
    with path.open('rb') as source:
        raw = source.read(16_777_217)
    require(len(raw) <= 16_777_216, ("lock-bytes", len(raw)))
    text = raw.decode("utf-8")
    require(not text.startswith("\ufeff"), "BOM")
    schema_token = lock_preflight(text)
    lock = tomllib.loads(text)
    forbid_floats(lock)
    fields(lock, ["schema", "engine-version", "format-serial", "executable", "bundle"])
    require(type(lock["schema"]) is int and lock["schema"] == 1
            and schema_token == "1", "schema raw token")
    require(lock["engine-version"] == "0.15.0", "engine-version")
    require(type(lock["format-serial"]) is int and lock["format-serial"] == 33, "format-serial")
    engine = lock["executable"]
    fields(engine, ["platform", "url", "archive-sha256", "member", "sha256"])
    require(engine["platform"] == "x86_64-unknown-linux-musl", "platform")
    require(platform.system() == "Linux" and platform.machine() == "x86_64", "unsupported platform")
    url(engine["url"])
    digest(engine["archive-sha256"])
    digest(engine["sha256"])
    member_name(engine["member"])
    bundle = lock["bundle"]
    fields(bundle, ["url", "archive-sha256", "bundle-digest", "files"])
    url(bundle["url"])
    digest(bundle["archive-sha256"])
    digest(bundle["bundle-digest"])
    rows = bundle["files"]
    require(type(rows) is list and 0 < len(rows) <= 262_144, ("file-records", len(rows)))
    previous = ""
    for row in rows:
        fields(row, ["member", "sha256"])
        name = member_name(row["member"])
        require(previous.encode() < name.encode(), ("order or duplicate", previous, name))
        previous = name
        digest(row["sha256"])
    return lock


def regular(path):
    require(stat.S_ISREG(path.lstat().st_mode), ("nonregular file", str(path)))


def hash_file(path):
    regular(path)
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def directory(path):
    if path.exists() or path.is_symlink():
        require(stat.S_ISDIR(path.lstat().st_mode), ("nonregular directory", str(path)))
    else:
        path.mkdir()


def support_location(value, repository):
    require(value is not None, "missing FTOO_ASSEMBLY_SUPPORT")
    candidate = Path(value)
    require(candidate.is_absolute(), ("RelativeSupport", str(candidate)))
    root = repository.resolve(strict=True)
    support = candidate.resolve()
    require(not support.is_relative_to(root) and not root.is_relative_to(support),
            ("OverlappingSupport", str(root), str(support)))
    # An escaping link is never silently canonicalized into an accepted layout.
    require(candidate == support, ("noncanonical or linked support", str(candidate), str(support)))
    candidate.mkdir(parents=True, exist_ok=True)
    directory(candidate)
    return candidate


class PinnedRedirects(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, fp, code, message, headers, newurl):
        url(newurl)
        return super().redirect_request(request, fp, code, message, headers, newurl)


def download(component, destination):
    if destination.exists() or destination.is_symlink():
        require(hash_file(destination) == component["archive-sha256"], ("archive integrity", str(destination)))
        return
    temporary = destination.with_suffix(".partial")
    with temporary.open("xb") as output:
        opener = urllib.request.build_opener(PinnedRedirects())
        with opener.open(component["url"], timeout=60) as response:
            shutil.copyfileobj(response, output, 1024 * 1024)
    require(hash_file(temporary) == component["archive-sha256"], ("archive integrity", str(temporary)))
    temporary.rename(destination)


def inventory(root):
    result = {}
    for path in sorted(root.rglob("*")):
        mode = path.lstat().st_mode
        require(not stat.S_ISLNK(mode), ("link", str(path)))
        if stat.S_ISDIR(mode):
            continue
        regular(path)
        result[path.relative_to(root).as_posix()] = hash_file(path)
    return result


def verify(root, expected):
    actual = inventory(root)
    require(actual == expected, ("extracted inventory/integrity", str(root),
            sorted(set(expected) - set(actual)), sorted(set(actual) - set(expected)),
            [(name, expected[name], actual[name]) for name in sorted(set(expected) & set(actual))
             if expected[name] != actual[name]]))


def extract(archive_path, destination, expected):
    if destination.exists() or destination.is_symlink():
        directory(destination)
        verify(destination, expected)
        return
    stage = Path(tempfile.mkdtemp(prefix="extract-", dir=destination.parent))
    seen = set()
    try:
        with tarfile.open(archive_path, "r:*") as archive:
            for entry in archive:
                name = member_name(entry.name)
                require(entry.isfile() and name not in seen and name in expected,
                        ("archive entry", name, entry.type))
                seen.add(name)
                source = archive.extractfile(entry)
                data = source.read()
                require(hashlib.sha256(data).hexdigest() == expected[name], ("member integrity", name))
                target = stage / name
                target.parent.mkdir(parents=True, exist_ok=True)
                with target.open("xb") as output:
                    output.write(data)
        verify(stage, expected)
        stage.rename(destination)
    finally:
        if stage.exists():
            shutil.rmtree(stage)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--support", default=os.environ.get("FTOO_ASSEMBLY_SUPPORT"))
    args = parser.parse_args()
    repository = Path(__file__).resolve().parents[2]
    support = support_location(args.support, repository)
    lock = read_lock(repository / "tools/assembly/native-lock.toml")
    for name in ["runtime", "downloads", "tmp"]:
        directory(support / name)
    engine = lock["executable"]
    bundle = lock["bundle"]
    executable_archive = support / "downloads/tectonic-0.15.0-x86_64-unknown-linux-musl.tar.gz"
    bundle_archive = support / "downloads/bundle.tar"
    download(engine, executable_archive)
    download(bundle, bundle_archive)
    require(engine["member"] == "tectonic", "executable member layout")
    extract(executable_archive, support / "runtime/bin", {engine["member"]: engine["sha256"]})
    extract(bundle_archive, support / "bundle", {r["member"]: r["sha256"] for r in bundle["files"]})
    actual_digest = (support / "bundle/SHA256SUM").read_text("ascii").strip()
    require(actual_digest == bundle["bundle-digest"], ("bundle digest", actual_digest))
    (support / "runtime/bin/tectonic").chmod(0o755)
    print(f"support={support}")
    print(f"verified executable and {len(bundle['files'])} bundle members; bundle-digest={actual_digest}")


if __name__ == "__main__":
    main()
