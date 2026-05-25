// .github/scripts/sync-audit-issues.js
//
// Per-advisory issue lifecycle for the nightly audit workflow.
// Called from .github/workflows/audit.yml via actions/github-script.
//
// Behavior:
//   For each advisory ID in the report:
//     If no open issue with this advisory ID → create one.
//     If one exists → leave it alone (no spam).
//   For each open issue with `audit:auto` whose advisory ID is NOT in the
//   current report:
//     Comment "Advisory no longer detected" and close.
//
// Required env: ECOSYSTEM (cargo|pnpm), REPORT_PATH (file path to audit JSON).

const fs = require('fs');

const ECOSYSTEM_LABELS = {
  cargo: { idPrefix: 'RUSTSEC', name: 'cargo' },
  pnpm:  { idPrefix: 'GHSA',    name: 'pnpm'  },
};

// Normalize a cargo-audit JSON report into our internal advisory shape.
function normalizeCargo(report) {
  const out = [];
  for (const v of (report.vulnerabilities?.list ?? [])) {
    out.push({
      id:             v.advisory?.id ?? 'UNKNOWN',
      severity:       v.advisory?.cvss ?? 'unknown',
      packageName:    v.package?.name ?? 'unknown',
      packageVersion: v.package?.version ?? 'unknown',
      title:          v.advisory?.title ?? '',
      url:            v.advisory?.url ?? '',
      description:    v.advisory?.description ?? '',
    });
  }
  return out;
}

// Normalize a pnpm-audit JSON report into our internal advisory shape.
// `pnpm audit --json` emits an object keyed by advisory id.
function normalizePnpm(report) {
  const out = [];
  const advisories = report.advisories ?? {};
  for (const [id, a] of Object.entries(advisories)) {
    out.push({
      id:             a.github_advisory_id ?? a.cves?.[0] ?? `pnpm-${id}`,
      severity:       a.severity ?? 'unknown',
      packageName:    a.module_name ?? 'unknown',
      packageVersion: a.vulnerable_versions ?? 'unknown',
      title:          a.title ?? '',
      url:            a.url ?? '',
      description:    a.overview ?? '',
    });
  }
  return out;
}

async function main({ github, context, core }) {
  const ecosystem = process.env.ECOSYSTEM;
  const reportPath = process.env.REPORT_PATH;
  if (!ecosystem || !reportPath) {
    core.setFailed('ECOSYSTEM and REPORT_PATH env vars are required.');
    return;
  }

  const eco = ECOSYSTEM_LABELS[ecosystem];
  if (!eco) {
    core.setFailed(`Unknown ECOSYSTEM: ${ecosystem}`);
    return;
  }

  let raw;
  try {
    raw = JSON.parse(fs.readFileSync(reportPath, 'utf8'));
  } catch (e) {
    core.warning(`Audit report unreadable at ${reportPath}: ${e.message}`);
    core.info('Treating as empty report (no advisories detected).');
    raw = {};
  }

  const advisories =
    ecosystem === 'cargo' ? normalizeCargo(raw)
    : ecosystem === 'pnpm' ? normalizePnpm(raw)
    : [];

  core.info(`Normalized ${advisories.length} advisories for ${ecosystem}.`);

  // List currently open audit:auto issues for this ecosystem.
  const openIssues = await github.paginate(github.rest.issues.listForRepo, {
    owner: context.repo.owner,
    repo:  context.repo.repo,
    state: 'open',
    labels: 'audit:auto',
    per_page: 100,
  });

  const ecosystemTagInTitle = `[audit/${eco.name}]`;
  const openByAdvisoryId = new Map();
  for (const issue of openIssues) {
    if (!issue.title.startsWith(ecosystemTagInTitle)) continue;
    const m = issue.title.match(/\b(RUSTSEC-\d{4}-\d+|GHSA-[a-z0-9-]+|pnpm-\d+)\b/);
    if (m) openByAdvisoryId.set(m[1], issue);
  }

  const currentIds = new Set(advisories.map(a => a.id));

  // 1) Open new issues for advisories that don't have one.
  for (const adv of advisories) {
    if (openByAdvisoryId.has(adv.id)) {
      core.info(`Already-open issue for ${adv.id}; skipping.`);
      continue;
    }
    const title = `${ecosystemTagInTitle} ${adv.id}: ${adv.packageName}@${adv.packageVersion} — ${adv.title}`;
    const body = [
      `**Ecosystem:** ${eco.name}`,
      `**Advisory:** \`${adv.id}\``,
      `**Severity:** ${adv.severity}`,
      `**Package:** \`${adv.packageName}\` @ \`${adv.packageVersion}\``,
      adv.url ? `**More info:** ${adv.url}` : '',
      '',
      '### Description',
      '',
      adv.description || '_(no description in advisory feed)_',
      '',
      '---',
      '',
      '_Filed automatically by `.github/workflows/audit.yml`. This issue will close itself when the advisory is no longer detected._',
    ].filter(Boolean).join('\n');

    const created = await github.rest.issues.create({
      owner: context.repo.owner,
      repo:  context.repo.repo,
      title,
      body,
      labels: ['area:security', 'audit:auto', 'severity:medium'],
    });
    core.info(`Opened issue #${created.data.number}: ${title}`);
  }

  // 2) Close issues whose advisory is no longer in the report.
  for (const [advId, issue] of openByAdvisoryId.entries()) {
    if (currentIds.has(advId)) continue;
    await github.rest.issues.createComment({
      owner: context.repo.owner,
      repo:  context.repo.repo,
      issue_number: issue.number,
      body: `Advisory no longer detected as of ${new Date().toISOString().slice(0, 10)} — closing.`,
    });
    await github.rest.issues.update({
      owner: context.repo.owner,
      repo:  context.repo.repo,
      issue_number: issue.number,
      state: 'closed',
    });
    core.info(`Closed issue #${issue.number} (${advId} no longer detected)`);
  }
}

module.exports = main;
