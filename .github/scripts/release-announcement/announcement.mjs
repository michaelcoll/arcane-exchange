// Builds the Discord webhook payload announcing a release, from the notes GitHub generates
// (.github/release.yml). Pure: no I/O, so it is tested directly with real release bodies.

const COLOR = 0x00daf3; // design-system primary

// Sections of the generated notes that are read; the others (Dependencies, Maintenance) are ignored.
const SOURCE_HEADINGS = { "### 🎁 Features": "feat", "### 🔧 Fixes": "fix" };

// Shape of a generated line: `* type(scope)!: description by @author in <PR url>`.
const LINE = /^\* (\w+)(?:\(([^)]*)\))?(?:!)?: (.+) by @\S+ in (\S+)$/;

// Issue reference closing a title, already linked from the PR number: `… (#619)`.
const TRAILING_ISSUE_REF = /\s*\(#\d+\)$/;

/** Reads the lines of the Features and Fixes sections into entries, grouped by their source section. */
function parseEntries(body) {
  const entries = { feat: [], fix: [] };
  let section;

  for (const line of body.split(/\r?\n/)) {
    if (line.startsWith("### ")) {
      section = SOURCE_HEADINGS[line.trim()];
      continue;
    }
    const match = section && LINE.exec(line.trim());
    if (!match) continue;

    const [, type, scope, rawDescription, url] = match;
    if (!Object.hasOwn(entries, type)) continue;

    entries[section].push({
      scope,
      description: rawDescription.replace(TRAILING_ISSUE_REF, ""),
      url,
      number: url.split("/").pop(),
    });
  }
  return entries;
}

// Player-facing name of a scope; any other scope is shown as is.
const SCOPE_TAGS = {
  web: "Site",
  frontend: "Site", // kept for the PRs titled before the naming rule (which bans this scope)
  backend: "Serveur",
  api: "Serveur",
};

// Scopes left out of the announcement (experiments).
const HIDDEN_SCOPES = ["prototype"];

const IOS_SCOPE = "ios";

// Discord's limit for an embed description.
const MAX_DESCRIPTION = 4096;

function renderLine({ description, url, number }, tag) {
  return `• ${tag}${description} ([#${number}](${url}))`;
}

const renderTaggedLine = (entry) =>
  renderLine(
    entry,
    entry.scope ? `**${SCOPE_TAGS[entry.scope] ?? entry.scope}** · ` : "",
  );

// The iOS section already says where the line comes from.
const renderUntaggedLine = (entry) => renderLine(entry, "");

function renderSection(title, entries, render = renderTaggedLine) {
  return entries.length === 0
    ? undefined
    : [`### ${title}`, ...entries.map(render)].join("\n");
}

/** Cuts the description at a line end so that it, plus the link to the release, fits in an embed. */
function truncate(description, url) {
  if (description.length <= MAX_DESCRIPTION) return description;

  const suffix = `\n\n[Voir la release](${url})`;
  const lines = description.split("\n");
  while (
    lines.length > 0 &&
    (lines.join("\n") + suffix).length > MAX_DESCRIPTION
  ) {
    lines.pop();
  }
  // A heading or a blank line left at the end would head nothing.
  while (lines.length > 0 && !lines.at(-1).startsWith("•")) {
    lines.pop();
  }
  return lines.join("\n").trimEnd() + suffix;
}

/**
 * @param {{ body: string, tag: string, url: string, publishedAt: string }} release
 * @returns the Discord webhook payload
 */
export function buildAnnouncement({ body, tag, url, publishedAt }) {
  const entries = parseEntries(body);
  const shown = (list) => list.filter((e) => !HIDDEN_SCOPES.includes(e.scope));
  const isIos = (e) => e.scope === IOS_SCOPE;
  const feat = shown(entries.feat);
  const fix = shown(entries.fix);

  const sections = [
    renderSection(
      "🎁 Nouveautés",
      feat.filter((e) => !isIos(e)),
    ),
    renderSection(
      "🔧 Corrections",
      fix.filter((e) => !isIos(e)),
    ),
    renderSection(
      "📱 App iOS (en développement)",
      [...feat, ...fix].filter(isIos),
      renderUntaggedLine,
    ),
  ].filter(Boolean);

  const description =
    sections.length === 0
      ? `Version ${tag} : améliorations techniques ([Voir la release](${url}))`
      : truncate(sections.join("\n\n"), url);

  return {
    embeds: [
      {
        title: `Version ${tag}`,
        url,
        color: COLOR,
        description,
        footer: { text: "Arcane Exchange" },
        timestamp: publishedAt,
      },
    ],
    allowed_mentions: { parse: [] },
  };
}
