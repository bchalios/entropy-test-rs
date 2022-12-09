# Entropy mechanisms tester

This is an experimental project for testing various Linux Random Number Generators (RNGs) under stress scenarios.

In its core, it is a benchmark which performs requests for random bytes from the RNG. The implementation allows configuring
the number of bytes per request, the wait time between requests and the total number of requests.

It allows spawning multiple threads performing the request loop (test for oversubscription).

Currently, it can test:

1. [ThreadRng](https://docs.rs/rand/latest/rand/rngs/struct.ThreadRng.html) from the `rand` crate.
2. [OsRng](https://docs.rs/rand/latest/rand/rngs/struct.OsRng.html) from the `rand` crate.
3. Reading directly from `/dev/urandom/`

In the end of the run it reports statistics for the time taken to perform the requests:

* Average request latency
* Standard deviation
* p50, p90, p99 and p99 percentile latencies
