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

function renderLine({ description, url, number }) {
  return `• ${description} ([#${number}](${url}))`;
}

function renderSection(title, entries) {
  return entries.length === 0
    ? undefined
    : [`### ${title}`, ...entries.map(renderLine)].join("\n");
}

/**
 * @param {{ body: string, tag: string, url: string, publishedAt: string }} release
 * @returns the Discord webhook payload
 */
export function buildAnnouncement({ body, tag, url, publishedAt }) {
  const { feat, fix } = parseEntries(body);

  const sections = [
    renderSection("🎁 Nouveautés", feat),
    renderSection("🔧 Corrections", fix),
  ].filter(Boolean);

  const description =
    sections.length === 0
      ? `Version ${tag} : améliorations techniques ([Voir la release](${url}))`
      : sections.join("\n\n");

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
