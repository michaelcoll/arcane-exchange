import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";

import { buildAnnouncement } from "./announcement.mjs";

const fixture = (name) =>
  readFileSync(new URL(`./fixtures/${name}.md`, import.meta.url), "utf8");

const announce = (name, tag) =>
  buildAnnouncement({
    body: fixture(name),
    tag,
    url: `https://github.com/michaelcoll/arcane-exchange/releases/tag/${tag}`,
    publishedAt: "2026-11-05T12:24:37Z",
  });

const PR = "https://github.com/michaelcoll/arcane-exchange/pull";

describe("buildAnnouncement", () => {
  describe("a release with features and fixes (1.39.0)", () => {
    const [embed] = announce("1.39.0", "1.39.0").embeds;

    it("titles the embed with the version and links it to the release", () => {
      assert.equal(embed.title, "Version 1.39.0");
      assert.equal(
        embed.url,
        "https://github.com/michaelcoll/arcane-exchange/releases/tag/1.39.0",
      );
    });

    it("uses the primary colour, the footer and the publication date", () => {
      assert.equal(embed.color, 0x00daf3);
      assert.deepEqual(embed.footer, { text: "Arcane Exchange" });
      assert.equal(embed.timestamp, "2026-11-05T12:24:37Z");
    });

    it("tags the lines, then puts the iOS ones apart after the other sections", () => {
      assert.equal(
        embed.description,
        [
          "### 🔧 Corrections",
          `• **trades** · opening a Trade from an Offer is safe to repeat ([#622](${PR}/622))`,
          "",
          "### 📱 App iOS (en développement)",
          `• reload the widgets from the API client ([#625](${PR}/625))`,
          `• forget the search recents on sign-out, device memory declared by scope ([#630](${PR}/630))`,
        ].join("\n"),
      );
    });

    it("ignores the Dependencies and Maintenance sections", () => {
      assert.ok(!embed.description.includes("docker-compose"));
      assert.ok(!embed.description.includes("UserButton"));
    });

    it("disables every mention", () => {
      assert.deepEqual(announce("1.39.0", "1.39.0").allowed_mentions, {
        parse: [],
      });
    });
  });

  it("announces technical improvements when no feature or fix remains", () => {
    const [embed] = announce("deps-only", "1.39.1").embeds;

    assert.equal(
      embed.description,
      "Version 1.39.1 : améliorations techniques ([Voir la release](https://github.com/michaelcoll/arcane-exchange/releases/tag/1.39.1))",
    );
  });

  it("handles breaking (`!`) and unscoped lines", () => {
    const [embed] = announce("breaking-and-unscoped", "1.40.0").embeds;

    assert.equal(
      embed.description,
      [
        "### 🎁 Nouveautés",
        `• drop the legacy export endpoint ([#660](${PR}/660))`,
        `• **collection** · rename the binder sort options ([#661](${PR}/661))`,
        `• show the wishlist count ([#662](${PR}/662))`,
        "",
        "### 🔧 Corrections",
        `• **Serveur** · return 404 for an unknown card ([#663](${PR}/663))`,
      ].join("\n"),
    );
  });

  it("tags web/frontend as Site, backend/api as Server, shows other scopes as is, hides prototype", () => {
    const [embed] = announce("scopes", "1.41.0").embeds;

    assert.equal(
      embed.description,
      [
        "### 🎁 Nouveautés",
        `• **Site** · show the wishlist count ([#701](${PR}/701))`,
        `• **Site** · add the binder filter ([#702](${PR}/702))`,
        `• **Serveur** · expose the price spread ([#703](${PR}/703))`,
        `• **Serveur** · paginate the offers ([#704](${PR}/704))`,
        `• **trades** · propose a counter offer ([#705](${PR}/705))`,
        `• greet new players ([#706](${PR}/706))`,
      ].join("\n"),
    );
  });

  it("announces technical improvements when only hidden scopes remain", () => {
    const [embed] = announce("prototype-only", "1.41.1").embeds;

    assert.equal(
      embed.description,
      "Version 1.41.1 : améliorations techniques ([Voir la release](https://github.com/michaelcoll/arcane-exchange/releases/tag/1.41.1))",
    );
  });

  it("produces only the iOS section, without tag, for an iOS-only release", () => {
    const [embed] = announce("ios-only", "1.42.0").embeds;

    assert.equal(
      embed.description,
      [
        "### 📱 App iOS (en développement)",
        `• add a card from the catalog ([#710](${PR}/710))`,
        `• keep the binder scroll position ([#711](${PR}/711))`,
      ].join("\n"),
    );
  });

  describe("lines that are not a feat or a fix", () => {
    const body = (lines) =>
      buildAnnouncement({
        body: `### 🎁 Features\n${lines}\n`,
        tag: "1.44.0",
        url: "https://example.test/release",
        publishedAt: "2026-11-05T12:24:37Z",
      }).embeds[0].description;

    it("keeps only the feat and fix types, even under a feature or fix heading", () => {
      assert.equal(
        body(
          [
            `* chore(ci): mislabelled by @me in ${PR}/720`,
            `* Update the readme by @me in ${PR}/721`,
            `* feat: kept by @me in ${PR}/722`,
          ].join("\n"),
        ),
        ["### 🎁 Nouveautés", `• kept ([#722](${PR}/722))`].join("\n"),
      );
    });

    it("drops the issue reference closing the title", () => {
      assert.equal(
        body(`* feat: safe to repeat (#619) by @me in ${PR}/722`),
        ["### 🎁 Nouveautés", `• safe to repeat ([#722](${PR}/722))`].join(
          "\n",
        ),
      );
    });
  });

  it("never leaves a section heading without a line under it when cutting", () => {
    for (let n = 20; n <= 35; n++) {
      for (let pad = 0; pad < 150; pad += 5) {
        const feats = Array.from(
          { length: n },
          (_, i) =>
            `* feat: ${"a".repeat(80 + (i === 0 ? pad : 0))} by @me in ${PR}/${800 + i}`,
        );
        const { description } = buildAnnouncement({
          body: [
            "### 🎁 Features",
            ...feats,
            "### 🔧 Fixes",
            `* fix: ${"b".repeat(80)} by @me in ${PR}/900`,
          ].join("\n"),
          tag: "1.45.0",
          url: "https://example.test/release",
          publishedAt: "2026-11-05T12:24:37Z",
        }).embeds[0];

        const lines = description.split("\n");
        lines.forEach((line, i) => {
          if (line.startsWith("###")) {
            assert.ok(
              lines[i + 1]?.startsWith("•"),
              `n ${n}, pad ${pad}: orphan heading`,
            );
          }
        });
      }
    }
  });

  describe("a release too long for an embed", () => {
    const { embeds } = announce("too-long", "1.43.0");
    const { description } = embeds[0];

    it("stays under 4096 characters and is cut at the end of a line", () => {
      assert.ok(description.length <= 4096);
      const [kept] = description.split("\n\n[Voir la release]");
      assert.ok(
        kept
          .split("\n")
          .every((l) => l === "" || /\(\[#\d+\]\(\S+\)\)$|^###/.test(l)),
      );
    });

    it("ends with the link to the release", () => {
      assert.ok(
        description.endsWith(
          "\n\n[Voir la release](https://github.com/michaelcoll/arcane-exchange/releases/tag/1.43.0)",
        ),
      );
    });
  });
});
