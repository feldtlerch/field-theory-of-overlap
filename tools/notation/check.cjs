'use strict';
const fs = require('node:fs');
const path = require('node:path');
const { TextDecoder } = require('node:util');
const vm = require('node:vm');

// Telemetry is optional, including when the destination disappears or fills.
// Validate once per process and stop recording after the first I/O failure.
let telemetry;
function record(phase, elapsed) {
  try {
    if (telemetry === undefined) {
      telemetry = null;
      const directory = process.env.FTOO_GATE_TELEMETRY;
      if (directory && fs.statSync(directory).isDirectory()) {
        telemetry = fs.openSync(path.join(directory, process.pid + '.jsonl'), 'a');
      }
    }
    if (telemetry !== null) {
      const bytes = Buffer.from(JSON.stringify({ pid: process.pid, phase,
        inclusive_ns: Number(elapsed), members: 0, bytes: 0 }) + '\n');
      let at = 0;
      while (at < bytes.length) {
        const written = fs.writeSync(telemetry, bytes, at, bytes.length - at);
        if (written === 0) throw new Error('telemetry write made no progress');
        at += written;
      }
    }
  } catch {
    const descriptor = telemetry;
    telemetry = null;
    if (typeof descriptor === 'number') {
      try { fs.closeSync(descriptor); } catch { /* Measurement cannot replace a result. */ }
    }
  }
}

function measured(phase, operation) {
  const start = process.hrtime.bigint();
  try { return operation(); }
  finally {
    record(phase, process.hrtime.bigint() - start);
  }
}

function options() {
  return { throwOnError: true, strict: 'error', trust: false,
    output: 'htmlAndMathml', displayMode: false, maxExpand: 1000, macros: {} };
}

function decode(input) {
  if (input.length > 262144) throw new Error('expression input limit exceeded');
  if (input.subarray(0, 3).equals(Buffer.from([239, 187, 191]))) throw new Error('input BOM');
  return new TextDecoder('utf-8', { fatal: true }).decode(input);
}

function exact(length, eof) {
  const buffer = Buffer.alloc(length);
  let at = 0;
  while (at < length) {
    const n = fs.readSync(0, buffer, at, length - at, null);
    if (!n) {
      if (!at && eof) return null;
      throw new Error('partial batch frame');
    }
    at += n;
  }
  return buffer;
}

function send(index, status, value) {
  const payload = Buffer.from(value, 'utf8');
  const header = Buffer.alloc(17);
  header.writeBigUInt64BE(index);
  header[8] = status;
  header.writeBigUInt64BE(BigInt(payload.length), 9);
  for (const bytes of [header, payload]) {
    let at = 0;
    while (at < bytes.length) at += fs.writeSync(1, bytes, at, bytes.length - at);
  }
}

function realm(distribution) {
  return measured('notation.realm_evaluation', () => {
  const context = vm.createContext({});
  vm.runInContext('var module = { exports: {} }; var exports = module.exports;', context);
  vm.runInContext(distribution, context);
  return context.module.exports;
  });
}

function render(katex, source) {
  try {
    return [0, measured('notation.expression_evaluation', () => katex.renderToString(source, options()))];
  } catch (error) {
    const status = error instanceof katex.ParseError ? 2 : 3;
    try { return [status, String(error?.message)]; }
    catch { return [3, 'unprintable thrown value']; }
  }
}

function batch(entry, expected) {
  const distribution = fs.readFileSync(entry, 'utf8');
  const version = realm(distribution).version;
  if (version !== expected) throw new Error('package version mismatch');
  fs.writeSync(1, process.versions.node + '\n' + version + '\n');
  for (let index = 0n; ; index++) {
    const header = exact(12, true);
    if (header === null) return;
    if (index >= 4096n) throw new Error('batch request limit exceeded: maximum 4096, actual 4097');
    if (header.readBigUInt64BE(0) !== index) throw new Error('batch request index mismatch');
    const length = header.readUInt32BE(8);
    if (length > 262144) throw new Error('expression input limit exceeded');
    const input = exact(length, false);
    try {
      const source = decode(input);
      const katex = realm(distribution);
      send(index, ...render(katex, source));
    } catch (error) {
      send(index, 3, String(error.message));
    }
  }
}

function main() {
try {
  const [mode, entry] = process.argv.slice(2);
  if (process.argv.length !== 4 || !['--version', '--expression', '--batch'].includes(mode) || !path.isAbsolute(entry)) {
    throw new Error('expected an explicit mode and absolute package entry');
  }
  const pin = fs.readFileSync(path.join(__dirname, '.node-version'), 'utf8');
  if (!/^\d+\.\d+\.\d+\n$/.test(pin) || process.versions.node !== pin.trimEnd()) {
    throw new Error('runtime version mismatch');
  }
  const expected = JSON.parse(fs.readFileSync(path.join(__dirname, 'package.json'), 'utf8')).dependencies.katex;
  if (mode === '--batch') {
    batch(entry, expected);
    return;
  }
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
    const source = decode(buffer.subarray(0, length));
    try {
      process.stdout.write(measured('notation.expression_evaluation', () => katex.renderToString(source, options())));
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
}

module.exports = { realm, render };
if (require.main === module) main();
