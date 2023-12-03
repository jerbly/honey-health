# Honey Health

Generates reports on the health of your Honeycomb datasets' attribute names.
Provide it with OpenTelemetry Semantic Convention compatible files to find mismatches and suggestions. Compare all, or a limited set of your datasets to find commonly used attributes that may benefit from being codified into Semantic Conventions.

The output depends on the number of datasets provided and found for analysis. If a single dataset is analysed, then a csv comparison file is NOT produced (there's no other dataset to compare against!) Instead you will see output in the console like so:

```text
  Dataset  Match Miss  Bad  Score
  dataset3    28   11    2  68.3%
  
              Column Suggestion
  aws.s3.bucket.name Missing  Extends aws.s3; Similar to aws.s3.bucket
             task.id Missing
              TaskId Bad      WrongCase; NoNamespace  
```

You will always see the top section showing the number of Matching, Missing and Bad attributes. The Score is the proportion of Matching attributes (those which have defined Semantic Conventions).

If there is more that one dataset, the output is a csv file like so:

| Name               | Type   | SemConv  | Hint                                     | Usage | dataset1 | dataset2 | dataset3 |
| ------------------ | ------ | -------- | ---------------------------------------- | ----- | -------- | -------- | -------- |
| aws.s3.bucket.name | string | Missing  | Extends aws.s3; Similar to aws.s3.bucket | 1     |          |          | x        |
| aws.s3.key         | string | Matching |                                          | 1     |          |          | x        |
| task.id            | string | Missing  |                                          | 2     | x        |          | x        |
| TaskId             | string | Bad      | WrongCase; NoNamespace                   | 1     |          | x        |          |

This example report is pointing out the follow:

- `aws.s3.bucket.name` has not been found in the provided semantic conventions. However, there is a namespace `aws.s3` that this attribute would extend. Also, there is an attribute in the model with a similar name: `aws.s3.bucket`. The application delivering to `dataset3` should have its instrumention adjusted to the standard.
- `aws.s3.key` is in use by `dataset3` and matches a semantic convention in the provided models.
- `task.id` is missing from the provided model but is used by 2 datasets: `dataset1` and `dataset3`. Perhaps this is a good candidate to standardise into your own semantic conventions?
- `TaskId` is in CamelCase which does not follow the recommended standard for attribute naming. Also, this is a top-level name with no namespace - this will pollute the namespace tree.

> **Note**
>
> Only datasets and attributes within them that have been written to in the last 60 days are retrieved for analysis.

## Building

This will build the binary: `honey-health`

```shell
cargo build --release
```

## Usage

```text
Honey Health

Usage: honey-health [OPTIONS] --model <MODEL>...

Options:
  -m, --model <MODEL>...        Model paths
  -d, --dataset [<DATASET>...]  Datasets
  -o, --output <OUTPUT>         Output file path [default: hh_report.csv]
  -h, --help                    Print help (see more with '--help')
  -V, --version                 Print version
```

You must provide `HONEYCOMB_API_KEY` as an environment variable or in a `.env` file. This api key must have access to read datasets and columns.

You must provide at least one path to the model root directory of OpenTelemetry Semantic Convention compatible yaml files. Provide multiple root directories separated by spaces after `--model`. It is recommended to clone the [OpenTelemetry Semantic Conventions](https://github.com/open-telemetry/semantic-conventions) project and add this alongside your own Semantic Conventions. For example: `honey-health --model /code/semantic-conventions/model`
