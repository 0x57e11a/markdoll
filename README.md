# markdoll

[![MADE WITH MARKDOLL](https://codeberg.org/0x57e11a/markdoll/raw/branch/main/button.png)](https://codeberg.org/0x57e11a/markdoll)

markdoll is a structured and extensible markup language

its syntax is relatively simple, with anything more complicated than frontmatter, sections, lists, and text, being delegated to tags

to get an idea of the syntax, [visit the repository](https://codeberg.org/0x57e11a/markdoll/src/branch/main/spec.doll)

## cargo features

- `cli-trace`
  print tracing information for the cli

## minimum supported rust version

this library requires features from rust 1.86

## json output

stderr output when the `--json` flag is specified

unless `--no-status` is set, status update lines will be emitted to stderr

regardless of whether status updates are emitted, the final line will be the diagnostics

### status updates

```json
{
  "kind": "status-update",
  "stage": "parse", // the stage <parse|emit>
  "status": "success" // the new status of the stage <working|success|failure|written> (note written is only for emit stage)
}
```

order:
  - `{ "kind": "status-update", "stage": "parse", "status": "working" }` - began parsing
  - `{ "kind": "status-update", "stage": "parse", "status": "<success|failure>" }` - parsing succeeded/failed
  - `{ "kind": "status-update", "stage": "emit", "status": "working" }` - began emitting
  - `{ "kind": "status-update", "stage": "emit", "status": "<success|failure>" }` - emitting succeeded/failed
  - `{ "kind": "status-update", "stage": "emit", "status": "written" }` - fully written output to stdout

### diagnostics

```json
{
  "kind": "diagnostics",
  "diagnostics": [ // array of diagnostics emitted during all stages that ran
    {
      "message": "unexpected indentation", // the user-readable message
      "code": "markdoll::lang::unexpected", // diagnostic code
      "severity": "error", // <advice|warning|error>
      "help": null, // helpful info, if applicable
      "url": null, // a relevant url, if applicable
      "labels": [ // array of all the labels of this diagnostics (there can be more than one)
        {
          "primary": false,
          "label": "expected any of: `&`", // the label of the span
          "location": "stdin:5:1"
        }
      ],
      "cause_chain": ["unexpected indentation"], // the chain of root causes
      "rendered": "markdoll::lang::unexpected\n\n  × unexpected indentation\n   ╭─[stdin:5:1]\n 4 │ \n 5 │     this is markdoll\n   · ──┬─\n   ·   ╰── expected any of: `&`\n 6 │ \n   ╰────\n" // the fully rendered diagnostic
    }
  ]
}
```