# roinstx (`r`ust c`oin` `tx`)

## Features

- processing of CSV data and reading them by buffer
- asynchronous TCP server capable of handling concurrent connections.

NOTE: take a look at `csv_stream` to how its works, and how to bind your infra

### Usage

- ##### CLI: 

```sh
cargo r -- transactions.csv > accounts.csv
```
- ##### TCP: 

```sh
cargo r
```


### NOTES:
- I didn't write basic unit tests for the `tx engine` since testing it with `given_sample.csv` is simpler.
- However, I have written a more complex test to examine the program's behavior with varied transactions. 
- Additionally, I've included a more intricate `test.csv` file in the repository for testing purposes along with `expected.csv`
