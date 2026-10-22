#!/usr/bin/env node
/**
 * Guards the Rust toolchain against drift.
 *
 * The Docker job failed on 2026-10-21 while every local check passed: the
 * Dockerfile pinned `rust:1.88-alpine` but `Cargo.lock` pulls crates that
 * require a newer rustc (uuid 1.28 needs >= 1.89). The CI itself used a
 * floating `dtolnay/rust-toolchain@stable`, so nothing checked that the
 * image, the CI and the local machine agreed.
 *
 * This script makes that class of bug fail fast, locally, before a push:
 *   1. the local rustc must match the version pinned in rust-toolchain.toml
 *   2. the version pinned in rust-toolchain.toml must match the `rust:` tag
 *      used by the Dockerfile
 *
 * Exit code 0 when everything is aligned, 1 otherwise.
 */
import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');

const errors = [];

/** "1.99.0" -> "1.99"; the toolchain line, the part that must agree. */
function line(version) {
  return version.split('.').slice(0, 2).join('.');
}

/** The version pinned in rust-toolchain.toml, e.g. "1.99.0". */
function pinnedToolchain() {
  const toml = readFileSync(join(root, 'rust-toolchain.toml'), 'utf8');
  const match = toml.match(/^channel\s*=\s*"([^"]+)"/m);
  if (!match) throw new Error('rust-toolchain.toml: no `channel` found');
  return match[1];
}

/** The `rust:<tag>` image used by the Dockerfile, e.g. "1.99-alpine". */
function dockerToolchain() {
  const dockerfile = readFileSync(join(root, 'Dockerfile'), 'utf8');
  const match = dockerfile.match(/^FROM rust:([\d.]+)/m);
  if (!match) throw new Error('Dockerfile: no `FROM rust:<version>` found');
  return match[1];
}

const pinned = pinnedToolchain();

// 1. Local rustc vs rust-toolchain.toml
let local = 'unknown';
try {
  local = execFileSync('rustc', ['--version'], { encoding: 'utf8' }).trim();
  const version = local.match(/rustc\s+([\d.]+)/)?.[1] ?? '';
  // Compare on the toolchain line; a newer patch release is fine.
  if (version && line(version) !== line(pinned)) {
    errors.push(
      `local rustc is ${version} but rust-toolchain.toml pins ${pinned}.\n` +
        `  Install it with: rustup toolchain install ${pinned}`,
    );
  }
} catch {
  errors.push('rustc not found in PATH');
}

// 2. Dockerfile vs rust-toolchain.toml
const docker = dockerToolchain();
if (line(docker) !== line(pinned)) {
  errors.push(
    `the Dockerfile builds with rust:${docker} but rust-toolchain.toml pins ${pinned}.\n` +
      `  Update the Dockerfile to: FROM rust:${pinned}-alpine`,
  );
}

if (errors.length > 0) {
  console.error('ci:toolchain: FAILED');
  for (const error of errors) console.error(`  - ${error}`);
  process.exit(1);
}

console.log(
  `ci:toolchain: ok (rustc ${local.replace(/^rustc\s+/, '')}, rust:${docker}, pinned ${pinned})`,
);
