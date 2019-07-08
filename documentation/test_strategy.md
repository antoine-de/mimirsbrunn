
### Integration tests

To test, you need to manually build mimir and then simply launch:

```shell
cargo test
```

Integration tests are spawning one ElasticSearch docker, so you'll need a recent docker version. Only one docker is spawn, so ES base has to be cleaned before each test.

To write a new test:

- write your test in a separate file in tests/
- add a call to your test in tests/tests.rs::test_all()
- pass a new ElasticSearchWrapper to your test method to get the right connection string for ES base
- the creation of this ElasticSearchWrapper automatically cleans ES base (you can also refresh ES base, clean up during tests, etc.)

### Geocoding tests

We use [geocoder-tester](https://github.com/geocoders/geocoder-tester) to run real search queries and check the output against expected to prevent regressions.

Feel free to add some tests cases here.

When a new Pull Request is submitted, it will be manually tested using [this repo](https://gitlab.com/QwantResearch/mimir-geocoder-tester/) that loads a bunch of data into the geocoder, runs geocoder-tester and then add the results as a comment in the PR.
