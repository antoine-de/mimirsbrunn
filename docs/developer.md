# Developer Documentation

Table of Contents
=================

  * [Design](#design)
	  * [Software Architecture](#software-architecture)
	* [Contributing](#contributing)
  * [Dependencies](#dependencies)
	* [Development Process](#development-process)
	* [Testing](#testing)
	* [Tools](#tools)
    * [import2mimir](#import2mimir)

## Design

### Software Architecture

## Contributing

## Dependencies

### Crates

<table>
<colgroup>
<col style="width: 20%" />
<col style="width: 19%" />
<col style="width: 41%" />
<col style="width: 18%" />
</colgroup>
<thead>
<tr class="header">
<th>Domain</th>
<th>Crate</th>
<th>Motivation</th>
<th>Alternatives</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td>logging</td>
<td>tracing</td>
<td><ul>
<li>Same team as tokio, warp, …</li>
<li>Support opentelemetry</li>
<li>Support tracing, logs</li>
</ul></td>
<td></td>
</tr>
<tr class="even">
<td>error handling</td>
<td>snafu</td>
<td></td>
<td></td>
</tr>
<tr class="odd">
<td>web framework</td>
<td>warp</td>
<td></td>
<td></td>
</tr>
<tr class="even">
<td>commandline</td>
<td>structopt</td>
<td></td>
<td></td>
</tr>
<tr class="odd">
<td>elasticsearch</td>
<td>elasticsearch</td>
<td></td>
<td></td>
</tr>
</tbody>
</table>

## Development Process

## Testing

You will find information about tests in general [here](/docs/process/testing.md).

This section is meant for developing your own tests.

### 
## Tools

### import2mimir


### autocomplete
