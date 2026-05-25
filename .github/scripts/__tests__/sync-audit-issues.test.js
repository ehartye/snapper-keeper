// Unit tests for .github/scripts/sync-audit-issues.js
//
// Pure Node, no dependencies. Run with `node .github/scripts/__tests__/sync-audit-issues.test.js`.
// Exits 0 on pass, non-zero on failure.

const path = require('path');
const assert = require('assert');

const sync = require('../sync-audit-issues.js');

// Minimal mock of the github + context + core objects passed by github-script.
// The github-script `github` parameter has both `.rest` and `.paginate` at top
// level — Octokit's Api shape. Returns a tracking handle alongside.
function makeMockGithub({ existingOpen = [] } = {}) {
  const tracked = { created: [], closed: [], comments: [] };
  const api = {
    rest: {
      issues: {
        listForRepo: async () => ({ data: existingOpen }),
        create:      async (params) => { tracked.created.push(params); return { data: { number: 1000 + tracked.created.length, ...params } }; },
        update:      async (params) => { if (params.state === 'closed') tracked.closed.push(params); return { data: params }; },
        createComment: async (params) => { tracked.comments.push(params); return { data: params }; },
      },
    },
    // github.paginate(method, params) collects all pages — mock just returns the array directly.
    paginate: async (_method, _params) => existingOpen,
  };
  return { api, tracked };
}

function makeCore() {
  const messages = [];
  let failed = null;
  return {
    info:    (m) => messages.push(`info: ${m}`),
    warning: (m) => messages.push(`warn: ${m}`),
    setFailed: (m) => { failed = m; },
    messages, getFailed: () => failed,
  };
}

const context = { repo: { owner: 'ehartye', repo: 'snapper-keeper' } };

async function runCase(name, fn) {
  try {
    await fn();
    console.log(`PASS  ${name}`);
  } catch (e) {
    console.error(`FAIL  ${name}\n  ${e.stack || e.message}`);
    process.exitCode = 1;
  }
}

async function main() {
  const fixtures = path.join(__dirname, 'fixtures');

  await runCase('cargo: empty report → no issues opened, no comments', async () => {
    process.env.ECOSYSTEM = 'cargo';
    process.env.REPORT_PATH = path.join(fixtures, 'cargo-empty.json');
    const { api, tracked: gh } = makeMockGithub({ existingOpen: [] });
    const core = makeCore();
    await sync({ github: api, context, core });
    assert.strictEqual(gh.created.length, 0, 'should not create issues');
    assert.strictEqual(gh.comments.length, 0, 'should not comment');
    assert.strictEqual(gh.closed.length, 0, 'should not close');
  });

  await runCase('cargo: one advisory, no existing issues → opens one', async () => {
    process.env.ECOSYSTEM = 'cargo';
    process.env.REPORT_PATH = path.join(fixtures, 'cargo-one-advisory.json');
    const { api, tracked: gh } = makeMockGithub({ existingOpen: [] });
    const core = makeCore();
    await sync({ github: api, context, core });
    assert.strictEqual(gh.created.length, 1);
    assert.match(gh.created[0].title, /RUSTSEC-2026-0099/);
    assert.match(gh.created[0].title, /\[audit\/cargo\]/);
    assert.deepStrictEqual(
      [...gh.created[0].labels].sort(),
      ['area:security', 'audit:auto', 'severity:medium'],
    );
  });

  await runCase('cargo: one advisory, already-open issue → no new issue, no spam', async () => {
    process.env.ECOSYSTEM = 'cargo';
    process.env.REPORT_PATH = path.join(fixtures, 'cargo-one-advisory.json');
    const { api, tracked: gh } = makeMockGithub({
      existingOpen: [{
        number: 42,
        title: '[audit/cargo] RUSTSEC-2026-0099: fake-crate@0.5.0 — Buffer overflow',
        labels: [{ name: 'audit:auto' }],
      }],
    });
    const core = makeCore();
    await sync({ github: api, context, core });
    assert.strictEqual(gh.created.length, 0);
    assert.strictEqual(gh.comments.length, 0);
    assert.strictEqual(gh.closed.length, 0);
  });

  await runCase('cargo: open advisory issue but advisory no longer in report → close + comment', async () => {
    process.env.ECOSYSTEM = 'cargo';
    process.env.REPORT_PATH = path.join(fixtures, 'cargo-empty.json');
    const { api, tracked: gh } = makeMockGithub({
      existingOpen: [{
        number: 99,
        title: '[audit/cargo] RUSTSEC-2026-0001: old-crate@1.0.0 — Old vuln',
        labels: [{ name: 'audit:auto' }],
      }],
    });
    const core = makeCore();
    await sync({ github: api, context, core });
    assert.strictEqual(gh.created.length, 0);
    assert.strictEqual(gh.comments.length, 1);
    assert.strictEqual(gh.comments[0].issue_number, 99);
    assert.match(gh.comments[0].body, /no longer detected/);
    assert.strictEqual(gh.closed.length, 1);
    assert.strictEqual(gh.closed[0].issue_number, 99);
  });

  await runCase('pnpm: one advisory, no existing → opens one with pnpm tag', async () => {
    process.env.ECOSYSTEM = 'pnpm';
    process.env.REPORT_PATH = path.join(fixtures, 'pnpm-one-advisory.json');
    const { api, tracked: gh } = makeMockGithub({ existingOpen: [] });
    const core = makeCore();
    await sync({ github: api, context, core });
    assert.strictEqual(gh.created.length, 1);
    assert.match(gh.created[0].title, /\[audit\/pnpm\]/);
    assert.match(gh.created[0].title, /GHSA-fake-fake-fake/);
  });

  await runCase('unreadable report file → treats as empty, no failure', async () => {
    process.env.ECOSYSTEM = 'cargo';
    process.env.REPORT_PATH = '/tmp/this-file-does-not-exist-xyz';
    const { api, tracked: gh } = makeMockGithub({ existingOpen: [] });
    const core = makeCore();
    await sync({ github: api, context, core });
    assert.strictEqual(core.getFailed(), null);
    assert.strictEqual(gh.created.length, 0);
  });
}

main().catch((e) => { console.error(e); process.exitCode = 1; });
