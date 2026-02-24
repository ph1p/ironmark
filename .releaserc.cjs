module.exports = {
  branches: ["main"],
  plugins: [
    "@semantic-release/commit-analyzer",
    "@semantic-release/release-notes-generator",
    [
      "@semantic-release/exec",
      {
        prepareCmd:
          'sed -i \'s/^version = ".*"/version = "${nextRelease.version}"/\' Cargo.toml && cargo generate-lockfile',
      },
    ],
    ["@semantic-release/npm", { provenance: true }],
    [
      "@semantic-release/git",
      {
        assets: ["Cargo.toml", "Cargo.lock"],
        message: "chore(release): ${nextRelease.version} [skip ci]\n\n${nextRelease.notes}",
      },
    ],
    "@semantic-release/github",
  ],
};
