'use strict';
const fs = require('node:fs');
const path = require('node:path');
const { TextDecoder } = require('node:util');

try {
  const [mode, entry] = process.argv.slice(2);
  if (process.argv.length !== 4 || !['--version', '--expression'].includes(mode) || !path.isAbsolute(entry)) {
    throw new Error('expected an explicit mode and absolute package entry');
  }
  const pin = fs.readFileSync(path.join(__dirname, '.node-version'), 'utf8');
  if (!/^\d+\.\d+\.\d+\n$/.test(pin) || process.versions.node !== pin.trimEnd()) {
    throw new Error('runtime version mismatch');
  }
  const expected = JSON.parse(fs.readFileSync(path.join(__dirname, 'package.json'), 'utf8')).dependencies.katex;
  const katex = require(entry);
  if (katex.version !== expected) throw new Error('package version mismatch');
  if (mode === '--version') {
    process.stdout.write(process.versions.node + '\n' + katex.version + '\n');
  } else {
    const buffer = Buffer.alloc(262145);
    let length = 0;
    while (length < buffer.length) {
      const count = fs.readSync(0, buffer, length, buffer.length - length, null);
      if (count === 0) break;
      length += count;
    }
    if (length > 262144) throw new Error('expression input limit exceeded');
    const input = buffer.subarray(0, length);
    if (input.subarray(0, 3).equals(Buffer.from([239, 187, 191]))) throw new Error('input BOM');
    const source = new TextDecoder('utf-8', { fatal: true }).decode(input);
    try {
      process.stdout.write(katex.renderToString(source, {
        throwOnError: true, strict: 'error', trust: false,
        output: 'htmlAndMathml', displayMode: false, maxExpand: 1000, macros: {},
      }));
    } catch (error) {
      if (!(error instanceof katex.ParseError)) throw error;
      process.stdout.write(error.message);
      process.exitCode = 2;
    }
  }
} catch (error) {
  process.stderr.write(String(error.message));
  process.exitCode = 3;
}
