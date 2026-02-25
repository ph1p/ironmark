module.exports = {
  branches: ["main"],
  plugins: [
    [
      "@semantic-release/commit-analyzer",
      {
        preset: "conventionalcommits",
        releaseRules: [
          { type: "feat", release: "minor" },
          { type: "refactor", release: "patch" },
          { type: "fix", release: "patch" },
          { type: "perf", release: "patch" },
          { type: "revert", release: "patch" },
          { breaking: true, release: "major" },
        ],
      },
    ],
    [
      "@semantic-release/release-notes-generator",
      {
        preset: "conventionalcommits",
        presetConfig: {
          types: [
            { type: "feat", section: "Features" },
            { type: "fix", section: "Bug Fixes" },
            { type: "perf", section: "Performance" },
            { type: "revert", section: "Reverts" },
            { type: "refactor", section: "Refactoring", hidden: true },
            { type: "chore", hidden: true },
            { type: "docs", hidden: true },
            { type: "style", hidden: true },
            { type: "test", hidden: true },
            { type: "ci", hidden: true },
          ],
        },
      },
    ],
    [
      "@semantic-release/exec",
      {
        prepareCmd:
          'sed -i \'s/^version = ".*"/version = "${nextRelease.version}"/\' Cargo.toml && cargo generate-lockfile',
        successCmd:
          'echo "new_release_published=true" >> $GITHUB_OUTPUT && echo "new_release_version=${nextRelease.version}" >> $GITHUB_OUTPUT',
      },
    ],
    ["@semantic-release/npm", { provenance: true }],
    [
      "@semantic-release/git",
      {
        assets: ["package.json", "Cargo.toml", "Cargo.lock"],
        message: "chore(release): ${nextRelease.version} [skip ci]\n\n${nextRelease.notes}",
      },
    ],
    "@semantic-release/github",
  ],
};
