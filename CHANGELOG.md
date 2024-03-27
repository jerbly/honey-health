# 0.5.0

- GitHub Issue Generation
  - The `-g` or `--github-issue` option can be used to create GitHub Issues for attribute and enum health. Provide the repo owner and name e.g. `myorg/myrepo`. You must have a [Personal Access Token](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens#creating-a-fine-grained-personal-access-token) that allows issue creation - put this in an environment variable `GITHUB_TOKEN` or a `.env` file.

  - Honey-health will create a markdown table, split over multiple comments if necessary. Here are examples for [Attributes](https://github.com/jerbly/honey-health/issues/1) and [Enums](https://github.com/jerbly/honey-health/issues/2).


# 0.4.3

- Fixed: "Similar" suggestions were including deprecated attributes.
- Fixed: "Extends" suggestions were not including all namespaces.

# 0.4.2

- Deprecated attributes will now show as `Bad` columns with the deprecation message. e.g. "`http.scheme` Bad - Deprecated: Replaced by `url.scheme` instead."

# 0.4.1

- Added `-s` or `--show-matches` to show all matching columns when analyzing a single dataset. By default only `Missing` and `Bad` columns are shown in the output.

# 0.4.0

- For single datasets you can now use the `-e` or `--enums` switch. This compares enum variants defined in semantic conventions with discovered variants used in tracing. Additional variants will be reported. If the attribute's enum definition has `allow_custom_values` set `true`, this is an _open enum_ and additional variants are "allowed". Honey-health still reports additional variants but as a warning (highlighted in yellow).
- Added progress bars. Some Honeycomb operations can take a while so this provides better feedback.

# 0.3.4

- Uses `cargo-dist` for build and release.

# 0.3.3

- Switched to the honeycomb-client library for consistency and significant performance increase using async to fetch dataset columns.
