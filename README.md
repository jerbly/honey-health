# Honey Health

Generates reports on the health of your [Honeycomb](https://honeycomb.io) datasets' attribute names.

Provide it with OpenTelemetry Semantic Convention compatible files to find mismatches and suggestions. Compare all, or a limited set of your datasets, to find commonly used attributes that may benefit from being codified into Semantic Conventions.

The output depends on the number of datasets provided and found for analysis. If a single dataset is analyzed, then a csv comparison file is NOT produced (there's no other dataset to compare against!) Instead you will see output in the console like so:

```text
  Dataset  Match Miss  Bad  Score
  dataset3    28   11    2  68.3%

              Column Suggestion
  aws.s3.bucket.name Missing  Extends aws.s3; Similar to aws.s3.bucket
             task.id Missing
              TaskId Bad      WrongCase; NoNamespace  
```

You will always see the top section showing the number of Matching, Missing and Bad attributes. The Score is the proportion of Matching attributes (those which have defined Semantic Conventions).

## Enums

For single datasets you can also use the `-e` or `--enums` switch. This compares enum variants defined in semantic conventions with discovered variants used in tracing. Additional variants will be reported.

```text
                  Column Undefined-variants
            browser.type
            message.type
                 os.type Linux, Windows 10, Mac OS
              rpc.system jsonrpc
                rpc.type error
  telemetry.sdk.language
```

## Multiple datasets

If there is more that one dataset, the output is a csv file like so:

| Name               | Type   | SemConv  | Hint                                     | Usage | dataset1 | dataset2 | dataset3 |
| ------------------ | ------ | -------- | ---------------------------------------- | ----- | -------- | -------- | -------- |
| aws.s3.bucket.name | string | Missing  | Extends aws.s3; Similar to aws.s3.bucket | 1     |          |          | x        |
| aws.s3.key         | string | Matching |                                          | 1     |          |          | x        |
| task.id            | string | Missing  |                                          | 2     | x        |          | x        |
| TaskId             | string | Bad      | WrongCase; NoNamespace                   | 1     |          | x        |          |

This example report is pointing out the following:

- `aws.s3.bucket.name` has not been found in the provided semantic conventions. However, there is a namespace `aws.s3` that this attribute would extend. Also, there is an attribute in the model with a similar name: `aws.s3.bucket`. The application delivering to `dataset3` should have its instrumentation adjusted to the standard.
- `aws.s3.key` is in use by `dataset3` and matches a semantic convention in the provided models.
- `task.id` is missing from the provided model but is used by 2 datasets: `dataset1` and `dataset3`. Perhaps this is a good candidate to standardize into your own semantic conventions?
- `TaskId` is in CamelCase which does not follow the recommended standard for attribute naming. Also, this is a top-level name with no namespace - this will pollute the namespace tree.

> **Note**
>
> Only datasets and attributes within them, that have been written to in the last 30 days, are retrieved for analysis. This can be overridden with the `--last-written-days` option.

## Installing

[Follow the instructions on the release page.](https://github.com/jerbly/honey-health/releases) There are installers of pre-built binaries for popular OSes.

## Building

If you really want to build from source and not use a [pre-built binary release](https://github.com/jerbly/honey-health/releases) then firstly you'll need a [Rust installation](https://www.rust-lang.org/) to compile it, then:

```shell
$ git clone https://github.com/jerbly/honey-health.git
$ cd honey-health
$ cargo build --release
$ ./target/release/honey-health --version
0.5.0
```

## Usage

```text
Honey Health

Usage: honey-health [OPTIONS] --model <MODEL>...

Options:
  -m, --model <MODEL>...                       Model paths
  -d, --dataset [<DATASET>...]                 Datasets
  -o, --output <OUTPUT>                        Output file path [default: hh_report.csv]
  -l, --last-written-days <LAST_WRITTEN_DAYS>  Max last written days [default: 30]
  -e, --enums                                  Enum check
  -s, --show-matches                           Show matches
  -g, --github-issue <GITHUB_ISSUE>            GitHub issue
  -h, --help                                   Print help (see more with '--help')
  -V, --version                                Print version
```

You must provide `HONEYCOMB_API_KEY` as an environment variable or in a `.env` file. This API key must be an [environment configuration key](https://docs.honeycomb.io/get-started/configure/environments/manage-api-keys/#configuration-keys) with permissions to `Create Datasets` and `Manage Queries and Columns`.

You must provide at least one path to the model root directory of OpenTelemetry Semantic Convention compatible yaml files. Provide multiple root directories separated by spaces after `--model`. It is recommended to clone the [OpenTelemetry Semantic Conventions](https://github.com/open-telemetry/semantic-conventions) project and add this alongside your own Semantic Conventions. For example: `honey-health --model /code/semantic-conventions/model`

### GitHub Issue Generation

The `-g` or `--github-issue` option can be used to create GitHub Issues for attribute and enum health. Provide the repo owner and name e.g. `myorg/myrepo`. You must have a [Personal Access Token](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens#creating-a-fine-grained-personal-access-token) that allows issue creation - put this in an environment variable `GITHUB_TOKEN` or a `.env` file.

Honey-health will create a markdown table, split over multiple comments if necessary. Here are examples for [Attributes](https://github.com/jerbly/honey-health/issues/1) and [Enums](https://github.com/jerbly/honey-health/issues/2).
