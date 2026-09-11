import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { chmodSync, copyFileSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir, arch, release } from 'node:os';
import { join } from 'node:path';

// A kernel-loading witness, not a QSR integration test. All executables are local,
// print fixed markers, and exit. No namespace, credential, or network operations.
assert.equal(process.platform, 'linux');
assert.equal(arch(), 'x64', 'this assembly witness is x86_64-specific');
const root = mkdtempSync(join(tmpdir(), 'qsr-elf-boundary-'));
const environment = { PATH: '/usr/bin:/bin', LANG: 'C' };
const sha256 = (path) => createHash('sha256').update(readFileSync(path)).digest('hex');
function command(program, args, expectedStatus = 0) {
  const result = spawnSync(program, args, {
    cwd: root, env: environment, input: '', encoding: 'utf8',
    timeout: 10000, maxBuffer: 1024 * 1024,
  });
  assert.ifError(result.error);
  assert.equal(result.signal, null);
  assert.equal(result.status, expectedStatus, result.stderr);
  return result;
}
try {
  writeFileSync(join(root, 'held_main.c'), `#include <stdio.h>\nint main(void) {\n  puts("HELD_MAIN_REACHED");\n  fflush(stdout);\n  return getchar() == EOF ? 77 : 0;\n}\n`);
  writeFileSync(join(root, 'marker_loader.S'), `.section .rodata\nmessage: .ascii "IMAGE_LOADER_RAN_BEFORE_GATE\\n"\n.set message_length, . - message\n.section .text\n.global _start\n_start:\n  mov $1, %rax\n  mov $1, %rdi\n  lea message(%rip), %rsi\n  mov $message_length, %rdx\n  syscall\n  mov $60, %rax\n  xor %rdi, %rdi\n  syscall\n.section .note.GNU-stack,"",@progbits\n`);
  command('/usr/bin/cc', ['-Wall', '-Wextra', '-Werror', 'held_main.c', '-o', 'system_gate']);
  const systemHeaders = command('/usr/bin/readelf', ['-lW', 'system_gate']).stdout;
  const loaderMatch = systemHeaders.match(/Requesting program interpreter: ([^\]]+)\]/);
  assert.ok(loaderMatch, 'system dynamic loader must be observed, not guessed');
  const loaderPath = join(root, 'fixture_loader');
  command('/usr/bin/cc', ['-Wall', '-Wextra', '-Werror', 'held_main.c', '-o', 'dynamic_gate', `-Wl,--dynamic-linker=${loaderPath}`]);
  command('/usr/bin/cc', ['-Wall', '-Wextra', '-Werror', '-static', 'held_main.c', '-o', 'static_gate']);
  command('/usr/bin/cc', ['-nostdlib', '-shared', '-Wl,-e,_start', 'marker_loader.S', '-o', 'marker_loader']);
  const dynamicHeaders = command('/usr/bin/readelf', ['-lW', 'dynamic_gate']).stdout;
  const staticHeaders = command('/usr/bin/readelf', ['-lW', 'static_gate']).stdout;
  assert.match(dynamicHeaders, /INTERP/);
  assert.doesNotMatch(staticHeaders, /INTERP/);
  const gateHash = sha256(join(root, 'dynamic_gate'));
  const trials = [];
  for (let index = 1; index <= 3; index += 1) {
    copyFileSync(loaderMatch[1], loaderPath);
    chmodSync(loaderPath, 0o555);
    const before = command(join(root, 'dynamic_gate'), [], 77);
    assert.equal(before.stdout, 'HELD_MAIN_REACHED\n');
    rmSync(loaderPath);
    copyFileSync(join(root, 'marker_loader'), loaderPath);
    chmodSync(loaderPath, 0o555);
    const changedLoader = command(join(root, 'dynamic_gate'), []);
    assert.equal(changedLoader.stdout, 'IMAGE_LOADER_RAN_BEFORE_GATE\n');
    assert.equal(sha256(join(root, 'dynamic_gate')), gateHash);
    const staticControl = command(join(root, 'static_gate'), [], 77);
    assert.equal(staticControl.stdout, 'HELD_MAIN_REACHED\n');
    trials.push({trial: index, no_token_sent: true, dynamic_gate_hash_unchanged: true,
      normal_loader_reaches_gate: true, replacement_loader_runs_before_gate: true,
      static_control_reaches_gate: true});
    rmSync(loaderPath);
  }
  console.log(JSON.stringify({
    evidence_class: 'local Linux ELF mechanism witness; not QSR/Rust/Podman isolation evidence',
    platform: process.platform, architecture: arch(), kernel: release(),
    cc_version: command('/usr/bin/cc', ['-dumpfullversion']).stdout.trim(),
    dynamic_gate_sha256: gateHash, dynamic_has_pt_interp: true, static_has_pt_interp: false,
    trials, assertions: 'PASS',
  }, null, 2));
} finally {
  rmSync(root, { recursive: true, force: true });
}
