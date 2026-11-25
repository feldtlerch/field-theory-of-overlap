project('field-theory-of-overlap', meson_version: '>=1.11.2')
if meson.version() != '1.11.2'
  error('Meson 1.11.2 is required')
endif
ninja = find_program('ninja', version: '1.13.2')
if run_command(ninja, '--version', check: true).stdout().strip() != '1.13.2'
  error('Ninja 1.13.2 is required')
endif
python = find_program('python3')
cargo = find_program('cargo')
node = find_program('node')
rustc = find_program('rustc')
path_check = run_command(python, '-c',
  'import os,shutil,sys; names=("meson","ninja","python3","cargo","rustc","node"); sys.exit(any(not shutil.which(n) or os.path.realpath(shutil.which(n)) != os.path.realpath(p) for n,p in zip(names,sys.argv[1:])))',
  find_program('meson').full_path(), ninja.full_path(), python.full_path(),
  cargo.full_path(), rustc.full_path(), node.full_path(), check: false)
if path_check.returncode() != 0
  error('PATH must resolve the configured Meson, Ninja, Python, Cargo, Rust and Node programs')
endif
support = get_option('support')
notation_support = get_option('notation_support')
cargo_target_dir = get_option('cargo_target_dir')
cargo_home = get_option('cargo_home')
if not support.startswith('/') or not notation_support.startswith('/') or not cargo_target_dir.startswith('/') or not cargo_home.startswith('/')
  error('support, notation_support, cargo_target_dir and cargo_home must be absolute')
endif
notation_env = environment()
notation_env.set('FTOO_NOTATION_SUPPORT', notation_support)
source = meson.project_source_root()
adapter = source / 'tools/paper/meson_adapter.py'
t40 = source / 'tools/paper/meson_t40.py'
paper_record = custom_target('paper-record', output: 'paper-record.json',
  command: [python, adapter, 'paper-record', '--repository', source,
    '--support', support, '--output', '@OUTPUT@'],
  build_always_stale: true)
rust_record = custom_target('rust-record', output: 'rust-record.json',
  command: [python, adapter, 'rust-record', '--repository', source,
    '--cargo-target-dir', cargo_target_dir, '--cargo-home', cargo_home,
    '--output', '@OUTPUT@'],
  build_always_stale: true)
t40_record = custom_target('t40-record', output: 't40-record.json',
  command: [python, adapter, 't40-record', '--repository', source,
    '--output', '@OUTPUT@'], build_always_stale: true)
codegen = custom_target('codegen', build_subdir: 'bin', input: rust_record, output: 'paper',
  command: [python, adapter, 'codegen', '--repository', source,
    '--cargo-target-dir', cargo_target_dir, '--cargo-home', cargo_home,
    '--record', '@INPUT@', '--output', '@OUTPUT@'])
run_target('documents-generate', command: [codegen, 'documents-generate',
  '--repository', source, '--support', support], env: notation_env)
documents_record = custom_target('documents-record', output: 'documents-record.json',
  command: [python, adapter, 'documents-record', '--repository', source,
    '--support', support, '--notation-support', notation_support,
    '--output', '@OUTPUT@'], build_always_stale: true)
documents_verified = custom_target('documents-verified',
  input: [documents_record, codegen], output: 'documents.checked',
  command: [python, adapter, 'documents-check', '--repository', source,
    '--support', support, '--record', '@INPUT0@', '--binary', '@INPUT1@',
    '--output', '@OUTPUT@'], env: notation_env)
pair = custom_target('pair', build_subdir: 'paper',
  input: [paper_record, documents_verified, codegen],
  output: ['paper.pdf', 'explainer.pdf', 'inputs.json', 'identity-ledger.json', 'pair.json'],
  command: [codegen, 'produce', '--repository', source, '--support', support,
    '--work', '@PRIVATE_DIR@/work', '--output-dir', '@OUTDIR@'],
  install: true,
  install_dir: ['generated/paper', 'generated/paper', 'generated/paper', false, false],
  install_mode: ['rw-r--r--'],
  install_tag: ['doc', 'doc', 'doc', 'doc', 'doc'])
t40_fixture = custom_target('fixture-T40-identities', build_subdir: 't40',
  input: [paper_record, t40_record], output: 't40-identities.json',
  command: [python, t40, 'fixture', '--repository', source, '--support', support,
    '--private', '@PRIVATE_DIR@', '--output', '@OUTPUT@'])
t40_assert = custom_target('assert-T40', build_subdir: 't40',
  input: [t40_fixture, t40_record], output: 'T40.verdict.json',
  command: [python, t40, 'assert', '--input', '@INPUT0@', '--output', '@OUTPUT@'])
cargo_env = environment()
cargo_env.set('CARGO_TARGET_DIR', cargo_target_dir)
cargo_env.set('CARGO_HOME', cargo_home)
support_env = environment()
support_env.set('CARGO_TARGET_DIR', cargo_target_dir)
support_env.set('CARGO_HOME', cargo_home)
support_env.set('FTOO_ASSEMBLY_SUPPORT', support)
support_env.set('FTOO_NOTATION_SUPPORT', notation_support)
test('unit', cargo, args: ['test', '--locked', '--offline', '--workspace', '--no-fail-fast',
  '--', '--skip', 'support_required'], suite: 'unit', timeout: 0,
  is_parallel: false, env: cargo_env, workdir: source)
test('regression', cargo, args: ['test', '--locked', '--offline', '--workspace', '--no-fail-fast',
  '--', 'support_required', '--skip', 'support_required_paper_gate'],
  suite: 'regression', timeout: 0, is_parallel: false, env: support_env, workdir: source)
test('byte-gate', codegen, args: ['check', '--repository', source,
  '--built', meson.current_build_dir() / 'paper'],
  depends: pair, suite: 'product', timeout: 60, is_parallel: false, workdir: source)
test('complete-proof-controls', python,
  args: ['-B', source / 'tools/paper/test_complete_gate.py'],
  suite: 'unit', timeout: 60, is_parallel: false, workdir: source)
test('complete-plain-conversion', python,
  args: ['-B', source / 'tools/paper/test_complete_gate.py', '--repository', source,
    '--binary', codegen, '--support', support,
    '--work', meson.current_build_dir() / 'complete-plain-conversion'],
  depends: codegen, suite: 'complete', timeout: 1800, is_parallel: false,
  env: notation_env, workdir: source)
test('live-release', codegen, args: ['check-registered-html-gaps', '--repository', source,
  '--support', support, '--built', meson.current_build_dir() / 'paper'],
  depends: pair, suite: 'product', timeout: 300, is_parallel: false,
  env: notation_env, workdir: source)
test('evidence-T40', python, args: [t40, 'evidence', '--input', t40_assert],
  depends: t40_assert, suite: 'product', timeout: 60, is_parallel: false)
test('graph-content', python, args: [source / 'tools/paper/meson_graph.py', 'G1',
  meson.current_build_dir() / 'graph', adapter, cargo_home],
  suite: 'graph', timeout: 300, is_parallel: false)
test('graph-publication', python, args: [source / 'tools/paper/meson_graph.py', 'G2',
  meson.current_build_dir() / 'graph', codegen, source],
  depends: codegen, suite: 'graph', timeout: 300, is_parallel: false)
