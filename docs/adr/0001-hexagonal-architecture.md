# Use Hexagonal Architecture

* Status: accepted
* Deciders:
	- Matthieu Paindavoine
	- Adrien Matissart
	- Remi Dupre
* Date: 2021-09-01

## Context and Problem Statement

This decision occured in the context of an Elasticsearch migration. Early 2021, mimirsbrunn was
still using Elasticsearch 2, while the current version is 7.XX. That represented a significant gap,
with risk of
- finding ourselves with a version no longer supported (although mitigated by docker)
- missing optimizations (smaller index, faster indexing, faster document retrieval)
- improved functionalities.

Could we migrate Elasticsearch to an up-to-date version, and make it easy in the future to migrate
version?

## Decision Drivers <!-- optional -->

* Elasticsearch was partially hidden behind a crate `rs-es` which was lagging in version.
* The existing architecture could not allow test doubles for Elasticsearch
* The existing architecture implementation relied on synchronous code
* The existing architecture relied on the REST API and reqwest for querying.
* Elasticsearch has released a crate for client code.

## Considered Options

* Just remove `rs-es` and use the official Elasticsearch crate instead, not really changing the
  architecture.
* Using the hexagonal architecture
* [option 3]
* … <!-- numbers of options can vary -->

## Decision Outcome

The original architecture was not well documented, as well as the information
that explained the ranking of documents.

The language (rust) has also progressed very quickly since the previous version
of mimirsbrunn had been written, offering new opportunities with traits, async
code, as well as crates.

We thought it would be best to sit mimirsbrunn on a new architecture enabling
- better tests
- future backend migrations
- additional frontend (for example monitoring)

### Positive Consequences <!-- optional -->

* [e.g., improvement of quality attribute satisfaction, follow-up decisions required, …]
* …

### Negative Consequences <!-- optional -->

* It's a big change and progress on actual new functionalities have been stalled pending the 
  completion of the implementation of this new architecture.

## Pros and Cons of the Options <!-- optional -->

### [option 1]

[example | description | pointer to more information | …] <!-- optional -->

* Good, because [argument a]
* Good, because [argument b]
* Bad, because [argument c]
* … <!-- numbers of pros and cons can vary -->

### [option 2]

[example | description | pointer to more information | …] <!-- optional -->

* Good, because [argument a]
* Good, because [argument b]
* Bad, because [argument c]
* … <!-- numbers of pros and cons can vary -->

### [option 3]

[example | description | pointer to more information | …] <!-- optional -->

* Good, because [argument a]
* Good, because [argument b]
* Bad, because [argument c]
* … <!-- numbers of pros and cons can vary -->

## Links <!-- optional -->

* [Structuring Rust Projects for
  Testability](https://betterprogramming.pub/structuring-rust-project-for-testability-18207b5d0243)
  provides some background on the hexagonal architecture and its implementation
  in Rust.
