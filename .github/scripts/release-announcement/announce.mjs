// Entry point: reads the release of $TAG through the GitHub API, builds the announcement and posts it
// to the Discord webhook, or only prints the payload when $DRY_RUN is "true".
//
// Environment: TAG, GITHUB_REPOSITORY, GITHUB_TOKEN (optional locally for a public repository),
// DRY_RUN, DISCORD_RELEASE_WEBHOOK_URL (not needed in dry run).

import { buildAnnouncement } from "./announcement.mjs";

const {
  TAG,
  GITHUB_REPOSITORY,
  GITHUB_TOKEN,
  DRY_RUN,
  DISCORD_RELEASE_WEBHOOK_URL,
} = process.env;

if (!TAG || !GITHUB_REPOSITORY) {
  throw new Error("TAG and GITHUB_REPOSITORY are required");
}

const releaseResponse = await fetch(
  `https://api.github.com/repos/${GITHUB_REPOSITORY}/releases/tags/${encodeURIComponent(TAG)}`,
  {
    headers: {
      Accept: "application/vnd.github+json",
      ...(GITHUB_TOKEN && { Authorization: `Bearer ${GITHUB_TOKEN}` }),
    },
  },
);
if (!releaseResponse.ok) {
  throw new Error(
    `Cannot read the release ${TAG}: HTTP ${releaseResponse.status}`,
  );
}
const release = await releaseResponse.json();

const payload = buildAnnouncement({
  body: release.body ?? "",
  tag: release.tag_name,
  url: release.html_url,
  publishedAt: release.published_at,
});

if (DRY_RUN === "true") {
  console.log(JSON.stringify(payload, null, 2));
} else {
  if (!DISCORD_RELEASE_WEBHOOK_URL) {
    throw new Error("DISCORD_RELEASE_WEBHOOK_URL is required to post");
  }
  const postResponse = await fetch(DISCORD_RELEASE_WEBHOOK_URL, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });
  if (!postResponse.ok) {
    throw new Error(
      `Discord answered HTTP ${postResponse.status}: ${await postResponse.text()}`,
    );
  }
  console.log(`Announced ${release.tag_name}`);
}
